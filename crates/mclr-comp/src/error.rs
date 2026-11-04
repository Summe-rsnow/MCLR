//! Errors for `mclr-comp`.

use mclr_core::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompError {
    #[error("network error: {0}")]
    Net(#[from] mclr_net::error::NetError),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("game error: {0}")]
    Game(#[from] mclr_game::GameError),

    #[error("loader error: {0}")]
    Loader(#[from] mclr_loaders::LoaderError),

    #[error("quilt modpacks are not supported")]
    QuiltUnsupported,

    #[error("unsupported or corrupted modpack format: {0}")]
    UnsupportedModpack(String),
    #[error("curseforge api key not configured")]
    NotConfigured,

    #[error("third-party distribution disabled for file {file_id} on project {project_id}")]
    DistributionDisabled {
        project_id: String,
        file_id: String,
        website_url: Option<String>,
    },

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("invalid query or parameters: {0}")]
    InvalidParams(String),

    #[error("toggle conflict: both enabled and disabled files exist with different contents")]
    ToggleConflict,

    #[error("instance is not modable")]
    NotModable,

    /// A resource-preset failure carrying its stable IPC error code
    /// (`preset.nameExists`, `preset.notFound`, `preset.kindUnsupported`, ...).
    #[error("preset error: {0}")]
    Preset(&'static str),
}

impl CompError {
    pub fn code(&self) -> &'static str {
        match self {
            CompError::Net(e) => e.code(),
            CompError::Http(_) => "comp.http",
            CompError::Io(_) => "comp.io",
            CompError::Json(_) => "comp.json",
            CompError::Zip(_) => "comp.zip",
            CompError::Game(e) => e.code(),
            CompError::Loader(e) => e.code(),
            CompError::QuiltUnsupported => "modpack.quiltUnsupported",
            CompError::UnsupportedModpack(_) => "modpack.unsupported",
            CompError::NotConfigured => "notConfigured",
            CompError::DistributionDisabled { .. } => "comp.cf.distributionDisabled",
            CompError::ProjectNotFound(_) => "comp.projectNotFound",
            CompError::FileNotFound(_) => "comp.fileNotFound",
            CompError::InvalidParams(_) => "comp.invalidParams",
            CompError::ToggleConflict => "resource.toggleConflict",
            CompError::NotModable => "resource.notModable",
            CompError::Preset(code) => code,
        }
    }
}

impl From<CompError> for AppError {
    fn from(err: CompError) -> Self {
        match err {
            CompError::Net(e) => e.into(),
            CompError::Game(e) => e.into(),
            CompError::Loader(e) => e.into(),
            CompError::NotConfigured => {
                AppError::new("notConfigured").with_param("what", "curseforge")
            }
            CompError::DistributionDisabled {
                project_id,
                file_id,
                website_url,
            } => {
                let mut app_err = AppError::new("comp.cf.distributionDisabled")
                    .with_param("projectId", project_id)
                    .with_param("fileId", file_id);
                if let Some(url) = website_url {
                    app_err = app_err.with_param("websiteUrl", url);
                }
                app_err
            }
            other => AppError::new(other.code()).with_detail(other.to_string()),
        }
    }
}
