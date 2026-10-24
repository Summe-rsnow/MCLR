//! Data types for crash analysis and IPC responses.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::causes::CrashCause;

/// Confidence level for a crash analysis finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum CrashConfidence {
    High,
    Medium,
    Low,
}

/// A specific crash cause finding returned to IPC callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CrashCauseFinding {
    pub code: String,
    pub params: HashMap<String, String>,
    pub confidence: CrashConfidence,
    pub evidence: Vec<String>,
}

/// Aggregated result of analyzing crash logs for an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CrashResult {
    pub instance_id: String,
    pub analyzed_at: String,
    pub causes: Vec<CrashCauseFinding>,
    pub summary_log_lines: Vec<String>,
    pub report_files: Vec<String>,
}

/// Internal classification of log sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrashLogKind {
    Game,
    Debug,
    CrashReport,
    HsErr,
    Custom,
}

/// A collected log entry with its source file path and text content.
#[derive(Debug, Clone)]
pub struct CollectedLogEntry {
    pub path: PathBuf,
    pub kind: CrashLogKind,
    pub lines: Vec<String>,
    pub text: String,
}

/// Container holding classified and prepared log texts for analysis.
#[derive(Debug, Clone, Default)]
pub struct CrashLogSet {
    pub game: Option<CollectedLogEntry>,
    pub debug: Option<CollectedLogEntry>,
    pub crash_report: Option<CollectedLogEntry>,
    pub hs_err: Option<CollectedLogEntry>,
    pub custom: Option<CollectedLogEntry>,
    pub all: String,
    pub all_paths: Vec<PathBuf>,
}

impl CrashLogSet {
    pub fn has_analyzable_log(&self) -> bool {
        self.game.is_some()
            || self.debug.is_some()
            || self.crash_report.is_some()
            || self.hs_err.is_some()
            || self.custom.is_some()
            || !self.all.trim().is_empty()
    }

    pub fn get_text(&self, kind: CrashLogKind) -> Option<&str> {
        match kind {
            CrashLogKind::Game => self.game.as_ref().map(|l| l.text.as_str()),
            CrashLogKind::Debug => self.debug.as_ref().map(|l| l.text.as_str()),
            CrashLogKind::CrashReport => self.crash_report.as_ref().map(|l| l.text.as_str()),
            CrashLogKind::HsErr => self.hs_err.as_ref().map(|l| l.text.as_str()),
            CrashLogKind::Custom => self.custom.as_ref().map(|l| l.text.as_str()),
        }
    }
}

/// Internal detection finding during rule execution.
#[derive(Debug, Clone)]
pub struct InternalCrashFinding {
    pub cause: CrashCause,
    pub confidence: CrashConfidence,
    pub params: HashMap<String, String>,
    pub evidence: Vec<String>,
    pub should_stop: bool,
}

impl InternalCrashFinding {
    pub fn new(cause: CrashCause, confidence: CrashConfidence) -> Self {
        Self {
            cause,
            confidence,
            params: HashMap::new(),
            evidence: Vec::new(),
            should_stop: false,
        }
    }

    pub fn with_stop(mut self, stop: bool) -> Self {
        self.should_stop = stop;
        self
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }

    pub fn into_finding(self) -> CrashCauseFinding {
        CrashCauseFinding {
            code: self.cause.code().to_string(),
            params: self.params,
            confidence: self.confidence,
            evidence: self.evidence,
        }
    }
}
