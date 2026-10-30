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

/// Quits the launcher even when it would otherwise hide to the tray.
#[tauri::command]
#[specta::specta]
pub fn app_quit(app: tauri::AppHandle) -> Result<(), AppError> {
    app.exit(0);
    Ok(())
}

/// Whether the software-renderer bridge (Mesa loader agent) is installed.
///
/// The renderer selector is only meaningful when this jar exists; without it every
/// non-default choice silently falls back to the game default.
#[tauri::command]
#[specta::specta]
pub fn renderer_agent_available() -> Result<bool, AppError> {
    Ok(mclr_core::paths::tools_dir()
        .join("mesa-loader")
        .join("Loader.jar")
        .is_file())
}
