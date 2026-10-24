//! Crash analysis and report errors.

use mclr_core::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrashError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("instance not found: {0}")]
    InstanceNotFound(String),

    #[error("comp error: {0}")]
    Comp(#[from] mclr_comp::CompError),

    #[error("game error: {0}")]
    Game(#[from] mclr_game::GameError),

    #[error("crash error: {0}")]
    Custom(String),
}

impl CrashError {
    pub fn code(&self) -> &'static str {
        match self {
            CrashError::Io(_) => "crash.io",
            CrashError::Zip(_) => "crash.zip",
            CrashError::Json(_) => "crash.json",
            CrashError::InstanceNotFound(_) => "crash.instanceNotFound",
            CrashError::Comp(_) => "crash.comp",
            CrashError::Game(_) => "crash.game",
            CrashError::Custom(_) => "crash.custom",
        }
    }
}

impl From<CrashError> for AppError {
    fn from(err: CrashError) -> Self {
        AppError::new(err.code()).with_detail(err.to_string())
    }
}
