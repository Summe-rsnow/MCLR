//! Instance management IPC commands.

use mclr_core::{AppError, SettingsStore};
use mclr_game::{
    InstanceConfig, InstanceInfo, delete_instance, duplicate_instance, rename_instance,
    save_instance_config, scan_instances,
};
use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

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
pub async fn instances_list(
    state: State<'_, SettingsStore>,
) -> Result<Vec<InstanceInfo>, AppError> {
    let folder = current_mc_folder(&state).await;
    scan_instances(&folder).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_rename(
    state: State<'_, SettingsStore>,
    id: String,
    new_name: String,
) -> Result<InstanceInfo, AppError> {
    let folder = current_mc_folder(&state).await;
    rename_instance(&folder, &id, &new_name).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_duplicate(
    state: State<'_, SettingsStore>,
    id: String,
    new_name: String,
) -> Result<InstanceInfo, AppError> {
    let folder = current_mc_folder(&state).await;
    duplicate_instance(&folder, &id, &new_name).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_delete(state: State<'_, SettingsStore>, id: String) -> Result<(), AppError> {
    let folder = current_mc_folder(&state).await;
    delete_instance(&folder, &id).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_update_config(
    state: State<'_, SettingsStore>,
    id: String,
    config: InstanceConfig,
) -> Result<InstanceInfo, AppError> {
    let folder = current_mc_folder(&state).await;
    save_instance_config(&folder, &id, &config).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_open_folder(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    id: String,
) -> Result<(), AppError> {
    let folder = current_mc_folder(&state).await;
    let instance_dir = folder.join("versions").join(&id);
    if !instance_dir.exists() {
        return Err(AppError::new("game.instanceNotFound").with_param("id", id));
    }
    app.opener()
        .open_path(instance_dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::new("game.io").with_detail(e.to_string()))?;
    Ok(())
}
