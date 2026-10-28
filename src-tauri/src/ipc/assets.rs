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
///
/// The directory comes from a native dialog, but the command is still callable
/// from the webview, so obviously system-critical locations are rejected: the
/// asset protocol must never be able to read `C:\Windows` or a whole drive.
#[tauri::command]
#[specta::specta]
pub fn assets_allow(app: AppHandle, path: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    let dir = if p.is_dir() {
        p
    } else {
        p.parent().unwrap_or(p)
    };

    reject_dangerous_dir(dir)?;

    app.asset_protocol_scope()
        .allow_directory(dir, true)
        .map_err(|e| AppError::new("app.assetScope").with_detail(e.to_string()))
}

/// Refuses whole drives and the Windows system directories.
fn reject_dangerous_dir(dir: &Path) -> Result<(), AppError> {
    let resolved = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    if resolved.parent().is_none() {
        return Err(AppError::new("core.pathDenied").with_detail(format!(
            "refusing to expose a filesystem root: {}",
            resolved.display()
        )));
    }

    let lowered = resolved.to_string_lossy().to_ascii_lowercase();
    let forbidden = [
        "\\windows",
        "\\windows\\system32",
        "\\program files",
        "\\program files (x86)",
    ];
    if forbidden.iter().any(|f| lowered.contains(f)) {
        return Err(AppError::new("core.pathDenied").with_detail(format!(
            "refusing to expose a system directory: {}",
            resolved.display()
        )));
    }
    Ok(())
}
