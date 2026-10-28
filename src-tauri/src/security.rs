//! Path containment for IPC commands.
//!
//! Tauri v2 capabilities only restrict **plugin** commands; our own
//! `#[tauri::command]`s are callable by anything running in the webview. Commands
//! that take a path from the frontend therefore validate it here against the set
//! of roots the launcher actually owns, so an injected script cannot delete or
//! overwrite files outside the game/data directories.
//!
//! Roots (all of them, no wildcards):
//! - launcher data dir (`%APPDATA%/MCLR`): config, accounts, cache, logs, tools
//! - every configured game folder and the current one
//! - an explicitly granted media/library directory (`assets_allow`)

use std::path::{Path, PathBuf};

use mclr_core::{AppError, Settings, paths};

/// Resolves a path to its absolute form without requiring it to exist.
///
/// `canonicalize` fails on missing files, so the nearest existing ancestor is
/// canonicalized and the remaining components are re-appended. Both sides of a
/// containment check therefore go through the same normalization (Windows
/// verbatim `\\?\` prefixes included), which is what makes `starts_with` reliable.
fn absolutize(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }

    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut current = path;
    loop {
        let Some(parent) = current.parent() else {
            return path.to_path_buf();
        };
        if let Some(name) = current.file_name() {
            missing.push(name.to_os_string());
        }
        current = parent;
        if let Ok(canonical) = current.canonicalize() {
            let mut resolved = canonical;
            for part in missing.iter().rev() {
                resolved.push(part);
            }
            return resolved;
        }
    }
}

/// System locations that must never be read for content or written by the UI.
const SYSTEM_PREFIXES: &[&str] = &[
    "\\windows",
    "\\program files",
    "\\program files (x86)",
    "\\programdata\\microsoft",
    "\\$recycle.bin",
    "\\system volume information",
];

/// Rejects filesystem roots and OS-critical directories.
///
/// This is the *read* tier: user-picked files (modpacks, schematics, skins) come
/// from a native dialog and may legitimately live in `Downloads` or on another
/// drive, so they are not confined to the launcher directories — but reading
/// `C:\Windows\...` over IPC is never legitimate.
pub fn ensure_not_system_path(path: &Path) -> Result<PathBuf, AppError> {
    if path.as_os_str().is_empty() {
        return Err(AppError::new("core.pathDenied").with_detail("empty path"));
    }
    let target = absolutize(path);

    if target.parent().is_none() {
        return Err(AppError::new("core.pathDenied")
            .with_detail(format!("refusing filesystem root: {}", target.display())));
    }

    // Normalize separators: a caller may pass `C:/Windows/...` or `C:\Windows\...`.
    let lowered = target
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    if SYSTEM_PREFIXES
        .iter()
        .any(|prefix| lowered.contains(prefix))
    {
        return Err(AppError::new("core.pathDenied")
            .with_detail(format!("refusing system path: {}", target.display())));
    }
    Ok(target)
}

/// Every directory the launcher may touch on the user's behalf.
pub fn allowed_roots(settings: &Settings) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = vec![
        paths::data_dir(),
        paths::cache_dir(),
        paths::logs_dir(),
        paths::runtime_dir(),
        paths::tools_dir(),
        paths::default_minecraft_dir(),
    ];

    for folder in &settings.game.folders {
        if !folder.path.trim().is_empty() {
            roots.push(PathBuf::from(&folder.path));
        }
    }
    if !settings.game.current_folder.trim().is_empty() {
        roots.push(PathBuf::from(&settings.game.current_folder));
    }

    roots
}

/// Returns `Ok(path)` when it lives under one of `roots`.
pub fn ensure_within_roots(path: &Path, roots: &[PathBuf]) -> Result<PathBuf, AppError> {
    if path.as_os_str().is_empty() {
        return Err(AppError::new("core.pathDenied").with_detail("empty path"));
    }

    let target = absolutize(path);

    // A bare drive root or a root directory itself is never a valid resource.
    if target.parent().is_none() {
        return Err(AppError::new("core.pathDenied")
            .with_detail(format!("refusing filesystem root: {}", target.display())));
    }

    for root in roots {
        let root = absolutize(root);
        if target.starts_with(&root) {
            return Ok(target);
        }
    }

    Err(AppError::new("core.pathDenied").with_detail(format!(
        "{} is outside the allowed directories",
        target.display()
    )))
}

/// Convenience wrapper for commands that hold a `SettingsStore` snapshot.
pub fn ensure_allowed(path: &Path, settings: &Settings) -> Result<PathBuf, AppError> {
    ensure_within_roots(path, &allowed_roots(settings))
}

/// Validates a batch of paths, failing on the first escape.
pub fn ensure_all_allowed(
    paths: &[PathBuf],
    settings: &Settings,
) -> Result<Vec<PathBuf>, AppError> {
    let roots = allowed_roots(settings);
    paths
        .iter()
        .map(|p| ensure_within_roots(p, &roots))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_paths_inside_roots_and_rejects_escapes() {
        let base = std::env::temp_dir().join("mclr-root-test");
        let inside = base.join("mods").join("example.jar");
        let roots = vec![base.clone()];

        assert!(ensure_within_roots(&inside, &roots).is_ok());

        // Traversal out of the root, absolute foreign paths and drive roots.
        assert!(ensure_within_roots(&base.join("../evil.jar"), &roots).is_err());
        assert!(
            ensure_within_roots(Path::new("C:/Windows/System32/drivers/etc/hosts"), &roots)
                .is_err()
        );
        assert!(ensure_within_roots(Path::new("C:/"), &roots).is_err());
        assert!(ensure_within_roots(Path::new(""), &roots).is_err());
    }

    #[test]
    fn read_tier_rejects_system_paths_only() {
        // Dialogs may hand us files from anywhere the user owns.
        assert!(
            ensure_not_system_path(Path::new("C:/Users/someone/Downloads/pack.mrpack")).is_ok()
        );
        // System locations and bare drives are refused.
        assert!(ensure_not_system_path(Path::new("C:/Windows/System32/config/SAM")).is_err());
        assert!(ensure_not_system_path(Path::new("C:/Program Files/secret.dll")).is_err());
        assert!(ensure_not_system_path(Path::new("C:/")).is_err());
        assert!(ensure_not_system_path(Path::new("")).is_err());
    }

    #[test]
    fn settings_roots_include_data_dir_and_game_folders() {
        let mut settings = Settings::default();
        settings.game.folders.push(mclr_core::GameFolderEntry {
            name: "demo".to_string(),
            path: "C:/mc/demo".to_string(),
        });
        settings.game.current_folder = "C:/mc/current".to_string();

        let roots = allowed_roots(&settings);
        assert!(
            roots
                .iter()
                .any(|r| r.ends_with("MCLR") || r.ends_with("mclr"))
        );
        assert!(
            roots
                .iter()
                .any(|r| r.to_string_lossy().contains("C:/mc/demo"))
        );
        assert!(
            roots
                .iter()
                .any(|r| r.to_string_lossy().contains("C:/mc/current"))
        );
    }
}
