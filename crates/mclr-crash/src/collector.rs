//! Crash log collection and classification.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::CrashError;
use crate::preparer::prepare_text;
use crate::types::{CollectedLogEntry, CrashLogKind, CrashLogSet};

/// Finds all log candidates under the instance directory (and parent directory if not isolated).
pub fn collect_crash_logs(
    instance_dir: &Path,
    game_dir: Option<&Path>,
    watcher_lines: &[String],
    custom_log_path: Option<&Path>,
) -> Result<CrashLogSet, CrashError> {
    let mut log_set = CrashLogSet::default();
    let mut all_texts = Vec::new();

    // 1. If custom log path is supplied, read it first
    if let Some(custom_path) = custom_log_path
        && custom_path.exists()
        && let Ok(content) = fs::read_to_string(custom_path)
    {
        let prepared = prepare_text(&content);
        let lines: Vec<String> = prepared.lines().map(|s| s.to_string()).collect();
        all_texts.push(prepared.clone());
        log_set.all_paths.push(custom_path.to_path_buf());
        log_set.custom = Some(CollectedLogEntry {
            path: custom_path.to_path_buf(),
            kind: CrashLogKind::Custom,
            lines,
            text: prepared,
        });
    }

    // 2. Collect crash reports: instance_dir/crash-reports/*.txt (and game_dir/crash-reports/*.txt)
    let mut crash_report_files = find_files(instance_dir.join("crash-reports"), "txt");
    if let Some(gd) = game_dir
        && gd != instance_dir
    {
        crash_report_files.extend(find_files(gd.join("crash-reports"), "txt"));
    }
    // Sort newest first
    crash_report_files.sort_by(|a, b| {
        let mtime_a = fs::metadata(a).and_then(|m| m.modified()).ok();
        let mtime_b = fs::metadata(b).and_then(|m| m.modified()).ok();
        mtime_b.cmp(&mtime_a)
    });

    if let Some(newest_crash_report) = crash_report_files.first()
        && let Ok(content) = fs::read_to_string(newest_crash_report)
    {
        let prepared = prepare_text(&content);
        let lines: Vec<String> = prepared.lines().map(|s| s.to_string()).collect();
        all_texts.push(prepared.clone());
        log_set.all_paths.push(newest_crash_report.clone());
        log_set.crash_report = Some(CollectedLogEntry {
            path: newest_crash_report.clone(),
            kind: CrashLogKind::CrashReport,
            lines,
            text: prepared,
        });
    }

    // 3. Collect JVM error logs: hs_err_pid*.log in instance_dir or game_dir
    let mut hs_err_files = find_hs_err_files(instance_dir);
    if let Some(gd) = game_dir
        && gd != instance_dir
    {
        hs_err_files.extend(find_hs_err_files(gd));
    }
    hs_err_files.sort_by(|a, b| {
        let mtime_a = fs::metadata(a).and_then(|m| m.modified()).ok();
        let mtime_b = fs::metadata(b).and_then(|m| m.modified()).ok();
        mtime_b.cmp(&mtime_a)
    });

    if let Some(newest_hs_err) = hs_err_files.first()
        && let Ok(content) = fs::read_to_string(newest_hs_err)
    {
        let prepared = prepare_text(&content);
        let lines: Vec<String> = prepared.lines().map(|s| s.to_string()).collect();
        all_texts.push(prepared.clone());
        log_set.all_paths.push(newest_hs_err.clone());
        log_set.hs_err = Some(CollectedLogEntry {
            path: newest_hs_err.clone(),
            kind: CrashLogKind::HsErr,
            lines,
            text: prepared,
        });
    }

    // 4. Collect logs/latest.log
    let latest_log_path = instance_dir.join("logs").join("latest.log");
    let fallback_latest = game_dir.map(|g| g.join("logs").join("latest.log"));
    let chosen_latest = if latest_log_path.exists() {
        Some(latest_log_path)
    } else if let Some(p) = &fallback_latest
        && p.exists()
    {
        Some(p.clone())
    } else {
        None
    };

    if let Some(latest_path) = chosen_latest
        && let Ok(content) = fs::read_to_string(&latest_path)
    {
        let prepared = prepare_text(&content);
        let lines: Vec<String> = prepared.lines().map(|s| s.to_string()).collect();
        all_texts.push(prepared.clone());
        log_set.all_paths.push(latest_path.clone());
        log_set.game = Some(CollectedLogEntry {
            path: latest_path,
            kind: CrashLogKind::Game,
            lines,
            text: prepared,
        });
    }

    // 5. If watcher lines are available, append or use as Game log if latest.log was empty/missing
    if !watcher_lines.is_empty() {
        let watcher_text = watcher_lines.join("\n");
        let prepared_watcher = prepare_text(&watcher_text);
        if log_set.game.is_none() {
            let lines: Vec<String> = prepared_watcher.lines().map(|s| s.to_string()).collect();
            all_texts.push(prepared_watcher.clone());
            log_set.game = Some(CollectedLogEntry {
                path: instance_dir.join("watcher-output.log"),
                kind: CrashLogKind::Game,
                lines,
                text: prepared_watcher,
            });
        } else {
            all_texts.push(prepared_watcher);
        }
    }

    // 6. Collect logs/debug.log
    let debug_log_path = instance_dir.join("logs").join("debug.log");
    let fallback_debug = game_dir.map(|g| g.join("logs").join("debug.log"));
    let chosen_debug = if debug_log_path.exists() {
        Some(debug_log_path)
    } else if let Some(p) = &fallback_debug
        && p.exists()
    {
        Some(p.clone())
    } else {
        None
    };

    if let Some(debug_path) = chosen_debug
        && let Ok(content) = fs::read_to_string(&debug_path)
    {
        let prepared = prepare_text(&content);
        let lines: Vec<String> = prepared.lines().map(|s| s.to_string()).collect();
        all_texts.push(prepared.clone());
        log_set.all_paths.push(debug_path.clone());
        log_set.debug = Some(CollectedLogEntry {
            path: debug_path,
            kind: CrashLogKind::Debug,
            lines,
            text: prepared,
        });
    }

    log_set.all = all_texts.join("\n\n");
    Ok(log_set)
}

fn find_files(dir: PathBuf, ext: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && let Some(e) = p.extension()
                && e.to_string_lossy().eq_ignore_ascii_case(ext)
            {
                files.push(p);
            }
        }
    }
    files
}

fn find_hs_err_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && let Some(name) = p.file_name()
            {
                let name_str = name.to_string_lossy().to_ascii_lowercase();
                if name_str.starts_with("hs_err_pid") && name_str.ends_with(".log") {
                    files.push(p);
                }
            }
        }
    }
    files
}
