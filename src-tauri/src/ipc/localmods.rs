//! Local mods, Jar-in-Jar, dependency resolution, and unified resource IPC commands.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::time::SystemTime;

use mclr_comp::resources::{
    ImportResourcesResult, OverwritePolicy, ResourceEntry, ResourceKind, ResourceState,
    SchematicInfo, delete_resources, export_resource_list, import_resources, scan_resources,
    schematic_info, toggle_resource,
};
use mclr_comp::{
    CompError, CompFile, CurseForgeClient, JarInJarNode, LocalModInfo, ModrinthClient,
    ResolvedDependencyItem, check_local_mods_updates, delete_local_mod, resolve_dependencies,
    scan_jar_in_jar, scan_local_mods, toggle_local_mod,
};
use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Emitter, State};
use tokio::sync::RwLock;
use url::Url;

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

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalModsScanResult {
    pub entries: Vec<ResourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LocalModUpdateItem {
    pub current_file_name: String,
    pub new_file: CompFile,
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

static LOCAL_MOD_CACHE: LazyLock<RwLock<HashMap<String, LocalModCacheEntry>>> =
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

async fn instance_mods_dir(state: &State<'_, SettingsStore>, instance_id: &str) -> PathBuf {
    let mods_dir = super::game_dir_for(state, instance_id).await.join("mods");
    let _ = tokio::fs::create_dir_all(&mods_dir).await;
    mods_dir
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_list(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<Vec<LocalModInfo>, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || scan_local_mods(&mods_dir)).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_toggle(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
    enabled: bool,
) -> Result<LocalModInfo, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || toggle_local_mod(&mods_dir, &file_name, enabled)).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_delete(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<(), AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || delete_local_mod(&mods_dir, &file_name)).await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_check_updates(
    state: State<'_, SettingsStore>,
    instance_id: String,
    mc_version: String,
    loader: String,
) -> Result<Vec<LocalModInfo>, AppError> {
    let settings = state.get().await;
    let modrinth = ModrinthClient::new(settings.net.source_priority);
    let curseforge = CurseForgeClient::new(settings.net.source_priority);

    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let mods = check_local_mods_updates(
        &mods_dir,
        &mc_version,
        &loader,
        &modrinth,
        Some(&curseforge),
    )
    .await
    .map_err(AppError::from)?;

    Ok(mods)
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_update(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    instance_id: String,
    current_file_name: String,
    new_file: CompFile,
) -> Result<String, AppError> {
    if new_file.distribution_disabled || new_file.download_urls.is_empty() {
        return Err(CompError::DistributionDisabled {
            project_id: new_file.project_id,
            file_id: new_file.id,
            website_url: None,
        }
        .into());
    }

    let urls: Vec<Url> = new_file
        .download_urls
        .iter()
        .filter_map(|u| Url::parse(u).ok())
        .collect();
    if urls.is_empty() {
        return Err(
            CompError::InvalidParams("No valid download URLs in CompFile".to_string()).into(),
        );
    }

    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let dest_path = mods_dir.join(&new_file.file_name);

    let mut params = HashMap::new();
    params.insert("name".to_string(), new_file.name.clone());
    params.insert("fileName".to_string(), new_file.file_name.clone());
    params.insert("targetInstance".to_string(), instance_id);

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.compDownload", params)
        .await;

    let spec = DownloadSpec {
        urls,
        dest: dest_path,
        size: if new_file.size > 0 {
            Some(new_file.size)
        } else {
            None
        },
        sha1: new_file.hashes.sha1,
        sha256: None,
        md5: None,
    };

    let settings = state.get().await;
    let client = http.client();
    let downloader = Downloader::new(client, settings.net.clone());
    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();
    let old_file_name = current_file_name.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match downloader.download(spec, Some((&tc, &tid))).await {
            Ok(()) => {
                if !old_file_name.is_empty() && old_file_name != new_file.file_name {
                    let old_path = mods_dir.join(&old_file_name);
                    if old_path.exists() {
                        let _ = delete_local_mod(&mods_dir, &old_file_name);
                    }
                }
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn local_mod_jar_in_jar(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<Vec<JarInJarNode>, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let jar_path = mods_dir.join(&file_name);
    super::run_blocking(move || scan_jar_in_jar(&jar_path)).await
}

#[tauri::command]
#[specta::specta]
pub async fn comp_resolve_dependencies(
    state: State<'_, SettingsStore>,
    file: CompFile,
    instance_id: String,
    mc_version: String,
    loader: String,
) -> Result<Vec<ResolvedDependencyItem>, AppError> {
    let settings = state.get().await;
    let modrinth = ModrinthClient::new(settings.net.source_priority);
    let curseforge = CurseForgeClient::new(settings.net.source_priority);

    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let installed_mods = super::run_blocking(move || scan_local_mods(&mods_dir))
        .await
        .unwrap_or_default();
    let mut installed_project_ids = HashSet::new();
    let mut installed_mod_ids = HashSet::new();

    for m in installed_mods {
        installed_mod_ids.insert(m.mod_id.to_ascii_lowercase());
        if let Some(pid) = m.project_id {
            installed_project_ids.insert(pid.to_ascii_lowercase());
        }
    }

    let items = resolve_dependencies(
        &file,
        &mc_version,
        &loader,
        &installed_project_ids,
        &installed_mod_ids,
        &modrinth,
        Some(&curseforge),
    )
    .await
    .map_err(AppError::from)?;

    Ok(items)
}

// -----------------------------------------------------------------------------
// Unified Resources IPC Commands (Phase B3)
// -----------------------------------------------------------------------------

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
                    "resources://patch",
                    ResourcesPatchEvent {
                        request_id: req_id.clone(),
                        instance_id: inst_id.clone(),
                        patches,
                    },
                );
            });

            let modrinth = ModrinthClient::new(settings.net.source_priority);
            let now_secs = current_timestamp_secs();

            for entry in entries_to_process {
                let jar_path = mods_dir.join(&entry.file_name);
                if !jar_path.exists() {
                    continue;
                }

                let file_name = entry.file_name.clone();
                let mtime = entry.modified_ms.unwrap_or(0);
                let size = entry.size;

                // Check if already completely cached with update check
                {
                    let cache = LOCAL_MOD_CACHE.read().await;
                    if let Some(c) = cache.get(&file_name) {
                        if c.size == size
                            && c.mtime_ms == mtime
                            && now_secs.saturating_sub(c.cached_at_secs) < 6 * 3600
                            && c.title.is_some()
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
pub async fn local_mods_scan_start(
    app: tauri::AppHandle,
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    instance_id: String,
    request_id: String,
) -> Result<LocalModsScanResult, AppError> {
    let entries = resources_list(
        app,
        http,
        state,
        instance_id,
        ResourceKind::Mod,
        None,
        Some(request_id),
    )
    .await?;
    Ok(LocalModsScanResult { entries })
}

#[tauri::command]
#[specta::specta]
pub async fn resource_toggle(
    _state: State<'_, SettingsStore>,
    _instance_id: String,
    path: String,
) -> Result<(), AppError> {
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

#[tauri::command]
#[specta::specta]
pub async fn local_mods_dependents(
    state: State<'_, SettingsStore>,
    instance_id: String,
    file_name: String,
) -> Result<Vec<String>, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    super::run_blocking(move || {
        let all_mods = scan_local_mods(&mods_dir)?;
        let target_mod = all_mods.iter().find(|m| m.file_name == file_name);
        let target_mod_id = match target_mod {
            Some(m) => m.mod_id.to_ascii_lowercase(),
            None => return Ok::<_, CompError>(Vec::new()),
        };

        let mut dependents = Vec::new();
        for m in &all_mods {
            if m.file_name == file_name {
                continue;
            }
            let jar_path = mods_dir.join(&m.file_name);
            if let Ok(nodes) = mclr_comp::jarinjar::scan_jar_in_jar(&jar_path) {
                fn has_id(nodes: &[mclr_comp::JarInJarNode], id: &str) -> bool {
                    for n in nodes {
                        if n.id.to_ascii_lowercase() == id || has_id(&n.children, id) {
                            return true;
                        }
                    }
                    false
                }
                if has_id(&nodes, &target_mod_id) {
                    dependents.push(m.name.clone());
                }
            }
        }
        Ok(dependents)
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn local_mods_update_many(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    instance_id: String,
    updates: Vec<LocalModUpdateItem>,
) -> Result<String, AppError> {
    let mods_dir = instance_mods_dir(&state, &instance_id).await;
    let mut params = HashMap::new();
    params.insert("targetInstance".to_string(), instance_id.clone());
    params.insert("count".to_string(), updates.len().to_string());

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.compDownload", params)
        .await;

    let settings = state.get().await;
    let client = http.client();
    let downloader = Downloader::new(client, settings.net.clone());
    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        let mut overall_err = None;

        for item in updates {
            let urls: Vec<Url> = item
                .new_file
                .download_urls
                .iter()
                .filter_map(|u| Url::parse(u).ok())
                .collect();
            if urls.is_empty() {
                continue;
            }
            let dest_path = mods_dir.join(&item.new_file.file_name);
            let spec = DownloadSpec {
                urls,
                dest: dest_path,
                size: if item.new_file.size > 0 {
                    Some(item.new_file.size)
                } else {
                    None
                },
                sha1: item.new_file.hashes.sha1,
                sha256: None,
                md5: None,
            };

            match downloader.download(spec, Some((&tc, &tid))).await {
                Ok(()) => {
                    if !item.current_file_name.is_empty()
                        && item.current_file_name != item.new_file.file_name
                    {
                        let old_path = mods_dir.join(&item.current_file_name);
                        if old_path.exists() {
                            let _ = delete_local_mod(&mods_dir, &item.current_file_name);
                        }
                    }
                }
                Err(e) => {
                    overall_err = Some(AppError::from(e));
                    break;
                }
            }
        }

        if let Some(e) = overall_err {
            let _ = tc.fail_task(&tid, e).await;
        } else {
            let _ = tc.complete_task(&tid).await;
        }
    });

    Ok(task_id)
}
