//! Error types for mclr-core.

use crate::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid settings patch")]
    SettingsInvalid(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already exists: {0}")]
    TaskAlreadyExists(String),

    #[error("Task in invalid state: {0}")]
    TaskInvalidState(String),
}

impl CoreError {
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::Io(_) => "core.io",
            CoreError::Json(_) => "core.json",
            CoreError::SettingsInvalid(_) => "settings.invalid",
            CoreError::TaskNotFound(_) => "task.notFound",
            CoreError::TaskAlreadyExists(_) => "task.alreadyExists",
            CoreError::TaskInvalidState(_) => "task.invalidState",
        }
    }
}

impl From<CoreError> for AppError {
    fn from(err: CoreError) -> Self {
        let code = err.code();
        let detail = err.to_string();
        AppError::new(code).with_detail(detail)
    }
}
