//! Local mods, Jar-in-Jar, and dependency resolution IPC commands.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use mclr_comp::{
    CompError, CompFile, CurseForgeClient, JarInJarNode, LocalModInfo, ModrinthClient,
    ResolvedDependencyItem, check_local_mods_updates, delete_local_mod, resolve_dependencies,
    scan_jar_in_jar, scan_local_mods, toggle_local_mod,
};
use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_game::InstanceConfig;
use mclr_net::{DownloadSpec, Downloader, build_http_client};
use tauri::State;
use url::Url;

async fn instance_mods_dir(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
    let settings = state.get().await;
    let mc_folder = if settings.game.current_folder.is_empty() {
        mclr_core::paths::default_minecraft_dir()
    } else {
        PathBuf::from(&settings.game.current_folder)
    };
    let instance_dir = mc_folder.join("versions").join(instance_id);
    let config_path = instance_dir.join("mclr.instance.json");
    let instance_config = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| serde_json::from_str::<InstanceConfig>(&content).ok())
            .unwrap_or_default()
    } else {
        InstanceConfig::default()
    };
    let base_dir = if instance_config.isolation == "on"
        || (instance_config.isolation == "global" && settings.game.default_isolation == "on")
    {
        instance_dir
    } else {
        mc_folder
    };
    let mods_dir = base_dir.join("mods");
    let _ = std::fs::create_dir_all(&mods_dir);
    mods_dir
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<LocalModInfo>, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let mods = scan_local_mods(&mods_dir).map_err(AppError::from)?;
    Ok(mods)
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
    let info = toggle_local_mod(&mods_dir, &file_name, enabled).map_err(AppError::from)?;
    Ok(info)
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_delete(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<(), AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    delete_local_mod(&mods_dir, &file_name).map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_check_updates(
    state: State<'_, SettingsStore>,
    instance_id: String,
    mc_version: String,
    loader: String,
) -> Result<Vec<LocalModInfo>, AppError> {
    let settings = state.get().await;
    let modrinth = ModrinthClient::new(settings.net.source_priority);
    let curseforge = CurseForgeClient::new(settings.net.source_priority);

    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let mods = check_local_mods_updates(
        &mods_dir,
        &mc_version,
        &loader,
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
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
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
    let nodes = scan_jar_in_jar(&jar_path).map_err(AppError::from)?;
    Ok(nodes)
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
    let installed_mods = scan_local_mods(&mods_dir).unwrap_or_default();
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
