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
}

impl CompError {
    pub fn code(&self) -> &'static str {
        match self {
            CompError::Net(e) => e.code(),
            CompError::Http(_) => "comp.http",
            CompError::Io(_) => "comp.io",
            CompError::Json(_) => "comp.json",
            CompError::NotConfigured => "notConfigured",
            CompError::DistributionDisabled { .. } => "comp.cf.distributionDisabled",
            CompError::ProjectNotFound(_) => "comp.projectNotFound",
            CompError::FileNotFound(_) => "comp.fileNotFound",
            CompError::InvalidParams(_) => "comp.invalidParams",
        }
    }
}

impl From<CompError> for AppError {
    fn from(err: CompError) -> Self {
        match err {
            CompError::Net(e) => e.into(),
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
