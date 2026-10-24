//! Errors for `mclr-loaders`.

use mclr_core::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error("network error: {0}")]
    Net(#[from] mclr_net::error::NetError),

    #[error("game error: {0}")]
    Game(#[from] mclr_game::GameError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("loader metadata error: {0}")]
    Metadata(String),

    #[error("loader version not found: {0}")]
    VersionNotFound(String),

    #[error("installer process failed: {0}")]
    InstallerFailed(String),

    #[error("java required for installer not found")]
    JavaNotFound,
}

impl LoaderError {
    pub fn code(&self) -> &'static str {
        match self {
            LoaderError::Net(e) => e.code(),
            LoaderError::Game(e) => e.code(),
            LoaderError::Io(_) => "loaders.io",
            LoaderError::Zip(_) => "loaders.zip",
            LoaderError::Json(_) => "loaders.json",
            LoaderError::Metadata(_) => "loaders.metadata",
            LoaderError::VersionNotFound(_) => "loaders.versionNotFound",
            LoaderError::InstallerFailed(_) => "loaders.installerFailed",
            LoaderError::JavaNotFound => "launch.java.notFound",
        }
    }
}

impl From<LoaderError> for AppError {
    fn from(err: LoaderError) -> Self {
        match err {
            LoaderError::Net(e) => e.into(),
            LoaderError::Game(e) => e.into(),
            other => AppError::new(other.code()).with_detail(other.to_string()),
        }
    }
}
