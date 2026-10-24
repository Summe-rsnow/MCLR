//! Error types for mclr-auth.

use mclr_core::AppError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Feature not configured: {what}")]
    NotConfigured { what: &'static str },

    #[error("Xbox account banned (XErr 2148916227)")]
    XstsBanned,

    #[error("No Xbox account associated (XErr 2148916233)")]
    XstsNoXboxAccount,

    #[error("Xbox Live not available in region (XErr 2148916235)")]
    XstsRegionBlocked,

    #[error("Xbox account is child/minor without family approval (XErr 2148916238)")]
    XstsMinor,

    #[error("Minecraft profile not found or game not owned")]
    MsNoProfile,

    #[error("Microsoft token expired")]
    MsExpiredToken,

    #[error("Microsoft login access denied")]
    MsAccessDenied,

    #[error("Invalid Yggdrasil credentials")]
    YggdrasilInvalidCredentials,

    #[error("Invalid skin image: {0}")]
    SkinInvalidImage(String),

    #[error("Account store corrupted: {0}")]
    StoreCorrupted(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Auth error: {0}")]
    Other(String),
}

impl AuthError {
    pub fn code(&self) -> &'static str {
        match self {
            AuthError::NotConfigured { .. } => "notConfigured",
            AuthError::XstsBanned => "auth.xsts.banned",
            AuthError::XstsNoXboxAccount => "auth.xsts.noXboxAccount",
            AuthError::XstsRegionBlocked => "auth.xsts.regionBlocked",
            AuthError::XstsMinor => "auth.xsts.minor",
            AuthError::MsNoProfile => "auth.ms.noProfile",
            AuthError::MsExpiredToken => "auth.ms.expiredToken",
            AuthError::MsAccessDenied => "auth.ms.accessDenied",
            AuthError::YggdrasilInvalidCredentials => "auth.yggdrasil.invalidCredentials",
            AuthError::SkinInvalidImage(_) => "auth.skin.invalidImage",
            AuthError::StoreCorrupted(_) => "auth.store.corrupted",
            AuthError::AccountNotFound(_) => "auth.accountNotFound",
            AuthError::Io(_) => "auth.io",
            AuthError::Http(_) => "auth.http",
            AuthError::Json(_) => "auth.json",
            AuthError::Crypto(_) => "auth.crypto",
            AuthError::Other(_) => "auth.other",
        }
    }
}

impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        let code = err.code();
        let detail = err.to_string();
        let mut app_err = AppError::new(code).with_detail(detail);
        if let AuthError::NotConfigured { what } = err {
            app_err = app_err.with_param("what", what);
        }
        app_err
    }
}

impl From<reqwest::Error> for AuthError {
    fn from(err: reqwest::Error) -> Self {
        AuthError::Http(err.to_string())
    }
}
