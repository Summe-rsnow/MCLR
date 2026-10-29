//! Unified resources (resource packs, shader packs, schematics, and mods metadata) IPC commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mclr_comp::LocalModInfo;
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
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesPatchEvent {
    pub request_id: String,
    pub instance_id: String,
    pub patches: Vec<ResourcePatch>,
}

/// Cover shown for an installed mod.
///
/// PCL resolves a local mod to its community project and draws `Comp.LogoUrl` first, falling
/// back to the artwork inside the jar — that is why its rows look uniform: every mod that
/// exists online gets the same high-resolution square icon. We mirror that order, and use
/// the API's own `icon_url` (the CDN serves content-hashed `.webp`, so a built path 404s).
fn mod_icon(remote: Option<&str>, local: Option<&str>) -> Option<String> {
    remote
        .map(str::to_string)
        .or_else(|| local.map(str::to_string))
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

    // Phase 1: fast filesystem scan, plus the single local-mod cache for mod metadata
    let mut entries =
        super::run_blocking(move || scan_resources(&g_dir, None, kind, r.as_deref())).await?;

    let mut local_mods: Vec<LocalModInfo> = Vec::new();
    if kind == ResourceKind::Mod {
        let mods_dir = game_dir.join("mods");
        local_mods = super::run_blocking(move || mclr_comp::local::scan_local_mods(&mods_dir))
            .await
            .unwrap_or_default();

        let by_name: HashMap<&str, &LocalModInfo> = local_mods
            .iter()
            .map(|m| (m.file_name.as_str(), m))
            .collect();
        for entry in &mut entries {
            let Some(m) = by_name.get(entry.file_name.as_str()) else {
                continue;
            };
            if m.size != entry.size {
                continue;
            }
            entry.title = Some(m.name.clone());
            entry.description = m.description.clone();
            entry.version = Some(m.version.clone()).filter(|v| !v.is_empty());
            entry.tags = if m.loader.is_empty() || m.loader == "Unknown" {
                Vec::new()
            } else {
                vec![m.loader.clone()]
            };
            entry.mod_id = Some(m.mod_id.clone());
            entry.project_id = m.project_id.clone();
            // Jar artwork is instant; project artwork arrives with the background patches.
            entry.icon = m.icon.clone();
        }
    }

    // Phase 2: resolve Modrinth project ids in batches and stream the patches.
    if let (ResourceKind::Mod, Some(req_id)) = (kind, request_id)
        && !local_mods.is_empty()
    {
        let inst_id = instance_id.clone();
        let app_handle = app.clone();
        let settings = state.snapshot().await;

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

            // One batched request per `MODRINTH_LOOKUP_BATCH` unresolved hashes instead of
            // one request per mod. Every hash we actually asked about is recorded so a miss
            // is not retried for six hours.
            let pending = mclr_comp::local::mods_needing_project_lookup(&local_mods);
            let mut project_ids: HashMap<String, String> = local_mods
                .iter()
                .filter_map(|m| m.project_id.clone().map(|p| (m.sha1.clone(), p)))
                .collect();
            let mut attempted: HashMap<String, Option<String>> = HashMap::new();
            for chunk in pending.chunks(mclr_comp::local::MODRINTH_LOOKUP_BATCH) {
                let Ok(files) = modrinth.get_version_files_by_sha1(chunk).await else {
                    // Network failure: leave the hashes unrecorded so they are retried.
                    continue;
                };
                for sha1 in chunk {
                    let resolved = files.get(sha1).map(|f| f.project_id.clone());
                    if let Some(project_id) = &resolved {
                        project_ids.insert(sha1.clone(), project_id.clone());
                    }
                    attempted.insert(sha1.clone(), resolved);
                }
            }
            if !attempted.is_empty() {
                let _ = tokio::task::spawn_blocking(move || {
                    mclr_comp::local::record_project_ids(&attempted);
                })
                .await;
            }

            // Project metadata (cover art + categories) in one batched request per lookup
            // batch. A failure just leaves those rows on the placeholder glyph and their
            // jar-derived tags.
            let mut icon_urls: HashMap<String, String> = HashMap::new();
            let mut project_categories: HashMap<String, Vec<String>> = HashMap::new();
            let mut metadata_request: Vec<String> = local_mods
                .iter()
                .filter_map(|m| project_ids.get(&m.sha1).cloned())
                .collect();
            metadata_request.sort_unstable();
            metadata_request.dedup();
            for chunk in metadata_request.chunks(mclr_comp::local::MODRINTH_LOOKUP_BATCH) {
                let Ok(projects) = modrinth.get_projects_batch(chunk).await else {
                    break;
                };
                for project in projects {
                    if let Some(url) = project.icon_url {
                        icon_urls.insert(project.id.clone(), url);
                    }
                    if !project.categories.is_empty() {
                        project_categories.insert(project.id, project.categories);
                    }
                }
            }

            for m in &local_mods {
                let project_id = project_ids.get(&m.sha1).cloned();
                let remote_icon = project_id
                    .as_ref()
                    .and_then(|id| icon_urls.get(id))
                    .map(String::as_str);
                let mut tags: Vec<String> = Vec::new();
                if !m.loader.is_empty() && m.loader != "Unknown" {
                    tags.push(m.loader.clone());
                }
                if let Some(categories) = project_id
                    .as_ref()
                    .and_then(|id| project_categories.get(id))
                {
                    for category in categories {
                        if tags.len() >= 4 {
                            break;
                        }
                        if !tags.contains(category) {
                            tags.push(category.clone());
                        }
                    }
                }
                batcher.push(ResourcePatch {
                    file_name: m.file_name.clone(),
                    title: Some(m.name.clone()),
                    description: m.description.clone(),
                    version: Some(m.version.clone()).filter(|v| !v.is_empty()),
                    tags: Some(tags),
                    mod_id: Some(m.mod_id.clone()),
                    project_id: project_id.clone(),
                    latest_file: None,
                    update_available: Some(false),
                    dependents: None,
                    state: Some(ResourceState::Ok),
                    icon: mod_icon(remote_icon, m.icon.as_deref()),
                });
            }

            drop(batcher);
        });
    }

    Ok(entries)
}

#[tauri::command]
#[specta::specta]
pub async fn resource_toggle(
    state: State<'_, SettingsStore>,
    _instance_id: String,
    path: String,
) -> Result<String, AppError> {
    let settings = state.snapshot().await;
    let target = crate::security::ensure_allowed(Path::new(&path), &settings)?;
    super::run_blocking(move || toggle_resource(&target)).await
}

#[tauri::command]
#[specta::specta]
pub async fn resources_delete(
    state: State<'_, SettingsStore>,
    paths: Vec<String>,
    permanent: bool,
) -> Result<(), AppError> {
    // Destructive: only files inside the launcher's own directories.
    let settings = state.snapshot().await;
    let requested: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let validated = crate::security::ensure_all_allowed(&requested, &settings)?;
    super::run_blocking(move || delete_resources(&validated, permanent)).await
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

    let settings = state.snapshot().await;
    let p: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    let validated = crate::security::ensure_all_allowed(&p, &settings)?;
    super::run_blocking(move || import_resources(kind, &game_dir, modable, &validated, policy))
        .await
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
    let target = crate::security::ensure_not_system_path(Path::new(&path))?;
    super::run_blocking(move || schematic_info(&target)).await
}
