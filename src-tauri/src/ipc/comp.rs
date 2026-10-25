//! Community platform (Modrinth / CurseForge) IPC commands.

use std::collections::HashMap;
use std::sync::Arc;

use mclr_comp::{
    CompError, CompFile, CompPlatform, CompProject, CompSearchQuery, CompSearchResult, CompType,
    CurseForgeClient, ModrinthClient,
};
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
    let dest_path = target_dir.join(&file.file_name);

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

        let mut compatible: Vec<&mclr_game::instance::InstanceInfo> =
            instances.iter().filter(|i| is_compatible(i)).collect();

        if compatible.is_empty() {
            return Ok(CompPickTargetResult {
                instance_id: None,
                dir: None,
            });
        }

        compatible.sort_by(|a, b| {
            let dir_a = mclr_game::gamedir::resolve_game_dir(&mc_folder, &a.id, &settings.game);
            let dir_b = mclr_game::gamedir::resolve_game_dir(&mc_folder, &b.id, &settings.game);

            let mtime_a = std::fs::metadata(&dir_a).and_then(|m| m.modified()).ok();
            let mtime_b = std::fs::metadata(&dir_b).and_then(|m| m.modified()).ok();

            match mtime_b.cmp(&mtime_a) {
                std::cmp::Ordering::Equal => {
                    let count_a = std::fs::read_dir(dir_a.join("mods"))
                        .map(|r| r.count())
                        .unwrap_or(0);
                    let count_b = std::fs::read_dir(dir_b.join("mods"))
                        .map(|r| r.count())
                        .unwrap_or(0);
                    count_b.cmp(&count_a)
                }
                ord => ord,
            }
        });

        let chosen = compatible[0];
        let dir = mclr_game::gamedir::resolve_game_dir(&mc_folder, &chosen.id, &settings.game);
        Ok::<_, AppError>(CompPickTargetResult {
            instance_id: Some(chosen.id.clone()),
            dir: Some(dir.to_string_lossy().to_string()),
        })
    })
    .await
}
