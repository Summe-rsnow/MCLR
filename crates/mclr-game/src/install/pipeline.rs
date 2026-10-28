//! Minecraft installation, repair pipeline, and stale directory cleanup.

use std::path::Path;

use mclr_core::settings::NetSettings;
use mclr_core::tasks::TaskCenter;
use mclr_net::{Downloader, SourceKind, race_get_bytes};
use tracing::{error, info};
use url::Url;

use super::RepairMode;
use super::manifest::{fetch_version_manifest, load_resolved_version_json};
use super::specs::{collect_version_download_specs, verify_specs_cached};
use super::verified_manifest::{self, VerifiedManifest};
use crate::error::GameError;
use crate::instance::{InstanceConfig, InstanceInfo, classify_instance, validate_instance_name};
use crate::version::VersionJson;

/// Installs a vanilla Minecraft instance.
///
/// Fetches version manifest, downloads version JSON, writes instance json,
/// and downloads all client jar, libraries, natives, assets, and logging configs.
pub async fn install_vanilla(
    client: &reqwest::Client,
    net_settings: &NetSettings,
    mc_folder: &Path,
    mc_version: &str,
    instance_name: &str,
    task_ctx: Option<(&TaskCenter, &str)>,
) -> Result<InstanceInfo, GameError> {
    validate_instance_name(instance_name)?;

    let instance_dir = mc_folder.join("versions").join(instance_name);
    let version_json_file = instance_dir.join(format!("{instance_name}.json"));

    // Resumable: if instance JSON already exists, overwrite/verify and download only missing files.
    // 1. Fetch manifest
    let manifest = fetch_version_manifest(client, net_settings.source_priority, false).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or_else(|| GameError::VersionNotFound(mc_version.to_string()))?;

    let version_url = Url::parse(&entry.url)
        .map_err(|e| GameError::InstallFailed(format!("Invalid version JSON URL: {e}")))?;

    // 2. Fetch version JSON
    let version_json_bytes = match race_get_bytes(
        client,
        &version_url,
        SourceKind::VersionMeta,
        net_settings.source_priority,
    )
    .await
    {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to download version JSON for {mc_version} from {version_url}: {e}");
            return Err(e.into());
        }
    };

    let mut version_json: VersionJson =
        serde_json::from_slice(&version_json_bytes).map_err(|e| {
            error!("Failed to parse version JSON for {mc_version}: {e}");
            GameError::InvalidVersionJson(e.to_string())
        })?;

    // Set id = instance_name
    version_json.id = instance_name.to_string();

    std::fs::create_dir_all(&instance_dir)?;
    let installing_marker = instance_dir.join(".mclr-installing");
    let _ = std::fs::write(&installing_marker, b"");

    std::fs::write(
        &version_json_file,
        serde_json::to_string_pretty(&version_json)
            .map_err(|e| GameError::InstallFailed(e.to_string()))?,
    )?;

    // Write initial mclr.instance.json
    let config_path = instance_dir.join("mclr.instance.json");
    let config = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|c| serde_json::from_str::<InstanceConfig>(&c).ok())
            .unwrap_or_default()
    } else {
        let mut cfg = InstanceConfig::default();
        cfg.isolation = "on".to_string();
        let _ = std::fs::write(
            &config_path,
            serde_json::to_string_pretty(&cfg).unwrap_or_default(),
        );
        cfg
    };

    // 3. Collect download specs
    let specs = collect_version_download_specs(
        client,
        mc_folder,
        instance_name,
        &version_json,
        net_settings.source_priority,
    )
    .await?;

    info!(
        "Total files to verify/download for {}: {}",
        instance_name,
        specs.len()
    );

    let unverified_specs = verify_specs_cached(specs).await;
    info!(
        "Files to download after checking cache: {}",
        unverified_specs.len()
    );

    // 4. Download files
    if !unverified_specs.is_empty() {
        let downloader = Downloader::new(client.clone(), net_settings.clone());
        if let Err(e) = downloader.download_many(unverified_specs, task_ctx).await {
            error!("Batch downloading failed for instance {instance_name}: {e}");
            return Err(e.into());
        }
    }

    let _ = std::fs::remove_file(installing_marker);

    // 5. Build and return InstanceInfo
    let (kind, loaders) = classify_instance(&version_json, mc_version);
    let state = crate::instance::InstanceState::Ok;

    Ok(InstanceInfo {
        id: instance_name.to_string(),
        name: instance_name.to_string(),
        folder: mc_folder.to_string_lossy().to_string(),
        path: instance_dir.to_string_lossy().to_string(),
        mc_version: mc_version.to_string(),
        kind,
        loaders,
        state,
        config,
        ..Default::default()
    })
}

/// Repairs an existing Minecraft instance by checking and downloading missing/corrupted files.
/// Returns the number of files downloaded.
///
/// With [`RepairMode::Fast`] a matching `.mclr-verified.json` short-circuits the run before the
/// asset index is read, so a normal launch only pays for the version JSON parse.
pub async fn repair_instance(
    client: &reqwest::Client,
    net_settings: &NetSettings,
    mc_folder: &Path,
    instance_id: &str,
    task_ctx: Option<(&TaskCenter, &str)>,
    mode: RepairMode,
) -> Result<usize, GameError> {
    let instance_dir = mc_folder.join("versions").join(instance_id);
    let version_json_file = instance_dir.join(format!("{instance_id}.json"));

    let resolved_json = load_resolved_version_json(mc_folder, instance_id)?;
    let asset_index_id = resolved_json
        .asset_index
        .as_ref()
        .map(|a| a.id.clone())
        .unwrap_or_default();

    // Fast path: the version JSON and asset index id are unchanged since the last
    // successful full repair, so every file was verified then and nothing on disk
    // has been touched since. Skipping here is what keeps startup cheap.
    if mode == RepairMode::Fast
        && let Some(manifest) = verified_manifest::load(&instance_dir)
        && let Some((mtime_ms, len)) = verified_manifest::file_fingerprint(&version_json_file)
        && manifest.version_json_mtime_ms == mtime_ms
        && manifest.version_json_len == len
        && manifest.asset_index_id == asset_index_id
    {
        info!("Repair: skipped (verified manifest)");
        return Ok(0);
    }

    let specs = collect_version_download_specs(
        client,
        mc_folder,
        instance_id,
        &resolved_json,
        net_settings.source_priority,
    )
    .await?;

    let unverified_specs = verify_specs_cached(specs).await;

    let count = unverified_specs.len();
    info!("Repairing instance {instance_id}: {count} files missing or corrupted");

    if count > 0 {
        let downloader = Downloader::new(client.clone(), net_settings.clone());
        if let Err(e) = downloader.download_many(unverified_specs, task_ctx).await {
            error!("Batch downloading failed during repair of instance {instance_id}: {e}");
            return Err(e.into());
        }
    }

    // Only a completed repair may leave a fingerprint behind; an interrupted or
    // failed one must stay untrusted so the next launch repairs in full.
    if let Some((mtime_ms, len)) = verified_manifest::file_fingerprint(&version_json_file) {
        verified_manifest::save(
            &instance_dir,
            &VerifiedManifest {
                version_json_mtime_ms: mtime_ms,
                version_json_len: len,
                asset_index_id,
                lib_count: resolved_json.libraries.len() as u32,
            },
        );
    }

    Ok(count)
}

/// A `.mclr-installing` marker older than this is treated as an abandoned install.
const STALE_MARKER_AGE_SECS: u64 = 6 * 60 * 60;

/// Remove incomplete instance folders (containing `.mclr-installing`) and `.mclr-staging` dirs left behind by interrupted installs.
pub fn cleanup_stale(mc_folder: &std::path::Path) {
    let versions_dir = mc_folder.join("versions");
    if versions_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let marker = path.join(".mclr-installing");
                if path.is_dir() && marker.exists() {
                    // Only delete markers that are genuinely stale. A finished
                    // install whose marker failed to unlink (AV lock, read-only
                    // attribute) must never lose the whole instance on next start.
                    let age = std::fs::metadata(&marker)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok());
                    let stale = age.is_some_and(|d| d.as_secs() >= STALE_MARKER_AGE_SECS);
                    if stale {
                        tracing::info!("Cleaning up incomplete instance: {}", path.display());
                        let _ = std::fs::remove_dir_all(&path);
                    } else {
                        tracing::warn!(
                            "Skipping cleanup of {}: install marker is recent",
                            path.display()
                        );
                    }
                }
            }
        }
    }
    let staging_dir = mc_folder.join(".mclr-staging");
    if staging_dir.exists() {
        tracing::info!("Cleaning up stale staging dir: {}", staging_dir.display());
        let _ = std::fs::remove_dir_all(&staging_dir);
    }
}
