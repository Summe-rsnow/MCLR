//! Local mod management: scanning, parsing, hash identification, toggling, and updates.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use mclr_core::paths;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use specta::Type;
use zip::ZipArchive;

use crate::curseforge::CurseForgeClient;
use crate::error::CompError;
use crate::model::CompFile;
use crate::modrinth::ModrinthClient;
use crate::resolver::select_best_file;

/// Information about a locally installed mod in the instance's `mods` folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalModInfo {
    pub file_name: String,
    pub path: String,
    pub enabled: bool,
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub loader: String,
    pub sha1: String,
    pub murmur2: u32,
    pub size: u64,
    pub project_id: Option<String>,
    pub update_file: Option<CompFile>,
    /// Path to the mod's own artwork, extracted from the jar and cached on disk.
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalModCacheItem {
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub loader: String,
    pub sha1: String,
    pub murmur2: u32,
    pub size: u64,
    pub mtime_secs: u64,
    pub project_id: Option<String>,
    pub update_file: Option<CompFile>,
    /// Cached icon path inside the launcher cache dir.
    #[serde(default)]
    pub icon: Option<String>,
    /// Artwork-selection revision the cached icon was produced with; older entries are
    /// re-parsed so a rule change reaches jars that were never modified.
    #[serde(default)]
    pub icon_rev: u32,
    /// Unix seconds of the last Modrinth project-id lookup for this file (`0` = never).
    #[serde(default)]
    pub cached_at_secs: u64,
}

/// Chunk size used when hashing a mod file without loading it into memory. This is the only
/// heap buffer the hashing path allocates, so it stays small enough to be irrelevant next to
/// the file it streams.
const HASH_CHUNK: usize = 128 * 1024;

/// Bytes stripped before CurseForge fingerprinting: `\t`, `\n`, `\r` and space.
#[inline]
fn is_curseforge_whitespace(b: u8) -> bool {
    b == 9 || b == 10 || b == 13 || b == 32
}

/// Computes the exact CurseForge MurmurHash2 (32-bit, seed = 1, m = 0x5bd1e995, r = 24)
/// over `data` with whitespace bytes (b'\t', b'\n', b'\r', b' ' -> 9, 10, 13, 32) stripped.
pub fn compute_curseforge_murmur2(data: &[u8]) -> u32 {
    let filtered: Vec<u8> = data
        .iter()
        .copied()
        .filter(|&b| !is_curseforge_whitespace(b))
        .collect();
    murmur2_fold(&filtered)
}

/// Streaming counterpart of [`compute_curseforge_murmur2`]: hashes `r` in `HASH_CHUNK`
/// chunks so the unfiltered file is never resident. The result is byte-for-byte identical to
/// the slice version.
///
/// MurmurHash2 mixes the *filtered* length into its seed (`1 ^ len`), so blocks can only be
/// folded once the stream ends; the whitespace-stripped bytes are retained (a single
/// allocation, smaller than the source) instead of the two buffers the slice path needs.
/// Callers that already know the filtered length (for example `hash_file`, which counts it
/// while hashing SHA1) should use `murmur2_stream_with_len` instead, which allocates nothing
/// beyond the read buffer.
pub fn compute_curseforge_murmur2_reader(r: &mut impl Read) -> std::io::Result<u32> {
    let mut filtered: Vec<u8> = Vec::new();
    let mut chunk = vec![0u8; HASH_CHUNK];
    loop {
        let n = r.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        filtered.extend(
            chunk[..n]
                .iter()
                .copied()
                .filter(|&b| !is_curseforge_whitespace(b)),
        );
    }
    Ok(murmur2_fold(&filtered))
}

/// Folds an already whitespace-stripped buffer with CurseForge's MurmurHash2 variant.
fn murmur2_fold(filtered: &[u8]) -> u32 {
    let m: u32 = 0x5bd1e995;
    let r: u32 = 24;

    let len = filtered.len();
    let mut h: u32 = 1 ^ (len as u32);
    let mut i = 0;

    while i + 4 <= len {
        let mut k = (filtered[i] as u32)
            | ((filtered[i + 1] as u32) << 8)
            | ((filtered[i + 2] as u32) << 16)
            | ((filtered[i + 3] as u32) << 24);

        k = k.wrapping_mul(m);
        k ^= k >> r;
        k = k.wrapping_mul(m);

        h = h.wrapping_mul(m);
        h ^= k;

        i += 4;
    }

    let rem = len - i;
    if rem == 3 {
        h ^= (filtered[i] as u32) | ((filtered[i + 1] as u32) << 8);
        h ^= (filtered[i + 2] as u32) << 16;
        h = h.wrapping_mul(m);
    } else if rem == 2 {
        h ^= (filtered[i] as u32) | ((filtered[i + 1] as u32) << 8);
        h = h.wrapping_mul(m);
    } else if rem == 1 {
        h ^= filtered[i] as u32;
        h = h.wrapping_mul(m);
    }

    h ^= h >> 13;
    h = h.wrapping_mul(m);
    h ^= h >> 15;

    h
}

/// Zero-allocation streaming counterpart of [`compute_curseforge_murmur2`] for callers that
/// already know the whitespace-stripped length (a first pass over the file counts it while
/// computing SHA1).
///
/// Bytes are read through one [`HASH_CHUNK`] buffer and folded 4 at a time with a 4-byte
/// stack-carried alignment tail, so no per-file allocation happens at all: the fingerprint
/// of a 500 MB jar costs exactly one 128 KiB buffer.
fn murmur2_stream_with_len(r: &mut impl Read, filtered_len: u32) -> std::io::Result<u32> {
    const M: u32 = 0x5bd1e995;
    const R: u32 = 24;

    let mut h: u32 = 1 ^ filtered_len;
    let mut tail = [0u8; 4];
    let mut tail_len = 0usize;
    let mut chunk = vec![0u8; HASH_CHUNK];

    loop {
        let n = r.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        for &b in &chunk[..n] {
            if is_curseforge_whitespace(b) {
                continue;
            }
            tail[tail_len] = b;
            tail_len += 1;
            if tail_len == 4 {
                let mut k = u32::from_le_bytes(tail);
                k = k.wrapping_mul(M);
                k ^= k >> R;
                k = k.wrapping_mul(M);
                h = h.wrapping_mul(M);
                h ^= k;
                tail_len = 0;
            }
        }
    }

    if tail_len == 3 {
        h ^= (tail[0] as u32) | ((tail[1] as u32) << 8);
        h ^= (tail[2] as u32) << 16;
        h = h.wrapping_mul(M);
    } else if tail_len == 2 {
        h ^= (tail[0] as u32) | ((tail[1] as u32) << 8);
        h = h.wrapping_mul(M);
    } else if tail_len == 1 {
        h ^= tail[0] as u32;
        h = h.wrapping_mul(M);
    }

    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;

    Ok(h)
}

/// Unfolds Java manifest lines according to RFC 822 / JAR specification (72-column limit,
/// continuation lines beginning with a single space ' ').
pub fn unfold_manifest(content: &str) -> String {
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim_end_matches('\r');
        if let Some(rest) = trimmed.strip_prefix(' ') {
            result.push_str(rest);
        } else {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(trimmed);
        }
    }
    result
}

/// Reads a specific attribute from unfolded manifest text (case-insensitive).
pub fn get_manifest_attribute(manifest_text: &str, attr_name: &str) -> Option<String> {
    let unfolded = unfold_manifest(manifest_text);
    let attr_prefix = format!("{}:", attr_name.to_ascii_lowercase());
    for line in unfolded.lines() {
        let trimmed = line.trim();
        if trimmed.to_ascii_lowercase().starts_with(&attr_prefix) {
            let val = trimmed[attr_prefix.len()..].trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Parsed metadata extracted from a mod jar.
#[derive(Debug, Clone, Default)]
pub struct ModMetadata {
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub loader: String,
    /// Raw PNG bytes of the mod's own artwork, when the jar ships one.
    pub icon: Option<Vec<u8>>,
}

fn read_entry_text<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

/// Reads a small archive entry fully. Icons are capped at 2 MiB so a crafted jar cannot
/// force a large allocation.
fn read_entry_bytes<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<Vec<u8>> {
    const MAX_ICON_BYTES: u64 = 2 * 1024 * 1024;
    let mut entry = archive.by_name(name).ok()?;
    if entry.size() > MAX_ICON_BYTES {
        return None;
    }
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

/// Bumped whenever the artwork selection changes, so caches written by an older rule (for
/// example a wide `logo.png` banner, or art scaled by the old resizer) are not reused.
const ICON_CACHE_REV: u32 = 4;

/// Largest square artwork kept as-is; bigger squares are still accepted (the cache scales
/// them), just ranked below anything that fits.
const MAX_ICON_SIDE: u32 = 512;

/// PNG dimensions straight from the IHDR chunk: 8-byte signature, 4-byte length, `IHDR`,
/// then width and height as big-endian u32 — no decoding needed.
fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((width, height))
}

/// Finds the mod's artwork inside the jar.
///
/// Fabric and Quilt store it under `assets/<modid>/icon.png`; Forge and NeoForge point at it
/// from `logoFile`; many jars ship `catalogue_icon.png` / `icon.png` / `logo.png` / `pack.png`
/// at the root. Jars often ship a wide marketing banner as `logo.png` (YUNG's mods:
/// 1200×240), so a square image always wins over whatever the priority order found first —
/// the row renders a square tile and would otherwise crop the banner.
fn find_mod_icon<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    mod_id: &str,
    logo_file: Option<&str>,
) -> Option<Vec<u8>> {
    let mut candidates: Vec<String> = Vec::new();
    if let Some(logo) = logo_file.map(str::trim).filter(|l| !l.is_empty()) {
        candidates.push(logo.trim_start_matches('/').to_string());
    }
    if !mod_id.is_empty() {
        candidates.push(format!("assets/{mod_id}/icon.png"));
        candidates.push(format!("assets/{mod_id}/logo.png"));
    }
    candidates.push("catalogue_icon.png".to_string());
    candidates.push("icon.png".to_string());
    candidates.push("logo.png".to_string());
    candidates.push("pack.png".to_string());

    let mut best_square: Option<(u32, Vec<u8>)> = None;
    let mut smallest_square: Option<(u32, Vec<u8>)> = None;
    let mut first_present: Option<Vec<u8>> = None;

    for candidate in candidates {
        let Some(bytes) = read_entry_bytes(archive, &candidate) else {
            continue;
        };
        if let Some((width, height)) = png_dimensions(&bytes)
            && width == height
            && width > 0
        {
            if width <= MAX_ICON_SIDE {
                if best_square.as_ref().is_none_or(|(side, _)| width > *side) {
                    best_square = Some((width, bytes));
                }
            } else if smallest_square
                .as_ref()
                .is_none_or(|(side, _)| width < *side)
            {
                smallest_square = Some((width, bytes));
            }
            continue;
        }
        if first_present.is_none() {
            first_present = Some(bytes);
        }
    }

    best_square
        .or(smallest_square)
        .map(|(_, bytes)| bytes)
        .or(first_present)
}

/// Parses mod metadata from zip archive bytes of a jar file.
pub fn parse_mod_metadata(bytes: &[u8], fallback_name: &str) -> ModMetadata {
    match ZipArchive::new(Cursor::new(bytes)) {
        Ok(mut archive) => parse_archive_metadata(&mut archive, fallback_name),
        Err(_) => fallback_metadata(fallback_name),
    }
}

/// Parses mod metadata straight from a jar on disk, reading only the handful of entries that
/// carry metadata. Returns `None` when the file cannot be opened or is not a zip archive.
pub fn parse_mod_metadata_path(path: &Path, fallback_name: &str) -> Option<ModMetadata> {
    let file = fs::File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    Some(parse_archive_metadata(&mut archive, fallback_name))
}

/// Metadata for a jar whose format could not be recognised (or could not be opened).
fn fallback_metadata(fallback_name: &str) -> ModMetadata {
    ModMetadata {
        mod_id: fallback_name.to_string(),
        name: fallback_name.to_string(),
        version: String::new(),
        description: None,
        authors: Vec::new(),
        loader: "Unknown".to_string(),
        icon: None,
    }
}

fn parse_archive_metadata<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    fallback_name: &str,
) -> ModMetadata {
    // 1. Check fabric.mod.json
    if let Some(text) = read_entry_text(archive, "fabric.mod.json")
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
    {
        let id = val
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(fallback_name)
            .to_string();
        let name = val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let version = val
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let description = val
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string());

        let mut authors = Vec::new();
        if let Some(arr) = val.get("authors").and_then(|v| v.as_array()) {
            for a in arr {
                if let Some(s) = a.as_str() {
                    authors.push(s.to_string());
                } else if let Some(obj) = a.as_object()
                    && let Some(s) = obj.get("name").and_then(|v| v.as_str())
                {
                    authors.push(s.to_string());
                }
            }
        }

        let icon_file = val.get("icon").and_then(|v| v.as_str());
        let icon = find_mod_icon(archive, &id, icon_file);
        return ModMetadata {
            mod_id: id,
            name,
            version,
            description,
            authors,
            loader: "Fabric".to_string(),
            icon,
        };
    }

    // 2. Check quilt.mod.json
    if let Some(text) = read_entry_text(archive, "quilt.mod.json")
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
    {
        let ql = val.get("quilt_loader");
        let id = ql
            .and_then(|q| q.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or(fallback_name)
            .to_string();
        let version = ql
            .and_then(|q| q.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let meta = ql.and_then(|q| q.get("metadata"));
        let name = meta
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let description = meta
            .and_then(|m| m.get("description"))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string());

        let mut authors = Vec::new();
        if let Some(contrib) = meta.and_then(|m| m.get("contributors")) {
            if let Some(obj) = contrib.as_object() {
                for k in obj.keys() {
                    authors.push(k.clone());
                }
            } else if let Some(arr) = contrib.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        authors.push(s.to_string());
                    }
                }
            }
        }

        let icon_file = ql.and_then(|q| q.get("icon")).and_then(|v| v.as_str());
        let icon = find_mod_icon(archive, &id, icon_file);
        return ModMetadata {
            mod_id: id,
            name,
            version,
            description,
            authors,
            loader: "Quilt".to_string(),
            icon,
        };
    }

    // 3. Check META-INF/neoforge.mods.toml and META-INF/mods.toml
    let (toml_entry_name, loader) = if archive.by_name("META-INF/neoforge.mods.toml").is_ok() {
        ("META-INF/neoforge.mods.toml", "NeoForge")
    } else if archive.by_name("META-INF/mods.toml").is_ok() {
        ("META-INF/mods.toml", "Forge")
    } else {
        ("", "")
    };

    let toml_text = if !toml_entry_name.is_empty() {
        read_entry_text(archive, toml_entry_name)
    } else {
        None
    };

    if let Some(text) = toml_text
        && let Ok(toml_val) = toml::from_str::<toml::Value>(&text)
        && let Some(mods_arr) = toml_val.get("mods").and_then(|v| v.as_array())
        && let Some(first_mod) = mods_arr.first()
    {
        let mod_id = first_mod
            .get("modId")
            .and_then(|v| v.as_str())
            .unwrap_or(fallback_name)
            .to_string();
        let name = first_mod
            .get("displayName")
            .or_else(|| first_mod.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(&mod_id)
            .to_string();
        let description = first_mod
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string());

        let mut version = first_mod
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Handle ${file.jarVersion} substitution from MANIFEST.MF
        if (version.contains("${file.jarVersion}") || version.is_empty())
            && let Some(m_text) = read_entry_text(archive, "META-INF/MANIFEST.MF")
            && let Some(impl_ver) = get_manifest_attribute(&m_text, "Implementation-Version")
        {
            if version.is_empty() {
                version = impl_ver;
            } else {
                version = version.replace("${file.jarVersion}", &impl_ver);
            }
        }

        // Clean version if unresolved placeholder remains
        if version.contains("${") {
            version = String::new();
        }

        let authors = first_mod
            .get("authors")
            .and_then(|v| v.as_str())
            .map(|s| {
                s.split(',')
                    .map(|a| a.trim().to_string())
                    .filter(|a| !a.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let icon = find_mod_icon(
            archive,
            &mod_id,
            first_mod.get("logoFile").and_then(|v| v.as_str()),
        );
        return ModMetadata {
            mod_id,
            name,
            version,
            description,
            authors,
            loader: loader.to_string(),
            icon,
        };
    }

    // 4. Check mcmod.info (legacy Forge)
    if let Some(text) = read_entry_text(archive, "mcmod.info")
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
    {
        let first_mod = if let Some(arr) = val.as_array() {
            arr.first()
        } else if let Some(arr) = val.get("modList").and_then(|v| v.as_array()) {
            arr.first()
        } else {
            None
        };

        if let Some(m) = first_mod {
            let mod_id = m
                .get("modid")
                .and_then(|v| v.as_str())
                .unwrap_or(fallback_name)
                .to_string();
            let name = m
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&mod_id)
                .to_string();
            let version = m
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let description = m
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string());

            let mut authors = Vec::new();
            if let Some(arr) = m
                .get("authorList")
                .or_else(|| m.get("authors"))
                .and_then(|v| v.as_array())
            {
                for a in arr {
                    if let Some(s) = a.as_str() {
                        authors.push(s.to_string());
                    }
                }
            }

            let icon = find_mod_icon(archive, &mod_id, m.get("logoFile").and_then(|v| v.as_str()));
            return ModMetadata {
                mod_id,
                name,
                version,
                description,
                authors,
                loader: "Forge".to_string(),
                icon,
            };
        }
    }

    // 5. Fallback: unrecognised format, but the jar may still ship artwork.
    let mut meta = fallback_metadata(fallback_name);
    meta.icon = find_mod_icon(archive, "", None);
    meta
}

/// Number of hashes sent per Modrinth `version_files` request.
pub const MODRINTH_LOOKUP_BATCH: usize = 100;

/// How long a failed Modrinth project-id lookup is remembered before retrying.
const PROJECT_LOOKUP_TTL_SECS: u64 = 6 * 3600;

fn current_timestamp_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Path-keyed cache map shared by every mods directory.
type ModCacheMap = HashMap<String, LocalModCacheItem>;

/// In-memory store: `cache file -> (file mtime, entries)`.
type ModCacheStore = HashMap<PathBuf, (Option<SystemTime>, ModCacheMap)>;

/// In-memory mirror of `localmods.json` instances: `cache file -> (file mtime, entries)`.
///
/// Keying by the cache file keeps several data directories (and tests) isolated, while the
/// stored mtime makes entries written by another process visible automatically. The lock is
/// only ever held for a `HashMap` lookup/clone — every `fs` call, JSON parse and JSON encode
/// happens outside it.
static LOCAL_MOD_CACHE: LazyLock<Mutex<ModCacheStore>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn mod_cache_file() -> PathBuf {
    paths::cache_dir().join("localmods.json")
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn lock_mod_cache() -> std::sync::MutexGuard<'static, ModCacheStore> {
    LOCAL_MOD_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn load_cache_from(cache_file: &Path) -> ModCacheMap {
    let disk_mtime = file_mtime(cache_file);

    // Fast path: the on-disk file is unchanged since the last load/save, so serve the
    // in-memory copy. Cloning under the lock is a few microseconds even for hundreds of mods.
    if disk_mtime.is_some() {
        let guard = lock_mod_cache();
        if let Some((cached_mtime, map)) = guard.get(cache_file)
            && *cached_mtime == disk_mtime
        {
            return map.clone();
        }
    }

    // Miss or external modification: read and parse with no lock held.
    let map: ModCacheMap = fs::read_to_string(cache_file)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();

    lock_mod_cache().insert(cache_file.to_path_buf(), (disk_mtime, map.clone()));
    map
}

fn save_cache_to(cache_file: &Path, cache: &ModCacheMap) {
    if let Some(parent) = cache_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(cache_file, json);
    }
    // Store the mtime produced by this write so the next `load_cache_from` stays in memory.
    let new_mtime = file_mtime(cache_file);
    lock_mod_cache().insert(cache_file.to_path_buf(), (new_mtime, cache.clone()));
}

fn load_cache() -> ModCacheMap {
    load_cache_from(&mod_cache_file())
}

fn save_cache(cache: &ModCacheMap) {
    save_cache_to(&mod_cache_file(), cache);
}

/// Moves a cache entry from `current_path` to `target_path` after a toggle renamed the file,
/// so the renamed jar keeps its hashes, metadata and icon instead of being re-hashed.
///
/// The entry is only migrated when its recorded `size` still matches the file on disk; a
/// mismatch means the file changed and a full re-scan is the correct answer. Returns whether
/// the entry was migrated (and the cache file rewritten).
fn migrate_cache_entry(
    cache_file: &Path,
    current_path: &Path,
    target_path: &Path,
    size: u64,
    mtime_secs: u64,
) -> bool {
    let current_key = current_path.to_string_lossy().to_string();
    let target_key = target_path.to_string_lossy().to_string();
    if current_key == target_key {
        return false;
    }

    let mut cache = load_cache_from(cache_file);
    let Some(mut item) = cache
        .get(&current_key)
        .filter(|item| item.size == size)
        .cloned()
    else {
        return false;
    };
    cache.remove(&current_key);
    item.mtime_secs = mtime_secs;
    cache.insert(target_key, item);
    save_cache_to(cache_file, &cache);
    true
}

/// SHA1s of `mods` whose Modrinth project id should be resolved now: hashes never looked up,
/// and hashes whose last lookup found nothing and is older than six hours. Already-resolved
/// mods are skipped (a project id never changes for a given SHA1).
pub fn mods_needing_project_lookup(mods: &[LocalModInfo]) -> Vec<String> {
    if mods.is_empty() {
        return Vec::new();
    }
    let cache = load_cache();
    let now = current_timestamp_secs();
    let mut seen: HashSet<&str> = HashSet::new();
    let mut pending = Vec::new();
    for m in mods {
        if m.project_id.is_some() || m.sha1.is_empty() || !seen.insert(m.sha1.as_str()) {
            continue;
        }
        let expired = match cache.get(&m.path) {
            Some(item) => now.saturating_sub(item.cached_at_secs) >= PROJECT_LOOKUP_TTL_SECS,
            None => true,
        };
        if expired {
            pending.push(m.sha1.clone());
        }
    }
    pending
}

/// Records the outcome of the Modrinth lookups for `lookups` (`sha1 -> project id`, a `None`
/// value meaning "looked up, not on Modrinth"), refreshing the six-hour retry window.
pub fn record_project_ids(lookups: &HashMap<String, Option<String>>) {
    if lookups.is_empty() {
        return;
    }
    let mut cache = load_cache();
    let now = current_timestamp_secs();
    let mut modified = false;
    for item in cache.values_mut() {
        if let Some(project_id) = lookups.get(&item.sha1) {
            item.project_id = project_id.clone();
            item.cached_at_secs = now;
            modified = true;
        }
    }
    if modified {
        save_cache(&cache);
    }
}

/// Streams a jar from disk to compute its SHA1 and CurseForge fingerprint without ever
/// holding the whole file (or even the whitespace-filtered file) in memory.
///
/// Two passes over one file handle:
///
/// 1. SHA1 of every byte plus a count of the bytes CurseForge keeps (whitespace stripped),
/// 2. MurmurHash2 re-read with `1 ^ filtered_len` as the seed.
///
/// The second pass hits the OS page cache warmed by the first, and the only heap buffer in
/// either pass is the 128 KiB [`HASH_CHUNK`] read buffer — a 500 MB jar costs the same
/// memory as a 1 KB one.
fn hash_file(path: &Path) -> std::io::Result<(String, u32)> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; HASH_CHUNK];
    let mut filtered_len: u32 = 0;

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        hasher.update(chunk);
        filtered_len = filtered_len.wrapping_add(
            chunk
                .iter()
                .filter(|&&b| !is_curseforge_whitespace(b))
                .count() as u32,
        );
    }
    let sha1 = hex::encode(hasher.finalize());

    file.seek(std::io::SeekFrom::Start(0))?;
    let murmur2 = murmur2_stream_with_len(&mut file, filtered_len)?;
    Ok((sha1, murmur2))
}

/// A mod file whose cache entry is missing or stale: it still needs hashing, metadata
/// parsing and (possibly) icon extraction.
struct PendingMod {
    file_name: String,
    path: PathBuf,
    path_key: String,
    enabled: bool,
    size: u64,
    mtime_secs: u64,
}

/// Hashes and parses one cache-miss mod, returning its cache entry and public description.
fn scan_pending_mod(pending: &PendingMod) -> Option<(LocalModCacheItem, LocalModInfo)> {
    let (sha1, murmur2) = hash_file(&pending.path).ok()?;

    let fallback_name = if let Some(stem) = pending.file_name.strip_suffix(".jar.disabled") {
        stem
    } else if let Some(stem) = pending.file_name.strip_suffix(".jar") {
        stem
    } else {
        pending.file_name.as_str()
    };

    let mod_meta = parse_mod_metadata_path(&pending.path, fallback_name)
        .unwrap_or_else(|| fallback_metadata(fallback_name));

    // Artwork is only decoded on a cache miss, and is keyed by content hash so an
    // updated jar always gets a fresh image.
    let icon = mod_meta.icon.as_deref().and_then(|bytes| {
        crate::resources::cache_icon_png(bytes, &format!("{sha1}:icon-v{ICON_CACHE_REV}"))
    });

    let cache_item = LocalModCacheItem {
        mod_id: mod_meta.mod_id.clone(),
        name: mod_meta.name.clone(),
        version: mod_meta.version.clone(),
        description: mod_meta.description.clone(),
        authors: mod_meta.authors.clone(),
        loader: mod_meta.loader.clone(),
        sha1: sha1.clone(),
        murmur2,
        size: pending.size,
        mtime_secs: pending.mtime_secs,
        project_id: None,
        update_file: None,
        icon: icon.clone(),
        icon_rev: ICON_CACHE_REV,
        cached_at_secs: 0,
    };

    let info = LocalModInfo {
        file_name: pending.file_name.clone(),
        path: pending.path_key.clone(),
        enabled: pending.enabled,
        mod_id: mod_meta.mod_id,
        name: mod_meta.name,
        version: mod_meta.version,
        description: mod_meta.description,
        authors: mod_meta.authors,
        loader: mod_meta.loader,
        sha1,
        murmur2,
        size: pending.size,
        project_id: None,
        update_file: None,
        icon,
    };

    Some((cache_item, info))
}

/// A freshly scanned mod plus the cache key it must be stored under.
type ScannedMod = (String, LocalModCacheItem, LocalModInfo);

/// Scans cache-miss mods, spreading the work over up to four worker threads.
///
/// Hashing and ZIP parsing are CPU- and disk-bound yet fully independent per file, so a mods
/// folder full of newly added jars would otherwise serialise on a single core. The pool is
/// capped at four workers: each holds one 128 KiB hash buffer, and saturating every core
/// would starve the async runtime that called the scan.
///
/// Results carry their `path_key` so the caller merges them into the cache without a shared
/// lock; a panicking worker only drops its own slice.
fn scan_pending_mods(pending: &[PendingMod]) -> Vec<ScannedMod> {
    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(4);

    if pending.len() <= 1 || workers <= 1 {
        return pending
            .iter()
            .filter_map(|p| {
                scan_pending_mod(p).map(|(item, info)| (p.path_key.clone(), item, info))
            })
            .collect();
    }

    let chunk_size = pending.len().div_ceil(workers);
    std::thread::scope(|scope| {
        let handles: Vec<_> = pending
            .chunks(chunk_size)
            .map(|slice| {
                scope.spawn(move || {
                    slice
                        .iter()
                        .filter_map(|p| {
                            scan_pending_mod(p).map(|(item, info)| (p.path_key.clone(), item, info))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();

        let mut fresh = Vec::with_capacity(pending.len());
        for handle in handles {
            if let Ok(scanned) = handle.join() {
                fresh.extend(scanned);
            }
        }
        fresh
    })
}

/// Scans the given `mods_dir` for `.jar` and `.jar.disabled` files, parsing metadata,
/// computing SHA1 & MurmurHash2, and caching results in `<data_dir>/cache/localmods.json`.
pub fn scan_local_mods(mods_dir: &Path) -> Result<Vec<LocalModInfo>, CompError> {
    if !mods_dir.exists() {
        return Ok(Vec::new());
    }

    let mut cache = load_cache();
    let mut cache_modified = false;
    let mut results = Vec::new();
    let mut pending: Vec<PendingMod> = Vec::new();

    let read_dir = fs::read_dir(mods_dir).map_err(CompError::Io)?;
    for entry in read_dir.flatten() {
        // Both checks read the data the enumeration already returned; `path.is_file()` and
        // `fs::metadata(&path)` would each cost a second full stat per entry.
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        // Plain files are the common case and need nothing extra; links reach the metadata
        // step below, which pays a path stat only for them.
        if !file_type.is_file() && !file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let is_jar = file_name.ends_with(".jar");
        let is_disabled = file_name.ends_with(".jar.disabled");
        if !is_jar && !is_disabled {
            continue;
        }

        let enabled = is_jar;
        let metadata = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            // `entry.metadata()` never traverses a link, while the previous `fs::metadata(&path)`
            // did. Follow it once here so a jar symlinked into the folder is still scanned.
            Ok(_) => match fs::metadata(&path) {
                Ok(m) if m.is_file() => m,
                _ => continue,
            },
            Err(_) => continue,
        };
        let size = metadata.len();
        let mtime_secs = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let path_key = path.to_string_lossy().to_string();

        if let Some(cached) = cache.get(&path_key)
            && cached.size == size
            && cached.mtime_secs == mtime_secs
            && cached.icon_rev == ICON_CACHE_REV
        {
            results.push(LocalModInfo {
                file_name,
                path: path_key,
                enabled,
                mod_id: cached.mod_id.clone(),
                name: cached.name.clone(),
                version: cached.version.clone(),
                description: cached.description.clone(),
                authors: cached.authors.clone(),
                loader: cached.loader.clone(),
                sha1: cached.sha1.clone(),
                murmur2: cached.murmur2,
                size,
                project_id: cached.project_id.clone(),
                update_file: cached.update_file.clone(),
                icon: cached.icon.clone(),
            });
            continue;
        }

        pending.push(PendingMod {
            file_name,
            path,
            path_key,
            enabled,
            size,
            mtime_secs,
        });
    }

    for (path_key, item, info) in scan_pending_mods(&pending) {
        cache.insert(path_key, item);
        cache_modified = true;
        results.push(info);
    }

    if cache_modified {
        save_cache(&cache);
    }

    results.sort_by_cached_key(|a| a.name.to_lowercase());
    Ok(results)
}

/// Toggles a local mod by renaming `foo.jar` <-> `foo.jar.disabled`.
pub fn toggle_local_mod(
    mods_dir: &Path,
    file_name: &str,
    enabled: bool,
) -> Result<LocalModInfo, CompError> {
    let current_path = mods_dir.join(file_name);
    let target_file_name = if enabled {
        if file_name.ends_with(".jar.disabled") {
            file_name.trim_end_matches(".disabled").to_string()
        } else {
            file_name.to_string()
        }
    } else if file_name.ends_with(".jar") {
        format!("{file_name}.disabled")
    } else {
        file_name.to_string()
    };

    let target_path = mods_dir.join(&target_file_name);

    if current_path != target_path {
        if !current_path.exists() {
            if target_path.exists() {
                // Already in desired state
            } else {
                return Err(CompError::FileNotFound(file_name.to_string()));
            }
        } else {
            fs::rename(&current_path, &target_path).map_err(CompError::Io)?;
            // The bytes did not change, only the name: migrate the cache entry so the
            // renamed jar keeps its hashes, metadata and icon without being re-hashed.
            if let Ok(metadata) = fs::metadata(&target_path) {
                let mtime_secs = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                migrate_cache_entry(
                    &mod_cache_file(),
                    &current_path,
                    &target_path,
                    metadata.len(),
                    mtime_secs,
                );
            }
        }
    }

    // Every other mod is a cache hit, so this only re-opens the toggled jar when the cache
    // entry could not be migrated (for example a first toggle before any scan).
    let all_mods = scan_local_mods(mods_dir)?;
    all_mods
        .into_iter()
        .find(|m| m.file_name == target_file_name)
        .ok_or_else(|| CompError::FileNotFound(target_file_name))
}

/// Deletes a local mod file, moving it to the system Recycle Bin via `trash`
/// (falling back to permanent removal).
pub fn delete_local_mod(mods_dir: &Path, file_name: &str) -> Result<(), CompError> {
    let target_path = mods_dir.join(file_name);
    if !target_path.exists() {
        return Err(CompError::FileNotFound(file_name.to_string()));
    }

    if let Err(e) = trash::delete(&target_path) {
        tracing::warn!(
            "trash::delete failed for {:?}: {}, falling back to fs::remove_file",
            target_path,
            e
        );
        fs::remove_file(&target_path).map_err(CompError::Io)?;
    }

    Ok(())
}

/// Checks updates for installed local mods against Modrinth and CurseForge.
pub async fn check_local_mods_updates(
    mods_dir: &Path,
    mc_version: &str,
    loader: &str,
    modrinth: &ModrinthClient,
    curseforge: Option<&CurseForgeClient>,
) -> Result<Vec<LocalModInfo>, CompError> {
    let mut mods = scan_local_mods(mods_dir)?;
    if mods.is_empty() {
        return Ok(mods);
    }

    let sha1_list: Vec<String> = mods.iter().map(|m| m.sha1.clone()).collect();

    // 1 & 2. Concurrently query Modrinth SHA1 identification and version files update
    let loaders = if loader.is_empty() {
        Vec::new()
    } else {
        vec![loader.to_string()]
    };
    let game_versions = if mc_version.is_empty() {
        Vec::new()
    } else {
        vec![mc_version.to_string()]
    };

    let (modrinth_files_res, modrinth_updates_res) = tokio::join!(
        modrinth.get_version_files_by_sha1(&sha1_list),
        modrinth.get_version_files_update(&sha1_list, &loaders, &game_versions)
    );
    let modrinth_files = modrinth_files_res.unwrap_or_default();
    let modrinth_updates = modrinth_updates_res.unwrap_or_default();

    for m in &mut mods {
        if let Some(cf_file) = modrinth_files.get(&m.sha1) {
            m.project_id = Some(cf_file.project_id.clone());
        }
        if let Some(update_file) = modrinth_updates.get(&m.sha1)
            && update_file.hashes.sha1.as_deref() != Some(&m.sha1)
        {
            m.project_id = Some(update_file.project_id.clone());
            m.update_file = Some(update_file.clone());
        }
    }

    // 3. Concurrently query CurseForge if configured and available
    if let Some(cf) = curseforge {
        let fingerprints: Vec<u32> = mods
            .iter()
            .filter(|m| m.update_file.is_none() && m.murmur2 > 0)
            .map(|m| m.murmur2)
            .collect();

        if !fingerprints.is_empty()
            && let Ok(cf_files) = cf.match_fingerprints(&fingerprints).await
        {
            let mut check_targets = Vec::new();
            for cf_file in cf_files {
                if let Some(m2) = cf_file.hashes.murmur2
                    && mods
                        .iter()
                        .any(|item| item.murmur2 == m2 && item.update_file.is_none())
                {
                    check_targets.push((cf_file.project_id.clone(), cf_file.id.clone(), m2));
                }
            }

            if !check_targets.is_empty() {
                let cf_client = Arc::new(cf.clone());
                let mc_v = if mc_version.is_empty() {
                    None
                } else {
                    Some(mc_version.to_string())
                };
                let ldr = if loader.is_empty() {
                    None
                } else {
                    Some(loader.to_string())
                };

                let mut join_set = tokio::task::JoinSet::new();
                for (project_id, current_file_id, m2) in check_targets {
                    let client = cf_client.clone();
                    let mc_v_c = mc_v.clone();
                    let ldr_c = ldr.clone();
                    join_set.spawn(async move {
                        let files_res = client
                            .get_files(&project_id, mc_v_c.as_deref(), ldr_c.as_deref())
                            .await;
                        (project_id, current_file_id, m2, files_res)
                    });
                }

                while let Some(res) = join_set.join_next().await {
                    if let Ok((project_id, current_file_id, m2, Ok(files))) = res
                        && let Some(m) = mods
                            .iter_mut()
                            .find(|item| item.murmur2 == m2 && item.update_file.is_none())
                    {
                        m.project_id = Some(project_id);
                        if let Some(best_file) = select_best_file(&files, mc_version, loader)
                            && best_file.id != current_file_id
                            && best_file.hashes.murmur2 != Some(m2)
                        {
                            m.update_file = Some(best_file);
                        }
                    }
                }
            }
        }
    }

    // Update disk cache with identified project_id and update_file
    let mut cache = load_cache();
    let now_secs = current_timestamp_secs();
    for m in &mods {
        if let Some(entry) = cache.get_mut(&m.path) {
            entry.project_id = m.project_id.clone();
            entry.update_file = m.update_file.clone();
            entry.cached_at_secs = now_secs;
        }
    }
    save_cache(&cache);

    Ok(mods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    #[test]
    fn test_compute_curseforge_murmur2_vectors() {
        let input = b"aklerfdhvkore;fhjbgoiwrfgbuio34htgb889rguiyufgvueirefvrvu9vhgg9wygf94u8fgw249fyhuwygf293ghf8h";
        assert_eq!(compute_curseforge_murmur2(input), 2672531333);

        // Stripping whitespace: \t (9), \n (10), \r (13), ' ' (32)
        let input_with_ws = b"aklerfdhvkore;fhjbgoiwrfgbuio34htgb889rguiyufgvueirefvrvu9vhgg9wygf94u8fgw249fyhuwygf293ghf8h \r\n\t";
        assert_eq!(compute_curseforge_murmur2(input_with_ws), 2672531333);

        let clean = b"helloworld";
        let with_ws = b"hello world\r\n\t ";
        assert_eq!(
            compute_curseforge_murmur2(clean),
            compute_curseforge_murmur2(with_ws)
        );
    }

    #[test]
    fn test_unfold_manifest() {
        let manifest = "Manifest-Version: 1.0\r\nImplementation-Title: Test\r\nImplementation-Version: 1.2\r\n .3\r\nEmbedded-Dependencies-Mod: META-INF/jarjar/\r\n child.jar\r\n";
        let unfolded = unfold_manifest(manifest);
        assert!(unfolded.contains("Implementation-Version: 1.2.3"));
        assert!(unfolded.contains("Embedded-Dependencies-Mod: META-INF/jarjar/child.jar"));
        assert_eq!(
            get_manifest_attribute(manifest, "Implementation-Version").as_deref(),
            Some("1.2.3")
        );
    }

    #[test]
    fn test_parse_fabric_mod() {
        let dir = tempdir().unwrap();
        let jar_path = dir.path().join("fabric-test.jar");
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let json = r#"{
            "id": "fabric_test",
            "name": "Fabric Test Mod",
            "version": "1.0.0",
            "description": "A test mod",
            "authors": ["Alice", {"name": "Bob"}]
        }"#;

        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(json.as_bytes()).unwrap();
        zip.finish().unwrap();

        let bytes = fs::read(&jar_path).unwrap();
        let meta = parse_mod_metadata(&bytes, "fabric-test");
        assert_eq!(meta.mod_id, "fabric_test");
        assert_eq!(meta.name, "Fabric Test Mod");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.description.as_deref(), Some("A test mod"));
        assert_eq!(meta.authors, vec!["Alice", "Bob"]);
        assert_eq!(meta.loader, "Fabric");
    }

    #[test]
    fn test_parse_neoforge_mod_with_manifest_substitution() {
        let dir = tempdir().unwrap();
        let jar_path = dir.path().join("neoforge-test.jar");
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let toml = r#"
            modLoader = "javafml"
            loaderVersion = "[1,)"

            [[mods]]
            modId = "neoforge_test"
            displayName = "NeoForge Test Mod"
            version = "${file.jarVersion}"
            description = "NeoForge description"
            authors = "Charlie, Dave"
        "#;

        let manifest = "Manifest-Version: 1.0\r\nImplementation-Version: 2.1.0\r\n";

        zip.start_file("META-INF/neoforge.mods.toml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(toml.as_bytes()).unwrap();

        zip.start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(manifest.as_bytes()).unwrap();

        zip.finish().unwrap();

        let bytes = fs::read(&jar_path).unwrap();
        let meta = parse_mod_metadata(&bytes, "neoforge-test");
        assert_eq!(meta.mod_id, "neoforge_test");
        assert_eq!(meta.name, "NeoForge Test Mod");
        assert_eq!(meta.version, "2.1.0");
        assert_eq!(meta.description.as_deref(), Some("NeoForge description"));
        assert_eq!(meta.authors, vec!["Charlie", "Dave"]);
        assert_eq!(meta.loader, "NeoForge");
    }

    #[test]
    fn test_parse_mcmod_info() {
        let dir = tempdir().unwrap();
        let jar_path = dir.path().join("mcmod-test.jar");
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);

        let json = r#"[
            {
                "modid": "legacy_mod",
                "name": "Legacy Mod",
                "version": "1.7.10-1.0",
                "description": "Legacy Forge mod",
                "authorList": ["Eve"]
            }
        ]"#;

        zip.start_file("mcmod.info", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(json.as_bytes()).unwrap();
        zip.finish().unwrap();

        let bytes = fs::read(&jar_path).unwrap();
        let meta = parse_mod_metadata(&bytes, "mcmod-test");
        assert_eq!(meta.mod_id, "legacy_mod");
        assert_eq!(meta.name, "Legacy Mod");
        assert_eq!(meta.version, "1.7.10-1.0");
        assert_eq!(meta.description.as_deref(), Some("Legacy Forge mod"));
        assert_eq!(meta.authors, vec!["Eve"]);
        assert_eq!(meta.loader, "Forge");
    }

    #[test]
    fn test_mod_icon_found_in_assets_and_logo_file() {
        let dir = tempdir().unwrap();

        // Fabric layout: assets/<modid>/icon.png
        let fabric_jar = dir.path().join("icon-fabric.jar");
        let file = fs::File::create(&fabric_jar).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"id": "icon_mod", "name": "Icon Mod", "version": "1.0"}"#)
            .unwrap();
        zip.start_file("assets/icon_mod/icon.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"fabric-artwork").unwrap();
        zip.finish().unwrap();

        let meta = parse_mod_metadata_path(&fabric_jar, "icon-fabric").unwrap();
        assert_eq!(meta.icon.as_deref(), Some(&b"fabric-artwork"[..]));

        // Forge layout: logoFile points at the artwork path.
        let forge_jar = dir.path().join("icon-forge.jar");
        let file = fs::File::create(&forge_jar).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let toml = "[[mods]]\nmodId = \"logo_mod\"\ndisplayName = \"Logo Mod\"\nversion = \"2.0\"\nlogoFile = \"art/logo.png\"\n";
        zip.start_file("META-INF/mods.toml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(toml.as_bytes()).unwrap();
        zip.start_file("art/logo.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"forge-artwork").unwrap();
        zip.finish().unwrap();

        let meta = parse_mod_metadata_path(&forge_jar, "icon-forge").unwrap();
        assert_eq!(meta.icon.as_deref(), Some(&b"forge-artwork"[..]));
    }

    #[test]
    fn test_square_artwork_beats_wide_banner() {
        // YUNG's mods ship a 1200x240 `logo.png` next to a 256x256 `icon.png`; cropping the
        // banner into the square row tile is what made those covers unreadable.
        fn png_header(width: u32, height: u32, marker: &[u8]) -> Vec<u8> {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
            bytes.extend_from_slice(&13u32.to_be_bytes());
            bytes.extend_from_slice(b"IHDR");
            bytes.extend_from_slice(&width.to_be_bytes());
            bytes.extend_from_slice(&height.to_be_bytes());
            bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
            bytes.extend_from_slice(marker);
            bytes
        }

        let dir = tempdir().unwrap();
        let jar_path = dir.path().join("banner.jar");
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("META-INF/mods.toml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(
            b"[[mods]]\nmodId = \"banner_mod\"\ndisplayName = \"Banner\"\nversion = \"1.0\"\n",
        )
        .unwrap();
        zip.start_file("logo.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&png_header(1200, 240, b"BANNER")).unwrap();
        zip.start_file("icon.png", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&png_header(256, 256, b"SQUARE")).unwrap();
        zip.finish().unwrap();

        let meta = parse_mod_metadata_path(&jar_path, "banner").unwrap();
        let icon = meta.icon.expect("icon selected");
        assert!(
            icon.ends_with(b"SQUARE"),
            "square icon.png must win over the wide logo.png"
        );
    }

    #[test]
    fn test_parsed_metadata_without_artwork_has_no_icon() {
        let dir = tempdir().unwrap();
        let jar_path = dir.path().join("no-art.jar");
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"id": "no_art", "name": "No Art", "version": "1.0"}"#)
            .unwrap();
        zip.finish().unwrap();

        let meta = parse_mod_metadata_path(&jar_path, "no-art").unwrap();
        assert!(meta.icon.is_none());
    }

    #[test]
    fn test_scan_and_toggle_local_mod() {
        let dir = tempdir().unwrap();
        let mods_dir = dir.path().join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        let jar_path = mods_dir.join("test_mod.jar");
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let json = r#"{"id": "test_toggle", "name": "Toggle Mod", "version": "1.0"}"#;
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(json.as_bytes()).unwrap();
        zip.finish().unwrap();

        let scanned = scan_local_mods(&mods_dir).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].file_name, "test_mod.jar");
        assert!(scanned[0].enabled);

        // Toggle disabled
        let toggled = toggle_local_mod(&mods_dir, "test_mod.jar", false).unwrap();
        assert_eq!(toggled.file_name, "test_mod.jar.disabled");
        assert!(!toggled.enabled);
        assert!(!mods_dir.join("test_mod.jar").exists());
        assert!(mods_dir.join("test_mod.jar.disabled").exists());

        // Toggle enabled back
        let toggled_back = toggle_local_mod(&mods_dir, "test_mod.jar.disabled", true).unwrap();
        assert_eq!(toggled_back.file_name, "test_mod.jar");
        assert!(toggled_back.enabled);
        assert!(mods_dir.join("test_mod.jar").exists());
        assert!(!mods_dir.join("test_mod.jar.disabled").exists());
    }

    #[test]
    fn test_hash_file_matches_separate_sha1_and_murmur2() {
        let dir = tempdir().unwrap();

        // Remainder 0..3 covers every tail branch of the Murmur2 fold, and the payload is
        // several 128 KiB chunks long with whitespace sprinkled in.
        for remainder in 0..4usize {
            let mut content: Vec<u8> = Vec::new();
            for i in 0..(300 * 1024u32) {
                content.push(b'a' + (i.wrapping_mul(31).wrapping_add(7) % 26) as u8);
                if i % 53 == 0 {
                    content.push(b' ');
                }
                if i % 97 == 0 {
                    content.push(b'\n');
                }
                if i % 131 == 0 {
                    content.push(b'\r');
                }
                if i % 211 == 0 {
                    content.push(b'\t');
                }
            }
            for k in 0..remainder {
                content.push(b'z' + k as u8);
            }

            let filtered_len = content
                .iter()
                .filter(|&&b| !is_curseforge_whitespace(b))
                .count();
            assert_eq!(filtered_len % 4, remainder);
            assert!(content.len() > 2 * HASH_CHUNK);

            let path = dir.path().join(format!("mod_{remainder}.jar"));
            fs::write(&path, &content).unwrap();

            let (sha1, murmur2) = hash_file(&path).unwrap();

            let mut hasher = Sha1::new();
            hasher.update(&content);
            assert_eq!(sha1, hex::encode(hasher.finalize()));

            let expected_murmur2 = compute_curseforge_murmur2(&content);
            assert_eq!(murmur2, expected_murmur2);

            let mut cursor = Cursor::new(content.clone());
            assert_eq!(
                compute_curseforge_murmur2_reader(&mut cursor).unwrap(),
                expected_murmur2
            );
        }
    }

    #[test]
    fn test_cache_isolation_and_toggle_migration() {
        let dir = tempdir().unwrap();
        let cache_file = dir.path().join("cache").join("localmods.json");

        let mods_dir = dir.path().join("mods");
        fs::create_dir_all(&mods_dir).unwrap();
        let current_path = mods_dir.join("example-mod.jar");
        fs::write(&current_path, b"jar-bytes").unwrap();
        let target_path = mods_dir.join("example-mod.jar.disabled");
        let size = fs::metadata(&current_path).unwrap().len();

        let item = LocalModCacheItem {
            mod_id: "example".into(),
            name: "Example".into(),
            version: "1.0".into(),
            description: None,
            authors: vec!["author".into()],
            loader: "fabric".into(),
            sha1: "abc123".into(),
            murmur2: 42,
            size,
            mtime_secs: 7,
            project_id: None,
            update_file: None,
            icon: Some("icon.png".into()),
            icon_rev: ICON_CACHE_REV,
            cached_at_secs: 0,
        };
        let current_key = current_path.to_string_lossy().to_string();
        let target_key = target_path.to_string_lossy().to_string();

        save_cache_to(
            &cache_file,
            &HashMap::from([(current_key.clone(), item.clone())]),
        );
        assert_eq!(load_cache_from(&cache_file).len(), 1);

        // The toggle migration carries the whole entry across and refreshes the mtime.
        assert!(migrate_cache_entry(
            &cache_file,
            &current_path,
            &target_path,
            size,
            9_999
        ));
        let cache = load_cache_from(&cache_file);
        assert!(!cache.contains_key(&current_key));
        let migrated = cache.get(&target_key).expect("entry must be migrated");
        assert_eq!(migrated.mtime_secs, 9_999);
        assert_eq!(migrated.sha1, "abc123");
        assert_eq!(migrated.icon.as_deref(), Some("icon.png"));

        // A size mismatch means the file changed; the entry must be left untouched.
        assert!(!migrate_cache_entry(
            &cache_file,
            &target_path,
            &current_path,
            size + 1,
            1
        ));
        assert!(load_cache_from(&cache_file).contains_key(&target_key));

        // An external write to the cache file invalidates the in-memory mirror.
        std::thread::sleep(std::time::Duration::from_millis(20));
        let external = HashMap::from([("external-key".to_string(), item)]);
        fs::write(&cache_file, serde_json::to_string(&external).unwrap()).unwrap();
        let reloaded = load_cache_from(&cache_file);
        assert!(reloaded.contains_key("external-key"));
        assert!(!reloaded.contains_key(&target_key));

        // The real toggle keeps the renamed jar's hash and flips its enabled flag.
        let jar_path = mods_dir.join("toggle_target.jar");
        let file = fs::File::create(&jar_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(br#"{"id": "toggle_target", "name": "Toggle Target", "version": "1.0"}"#)
            .unwrap();
        zip.finish().unwrap();

        let sha1_before = scan_local_mods(&mods_dir)
            .unwrap()
            .into_iter()
            .find(|m| m.file_name == "toggle_target.jar")
            .expect("scanned jar")
            .sha1;

        let toggled = toggle_local_mod(&mods_dir, "toggle_target.jar", false).unwrap();
        assert_eq!(toggled.file_name, "toggle_target.jar.disabled");
        assert!(!toggled.enabled);
        assert_eq!(toggled.sha1, sha1_before);
        assert!(mods_dir.join("toggle_target.jar.disabled").exists());
    }
}
