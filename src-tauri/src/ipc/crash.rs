//! Crash analysis and report export IPC commands.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mclr_core::{AppError, SettingsStore};
pub use mclr_crash::{CrashCauseFinding, CrashConfidence, CrashResult};
use mclr_game::LaunchWatcher;
use tauri::State;

async fn resolve_instance_dirs(
    state: &State<'_, SettingsStore>,
    instance_id: &str,
) -> (PathBuf, PathBuf) {
    let mc_folder = super::current_mc_folder(state).await;
    let instance_dir = mc_folder.join("versions").join(instance_id);
    let game_dir = super::game_dir_for(state, instance_id).await;
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

    super::run_blocking(move || {
        let custom_path = custom_log_path.as_deref().map(Path::new);
        let logs = mclr_crash::collect_crash_logs(
            &instance_dir,
            Some(&game_dir),
            &watcher_lines,
            custom_path,
        )?;
        Ok::<_, mclr_crash::CrashError>(mclr_crash::analyze_crash(
            &instance_id,
            &instance_dir,
            &logs,
        ))
    })
    .await
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

    super::run_blocking(move || {
        let out_path = mclr_crash::export_crash_report_zip(
            &instance_dir,
            &instance_id,
            &watcher_lines,
            Path::new(&dest_zip_path),
        )?;
        Ok::<_, mclr_crash::CrashError>(out_path.to_string_lossy().to_string())
    })
    .await
}
