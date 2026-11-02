//! Secret configuration detection and lookup from environment or `.env`.
//!
//! Secrets (`MCLR_MS_CLIENT_ID`, `MCLR_CURSEFORGE_API_KEY`, `MCLR_UPDATE_ENDPOINT`, `MCLR_UPDATE_PUBKEY`)
//! are read in order:
//! 1. Process environment variables
//! 2. `<data_dir>/.env`
//! 3. `.env` in current working directory (when `MCLR_DATA_DIR` is not overridden)

use crate::paths;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const KEY_MS_CLIENT_ID: &str = "MCLR_MS_CLIENT_ID";
pub const KEY_CURSEFORGE_API_KEY: &str = "MCLR_CURSEFORGE_API_KEY";
pub const KEY_UPDATE_ENDPOINT: &str = "MCLR_UPDATE_ENDPOINT";
pub const KEY_UPDATE_PUBKEY: &str = "MCLR_UPDATE_PUBKEY";

/// Presence status of optional launcher features and secrets.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SecretsStatus {
    pub ms_client_id: bool,
    pub curseforge_api_key: bool,
    pub update_endpoint: bool,
    pub update_pubkey: bool,
}

fn parse_env_file(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(content) = fs::read_to_string(path) else {
        return map;
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            let mut val = v.trim().to_string();
            let is_double_quoted = val.starts_with('"') && val.ends_with('"');
            let is_single_quoted = val.starts_with('\'') && val.ends_with('\'');
            if (is_double_quoted || is_single_quoted) && val.len() >= 2 {
                val = val[1..val.len() - 1].to_string();
            }
            map.insert(key, val);
        }
    }
    map
}

/// Retrieves a secret value from process env -> `<data_dir>/.env` -> current dir `.env`.
pub fn get_secret(key: &str) -> Option<String> {
    // 1. Process env
    if let Ok(val) = std::env::var(key) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // 2. <data_dir>/.env
    let data_env = paths::data_dir().join(".env");
    let map = parse_env_file(&data_env);
    if let Some(val) = map.get(key) {
        let trimmed = val.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // 3. .env in cwd (skipped during unit testing when MCLR_DATA_DIR is explicitly redirected)
    if std::env::var("MCLR_DATA_DIR").is_err() {
        let cwd_env = Path::new(".env");
        let map = parse_env_file(cwd_env);
        if let Some(val) = map.get(key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
}

/// Returns the presence status of optional launcher features.
pub fn secrets_status() -> SecretsStatus {
    SecretsStatus {
        ms_client_id: get_secret(KEY_MS_CLIENT_ID).is_some(),
        curseforge_api_key: get_secret(KEY_CURSEFORGE_API_KEY).is_some(),
        update_endpoint: get_secret(KEY_UPDATE_ENDPOINT).is_some(),
        update_pubkey: get_secret(KEY_UPDATE_PUBKEY).is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secrets_status_with_isolated_dir() {
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("MCLR_DATA_DIR", temp.path().to_str().unwrap());
            std::env::remove_var(KEY_MS_CLIENT_ID);
            std::env::remove_var(KEY_CURSEFORGE_API_KEY);
            std::env::remove_var(KEY_UPDATE_ENDPOINT);
            std::env::remove_var(KEY_UPDATE_PUBKEY);
        }
        let status = secrets_status();
        assert!(!status.ms_client_id);
        assert!(!status.curseforge_api_key);
        assert!(!status.update_endpoint);
        assert!(!status.update_pubkey);
        unsafe {
            std::env::remove_var("MCLR_DATA_DIR");
        }
    }

    #[test]
    fn test_parse_env_file_supports_quotes_and_comments() {
        let temp = tempfile::tempdir().unwrap();
        let env_path = temp.path().join(".env");
        let content = r#"
# Comment line
MCLR_KEY1=value1
MCLR_KEY2="quoted value"
MCLR_KEY3='single quoted'
"#;
        std::fs::write(&env_path, content).unwrap();
        let map = parse_env_file(&env_path);
        assert_eq!(map.get("MCLR_KEY1").map(|s| s.as_str()), Some("value1"));
        assert_eq!(
            map.get("MCLR_KEY2").map(|s| s.as_str()),
            Some("quoted value")
        );
        assert_eq!(
            map.get("MCLR_KEY3").map(|s| s.as_str()),
            Some("single quoted")
        );
    }
}
