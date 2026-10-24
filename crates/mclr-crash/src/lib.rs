//! Minecraft crash log analysis and diagnostic report export.

pub mod causes;
pub mod collector;
pub mod error;
pub mod export;
pub mod preparer;
pub mod rules;
pub mod types;

pub use causes::CrashCause;
pub use collector::collect_crash_logs;
pub use error::CrashError;
pub use export::export_crash_report_zip;
pub use preparer::{get_head_tail_lines, mask_sensitive, normalize_crlf, prepare_text, strip_ansi};
pub use rules::analyze_crash;
pub use types::{
    CollectedLogEntry, CrashCauseFinding, CrashConfidence, CrashLogKind, CrashLogSet, CrashResult,
};
