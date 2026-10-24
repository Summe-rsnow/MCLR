//! Authlib-Injector / Yggdrasil authentication and API location discovery.

use crate::error::AuthError;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use specta::Type;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AuthlibProfileView {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
struct Agent {
    name: &'static str,
    version: u32,
}

#[derive(Debug, Serialize)]
struct AuthenticateRequest<'a> {
    agent: Agent,
    username: &'a str,
    password: &'a str,
    #[serde(rename = "requestUser")]
    request_user: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YggdrasilProfile {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct YggdrasilAuthResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: String,
    #[serde(rename = "availableProfiles", default)]
    pub available_profiles: Vec<YggdrasilProfile>,
    #[serde(rename = "selectedProfile")]
    pub selected_profile: Option<YggdrasilProfile>,
}

#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    #[serde(rename = "accessToken")]
    access_token: &'a str,
    #[serde(rename = "clientToken")]
    client_token: &'a str,
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    selected_profile: Option<&'a YggdrasilProfile>,
    #[serde(rename = "requestUser")]
    request_user: bool,
}

#[derive(Debug, Serialize)]
struct ValidateRequest<'a> {
    #[serde(rename = "accessToken")]
    access_token: &'a str,
    #[serde(rename = "clientToken")]
    client_token: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct YggdrasilMeta {
    #[serde(default)]
    pub meta: Option<YggdrasilMetaInner>,
}

#[derive(Debug, Deserialize)]
pub struct YggdrasilMetaInner {
    #[serde(rename = "serverName")]
    pub server_name: Option<String>,
    #[serde(default)]
    pub feature: Option<YggdrasilFeature>,
}

#[derive(Debug, Deserialize)]
pub struct YggdrasilFeature {
    #[serde(rename = "openid_configuration_url")]
    pub openid_configuration_url: Option<String>,
}

/// Normalizes server URL by ensuring scheme and trimming trailing slash / `/authserver`.
pub fn normalize_server_url(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    let with_scheme = if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        format!("https://{trimmed}")
    } else {
        trimmed.to_string()
    };
    let stripped = with_scheme.trim_end_matches('/');
    if stripped.to_ascii_lowercase().ends_with("/authserver") {
        stripped[..stripped.len() - "/authserver".len()].to_string()
    } else {
        stripped.to_string()
    }
}

/// Discovers the canonical authlib-injector API location via HEAD request and `X-Authlib-Injector-Api-Location` header.
pub async fn discover_api_location(client: &Client, server: &str) -> Result<String, AuthError> {
    let norm = normalize_server_url(server);
    let origin_url = Url::parse(&norm)
        .map_err(|e| AuthError::Other(format!("invalid server URL '{norm}': {e}")))?;

    let head_res = match client.head(origin_url.as_str()).send().await {
        Ok(res) => res,
        Err(_) => return Ok(norm),
    };

    if let Some(header_val) = head_res.headers().get("X-Authlib-Injector-Api-Location")
        && let Ok(loc_str) = header_val.to_str()
    {
        let loc_str = loc_str.trim();
        if !loc_str.is_empty() {
            let resolved = match origin_url.join(loc_str) {
                Ok(u) => u,
                Err(_) => origin_url.clone(),
            };

            let mut final_str = resolved.to_string();
            if norm.starts_with("https://") && final_str.starts_with("http://") {
                final_str = format!("https://{}", &final_str["http://".len()..]);
            }
            return Ok(normalize_server_url(&final_str));
        }
    }

    Ok(norm)
}

/// Fetches server metadata (e.g. serverName, openid_configuration_url).
pub async fn fetch_server_metadata(
    client: &Client,
    api_root: &str,
) -> Result<YggdrasilMeta, AuthError> {
    let res = client.get(api_root).send().await?;
    if !res.status().is_success() {
        return Ok(YggdrasilMeta { meta: None });
    }
    let meta: YggdrasilMeta = res.json().await.unwrap_or(YggdrasilMeta { meta: None });
    Ok(meta)
}

/// Authenticates with an authlib server via `POST /authserver/authenticate`.
pub async fn authenticate(
    client: &Client,
    api_root: &str,
    username: &str,
    password: &str,
) -> Result<YggdrasilAuthResponse, AuthError> {
    let url = format!("{}/authserver/authenticate", api_root.trim_end_matches('/'));
    let req = AuthenticateRequest {
        agent: Agent {
            name: "Minecraft",
            version: 1,
        },
        username,
        password,
        request_user: true,
    };

    let res = client.post(&url).json(&req).send().await?;
    if res.status().as_u16() == 403 || res.status().as_u16() == 400 {
        return Err(AuthError::YggdrasilInvalidCredentials);
    }
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AuthError::Http(format!(
            "Yggdrasil authenticate failed {status}: {body}"
        )));
    }

    let auth_resp: YggdrasilAuthResponse = res.json().await?;
    Ok(auth_resp)
}

/// Refreshes token and optionally binds a selected profile via `POST /authserver/refresh`.
pub async fn refresh(
    client: &Client,
    api_root: &str,
    access_token: &str,
    client_token: &str,
    selected_profile: Option<&YggdrasilProfile>,
) -> Result<YggdrasilAuthResponse, AuthError> {
    let url = format!("{}/authserver/refresh", api_root.trim_end_matches('/'));
    let req = RefreshRequest {
        access_token,
        client_token,
        selected_profile,
        request_user: true,
    };

    let res = client.post(&url).json(&req).send().await?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AuthError::Http(format!(
            "Yggdrasil refresh failed {status}: {body}"
        )));
    }

    let auth_resp: YggdrasilAuthResponse = res.json().await?;
    Ok(auth_resp)
}

/// Validates an access token via `POST /authserver/validate`.
pub async fn validate(
    client: &Client,
    api_root: &str,
    access_token: &str,
    client_token: &str,
) -> Result<bool, AuthError> {
    let url = format!("{}/authserver/validate", api_root.trim_end_matches('/'));
    let req = ValidateRequest {
        access_token,
        client_token,
    };

    let res = client.post(&url).json(&req).send().await?;
    Ok(res.status().is_success() || res.status().as_u16() == 204)
}

/// Invalidates access tokens via `POST /authserver/invalidate`.
pub async fn invalidate(
    client: &Client,
    api_root: &str,
    access_token: &str,
    client_token: &str,
) -> Result<(), AuthError> {
    let url = format!("{}/authserver/invalidate", api_root.trim_end_matches('/'));
    let req = ValidateRequest {
        access_token,
        client_token,
    };

    let _ = client.post(&url).json(&req).send().await;
    Ok(())
}

/// Resolves client_id for Yggdrasil Connect (checks metadata `shared_client_id` or host fallback).
pub fn resolve_connect_client_id(host: &str, shared_client_id: Option<&str>) -> String {
    if let Some(id) = shared_client_id
        && !id.trim().is_empty()
    {
        return id.trim().to_string();
    }
    let norm_host = host.trim().trim_end_matches('.');
    if norm_host.eq_ignore_ascii_case("littleskin.cn") {
        "1514".to_string()
    } else {
        String::new()
    }
}
