//! Shared account session traits and launch authentication payload so `mclr-game` can
//! consume sessions from `mclr-auth` without cyclic crate dependencies.

use crate::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AccountKind {
    Microsoft,
    Offline,
    Authlib,
    YggdrasilConnect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAccountSession {
    pub id: String,
    pub kind: AccountKind,
    pub username: String,
    pub uuid: String,
    pub access_token: String,
    pub user_type: String,
    pub api_location: Option<String>,
    pub xuid: Option<String>,
}

#[async_trait]
pub trait AccountSessionProvider: Send + Sync {
    async fn resolve_session(&self, account_id: &str) -> Result<ResolvedAccountSession, AppError>;
}
