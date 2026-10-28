//! Download spec generation, asset index resolution, and file verification.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use mclr_core::paths;
use mclr_core::settings::SourcePriority;
use mclr_net::{DownloadSpec, SourceKind, race_get_bytes, rewrite_url};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tracing::error;
use url::Url;

use crate::error::GameError;
use crate::instance::extract_mc_version;
use crate::version::{Artifact, Library, RuleContext, VersionJson, evaluate_rules};

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

/// Memo table type: `index path -> (mtime_ms, parsed index)`.
type AssetIndexMemo = Mutex<HashMap<String, (u64, Arc<AssetIndexFile>)>>;

/// Parsed asset indexes keyed by absolute index path, tagged with the file mtime used
/// to validate the entry. A 1.20 asset index is several megabytes of JSON and is parsed
/// on every repair/verify pass; keeping the parsed map warm removes both the re-read and
/// the re-parse for the whole process lifetime.
static ASSET_INDEX_MEMO: LazyLock<AssetIndexMemo> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Upper bound on memo entries; reaching it clears the whole map (cheap, and the next
/// call simply reparses once).
const ASSET_INDEX_MEMO_MAX_ENTRIES: usize = 4;

fn file_mtime_ms(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn remember_asset_index(path: &Path, index: &Arc<AssetIndexFile>) {
    let Some(mtime_ms) = file_mtime_ms(path) else {
        return;
    };
    let Ok(mut memo) = ASSET_INDEX_MEMO.lock() else {
        return;
    };
    if memo.len() > ASSET_INDEX_MEMO_MAX_ENTRIES {
        memo.clear();
    }
    memo.insert(
        path.to_string_lossy().to_string(),
        (mtime_ms, Arc::clone(index)),
    );
}

fn load_asset_index_memoized(path: &Path) -> Option<Arc<AssetIndexFile>> {
    let mtime_ms = file_mtime_ms(path)?;
    let key = path.to_string_lossy().to_string();

    if let Ok(memo) = ASSET_INDEX_MEMO.lock() {
        if let Some((cached_mtime_ms, index)) = memo.get(&key) {
            if *cached_mtime_ms == mtime_ms {
                return Some(Arc::clone(index));
            }
        }
    }

    let bytes = std::fs::read(path).ok()?;
    let index = Arc::new(serde_json::from_slice::<AssetIndexFile>(&bytes).ok()?);

    if let Ok(mut memo) = ASSET_INDEX_MEMO.lock() {
        if memo.len() > ASSET_INDEX_MEMO_MAX_ENTRIES {
            memo.clear();
        }
        memo.insert(key, (mtime_ms, Arc::clone(&index)));
    }

    Some(index)
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
pub(crate) fn build_urls(
    primary_url: &str,
    kind: SourceKind,
    priority: SourcePriority,
) -> Vec<Url> {
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
pub(crate) fn file_is_verified(
    path: &Path,
    expected_size: Option<u64>,
    expected_sha1: Option<&str>,
) -> bool {
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

        // Everything below touches a 3-4 MB JSON file, hashes it, or expands ~4000
        // object entries: none of that belongs on an async worker.
        let probe_dest = index_dest.clone();
        let expected_size = asset_index_info.size;
        let expected_sha1 = asset_index_info.sha1.clone();
        let cached_index = tokio::task::spawn_blocking(move || {
            if file_is_verified(&probe_dest, expected_size, expected_sha1.as_deref()) {
                load_asset_index_memoized(&probe_dest)
            } else {
                None
            }
        })
        .await
        .map_err(|e| GameError::Io(std::io::Error::other(e.to_string())))?;

        // If the index doesn't exist or is invalid, download it first; otherwise the
        // memoized parse above is authoritative.
        let asset_index = match cached_index {
            Some(index) => Some(index),
            None => match asset_index_info.url.clone() {
                Some(url_str) => match Url::parse(&url_str) {
                    Ok(parsed_url) => {
                        let bytes =
                            race_get_bytes(client, &parsed_url, SourceKind::VersionMeta, priority)
                                .await?;
                        if let Some(parent) = index_dest.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&index_dest, &bytes);
                        let parse_dest = index_dest.clone();
                        tokio::task::spawn_blocking(move || {
                            let parsed = serde_json::from_slice::<AssetIndexFile>(&bytes)
                                .ok()
                                .map(Arc::new);
                            if let Some(index) = &parsed {
                                remember_asset_index(&parse_dest, index);
                            }
                            parsed
                        })
                        .await
                        .map_err(|e| GameError::Io(std::io::Error::other(e.to_string())))?
                    }
                    Err(_) => None,
                },
                None => None,
            },
        };

        if let Some(asset_index) = asset_index {
            let folder = mc_folder.to_path_buf();
            specs = tokio::task::spawn_blocking(move || {
                let mut specs = specs;
                for object in asset_index.objects.values() {
                    let hash = &object.hash;
                    if hash.len() < 2 {
                        continue;
                    }
                    let sub = &hash[0..2];
                    let object_dest = folder.join("assets").join("objects").join(sub).join(hash);

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
                specs
            })
            .await
            .map_err(|e| GameError::Io(std::io::Error::other(e.to_string())))?;
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerifiedCacheEntry {
    pub size: u64,
    #[serde(rename = "mtimeMs")]
    pub mtime_ms: u64,
}

pub type VerifiedCache = HashMap<String, VerifiedCacheEntry>;

pub(crate) fn load_verified_cache() -> VerifiedCache {
    let path = paths::cache_dir().join("verified.json");
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(cache) = serde_json::from_str(&data) {
                return cache;
            }
        }
    }
    HashMap::new()
}

pub(crate) fn save_verified_cache(cache: &VerifiedCache) {
    let cache_dir = paths::cache_dir();
    let _ = std::fs::create_dir_all(&cache_dir);
    let path = cache_dir.join("verified.json");
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = std::fs::write(path, json);
    }
}

pub(crate) fn calculate_file_sha1(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(_) => return None,
        }
    }
    Some(hex::encode(hasher.finalize()))
}

/// One entry awaiting a SHA1 hash: `(spec, cache key, size, mtime_ms, expected sha1)`.
type PendingHash = (DownloadSpec, String, u64, u64, String);

/// Blocking half of [`verify_specs_cached`]: loads the cache and stats every spec.
///
/// Kept separate so the whole thing can run on the blocking pool — a real instance has
/// ~4000 paths to stat plus a JSON cache read, which must not stall an async worker.
fn scan_specs_blocking(
    specs: Vec<DownloadSpec>,
) -> (Vec<DownloadSpec>, Vec<PendingHash>, VerifiedCache) {
    let cache = load_verified_cache();
    let mut unverified = Vec::new();
    let mut candidates_to_hash = Vec::new();

    for spec in specs {
        if !spec.dest.exists() {
            unverified.push(spec);
            continue;
        }

        let metadata = match std::fs::metadata(&spec.dest) {
            Ok(m) => m,
            Err(_) => {
                unverified.push(spec);
                continue;
            }
        };
        let file_size = metadata.len();
        let mtime_ms = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if let Some(expected_size) = spec.size {
            if file_size != expected_size {
                unverified.push(spec);
                continue;
            }
        }

        if let Some(expected_sha1) = spec.sha1.clone() {
            let dest_key = spec.dest.to_string_lossy().to_string();
            if let Some(entry) = cache.get(&dest_key) {
                if entry.size == file_size && entry.mtime_ms == mtime_ms {
                    continue;
                }
            }
            candidates_to_hash.push((spec, dest_key, file_size, mtime_ms, expected_sha1));
        }
    }

    (unverified, candidates_to_hash, cache)
}

pub(crate) async fn verify_specs_cached(specs: Vec<DownloadSpec>) -> Vec<DownloadSpec> {
    let scanned = tokio::task::spawn_blocking(move || scan_specs_blocking(specs)).await;
    let (mut unverified, candidates_to_hash, mut cache) = match scanned {
        Ok(scanned) => scanned,
        Err(e) => {
            // The scan panicked, so the specs are gone; reporting "nothing to download"
            // is the only non-destructive option left and the panic is logged above.
            error!("Repair: file verification scan failed: {e}");
            return Vec::new();
        }
    };

    if !candidates_to_hash.is_empty() {
        for batch in candidates_to_hash.chunks(64) {
            let tasks: Vec<_> = batch
                .iter()
                .map(|(spec, dest_key, file_size, mtime_ms, expected_sha1)| {
                    let path = spec.dest.clone();
                    let exp_sha1 = expected_sha1.clone();
                    let dest_key = dest_key.clone();
                    let file_size = *file_size;
                    let mtime_ms = *mtime_ms;
                    let spec = spec.clone();
                    tokio::task::spawn_blocking(move || {
                        let passed = calculate_file_sha1(&path)
                            .map(|actual| actual.eq_ignore_ascii_case(&exp_sha1))
                            .unwrap_or(false);
                        (spec, dest_key, file_size, mtime_ms, passed)
                    })
                })
                .collect();

            for task in tasks {
                if let Ok((spec, dest_key, file_size, mtime_ms, passed)) = task.await {
                    if passed {
                        cache.insert(
                            dest_key,
                            VerifiedCacheEntry {
                                size: file_size,
                                mtime_ms,
                            },
                        );
                    } else {
                        unverified.push(spec);
                    }
                }
            }
        }
        let _ = tokio::task::spawn_blocking(move || save_verified_cache(&cache)).await;
    }

    unverified
}
