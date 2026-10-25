//! Version manifest fetching, parsing, caching, and version inheritance resolution.

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, SystemTime};

use mclr_core::paths;
use mclr_core::settings::SourcePriority;
use mclr_net::{SourceKind, race_get_bytes};
use serde::{Deserialize, Serialize};
use specta::Type;
use tracing::{debug, error, warn};
use url::Url;

use crate::error::GameError;
use crate::version::VersionJson;

/// Entry in the official Mojang version manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub release_time: String,
}

/// Raw version manifest format from piston-meta.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVersionManifestLatest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawVersionManifest {
    pub latest: RawVersionManifestLatest,
    pub versions: Vec<VersionManifestEntry>,
}

/// Normalized version manifest list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionManifestList {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<VersionManifestEntry>,
}

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const UVMC_MANIFEST_URL: &str =
    "https://alist.8mi.tech/d/mirror/unlisted-versions-of-minecraft/Auto/version_manifest.json";
const MANIFEST_CACHE_DURATION: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UvmcEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    #[serde(default)]
    pub release_time: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UvmcManifest {
    #[serde(default)]
    pub versions: Vec<UvmcEntry>,
}

/// Fetches the version manifest list.
///
/// Uses cache at `<data_dir>/cache/version_manifest_v2.json` if `< 10 minutes` old and `!force_refresh`.
/// Merges unlisted versions from UVMC (cached at `<data_dir>/cache/uvmc_manifest.json`).
pub async fn fetch_version_manifest(
    client: &reqwest::Client,
    priority: SourcePriority,
    force_refresh: bool,
) -> Result<VersionManifestList, GameError> {
    let cache_dir = paths::cache_dir();
    let cache_file = cache_dir.join("version_manifest_v2.json");
    let uvmc_cache_file = cache_dir.join("uvmc_manifest.json");

    if !force_refresh && cache_file.exists() {
        if let Ok(metadata) = std::fs::metadata(&cache_file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                    if elapsed < MANIFEST_CACHE_DURATION {
                        if let Ok(content) = std::fs::read_to_string(&cache_file) {
                            if let Ok(raw) = serde_json::from_str::<RawVersionManifest>(&content) {
                                return Ok(VersionManifestList {
                                    latest_release: raw.latest.release,
                                    latest_snapshot: raw.latest.snapshot,
                                    versions: raw.versions,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    let manifest_url = Url::parse(MANIFEST_URL)
        .map_err(|e| GameError::InstallFailed(format!("Invalid manifest URL: {e}")))?;

    let bytes_res = race_get_bytes(client, &manifest_url, SourceKind::VersionMeta, priority).await;

    let mut raw: RawVersionManifest = match bytes_res {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
            error!(
                "Failed to parse version manifest from {}: {}",
                manifest_url, e
            );
            GameError::InstallFailed(format!("Failed to parse version manifest: {e}"))
        })?,
        Err(e) => {
            warn!(
                "Failed to download version manifest from {}: {}",
                manifest_url, e
            );
            // Offline fallback to existing cached version_manifest_v2.json if available
            if cache_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&cache_file) {
                    if let Ok(cached_raw) = serde_json::from_str::<RawVersionManifest>(&content) {
                        return Ok(VersionManifestList {
                            latest_release: cached_raw.latest.release,
                            latest_snapshot: cached_raw.latest.snapshot,
                            versions: cached_raw.versions,
                        });
                    }
                }
            }
            error!(
                "Version manifest unavailable and offline cache invalid: {}",
                e
            );
            return Err(e.into());
        }
    };

    // Attempt to fetch UVMC unlisted versions or fallback to cache
    let uvmc_bytes = match client
        .get(UVMC_MANIFEST_URL)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.bytes().await {
            Ok(b) => {
                let _ = std::fs::create_dir_all(&cache_dir);
                let _ = std::fs::write(&uvmc_cache_file, &b);
                Some(b.to_vec())
            }
            Err(_) => None,
        },
        _ => None,
    };

    let uvmc_content = match uvmc_bytes {
        Some(b) => Some(b),
        None => {
            if uvmc_cache_file.exists() {
                std::fs::read(&uvmc_cache_file).ok()
            } else {
                let legacy_cache = cache_dir.join("uvmc-download.json");
                if legacy_cache.exists() {
                    std::fs::read(&legacy_cache).ok()
                } else {
                    None
                }
            }
        }
    };

    if let Some(content) = uvmc_content {
        if let Ok(uvmc) = serde_json::from_slice::<UvmcManifest>(&content) {
            let existing_ids: HashSet<String> = raw.versions.iter().map(|v| v.id.clone()).collect();
            for u in uvmc.versions {
                if !existing_ids.contains(&u.id) {
                    raw.versions.push(VersionManifestEntry {
                        id: u.id,
                        version_type: u.version_type,
                        url: u.url,
                        release_time: u.release_time.or(u.time).unwrap_or_default(),
                    });
                }
            }
        }
    }

    let _ = std::fs::create_dir_all(&cache_dir);
    if let Ok(merged_bytes) = serde_json::to_vec(&raw) {
        let _ = std::fs::write(&cache_file, &merged_bytes);
    }

    Ok(VersionManifestList {
        latest_release: raw.latest.release,
        latest_snapshot: raw.latest.snapshot,
        versions: raw.versions,
    })
}

/// Resolve inheritance chain of version JSON files if `inheritsFrom` is present.
pub fn load_resolved_version_json(
    mc_folder: &Path,
    instance_id: &str,
) -> Result<VersionJson, GameError> {
    let json_path = mc_folder
        .join("versions")
        .join(instance_id)
        .join(format!("{instance_id}.json"));

    if !json_path.exists() {
        return Err(GameError::InstanceNotFound(format!(
            "Version JSON not found at {}",
            json_path.display()
        )));
    }

    let content = std::fs::read_to_string(&json_path)?;
    let mut current = VersionJson::parse(&content)?;

    let mut visited = HashSet::new();
    visited.insert(instance_id.to_string());

    while let Some(parent_id) = current.inherits_from.clone() {
        if parent_id.is_empty() || visited.contains(&parent_id) {
            break;
        }
        visited.insert(parent_id.clone());

        let parent_json_path = mc_folder
            .join("versions")
            .join(&parent_id)
            .join(format!("{parent_id}.json"));

        if !parent_json_path.exists() {
            debug!(
                "Parent version JSON for {} not found at {}, using current",
                parent_id,
                parent_json_path.display()
            );
            break;
        }

        let parent_content = std::fs::read_to_string(&parent_json_path)?;
        let parent = VersionJson::parse(&parent_content)?;
        current = crate::version::merge_version_json(current, parent);
    }

    Ok(current)
}
