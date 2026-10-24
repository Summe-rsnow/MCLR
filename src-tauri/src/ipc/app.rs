//! Application-level IPC commands.

use mclr_core::{AppError, AppInfo, SecretsStatus, secrets_status as get_secrets_status};

#[tauri::command]
#[specta::specta]
pub fn app_info() -> Result<AppInfo, AppError> {
    Ok(AppInfo::current())
}

#[tauri::command]
#[specta::specta]
pub fn secrets_status() -> Result<SecretsStatus, AppError> {
    Ok(get_secrets_status())
}

#[tauri::command]
#[specta::specta]
pub fn window_set_material(window: tauri::Window, glass: bool) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        if glass {
            // Apply Mica effect. If unsupported (e.g. earlier Windows versions), ignore gracefully.
            let _ = window_vibrancy::apply_mica(&window, None);
        } else {
            let _ = window_vibrancy::clear_mica(&window);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, glass);
    }

    Ok(())
}
