//! Server list and SLP ping IPC commands.

use std::path::PathBuf;

use mclr_core::{AppError, SettingsStore};
use mclr_saves::{
    ServerEntry, ServerPingResult, add_server, ping_server, read_servers_dat, remove_server,
    update_server,
};
use tauri::State;

async fn instance_servers_dat_path(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
    super::game_dir_for(state, instance_id)
        .await
        .join("servers.dat")
}

#[tauri::command]
#[specta::specta]
pub async fn servers_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<ServerEntry>, AppError> {
    let path = instance_servers_dat_path(&state, &instance_id).await;
    super::run_blocking(move || read_servers_dat(&path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn server_add(
    state: State<'_, SettingsStore>,
    instance_id: String,
    name: String,
    ip: String,
    accept_textures: Option<i8>,
) -> Result<Vec<ServerEntry>, AppError> {
    let path = instance_servers_dat_path(&state, &instance_id).await;
    super::run_blocking(move || add_server(&path, name, ip, accept_textures)).await
}

#[tauri::command]
#[specta::specta]
pub async fn server_update(
    state: State<'_, SettingsStore>,
    instance_id: String,
    index: usize,
    name: String,
    ip: String,
    accept_textures: Option<i8>,
) -> Result<Vec<ServerEntry>, AppError> {
    let path = instance_servers_dat_path(&state, &instance_id).await;
    super::run_blocking(move || update_server(&path, index, name, ip, accept_textures)).await
}

#[tauri::command]
#[specta::specta]
pub async fn server_remove(
    state: State<'_, SettingsStore>,
    instance_id: String,
    index: usize,
) -> Result<Vec<ServerEntry>, AppError> {
    let path = instance_servers_dat_path(&state, &instance_id).await;
    super::run_blocking(move || remove_server(&path, index)).await
}

#[tauri::command]
#[specta::specta]
pub async fn server_ping(ip: String) -> Result<ServerPingResult, AppError> {
    ping_server(&ip).await.map_err(AppError::from)
}
