//! Application-level IPC commands.

use mclr_core::{AppError, AppInfo};

#[tauri::command]
#[specta::specta]
pub fn app_info() -> Result<AppInfo, AppError> {
    Ok(AppInfo::current())
}
