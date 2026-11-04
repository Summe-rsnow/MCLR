//! Community platform (Modrinth / CurseForge) IPC commands.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use futures_util::stream::{self, StreamExt};
use mclr_comp::{
    CompError, CompFile, CompPlatform, CompProject, CompSearchQuery, CompSearchResult, CompType,
    CurseForgeClient, ModrinthClient,
};
use mclr_comp::{modname, naming};
use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompPickTargetResult {
    pub instance_id: Option<String>,
    pub dir: Option<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn comp_search(
    state: State<'_, SettingsStore>,
    query: CompSearchQuery,
) -> Result<CompSearchResult, AppError> {
    let settings = state.get().await;
    match query.platform {
        CompPlatform::Modrinth => {
            let client = ModrinthClient::new(settings.net.source_priority);
            client.search(&query).await.map_err(AppError::from)
        }
        CompPlatform::Curseforge => {
            let client = CurseForgeClient::new(settings.net.source_priority);
            client.search(&query).await.map_err(AppError::from)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn comp_project(
    state: State<'_, SettingsStore>,
    platform: CompPlatform,
    project_id: String,
) -> Result<CompProject, AppError> {
    let settings = state.get().await;
    match platform {
        CompPlatform::Modrinth => {
            let client = ModrinthClient::new(settings.net.source_priority);
            client
                .get_project(&project_id)
                .await
                .map_err(AppError::from)
        }
        CompPlatform::Curseforge => {
            let client = CurseForgeClient::new(settings.net.source_priority);
            client
                .get_project(&project_id)
                .await
                .map_err(AppError::from)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn comp_files(
    state: State<'_, SettingsStore>,
    platform: CompPlatform,
    project_id: String,
    mc_version: Option<String>,
    loader: Option<String>,
) -> Result<Vec<CompFile>, AppError> {
    let settings = state.get().await;
    match platform {
        CompPlatform::Modrinth => {
            let client = ModrinthClient::new(settings.net.source_priority);
            client
                .get_versions(&project_id, mc_version.as_deref(), loader.as_deref())
                .await
                .map_err(AppError::from)
        }
        CompPlatform::Curseforge => {
            let client = CurseForgeClient::new(settings.net.source_priority);
            client
                .get_files(&project_id, mc_version.as_deref(), loader.as_deref())
                .await
                .map_err(AppError::from)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn comp_download(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    file: CompFile,
    target_instance: String,
    kind: CompType,
    project_slug: Option<String>,
) -> Result<String, AppError> {
    if file.distribution_disabled || file.download_urls.is_empty() {
        return Err(CompError::DistributionDisabled {
            project_id: file.project_id,
            file_id: file.id,
            website_url: None,
        }
        .into());
    }

    let urls: Vec<Url> = file
        .download_urls
        .iter()
        .filter_map(|u| Url::parse(u).ok())
        .collect();
    if urls.is_empty() {
        return Err(
            CompError::InvalidParams("No valid download URLs in CompFile".to_string()).into(),
        );
    }

    let settings = state.get().await;
    let client = http.client();

    let base_dir = super::game_dir_for(&state, &target_instance).await;

    let target_dir = base_dir.join(kind.target_subfolder());
    tokio::fs::create_dir_all(&target_dir)
        .await
        .map_err(CompError::Io)?;
    let zh_name = if settings.comp.show_zh_name {
        project_slug
            .as_deref()
            .and_then(modname::lookup_by_slug)
            .map(modname::display_zh)
    } else {
        None
    };
    let dest_name = naming::format_download_name(
        &file.file_name,
        zh_name,
        kind,
        settings.comp.filename_format,
    );
    let dest_path = target_dir.join(dest_name);

    let mut params = HashMap::new();
    params.insert("name".to_string(), file.name.clone());
    params.insert("fileName".to_string(), file.file_name.clone());
    params.insert("targetInstance".to_string(), target_instance);

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.compDownload", params)
        .await;

    let spec = DownloadSpec {
        urls,
        dest: dest_path,
        size: if file.size > 0 { Some(file.size) } else { None },
        sha1: file.hashes.sha1,
        sha256: None,
        md5: None,
    };

    let downloader = Downloader::new(client, settings.net.clone());
    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match downloader.download(spec, Some((&tc, &tid))).await {
            Ok(()) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn comp_pick_target(
    state: State<'_, SettingsStore>,
    project_type: CompType,
    preferred_instance_id: Option<String>,
    mc_versions: Vec<String>,
    loaders: Vec<String>,
) -> Result<CompPickTargetResult, AppError> {
    let settings = state.get().await;
    let mc_folder = super::current_mc_folder(&state).await;

    super::run_blocking(move || {
        let instances = mclr_game::instance::scan_instances(&mc_folder).unwrap_or_default();

        let is_compatible = |inst: &mclr_game::instance::InstanceInfo| -> bool {
            if !mc_versions.is_empty() {
                let match_ver = mc_versions.iter().any(|v| {
                    v.eq_ignore_ascii_case(&inst.mc_version) || inst.mc_version.starts_with(v)
                });
                if !match_ver {
                    return false;
                }
            }
            if project_type == CompType::Mod && !loaders.is_empty() {
                let any_loader = loaders.iter().any(|l| l.eq_ignore_ascii_case("any"));
                if !any_loader {
                    let has_loader = loaders
                        .iter()
                        .any(|l| inst.loaders.iter().any(|il| il.eq_ignore_ascii_case(l)));
                    if !has_loader {
                        return false;
                    }
                }
            }
            true
        };

        if let Some(pref_id) = &preferred_instance_id {
            if let Some(inst) = instances
                .iter()
                .find(|i| i.id == *pref_id || i.name == *pref_id)
            {
                if is_compatible(inst) {
                    let dir =
                        mclr_game::gamedir::resolve_game_dir(&mc_folder, &inst.id, &settings.game);
                    return Ok(CompPickTargetResult {
                        instance_id: Some(inst.id.clone()),
                        dir: Some(dir.to_string_lossy().to_string()),
                    });
                }
            }
        }

        // One IO pass per instance (resolve dir + metadata + mod count), then the
        // ordering itself is pure in-memory comparison.
        let mut candidates: Vec<(
            &mclr_game::instance::InstanceInfo,
            PathBuf,
            Option<std::time::SystemTime>,
            usize,
        )> = instances
            .iter()
            .filter(|i| is_compatible(i))
            .map(|inst| {
                let dir =
                    mclr_game::gamedir::resolve_game_dir(&mc_folder, &inst.id, &settings.game);
                let mtime = std::fs::metadata(&dir).and_then(|m| m.modified()).ok();
                let mod_count = std::fs::read_dir(dir.join("mods"))
                    .map(|r| r.count())
                    .unwrap_or(0);
                (inst, dir, mtime, mod_count)
            })
            .collect();

        if candidates.is_empty() {
            return Ok(CompPickTargetResult {
                instance_id: None,
                dir: None,
            });
        }

        candidates.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| b.3.cmp(&a.3)));

        let (chosen, chosen_dir, _, _) = candidates.swap_remove(0);
        Ok::<_, AppError>(CompPickTargetResult {
            instance_id: Some(chosen.id.clone()),
            dir: Some(chosen_dir.to_string_lossy().to_string()),
        })
    })
    .await
}

/// Resolves a bare mod id (or Modrinth slug) to its Modrinth project.
///
/// Used by the dependency drawer for a quick jump: a miss is an ordinary `None`, never an
/// error, because a mod id simply does not have to exist on Modrinth.
#[tauri::command]
#[specta::specta]
pub async fn comp_dep_lookup(
    state: State<'_, SettingsStore>,
    mod_id: String,
) -> Result<Option<CompProject>, AppError> {
    let settings = state.get().await;
    let client = ModrinthClient::new(settings.net.source_priority);
    Ok(client.get_project(&mod_id).await.ok())
}

/// Fetches several projects of one platform for the dependency list.
///
/// Missing or unreachable projects are dropped instead of failing the batch: a dependency
/// list is still useful when one of its entries vanished from the platform.
#[tauri::command]
#[specta::specta]
pub async fn comp_dep_projects(
    state: State<'_, SettingsStore>,
    platform: CompPlatform,
    project_ids: Vec<String>,
) -> Result<Vec<CompProject>, AppError> {
    let settings = state.get().await;
    let mut seen = HashSet::new();
    let ids: Vec<String> = project_ids
        .into_iter()
        .filter(|id| !id.is_empty() && seen.insert(id.clone()))
        .collect();

    let projects: Vec<Option<CompProject>> = match platform {
        CompPlatform::Modrinth => {
            let client = ModrinthClient::new(settings.net.source_priority);
            let client = &client;
            stream::iter(
                ids.into_iter()
                    .map(move |id| async move { client.get_project(&id).await.ok() }),
            )
            .buffer_unordered(6)
            .collect()
            .await
        }
        CompPlatform::Curseforge => {
            let client = CurseForgeClient::new(settings.net.source_priority);
            let client = &client;
            stream::iter(
                ids.into_iter()
                    .map(move |id| async move { client.get_project(&id).await.ok() }),
            )
            .buffer_unordered(6)
            .collect()
            .await
        }
    };

    Ok(projects.into_iter().flatten().collect())
}
