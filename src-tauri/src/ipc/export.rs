//! Modpack export IPC commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mclr_comp::{
    CompError, CurseForgeClient, ExportCandidateEntry, ModpackExportOptions, ModrinthClient,
    export_modpack as do_export_modpack, scan_export_candidates,
};
use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_game::InstanceConfig;
use tauri::State;

async fn instance_dir_path(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
    let settings = state.get().await;
    let mc_folder = if settings.game.current_folder.is_empty() {
        mclr_core::paths::default_minecraft_dir()
    } else {
        PathBuf::from(&settings.game.current_folder)
    };
    let inst_dir = mc_folder.join("versions").join(instance_id);
    let config_path = inst_dir.join("mclr.instance.json");
    let instance_config = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| serde_json::from_str::<InstanceConfig>(&content).ok())
            .unwrap_or_default()
    } else {
        InstanceConfig::default()
    };

    if instance_config.isolation == "on"
        || (instance_config.isolation == "global" && settings.game.default_isolation == "on")
    {
        inst_dir
    } else {
        mc_folder
    }
}

#[tauri::command]
#[specta::specta]
pub async fn export_scan_instance(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<ExportCandidateEntry>, AppError> {
    let inst_dir = instance_dir_path(&state, &instance_id).await;
    if !inst_dir.exists() {
        return Err(CompError::FileNotFound(inst_dir.display().to_string()).into());
    }

    scan_export_candidates(&inst_dir).map_err(AppError::from)
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
