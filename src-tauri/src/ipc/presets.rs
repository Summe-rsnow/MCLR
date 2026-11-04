//! Resource presets: CRUD, apply/caching orchestration, and the content-addressed store.
//!
//! The heavy lifting lives in `mclr_comp::preset` (pure planning, repository, store); this module
//! wires it to Tauri state, the task center and the network clients. Every apply/cache run is a
//! task so the UI can show progress and cancel it, and the resulting report is parked in
//! [`PresetState::reports`] until the frontend fetches it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::stream::{self, StreamExt};
use mclr_comp::preset::installed::{InstalledHit, build as build_installed_index};
use mclr_comp::preset::ledger::{self, LedgerEntry};
use mclr_comp::preset::model::{
    AddItemOutcome, ApplyReport, ApplyRequest, ApplyRow, ApplyStatus, LinkMode, PresetItem,
    PruneResult, ResourcePreset, StoreInfo,
};
use mclr_comp::preset::plan::{Decision, InstanceCtx, MergedItem, decide, merge_presets};
use mclr_comp::preset::repo::PresetRepo;
use mclr_comp::preset::store::Store;
use mclr_comp::{
    CompError, CompFile, CompPlatform, CompType, CurseForgeClient, ModrinthClient,
    ResolvedDependencyItem, delete_resources, modname, naming,
    resolver::{
        MatchFailure, expand_loaders, file_supports_any_loader, file_supports_mc,
        resolve_dependencies, select_best_file_filtered,
    },
};
use mclr_core::tasks::TaskControl;
use mclr_core::{AppError, Settings, SettingsStore, TaskCenter};
use mclr_net::Downloader;
use tauri::{AppHandle, Emitter, State};

/// Preset repository plus the apply reports waiting to be collected by the frontend.
pub struct PresetState {
    pub repo: tokio::sync::Mutex<PresetRepo>,
    pub reports: tokio::sync::Mutex<HashMap<String, ApplyReport>>,
}

impl PresetState {
    /// Loads the repository at `path` (a corrupt or missing file yields an empty repository).
    pub fn new(path: PathBuf) -> Self {
        Self {
            repo: tokio::sync::Mutex::new(PresetRepo::load(path)),
            reports: tokio::sync::Mutex::new(HashMap::new()),
        }
    }
}

/// Root directory of the content-addressed store.
///
/// Empty `storeDir` uses the launcher cache; a custom value may live on another volume, which is
/// also why it is added to the security roots.
pub fn store_root(settings: &Settings) -> PathBuf {
    let custom = settings.comp.store_dir.trim();
    if custom.is_empty() {
        mclr_core::paths::cache_dir().join("store")
    } else {
        PathBuf::from(custom)
    }
}

type FileKey = (CompPlatform, String);
type FileResults = HashMap<FileKey, Result<Vec<CompFile>, AppError>>;

/// A planned row plus the data the install phase needs.
struct Planned {
    row: ApplyRow,
    file: Option<CompFile>,
    existing: Option<InstalledHit>,
    /// Ids of the presets that requested this row (ledger bookkeeping).
    preset_ids: Vec<String>,
    /// True when the row still has to be downloaded and placed on disk.
    install: bool,
    /// Project slug, used for the Chinese display name.
    slug: Option<String>,
}

/// Partial preset payload: the UI creates a preset by name only, so every field is optional and
/// missing ones fall back to empty/zero. Timestamps and ids are always assigned by the repository.
#[derive(Debug, Clone, Default, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PresetSaveInput {
    #[specta(optional)]
    pub id: String,
    #[specta(optional)]
    pub name: String,
    #[specta(optional)]
    pub description: String,
    #[specta(optional)]
    pub game_versions: Vec<String>,
    #[specta(optional)]
    pub loaders: Vec<String>,
    #[specta(optional)]
    pub items: Vec<PresetItem>,
    #[specta(optional)]
    pub created_at: u64,
    #[specta(optional)]
    pub updated_at: u64,
}

impl From<PresetSaveInput> for ResourcePreset {
    fn from(input: PresetSaveInput) -> Self {
        Self {
            id: input.id,
            name: input.name,
            description: input.description,
            game_versions: input.game_versions,
            loaders: input.loaders,
            items: input.items,
            created_at: input.created_at,
            updated_at: input.updated_at,
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn presets_list(
    state: State<'_, Arc<PresetState>>,
) -> Result<Vec<ResourcePreset>, AppError> {
    Ok(state.repo.lock().await.list())
}

#[tauri::command]
#[specta::specta]
pub async fn presets_save(
    state: State<'_, Arc<PresetState>>,
    preset: PresetSaveInput,
) -> Result<ResourcePreset, AppError> {
    state
        .repo
        .lock()
        .await
        .save(preset.into())
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn presets_delete(
    state: State<'_, Arc<PresetState>>,
    id: String,
) -> Result<(), AppError> {
    state.repo.lock().await.delete(&id).map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn presets_add_items(
    state: State<'_, SettingsStore>,
    presets: State<'_, Arc<PresetState>>,
    preset_id: String,
    items: Vec<PresetItem>,
) -> Result<Vec<AddItemOutcome>, AppError> {
    let preset = presets
        .repo
        .lock()
        .await
        .get(&preset_id)
        .ok_or_else(|| AppError::new("preset.notFound").with_param("id", preset_id.clone()))?;

    let settings = state.get().await;
    let modrinth = ModrinthClient::new(settings.net.source_priority);
    let curseforge = CurseForgeClient::new(settings.net.source_priority);

    let mut keys: Vec<FileKey> = Vec::with_capacity(items.len());
    for item in &items {
        // Unsupported kinds are reported by the repository without touching the network.
        if !kind_can_be_preset(item.platform, item.kind) {
            continue;
        }
        let key = (item.platform, item.project_id.clone());
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    let fetched: Vec<(FileKey, Result<Vec<CompFile>, CompError>)> = stream::iter(keys)
        .map(|key| {
            let modrinth = &modrinth;
            let curseforge = &curseforge;
            async move {
                let result = fetch_files(key.0, &key.1, modrinth, curseforge).await;
                (key, result)
            }
        })
        .buffer_unordered(6)
        .collect()
        .await;

    let mut files: HashMap<FileKey, Vec<CompFile>> = HashMap::with_capacity(fetched.len());
    for (key, result) in fetched {
        match result {
            Ok(list) => {
                files.insert(key, list);
            }
            Err(err) => {
                tracing::warn!(project = %key.1, error = %err, "failed to fetch files while adding preset items");
            }
        }
    }

    let outcomes = presets
        .repo
        .lock()
        .await
        .add_items(&preset_id, items, |item| {
            let Some(list) = files.get(&(item.platform, item.project_id.clone())) else {
                return Err(MatchFailure::NoFile);
            };
            resolve_item_file(item, list, &preset.game_versions, &preset.loaders)
        })?;

    Ok(outcomes)
}

#[tauri::command]
#[specta::specta]
pub async fn presets_remove_item(
    state: State<'_, Arc<PresetState>>,
    preset_id: String,
    platform: CompPlatform,
    project_id: String,
) -> Result<(), AppError> {
    state
        .repo
        .lock()
        .await
        .remove_item(&preset_id, platform, &project_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn presets_pin(
    state: State<'_, Arc<PresetState>>,
    preset_id: String,
    platform: CompPlatform,
    project_id: String,
    file_id: Option<String>,
    file_name: Option<String>,
) -> Result<(), AppError> {
    state
        .repo
        .lock()
        .await
        .pin(&preset_id, platform, &project_id, file_id, file_name)
        .map_err(AppError::from)
}
#[tauri::command]
#[specta::specta]
pub async fn presets_pin_rule(
    state: State<'_, Arc<PresetState>>,
    preset_id: String,
    platform: CompPlatform,
    project_id: String,
    rule: mclr_comp::preset::model::PresetPinRule,
) -> Result<(), AppError> {
    state
        .repo
        .lock()
        .await
        .pin_rule(&preset_id, platform, &project_id, rule)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn presets_remove_pin_rule(
    state: State<'_, Arc<PresetState>>,
    preset_id: String,
    platform: CompPlatform,
    project_id: String,
    file_id: String,
) -> Result<(), AppError> {
    state
        .repo
        .lock()
        .await
        .remove_pin_rule(&preset_id, platform, &project_id, &file_id)
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::too_many_arguments)]
pub async fn presets_apply(
    http: State<'_, crate::http::HttpState>,
    settings_state: State<'_, SettingsStore>,
    tasks: State<'_, Arc<TaskCenter>>,
    presets: State<'_, Arc<PresetState>>,
    request: ApplyRequest,
) -> Result<String, AppError> {
    let requested: Vec<ResourcePreset> = {
        let repo = presets.repo.lock().await;
        let mut list = Vec::with_capacity(request.preset_ids.len());
        for id in &request.preset_ids {
            match repo.get(id) {
                Some(preset) => list.push(preset),
                None => {
                    return Err(AppError::new("preset.notFound").with_param("id", id.clone()));
                }
            }
        }
        list
    };

    let settings = settings_state.get().await;
    let game_dir = super::game_dir_for(&settings_state, &request.instance_id).await;
    let mc_folder = super::current_mc_folder(&settings_state).await;
    let scan_id = request.instance_id.clone();
    let instance =
        tokio::task::spawn_blocking(move || mclr_game::instance::scan_instances(&mc_folder))
            .await
            .ok()
            .and_then(Result::ok)
            .and_then(|instances| {
                instances
                    .into_iter()
                    .find(|inst| inst.id == scan_id || inst.name == scan_id)
            });

    let mc_version = instance
        .as_ref()
        .map(|inst| normalize_mc_version(&inst.mc_version))
        .unwrap_or_default();
    let labels = instance.map(|inst| inst.loaders).unwrap_or_default();
    let ctx = InstanceCtx {
        loaders: expand_loaders(&normalize_loaders(&labels), &mc_version),
        mc_version,
    };

    // A datapack needs a world that actually exists; anything else is reported as `NeedsWorld`.
    let datapack_world = request
        .datapack_world
        .as_deref()
        .map(str::trim)
        .filter(|world| !world.is_empty())
        .filter(|world| {
            mclr_core::safe_path::safe_join(&game_dir, &format!("saves/{world}"))
                .is_some_and(|path| path.is_dir())
        })
        .map(str::to_string);

    let mut params = HashMap::new();
    params.insert("instance".to_string(), request.instance_id.clone());
    params.insert("count".to_string(), requested.len().to_string());
    let (task_id, control) = tasks
        .create_task(None, "tasks.titles.presetApply", params)
        .await;

    let tc = Arc::clone(&tasks);
    let state = Arc::clone(&presets);
    let downloader = Downloader::new(http.client(), settings.net.clone());
    let tid = task_id.clone();
    let instance_id = request.instance_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        // One flat stage: `finished / total` is the whole bar.
        let _ = tc.set_progress_window(&tid, 0.0, 1.0).await;
        let result = execute_apply(
            &tid,
            &tc,
            &control,
            &requested,
            &request,
            &settings,
            &game_dir,
            &ctx,
            datapack_world.as_deref(),
            &downloader,
        )
        .await;
        match result {
            Ok(report) => {
                state.reports.lock().await.insert(tid.clone(), report);
                let _ = tc.complete_task(&tid).await;
            }
            Err(err) => {
                tracing::warn!(instance = %instance_id, error = %err, "preset apply failed");
                let _ = tc.fail_task(&tid, err).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn presets_apply_report(
    state: State<'_, Arc<PresetState>>,
    task_id: String,
) -> Result<ApplyReport, AppError> {
    state
        .reports
        .lock()
        .await
        .remove(&task_id)
        .ok_or_else(|| AppError::new("preset.reportMissing").with_param("taskId", task_id))
}

#[tauri::command]
#[specta::specta]
pub async fn presets_cache(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks: State<'_, Arc<TaskCenter>>,
    presets: State<'_, Arc<PresetState>>,
    preset_id: String,
) -> Result<String, AppError> {
    let preset = presets
        .repo
        .lock()
        .await
        .get(&preset_id)
        .ok_or_else(|| AppError::new("preset.notFound").with_param("id", preset_id))?;

    let settings = state.get().await;
    let downloader = Downloader::new(http.client(), settings.net.clone());
    let store = Store::new(store_root(&settings));

    let mut params = HashMap::new();
    params.insert("name".to_string(), preset.name.clone());
    params.insert("count".to_string(), preset.items.len().to_string());
    let (task_id, control) = tasks
        .create_task(None, "tasks.titles.presetCache", params)
        .await;

    let tc = Arc::clone(&tasks);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        let modrinth = ModrinthClient::new(settings.net.source_priority);
        let curseforge = CurseForgeClient::new(settings.net.source_priority);
        let total = preset.items.len().max(1);

        for (index, item) in preset.items.iter().enumerate() {
            if control.checkpoint().await.is_err() {
                break;
            }
            match fetch_files(item.platform, &item.project_id, &modrinth, &curseforge).await {
                Ok(files) => {
                    let loaders: Vec<String> = if item.kind == CompType::Mod {
                        preset.loaders.clone()
                    } else {
                        Vec::new()
                    };
                    let resolved = match item.pinned_file_id.as_deref() {
                        Some(pin) => files.into_iter().find(|file| file.id == pin),
                        None => {
                            select_best_file_filtered(&files, &preset.game_versions, &loaders).ok()
                        }
                    };
                    match resolved {
                        Some(file) => {
                            if let Err(err) = store.ensure(&downloader, &file, item.kind).await {
                                tracing::warn!(file = %file.file_name, error = %err, "preset cache download failed");
                            }
                        }
                        None => {
                            tracing::info!(project = %item.project_id, "no preset file matched; skipped caching");
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(project = %item.project_id, error = %err, "preset cache file lookup failed");
                }
            }
            let _ = tc
                .update_progress(&tid, (index + 1) as f32 / total as f32, 0)
                .await;
        }

        let _ = tc.complete_task(&tid).await;
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn presets_store_info(
    state: State<'_, SettingsStore>,
    presets: State<'_, Arc<PresetState>>,
) -> Result<StoreInfo, AppError> {
    let settings = state.get().await;
    let pinned = presets.repo.lock().await.pinned_file_ids();
    let root = store_root(&settings);
    super::run_blocking(move || Store::new(root).info(&pinned)).await
}

#[tauri::command]
#[specta::specta]
pub async fn presets_store_set_dir(
    app: AppHandle,
    state: State<'_, SettingsStore>,
    presets: State<'_, Arc<PresetState>>,
    dir: String,
    migrate: bool,
) -> Result<StoreInfo, AppError> {
    let settings = state.get().await;
    let current = store_root(&settings);
    let trimmed = dir.trim().to_string();
    let target = if trimmed.is_empty() {
        mclr_core::paths::cache_dir().join("store")
    } else {
        PathBuf::from(&trimmed)
    };

    let target_for_probe = target.clone();
    let current_for_move = current.clone();
    super::run_blocking(move || -> Result<(), AppError> {
        crate::ipc::assets::reject_dangerous_dir(&target_for_probe)?;
        std::fs::create_dir_all(&target_for_probe)
            .map_err(|err| AppError::new("core.io").with_detail(err.to_string()))?;
        let probe = target_for_probe.join(format!(".mclr-probe-{}", std::process::id()));
        std::fs::write(&probe, b"probe")
            .map_err(|err| AppError::new("core.io").with_detail(err.to_string()))?;
        let _ = std::fs::remove_file(&probe);
        if migrate && current_for_move != target_for_probe {
            Store::new(current_for_move)
                .migrate_to(&target_for_probe)
                .map_err(AppError::from)?;
        }
        Ok(())
    })
    .await?;

    let patch = serde_json::json!({ "comp": { "storeDir": trimmed } });
    let updated = state.patch(&patch).await.map_err(AppError::from)?;
    crate::ipc::assets::allow_presets_store(&app, &store_root(&updated));
    let _ = app.emit("settings://changed", &updated);

    let pinned = presets.repo.lock().await.pinned_file_ids();
    let root = store_root(&updated);
    super::run_blocking(move || Store::new(root).info(&pinned)).await
}

#[tauri::command]
#[specta::specta]
pub async fn presets_store_prune(
    state: State<'_, SettingsStore>,
    presets: State<'_, Arc<PresetState>>,
) -> Result<PruneResult, AppError> {
    let settings = state.get().await;
    let pinned = presets.repo.lock().await.pinned_file_ids();
    let root = store_root(&settings);
    super::run_blocking(move || Store::new(root).prune(&pinned)).await
}

#[tauri::command]
#[specta::specta]
pub async fn presets_store_remove(
    state: State<'_, SettingsStore>,
    sha1: String,
) -> Result<(), AppError> {
    let settings = state.get().await;
    let root = store_root(&settings);
    super::run_blocking(move || Store::new(root).remove(&sha1)).await
}

/// Runs a preset application and returns the report handed to the UI.
#[allow(clippy::too_many_arguments)]
async fn execute_apply(
    task_id: &str,
    tc: &TaskCenter,
    control: &TaskControl,
    requested: &[ResourcePreset],
    request: &ApplyRequest,
    settings: &Settings,
    game_dir: &Path,
    ctx: &InstanceCtx,
    datapack_world: Option<&str>,
    downloader: &Downloader,
) -> Result<ApplyReport, AppError> {
    let (gates, merged) = merge_presets(requested, ctx);

    let mut kinds: HashSet<CompType> = merged.iter().map(|item| item.item.kind).collect();
    if request.include_dependencies {
        kinds.insert(CompType::Mod);
    }

    let modrinth = ModrinthClient::new(settings.net.source_priority);
    let curseforge = CurseForgeClient::new(settings.net.source_priority);

    let mut keys: Vec<FileKey> = Vec::with_capacity(merged.len());
    for item in &merged {
        let key = (item.item.platform, item.item.project_id.clone());
        if !keys.contains(&key) {
            keys.push(key);
        }
    }

    let fetched: Vec<(FileKey, Result<Vec<CompFile>, AppError>)> = stream::iter(keys)
        .map(|key| {
            let modrinth = &modrinth;
            let curseforge = &curseforge;
            async move {
                let result = fetch_files(key.0, &key.1, modrinth, curseforge)
                    .await
                    .map_err(AppError::from);
                (key, result)
            }
        })
        .buffer_unordered(6)
        .collect()
        .await;
    let files: FileResults = fetched.into_iter().collect();

    let installed = build_installed_index(game_dir, &kinds, datapack_world, &modrinth).await;

    let name_to_id: HashMap<&str, &str> = requested
        .iter()
        .map(|preset| (preset.name.as_str(), preset.id.as_str()))
        .collect();

    let mut planned: Vec<Planned> = Vec::with_capacity(merged.len());
    for merged_item in &merged {
        let key = (
            merged_item.item.platform,
            merged_item.item.project_id.clone(),
        );
        let preset_ids: Vec<String> = merged_item
            .from_presets
            .iter()
            .filter_map(|name| name_to_id.get(name.as_str()).map(|id| (*id).to_string()))
            .collect();
        match files.get(&key) {
            Some(Ok(list)) => {
                let decision = decide(
                    merged_item,
                    list,
                    ctx,
                    &installed,
                    request.on_other_version,
                    datapack_world.is_some(),
                );
                planned.push(planned_row(merged_item, decision, preset_ids, settings));
            }
            Some(Err(err)) => {
                planned.push(failed_row(merged_item, &err.code, preset_ids, settings));
            }
            None => planned.push(failed_row(
                merged_item,
                "comp.fileNotFound",
                preset_ids,
                settings,
            )),
        }
    }

    if request.include_dependencies {
        append_dependency_rows(
            &mut planned,
            game_dir,
            ctx,
            settings,
            &modrinth,
            &curseforge,
        )
        .await;
    }

    mark_overlaps(&mut planned);
    record_already_same(&planned, game_dir).await;

    // Rows that need no work are final as-is.
    let total = planned.iter().filter(|item| item.install).count();
    let done = Arc::new(AtomicUsize::new(0));
    let store = Store::new(store_root(settings));

    let mut finished: Vec<(usize, ApplyRow)> = Vec::with_capacity(planned.len());
    let mut jobs: Vec<(usize, Planned)> = Vec::new();
    for (index, item) in planned.into_iter().enumerate() {
        if item.install {
            jobs.push((index, item));
        } else {
            finished.push((index, item.row));
        }
    }

    let installed_rows: Vec<Option<(usize, ApplyRow)>> =
        stream::iter(jobs.into_iter().map(|(index, item)| {
            let store = store.clone();
            let game_dir = game_dir.to_path_buf();
            let world = datapack_world.map(str::to_string);
            let control = control.clone();
            let tid = task_id.to_string();
            let done = Arc::clone(&done);
            async move {
                if control.checkpoint().await.is_err() {
                    return None;
                }
                let Planned {
                    row,
                    file,
                    existing,
                    preset_ids,
                    slug,
                    ..
                } = item;
                let row = match file {
                    Some(file) => {
                        install_row(
                            row,
                            &file,
                            existing.as_ref(),
                            &preset_ids,
                            slug.as_deref(),
                            &game_dir,
                            world.as_deref(),
                            &store,
                            downloader,
                            settings,
                        )
                        .await
                    }
                    None => {
                        let mut row = row;
                        mark_failed(&mut row, "comp.fileNotFound");
                        row
                    }
                };
                let processed = done.fetch_add(1, Ordering::Relaxed) + 1;
                if total > 0 {
                    let _ = tc
                        .update_progress(&tid, (processed as f32 / total as f32).clamp(0.0, 1.0), 0)
                        .await;
                }
                Some((index, row))
            }
        }))
        .buffer_unordered(4)
        .collect()
        .await;

    finished.extend(installed_rows.into_iter().flatten());
    finished.sort_by_key(|(index, _)| *index);

    Ok(ApplyReport {
        instance_id: request.instance_id.clone(),
        gates,
        rows: finished.into_iter().map(|(_, row)| row).collect(),
    })
}

/// Resolves missing required dependencies of every planned mod row.
async fn append_dependency_rows(
    planned: &mut Vec<Planned>,
    game_dir: &Path,
    ctx: &InstanceCtx,
    settings: &Settings,
    modrinth: &ModrinthClient,
    curseforge: &CurseForgeClient,
) {
    let mods_dir = game_dir.join("mods");
    let mods = super::run_blocking(move || mclr_comp::scan_local_mods(&mods_dir))
        .await
        .unwrap_or_default();

    let mut installed_project_ids: HashSet<String> = HashSet::new();
    let mut installed_mod_ids: HashSet<String> = HashSet::new();
    for info in mods {
        installed_mod_ids.insert(info.mod_id.to_ascii_lowercase());
        if let Some(project_id) = info.project_id {
            installed_project_ids.insert(project_id.to_ascii_lowercase());
        }
    }
    for row in planned.iter() {
        installed_project_ids.insert(row.row.project_id.to_ascii_lowercase());
    }

    let loader = ctx.loaders.first().cloned().unwrap_or_default();
    let mut known: HashSet<FileKey> = planned
        .iter()
        .map(|item| (item.row.platform, item.row.project_id.clone()))
        .collect();

    let parents: Vec<(String, CompFile)> = planned
        .iter()
        .filter(|item| item.install && item.row.kind == CompType::Mod)
        .filter_map(|item| item.file.clone().map(|file| (item.row.title.clone(), file)))
        .collect();

    let mut extra: Vec<Planned> = Vec::new();
    for (parent_title, file) in parents {
        let resolved = resolve_dependencies(
            &file,
            &ctx.mc_version,
            &loader,
            &installed_project_ids,
            &installed_mod_ids,
            modrinth,
            Some(curseforge),
        )
        .await;
        match resolved {
            Ok(items) => {
                for item in items {
                    if item.installed {
                        continue;
                    }
                    let key = (item.project.platform, item.project.id.clone());
                    if !known.insert(key) {
                        continue;
                    }
                    extra.push(dependency_row(&item, &parent_title, settings));
                }
            }
            Err(err) => {
                tracing::warn!(parent = %parent_title, error = %err, "dependency resolution failed");
            }
        }
    }
    planned.extend(extra);
}

/// Marks rows whose chosen file content repeats an earlier row and stops them installing again.
fn mark_overlaps(planned: &mut [Planned]) {
    let mut seen: HashMap<String, (ApplyStatus, usize)> = HashMap::new();
    for (index, item) in planned.iter_mut().enumerate() {
        let Some(file) = &item.file else {
            continue;
        };
        let Some(sha1) = file
            .hashes
            .sha1
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        match seen.get(&sha1) {
            Some((status, first)) if *first != index => {
                item.row.overlap = true;
                item.row.status = *status;
                item.install = false;
            }
            Some(_) => {}
            None => {
                seen.insert(sha1, (item.row.status, index));
            }
        }
    }
}

/// Records `AlreadySame` rows in the instance ledger (they were installed outside the store).
async fn record_already_same(planned: &[Planned], game_dir: &Path) {
    for item in planned
        .iter()
        .filter(|item| item.row.status == ApplyStatus::AlreadySame)
    {
        let (Some(file), Some(existing)) = (item.file.as_ref(), item.existing.as_ref()) else {
            continue;
        };
        let mode = if item.row.kind == CompType::World {
            LinkMode::Extracted
        } else {
            LinkMode::Copy
        };
        record_ledger(
            &item.row,
            file,
            &existing.rel_path,
            mode,
            &item.preset_ids,
            game_dir,
        )
        .await;
    }
}

/// Downloads (or reuses) one file and places it into the instance.
#[allow(clippy::too_many_arguments)]
async fn install_row(
    mut row: ApplyRow,
    file: &CompFile,
    existing: Option<&InstalledHit>,
    preset_ids: &[String],
    slug: Option<&str>,
    game_dir: &Path,
    datapack_world: Option<&str>,
    store: &Store,
    downloader: &Downloader,
    settings: &Settings,
) -> ApplyRow {
    let kind = row.kind;
    let zh = zh_name(slug, settings);
    let formatted = naming::format_download_name(
        &file.file_name,
        zh.as_deref(),
        kind,
        settings.comp.filename_format,
    );

    let object = match store.ensure(downloader, file, kind).await {
        Ok(object) => object,
        Err(err) => {
            mark_failed(&mut row, err.code());
            return row;
        }
    };

    if kind == CompType::World {
        let saves_dir = game_dir.join("saves");
        let object_path = object.path.clone();
        match super::run_blocking(move || mclr_saves::import_save_zip(&saves_dir, &object_path))
            .await
        {
            Ok(info) => {
                let rel_path = format!("saves/{}", info.folder_name);
                row.link_mode = Some(LinkMode::Extracted);
                row.file_name = Some(info.folder_name);
                record_ledger(
                    &row,
                    file,
                    &rel_path,
                    LinkMode::Extracted,
                    preset_ids,
                    game_dir,
                )
                .await;
            }
            Err(err) => mark_failed(&mut row, &err.code),
        }
        return row;
    }

    let rel_dir = match kind {
        CompType::Mod => "mods".to_string(),
        CompType::Resourcepack => "resourcepacks".to_string(),
        CompType::Shader => "shaderpacks".to_string(),
        CompType::Datapack => match datapack_world {
            Some(world) => format!("saves/{world}/datapacks"),
            None => {
                row.status = ApplyStatus::NeedsWorld;
                return row;
            }
        },
        CompType::Modpack | CompType::World => {
            mark_failed(&mut row, "preset.kindUnsupported");
            return row;
        }
    };

    let rel_path = format!("{rel_dir}/{formatted}");
    let Some(dest) = mclr_core::safe_path::safe_join(game_dir, &rel_path) else {
        mark_failed(&mut row, "preset.pathInvalid");
        return row;
    };

    match store.place(
        &object,
        &dest,
        settings.comp.use_hardlink,
        game_dir,
        &rel_path,
    ) {
        Ok(mode) => row.link_mode = Some(mode),
        Err(err) => {
            mark_failed(&mut row, err.code());
            return row;
        }
    }

    if row.status == ApplyStatus::Replaced
        && let Some(old) = existing
        && let Some(old_path) = mclr_core::safe_path::safe_join(game_dir, &old.rel_path)
        && old_path != dest
    {
        let _ = super::run_blocking(move || delete_resources(&[old_path], false)).await;
        let old_rel = old.rel_path.clone();
        let game = game_dir.to_path_buf();
        let _ = super::run_blocking(move || ledger::remove_by_rel_path(&game, &old_rel)).await;
    }

    let mode = row.link_mode.unwrap_or(LinkMode::Copy);
    row.file_name = Some(formatted);
    record_ledger(&row, file, &rel_path, mode, preset_ids, game_dir).await;
    row
}

fn mark_failed(row: &mut ApplyRow, code: &str) {
    row.status = ApplyStatus::Failed;
    row.message_code = Some(code.to_string());
}

async fn record_ledger(
    row: &ApplyRow,
    file: &CompFile,
    rel_path: &str,
    mode: LinkMode,
    preset_ids: &[String],
    game_dir: &Path,
) {
    let entry = LedgerEntry {
        platform: row.platform,
        project_id: row.project_id.clone(),
        file_id: file.id.clone(),
        sha1: file
            .hashes
            .sha1
            .clone()
            .unwrap_or_default()
            .to_ascii_lowercase(),
        kind: row.kind,
        rel_path: rel_path.to_string(),
        preset_ids: preset_ids.to_vec(),
        link_mode: mode,
        applied_at: ledger::now_ms(),
    };
    let game = game_dir.to_path_buf();
    if let Err(err) = super::run_blocking(move || ledger::upsert(&game, entry)).await {
        tracing::warn!(rel = rel_path, error = %err, "failed to record preset ledger entry");
    }
}

fn planned_row(
    merged: &MergedItem,
    decision: Decision,
    preset_ids: Vec<String>,
    settings: &Settings,
) -> Planned {
    let Decision {
        status,
        file,
        existing,
    } = decision;
    let install =
        matches!(status, ApplyStatus::Installed | ApplyStatus::Replaced) && file.is_some();
    Planned {
        row: ApplyRow {
            platform: merged.item.platform,
            project_id: merged.item.project_id.clone(),
            kind: merged.item.kind,
            title: merged.item.title.clone(),
            zh_title: zh_name(merged.item.slug.as_deref(), settings),
            status,
            file_id: file.as_ref().map(|chosen| chosen.id.clone()),
            file_name: file.as_ref().map(|chosen| chosen.file_name.clone()),
            installed_file_name: existing.as_ref().map(|hit| hit.file_name.clone()),
            link_mode: None,
            from_presets: merged.from_presets.clone(),
            overlap: merged.from_presets.len() > 1,
            version_conflict: merged.version_conflict,
            via_dependency_of: None,
            message_code: None,
        },
        file,
        existing,
        preset_ids,
        install,
        slug: merged.item.slug.clone(),
    }
}

fn failed_row(
    merged: &MergedItem,
    code: &str,
    preset_ids: Vec<String>,
    settings: &Settings,
) -> Planned {
    Planned {
        row: ApplyRow {
            platform: merged.item.platform,
            project_id: merged.item.project_id.clone(),
            kind: merged.item.kind,
            title: merged.item.title.clone(),
            zh_title: zh_name(merged.item.slug.as_deref(), settings),
            status: ApplyStatus::Failed,
            file_id: None,
            file_name: None,
            installed_file_name: None,
            link_mode: None,
            from_presets: merged.from_presets.clone(),
            overlap: merged.from_presets.len() > 1,
            version_conflict: merged.version_conflict,
            via_dependency_of: None,
            message_code: Some(code.to_string()),
        },
        file: None,
        existing: None,
        preset_ids,
        install: false,
        slug: merged.item.slug.clone(),
    }
}

fn dependency_row(item: &ResolvedDependencyItem, parent: &str, settings: &Settings) -> Planned {
    let project = &item.project;
    let file = item.file.clone();
    let zh = project
        .zh_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| zh_name(Some(&project.slug), settings));
    Planned {
        row: ApplyRow {
            platform: project.platform,
            project_id: project.id.clone(),
            kind: CompType::Mod,
            title: project.title.clone(),
            zh_title: zh,
            status: ApplyStatus::Installed,
            file_id: Some(file.id.clone()),
            file_name: Some(file.file_name.clone()),
            installed_file_name: None,
            link_mode: None,
            from_presets: Vec::new(),
            overlap: false,
            version_conflict: false,
            via_dependency_of: Some(parent.to_string()),
            message_code: None,
        },
        file: Some(file),
        existing: None,
        preset_ids: Vec::new(),
        install: true,
        slug: Some(project.slug.clone()),
    }
}

/// Chinese display name of a project slug when the setting enables it.
fn zh_name(slug: Option<&str>, settings: &Settings) -> Option<String> {
    if !settings.comp.show_zh_name {
        return None;
    }
    slug.and_then(modname::lookup_by_slug)
        .map(|entry| modname::display_zh(entry).to_string())
}

/// Kinds a preset can hold: modpacks are not resources, and Modrinth has no world type.
fn kind_can_be_preset(platform: CompPlatform, kind: CompType) -> bool {
    kind != CompType::Modpack && !(kind == CompType::World && platform == CompPlatform::Modrinth)
}

/// Resolves one preset item against the preset filters, honouring an explicit pin.
fn resolve_item_file(
    item: &PresetItem,
    files: &[CompFile],
    game_versions: &[String],
    preset_loaders: &[String],
) -> Result<Option<CompFile>, MatchFailure> {
    let loaders: Vec<String> = if item.kind == CompType::Mod {
        preset_loaders.to_vec()
    } else {
        Vec::new()
    };

    if let Some(pin) = item.pinned_file_id.as_deref() {
        let Some(file) = files.iter().find(|file| file.id == pin) else {
            return Err(MatchFailure::NoFile);
        };
        if !game_versions.is_empty()
            && !game_versions
                .iter()
                .any(|version| file_supports_mc(file, version))
        {
            return Err(MatchFailure::NoGameVersion);
        }
        if !loaders.is_empty() {
            let instance_loaders: Vec<&str> = loaders.iter().map(String::as_str).collect();
            if !file_supports_any_loader(&file.loaders, &instance_loaders) {
                return Err(MatchFailure::NoLoader);
            }
        }
        return Ok(Some(file.clone()));
    }

    select_best_file_filtered(files, game_versions, &loaders).map(Some)
}

async fn fetch_files(
    platform: CompPlatform,
    project_id: &str,
    modrinth: &ModrinthClient,
    curseforge: &CurseForgeClient,
) -> Result<Vec<CompFile>, CompError> {
    match platform {
        CompPlatform::Modrinth => modrinth.get_versions(project_id, None, None).await,
        CompPlatform::Curseforge => curseforge.get_files(project_id, None, None).await,
    }
}

/// Instance metadata reports `Unknown` when detection failed; that must not gate every preset.
fn normalize_mc_version(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("unknown") {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// Maps instance loader labels to canonical ids while keeping flavours `canonical_loader_id`
/// deliberately drops (Cleanroom, Legacy Fabric) so [`expand_loaders`] can map them later.
fn normalize_loaders(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter_map(|label| {
            let lower = label.trim().to_ascii_lowercase();
            if lower.is_empty() {
                return None;
            }
            Some(
                mclr_game::instance::canonical_loader_id(label)
                    .map(str::to_string)
                    .unwrap_or(lower),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclr_comp::CompFileHashes;
    use mclr_comp::preset::installed::InstalledIndex;

    fn sample_file(id: &str, sha1: Option<&str>) -> CompFile {
        CompFile {
            id: id.to_string(),
            project_id: "proj".to_string(),
            platform: CompPlatform::Modrinth,
            name: format!("File {id}"),
            file_name: format!("{id}.jar"),
            mc_versions: vec!["1.20.1".to_string()],
            loaders: vec!["fabric".to_string()],
            channel: mclr_comp::CompReleaseChannel::Release,
            release_date: "2024-01-01T00:00:00Z".to_string(),
            size: 10,
            hashes: CompFileHashes {
                sha1: sha1.map(str::to_string),
                sha512: None,
                murmur2: None,
            },
            download_urls: vec!["https://example.com/file.jar".to_string()],
            distribution_disabled: false,
            dependencies: Vec::new(),
        }
    }

    fn merged_item() -> MergedItem {
        MergedItem {
            item: PresetItem {
                platform: CompPlatform::Modrinth,
                project_id: "proj".to_string(),
                kind: CompType::Mod,
                title: "Example".to_string(),
                slug: Some("example".to_string()),
                icon_url: None,
                pinned_file_id: None,
                pinned_file_name: None,
                pin_rules: Vec::new(),
            },
            from_presets: vec!["A".to_string()],
            version_conflict: false,
            preset_loaders: None,
        }
    }

    fn fabric_ctx() -> InstanceCtx {
        InstanceCtx {
            mc_version: "1.20.1".to_string(),
            loaders: vec!["fabric".to_string()],
        }
    }

    #[test]
    fn store_root_defaults_under_cache_and_honours_custom_dir() {
        let mut settings = Settings::default();
        assert_eq!(
            store_root(&settings),
            mclr_core::paths::cache_dir().join("store")
        );

        settings.comp.store_dir = "  D:/MCLR/store  ".to_string();
        assert_eq!(store_root(&settings), PathBuf::from("D:/MCLR/store"));
    }

    #[test]
    fn plan_to_report_marks_existing_content_as_already_same() {
        let settings = Settings::default();
        let file = sample_file("abc", Some("deadbeef"));
        let hit = InstalledHit {
            file_name: "abc.jar".to_string(),
            rel_path: "mods/abc.jar".to_string(),
            sha1: "deadbeef".to_string(),
            file_id: Some("abc".to_string()),
        };
        let mut index = InstalledIndex::default();
        index.by_sha1.insert("deadbeef".to_string(), hit.clone());
        index
            .by_project
            .insert((CompPlatform::Modrinth, "proj".to_string()), vec![hit]);

        let item = merged_item();
        let decision = decide(
            &item,
            std::slice::from_ref(&file),
            &fabric_ctx(),
            &index,
            mclr_comp::preset::model::OtherVersionPolicy::Skip,
            true,
        );
        let planned = planned_row(&item, decision, vec!["id-a".to_string()], &settings);

        assert_eq!(planned.row.status, ApplyStatus::AlreadySame);
        assert_eq!(planned.row.installed_file_name.as_deref(), Some("abc.jar"));
        assert!(!planned.install);
    }

    #[test]
    fn overlap_marks_the_second_row_and_skips_its_install() {
        let settings = Settings::default();
        let file = sample_file("abc", Some("deadbeef"));
        let index = InstalledIndex::default();
        let ctx = fabric_ctx();

        let first_item = merged_item();
        let first = decide(
            &first_item,
            std::slice::from_ref(&file),
            &ctx,
            &index,
            mclr_comp::preset::model::OtherVersionPolicy::Skip,
            true,
        );
        let mut second_item = merged_item();
        second_item.item.project_id = "proj2".to_string();
        second_item.from_presets = vec!["A".to_string(), "B".to_string()];
        let second = decide(
            &second_item,
            std::slice::from_ref(&file),
            &ctx,
            &index,
            mclr_comp::preset::model::OtherVersionPolicy::Skip,
            true,
        );

        let mut planned = vec![
            planned_row(&first_item, first, Vec::new(), &settings),
            planned_row(&second_item, second, Vec::new(), &settings),
        ];
        mark_overlaps(&mut planned);

        assert!(!planned[0].row.overlap);
        assert!(planned[0].install);
        assert!(planned[1].row.overlap);
        assert_eq!(planned[1].row.status, ApplyStatus::Installed);
        assert!(!planned[1].install);
    }

    #[test]
    fn preset_item_resolution_reports_missing_filters() {
        let item = PresetItem {
            platform: CompPlatform::Modrinth,
            project_id: "proj".to_string(),
            kind: CompType::Mod,
            title: "Example".to_string(),
            slug: None,
            icon_url: None,
            pinned_file_id: None,
            pinned_file_name: None,
            pin_rules: Vec::new(),
        };
        let files = vec![sample_file("abc", None)];
        let game_versions = vec!["1.19.2".to_string()];
        let loaders = vec!["fabric".to_string()];
        let result = resolve_item_file(&item, &files, &game_versions, &loaders);
        assert_eq!(result.unwrap_err(), MatchFailure::NoGameVersion);

        // A pinned file that is absent from the project's file list is a hard "no file".
        let mut pinned = item;
        pinned.pinned_file_id = Some("missing".to_string());
        let result = resolve_item_file(&pinned, &files, &[], &loaders);
        assert_eq!(result.unwrap_err(), MatchFailure::NoFile);
    }
}
