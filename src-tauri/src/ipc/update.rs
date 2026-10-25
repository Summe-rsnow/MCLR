//! Update and Launcher Logs IPC commands.

use mclr_core::{AppError, SettingsStore, TaskCenter, is_launcher_log_file, paths};
use mclr_update::{UpdateCheckResult, UpdateConfig, check_update_manifest};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_updater::UpdaterExt;

/// Information about a launcher log file.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LauncherLogFile {
    pub file_name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: String,
}

#[tauri::command]
#[specta::specta]
pub async fn update_check(
    _app: AppHandle,
    settings_state: State<'_, SettingsStore>,
    channel_override: Option<String>,
) -> Result<UpdateCheckResult, AppError> {
    let settings = settings_state.get().await;
    let channel = channel_override
        .filter(|c| !c.trim().is_empty())
        .unwrap_or(settings.update.channel);

    let config = UpdateConfig::from_runtime(&channel)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::new("update.http").with_detail(e.to_string()))?;

    let current_version = env!("CARGO_PKG_VERSION");
    let result = check_update_manifest(&client, &config, current_version).await?;
    Ok(result)
}

#[tauri::command]
#[specta::specta]
pub async fn update_install(
    app: AppHandle,
    settings_state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    channel_override: Option<String>,
) -> Result<String, AppError> {
    let settings = settings_state.get().await;
    let channel = channel_override
        .filter(|c| !c.trim().is_empty())
        .unwrap_or(settings.update.channel);

    let config = UpdateConfig::from_runtime(&channel)?;

    // Build the updater from runtime config
    let updater = app
        .updater_builder()
        .endpoints(vec![config.endpoint.clone()])
        .map_err(|e| AppError::new("update.invalidUrl").with_detail(e.to_string()))?
        .pubkey(config.pubkey.clone())
        .build()
        .map_err(|e| AppError::new("update.http").with_detail(e.to_string()))?;

    let mut params = HashMap::new();
    params.insert("channel".to_string(), channel);
    let (task_id, _token) = tasks_state
        .create_task(None, "task.update.download", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let task_id_clone = task_id.clone();

    tauri::async_runtime::spawn(async move {
        let _ = tc.start_task(&task_id_clone).await;

        let check_res = match updater.check().await {
            Ok(res) => res,
            Err(e) => {
                let _ = tc
                    .fail_task(
                        &task_id_clone,
                        AppError::new("update.http").with_detail(e.to_string()),
                    )
                    .await;
                return;
            }
        };

        let Some(update) = check_res else {
            let _ = tc.complete_task(&task_id_clone).await;
            return;
        };

        let mut downloaded_bytes: u64 = 0;
        let tc_progress = tc.clone();
        let tid = task_id_clone.clone();

        let download_res = update
            .download_and_install(
                move |chunk_length, content_length| {
                    downloaded_bytes += chunk_length as u64;
                    if let Some(total) = content_length
                        && total > 0
                    {
                        let progress = (downloaded_bytes as f32 / total as f32).clamp(0.0, 1.0);
                        let tc_clone = tc_progress.clone();
                        let t = tid.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = tc_clone.update_progress(&t, progress, 0).await;
                        });
                    }
                },
                || {},
            )
            .await;

        match download_res {
            Ok(()) => {
                let _ = tc.complete_task(&task_id_clone).await;
            }
            Err(e) => {
                let _ = tc
                    .fail_task(
                        &task_id_clone,
                        AppError::new("update.install").with_detail(e.to_string()),
                    )
                    .await;
            }
        }
    });
    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn launcher_logs_list() -> Result<Vec<LauncherLogFile>, AppError> {
    super::run_blocking(|| {
        let logs_dir = paths::logs_dir();
        if !logs_dir.exists() {
            return Ok::<_, AppError>(Vec::new());
        }

        let read_dir = fs::read_dir(&logs_dir)
            .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))?;

        let mut files = Vec::new();

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();
                if is_launcher_log_file(&file_name) {
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let modified_at = metadata
                        .and_then(|m| m.modified().ok())
                        .map(|time| {
                            let datetime: chrono::DateTime<chrono::Utc> = time.into();
                            datetime.to_rfc3339()
                        })
                        .unwrap_or_default();

                    files.push(LauncherLogFile {
                        file_name,
                        path: path.to_string_lossy().to_string(),
                        size,
                        modified_at,
                    });
                }
            }
        }

        // Sort newest first by modified_at or file_name descending
        files.sort_by(|a, b| b.file_name.cmp(&a.file_name));

        Ok(files)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn launcher_log_read(file_name: String) -> Result<String, AppError> {
    // Validate file_name does not contain directory separators or parent traversal
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(AppError::new("core.io").with_detail("Invalid log file name"));
    }

    super::run_blocking(move || {
        let logs_dir = paths::logs_dir();
        let file_path = logs_dir.join(&file_name);

        if !file_path.exists() || !file_path.is_file() {
            return Err(
                AppError::new("core.io").with_detail(format!("Log file not found: {}", file_name))
            );
        }

        let content_bytes = fs::read(&file_path)
            .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))?;

        // Keep last 200 KB if file is larger
        const MAX_BYTES: usize = 200 * 1024;
        let slice = if content_bytes.len() > MAX_BYTES {
            &content_bytes[content_bytes.len() - MAX_BYTES..]
        } else {
            &content_bytes[..]
        };

        let text = String::from_utf8_lossy(slice).to_string();
        Ok(text)
    })
    .await
}
