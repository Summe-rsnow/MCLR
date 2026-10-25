//! Settings IPC commands.

use mclr_core::{AppError, Settings, SettingsStore};
use tauri::{AppHandle, Emitter, State};

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
    http.sync(&updated.net)?;
    super::assets::allow_game_folders(&app, &updated);
    let _ = app.emit("settings://changed", &updated);
    Ok(updated)
}
