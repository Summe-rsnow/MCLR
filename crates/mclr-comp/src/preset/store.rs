//! Content-addressed store for community resources.
//!
//! Every downloaded archive lives exactly once under `objects/<sha1[0..2]>/<sha1>`.
//! Instance files are either hard links to that object (same volume, NTFS) or plain
//! copies when linking is impossible, so a renamed or deleted instance file can never
//! corrupt the store and a deleted store object can never corrupt an instance file.
//!
//! Layout under the store root:
//!
//! ```text
//! objects/<xx>/<sha1>   raw file bytes, no extension
//! tmp/                  in-flight downloads
//! index.json            object metadata + instance references (atomic write)
//! ```

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use url::Url;
use uuid::Uuid;

use crate::CompError;
use crate::model::{CompFile, CompPlatform, CompType};
use crate::preset::model::{LinkMode, PruneResult, StoreEntry, StoreInfo};

const INDEX_VERSION: u32 = 1;
const INDEX_FILE: &str = "index.json";
const HASH_CHUNK: usize = 64 * 1024;

/// Why a single object may be considered pinned and therefore never pruned.
fn is_pinned(pinned_file_ids: &HashSet<String>, sha1: &str, file_id: &str) -> bool {
    pinned_file_ids.contains(file_id) || pinned_file_ids.contains(sha1)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// A store object on disk, ready to be linked into an instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredObject {
    pub sha1: String,
    pub size: u64,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexObject {
    platform: CompPlatform,
    project_id: String,
    file_id: String,
    file_name: String,
    kind: CompType,
    size: u64,
    added_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RefMode {
    HardLink,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexRef {
    sha1: String,
    game_dir: String,
    rel_path: String,
    mode: RefMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoreIndex {
    version: u32,
    #[serde(default)]
    objects: BTreeMap<String, IndexObject>,
    #[serde(default)]
    refs: Vec<IndexRef>,
}

impl Default for StoreIndex {
    fn default() -> Self {
        Self {
            version: INDEX_VERSION,
            objects: BTreeMap::new(),
            refs: Vec::new(),
        }
    }
}

/// Content-addressed resource store rooted at a directory.
///
/// Clones share the same index lock, so a [`Store`] may be cheaply cloned into
/// concurrent install workers.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl Store {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Fetch `file` into the store, or return the existing object on a cache hit.
    pub async fn ensure(
        &self,
        dl: &Downloader,
        file: &CompFile,
        kind: CompType,
    ) -> Result<StoredObject, CompError> {
        let known = file
            .hashes
            .sha1
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());

        if let Some(sha1) = &known {
            let path = self.object_path(sha1);
            if let Ok(meta) = fs::metadata(&path)
                && (file.size == 0 || meta.len() == file.size)
            {
                let size = meta.len();
                self.record_object(sha1, file, kind, size)?;
                return Ok(StoredObject {
                    sha1: sha1.clone(),
                    size,
                    path,
                });
            }
        }

        if file.distribution_disabled || file.download_urls.is_empty() {
            return Err(CompError::DistributionDisabled {
                project_id: file.project_id.clone(),
                file_id: file.id.clone(),
                website_url: None,
            });
        }

        let urls: Vec<Url> = file
            .download_urls
            .iter()
            .filter_map(|raw| Url::parse(raw).ok())
            .collect();
        if urls.is_empty() {
            return Err(CompError::InvalidParams(
                "no valid download URLs in CompFile".to_string(),
            ));
        }

        let tmp_dir = self.tmp_dir();
        fs::create_dir_all(&tmp_dir)?;
        let tmp_path = tmp_dir.join(format!("{}.part", Uuid::new_v4()));
        let spec = DownloadSpec {
            urls,
            dest: tmp_path.clone(),
            size: if file.size > 0 { Some(file.size) } else { None },
            sha1: known.clone(),
            sha256: None,
            md5: None,
        };

        if let Err(err) = dl.download(spec, None).await {
            let _ = fs::remove_file(&tmp_path);
            return Err(err.into());
        }

        let sha1 = match known {
            Some(sha1) => sha1,
            None => {
                let path = tmp_path.clone();
                match tokio::task::spawn_blocking(move || hash_sha1(&path)).await {
                    Ok(result) => result?,
                    Err(_) => {
                        let _ = fs::remove_file(&tmp_path);
                        return Err(CompError::InvalidParams("hash worker failed".to_string()));
                    }
                }
            }
        };

        let object_path = self.object_path(&sha1);
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let size = fs::metadata(&tmp_path)?.len();
        if object_path.exists() {
            // A concurrent `ensure` for the same content already stored it.
            let _ = fs::remove_file(&tmp_path);
        } else {
            match fs::rename(&tmp_path, &object_path) {
                Ok(()) => {}
                Err(err) => {
                    let _ = fs::remove_file(&tmp_path);
                    // Lost the rename race against another worker: the object exists now.
                    if !object_path.exists() {
                        return Err(err.into());
                    }
                }
            }
        }

        self.record_object(&sha1, file, kind, size)?;
        Ok(StoredObject {
            sha1,
            size,
            path: object_path,
        })
    }

    /// Link (or copy) a stored object into an instance directory.
    ///
    /// Never overwrites: an existing destination returns `preset.destExists`.
    pub fn place(
        &self,
        obj: &StoredObject,
        dest: &Path,
        use_hardlink: bool,
        game_dir: &Path,
        rel_path: &str,
    ) -> Result<LinkMode, CompError> {
        if dest.exists() {
            return Err(CompError::Preset("preset.destExists"));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let mode = if use_hardlink {
            match fs::hard_link(&obj.path, dest) {
                Ok(()) => LinkMode::HardLink,
                Err(_) => {
                    // Different volume, FAT, permissions, or the 1023-link limit.
                    fs::copy(&obj.path, dest)?;
                    LinkMode::Copy
                }
            }
        } else {
            fs::copy(&obj.path, dest)?;
            LinkMode::Copy
        };

        let ref_mode = match mode {
            LinkMode::HardLink => RefMode::HardLink,
            _ => RefMode::Copy,
        };
        let game_dir_str = normalize_path(Path::new(game_dir))
            .to_string_lossy()
            .into_owned();
        let rel = normalize_rel_path(rel_path);

        self.with_index(|index| {
            index.refs.retain(|existing| {
                !(existing.game_dir == game_dir_str && existing.rel_path == rel)
            });
            index.refs.push(IndexRef {
                sha1: obj.sha1.clone(),
                game_dir: game_dir_str.clone(),
                rel_path: rel.clone(),
                mode: ref_mode,
            });
            Ok(((), true))
        })?;

        Ok(mode)
    }

    /// Store statistics; dead instance references are dropped first.
    pub fn info(&self, pinned_file_ids: &HashSet<String>) -> Result<StoreInfo, CompError> {
        self.with_index(|index| {
            let changed = prune_dead_refs(index);

            let mut live_refs: BTreeMap<&str, u32> = BTreeMap::new();
            let mut hardlink_refs: BTreeMap<&str, u32> = BTreeMap::new();
            for reference in &index.refs {
                *live_refs.entry(reference.sha1.as_str()).or_default() += 1;
                if reference.mode == RefMode::HardLink {
                    *hardlink_refs.entry(reference.sha1.as_str()).or_default() += 1;
                }
            }

            let mut entries = Vec::with_capacity(index.objects.len());
            let mut total_bytes = 0u64;
            let mut saved_bytes = 0u64;
            for (sha1, object) in &index.objects {
                if !self.object_path(sha1).is_file() {
                    continue;
                }
                let refs = live_refs.get(sha1.as_str()).copied().unwrap_or(0);
                let linked = hardlink_refs.get(sha1.as_str()).copied().unwrap_or(0);
                total_bytes = total_bytes.saturating_add(object.size);
                saved_bytes =
                    saved_bytes.saturating_add(object.size.saturating_mul(u64::from(linked)));
                entries.push(StoreEntry {
                    sha1: sha1.clone(),
                    platform: object.platform,
                    project_id: object.project_id.clone(),
                    file_id: object.file_id.clone(),
                    file_name: object.file_name.clone(),
                    kind: object.kind,
                    size: object.size,
                    added_at: object.added_at,
                    live_refs: refs,
                    pinned: is_pinned(pinned_file_ids, sha1, &object.file_id),
                });
            }
            entries.sort_by(|a, b| b.added_at.cmp(&a.added_at).then(a.sha1.cmp(&b.sha1)));

            let info = StoreInfo {
                dir: self.root.to_string_lossy().into_owned(),
                default_dir: mclr_core::paths::cache_dir()
                    .join("store")
                    .to_string_lossy()
                    .into_owned(),
                object_count: entries.len() as u32,
                total_bytes,
                saved_bytes,
                entries,
            };
            Ok((info, changed))
        })
    }

    /// Drop unreferenced, unpinned objects.
    pub fn prune(&self, pinned_file_ids: &HashSet<String>) -> Result<PruneResult, CompError> {
        self.with_index(|index| {
            let mut changed = prune_dead_refs(index);

            let mut live_refs: BTreeMap<String, u32> = BTreeMap::new();
            for reference in &index.refs {
                *live_refs.entry(reference.sha1.clone()).or_default() += 1;
            }

            let mut removed_count = 0u32;
            let mut freed_bytes = 0u64;
            let mut doomed: Vec<String> = Vec::new();
            for (sha1, object) in &index.objects {
                let refs = live_refs.get(sha1).copied().unwrap_or(0);
                let path = self.object_path(sha1);
                let exists = path.is_file();
                if exists && refs == 0 && !is_pinned(pinned_file_ids, sha1, &object.file_id) {
                    fs::remove_file(&path)?;
                    removed_count = removed_count.saturating_add(1);
                    freed_bytes = freed_bytes.saturating_add(object.size);
                    doomed.push(sha1.clone());
                } else if !exists {
                    // The object vanished behind our back; forget it.
                    doomed.push(sha1.clone());
                }
            }
            for sha1 in doomed {
                index.objects.remove(&sha1);
                index.refs.retain(|reference| reference.sha1 != sha1);
                changed = true;
            }
            remove_empty_shards(&self.objects_dir());

            Ok((
                PruneResult {
                    removed_count,
                    freed_bytes,
                },
                changed,
            ))
        })
    }

    /// Delete one object and every reference to it. Instance copies/links stay valid.
    pub fn remove(&self, sha1: &str) -> Result<(), CompError> {
        let sha1 = sha1.trim().to_ascii_lowercase();
        self.with_index(|index| {
            let path = self.object_path(&sha1);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err.into()),
            }
            index.objects.remove(&sha1);
            index.refs.retain(|reference| reference.sha1 != sha1);
            Ok(((), true))
        })
    }

    /// Move every object and the index into `new_root`.
    ///
    /// The current `Store` keeps pointing at the old root; callers create a new
    /// `Store::new(new_root)` afterwards. Instance links stay valid either way.
    pub fn migrate_to(&self, new_root: &Path) -> Result<(), CompError> {
        let root = normalize_path(&self.root);
        let new_root = normalize_path(new_root);
        if new_root == root || new_root.starts_with(&root) {
            return Err(CompError::Preset("preset.storeDirInvalid"));
        }

        let _guard = self.lock();
        let index = self.read_index()?;

        let new_objects = new_root.join("objects");
        fs::create_dir_all(&new_objects)?;
        fs::create_dir_all(new_root.join("tmp"))?;

        let old_objects = self.objects_dir();
        if old_objects.is_dir() {
            for shard in fs::read_dir(&old_objects)? {
                let shard = shard?;
                if !shard.file_type()?.is_dir() {
                    continue;
                }
                let dest_shard = new_objects.join(shard.file_name());
                fs::create_dir_all(&dest_shard)?;
                for entry in fs::read_dir(shard.path())? {
                    let entry = entry?;
                    if !entry.file_type()?.is_file() {
                        continue;
                    }
                    move_file(&entry.path(), &dest_shard.join(entry.file_name()))?;
                }
                let _ = fs::remove_dir(shard.path());
            }
            let _ = fs::remove_dir(&old_objects);
        }

        write_index_at(&new_root, &index)?;
        match fs::remove_file(self.index_path()) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        Ok(())
    }

    fn object_path(&self, sha1: &str) -> PathBuf {
        self.objects_dir()
            .join(&sha1[..2.min(sha1.len())])
            .join(sha1)
    }

    fn objects_dir(&self) -> PathBuf {
        self.root.join("objects")
    }

    fn tmp_dir(&self) -> PathBuf {
        self.root.join("tmp")
    }

    fn index_path(&self) -> PathBuf {
        self.root.join(INDEX_FILE)
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// Read the index without taking the lock; a corrupt file is moved aside
    /// rather than aborting the operation.
    fn read_index(&self) -> Result<StoreIndex, CompError> {
        let path = self.index_path();
        match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<StoreIndex>(&bytes) {
                Ok(index) => Ok(index),
                Err(err) => {
                    let bad = path.with_extension("json.bad");
                    let _ = fs::remove_file(&bad);
                    let _ = fs::rename(&path, &bad);
                    tracing::warn!("preset store index corrupted, reset: {err}");
                    Ok(StoreIndex::default())
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(StoreIndex::default()),
            Err(err) => Err(err.into()),
        }
    }

    fn write_index(&self, index: &StoreIndex) -> Result<(), CompError> {
        fs::create_dir_all(&self.root)?;
        write_index_at(&self.root, index)
    }

    /// Run `f` with exclusive access to the index and persist it when `f` says so.
    fn with_index<T>(
        &self,
        f: impl FnOnce(&mut StoreIndex) -> Result<(T, bool), CompError>,
    ) -> Result<T, CompError> {
        let _guard = self.lock();
        let mut index = self.read_index()?;
        let (value, changed) = f(&mut index)?;
        if changed {
            self.write_index(&index)?;
        }
        Ok(value)
    }

    fn record_object(
        &self,
        sha1: &str,
        file: &CompFile,
        kind: CompType,
        size: u64,
    ) -> Result<(), CompError> {
        self.with_index(|index| {
            if let Some(existing) = index.objects.get_mut(sha1) {
                if existing.size != size {
                    existing.size = size;
                    return Ok(((), true));
                }
                return Ok(((), false));
            }
            index.objects.insert(
                sha1.to_string(),
                IndexObject {
                    platform: file.platform,
                    project_id: file.project_id.clone(),
                    file_id: file.id.clone(),
                    file_name: file.file_name.clone(),
                    kind,
                    size,
                    added_at: now_ms(),
                },
            );
            Ok(((), true))
        })
    }
}

fn write_index_at(root: &Path, index: &StoreIndex) -> Result<(), CompError> {
    let tmp = root.join(format!(".index-{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(index)?;
    fs::write(&tmp, bytes)?;
    match fs::rename(&tmp, root.join(INDEX_FILE)) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&tmp);
            Err(err.into())
        }
    }
}

fn prune_dead_refs(index: &mut StoreIndex) -> bool {
    let before = index.refs.len();
    index.refs.retain(|reference| {
        Path::new(&reference.game_dir)
            .join(
                reference
                    .rel_path
                    .replace('/', std::path::MAIN_SEPARATOR_STR),
            )
            .exists()
    });
    index.refs.len() != before
}

fn remove_empty_shards(objects: &Path) {
    let Ok(shards) = fs::read_dir(objects) else {
        return;
    };
    for shard in shards.flatten() {
        if shard.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let _ = fs::remove_dir(shard.path());
        }
    }
}

fn move_file(src: &Path, dest: &Path) -> Result<(), CompError> {
    if dest.exists() {
        let _ = fs::remove_file(src);
        return Ok(());
    }
    if fs::rename(src, dest).is_ok() {
        return Ok(());
    }
    fs::copy(src, dest)?;
    let _ = fs::remove_file(src);
    Ok(())
}

fn normalize_rel_path(rel_path: &str) -> String {
    rel_path
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string()
}

/// Lexical path normalisation (the path may not exist yet, so `canonicalize` is unusable).
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn hash_sha1(path: &Path) -> Result<String, CompError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = vec![0u8; HASH_CHUNK];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_lexical_normalisation_collapses_dots() {
        let root = PathBuf::from("C:/games/store");
        assert_eq!(
            normalize_path(Path::new("C:/games/store/../store")),
            PathBuf::from("C:/games/store")
        );
        assert!(normalize_path(Path::new("C:/games/store/nested")).starts_with(&root));
    }
}
