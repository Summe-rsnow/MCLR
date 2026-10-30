//! Settings IPC commands.

use mclr_core::{AppError, Settings, SettingsStore};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
#[specta::specta]
pub async fn settings_get(state: State<'_, SettingsStore>) -> Result<Settings, AppError> {
    Ok(state.get().await)
}

#[tauri::command]
#[specta::specta]
pub async fn settings_patch(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    http: State<'_, crate::http::HttpState>,
    patch: serde_json::Value,
) -> Result<Settings, AppError> {
    let updated = state.patch(&patch).await.map_err(AppError::from)?;
    // The window close handler reads this atomic instead of awaiting the store.
    app.state::<crate::tray::BackgroundResidency>()
        .set(updated.system.keep_in_background);
    // A new retention window applies immediately; waiting for the next start would
    // leave the user looking at logs they just asked to be cleaned up.
    if patch.pointer("/system/logRetentionDays").is_some() {
        let days = updated.system.log_retention_days;
        tauri::async_runtime::spawn_blocking(move || {
            let _ = mclr_core::prune_old_logs(&mclr_core::paths::logs_dir(), days);
        });
    }
    http.sync(&updated.net)?;
    super::assets::allow_game_folders(&app, &updated);
    let _ = app.emit("settings://changed", &updated);
    Ok(updated)
}
