//! Minecraft vanilla installation and instance repair IPC commands.

use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_game::{
    VersionManifestList, fetch_version_manifest, install_vanilla as game_install_vanilla,
    repair_instance as game_repair_instance,
};
use mclr_net::build_http_client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

async fn current_mc_folder(state: &State<'_, SettingsStore>) -> PathBuf {
    let settings = state.get().await;
    if settings.game.current_folder.is_empty() {
        mclr_core::paths::default_minecraft_dir()
    } else {
        PathBuf::from(settings.game.current_folder)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn install_manifest(
    state: State<'_, SettingsStore>,
    force_refresh: bool,
) -> Result<VersionManifestList, AppError> {
    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    fetch_version_manifest(&client, settings.net.source_priority, force_refresh)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn install_vanilla(
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    version: String,
    name: String,
) -> Result<String, AppError> {
    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    let mc_folder = current_mc_folder(&state).await;
    let net_settings = settings.net.clone();

    let mut params = HashMap::new();
    params.insert("version".to_string(), version.clone());
    params.insert("name".to_string(), name.clone());

    let (task_id, _token) = tasks_state
        .create_task(None, "tasks.item.download.group", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match game_install_vanilla(
            &client,
            &net_settings,
            &mc_folder,
            &version,
            &name,
            Some((&tc, &tid)),
        )
        .await
        {
            Ok(_) => {
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
pub async fn install_repair(
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    instance_id: String,
) -> Result<String, AppError> {
    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    let mc_folder = current_mc_folder(&state).await;
    let net_settings = settings.net.clone();

    let mut params = HashMap::new();
    params.insert("instanceId".to_string(), instance_id.clone());

    let (task_id, _token) = tasks_state
        .create_task(None, "tasks.item.download.group", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match game_repair_instance(
            &client,
            &net_settings,
            &mc_folder,
            &instance_id,
            Some((&tc, &tid)),
        )
        .await
        {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}
