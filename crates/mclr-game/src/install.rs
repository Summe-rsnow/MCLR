//! Minecraft vanilla installation, manifest fetching, and instance repair.

use crate::error::GameError;
use crate::instance::{
    InstanceConfig, InstanceInfo, classify_instance, extract_mc_version, validate_instance_name,
};
use crate::version::{Artifact, Library, RuleContext, VersionJson, evaluate_rules};
use mclr_core::paths;
use mclr_core::settings::{NetSettings, SourcePriority};
use mclr_core::tasks::TaskCenter;
use mclr_net::{DownloadSpec, Downloader, SourceKind, race_get_bytes, rewrite_url};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use specta::Type;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tracing::{debug, info};
use url::Url;

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

/// Asset index file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetIndexObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssetIndexFile {
    pub objects: HashMap<String, AssetIndexObject>,
}

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const MANIFEST_CACHE_DURATION: Duration = Duration::from_secs(10 * 60);

/// Fetches the version manifest list.
///
/// Uses cache at `<data_dir>/cache/version_manifest_v2.json` if `< 10 minutes` old and `!force_refresh`.
pub async fn fetch_version_manifest(
    client: &reqwest::Client,
    priority: SourcePriority,
    force_refresh: bool,
) -> Result<VersionManifestList, GameError> {
    let cache_dir = paths::cache_dir();
    let cache_file = cache_dir.join("version_manifest_v2.json");

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

    let bytes = race_get_bytes(client, &manifest_url, SourceKind::VersionMeta, priority).await?;

    let raw: RawVersionManifest = serde_json::from_slice(&bytes)
        .map_err(|e| GameError::InstallFailed(format!("Failed to parse version manifest: {e}")))?;

    let _ = std::fs::create_dir_all(&cache_dir);
    let _ = std::fs::write(&cache_file, &bytes);

    Ok(VersionManifestList {
        latest_release: raw.latest.release,
        latest_snapshot: raw.latest.snapshot,
        versions: raw.versions,
    })
}

/// Convert Maven coordinate `group:artifact:version[:classifier][@ext]` into relative path.
pub fn maven_coordinate_to_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return name.to_string();
    }

    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let (version, classifier, ext) = if parts.len() >= 4 {
        let ver = parts[2];
        let fourth = parts[3];
        if let Some(idx) = fourth.find('@') {
            (ver, Some(&fourth[..idx]), &fourth[idx + 1..])
        } else {
            (ver, Some(fourth), "jar")
        }
    } else {
        let third = parts[2];
        if let Some(idx) = third.find('@') {
            (&third[..idx], None, &third[idx + 1..])
        } else {
            (third, None, "jar")
        }
    };

    let filename = if let Some(clf) = classifier {
        format!("{artifact}-{version}-{clf}.{ext}")
    } else {
        format!("{artifact}-{version}.{ext}")
    };
    format!("{group}/{artifact}/{version}/{filename}")
}

/// Builds URL candidates for a given URL string, SourceKind, and SourcePriority.
fn build_urls(primary_url: &str, kind: SourceKind, priority: SourcePriority) -> Vec<Url> {
    if let Ok(parsed) = Url::parse(primary_url) {
        let rewritten = rewrite_url(&parsed, kind);
        if rewritten.is_empty() {
            return vec![parsed];
        }

        let ladder = mclr_net::racing_ladder(priority);
        let mut ordered_urls = Vec::new();

        for (ladder_origin, _) in ladder {
            for (origin, candidate_url) in &rewritten {
                if *origin == ladder_origin && !ordered_urls.contains(candidate_url) {
                    ordered_urls.push(candidate_url.clone());
                }
            }
        }

        for (_, candidate_url) in &rewritten {
            if !ordered_urls.contains(candidate_url) {
                ordered_urls.push(candidate_url.clone());
            }
        }

        if !ordered_urls.is_empty() {
            ordered_urls
        } else {
            vec![parsed]
        }
    } else {
        Vec::new()
    }
}

/// Checks if a file already exists on disk and satisfies expected size and sha1.
fn file_is_verified(path: &Path, expected_size: Option<u64>, expected_sha1: Option<&str>) -> bool {
    if !path.exists() {
        return false;
    }

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };

    if let Some(size) = expected_size {
        if metadata.len() != size {
            return false;
        }
    }

    if let Some(sha1) = expected_sha1 {
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut hasher = Sha1::new();
        let mut buf = [0u8; 64 * 1024];
        use std::io::Read;
        loop {
            match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => hasher.update(&buf[..n]),
                Err(_) => return false,
            }
        }
        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(sha1) {
            return false;
        }
    }

    true
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

    let mut visited = std::collections::HashSet::new();
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

/// Builds the complete list of download specs required by a resolved `VersionJson`.
pub async fn collect_version_download_specs(
    client: &reqwest::Client,
    mc_folder: &Path,
    instance_name: &str,
    version_json: &VersionJson,
    priority: SourcePriority,
) -> Result<Vec<DownloadSpec>, GameError> {
    let mut specs = Vec::new();
    let rule_ctx = RuleContext::windows_default();

    // 1. Client JAR
    if let Some(downloads) = &version_json.downloads {
        if let Some(client_dl) = &downloads.client {
            let client_jar_dest = mc_folder
                .join("versions")
                .join(instance_name)
                .join(format!("{instance_name}.jar"));

            if let Some(url_str) = &client_dl.url {
                let mut urls = build_urls(url_str, SourceKind::Libraries, priority);

                // Add BMCLAPI specific client jar mirror if we have an MC version
                let mc_ver = extract_mc_version(version_json, instance_name);
                if let Ok(bmclapi_client) = Url::parse(&format!(
                    "https://bmclapi2.bangbang93.com/version/{mc_ver}/client"
                )) {
                    if !urls.contains(&bmclapi_client) {
                        urls.insert(0, bmclapi_client);
                    }
                }
                if !urls.is_empty() {
                    specs.push(DownloadSpec {
                        urls,
                        dest: client_jar_dest,
                        size: client_dl.size,
                        sha1: client_dl.sha1.clone(),
                        sha256: None,
                        md5: None,
                    });
                }
            }
        }
    }

    // 2. Libraries
    for lib in &version_json.libraries {
        if !evaluate_rules(&lib.rules, &rule_ctx) {
            continue;
        }

        // Standard artifact
        if let Some(downloads) = &lib.downloads {
            if let Some(artifact) = &downloads.artifact {
                add_artifact_spec(&mut specs, mc_folder, lib, artifact, priority);
            } else if lib.natives.is_empty() {
                // Fallback to Maven URL from name if no downloads.artifact
                add_maven_library_spec(&mut specs, mc_folder, lib, priority);
            }

            // Natives classifiers
            if let Some(native_key) = lib.natives.get("windows") {
                let classifier_key = native_key.replace("${arch}", "64");
                if let Some(classifier_artifact) = downloads.classifiers.get(&classifier_key) {
                    add_artifact_spec(&mut specs, mc_folder, lib, classifier_artifact, priority);
                }
            }
        } else {
            // Legacy / no downloads block
            if lib.natives.is_empty() {
                add_maven_library_spec(&mut specs, mc_folder, lib, priority);
            } else if let Some(native_key) = lib.natives.get("windows") {
                let classifier = native_key.replace("${arch}", "64");
                add_legacy_native_spec(&mut specs, mc_folder, lib, &classifier, priority);
            }
        }
    }

    // 3. Asset Index + Asset Objects
    if let Some(asset_index_info) = &version_json.asset_index {
        let index_dest = mc_folder
            .join("assets")
            .join("indexes")
            .join(format!("{}.json", asset_index_info.id));

        // If index doesn't exist or is invalid, download it first or read existing
        let index_bytes = if !file_is_verified(
            &index_dest,
            asset_index_info.size,
            asset_index_info.sha1.as_deref(),
        ) {
            if let Some(url_str) = &asset_index_info.url {
                if let Ok(parsed_url) = Url::parse(url_str) {
                    let bytes =
                        race_get_bytes(client, &parsed_url, SourceKind::VersionMeta, priority)
                            .await?;
                    if let Some(parent) = index_dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&index_dest, &bytes);
                    Some(bytes)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            std::fs::read(&index_dest).ok()
        };

        if let Some(bytes) = index_bytes {
            if let Ok(asset_index) = serde_json::from_slice::<AssetIndexFile>(&bytes) {
                for object in asset_index.objects.values() {
                    let hash = &object.hash;
                    if hash.len() < 2 {
                        continue;
                    }
                    let sub = &hash[0..2];
                    let object_dest = mc_folder
                        .join("assets")
                        .join("objects")
                        .join(sub)
                        .join(hash);

                    let obj_url = format!("https://resources.download.minecraft.net/{sub}/{hash}");
                    let urls = build_urls(&obj_url, SourceKind::Assets, priority);

                    if !urls.is_empty() {
                        specs.push(DownloadSpec {
                            urls,
                            dest: object_dest,
                            size: Some(object.size),
                            sha1: Some(hash.clone()),
                            sha256: None,
                            md5: None,
                        });
                    }
                }
            }
        }
    }

    // 4. Logging Config XML
    if let Some(logging) = &version_json.logging {
        if let Some(client_log) = &logging.client {
            if let Some(log_file) = &client_log.file {
                if let (Some(id), Some(url_str)) = (&log_file.id, &log_file.url) {
                    let dest = mc_folder.join("assets").join("log_configs").join(id);

                    let urls = build_urls(url_str, SourceKind::Assets, priority);
                    if !urls.is_empty() {
                        specs.push(DownloadSpec {
                            urls,
                            dest,
                            size: log_file.size,
                            sha1: log_file.sha1.clone(),
                            sha256: None,
                            md5: None,
                        });
                    }
                }
            }
        }
    }

    Ok(specs)
}

fn add_artifact_spec(
    specs: &mut Vec<DownloadSpec>,
    mc_folder: &Path,
    lib: &Library,
    artifact: &Artifact,
    priority: SourcePriority,
) {
    let rel_path = if let Some(p) = &artifact.path {
        p.replace('\\', "/")
    } else {
        maven_coordinate_to_path(&lib.name)
    };

    let dest = mc_folder
        .join("libraries")
        .join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));

    let mut urls = Vec::new();
    if let Some(url_str) = &artifact.url {
        urls.extend(build_urls(url_str, SourceKind::Libraries, priority));
    } else if let Some(base_url) = &lib.url {
        let joined = format!("{}/{}", base_url.trim_end_matches('/'), rel_path);
        urls.extend(build_urls(&joined, SourceKind::Libraries, priority));
    } else {
        let joined = format!("https://libraries.minecraft.net/{}", rel_path);
        urls.extend(build_urls(&joined, SourceKind::Libraries, priority));
    }

    if !urls.is_empty() {
        specs.push(DownloadSpec {
            urls,
            dest,
            size: artifact.size,
            sha1: artifact.sha1.clone(),
            sha256: None,
            md5: None,
        });
    }
}

fn add_maven_library_spec(
    specs: &mut Vec<DownloadSpec>,
    mc_folder: &Path,
    lib: &Library,
    priority: SourcePriority,
) {
    let rel_path = maven_coordinate_to_path(&lib.name);
    let dest = mc_folder
        .join("libraries")
        .join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));

    let mut urls = Vec::new();
    if let Some(base_url) = &lib.url {
        let joined = format!("{}/{}", base_url.trim_end_matches('/'), rel_path);
        urls.extend(build_urls(&joined, SourceKind::Libraries, priority));
    } else {
        let joined = format!("https://libraries.minecraft.net/{}", rel_path);
        urls.extend(build_urls(&joined, SourceKind::Libraries, priority));
    }

    if !urls.is_empty() {
        specs.push(DownloadSpec {
            urls,
            dest,
            size: None,
            sha1: None,
            sha256: None,
            md5: None,
        });
    }
}

fn add_legacy_native_spec(
    specs: &mut Vec<DownloadSpec>,
    mc_folder: &Path,
    lib: &Library,
    classifier: &str,
    priority: SourcePriority,
) {
    // Append classifier to coordinate: group:artifact:version:classifier
    let parts: Vec<&str> = lib.name.split(':').collect();
    if parts.len() < 3 {
        return;
    }
    let native_coord = format!("{}:{}:{}:{}", parts[0], parts[1], parts[2], classifier);
    let rel_path = maven_coordinate_to_path(&native_coord);
    let dest = mc_folder
        .join("libraries")
        .join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));

    let mut urls = Vec::new();
    if let Some(base_url) = &lib.url {
        let joined = format!("{}/{}", base_url.trim_end_matches('/'), rel_path);
        urls.extend(build_urls(&joined, SourceKind::Libraries, priority));
    } else {
        let joined = format!("https://libraries.minecraft.net/{}", rel_path);
        urls.extend(build_urls(&joined, SourceKind::Libraries, priority));
    }

    if !urls.is_empty() {
        specs.push(DownloadSpec {
            urls,
            dest,
            size: None,
            sha1: None,
            sha256: None,
            md5: None,
        });
    }
}

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
    let version_json_bytes = race_get_bytes(
        client,
        &version_url,
        SourceKind::VersionMeta,
        net_settings.source_priority,
    )
    .await?;

    let mut version_json: VersionJson = serde_json::from_slice(&version_json_bytes)
        .map_err(|e| GameError::InvalidVersionJson(e.to_string()))?;

    // Set id = instance_name
    version_json.id = instance_name.to_string();

    std::fs::create_dir_all(&instance_dir)?;
    let serialized_json = serde_json::to_string_pretty(&version_json)
        .map_err(|e| GameError::InvalidVersionJson(e.to_string()))?;
    std::fs::write(&version_json_file, serialized_json)?;

    // Write default mclr.instance.json if missing
    let config_path = instance_dir.join("mclr.instance.json");
    if !config_path.exists() {
        let cfg = InstanceConfig::default();
        if let Ok(cfg_json) = serde_json::to_string_pretty(&cfg) {
            let _ = std::fs::write(config_path, cfg_json);
        }
    }

    // 3. Collect download specs
    let specs = collect_version_download_specs(
        client,
        mc_folder,
        instance_name,
        &version_json,
        net_settings.source_priority,
    )
    .await?;

    // Filter out verified files
    let unverified_specs: Vec<DownloadSpec> = specs
        .into_iter()
        .filter(|spec| !file_is_verified(&spec.dest, spec.size, spec.sha1.as_deref()))
        .collect();

    info!(
        "Installing vanilla {mc_version} as {instance_name}: {} files to download",
        unverified_specs.len()
    );

    if !unverified_specs.is_empty() {
        let downloader = Downloader::new(client.clone(), net_settings.clone());
        downloader.download_many(unverified_specs, task_ctx).await?;
    }

    // Return final InstanceInfo
    let (kind, loaders) = classify_instance(&version_json, mc_version);
    let state = crate::instance::InstanceState::Ok;
    let config = crate::instance::InstanceConfig::default();

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
    })
}

/// Repairs an existing Minecraft instance by checking and downloading missing/corrupted files.
/// Returns the number of files downloaded.
pub async fn repair_instance(
    client: &reqwest::Client,
    net_settings: &NetSettings,
    mc_folder: &Path,
    instance_id: &str,
    task_ctx: Option<(&TaskCenter, &str)>,
) -> Result<usize, GameError> {
    let resolved_json = load_resolved_version_json(mc_folder, instance_id)?;

    let specs = collect_version_download_specs(
        client,
        mc_folder,
        instance_id,
        &resolved_json,
        net_settings.source_priority,
    )
    .await?;

    let unverified_specs: Vec<DownloadSpec> = specs
        .into_iter()
        .filter(|spec| !file_is_verified(&spec.dest, spec.size, spec.sha1.as_deref()))
        .collect();

    let count = unverified_specs.len();
    info!("Repairing instance {instance_id}: {count} files missing or corrupted");

    if count > 0 {
        let downloader = Downloader::new(client.clone(), net_settings.clone());
        downloader.download_many(unverified_specs, task_ctx).await?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha1::{Digest, Sha1};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_maven_coordinate_to_path() {
        assert_eq!(
            maven_coordinate_to_path("net.sf.jopt-simple:jopt-simple:5.0.4"),
            "net/sf/jopt-simple/jopt-simple/5.0.4/jopt-simple-5.0.4.jar"
        );
        assert_eq!(
            maven_coordinate_to_path("org.lwjgl:lwjgl:3.3.1:natives-windows"),
            "org/lwjgl/lwjgl/3.3.1/lwjgl-3.3.1-natives-windows.jar"
        );
        assert_eq!(
            maven_coordinate_to_path("com.mojang:patchy:1.1@jar"),
            "com/mojang/patchy/1.1/patchy-1.1.jar"
        );
    }

    #[tokio::test]
    async fn test_install_and_repair_vanilla_wiremock() {
        let mock_server = MockServer::start().await;

        let client_jar_bytes = b"fake-minecraft-client-jar-data";
        let client_jar_sha1 = hex::encode(Sha1::digest(client_jar_bytes));

        let lib_bytes = b"fake-library-jar-content";
        let lib_sha1 = hex::encode(Sha1::digest(lib_bytes));

        let asset_obj_bytes = b"fake-sound-asset-bytes";
        let asset_obj_sha1 = hex::encode(Sha1::digest(asset_obj_bytes));

        let asset_index_json = serde_json::json!({
            "objects": {
                "minecraft/sounds/click.ogg": {
                    "hash": asset_obj_sha1,
                    "size": asset_obj_bytes.len()
                }
            }
        });
        let asset_index_bytes = serde_json::to_vec(&asset_index_json).unwrap();
        let asset_index_sha1 = hex::encode(Sha1::digest(&asset_index_bytes));

        let version_json_obj = serde_json::json!({
            "id": "1.20.1",
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "downloads": {
                "client": {
                    "sha1": client_jar_sha1,
                    "size": client_jar_bytes.len(),
                    "url": format!("{}/client.jar", mock_server.uri())
                }
            },
            "libraries": [
                {
                    "name": "org.example:testlib:1.0.0",
                    "downloads": {
                        "artifact": {
                            "path": "org/example/testlib/1.0.0/testlib-1.0.0.jar",
                            "sha1": lib_sha1,
                            "size": lib_bytes.len(),
                            "url": format!("{}/testlib-1.0.0.jar", mock_server.uri())
                        }
                    }
                }
            ],
            "assetIndex": {
                "id": "1.20",
                "sha1": asset_index_sha1,
                "size": asset_index_bytes.len(),
                "totalSize": asset_index_bytes.len(),
                "url": format!("{}/1.20.json", mock_server.uri())
            }
        });
        let version_json_bytes = serde_json::to_vec(&version_json_obj).unwrap();

        let manifest_json = serde_json::json!({
            "latest": {
                "release": "1.20.1",
                "snapshot": "23w31a"
            },
            "versions": [
                {
                    "id": "1.20.1",
                    "type": "release",
                    "url": format!("{}/1.20.1.json", mock_server.uri()),
                    "time": "2023-06-12T15:00:00+00:00",
                    "releaseTime": "2023-06-12T15:00:00+00:00"
                }
            ]
        });
        let manifest_bytes = serde_json::to_vec(&manifest_json).unwrap();

        Mock::given(method("GET"))
            .and(path("/1.20.1.json"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(version_json_bytes))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/client.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(client_jar_bytes.to_vec()))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/testlib-1.0.0.jar"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(lib_bytes.to_vec()))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/1.20.json"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(asset_index_bytes))
            .mount(&mock_server)
            .await;

        // Assets download uses https://resources.download.minecraft.net/sub/hash, so wiremock can also mock if needed,
        // but since rewrite_url rewrites resources.download.minecraft.net to bmclapi2.bangbang93.com,
        // we can test directly via collect_version_download_specs and repair_instance!

        let temp_dir = tempfile::tempdir().unwrap();
        let mc_folder = temp_dir.path();
        let cache_dir = mc_folder.join("cache");
        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::write(cache_dir.join("version_manifest_v2.json"), manifest_bytes).unwrap();

        // Pre-populate asset object so install_vanilla skips downloading it from internet
        let asset_sub = &asset_obj_sha1[0..2];
        let asset_dir = mc_folder.join("assets/objects").join(asset_sub);
        std::fs::create_dir_all(&asset_dir).unwrap();
        std::fs::write(asset_dir.join(&asset_obj_sha1), asset_obj_bytes).unwrap();
        let client = reqwest::Client::new();
        let net_settings = NetSettings {
            source_priority: SourcePriority::OfficialFirst,
            ..Default::default()
        };

        // Override MCLR_DATA_DIR so cache is read from temp_dir
        unsafe {
            std::env::set_var("MCLR_DATA_DIR", mc_folder.to_str().unwrap());
        }

        let instance_name = "test_instance";
        let info = install_vanilla(
            &client,
            &net_settings,
            mc_folder,
            "1.20.1",
            instance_name,
            None,
        )
        .await
        .unwrap();

        assert_eq!(info.id, "test_instance");
        assert_eq!(info.mc_version, "1.20.1");

        // Verify files exist on disk
        let client_jar = mc_folder.join("versions/test_instance/test_instance.jar");
        assert!(client_jar.exists());
        assert_eq!(std::fs::read(&client_jar).unwrap(), client_jar_bytes);

        let lib_jar = mc_folder.join("libraries/org/example/testlib/1.0.0/testlib-1.0.0.jar");
        assert!(lib_jar.exists());
        assert_eq!(std::fs::read(&lib_jar).unwrap(), lib_bytes);

        let asset_index = mc_folder.join("assets/indexes/1.20.json");
        assert!(asset_index.exists());

        // A second run (repair) should download 0 files because everything is already verified!
        let repaired = repair_instance(&client, &net_settings, mc_folder, instance_name, None)
            .await
            .unwrap();

        assert_eq!(repaired, 0);

        unsafe {
            std::env::remove_var("MCLR_DATA_DIR");
        }
    }
}
