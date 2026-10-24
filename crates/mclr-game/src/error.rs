//! Error types for mclr-game.

use mclr_core::AppError;
use mclr_net::error::NetError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Network error: {0}")]
    Net(#[from] NetError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid version JSON: {0}")]
    InvalidVersionJson(String),

    #[error("Instance not found: {0}")]
    InstanceNotFound(String),

    #[error("Instance already exists: {0}")]
    InstanceExists(String),

    #[error("Invalid instance name: {0}")]
    InvalidInstanceName(String),

    #[error("Java not found: {0}")]
    JavaNotFound(String),

    #[error("Runtime manifest error: {0}")]
    RuntimeManifest(String),

    #[error("Version not found in manifest: {0}")]
    VersionNotFound(String),

    #[error("Install failed: {0}")]
    InstallFailed(String),
}

impl GameError {
    pub fn code(&self) -> &'static str {
        match self {
            GameError::Io(_) => "game.io",
            GameError::InvalidVersionJson(_) => "game.invalidVersionJson",
            GameError::InstanceNotFound(_) => "game.instanceNotFound",
            GameError::InstanceExists(_) => "game.instanceExists",
            GameError::InvalidInstanceName(_) => "game.invalidInstanceName",
            GameError::JavaNotFound(_) => "game.javaNotFound",
            GameError::RuntimeManifest(_) => "game.runtimeManifest",
            GameError::Net(e) => e.code(),
            GameError::VersionNotFound(_) => "install.versionNotFound",
            GameError::InstallFailed(_) => "install.manifestFailed",
        }
    }
}

impl From<reqwest::Error> for GameError {
    fn from(err: reqwest::Error) -> Self {
        GameError::Net(NetError::Http(err))
    }
}

impl From<GameError> for AppError {
    fn from(err: GameError) -> Self {
        let code = err.code();
        let detail = err.to_string();
        AppError::new(code).with_detail(detail)
    }
}
