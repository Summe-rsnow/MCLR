//! Modpack export IPC commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mclr_comp::{
    CompError, CurseForgeClient, ExportCandidateEntry, ModpackExportOptions, ModrinthClient,
    export_modpack as do_export_modpack, scan_export_candidates,
};
use mclr_core::{AppError, SettingsStore, TaskCenter};
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
    tasks_state: State<'_, Arc<TaskCenter>>,
    options: ModpackExportOptions,
) -> Result<String, AppError> {
    let inst_dir = instance_dir_path(&state, &options.instance_id).await;
    if !inst_dir.exists() {
        return Err(CompError::FileNotFound(inst_dir.display().to_string()).into());
    }

    let settings = state.get().await;
    let mr_client = ModrinthClient::new(settings.net.source_priority);
    let cf_client = CurseForgeClient::new(settings.net.source_priority);

    let mut params = HashMap::new();
    params.insert("instanceId".to_string(), options.instance_id.clone());
    params.insert("packName".to_string(), options.pack_name.clone());
    params.insert("outputPath".to_string(), options.output_path.clone());

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "task.export.title", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();
    let opts = options.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match do_export_modpack(
            &inst_dir,
            &opts,
            Some(&mr_client),
            Some(&cf_client),
            Some((&tc, &tid)),
        )
        .await
        {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, e.into()).await;
            }
        }
    });

    Ok(task_id)
}
