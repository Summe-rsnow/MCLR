//! Installed resources indexing across instances.
//!
//! The index answers two questions the preset planner asks: "is this exact file already
//! installed?" (by SHA1) and "is another version of this project installed?" (by project id).
//! Mods come from the cached local-mod scan, other resource folders are hashed on the spot, and
//! the instance ledger supplies the project identity of files that cannot be identified from
//! disk (CurseForge resource packs, shaders, datapacks, worlds). Hashes the ledger does not know
//! are looked up on Modrinth in one batch.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::local::{hash_file, scan_local_mods};
use crate::model::{CompPlatform, CompType};
use crate::modrinth::ModrinthClient;
use crate::preset::ledger;

// The planner keys a `HashSet<CompType>` and a `HashMap<(CompPlatform, _), _>` (both mandated by
// the preset contract), while `model.rs` — a shared contract file this slice must not edit —
// derives neither `Hash`. Both enums are plain fieldless discriminants, so hashing by discriminant
// is equivalent to a derived implementation.
impl Hash for CompType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).hash(state);
    }
}

impl Hash for CompPlatform {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self as u8).hash(state);
    }
}

/// A resource file already present in a game directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledHit {
    pub file_name: String,
    /// `/`-separated path relative to the game directory.
    pub rel_path: String,
    pub sha1: String,
    pub file_id: Option<String>,
}

/// Installed resources indexed by content hash and by community project.
#[derive(Debug, Clone, Default)]
pub struct InstalledIndex {
    pub by_sha1: HashMap<String, InstalledHit>,
    pub by_project: HashMap<(CompPlatform, String), Vec<InstalledHit>>,
}

impl InstalledIndex {
    /// Returns the installed file with exactly this SHA1.
    pub fn by_sha1(&self, sha1: &str) -> Option<&InstalledHit> {
        self.by_sha1.get(&sha1.to_ascii_lowercase())
    }

    /// Returns every installed file belonging to `(platform, project_id)`.
    pub fn by_project(&self, platform: CompPlatform, project_id: &str) -> &[InstalledHit] {
        self.by_project
            .get(&(platform, project_id.to_string()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// True when the index holds no hits at all.
    pub fn is_empty(&self) -> bool {
        self.by_sha1.is_empty() && self.by_project.is_empty()
    }

    /// Returns every installed file name (used for dependency "already installed" checks).
    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.by_sha1.values().map(|hit| hit.file_name.as_str())
    }
}

/// Collects the installed resources of `game_dir` relevant to `kinds`.
///
/// Never fails: unreadable folders and failed Modrinth lookups simply contribute no identity,
/// which degrades "already installed" detection but never blocks an apply.
pub async fn build(
    game_dir: &Path,
    kinds: &HashSet<CompType>,
    datapack_world: Option<&str>,
    modrinth: &ModrinthClient,
) -> InstalledIndex {
    // `rel_path` -> (hit, identity). Identity is filled by the ledger or the Modrinth batch.
    let mut found: HashMap<String, Found> = HashMap::new();

    if kinds.contains(&CompType::Mod) {
        let mods_dir = game_dir.join("mods");
        let scan_dir = mods_dir.clone();
        match tokio::task::spawn_blocking(move || scan_local_mods(&scan_dir)).await {
            Ok(Ok(mods)) => {
                for info in mods {
                    let rel_path = format!("mods/{}", info.file_name);
                    found.entry(rel_path.clone()).or_insert(Found {
                        hit: InstalledHit {
                            file_name: info.file_name,
                            rel_path,
                            sha1: info.sha1.to_ascii_lowercase(),
                            file_id: None,
                        },
                        platform: info.project_id.as_ref().map(|_| CompPlatform::Modrinth),
                        project_id: info.project_id,
                    });
                }
            }
            Ok(Err(err)) => {
                tracing::warn!(dir = %mods_dir.display(), error = %err, "failed to scan local mods for preset planning");
            }
            Err(err) => {
                tracing::warn!(error = %err, "local mod scan task failed for preset planning");
            }
        }
    }

    let mut dirs: Vec<(PathBuf, &str)> = Vec::new();
    if kinds.contains(&CompType::Resourcepack) {
        dirs.push((game_dir.join("resourcepacks"), "resourcepacks"));
    }
    if kinds.contains(&CompType::Shader) {
        dirs.push((game_dir.join("shaderpacks"), "shaderpacks"));
    }
    if kinds.contains(&CompType::Datapack)
        && let Some(world) = datapack_world.filter(|w| !w.trim().is_empty())
    {
        dirs.push((
            game_dir.join("saves").join(world).join("datapacks"),
            "datapacks-world",
        ));
    }

    for (dir, rel_prefix) in dirs {
        let world = if rel_prefix == "datapacks-world" {
            datapack_world.map(str::to_string)
        } else {
            None
        };
        let hashed = tokio::task::spawn_blocking(move || hash_zip_dir(&dir, world.as_deref()))
            .await
            .unwrap_or_default();
        for (rel_path, file_name, sha1) in hashed {
            found.entry(rel_path.clone()).or_insert(Found {
                hit: InstalledHit {
                    file_name,
                    rel_path,
                    sha1,
                    file_id: None,
                },
                platform: None,
                project_id: None,
            });
        }
    }

    // Ledger entries carry project identity and cover files that are not plain `.zip` resources.
    for entry in ledger::load(game_dir) {
        if !kinds.contains(&entry.kind) {
            continue;
        }
        if entry.rel_path.trim().is_empty() {
            continue;
        }
        // Worlds are extracted folders; every other kind is a single file.
        let exists = mclr_core::safe_path::safe_join(game_dir, &entry.rel_path)
            .map(|path| {
                if entry.kind == CompType::World {
                    path.exists()
                } else {
                    path.is_file()
                }
            })
            .unwrap_or(false);
        if !exists {
            continue;
        }
        let file_name = entry
            .rel_path
            .rsplit('/')
            .next()
            .unwrap_or(&entry.rel_path)
            .to_string();
        match found.get_mut(&entry.rel_path) {
            Some(existing) => {
                existing.platform = Some(entry.platform);
                existing.project_id = Some(entry.project_id.clone());
                if !entry.file_id.is_empty() {
                    existing.hit.file_id = Some(entry.file_id.clone());
                }
            }
            None => {
                found.insert(
                    entry.rel_path.clone(),
                    Found {
                        hit: InstalledHit {
                            file_name,
                            rel_path: entry.rel_path.clone(),
                            sha1: entry.sha1.to_ascii_lowercase(),
                            file_id: Some(entry.file_id.clone()),
                        },
                        platform: Some(entry.platform),
                        project_id: Some(entry.project_id.clone()),
                    },
                );
            }
        }
    }

    // CurseForge non-mod files and anything else the ledger did not record are identified by
    // content hash on Modrinth.
    let unmatched: Vec<String> = found
        .values()
        .filter(|f| f.project_id.is_none() && !f.hit.sha1.is_empty())
        .map(|f| f.hit.sha1.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if !unmatched.is_empty() {
        match modrinth.get_version_files_by_sha1(&unmatched).await {
            Ok(versions) => {
                for entry in found.values_mut() {
                    if entry.project_id.is_some() {
                        continue;
                    }
                    if let Some(file) = versions.get(&entry.hit.sha1) {
                        entry.platform = Some(CompPlatform::Modrinth);
                        entry.project_id = Some(file.project_id.clone());
                        entry.hit.file_id = Some(file.id.clone());
                    }
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "Modrinth hash lookup failed during preset planning");
            }
        }
    }

    let mut index = InstalledIndex::default();
    for entry in found.into_values() {
        index
            .by_sha1
            .entry(entry.hit.sha1.clone())
            .or_insert_with(|| entry.hit.clone());
        if let Some(project_id) = entry.project_id.clone() {
            let platform = entry.platform.unwrap_or(CompPlatform::Modrinth);
            index
                .by_project
                .entry((platform, project_id))
                .or_default()
                .push(entry.hit);
        }
    }
    index
}

/// Internal pairing of a hit with the identity discovered for it.
struct Found {
    hit: InstalledHit,
    platform: Option<CompPlatform>,
    project_id: Option<String>,
}

/// Hashes every `.zip` / `.zip.disabled` file in `dir`, returning `(rel_path, file_name, sha1)`.
fn hash_zip_dir(dir: &Path, world: Option<&str>) -> Vec<(String, String, String)> {
    let read_dir = match std::fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return Vec::new(),
    };
    let prefix = match world {
        Some(world) => format!("saves/{world}/datapacks"),
        None => dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string(),
    };
    let mut out = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !(file_name.ends_with(".zip") || file_name.ends_with(".zip.disabled")) {
            continue;
        }
        match hash_file(&path) {
            Ok((sha1, _)) => out.push((
                format!("{prefix}/{file_name}"),
                file_name.to_string(),
                sha1.to_ascii_lowercase(),
            )),
            Err(err) => {
                tracing::debug!(path = %path.display(), error = %err, "failed to hash resource file");
            }
        }
    }
    out
}
