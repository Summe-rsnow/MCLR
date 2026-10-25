//! Unified resources (resource packs, shader packs, schematics, and mods metadata) IPC commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::SystemTime;

use mclr_comp::model::CompFile;
use mclr_comp::modrinth::ModrinthClient;
use mclr_comp::resources::{
    ImportResourcesResult, OverwritePolicy, ResourceEntry, ResourceKind, ResourceState,
    SchematicInfo, delete_resources, export_resource_list, import_resources, scan_resources,
    schematic_info, toggle_resource,
};
use mclr_core::{AppError, SettingsStore};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Emitter, State};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePatch {
    pub file_name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub tags: Option<Vec<String>>,
    pub mod_id: Option<String>,
    pub project_id: Option<String>,
    pub latest_file: Option<CompFile>,
    pub update_available: Option<bool>,
    pub dependents: Option<Vec<String>>,
    pub state: Option<ResourceState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesPatchEvent {
    pub request_id: String,
    pub instance_id: String,
    pub patches: Vec<ResourcePatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalModCacheEntry {
    pub file_name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub tags: Vec<String>,
    pub mod_id: Option<String>,
    pub project_id: Option<String>,
    pub latest_file: Option<CompFile>,
    pub update_available: bool,
    pub dependents: Vec<String>,
    pub size: u64,
    pub mtime_ms: u64,
    pub cached_at_secs: u64,
}

pub(crate) static LOCAL_MOD_CACHE: LazyLock<RwLock<HashMap<String, LocalModCacheEntry>>> =
    LazyLock::new(|| {
        let cache_file = mclr_core::paths::cache_dir().join("localmods.json");
        let map: HashMap<String, LocalModCacheEntry> = std::fs::read_to_string(&cache_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        RwLock::new(map)
    });

fn current_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

async fn flush_cache_to_disk() {
    let cache = LOCAL_MOD_CACHE.read().await;
    let cache_dir = mclr_core::paths::cache_dir();
    let _ = tokio::fs::create_dir_all(&cache_dir).await;
    let cache_file = cache_dir.join("localmods.json");
    if let Ok(json) = serde_json::to_string(&*cache) {
        let _ = tokio::fs::write(&cache_file, json).await;
    }
}

#[tauri::command]
#[specta::specta]
pub async fn resources_list(
    app: tauri::AppHandle,
    _http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    instance_id: String,
    kind: ResourceKind,
    rel: Option<String>,
    request_id: Option<String>,
) -> Result<Vec<ResourceEntry>, AppError> {
    let game_dir = super::game_dir_for(&state, &instance_id).await;
    let g_dir = game_dir.clone();
    let r = rel.clone();

    // Phase 1: fast filesystem scan + memory/disk cache for mods
    let mut entries =
        super::run_blocking(move || scan_resources(&g_dir, None, kind, r.as_deref())).await?;

    if kind == ResourceKind::Mod {
        let now_secs = current_timestamp_secs();
        let cache = LOCAL_MOD_CACHE.read().await;

        for entry in &mut entries {
            if let Some(cached) = cache.get(&entry.file_name) {
                let mtime = entry.modified_ms.unwrap_or(0);
                if cached.size == entry.size
                    && cached.mtime_ms == mtime
                    && now_secs.saturating_sub(cached.cached_at_secs) < 6 * 3600
                {
                    entry.title = cached.title.clone();
                    entry.description = cached.description.clone();
                    entry.version = cached.version.clone();
                    entry.tags = cached.tags.clone();
                    entry.mod_id = cached.mod_id.clone();
                    entry.project_id = cached.project_id.clone();
                    entry.latest_file = cached.latest_file.clone();
                    entry.update_available = cached.update_available;
                    entry.dependents = cached.dependents.clone();
                }
            }
        }
    }

    // Phase 2: If mod and request_id provided, spawn background streaming task
    if let (ResourceKind::Mod, Some(req_id)) = (kind, request_id) {
        let inst_id = instance_id.clone();
        let app_handle = app.clone();
        let mods_dir = game_dir.join("mods");
        let entries_to_process = entries.clone();
        let settings = state.get().await;

        tokio::spawn(async move {
            let batcher = crate::events::spawn_default(move |patches: Vec<ResourcePatch>| {
                let _ = app_handle.emit(
                    "resources://patchBatch",
                    ResourcesPatchEvent {
                        request_id: req_id.clone(),
                        instance_id: inst_id.clone(),
                        patches,
                    },
                );
            });

            let modrinth = Arc::new(ModrinthClient::new(settings.net.source_priority));
            let now_secs = current_timestamp_secs();

            for entry in entries_to_process {
                let jar_path = mods_dir.join(&entry.file_name);
                let file_name = entry.file_name.clone();
                let size = entry.size;
                let mtime = entry.modified_ms.unwrap_or(0);

                // Quick cache check
                {
                    let cache = LOCAL_MOD_CACHE.read().await;
                    if let Some(c) = cache.get(&file_name) {
                        if c.size == size
                            && c.mtime_ms == mtime
                            && now_secs.saturating_sub(c.cached_at_secs) < 6 * 3600
                        {
                            continue;
                        }
                    }
                }

                // Resolve metadata
                let meta_res = tokio::task::spawn_blocking(move || {
                    let bytes = std::fs::read(&jar_path).ok()?;
                    let meta = mclr_comp::local::parse_mod_metadata(&bytes, &file_name);
                    let sha1 = hex::encode(<sha1::Sha1 as sha1::Digest>::digest(&bytes));
                    let murmur2 = mclr_comp::local::compute_curseforge_murmur2(&bytes);
                    Some((meta, sha1, murmur2))
                })
                .await
                .ok()
                .flatten();

                if let Some((meta, sha1, _murmur2)) = meta_res {
                    let mut patch = ResourcePatch {
                        file_name: entry.file_name.clone(),
                        title: Some(meta.name.clone()),
                        description: meta.description.clone(),
                        version: if meta.version.is_empty() {
                            None
                        } else {
                            Some(meta.version.clone())
                        },
                        tags: Some(if meta.loader.is_empty() || meta.loader == "Unknown" {
                            Vec::new()
                        } else {
                            vec![meta.loader.clone()]
                        }),
                        mod_id: Some(meta.mod_id.clone()),
                        project_id: None,
                        latest_file: None,
                        update_available: Some(false),
                        dependents: None,
                        state: Some(ResourceState::Ok),
                    };

                    // Check Modrinth by sha1
                    if let Ok(files_map) = modrinth.get_version_files_by_sha1(&[sha1]).await {
                        if let Some((_, f)) = files_map.into_iter().next() {
                            patch.project_id = Some(f.project_id.clone());
                        }
                    }

                    // Push patch
                    batcher.push(patch.clone());

                    // Store in cache
                    let mut cache = LOCAL_MOD_CACHE.write().await;
                    cache.insert(
                        entry.file_name.clone(),
                        LocalModCacheEntry {
                            file_name: entry.file_name,
                            title: patch.title,
                            description: patch.description,
                            version: patch.version,
                            tags: patch.tags.unwrap_or_default(),
                            mod_id: patch.mod_id,
                            project_id: patch.project_id,
                            latest_file: patch.latest_file,
                            update_available: patch.update_available.unwrap_or(false),
                            dependents: Vec::new(),
                            size,
                            mtime_ms: mtime,
                            cached_at_secs: now_secs,
                        },
                    );
                }
            }

            drop(batcher);
            flush_cache_to_disk().await;
        });
    }

    Ok(entries)
}

#[tauri::command]
#[specta::specta]
pub async fn resource_toggle(
    _state: State<'_, SettingsStore>,
    _instance_id: String,
    path: String,
) -> Result<String, AppError> {
    super::run_blocking(move || toggle_resource(Path::new(&path))).await
}

#[tauri::command]
#[specta::specta]
pub async fn resources_delete(paths: Vec<String>, permanent: bool) -> Result<(), AppError> {
    super::run_blocking(move || {
        let p: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
        delete_resources(&p, permanent)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn resources_import(
    state: State<'_, SettingsStore>,
    instance_id: String,
    kind: ResourceKind,
    sources: Vec<String>,
    policy: OverwritePolicy,
) -> Result<ImportResourcesResult, AppError> {
    let game_dir = super::game_dir_for(&state, &instance_id).await;
    let mc_folder = super::current_mc_folder(&state).await;

    // Check if modable
    let modable = super::run_blocking(move || {
        let instances = mclr_game::instance::scan_instances(&mc_folder).unwrap_or_default();
        Ok::<_, AppError>(
            instances
                .iter()
                .find(|i| i.id == instance_id || i.name == instance_id)
                .map(|inst| inst.modable || mclr_game::gamedir::is_modable_loaders(&inst.loaders))
                .unwrap_or(true),
        )
    })
    .await?;

    let p: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    super::run_blocking(move || import_resources(kind, &game_dir, modable, &p, policy)).await
}

#[tauri::command]
#[specta::specta]
pub async fn resources_export(
    entries: Vec<ResourceEntry>,
    format: String,
) -> Result<String, AppError> {
    super::run_blocking(move || Ok::<_, AppError>(export_resource_list(&entries, &format))).await
}

#[tauri::command]
#[specta::specta]
pub async fn schematic_inspect(path: String) -> Result<SchematicInfo, AppError> {
    super::run_blocking(move || schematic_info(Path::new(&path))).await
}
