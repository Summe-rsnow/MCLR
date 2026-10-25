#![allow(clippy::collapsible_if)]
//! Tauri application library for MCLR.

pub mod commands;
pub mod events;
pub mod http;
pub mod ipc;

use mclr_auth::AccountStore;
use mclr_core::{SettingsStore, TaskCenter, TaskNode, init_logging, paths};
use mclr_game::LaunchWatcher;
use std::sync::Arc;
use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = commands::builder();

    // `main` has no tokio runtime and these stores spawn tokio tasks on construction.
    // Enter Tauri's global async runtime (the same one async commands run on).
    let (settings_store, account_store, initial_settings) = tauri::async_runtime::block_on(async {
        let settings_store = SettingsStore::open().expect("failed to open settings store");
        let account_store = AccountStore::open().expect("failed to open account store");
        let initial_settings = settings_store.get().await;
        (settings_store, account_store, initial_settings)
    });
    let log_level = initial_settings.system.log_level.clone();
    let log_retention = initial_settings.system.log_retention_days;
    let http_state =
        http::HttpState::new(&initial_settings.net).expect("failed to build HTTP client");
    let account_store = Arc::new(account_store);
    let task_center = Arc::new(TaskCenter::new());
    let launch_watcher = Arc::new(LaunchWatcher::new());

    // Initialize logging according to settings
    let _logging_guard = init_logging(&paths::logs_dir(), &log_level, log_retention);

    let tc_for_events = Arc::clone(&task_center);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if let Some(pos) = args.iter().position(|a| a == "--launch")
                && let Some(inst_id) = args.get(pos + 1)
            {
                let _ = app.emit("cli://launch", inst_id.clone());
            }
        }))
        .manage(settings_store)
        .manage(http_state)
        .manage(task_center)
        .manage(account_store)
        .manage(launch_watcher)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            ipc::assets::allow_defaults(app.handle(), &initial_settings);
            let startup_mc = if initial_settings.game.current_folder.is_empty() {
                paths::default_minecraft_dir()
            } else {
                std::path::PathBuf::from(&initial_settings.game.current_folder)
            };
            tauri::async_runtime::spawn(async move {
                let _ = tokio::task::spawn_blocking(move || {
                    mclr_game::install::cleanup_stale(&startup_mc);
                })
                .await;
            });

            // Forward task center updates to Tauri event "tasks://update"
            let app_handle = app.handle().clone();
            // Coalesce per task id (latest node wins) and emit at most once per 50 ms window.
            let batcher = events::spawn_default(move |batch: Vec<Vec<TaskNode>>| {
                let mut latest: Vec<TaskNode> = Vec::new();
                for node in batch.into_iter().flatten() {
                    match latest.iter_mut().find(|n| n.id == node.id) {
                        Some(slot) => *slot = node,
                        None => latest.push(node),
                    }
                }
                let _ = app_handle.emit("tasks://update", latest);
            });
            let mut task_rx = tc_for_events.subscribe();
            tauri::async_runtime::spawn(async move {
                while let Ok(diff) = task_rx.recv().await {
                    batcher.push(diff);
                }
            });
            let startup_args: Vec<String> = std::env::args().collect();
            if let Some(pos) = startup_args.iter().position(|a| a == "--launch")
                && let Some(inst_id) = startup_args.get(pos + 1).cloned()
            {
                let app_handle2 = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let _ = app_handle2.emit("cli://launch", inst_id);
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running MCLR application");
}
