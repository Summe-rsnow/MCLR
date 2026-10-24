//! Game launch and process watch IPC commands.

use mclr_auth::AccountStore;
use mclr_core::account_session::AccountSessionProvider;
use mclr_core::{AppError, SettingsStore, TaskCenter};
pub use mclr_game::launch::{
    LaunchExitPayload, LaunchProcessInfo, LaunchStartResult, LaunchWatcher, LogLevel, LogLine,
};
use mclr_game::launch::{WatcherEvent, launch as game_launch};
use mclr_net::build_http_client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

async fn current_mc_folder(state: &State<'_, SettingsStore>) -> PathBuf {
    let settings = state.get().await;
    if settings.game.current_folder.is_empty() {
        mclr_core::paths::default_minecraft_dir()
    } else {
        PathBuf::from(settings.game.current_folder)
    }
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn launch_instance(
    app: AppHandle,
    settings_state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    accounts_state: State<'_, Arc<AccountStore>>,
    watcher_state: State<'_, Arc<LaunchWatcher>>,
    instance_id: String,
    account_id: String,
) -> Result<LaunchStartResult, AppError> {
    let settings = settings_state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
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
    let launch_res = game_launch(
        &client,
        &settings,
        &mc_folder,
        &instance_id,
        &session,
        Some((&tc, &task_id)),
        &watcher,
    )
    .await;

    match launch_res {
        Ok(result) => {
            if let Some(pid) = result.process_id {
                let mut rx = watcher.subscribe();
                let app_handle = app.clone();
                let target_pid = pid;

                tokio::spawn(async move {
                    while let Ok(evt) = rx.recv().await {
                        match evt {
                            WatcherEvent::Started(info) if info.process_id == target_pid => {
                                let _ = app_handle.emit("launch://started", &info);
                            }
                            WatcherEvent::Log(log) if log.process_id == target_pid => {
                                let _ = app_handle.emit("launch://log", &log);
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
