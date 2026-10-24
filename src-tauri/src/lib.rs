//! Tauri application library for MCLR.

pub mod commands;
pub mod ipc;

use mclr_core::{SettingsStore, TaskCenter, init_logging, paths};
use std::sync::Arc;
use tauri::Emitter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = commands::builder();

    // Initialize core settings
    let settings_store = SettingsStore::open().expect("failed to open settings store");
    let task_center = Arc::new(TaskCenter::new());

    // Initialize logging according to settings
    let (log_level, log_retention) = {
        // Safe blocking read on startup before runtime begins
        let initial_settings = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(settings_store.get())
        });
        (
            initial_settings.system.log_level,
            initial_settings.system.log_retention_days,
        )
    };
    let _logging_guard = init_logging(&paths::logs_dir(), &log_level, log_retention);

    let tc_for_events = Arc::clone(&task_center);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .manage(settings_store)
        .manage(task_center)
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);

            // Forward task center updates to Tauri event "tasks://update"
            let app_handle = app.handle().clone();
            let mut task_rx = tc_for_events.subscribe();
            tauri::async_runtime::spawn(async move {
                while let Ok(diff) = task_rx.recv().await {
                    let _ = app_handle.emit("tasks://update", diff);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running MCLR application");
}
