//! Game launch and process watch IPC commands.

use mclr_auth::AccountStore;
use mclr_core::account_session::AccountSessionProvider;
use mclr_core::{AppError, SettingsStore, TaskCenter};
pub use mclr_game::launch::{
    LaunchExitPayload, LaunchProcessInfo, LaunchStartResult, LaunchWatcher, LogLevel, LogLine,
};
use mclr_game::launch::{LaunchPipelineParams, WatcherEvent, launch as game_launch};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

use super::current_mc_folder;

#[tauri::command]
#[specta::specta]
pub async fn launch_instance(
    http: State<'_, crate::http::HttpState>,
    app: AppHandle,
    settings_state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    watcher_state: State<'_, Arc<LaunchWatcher>>,
    instance_id: String,
    account_id: String,
) -> Result<LaunchStartResult, AppError> {
    let accounts_state = app.state::<Arc<AccountStore>>();
    let settings = settings_state.get().await;
    let client = http.client();
    let mc_folder = current_mc_folder(&settings_state).await;

    // 1. Resolve account session
    let session = accounts_state.resolve_session(&account_id).await?;

    // 2. Create long-running task
    let mut params = HashMap::new();
    params.insert("instanceId".to_string(), instance_id.clone());
    params.insert("username".to_string(), session.username.clone());

    let (task_id, _token) = tasks_state.create_task(None, "task.launch", params).await;

    let _ = tasks_state.start_task(&task_id).await;

    let tc = Arc::clone(&tasks_state);
    let watcher = Arc::clone(&watcher_state);

    // 3. Run launch preparation and spawn
    let launch_res = game_launch(LaunchPipelineParams {
        client: &client,
        settings: &settings,
        mc_folder: &mc_folder,
        instance_id: &instance_id,
        session: &session,
        task_center: Some((&tc, &task_id)),
        watcher: &watcher,
    })
    .await;

    match launch_res {
        Ok(result) => {
            if result.process_id.is_some() {
                // Feed the instance list sort "by last played": the launch actually happened.
                let _ = mclr_game::instance::touch_instance_last_played(
                    &mc_folder,
                    &instance_id,
                    &chrono::Utc::now().to_rfc3339(),
                );
            }
            if let Some(pid) = result.process_id {
                let mut rx = watcher.subscribe();
                let app_handle = app.clone();
                let target_pid = pid;
                let target_instance_id = instance_id.clone();

                let log_batcher = crate::events::spawn_default({
                    let app_handle = app_handle.clone();
                    let inst_id = target_instance_id.clone();
                    move |batch: Vec<LogLine>| {
                        let _ = app_handle.emit(
                            "launch://logBatch",
                            &serde_json::json!({
                                "instanceId": inst_id,
                                "lines": batch,
                            }),
                        );
                    }
                });

                tokio::spawn(async move {
                    while let Ok(evt) = rx.recv().await {
                        match evt {
                            WatcherEvent::Started(info) if info.process_id == target_pid => {
                                let _ = app_handle.emit("launch://started", &info);
                            }
                            WatcherEvent::Log(log) if log.process_id == target_pid => {
                                // Only the batcher emits: sending both `launch://log`
                                // and `launch://logBatch` made every line appear twice.
                                log_batcher.push(log);
                            }
                            WatcherEvent::Crashed(payload) if payload.process_id == target_pid => {
                                let _ = app_handle.emit("launch://crashed", &payload);
                            }
                            WatcherEvent::Exited(payload) if payload.process_id == target_pid => {
                                let _ = app_handle.emit("launch://exit", &payload);
                                break;
                            }
                            _ => {}
                        }
                    }
                });
            }

            Ok(result)
        }
        Err(e) => {
            let app_err = AppError::from(e);
            let _ = tc.fail_task(&task_id, app_err.clone()).await;
            Err(app_err)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn launch_logs(
    watcher_state: State<'_, Arc<LaunchWatcher>>,
    instance_id: Option<String>,
) -> Result<Vec<LogLine>, AppError> {
    Ok(watcher_state.get_logs(instance_id.as_deref()).await)
}

#[tauri::command]
#[specta::specta]
pub async fn launch_processes(
    watcher_state: State<'_, Arc<LaunchWatcher>>,
) -> Result<Vec<LaunchProcessInfo>, AppError> {
    Ok(watcher_state.list_processes().await)
}

#[tauri::command]
#[specta::specta]
pub async fn launch_kill(
    watcher_state: State<'_, Arc<LaunchWatcher>>,
    process_id: u32,
) -> Result<(), AppError> {
    watcher_state.kill(process_id).await.map_err(AppError::from)
}
