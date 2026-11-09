//! Unified resources (resource packs, shader packs, schematics, and mods metadata) IPC commands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use mclr_comp::LocalModInfo;
use mclr_comp::curseforge::CurseForgeClient;
use mclr_comp::model::{CompFile, CompPlatform};
use mclr_comp::modname;
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
    /// Chinese name resolved from the project; omitted (not null) when unknown so the
    /// already-scanned entry keeps its dictionary-based name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zh_name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub tags: Option<Vec<String>>,
    pub mod_id: Option<String>,
    pub project_id: Option<String>,
    /// Platform + platform file id of the installed jar, once identified. Omitted when unknown
    /// so the row keeps whatever Phase 1 already resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<CompPlatform>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_file: Option<CompFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    let mc_folder = super::current_mc_folder(&state).await;
    let g_dir = super::game_dir_for(&state, &instance_id).await;
    let settings = state.snapshot().await;
    let show_zh_name = settings.comp.show_zh_name;
    let r = rel.clone();
    let inst_id = instance_id.clone();

    // Phase 1: a single blocking pass resolves the instance's Minecraft version, scans the
    // resource folder (legacy `mods/<mc_version>/` included) and loads the local-mod cache.
    let (mut entries, mut local_mods, ledger) = super::run_blocking(move || {
        // Only mods have a version-specific legacy folder, so no other kind pays for the
        // instance scan.
        let mc_version = if kind == ResourceKind::Mod {
            mclr_game::instance::scan_instances(&mc_folder)
                .unwrap_or_default()
                .into_iter()
                .find(|i| i.id == inst_id || i.name == inst_id)
                .map(|i| i.mc_version)
                .filter(|v| !v.is_empty() && v.as_str() != "Unknown")
        } else {
            None
        };

        let entries = scan_resources(&g_dir, mc_version.as_deref(), kind, r.as_deref())?;

        let mut local_mods: Vec<LocalModInfo> = Vec::new();
        let mut ledger = Vec::new();
        if kind == ResourceKind::Mod {
            let mods_dir = g_dir.join("mods");
            local_mods = mclr_comp::local::scan_local_mods(&mods_dir).unwrap_or_default();
            // Legacy Forge installs keep their jars in `mods/<mc_version>/`, so that folder
            // has to contribute local-mod metadata too instead of falling back to bare file
            // names. `scan_resources` already listed those entries.
            if let Some(ver) = mc_version.as_deref() {
                let legacy_dir = mods_dir.join(ver);
                if legacy_dir.is_dir() {
                    local_mods
                        .extend(mclr_comp::local::scan_local_mods(&legacy_dir).unwrap_or_default());
                }
            }
            // Mods applied by a preset (or installed from CurseForge) carry an identity that the
            // jar alone cannot prove; the ledger is the only place that records it, and reading
            // it here costs one small file instead of a network round trip on first paint.
            ledger = mclr_comp::preset::ledger::load(&g_dir);
        }

        Ok::<_, AppError>((entries, local_mods, ledger))
    })
    .await?;

    if kind == ResourceKind::Mod {
        let by_path: HashMap<PathBuf, &LocalModInfo> = local_mods
            .iter()
            .map(|m| (PathBuf::from(&m.path), m))
            .collect();
        let by_name_size: HashMap<(&str, u64), &LocalModInfo> = local_mods
            .iter()
            .map(|m| ((m.file_name.as_str(), m.size), m))
            .collect();
        for entry in &mut entries {
            // Full path first: the flat folder and the legacy subfolder can both hold a jar
            // with the same name, so file name plus size alone is ambiguous.
            let matched = by_path.get(Path::new(&entry.path)).copied().or_else(|| {
                by_name_size
                    .get(&(entry.file_name.as_str(), entry.size))
                    .copied()
            });
            let Some(m) = matched else {
                continue;
            };
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
            entry.platform = m.platform;
            entry.file_id = m.file_id.clone();
            // The dictionary is offline and instant, so the Chinese name is already there
            // on the first paint instead of waiting for the network patches.
            entry.zh_name = if show_zh_name {
                modname::lookup_by_slug(&m.mod_id)
                    .map(modname::display_zh)
                    .map(str::to_string)
            } else {
                None
            };
            // Jar artwork is instant; project artwork arrives with the background patches.
            entry.icon = m.icon.clone();
        }

        // Ledger identities win over the jar-derived ones: they record which exact file a preset
        // (or a CurseForge install) put on disk, including the `rel_path` inside the mods folder.
        let by_rel: HashMap<String, &mclr_comp::preset::ledger::LedgerEntry> = ledger
            .iter()
            .map(|item| (item.rel_path.replace('\\', "/"), item))
            .collect();
        for entry in &mut entries {
            let rel = format!("mods/{}", entry.rel_path.replace('\\', "/"));
            let Some(item) = by_rel.get(&rel) else {
                continue;
            };
            entry.platform = Some(item.platform);
            entry.project_id = Some(item.project_id.clone());
            entry.file_id = Some(item.file_id.clone());
            // Carry the identity into the local-mod record too: Phase 2 then neither re-queries
            // the platforms for this jar nor replaces the ledger's platform with another one
            // that happens to know the same hash.
            if let Some(m) = local_mods.iter_mut().find(|m| m.path == entry.path) {
                m.platform = Some(item.platform);
                m.project_id = Some(item.project_id.clone());
                m.file_id = Some(item.file_id.clone());
            }
        }
    }

    // Phase 2: resolve Modrinth project ids in batches and stream the patches.
    if let (ResourceKind::Mod, Some(req_id)) = (kind, request_id)
        && !local_mods.is_empty()
    {
        let inst_id = instance_id.clone();
        let app_handle = app.clone();
        let settings = Arc::clone(&settings);

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

            // `sha1 -> (platform, project_id, file_id)`: the full identity a preset needs to
            // reproduce the exact installed jar. Mods the scan already knew keep their triple
            // until a lookup replaces it.
            let mut identities: HashMap<String, (CompPlatform, String, String)> = local_mods
                .iter()
                .filter_map(|m| {
                    Some((
                        m.sha1.clone(),
                        (m.platform?, m.project_id.clone()?, m.file_id.clone()?),
                    ))
                })
                .collect();

            // `sha1 -> Modrinth project id`, used only for the metadata batch below: a
            // CurseForge project id is not a valid Modrinth id and must never be requested.
            let mut project_ids: HashMap<String, String> = local_mods
                .iter()
                .filter(|m| m.platform.unwrap_or(CompPlatform::Modrinth) == CompPlatform::Modrinth)
                .filter_map(|m| m.project_id.clone().map(|p| (m.sha1.clone(), p)))
                .collect();

            // One batched request per `MODRINTH_LOOKUP_BATCH` unresolved hashes instead of
            // one request per mod. Every hash we actually asked about is recorded so a miss
            // is not retried for six hours.
            let pending = mclr_comp::local::mods_needing_project_lookup(&local_mods);
            let mut attempted: HashMap<String, Option<(CompPlatform, String, String)>> =
                HashMap::new();
            // Hashes Modrinth answered without knowing; the CurseForge fingerprint fallback
            // covers exactly those.
            let mut modrinth_misses: Vec<&str> = Vec::new();
            let murmur2_by_sha1: HashMap<&str, u32> = local_mods
                .iter()
                .map(|m| (m.sha1.as_str(), m.murmur2))
                .collect();
            for chunk in pending.chunks(mclr_comp::local::MODRINTH_LOOKUP_BATCH) {
                let Ok(files) = modrinth.get_version_files_by_sha1(chunk).await else {
                    // Network failure: leave the hashes unrecorded so they are retried.
                    continue;
                };
                for sha1 in chunk {
                    match files.get(sha1) {
                        Some(file) => {
                            let identity = (
                                CompPlatform::Modrinth,
                                file.project_id.clone(),
                                file.id.clone(),
                            );
                            project_ids.insert(sha1.clone(), file.project_id.clone());
                            identities.insert(sha1.clone(), identity.clone());
                            attempted.insert(sha1.clone(), Some(identity));
                        }
                        None => modrinth_misses.push(sha1.as_str()),
                    }
                }
            }

            // CurseForge fingerprints resolve the jars Modrinth has never seen. The lookup is
            // batched the same way, and a missing API key simply means no fallback.
            let mut cf_identities: HashMap<u32, (CompPlatform, String, String)> = HashMap::new();
            if !modrinth_misses.is_empty() {
                let fingerprints: Vec<u32> = modrinth_misses
                    .iter()
                    .filter_map(|sha1| murmur2_by_sha1.get(sha1).copied())
                    .filter(|m2| *m2 > 0)
                    .collect();
                let curseforge = CurseForgeClient::new(settings.net.source_priority);
                for chunk in fingerprints.chunks(mclr_comp::local::MODRINTH_LOOKUP_BATCH) {
                    let Ok(files) = curseforge.match_fingerprints(chunk).await else {
                        // Network failure: leave the hashes unrecorded so they are retried.
                        continue;
                    };
                    for file in files {
                        if let Some(murmur2) = file.hashes.murmur2 {
                            cf_identities.insert(
                                murmur2,
                                (CompPlatform::Curseforge, file.project_id, file.id),
                            );
                        }
                    }
                }
            }
            for sha1 in modrinth_misses {
                let identity = murmur2_by_sha1
                    .get(sha1)
                    .and_then(|murmur2| cf_identities.get(murmur2))
                    .cloned();
                if let Some(identity) = &identity {
                    identities.insert(sha1.to_string(), identity.clone());
                }
                // Recording the miss (even after the fallback) is what keeps the six-hour
                // retry window from firing on every visit.
                attempted.insert(sha1.to_string(), identity);
            }
            if !attempted.is_empty() {
                let _ = tokio::task::spawn_blocking(move || {
                    mclr_comp::local::record_mod_identities(&attempted);
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
            let mut zh_names: HashMap<String, String> = HashMap::new();
            for chunk in metadata_request.chunks(mclr_comp::local::MODRINTH_LOOKUP_BATCH) {
                let Ok(projects) = modrinth.get_projects_batch(chunk).await else {
                    break;
                };
                for project in projects {
                    if let Some(url) = project.icon_url {
                        icon_urls.insert(project.id.clone(), url);
                    }
                    if show_zh_name && let Some(zh_name) = project.zh_name.clone() {
                        zh_names.insert(project.id.clone(), zh_name);
                    }
                    if !project.categories.is_empty() {
                        project_categories.insert(project.id, project.categories);
                    }
                }
            }

            for m in &local_mods {
                // The network result wins over what the scan cached; either way the three
                // fields always describe the same file.
                let identity = identities.get(&m.sha1);
                let platform = identity.map(|i| i.0).or(m.platform);
                let project_id = identity
                    .map(|i| i.1.clone())
                    .or_else(|| m.project_id.clone());
                let file_id = identity.map(|i| i.2.clone()).or_else(|| m.file_id.clone());
                // Artwork, categories and the Chinese name only exist on Modrinth here.
                let metadata_project = project_ids.get(&m.sha1);
                let remote_icon = metadata_project
                    .and_then(|id| icon_urls.get(id))
                    .map(String::as_str);
                let mut tags: Vec<String> = Vec::new();
                if !m.loader.is_empty() && m.loader != "Unknown" {
                    tags.push(m.loader.clone());
                }
                if let Some(categories) = metadata_project.and_then(|id| project_categories.get(id))
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
                    // The project's own name wins over the dictionary: it is the same table
                    // the project page shows, and slugs can differ from local mod ids.
                    zh_name: if show_zh_name {
                        metadata_project
                            .and_then(|id| zh_names.get(id))
                            .cloned()
                            .or_else(|| {
                                modname::lookup_by_slug(&m.mod_id)
                                    .map(modname::display_zh)
                                    .map(str::to_string)
                            })
                    } else {
                        None
                    },
                    description: m.description.clone(),
                    version: Some(m.version.clone()).filter(|v| !v.is_empty()),
                    tags: Some(tags),
                    mod_id: Some(m.mod_id.clone()),
                    project_id,
                    platform,
                    file_id,
                    latest_file: None,
                    update_available: None,
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
                .map(|inst| inst.modable || mclr_core::loader::is_modable_labels(&inst.loaders))
                .unwrap_or(true),
        )
    })
    .await?;

    let mut validated: Vec<PathBuf> = Vec::with_capacity(sources.len());
    for s in &sources {
        validated.push(crate::security::ensure_not_system_path(Path::new(s))?);
    }
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
