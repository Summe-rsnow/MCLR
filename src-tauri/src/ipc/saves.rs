//! Saves, Datapacks, Screenshots, and Instance Files IPC commands.

use std::path::{Path, PathBuf};

use mclr_core::{AppError, SettingsStore};
use mclr_game::InstanceConfig;
use mclr_saves::{DatapackInfo, InstanceFileItem, SaveInfo, ScreenshotInfo};
use tauri::State;

async fn instance_base_dir(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
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
    if instance_config.isolation == "on"
        || (instance_config.isolation == "global" && settings.game.default_isolation == "on")
    {
        instance_dir
    } else {
        mc_folder
    }
}

fn validate_folder_kind(folder_kind: &str) -> Result<(), AppError> {
    match folder_kind {
        "resourcepacks" | "shaderpacks" | "schematics" => Ok(()),
        _ => Err(AppError::new("saves.invalidPath").with_detail("invalid folder_kind")),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn saves_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<SaveInfo>, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    mclr_saves::list_saves(&saves_dir).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_delete(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
) -> Result<(), AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    mclr_saves::delete_save(&saves_dir, &folder_name).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_backup(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
) -> Result<String, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    let backups_dir = base_dir.join("backups");
    let backup_path =
        mclr_saves::backup_save(&saves_dir, &backups_dir, &folder_name).map_err(AppError::from)?;
    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn save_export(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
    dest_zip_path: String,
) -> Result<(), AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    mclr_saves::export_save_zip(&saves_dir, &folder_name, Path::new(&dest_zip_path))
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_import(
    state: State<'_, SettingsStore>,
    instance_id: String,
    zip_path: String,
) -> Result<SaveInfo, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    mclr_saves::import_save_zip(&saves_dir, Path::new(&zip_path)).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_datapacks_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
) -> Result<Vec<DatapackInfo>, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let save_dir = base_dir.join("saves").join(&folder_name);
    mclr_saves::list_datapacks(&save_dir).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn save_datapack_toggle(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
    pack_id: String,
    enabled: bool,
) -> Result<Vec<DatapackInfo>, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let save_dir = base_dir.join("saves").join(&folder_name);
    mclr_saves::toggle_datapack(&save_dir, &pack_id, enabled).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn screenshots_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<ScreenshotInfo>, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let screenshots_dir = base_dir.join("screenshots");
    mclr_saves::list_screenshots(&screenshots_dir).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn screenshot_delete(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<(), AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    let screenshots_dir = base_dir.join("screenshots");
    mclr_saves::delete_screenshot(&screenshots_dir, &file_name).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_files_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_kind: String,
) -> Result<Vec<InstanceFileItem>, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    validate_folder_kind(&folder_kind)?;
    let dir = base_dir.join(&folder_kind);
    mclr_saves::list_instance_files(&dir).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_file_toggle(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_kind: String,
    file_name: String,
    enabled: bool,
) -> Result<InstanceFileItem, AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    validate_folder_kind(&folder_kind)?;
    let dir = base_dir.join(&folder_kind);
    mclr_saves::toggle_instance_file(&dir, &file_name, enabled).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn instance_file_delete(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_kind: String,
    file_name: String,
) -> Result<(), AppError> {
    let base_dir = instance_base_dir(&state, &instance_id).await;
    validate_folder_kind(&folder_kind)?;
    let dir = base_dir.join(&folder_kind);
    mclr_saves::delete_instance_file(&dir, &file_name).map_err(AppError::from)
}
