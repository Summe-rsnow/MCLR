//! Error types for mclr-game.

use mclr_core::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
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
        }
    }
}

impl From<GameError> for AppError {
    fn from(err: GameError) -> Self {
        let code = err.code();
        let detail = err.to_string();
        AppError::new(code).with_detail(detail)
    }
}
