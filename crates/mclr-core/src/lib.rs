//! Core runtime primitives, paths, settings, tasks, and error contract for MCLR.

use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fmt;

/// Cross-crate application error transferred over Tauri IPC.
/// Rust never emits user-facing strings; UI renders `t('error.' + code, params)`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    #[serde(default)]
    pub params: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            params: HashMap::new(),
            detail: None,
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.detail {
            Some(detail) => write!(f, "{} ({})", self.code, detail),
            None => write!(f, "{}", self.code),
        }
    }
}

impl std::error::Error for AppError {}

/// Application metadata returned by `app_info`.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

impl AppInfo {
    pub fn current() -> Self {
        Self {
            name: "MCLR".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
