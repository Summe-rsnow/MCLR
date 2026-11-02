//! Launcher self-update wrapper around Tauri updater plugin manifest and runtime configs.

use mclr_core::AppError;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Updater is not configured")]
    NotConfigured,

    #[error("Invalid update URL: {0}")]
    InvalidUrl(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Invalid update signature")]
    SignatureInvalid,

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl UpdateError {
    pub fn code(&self) -> &'static str {
        match self {
            UpdateError::NotConfigured => "notConfigured",
            UpdateError::InvalidUrl(_) => "update.invalidUrl",
            UpdateError::Http(_) => "update.http",
            UpdateError::SignatureInvalid => "update.signatureInvalid",
            UpdateError::Json(_) => "update.json",
        }
    }
}

impl From<UpdateError> for AppError {
    fn from(err: UpdateError) -> Self {
        match err {
            UpdateError::NotConfigured => {
                AppError::new("notConfigured").with_param("what", "updater")
            }
            other => {
                let code = other.code();
                let detail = other.to_string();
                AppError::new(code).with_detail(detail)
            }
        }
    }
}

/// Runtime configuration for checking launcher updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateConfig {
    pub endpoint: Url,
    pub pubkey: String,
    pub channel: String,
}

impl UpdateConfig {
    /// Constructs `UpdateConfig` from secrets (`MCLR_UPDATE_ENDPOINT`, `MCLR_UPDATE_PUBKEY`).
    ///
    /// If either secret is missing or empty, returns `UpdateError::NotConfigured`.
    /// Appends `?channel=<channel>` or `&channel=<channel>` to the endpoint URL.
    pub fn from_runtime(channel: &str) -> Result<Self, UpdateError> {
        let endpoint_raw = mclr_core::get_secret(mclr_core::KEY_UPDATE_ENDPOINT)
            .filter(|s| !s.trim().is_empty())
            .ok_or(UpdateError::NotConfigured)?;
        let pubkey = mclr_core::get_secret(mclr_core::KEY_UPDATE_PUBKEY)
            .filter(|s| !s.trim().is_empty())
            .ok_or(UpdateError::NotConfigured)?;

        let mut endpoint =
            Url::parse(&endpoint_raw).map_err(|e| UpdateError::InvalidUrl(e.to_string()))?;
        let active_channel = if channel.trim().is_empty() {
            "stable"
        } else {
            channel.trim()
        };
        let mut query_pairs: Vec<(String, String)> = endpoint
            .query_pairs()
            .filter(|(k, _)| k != "channel")
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        query_pairs.push(("channel".to_string(), active_channel.to_string()));
        endpoint
            .query_pairs_mut()
            .clear()
            .extend_pairs(query_pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())));

        Ok(Self {
            endpoint,
            pubkey,
            channel: active_channel.to_string(),
        })
    }
}

/// Result of an update check.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub available: bool,
    pub current_version: String,
    pub version: Option<String>,
    pub date: Option<String>,
    pub body: Option<String>,
    pub download_url: Option<String>,
    pub signature: Option<String>,
}

/// Platform target entry in Tauri v2 updater JSON manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriUpdatePlatform {
    pub signature: String,
    pub url: String,
}

/// Tauri v2 updater JSON manifest format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriUpdateManifest {
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub pub_date: Option<String>,
    #[serde(default)]
    pub platforms: HashMap<String, TauriUpdatePlatform>,
}

/// Compares version string `candidate` against `current` using semver rules.
/// Returns true if `candidate` is newer than `current`.
pub fn is_version_newer(candidate: &str, current: &str) -> bool {
    let parse_tuple = |v: &str| -> (Vec<u64>, Option<String>) {
        let cleaned = v.trim().trim_start_matches('v');
        let (num_part, pre_part) = match cleaned.find('-') {
            Some(idx) => (&cleaned[..idx], Some(cleaned[idx + 1..].to_string())),
            None => (cleaned, None),
        };
        let nums: Vec<u64> = num_part
            .split('.')
            .map(|segment| segment.parse::<u64>().unwrap_or(0))
            .collect();
        (nums, pre_part)
    };

    let (c_nums, c_pre) = parse_tuple(candidate);
    let (cur_nums, cur_pre) = parse_tuple(current);

    let max_len = c_nums.len().max(cur_nums.len());
    for i in 0..max_len {
        let c_val = *c_nums.get(i).unwrap_or(&0);
        let cur_val = *cur_nums.get(i).unwrap_or(&0);
        if c_val > cur_val {
            return true;
        }
        if c_val < cur_val {
            return false;
        }
    }

    // Numbers are equal: a version with pre-release is older than a release without pre-release
    match (c_pre, cur_pre) {
        (None, Some(_)) => true, // candidate is stable release, current is pre-release -> newer
        (Some(_), None) => false, // candidate is pre-release, current is stable -> older
        (Some(c_p), Some(cur_p)) => c_p > cur_p,
        (None, None) => false,
    }
}

/// Fetches the update manifest from the given `config.endpoint` and checks against `current_version`.
pub async fn check_update_manifest(
    client: &reqwest::Client,
    config: &UpdateConfig,
    current_version: &str,
) -> Result<UpdateCheckResult, UpdateError> {
    let response = client
        .get(config.endpoint.clone())
        .header("User-Agent", format!("MCLR/{}", current_version))
        .send()
        .await
        .map_err(|e| UpdateError::Http(e.to_string()))?;

    if response.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(UpdateCheckResult {
            available: false,
            current_version: current_version.to_string(),
            version: None,
            date: None,
            body: None,
            download_url: None,
            signature: None,
        });
    }

    if !response.status().is_success() {
        return Err(UpdateError::Http(format!(
            "Server returned status code {}",
            response.status()
        )));
    }

    let manifest: TauriUpdateManifest = response
        .json()
        .await
        .map_err(|e| UpdateError::Http(e.to_string()))?;

    let newer = is_version_newer(&manifest.version, current_version);
    // Find Windows x86_64 platform entry, or fallback to first available
    let platform = manifest
        .platforms
        .get("windows-x86_64")
        .or_else(|| manifest.platforms.get("windows-x64"))
        .or_else(|| manifest.platforms.values().next());

    let (download_url, signature) = match platform {
        Some(p) => (Some(p.url.clone()), Some(p.signature.clone())),
        None => (None, None),
    };

    Ok(UpdateCheckResult {
        available: newer,
        current_version: current_version.to_string(),
        version: Some(manifest.version),
        date: manifest.pub_date,
        body: manifest.notes,
        download_url,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_update_not_configured_when_env_missing() {
        let res = UpdateConfig::from_runtime("stable");
        assert!(matches!(res, Err(UpdateError::NotConfigured)));
        let app_err: AppError = res.unwrap_err().into();
        assert_eq!(app_err.code, "notConfigured");
        assert_eq!(
            app_err.params.get("what").map(|s| s.as_str()),
            Some("updater")
        );
    }

    #[tokio::test]
    async fn test_update_check_staged_manifest_wiremock() {
        let server = MockServer::start().await;

        let manifest_json = serde_json::json!({
            "version": "0.0.99",
            "notes": "Test release notes",
            "pub_date": "2026-10-25T12:00:00Z",
            "platforms": {
                "windows-x86_64": {
                    "signature": "test-signature-base64",
                    "url": format!("{}/download/mclr_0.0.99_x64-setup.exe", server.uri())
                }
            }
        });

        Mock::given(method("GET"))
            .and(path("/update"))
            .and(query_param("channel", "stable"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&manifest_json))
            .mount(&server)
            .await;

        let config = UpdateConfig {
            endpoint: Url::parse(&format!("{}/update?channel=stable", server.uri())).unwrap(),
            pubkey: "dummy-pubkey".to_string(),
            channel: "stable".to_string(),
        };

        let client = reqwest::Client::new();
        let result = check_update_manifest(&client, &config, "0.0.11")
            .await
            .expect("manifest check should succeed");

        assert!(result.available);
        assert_eq!(result.current_version, "0.0.11");
        assert_eq!(result.version.as_deref(), Some("0.0.99"));
        assert_eq!(result.body.as_deref(), Some("Test release notes"));
        assert_eq!(
            result.download_url.as_deref(),
            Some(format!("{}/download/mclr_0.0.99_x64-setup.exe", server.uri()).as_str())
        );
        assert_eq!(result.signature.as_deref(), Some("test-signature-base64"));
    }

    #[test]
    fn test_is_version_newer() {
        assert!(is_version_newer("0.0.12", "0.0.11"));
        assert!(is_version_newer("0.1.0", "0.0.11"));
        assert!(is_version_newer("1.0.0", "0.9.9"));
        assert!(!is_version_newer("0.0.11", "0.0.11"));
        assert!(!is_version_newer("0.0.10", "0.0.11"));
        assert!(!is_version_newer("0.0.11-beta", "0.0.11"));
    }
}
