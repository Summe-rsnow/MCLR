//! Asset-protocol scope management: lets the webview load local files via `convertFileSrc`.

use mclr_core::{AppError, Settings, paths};
use std::path::Path;
use tauri::{AppHandle, Manager, Runtime};

/// Allow the launcher data dir and every configured game folder.
pub fn allow_defaults<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    allow_dir(app, &paths::data_dir());
    allow_game_folders(app, settings);
}

pub fn allow_game_folders<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    for folder in &settings.game.folders {
        allow_dir(app, Path::new(&folder.path));
    }
    if !settings.game.current_folder.is_empty() {
        allow_dir(app, Path::new(&settings.game.current_folder));
    }
}

fn allow_dir<R: Runtime>(app: &AppHandle<R>, dir: &Path) {
    if let Err(e) = app.asset_protocol_scope().allow_directory(dir, true) {
        tracing::warn!(path = %dir.display(), error = %e, "asset scope allow failed");
    }
}

/// Allow a user-picked directory (background image / music folder) recursively.
#[tauri::command]
#[specta::specta]
pub fn assets_allow(app: AppHandle, path: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    let dir = if p.is_dir() {
        p
    } else {
        p.parent().unwrap_or(p)
    };
    app.asset_protocol_scope()
        .allow_directory(dir, true)
        .map_err(|e| AppError::new("app.assetScope").with_detail(e.to_string()))
}
