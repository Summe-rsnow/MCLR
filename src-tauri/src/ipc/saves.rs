//! Saves, Datapacks, Screenshots, and Instance Files IPC commands.

use std::path::Path;

use mclr_core::{AppError, SettingsStore};
pub use mclr_saves::{
    DatapackInfo, InstanceFileItem, SaveInfo, ScreenshotInfo, ScreenshotMeta,
    ScreenshotsPagedResult,
};
use tauri::State;

static THUMB_SEMAPHORE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

#[tauri::command]
#[specta::specta]
pub async fn saves_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<SaveInfo>, AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    super::run_blocking(move || mclr_saves::list_saves(&saves_dir)).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_delete(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
) -> Result<(), AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    super::run_blocking(move || mclr_saves::delete_save(&saves_dir, &folder_name)).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_backup(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
) -> Result<String, AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    let backups_dir = base_dir.join("backups");
    super::run_blocking(move || {
        mclr_saves::backup_save(&saves_dir, &backups_dir, &folder_name)
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_export(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
    dest_zip_path: String,
) -> Result<(), AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    super::run_blocking(move || {
        mclr_saves::export_save_zip(&saves_dir, &folder_name, Path::new(&dest_zip_path))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn save_import(
    state: State<'_, SettingsStore>,
    instance_id: String,
    zip_path: String,
) -> Result<SaveInfo, AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let saves_dir = base_dir.join("saves");
    super::run_blocking(move || mclr_saves::import_save_zip(&saves_dir, Path::new(&zip_path))).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_datapacks_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
    folder_name: String,
) -> Result<Vec<DatapackInfo>, AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let save_dir = base_dir.join("saves").join(&folder_name);
    super::run_blocking(move || mclr_saves::list_datapacks(&save_dir)).await
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
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let save_dir = base_dir.join("saves").join(&folder_name);
    super::run_blocking(move || mclr_saves::toggle_datapack(&save_dir, &pack_id, enabled)).await
}

#[tauri::command]
#[specta::specta]
pub async fn screenshots_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<ScreenshotInfo>, AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let screenshots_dir = base_dir.join("screenshots");
    super::run_blocking(move || mclr_saves::list_screenshots(&screenshots_dir)).await
}

#[tauri::command]
#[specta::specta]
pub async fn screenshots_list_paged(
    state: State<'_, SettingsStore>,
    instance_id: String,
    offset: usize,
    limit: usize,
) -> Result<ScreenshotsPagedResult, AppError> {
    let base_dir = super::game_dir_for(&state, &instance_id).await;
    let screenshots_dir = base_dir.join("screenshots");
    super::run_blocking(move || mclr_saves::list_screenshots_paged(&screenshots_dir, offset, limit))
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn screenshot_thumb(path: String) -> Result<String, AppError> {
    let _permit = THUMB_SEMAPHORE
        .acquire()
        .await
        .map_err(|e| AppError::new("media.semaphore").with_detail(e.to_string()))?;
    super::run_blocking(move || mclr_saves::generate_screenshot_thumb(&path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn screenshot_delete(path: String) -> Result<(), AppError> {
    super::run_blocking(move || mclr_saves::delete_screenshot_path(Path::new(&path))).await
}

#[tauri::command]
#[specta::specta]
pub async fn screenshot_reveal(path: String) -> Result<(), AppError> {
    super::run_blocking(move || {
        let p = Path::new(&path);
        if !p.exists() {
            return Err(AppError::new("media.fileNotFound").with_detail("Screenshot not found"));
        }
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("explorer")
                .arg(format!("/select,{}", path))
                .spawn()
                .map_err(|e| AppError::new("media.revealFailed").with_detail(e.to_string()))?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-R", &path])
                .spawn()
                .map_err(|e| AppError::new("media.revealFailed").with_detail(e.to_string()))?;
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            if let Some(parent) = p.parent() {
                opener::open(parent)
                    .map_err(|e| AppError::new("media.revealFailed").with_detail(e.to_string()))?;
            }
        }
        Ok(())
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn screenshot_open(app: tauri::AppHandle, path: String) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt;
    super::run_blocking(move || {
        app.opener()
            .open_path(path, None::<&str>)
            .map_err(|e| AppError::new("saves.io").with_detail(e.to_string()))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn screenshot_copy(path: String) -> Result<(), AppError> {
    super::run_blocking(move || mclr_saves::copy_image_to_clipboard(&path)).await
}
