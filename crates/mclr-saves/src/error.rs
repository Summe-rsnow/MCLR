//! Errors for `mclr-saves`.

use mclr_core::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SavesError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("nbt error: {0}")]
    Nbt(#[from] fastnbt::error::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("save not found: {0}")]
    SaveNotFound(String),

    #[error("server ping failed: {0}")]
    PingFailed(String),

    #[error("invalid path or file: {0}")]
    InvalidPath(String),
}

impl SavesError {
    pub fn code(&self) -> &'static str {
        match self {
            SavesError::Io(_) => "saves.io",
            SavesError::Nbt(_) => "saves.nbt",
            SavesError::Zip(_) => "saves.zip",
            SavesError::Json(_) => "saves.json",
            SavesError::SaveNotFound(_) => "saves.notFound",
            SavesError::PingFailed(_) => "saves.pingFailed",
            SavesError::InvalidPath(_) => "saves.invalidPath",
        }
    }
}

impl From<SavesError> for AppError {
    fn from(err: SavesError) -> Self {
        AppError::new(err.code()).with_detail(err.to_string())
    }
}
