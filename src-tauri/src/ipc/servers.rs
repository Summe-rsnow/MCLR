//! Server list and SLP ping IPC commands.

use std::path::PathBuf;

use mclr_core::{AppError, SettingsStore};
use mclr_game::InstanceConfig;
use mclr_saves::{
    ServerEntry, ServerPingResult, add_server, ping_server, read_servers_dat, remove_server,
    update_server,
};
use tauri::State;

async fn instance_servers_dat_path(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
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
    base_dir.join("servers.dat")
}

#[tauri::command]
#[specta::specta]
pub async fn servers_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<ServerEntry>, AppError> {
    let path = instance_servers_dat_path(&state, &instance_id).await;
    read_servers_dat(&path).map_err(AppError::from)
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
    add_server(&path, name, ip, accept_textures).map_err(AppError::from)
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
    update_server(&path, index, name, ip, accept_textures).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn server_remove(
    state: State<'_, SettingsStore>,
    instance_id: String,
    index: usize,
) -> Result<Vec<ServerEntry>, AppError> {
    let path = instance_servers_dat_path(&state, &instance_id).await;
    remove_server(&path, index).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn server_ping(ip: String) -> Result<ServerPingResult, AppError> {
    ping_server(&ip).await.map_err(AppError::from)
}
