//! Instance management IPC commands.

use std::path::PathBuf;

use mclr_core::{AppError, SettingsStore};
use mclr_game::instance::{delete_instance_ext, ensure_game_folder};
use mclr_game::{
    InstanceConfig, InstanceInfo, duplicate_instance, rename_instance, save_instance_config,
    scan_instances,
};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use super::current_mc_folder;

#[tauri::command]
#[specta::specta]
pub async fn instances_list(
    state: State<'_, SettingsStore>,
) -> Result<Vec<InstanceInfo>, AppError> {
    let folder = current_mc_folder(&state).await;
    super::run_blocking(move || {
        let _ = ensure_game_folder(&folder);
        scan_instances(&folder)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn instance_rename(
    state: State<'_, SettingsStore>,
    id: String,
    new_name: String,
) -> Result<InstanceInfo, AppError> {
    let folder = current_mc_folder(&state).await;
    super::run_blocking(move || rename_instance(&folder, &id, &new_name)).await
}

#[tauri::command]
#[specta::specta]
pub async fn instance_duplicate(
    state: State<'_, SettingsStore>,
    id: String,
    new_name: String,
) -> Result<InstanceInfo, AppError> {
    let folder = current_mc_folder(&state).await;
    super::run_blocking(move || duplicate_instance(&folder, &id, &new_name)).await
}

#[tauri::command]
#[specta::specta]
pub async fn instance_delete(
    state: State<'_, SettingsStore>,
    id: String,
    permanent: Option<bool>,
) -> Result<(), AppError> {
    let folder = current_mc_folder(&state).await;
    let perm = permanent.unwrap_or(false);
    super::run_blocking(move || delete_instance_ext(&folder, &id, perm)).await
}

#[tauri::command]
#[specta::specta]
pub async fn instance_update_config(
    state: State<'_, SettingsStore>,
    id: String,
    config: InstanceConfig,
) -> Result<InstanceInfo, AppError> {
    let folder = current_mc_folder(&state).await;
    super::run_blocking(move || save_instance_config(&folder, &id, &config)).await
}

/// Open the game directory (or one of its well-known sub-folders) in the OS file manager.
/// `sub` ∈ `mods|resourcepacks|shaderpacks|schematics|saves|screenshots|""`; missing folders are created.
#[tauri::command]
#[specta::specta]
pub async fn instance_open_path(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    id: String,
    sub: Option<String>,
) -> Result<(), AppError> {
    let folder = current_mc_folder(&state).await;
    if !folder.join("versions").join(&id).is_dir() {
        return Err(AppError::new("game.instanceNotFound").with_param("id", id));
    }
    let game_dir = super::game_dir_for(&state, &id).await;
    let sub = sub.unwrap_or_default();
    let target = mclr_game::gamedir::game_subdir(&game_dir, &sub)
        .filter(|_| sub != "servers.dat")
        .ok_or_else(|| AppError::new("game.invalidPath").with_param("sub", sub.clone()))?;
    tokio::fs::create_dir_all(&target)
        .await
        .map_err(|e| AppError::new("game.io").with_detail(e.to_string()))?;
    app.opener()
        .open_path(target.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::new("game.io").with_detail(e.to_string()))?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn path_open(app: AppHandle, path: String) -> Result<(), AppError> {
    let p = PathBuf::from(&path);
    if !p.is_dir() {
        return Err(AppError::new("game.folderNotFound").with_param("path", path));
    }
    app.opener()
        .open_path(p.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::new("game.io").with_detail(e.to_string()))
}
