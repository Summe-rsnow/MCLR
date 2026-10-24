//! Crash analysis detection rules and pipeline coordination.

pub mod detection;
pub mod modindex;
pub mod stack;

use chrono::Utc;
use std::path::Path;

use crate::rules::detection::run_detection;
use crate::rules::modindex::CrashModIndex;
use crate::types::{CrashLogSet, CrashResult};

/// Coordinates log normalization, mod indexing, and the 4-phase detection pipeline.
pub fn analyze_crash(instance_id: &str, instance_dir: &Path, logs: &CrashLogSet) -> CrashResult {
    let mod_index = CrashModIndex::build(instance_dir, &logs.all);
    let internal_findings = run_detection(logs, &mod_index);

    let causes = internal_findings
        .into_iter()
        .map(|f| f.into_finding())
        .collect();

    // Summary lines: take up to last 50 lines of crash report, or game log, or custom log
    let summary_lines = if let Some(cr) = &logs.crash_report {
        cr.lines.iter().rev().take(50).rev().cloned().collect()
    } else if let Some(game) = &logs.game {
        game.lines.iter().rev().take(50).rev().cloned().collect()
    } else if let Some(custom) = &logs.custom {
        custom.lines.iter().rev().take(50).rev().cloned().collect()
    } else {
        Vec::new()
    };

    let report_files = logs
        .all_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    CrashResult {
        instance_id: instance_id.to_string(),
        analyzed_at: Utc::now().to_rfc3339(),
        causes,
        summary_log_lines: summary_lines,
        report_files,
    }
}
