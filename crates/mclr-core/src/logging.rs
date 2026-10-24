//! Logging initialization and retention management for MCLR.
//!
//! Writes rolling daily logs to `<logs_dir>/mclr-<YYYY-MM-DD>.log` and prunes
//! older log files keeping only the newest `retention_days` files.

use std::fs;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Prunes old log files in `logs_dir`, keeping the newest `retention_days` files matching `mclr-*.log`.
pub fn prune_old_logs(logs_dir: &Path, retention_days: u32) -> std::io::Result<()> {
    if !logs_dir.exists() {
        return Ok(());
    }

    let mut log_files: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(logs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            && file_name.starts_with("mclr-")
            && file_name.ends_with(".log")
        {
            log_files.push(path);
        }
    }

    // Sort ascending by file name (mclr-YYYY-MM-DD.log sorts chronologically)
    log_files.sort();

    let keep = retention_days as usize;
    if log_files.len() > keep {
        let to_remove_count = log_files.len() - keep;
        for path in log_files.iter().take(to_remove_count) {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

/// Initializes tracing subscriber with daily rolling file appender and stdout formatting.
/// Returns a WorkerGuard that flushes logs on drop.
pub fn init_logging(
    logs_dir: &Path,
    level: &str,
    retention_days: u32,
) -> Result<WorkerGuard, Box<dyn std::error::Error + Send + Sync>> {
    fs::create_dir_all(logs_dir)?;
    let _ = prune_old_logs(logs_dir, retention_days);

    let file_appender = tracing_appender::rolling::daily(logs_dir, "mclr");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);

    let stdout_layer = fmt::layer().with_ansi(true).with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer)
        .try_init()?;

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prune_old_logs() {
        let temp = tempfile::tempdir().unwrap();
        let logs_path = temp.path();

        // Create 10 fake log files
        for day in 1..=10 {
            let filename = format!("mclr-2026-10-{:02}.log", day);
            fs::write(logs_path.join(filename), "log content").unwrap();
        }
        // And an unrelated file
        fs::write(logs_path.join("unrelated.txt"), "keep").unwrap();

        prune_old_logs(logs_path, 7).unwrap();

        let mut remaining = Vec::new();
        for entry in fs::read_dir(logs_path).unwrap() {
            remaining.push(entry.unwrap().file_name().into_string().unwrap());
        }
        remaining.sort();

        // Should keep 7 newest logs + unrelated.txt = 8 files
        assert_eq!(remaining.len(), 8);
        assert!(remaining.contains(&"unrelated.txt".to_string()));
        assert!(!remaining.contains(&"mclr-2026-10-01.log".to_string()));
        assert!(!remaining.contains(&"mclr-2026-10-02.log".to_string()));
        assert!(!remaining.contains(&"mclr-2026-10-03.log".to_string()));
        assert!(remaining.contains(&"mclr-2026-10-04.log".to_string()));
        assert!(remaining.contains(&"mclr-2026-10-10.log".to_string()));
    }
}
