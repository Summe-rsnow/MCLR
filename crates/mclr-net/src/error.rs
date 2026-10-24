use mclr_core::AppError;

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Network timeout: {0}")]
    Timeout(String),

    #[error("Hash mismatch: expected {expected}, actual {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("Size mismatch: expected {expected}, actual {actual}")]
    SizeMismatch { expected: u64, actual: u64 },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Task was canceled")]
    Canceled,

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Proxy configuration error: {0}")]
    Proxy(String),

    #[error("Source racing failed: {0}")]
    RacingFailed(String),

    #[error("All download sources exhausted: {0}")]
    SourcesExhausted(String),
}

impl NetError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Http(_) => "net.http",
            Self::Timeout(_) => "net.timeout",
            Self::HashMismatch { .. } => "net.hashMismatch",
            Self::SizeMismatch { .. } => "net.sizeMismatch",
            Self::Io(_) => "net.io",
            Self::Canceled => "net.canceled",
            Self::InvalidUrl(_) => "net.invalidUrl",
            Self::Proxy(_) => "net.proxy",
            Self::RacingFailed(_) => "net.racingFailed",
            Self::SourcesExhausted(_) => "net.sourcesExhausted",
        }
    }
}

impl From<NetError> for AppError {
    fn from(err: NetError) -> Self {
        let code = err.code();
        let mut app_err = AppError::new(code).with_detail(err.to_string());
        match &err {
            NetError::HashMismatch { expected, actual } => {
                app_err = app_err
                    .with_param("expected", expected)
                    .with_param("actual", actual);
            }
            NetError::SizeMismatch { expected, actual } => {
                app_err = app_err
                    .with_param("expected", expected.to_string())
                    .with_param("actual", actual.to_string());
            }
            NetError::InvalidUrl(url) => {
                app_err = app_err.with_param("url", url);
            }
            NetError::Proxy(msg) => {
                app_err = app_err.with_param("reason", msg);
            }
            _ => {}
        }
        app_err
    }
}
