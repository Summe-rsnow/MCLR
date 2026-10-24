//! Local mod management: scanning, parsing, hash identification, toggling, and updates.

use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

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
}

/// Computes the exact CurseForge MurmurHash2 (32-bit, seed = 1, m = 0x5bd1e995, r = 24)
/// over `data` with whitespace bytes (b'\t', b'\n', b'\r', b' ' -> 9, 10, 13, 32) stripped.
pub fn compute_curseforge_murmur2(data: &[u8]) -> u32 {
    let filtered: Vec<u8> = data
        .iter()
        .copied()
        .filter(|&b| b != 9 && b != 10 && b != 13 && b != 32)
        .collect();

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
}

fn read_entry_text<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

/// Parses mod metadata from zip archive bytes of a jar file.
pub fn parse_mod_metadata(bytes: &[u8], fallback_name: &str) -> ModMetadata {
    let cursor = Cursor::new(bytes);
    let mut archive = match ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => {
            return ModMetadata {
                mod_id: fallback_name.to_string(),
                name: fallback_name.to_string(),
                version: String::new(),
                description: None,
                authors: Vec::new(),
                loader: "Unknown".to_string(),
            };
        }
    };

    // 1. Check fabric.mod.json
    if let Some(text) = read_entry_text(&mut archive, "fabric.mod.json")
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

        return ModMetadata {
            mod_id: id,
            name,
            version,
            description,
            authors,
            loader: "Fabric".to_string(),
        };
    }

    // 2. Check quilt.mod.json
    if let Some(text) = read_entry_text(&mut archive, "quilt.mod.json")
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

        return ModMetadata {
            mod_id: id,
            name,
            version,
            description,
            authors,
            loader: "Quilt".to_string(),
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
        read_entry_text(&mut archive, toml_entry_name)
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
            && let Some(m_text) = read_entry_text(&mut archive, "META-INF/MANIFEST.MF")
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

        return ModMetadata {
            mod_id,
            name,
            version,
            description,
            authors,
            loader: loader.to_string(),
        };
    }

    // 4. Check mcmod.info (legacy Forge)
    if let Some(text) = read_entry_text(&mut archive, "mcmod.info")
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

            return ModMetadata {
                mod_id,
                name,
                version,
                description,
                authors,
                loader: "Forge".to_string(),
            };
        }
    }

    // 5. Fallback
    ModMetadata {
        mod_id: fallback_name.to_string(),
        name: fallback_name.to_string(),
        version: String::new(),
        description: None,
        authors: Vec::new(),
        loader: "Unknown".to_string(),
    }
}

fn load_cache() -> HashMap<String, LocalModCacheItem> {
    let cache_file = paths::cache_dir().join("localmods.json");
    fs::read_to_string(&cache_file)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_cache(cache: &HashMap<String, LocalModCacheItem>) {
    let cache_dir = paths::cache_dir();
    let _ = fs::create_dir_all(&cache_dir);
    let cache_file = cache_dir.join("localmods.json");
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(&cache_file, json);
    }
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

    let read_dir = fs::read_dir(mods_dir).map_err(CompError::Io)?;
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

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
        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
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
            });
            continue;
        }

        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let sha1 = hex::encode(hasher.finalize());
        let murmur2 = compute_curseforge_murmur2(&bytes);

        let fallback_name = if let Some(stem) = file_name.strip_suffix(".jar.disabled") {
            stem
        } else if let Some(stem) = file_name.strip_suffix(".jar") {
            stem
        } else {
            &file_name
        };

        let mod_meta = parse_mod_metadata(&bytes, fallback_name);

        let cache_item = LocalModCacheItem {
            mod_id: mod_meta.mod_id.clone(),
            name: mod_meta.name.clone(),
            version: mod_meta.version.clone(),
            description: mod_meta.description.clone(),
            authors: mod_meta.authors.clone(),
            loader: mod_meta.loader.clone(),
            sha1: sha1.clone(),
            murmur2,
            size,
            mtime_secs,
            project_id: None,
            update_file: None,
        };

        cache.insert(path_key.clone(), cache_item);
        cache_modified = true;

        results.push(LocalModInfo {
            file_name,
            path: path_key,
            enabled,
            mod_id: mod_meta.mod_id,
            name: mod_meta.name,
            version: mod_meta.version,
            description: mod_meta.description,
            authors: mod_meta.authors,
            loader: mod_meta.loader,
            sha1,
            murmur2,
            size,
            project_id: None,
            update_file: None,
        });
    }

    if cache_modified {
        save_cache(&cache);
    }

    results.sort_by_key(|a| a.name.to_lowercase());
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
        }
    }

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

    // 1. Query Modrinth version files by SHA1 to identify projects
    let modrinth_files = modrinth
        .get_version_files_by_sha1(&sha1_list)
        .await
        .unwrap_or_default();

    for m in &mut mods {
        if let Some(cf_file) = modrinth_files.get(&m.sha1) {
            m.project_id = Some(cf_file.project_id.clone());
        }
    }

    // 2. Query Modrinth version files update
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

    let modrinth_updates = modrinth
        .get_version_files_update(&sha1_list, &loaders, &game_versions)
        .await
        .unwrap_or_default();

    for m in &mut mods {
        if let Some(update_file) = modrinth_updates.get(&m.sha1)
            && update_file.hashes.sha1.as_deref() != Some(&m.sha1)
        {
            m.project_id = Some(update_file.project_id.clone());
            m.update_file = Some(update_file.clone());
        }
    }

    // 3. Query CurseForge if configured and available
    if let Some(cf) = curseforge {
        let fingerprints: Vec<u32> = mods
            .iter()
            .filter(|m| m.update_file.is_none() && m.murmur2 > 0)
            .map(|m| m.murmur2)
            .collect();

        if !fingerprints.is_empty()
            && let Ok(cf_files) = cf.match_fingerprints(&fingerprints).await
        {
            for cf_file in cf_files {
                if let Some(m2) = cf_file.hashes.murmur2
                    && let Some(m) = mods
                        .iter_mut()
                        .find(|item| item.murmur2 == m2 && item.update_file.is_none())
                {
                    m.project_id = Some(cf_file.project_id.clone());

                    if let Ok(files) = cf
                        .get_files(&cf_file.project_id, Some(mc_version), Some(loader))
                        .await
                        && let Some(best_file) = select_best_file(&files, mc_version, loader)
                        && best_file.id != cf_file.id
                        && best_file.hashes.murmur2 != Some(m2)
                    {
                        m.update_file = Some(best_file);
                    }
                }
            }
        }
    }

    // Update disk cache with identified project_id and update_file
    let mut cache = load_cache();
    for m in &mods {
        if let Some(entry) = cache.get_mut(&m.path) {
            entry.project_id = m.project_id.clone();
            entry.update_file = m.update_file.clone();
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
}
