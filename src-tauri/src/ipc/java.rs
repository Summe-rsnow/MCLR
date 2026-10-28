//! Java discovery IPC commands.

use mclr_core::{AppError, SettingsStore};
use mclr_game::{JavaInstallation, scan_java};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
#[specta::specta]
pub async fn java_scan(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    force_refresh: Option<bool>,
) -> Result<Vec<JavaInstallation>, AppError> {
    let settings = state.get().await;
    let custom = settings.java.custom.clone();
    let force = force_refresh.unwrap_or(false);

    if !force {
        let cached = mclr_game::java::cached_java();
        if !cached.is_empty() {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                let updated =
                    super::run_blocking(move || Ok::<_, AppError>(scan_java(&custom))).await;
                if let Ok(list) = updated {
                    let _ = app_clone.emit("java://updated", &list);
                }
            });
            return Ok(cached);
        }
    }

    let list = super::run_blocking(move || Ok::<_, AppError>(scan_java(&custom))).await?;
    let _ = app.emit("java://updated", &list);
    Ok(list)
}

#[tauri::command]
#[specta::specta]
pub async fn java_list(
    app: AppHandle,
    state: State<'_, SettingsStore>,
) -> Result<Vec<JavaInstallation>, AppError> {
    java_scan(app, state, Some(false)).await
}
