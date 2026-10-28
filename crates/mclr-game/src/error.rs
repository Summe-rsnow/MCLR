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

    #[error("Invalid path contains '!' or ';': {0}")]
    LaunchInvalidPath(String),

    #[error("Login requirement not satisfied: required '{required}', but got '{actual}'")]
    LaunchLoginRequirement { required: String, actual: String },

    #[error("Java {actual} is too old for this instance (requires Java {required})")]
    LaunchJavaTooOld { required: u32, actual: u32 },
    #[error("Matching Java installation not found for major version {major}")]
    LaunchJavaNotFound {
        major: u32,
        /// Mojang runtime component that provides `major` (e.g. `java-runtime-epsilon`).
        component: Option<String>,
    },

    #[error("Failed to extract natives: {0}")]
    LaunchNativesFailed(String),

    #[error("Process launch failed: {0}")]
    LaunchProcessFailed(String),

    #[error("Process {0} not found")]
    LaunchProcessNotFound(u32),
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
            GameError::LaunchInvalidPath(_) => "launch.invalidPath",
            GameError::LaunchLoginRequirement { .. } => "launch.loginRequirement",
            GameError::LaunchJavaNotFound { .. } => "launch.java.notFound",
            GameError::LaunchJavaTooOld { .. } => "launch.java.tooOld",
            GameError::LaunchNativesFailed(_) => "launch.nativesFailed",
            GameError::LaunchProcessFailed(_) => "launch.processFailed",
            GameError::LaunchProcessNotFound(_) => "launch.processNotFound",
        }
    }
}

impl From<reqwest::Error> for GameError {
    fn from(err: reqwest::Error) -> Self {
        GameError::Net(NetError::Http(err))
    }
}

impl From<zip::result::ZipError> for GameError {
    fn from(err: zip::result::ZipError) -> Self {
        GameError::LaunchNativesFailed(err.to_string())
    }
}

impl From<GameError> for AppError {
    fn from(err: GameError) -> Self {
        let code = err.code();
        let detail = err.to_string();
        let app_err = AppError::new(code).with_detail(detail);
        match err {
            GameError::LaunchJavaTooOld { required, actual } => app_err
                .with_param("required", required.to_string())
                .with_param("actual", actual.to_string()),
            GameError::LaunchJavaNotFound { major, component } => {
                let app_err = app_err.with_param("major", major.to_string());
                match component {
                    Some(component) => app_err.with_param("component", component),
                    None => app_err,
                }
            }
            GameError::LaunchLoginRequirement { required, actual } => app_err
                .with_param("required", required)
                .with_param("actual", actual),
            _ => app_err,
        }
    }
}
