//! Scanning and caching of Minecraft instances in versions folder.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use super::classifier::{
    classify_cards, classify_instance, extract_loader_versions, extract_mc_version,
    is_modable_instance,
};
use super::{CardKind, InstanceConfig, InstanceInfo, InstanceKind, InstanceState};
use crate::error::GameError;
use crate::version::{VersionJson, merge_version_json};

#[derive(Debug, Clone)]
pub(crate) struct CachedInstance {
    pub(crate) json_mtime: Option<SystemTime>,
    pub(crate) config_mtime: Option<SystemTime>,
    pub(crate) info: InstanceInfo,
}

#[derive(Default)]
pub(crate) struct ScanCache {
    pub(crate) instances: HashMap<String, CachedInstance>,
}

pub(crate) static INSTANCE_CACHE: LazyLock<Mutex<HashMap<PathBuf, ScanCache>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Ensures `<mc_folder>/versions` directory and `launcher_profiles.json` exist.
pub fn ensure_game_folder(path: &Path) -> Result<(), GameError> {
    let versions_dir = path.join("versions");
    if !versions_dir.exists() {
        fs::create_dir_all(&versions_dir).map_err(GameError::Io)?;
    }

    let profiles = path.join("launcher_profiles.json");
    if !profiles.exists() {
        let default_content = r#"{"profiles":{}}"#;
        fs::write(profiles, default_content).map_err(GameError::Io)?;
    }

    Ok(())
}

/// Scans instances inside `<mc_folder>/versions`.
pub fn scan_instances(mc_folder: &Path) -> Result<Vec<InstanceInfo>, GameError> {
    let versions_dir = mc_folder.join("versions");
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&versions_dir).map_err(GameError::Io)?;
    let mut active_ids = HashSet::new();
    let mut instances = Vec::new();
    // Freshly computed cache entries for this pass; published in one short lock
    // at the end so no filesystem work happens while holding the cache lock.
    let mut pending: Vec<(String, CachedInstance)> = Vec::new();

    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if !file_type.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let instance_path = entry.path();

        // Skip cache directory, .mclr directories, and folders containing .mclr-installing marker
        if dir_name == "cache"
            || dir_name.starts_with(".mclr")
            || instance_path.join(".mclr-installing").exists()
        {
            continue;
        }

        // Skip empty directories
        let is_empty = fs::read_dir(&instance_path)
            .map(|mut it| it.next().is_none())
            .unwrap_or(true);
        if is_empty {
            continue;
        }

        active_ids.insert(dir_name.clone());

        let json_path = instance_path.join(format!("{dir_name}.json"));
        let config_path = instance_path.join("mclr.instance.json");

        let json_mtime = fs::metadata(&json_path)
            .ok()
            .and_then(|m| m.modified().ok());
        let config_mtime = fs::metadata(&config_path)
            .ok()
            .and_then(|m| m.modified().ok());

        // Check cache hit (short critical section: cache lookup only)
        let cached_info = {
            let cache_guard = INSTANCE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
            cache_guard
                .get(mc_folder)
                .and_then(|c| c.instances.get(&dir_name))
                .filter(|c| c.json_mtime == json_mtime && c.config_mtime == config_mtime)
                .map(|c| c.info.clone())
        };
        if let Some(info) = cached_info {
            instances.push(info);
            continue;
        }

        // Read config or default
        let mut config = if config_path.exists() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|content| serde_json::from_str::<InstanceConfig>(&content).ok())
                .unwrap_or_default()
        } else {
            InstanceConfig::default()
        };
        if config.pinned && !config.starred {
            config.starred = true;
        } else if config.starred && !config.pinned {
            config.pinned = true;
        }

        if !json_path.exists() {
            let info = InstanceInfo {
                id: dir_name.clone(),
                name: dir_name.clone(),
                folder: mc_folder.to_string_lossy().to_string(),
                path: instance_path.to_string_lossy().to_string(),
                mc_version: "Unknown".to_string(),
                kind: InstanceKind::Vanilla,
                loaders: Vec::new(),
                state: InstanceState::Incomplete,
                config,
                modable: false,
                card: CardKind::Error,
                release_time: None,
                loader_versions: BTreeMap::new(),
            };
            pending.push((
                dir_name,
                CachedInstance {
                    json_mtime,
                    config_mtime,
                    info: info.clone(),
                },
            ));
            instances.push(info);
            continue;
        }

        let json_str = match fs::read_to_string(&json_path) {
            Ok(s) => s,
            Err(_) => {
                let info = InstanceInfo {
                    id: dir_name.clone(),
                    name: dir_name.clone(),
                    folder: mc_folder.to_string_lossy().to_string(),
                    path: instance_path.to_string_lossy().to_string(),
                    mc_version: "Unknown".to_string(),
                    kind: InstanceKind::Vanilla,
                    loaders: Vec::new(),
                    state: InstanceState::Broken,
                    config,
                    modable: false,
                    card: CardKind::Error,
                    release_time: None,
                    loader_versions: BTreeMap::new(),
                };
                pending.push((
                    dir_name,
                    CachedInstance {
                        json_mtime,
                        config_mtime,
                        info: info.clone(),
                    },
                ));
                instances.push(info);
                continue;
            }
        };

        let version_json = match VersionJson::parse(&json_str) {
            Ok(v) => v,
            Err(_) => {
                let info = InstanceInfo {
                    id: dir_name.clone(),
                    name: dir_name.clone(),
                    folder: mc_folder.to_string_lossy().to_string(),
                    path: instance_path.to_string_lossy().to_string(),
                    mc_version: "Unknown".to_string(),
                    kind: InstanceKind::Vanilla,
                    loaders: Vec::new(),
                    state: InstanceState::Broken,
                    config,
                    modable: false,
                    card: CardKind::Error,
                    release_time: None,
                    loader_versions: BTreeMap::new(),
                };
                pending.push((
                    dir_name,
                    CachedInstance {
                        json_mtime,
                        config_mtime,
                        info: info.clone(),
                    },
                ));
                instances.push(info);
                continue;
            }
        };

        let effective_json = if let Some(parent_id) = &version_json.inherits_from {
            let parent_json_path = versions_dir
                .join(parent_id)
                .join(format!("{parent_id}.json"));
            if parent_json_path.exists() {
                if let Ok(parent_str) = fs::read_to_string(&parent_json_path) {
                    if let Ok(parent_v) = VersionJson::parse(&parent_str) {
                        merge_version_json(version_json.clone(), parent_v)
                    } else {
                        version_json.clone()
                    }
                } else {
                    version_json.clone()
                }
            } else {
                version_json.clone()
            }
        } else {
            version_json.clone()
        };

        let mc_version = extract_mc_version(&effective_json, &dir_name);
        let (kind, loaders) = classify_instance(&effective_json, &mc_version);
        let release_time = effective_json
            .release_time
            .clone()
            .or_else(|| effective_json.time.clone());
        let loader_versions = extract_loader_versions(&loaders);
        let modable = is_modable_instance(&loaders, config.card_type);

        let info = InstanceInfo {
            id: dir_name.clone(),
            name: dir_name.clone(),
            folder: mc_folder.to_string_lossy().to_string(),
            path: instance_path.to_string_lossy().to_string(),
            mc_version,
            kind,
            loaders,
            state: InstanceState::Ok,
            config,
            modable,
            card: CardKind::Regular,
            release_time,
            loader_versions,
        };

        pending.push((
            dir_name,
            CachedInstance {
                json_mtime,
                config_mtime,
                info: info.clone(),
            },
        ));
        instances.push(info);
    }

    classify_cards(&mut instances);

    // Single short critical section publishing the results; no IO above happens
    // while the cache lock is held.
    {
        let mut cache_guard = INSTANCE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        let folder_cache = cache_guard.entry(mc_folder.to_path_buf()).or_default();

        folder_cache
            .instances
            .retain(|id, _| active_ids.contains(id));
        for (id, entry) in pending {
            folder_cache.instances.insert(id, entry);
        }

        for inst in &instances {
            if let Some(cached) = folder_cache.instances.get_mut(&inst.id) {
                cached.info = inst.clone();
            }
        }
    }

    Ok(instances)
}
