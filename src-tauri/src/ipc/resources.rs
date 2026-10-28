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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesPatchEvent {
    pub request_id: String,
    pub instance_id: String,
    pub patches: Vec<ResourcePatch>,
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

            for m in &local_mods {
                batcher.push(ResourcePatch {
                    file_name: m.file_name.clone(),
                    title: Some(m.name.clone()),
                    description: m.description.clone(),
                    version: Some(m.version.clone()).filter(|v| !v.is_empty()),
                    tags: Some(if m.loader.is_empty() || m.loader == "Unknown" {
                        Vec::new()
                    } else {
                        vec![m.loader.clone()]
                    }),
                    mod_id: Some(m.mod_id.clone()),
                    project_id: project_ids.get(&m.sha1).cloned(),
                    latest_file: None,
                    update_available: Some(false),
                    dependents: None,
                    state: Some(ResourceState::Ok),
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
