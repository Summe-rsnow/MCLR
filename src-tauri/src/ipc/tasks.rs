//! Task IPC commands and leftover cleanup.

use mclr_core::paths;
use mclr_core::tasks::{TaskCenter, TaskEngine, TaskNode, TaskStoreStatus};
use mclr_core::{AppError, SettingsStore};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::Path;
use std::sync::Arc;
use tauri::State;

const FAILED_TASK_MIN_AGE: std::time::Duration = std::time::Duration::from_secs(3600);

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskLeftovers {
    #[specta(type = u32)]
    pub bytes: u64,
    pub files: u32,
}

#[tauri::command]
#[specta::specta]
pub async fn tasks_list(state: State<'_, Arc<TaskCenter>>) -> Result<Vec<TaskNode>, AppError> {
    Ok(state.list_tasks().await)
}

/// Drops every finished task from the registry and returns the removed ids.
#[tauri::command]
#[specta::specta]
pub async fn tasks_clear_finished(
    state: State<'_, Arc<TaskCenter>>,
) -> Result<Vec<String>, AppError> {
    Ok(state.clear_finished().await)
}

#[tauri::command]
#[specta::specta]
pub async fn task_cancel(state: State<'_, Arc<TaskCenter>>, id: String) -> Result<(), AppError> {
    state.cancel_task(&id).await.map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn task_pause(state: State<'_, Arc<TaskCenter>>, id: String) -> Result<(), AppError> {
    state.pause_task(&id).await.map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn task_resume(state: State<'_, Arc<TaskCenter>>, id: String) -> Result<(), AppError> {
    state.resume_task(&id).await.map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn task_restore(engine: State<'_, Arc<TaskEngine>>, id: String) -> Result<(), AppError> {
    engine.restore(&id).await
}

#[tauri::command]
#[specta::specta]
pub async fn task_retry(engine: State<'_, Arc<TaskEngine>>, id: String) -> Result<(), AppError> {
    engine.retry(&id).await
}

#[tauri::command]
#[specta::specta]
pub async fn task_remove(engine: State<'_, Arc<TaskEngine>>, id: String) -> Result<(), AppError> {
    engine.remove(&id).await
}

#[tauri::command]
#[specta::specta]
pub async fn tasks_restore_all(
    engine: State<'_, Arc<TaskEngine>>,
) -> Result<Vec<String>, AppError> {
    Ok(engine.restore_all().await)
}

#[tauri::command]
#[specta::specta]
pub async fn tasks_reorder(
    engine: State<'_, Arc<TaskEngine>>,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    engine.reorder(ordered_ids).await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn tasks_store_status(
    tasks: State<'_, Arc<TaskCenter>>,
) -> Result<TaskStoreStatus, AppError> {
    Ok(tasks.store_status().await)
}

#[tauri::command]
#[specta::specta]
pub async fn tasks_ack_reset(tasks: State<'_, Arc<TaskCenter>>) -> Result<(), AppError> {
    tasks.ack_reset().await;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn tasks_leftovers_info(
    state: State<'_, SettingsStore>,
    tasks: State<'_, Arc<TaskCenter>>,
) -> Result<TaskLeftovers, AppError> {
    let mc_folder = super::current_mc_folder(&state).await;
    let protected = tasks.protected_paths().await;
    let (bytes, files) = tokio::task::spawn_blocking(move || {
        let items = scan_task_leftovers(&paths::cache_dir(), Some(&mc_folder), &protected);
        let bytes: u64 = items.iter().map(|(_, sz)| *sz).sum();
        (bytes, items.len() as u32)
    })
    .await
    .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))?;

    Ok(TaskLeftovers { bytes, files })
}

#[tauri::command]
#[specta::specta]
pub async fn tasks_cleanup_leftovers(
    state: State<'_, SettingsStore>,
    tasks: State<'_, Arc<TaskCenter>>,
) -> Result<TaskLeftovers, AppError> {
    if tasks.has_active_tasks().await {
        return Err(AppError::new("tools.tasksRunning").with_detail(
            "wait for or cancel running tasks before cleaning leftovers".to_string(),
        ));
    }
    let mc_folder = super::current_mc_folder(&state).await;
    let protected = tasks.protected_paths().await;
    let result = tokio::task::spawn_blocking(move || {
        let items = scan_task_leftovers(&paths::cache_dir(), Some(&mc_folder), &protected);
        let mut deleted_bytes = 0u64;
        let mut deleted_count = 0u32;
        for (path, size) in items {
            if std::fs::remove_file(&path).is_ok() {
                deleted_bytes += size;
                deleted_count += 1;
            }
        }
        TaskLeftovers {
            bytes: deleted_bytes,
            files: deleted_count,
        }
    })
    .await
    .map_err(|e| AppError::new("core.io").with_detail(e.to_string()))?;

    Ok(result)
}

fn scan_task_leftovers(
    cache_root: &Path,
    mc_folder: Option<&Path>,
    protected: &std::collections::HashSet<String>,
) -> Vec<(std::path::PathBuf, u64)> {
    let now = std::time::SystemTime::now();
    let is_stale = |path: &Path| {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|mtime| now.duration_since(mtime).ok())
            .is_some_and(|age| age >= FAILED_TASK_MIN_AGE)
    };

    let mut leftovers = Vec::new();

    // Modpack archives: a completed install deletes its own, so whatever is left is abandoned.
    let modpacks_dir = cache_root.join("modpacks");
    if let Ok(entries) = std::fs::read_dir(&modpacks_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let np = mclr_core::norm_path(&path);
            if path.is_file() && !protected.contains(&np) && is_stale(&path) {
                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                leftovers.push((path, size));
            }
        }
    }

    let mut roots = vec![cache_root.to_path_buf()];
    if let Some(mc) = mc_folder {
        for instance in mclr_game::instance::scan_instances(mc).unwrap_or_default() {
            let base = mc.join("versions").join(&instance.id);
            for sub in ["mods", "resourcepacks", "shaderpacks"] {
                roots.push(base.join(sub));
            }
        }
    }

    for root in roots {
        collect_leftover_parts(&root, &is_stale, protected, &mut leftovers, 0);
    }

    leftovers
}

fn collect_leftover_parts(
    dir: &Path,
    is_stale: &impl Fn(&Path) -> bool,
    protected: &std::collections::HashSet<String>,
    out: &mut Vec<(std::path::PathBuf, u64)>,
    depth: u32,
) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(t) if t.is_dir() => {
                collect_leftover_parts(&path, is_stale, protected, out, depth + 1)
            }
            Ok(t) if t.is_file() => {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let is_part = name.ends_with(".mclr-part") || name.ends_with(".mclr-part.meta");
                let np = mclr_core::norm_path(&path);
                if is_part && !protected.contains(&np) && is_stale(&path) {
                    let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                    out.push((path, size));
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::time::{Duration, SystemTime};

    fn write_aged(path: &Path, age: Duration) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"leftover").unwrap();
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    #[test]
    fn leftover_scan_only_reports_abandoned_task_files() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache");
        let old = Duration::from_secs(3 * 60 * 60);
        let fresh = Duration::ZERO;

        let stale_archive = cache.join("modpacks").join("abandoned.mrpack");
        let live_archive = cache.join("modpacks").join("running.mrpack");
        let stale_part = cache.join("icons").join("icon.png.mclr-part");
        let stale_meta = cache.join("icons").join("icon.png.mclr-part.meta");
        let live_part = cache.join("icons").join("other.png.mclr-part");
        let plain_cache = cache.join("icons").join("icon.png");

        write_aged(&stale_archive, old);
        write_aged(&live_archive, fresh);
        write_aged(&stale_part, old);
        write_aged(&stale_meta, old);
        write_aged(&live_part, fresh);
        write_aged(&plain_cache, old);

        let protected = HashSet::new();
        let mut found: Vec<_> = scan_task_leftovers(&cache, None, &protected)
            .into_iter()
            .map(|(p, _)| p)
            .collect();
        found.sort();

        let mut expected = vec![stale_part, stale_meta, stale_archive];
        expected.sort();
        assert_eq!(found, expected);
    }
}
