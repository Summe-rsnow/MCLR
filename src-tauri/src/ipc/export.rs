//! Modpack export IPC commands.

use std::path::PathBuf;
use std::sync::Arc;

use mclr_comp::{CompError, ExportCandidateEntry, ModpackExportOptions, scan_export_candidates};
use mclr_core::{AppError, SettingsStore};
use tauri::State;

async fn instance_dir_path(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
    super::game_dir_for(state, instance_id).await
}

#[tauri::command]
#[specta::specta]
pub async fn export_scan_instance(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<ExportCandidateEntry>, AppError> {
    let inst_dir = instance_dir_path(&state, &instance_id).await;
    super::run_blocking(move || {
        if !inst_dir.exists() {
            return Err(CompError::FileNotFound(inst_dir.display().to_string()));
        }
        scan_export_candidates(&inst_dir)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn export_modpack(
    state: State<'_, SettingsStore>,
    engine: State<'_, Arc<mclr_core::tasks::TaskEngine>>,
    options: ModpackExportOptions,
) -> Result<String, AppError> {
    let inst_dir = instance_dir_path(&state, &options.instance_id).await;
    if !inst_dir.exists() {
        return Err(CompError::FileNotFound(inst_dir.display().to_string()).into());
    }

    engine
        .submit(
            "export.modpack",
            serde_json::to_value(crate::tasks::handlers::export::ExportModpackPayload { options })?,
        )
        .await
}
