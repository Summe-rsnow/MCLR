//! Local mods, Jar-in-Jar, and dependency resolution IPC commands.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::stream::{self, StreamExt};
use mclr_comp::resources::{ResourceEntry, ResourceKind};
use mclr_comp::{
    CompError, CompFile, CurseForgeClient, JarInJarNode, LocalModInfo, ModrinthClient,
    ResolvedDependencyItem, check_local_mods_updates, delete_local_mod, resolve_dependencies,
    scan_jar_in_jar, scan_local_mods, toggle_local_mod,
};
use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalModsScanResult {
    pub entries: Vec<ResourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalModUpdateItem {
    pub current_file_name: String,
    pub new_file: CompFile,
}

async fn instance_mods_dir(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
    let mods_dir = super::game_dir_for(state, instance_id).await.join("mods");
    let _ = tokio::fs::create_dir_all(&mods_dir).await;
    mods_dir
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<LocalModInfo>, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || scan_local_mods(&mods_dir)).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_toggle(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
    enabled: bool,
) -> Result<LocalModInfo, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || toggle_local_mod(&mods_dir, &file_name, enabled)).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_delete(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<(), AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || delete_local_mod(&mods_dir, &file_name)).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_check_updates(
    state: State<'_, SettingsStore>,
    instance_id: String,
    mc_version: String,
    loader: String,
    force: bool,
) -> Result<Vec<LocalModInfo>, AppError> {
    let settings = state.get().await;
    let modrinth = ModrinthClient::new(settings.net.source_priority);
    let curseforge = CurseForgeClient::new(settings.net.source_priority);

    let mods_dir = instance_mods_dir(&state, &instance_id).await;

    // The instance, not the caller, knows which loaders it runs: detection can find several
    // (Forge + OptiFine, Cleanroom + Forge) and the caller's single label only carries the first.
    let mut loaders: Vec<String> = Vec::new();
    let mc_folder = super::current_mc_folder(&state).await;
    let scan_id = instance_id.clone();
    if let Ok(instances) =
        tokio::task::spawn_blocking(move || mclr_game::instance::scan_instances(&mc_folder)).await
        && let Ok(instances) = instances
        && let Some(instance) = instances.into_iter().find(|i| i.id == scan_id)
    {
        loaders = instance.loaders;
    }
    if loaders.is_empty() && !loader.trim().is_empty() {
        loaders.push(loader);
    }

    let mods = check_local_mods_updates(
        &mods_dir,
        &mc_version,
        &loaders,
        force,
        &modrinth,
        Some(&curseforge),
    )
    .await
    .map_err(AppError::from)?;

    Ok(mods)
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_update(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    instance_id: String,
    current_file_name: String,
    new_file: CompFile,
) -> Result<String, AppError> {
    if new_file.distribution_disabled || new_file.download_urls.is_empty() {
        return Err(CompError::DistributionDisabled {
            project_id: new_file.project_id,
            file_id: new_file.id,
            website_url: None,
        }
        .into());
    }

    let urls: Vec<Url> = new_file
        .download_urls
        .iter()
        .filter_map(|u| Url::parse(u).ok())
        .collect();
    if urls.is_empty() {
        return Err(
            CompError::InvalidParams("No valid download URLs in CompFile".to_string()).into(),
        );
    }

    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let dest_path = mods_dir.join(&new_file.file_name);

    let mut params = HashMap::new();
    params.insert("name".to_string(), new_file.name.clone());
    params.insert("fileName".to_string(), new_file.file_name.clone());
    params.insert("targetInstance".to_string(), instance_id);

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.compDownload", params)
        .await;

    let spec = DownloadSpec {
        urls,
        dest: dest_path,
        size: if new_file.size > 0 {
            Some(new_file.size)
        } else {
            None
        },
        sha1: new_file.hashes.sha1,
        sha256: None,
        md5: None,
    };

    let settings = state.get().await;
    let client = http.client();
    let downloader = Downloader::new(client, settings.net.clone());
    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();
    let old_file_name = current_file_name.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match downloader.download(spec, Some((&tc, &tid))).await {
            Ok(()) => {
                if !old_file_name.is_empty() && old_file_name != new_file.file_name {
                    let old_path = mods_dir.join(&old_file_name);
                    if old_path.exists() {
                        let _ = delete_local_mod(&mods_dir, &old_file_name);
                    }
                }
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
pub async fn local_mod_jar_in_jar(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<Vec<JarInJarNode>, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let jar_path = mods_dir.join(&file_name);
    super::run_blocking(move || scan_jar_in_jar(&jar_path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn comp_resolve_dependencies(
    state: State<'_, SettingsStore>,
    file: CompFile,
    instance_id: String,
    mc_version: String,
    loader: String,
) -> Result<Vec<ResolvedDependencyItem>, AppError> {
    let settings = state.get().await;
    let modrinth = ModrinthClient::new(settings.net.source_priority);
    let curseforge = CurseForgeClient::new(settings.net.source_priority);

    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let installed_mods = super::run_blocking(move || scan_local_mods(&mods_dir))
        .await
        .unwrap_or_default();
    let mut installed_project_ids = HashSet::new();
    let mut installed_mod_ids = HashSet::new();

    for m in installed_mods {
        installed_mod_ids.insert(m.mod_id.to_ascii_lowercase());
        if let Some(pid) = m.project_id {
            installed_project_ids.insert(pid.to_ascii_lowercase());
        }
    }

    let items = resolve_dependencies(
        &file,
        &mc_version,
        &loader,
        &installed_project_ids,
        &installed_mod_ids,
        &modrinth,
        Some(&curseforge),
    )
    .await
    .map_err(AppError::from)?;

    Ok(items)
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_scan_start(
    app: tauri::AppHandle,
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    instance_id: String,
    request_id: String,
) -> Result<LocalModsScanResult, AppError> {
    let entries = super::resources::resources_list(
        app,
        http,
        state,
        instance_id,
        ResourceKind::Mod,
        None,
        Some(request_id),
    )
    .await?;
    Ok(LocalModsScanResult { entries })
}
#[tauri::command]
#[specta::specta]
pub async fn local_mods_dependents(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<Vec<String>, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || {
        let all_mods = scan_local_mods(&mods_dir)?;
        let target_mod = all_mods.iter().find(|m| m.file_name == file_name);
        let target_mod_id = match target_mod {
            Some(m) => m.mod_id.to_ascii_lowercase(),
            None => return Ok::<_, CompError>(Vec::new()),
        };

        let mut dependents = Vec::new();
        for m in &all_mods {
            if m.file_name == file_name {
                continue;
            }
            let jar_path = mods_dir.join(&m.file_name);
            if let Ok(nodes) = mclr_comp::jarinjar::scan_jar_in_jar(&jar_path) {
                fn has_id(nodes: &[mclr_comp::JarInJarNode], id: &str) -> bool {
                    for n in nodes {
                        if n.id.to_ascii_lowercase() == id || has_id(&n.children, id) {
                            return true;
                        }
                    }
                    false
                }
                if has_id(&nodes, &target_mod_id) {
                    dependents.push(m.name.clone());
                }
            }
        }
        Ok(dependents)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_update_many(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    instance_id: String,
    updates: Vec<LocalModUpdateItem>,
) -> Result<String, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let mut params = HashMap::new();
    params.insert("targetInstance".to_string(), instance_id.clone());
    params.insert("count".to_string(), updates.len().to_string());

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.compDownload", params)
        .await;

    let settings = state.get().await;
    let concurrency = settings.net.max_concurrent_files.clamp(1, 16) as usize;
    let client = http.client();
    let downloader = Downloader::new(client, settings.net.clone());
    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        let control = tc.control(&tid).await;
        let total = updates.len().max(1);
        let completed = Arc::new(AtomicUsize::new(0));

        // Bounded concurrency: each mod owns its own jar, so one slow host no longer
        // serialises the whole batch. Every superseded jar is deleted the moment its
        // replacement is verified on disk, so a failure later in the batch can never
        // leave a half-migrated instance behind.
        let downloader = &downloader;

        let results: Vec<Result<(), AppError>> = stream::iter(updates.into_iter().map(|item| {
            let tc = Arc::clone(&tc);
            let tid = tid.clone();
            let completed = Arc::clone(&completed);
            let control = control.clone();
            let mods_dir = mods_dir.clone();
            async move {
                if let Some(ctrl) = &control
                    && ctrl.checkpoint().await.is_err()
                {
                    return Err(AppError::from(mclr_net::error::NetError::Canceled));
                }

                let urls: Vec<Url> = item
                    .new_file
                    .download_urls
                    .iter()
                    .filter_map(|u| Url::parse(u).ok())
                    .collect();
                if !urls.is_empty() {
                    let spec = DownloadSpec {
                        urls,
                        dest: mods_dir.join(&item.new_file.file_name),
                        size: if item.new_file.size > 0 {
                            Some(item.new_file.size)
                        } else {
                            None
                        },
                        sha1: item.new_file.hashes.sha1.clone(),
                        sha256: None,
                        md5: None,
                    };

                    downloader.download(spec, Some((&tc, &tid))).await?;

                    // Per-file atomic replacement: the old jar only disappears once its
                    // successor is fully downloaded and hash-verified.
                    if !item.current_file_name.is_empty()
                        && item.current_file_name != item.new_file.file_name
                    {
                        let old_path = mods_dir.join(&item.current_file_name);
                        if old_path.exists() {
                            let _ = delete_local_mod(&mods_dir, &item.current_file_name);
                        }
                    }
                }

                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = tc
                    .update_progress(&tid, done as f32 / total as f32, 0)
                    .await;
                Ok(())
            }
        }))
        .buffer_unordered(concurrency)
        .collect()
        .await;

        match results.into_iter().find_map(Result::err) {
            Some(e) => {
                let _ = tc.fail_task(&tid, e).await;
            }
            None => {
                let _ = tc.complete_task(&tid).await;
            }
        }
    });

    Ok(task_id)
}
