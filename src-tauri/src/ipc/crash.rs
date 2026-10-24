//! Crash analysis and report export IPC commands.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mclr_core::{AppError, SettingsStore};
pub use mclr_crash::{CrashCauseFinding, CrashConfidence, CrashResult};
use mclr_game::{InstanceConfig, LaunchWatcher};
use tauri::State;

async fn resolve_instance_dirs(
    state: &State<'_, SettingsStore>,
    instance_id: &str,
) -> (PathBuf, PathBuf) {
    let settings = state.get().await;
    let mc_folder = if settings.game.current_folder.is_empty() {
        mclr_core::paths::default_minecraft_dir()
    } else {
        PathBuf::from(&settings.game.current_folder)
    };
    let instance_dir = mc_folder.join("versions").join(instance_id);
    let config_path = instance_dir.join("mclr.instance.json");
    let instance_config = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| serde_json::from_str::<InstanceConfig>(&content).ok())
            .unwrap_or_default()
    } else {
        InstanceConfig::default()
    };
    let game_dir = if instance_config.isolation == "on"
        || (instance_config.isolation == "global" && settings.game.default_isolation == "on")
    {
        instance_dir.clone()
    } else {
        mc_folder.clone()
    };
    (instance_dir, game_dir)
}

#[tauri::command]
#[specta::specta]
pub async fn crash_analyze(
    settings_state: State<'_, SettingsStore>,
    watcher_state: State<'_, Arc<LaunchWatcher>>,
    instance_id: String,
    custom_log_path: Option<String>,
) -> Result<CrashResult, AppError> {
    let (instance_dir, game_dir) = resolve_instance_dirs(&settings_state, &instance_id).await;
    let watcher_logs = watcher_state.get_logs(Some(&instance_id)).await;
    let watcher_lines: Vec<String> = watcher_logs.into_iter().map(|l| l.text).collect();

    let custom_path = custom_log_path.as_deref().map(Path::new);
    let logs =
        mclr_crash::collect_crash_logs(&instance_dir, Some(&game_dir), &watcher_lines, custom_path)
            .map_err(AppError::from)?;

    let result = mclr_crash::analyze_crash(&instance_id, &instance_dir, &logs);
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn crash_export_report(
    settings_state: State<'_, SettingsStore>,
    watcher_state: State<'_, Arc<LaunchWatcher>>,
    instance_id: String,
    dest_zip_path: String,
) -> Result<String, AppError> {
    let (instance_dir, _game_dir) = resolve_instance_dirs(&settings_state, &instance_id).await;
    let watcher_logs = watcher_state.get_logs(Some(&instance_id)).await;
    let watcher_lines: Vec<String> = watcher_logs.into_iter().map(|l| l.text).collect();

    let out_path = mclr_crash::export_crash_report_zip(
        &instance_dir,
        &instance_id,
        &watcher_lines,
        Path::new(&dest_zip_path),
    )
    .map_err(AppError::from)?;

    Ok(out_path.to_string_lossy().to_string())
}
