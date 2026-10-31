//! Jar-in-Jar (embedded mod) scanning and inspection.

use std::collections::HashMap;
use std::fs;
use std::io::{Cursor, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use mclr_core::paths;
use serde::{Deserialize, Serialize};
use specta::Type;
use zip::ZipArchive;

use crate::error::CompError;
use crate::local::{parse_mod_metadata, unfold_manifest};

pub const MAX_DEPTH: usize = 5;
pub const MAX_NODES: usize = 512;
pub const MAX_ENTRY_BYTES: u64 = 256 * 1024 * 1024; // 256 MiB

/// A node in the nested Jar-in-Jar tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JarInJarNode {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: String,
    pub size: u64,
    pub children: Vec<JarInJarNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JarInJarCacheEntry {
    pub size: u64,
    pub mtime_secs: u64,
    pub nodes: Vec<JarInJarNode>,
}

#[derive(Debug, Clone)]
struct NestedJarRef {
    path: String,
    identifier: Option<String>,
    artifact_version: Option<String>,
}

/// Entries of one `jarinjar.json` cache file.
type JarCacheMap = HashMap<String, JarInJarCacheEntry>;

/// In-memory store: `cache file -> (file mtime, entries)`.
type JarCacheStore = HashMap<PathBuf, (Option<SystemTime>, JarCacheMap)>;

/// In-memory mirror of `jarinjar.json`: `cache file -> (file mtime, entries)`.
///
/// Storing the mtime lets an external write (another process, a cleared cache) invalidate
/// the in-memory copy automatically, and keying by path keeps separate data directories
/// isolated. The mutex is only held for a lookup or a clone; all `fs` and serde work runs
/// outside it so a slow disk never serialises concurrent scans.
static JAR_IN_JAR_CACHE: LazyLock<Mutex<JarCacheStore>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn jar_cache_file() -> PathBuf {
    paths::cache_dir().join("jarinjar.json")
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn lock_cache() -> std::sync::MutexGuard<'static, JarCacheStore> {
    JAR_IN_JAR_CACHE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn load_cache() -> JarCacheMap {
    let cache_file = jar_cache_file();
    let disk_mtime = file_mtime(&cache_file);

    // Fast path: disk unchanged since the last load/save, so serve the in-memory copy. The
    // lock is held only for the lookup and the clone that follows it.
    if disk_mtime.is_some() {
        let guard = lock_cache();
        if let Some((cached_mtime, map)) = guard.get(&cache_file)
            && *cached_mtime == disk_mtime
        {
            return map.clone();
        }
    }

    // Miss or external modification: read and parse with no lock held.
    let map: JarCacheMap = fs::read_to_string(&cache_file)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();

    lock_cache().insert(cache_file, (disk_mtime, map.clone()));
    map
}

fn save_cache(cache: &JarCacheMap) {
    let cache_file = jar_cache_file();
    if let Some(parent) = cache_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(cache) {
        let _ = fs::write(&cache_file, json);
    }
    let new_mtime = file_mtime(&cache_file);
    lock_cache().insert(cache_file, (new_mtime, cache.clone()));
}

fn read_entry_text<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

/// Recursively discovers and resolves nested jar files inside a zip archive.
fn resolve_nested_archive<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    parent_path: &str,
    depth: usize,
    budget: &mut usize,
) -> Result<Vec<JarInJarNode>, CompError> {
    if depth >= MAX_DEPTH || *budget == 0 {
        return Ok(Vec::new());
    }

    let mut nested_refs: Vec<NestedJarRef> = Vec::new();

    // 1. Detect nested jars in fabric.mod.json -> jars[].file
    if let Some(text) = read_entry_text(archive, "fabric.mod.json")
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(arr) = val.get("jars").and_then(|v| v.as_array())
    {
        for item in arr {
            if let Some(file_str) = item.get("file").and_then(|v| v.as_str()) {
                nested_refs.push(NestedJarRef {
                    path: file_str.to_string(),
                    identifier: None,
                    artifact_version: None,
                });
            }
        }
    }

    // 2. Detect nested jars in quilt.mod.json -> quilt_loader.jars[]
    if let Some(text) = read_entry_text(archive, "quilt.mod.json")
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(arr) = val
            .get("quilt_loader")
            .and_then(|q| q.get("jars"))
            .and_then(|v| v.as_array())
    {
        for item in arr {
            if let Some(s) = item.as_str() {
                nested_refs.push(NestedJarRef {
                    path: s.to_string(),
                    identifier: None,
                    artifact_version: None,
                });
            } else if let Some(s) = item.get("file").and_then(|v| v.as_str()) {
                nested_refs.push(NestedJarRef {
                    path: s.to_string(),
                    identifier: None,
                    artifact_version: None,
                });
            }
        }
    }

    // 3. Detect nested jars in META-INF/jarjar/metadata.json -> jars[].path
    if let Some(text) = read_entry_text(archive, "META-INF/jarjar/metadata.json")
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&text)
        && let Some(arr) = val.get("jars").and_then(|v| v.as_array())
    {
        for item in arr {
            if let Some(p) = item.get("path").and_then(|v| v.as_str()) {
                let group = item
                    .get("identifier")
                    .and_then(|i| i.get("group"))
                    .and_then(|v| v.as_str());
                let artifact = item
                    .get("identifier")
                    .and_then(|i| i.get("artifact"))
                    .and_then(|v| v.as_str());
                let identifier = match (group, artifact) {
                    (Some(g), Some(a)) => Some(format!("{g}:{a}")),
                    _ => None,
                };
                let artifact_version = item
                    .get("version")
                    .and_then(|ver| ver.get("artifactVersion"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                nested_refs.push(NestedJarRef {
                    path: p.to_string(),
                    identifier,
                    artifact_version,
                });
            }
        }
    }

    // 4. Detect nested jars in META-INF/MANIFEST.MF -> Embedded-Dependencies-Mod
    if let Some(text) = read_entry_text(archive, "META-INF/MANIFEST.MF") {
        let unfolded = unfold_manifest(&text);
        for line in unfolded.lines() {
            let trimmed = line.trim();
            let prefix = "embedded-dependencies-mod:";
            if trimmed.to_ascii_lowercase().starts_with(prefix) {
                let val = trimmed[prefix.len()..].trim();
                for p in val.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    nested_refs.push(NestedJarRef {
                        path: p.to_string(),
                        identifier: None,
                        artifact_version: None,
                    });
                }
            }
        }
    }

    // Deduplicate by path, keeping earliest metadata
    let mut deduped: Vec<NestedJarRef> = Vec::new();
    for r in nested_refs {
        if !deduped.iter().any(|d| d.path == r.path) {
            deduped.push(r);
        }
    }

    let mut nodes = Vec::new();

    for r in deduped {
        if *budget == 0 {
            break;
        }

        let mut entry = match archive.by_name(&r.path) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let entry_size = entry.size();
        if entry_size > MAX_ENTRY_BYTES {
            tracing::warn!(
                "Skipping nested jar {:?} due to excessive size ({} bytes > 256 MiB)",
                r.path,
                entry_size
            );
            continue;
        }

        let mut child_bytes = Vec::with_capacity(entry_size as usize);
        if entry.read_to_end(&mut child_bytes).is_err() {
            continue;
        }

        *budget = budget.saturating_sub(1);
        let child_path = format!("{parent_path}!/{}", r.path);
        let fallback_stem = Path::new(&r.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(&r.path);

        let mut meta = parse_mod_metadata(&child_bytes, fallback_stem);

        if meta.loader == "Unknown" {
            if let Some(id) = r.identifier {
                meta.mod_id = id.clone();
                meta.name = id;
            }
            if let Some(ver) = r.artifact_version {
                meta.version = ver;
            }
        }

        let child_nodes = if depth + 1 < MAX_DEPTH && *budget > 0 {
            let cursor = Cursor::new(&child_bytes);
            if let Ok(mut child_archive) = ZipArchive::new(cursor) {
                resolve_nested_archive(&mut child_archive, &child_path, depth + 1, budget)?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        nodes.push(JarInJarNode {
            id: meta.mod_id,
            name: meta.name,
            version: meta.version,
            path: child_path,
            size: entry_size,
            children: child_nodes,
        });
    }

    Ok(nodes)
}

/// Scans a mod jar file on disk for embedded nested jars (Jar-in-Jar),
/// returning the tree of `JarInJarNode`s.
pub fn scan_jar_in_jar(jar_path: &Path) -> Result<Vec<JarInJarNode>, CompError> {
    if !jar_path.exists() {
        return Err(CompError::FileNotFound(
            jar_path.to_string_lossy().to_string(),
        ));
    }

    let metadata = fs::metadata(jar_path).map_err(CompError::Io)?;
    let size = metadata.len();
    let mtime_secs = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cache_key = format!("{}:{}:{}", jar_path.display(), mtime_secs, size);
    let mut cache = load_cache();
    if let Some(cached) = cache.get(&cache_key)
        && cached.size == size
        && cached.mtime_secs == mtime_secs
    {
        return Ok(cached.nodes.clone());
    }

    let file = fs::File::open(jar_path).map_err(CompError::Io)?;
    let mut archive = ZipArchive::new(file).map_err(CompError::Zip)?;

    let mut budget = MAX_NODES;
    let file_name = jar_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mod.jar");

    let nodes = resolve_nested_archive(&mut archive, file_name, 0, &mut budget)?;

    cache.insert(
        cache_key,
        JarInJarCacheEntry {
            size,
            mtime_secs,
            nodes: nodes.clone(),
        },
    );
    save_cache(&cache);

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    fn create_mock_child_jar(id: &str, name: &str, version: &str) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let json = format!(
                r#"{{"id": "{}", "name": "{}", "version": "{}"}}"#,
                id, name, version
            );
            zip.start_file("fabric.mod.json", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(json.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn test_jar_in_jar_detection_and_tree() {
        let dir = tempdir().unwrap();
        let parent_jar_path = dir.path().join("parent-mod.jar");

        // 1. Create a grandchild jar
        let grandchild_bytes = create_mock_child_jar("grandchild", "Grand Child", "0.1.0");

        // 2. Create child jar that embeds the grandchild jar
        let mut child_buf = Cursor::new(Vec::new());
        {
            let mut child_zip = zip::ZipWriter::new(&mut child_buf);
            let child_meta = r#"{
                "id": "child_mod",
                "name": "Child Mod",
                "version": "1.0.0",
                "jars": [{"file": "META-INF/jars/grandchild.jar"}]
            }"#;
            child_zip
                .start_file("fabric.mod.json", SimpleFileOptions::default())
                .unwrap();
            child_zip.write_all(child_meta.as_bytes()).unwrap();

            child_zip
                .start_file("META-INF/jars/grandchild.jar", SimpleFileOptions::default())
                .unwrap();
            child_zip.write_all(&grandchild_bytes).unwrap();
            child_zip.finish().unwrap();
        }
        let child_bytes = child_buf.into_inner();

        // 3. Create another child jar referenced via META-INF/jarjar/metadata.json
        let jarjar_child_bytes = create_mock_child_jar("forge_dep", "Forge Dep", "2.0.0");

        // 4. Create another child jar referenced via MANIFEST.MF
        let manifest_child_bytes = create_mock_child_jar("manifest_dep", "Manifest Dep", "3.0.0");

        // 5. Create parent jar
        let file = fs::File::create(&parent_jar_path).unwrap();
        let mut parent_zip = zip::ZipWriter::new(file);

        let parent_fabric = r#"{
            "id": "parent_mod",
            "name": "Parent Mod",
            "version": "1.0.0",
            "jars": [{"file": "META-INF/jars/child.jar"}]
        }"#;
        parent_zip
            .start_file("fabric.mod.json", SimpleFileOptions::default())
            .unwrap();
        parent_zip.write_all(parent_fabric.as_bytes()).unwrap();

        parent_zip
            .start_file("META-INF/jars/child.jar", SimpleFileOptions::default())
            .unwrap();
        parent_zip.write_all(&child_bytes).unwrap();

        let jarjar_metadata = r#"{
            "jars": [
                {
                    "identifier": {"group": "org.dep", "artifact": "forge_dep"},
                    "version": {"range": "[2.0.0,)", "artifactVersion": "2.0.0"},
                    "path": "META-INF/jarjar/forge_dep.jar"
                }
            ]
        }"#;
        parent_zip
            .start_file(
                "META-INF/jarjar/metadata.json",
                SimpleFileOptions::default(),
            )
            .unwrap();
        parent_zip.write_all(jarjar_metadata.as_bytes()).unwrap();

        parent_zip
            .start_file(
                "META-INF/jarjar/forge_dep.jar",
                SimpleFileOptions::default(),
            )
            .unwrap();
        parent_zip.write_all(&jarjar_child_bytes).unwrap();

        let manifest = "Manifest-Version: 1.0\r\nEmbedded-Dependencies-Mod: META-INF/embedded/\r\n manifest_dep.jar\r\n";
        parent_zip
            .start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
            .unwrap();
        parent_zip.write_all(manifest.as_bytes()).unwrap();

        parent_zip
            .start_file(
                "META-INF/embedded/manifest_dep.jar",
                SimpleFileOptions::default(),
            )
            .unwrap();
        parent_zip.write_all(&manifest_child_bytes).unwrap();

        parent_zip.finish().unwrap();

        // Scan parent jar
        let tree = scan_jar_in_jar(&parent_jar_path).unwrap();
        assert_eq!(tree.len(), 3);

        // Find child_mod
        let child_node = tree.iter().find(|n| n.id == "child_mod").unwrap();
        assert_eq!(child_node.name, "Child Mod");
        assert_eq!(child_node.version, "1.0.0");
        assert!(
            child_node
                .path
                .contains("parent-mod.jar!/META-INF/jars/child.jar")
        );
        assert_eq!(child_node.children.len(), 1);
        assert_eq!(child_node.children[0].id, "grandchild");
        assert_eq!(child_node.children[0].version, "0.1.0");

        // Find forge_dep
        let forge_node = tree.iter().find(|n| n.id == "forge_dep").unwrap();
        assert_eq!(forge_node.name, "Forge Dep");
        assert_eq!(forge_node.version, "2.0.0");

        // Find manifest_dep
        let manifest_node = tree.iter().find(|n| n.id == "manifest_dep").unwrap();
        assert_eq!(manifest_node.name, "Manifest Dep");
        assert_eq!(manifest_node.version, "3.0.0");
    }

    #[test]
    fn test_jar_in_jar_depth_limit() {
        // Construct a deep nesting chain of jars to test depth cap
        let mut current_bytes = create_mock_child_jar("deep_leaf", "Deep Leaf", "0.0.1");

        for i in (0..7).rev() {
            let mut buf = Cursor::new(Vec::new());
            {
                let mut zip = zip::ZipWriter::new(&mut buf);
                let meta = format!(
                    r#"{{"id": "node_{}", "name": "Node {}", "version": "1.0", "jars": [{{"file": "child_{}.jar"}}]}}"#,
                    i, i, i
                );
                zip.start_file("fabric.mod.json", SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(meta.as_bytes()).unwrap();

                zip.start_file(format!("child_{}.jar", i), SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(&current_bytes).unwrap();
                zip.finish().unwrap();
            }
            current_bytes = buf.into_inner();
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("root_deep.jar");
        fs::write(&path, &current_bytes).unwrap();

        let tree = scan_jar_in_jar(&path).unwrap();
        assert_eq!(tree.len(), 1);

        // Check depth <= 5
        let mut depth = 0;
        let mut curr = &tree[0];
        while !curr.children.is_empty() {
            depth += 1;
            curr = &curr.children[0];
        }
        assert!(depth < MAX_DEPTH);
    }
}
