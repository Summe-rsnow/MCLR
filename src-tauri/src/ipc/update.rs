//! Update and Launcher Logs IPC commands.

use mclr_core::{AppError, SettingsStore, TaskCenter, is_launcher_log_file, paths};
use mclr_update::{UpdateCheckResult, UpdateConfig, check_update_manifest};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
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
    let settings = settings_state.snapshot().await;
    let channel = channel_override
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| settings.update.channel.clone());

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::new("update.http").with_detail(e.to_string()))?;

    let current_version = env!("CARGO_PKG_VERSION");
    // Launcher self-update stays a placeholder for now: it only works when the
    // build was shipped with signed-updater secrets. The UI shows "敬请期待"
    // instead of a failure when they are missing.
    let config = UpdateConfig::from_runtime(&channel)?;
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

        read_log_tail(&file_path)
    })
    .await
}

/// Maximum number of trailing bytes returned by [`read_log_tail`].
const LAUNCHER_LOG_MAX_BYTES: usize = 200 * 1024;

/// Reads at most the last [`LAUNCHER_LOG_MAX_BYTES`] bytes of `path` and decodes them lossily.
///
/// Only the tail is read from disk, so a multi-megabyte log does not have to be
/// loaded in full. Seeking may land the window in the middle of a multi-byte UTF-8
/// character; `from_utf8_lossy` replaces that partial sequence with `U+FFFD`,
/// which is the same decoding behaviour the previous full-read implementation used.
fn read_log_tail(path: &Path) -> Result<String, AppError> {
    let io_err = |e: std::io::Error| AppError::new("core.io").with_detail(e.to_string());

    let mut file = fs::File::open(path).map_err(io_err)?;
    let len = file.metadata().map_err(io_err)?.len();

    if len > LAUNCHER_LOG_MAX_BYTES as u64 {
        file.seek(SeekFrom::End(-(LAUNCHER_LOG_MAX_BYTES as i64)))
            .map_err(io_err)?;
    }

    let mut buf = Vec::new();
    file.read_to_end(&mut buf).map_err(io_err)?;

    Ok(String::from_utf8_lossy(&buf).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "mclr-launcher-log-{}-{tag}-{nanos}.log",
            std::process::id()
        ))
    }

    #[test]
    fn read_log_tail_returns_only_the_trailing_200kb() {
        const TOTAL: usize = 5 * 1024 * 1024;
        let path = temp_log_path("big");

        let mut content = String::with_capacity(TOTAL + 128);
        content.push_str("HEAD-MARKER this line must not be part of the tail\n");
        while content.len() < TOTAL {
            content.push_str("12345 [main/INFO] launcher log line padding padding padding\n");
        }
        fs::write(&path, &content).unwrap();
        let file_len = fs::metadata(&path).unwrap().len();
        assert!(file_len > LAUNCHER_LOG_MAX_BYTES as u64);

        let tail = read_log_tail(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert!(
            tail.len() <= LAUNCHER_LOG_MAX_BYTES,
            "tail must be capped at 200 KB, got {}",
            tail.len()
        );
        assert_eq!(tail.len(), LAUNCHER_LOG_MAX_BYTES);
        // Content is the file's tail: a suffix of the whole file, nothing from the head.
        assert!(content.ends_with(&tail));
        assert!(!tail.contains("HEAD-MARKER"));
    }

    #[test]
    fn read_log_tail_reads_short_files_whole() {
        let path = temp_log_path("small");
        fs::write(&path, "hello\nworld\n").unwrap();

        let text = read_log_tail(&path).unwrap();
        let _ = fs::remove_file(&path);

        assert_eq!(text, "hello\nworld\n");
    }
}
