//! Task handlers for resource presets application and caching.

use mclr_comp::curseforge::CurseForgeClient;
use mclr_comp::modrinth::ModrinthClient;
use mclr_comp::preset::model::ApplyRequest;
use mclr_comp::preset::store::Store;
use mclr_comp::resolver::select_best_file_filtered;
use mclr_comp::{CompFile, CompType};
use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_net::downloader::Downloader;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::{AppHandle, Manager};

use crate::ipc::presets::{PresetState, execute_apply, fetch_files, store_root};

pub struct PresetsApplyHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsApplyPayload {
    pub request: ApplyRequest,
}

#[async_trait::async_trait]
impl TaskHandler for PresetsApplyHandler {
    fn kind(&self) -> &'static str {
        "presets.apply"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: PresetsApplyPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("instance".to_string(), p.request.instance_id.clone());
        params.insert("count".to_string(), p.request.preset_ids.len().to_string());
        Ok(TaskPlan {
            title_key: "tasks.titles.presetApply".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "resolve",
                    weight: 15.0,
                },
                StepPlan {
                    key: "scan",
                    weight: 5.0,
                },
                StepPlan {
                    key: "plan",
                    weight: 5.0,
                },
                StepPlan {
                    key: "install",
                    weight: 75.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: PresetsApplyPayload = ctx.payload()?;
        let presets_state = self.app.state::<std::sync::Arc<PresetState>>();
        let settings_state = self.app.state::<SettingsStore>();
        let settings = settings_state.get().await;
        let http = self.app.state::<crate::http::HttpState>();

        let requested = {
            let repo = presets_state.repo.lock().await;
            let mut list = Vec::with_capacity(p.request.preset_ids.len());
            for id in &p.request.preset_ids {
                match repo.get(id) {
                    Some(preset) => list.push(preset),
                    None => {
                        return Err(AppError::new("preset.notFound").with_param("id", id.clone()));
                    }
                }
            }
            list
        };

        let game_dir = crate::ipc::game_dir_for(&settings_state, &p.request.instance_id).await;
        let mc_folder = crate::ipc::current_mc_folder(&settings_state).await;
        let scan_id = p.request.instance_id.clone();
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
            .map(|inst| {
                mclr_core::mcver::known(&inst.mc_version)
                    .unwrap_or("")
                    .to_string()
            })
            .unwrap_or_default();
        let labels = instance.map(|inst| inst.loaders).unwrap_or_default();
        let inst_ctx = mclr_comp::preset::plan::InstanceCtx {
            loaders: mclr_core::loader::expand_mod_loaders(
                &mclr_core::loader::canonical_ids(&labels),
                &mc_version,
            ),
            mc_version,
        };

        let datapack_world = p
            .request
            .datapack_world
            .as_deref()
            .map(str::trim)
            .filter(|world| !world.is_empty())
            .filter(|world| {
                mclr_core::safe_path::safe_join(&game_dir, &format!("saves/{world}"))
                    .is_some_and(|path| path.is_dir())
            })
            .map(str::to_string);

        let downloader = Downloader::new(http.client(), settings.net.clone());

        let report = execute_apply(
            ctx,
            &requested,
            &p.request,
            &settings,
            &game_dir,
            &inst_ctx,
            datapack_world.as_deref(),
            &downloader,
        )
        .await?;

        presets_state
            .reports
            .lock()
            .await
            .insert(ctx.id.clone(), report);
        Ok(())
    }
}

pub struct PresetsCacheHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct PresetsCachePayload {
    pub preset_id: String,
    pub name: String,
    pub count: usize,
}

#[async_trait::async_trait]
impl TaskHandler for PresetsCacheHandler {
    fn kind(&self) -> &'static str {
        "presets.cache"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: PresetsCachePayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("name".to_string(), p.name);
        params.insert("count".to_string(), p.count.to_string());
        Ok(TaskPlan {
            title_key: "tasks.titles.presetCache".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "resolve",
                    weight: 20.0,
                },
                StepPlan {
                    key: "download",
                    weight: 80.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: PresetsCachePayload = ctx.payload()?;
        let presets_state = self.app.state::<std::sync::Arc<PresetState>>();
        let preset = {
            let repo = presets_state.repo.lock().await;
            repo.get(&p.preset_id).ok_or_else(|| {
                AppError::new("preset.notFound").with_param("id", p.preset_id.clone())
            })?
        };
        let settings_state = self.app.state::<SettingsStore>();
        let settings = settings_state.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let downloader = Downloader::new(http.client(), settings.net.clone());
        let store = Store::new(store_root(&settings));

        let modrinth = ModrinthClient::new(settings.net.source_priority);
        let curseforge = CurseForgeClient::new(settings.net.source_priority);

        ctx.begin_step("resolve").await;
        let mut resolved_files: Vec<(CompFile, CompType)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for item in &preset.items {
            if ctx.control().checkpoint().await.is_err() {
                return Err(AppError::new("task.canceled"));
            }
            let files = match fetch_files(item.platform, &item.project_id, &modrinth, &curseforge)
                .await
            {
                Ok(files) => files,
                Err(err) => {
                    tracing::warn!(project = %item.project_id, error = %err, "preset cache file lookup failed");
                    continue;
                }
            };

            let loaders: Vec<String> = if item.kind == CompType::Mod {
                preset.loaders.clone()
            } else {
                Vec::new()
            };
            let mut picked: Vec<CompFile> = Vec::new();
            let mut need_latest = false;
            if item.pin_rules.is_empty() {
                match item.pinned_file_id.as_deref() {
                    Some(pin) => {
                        if let Some(file) = files.iter().find(|file| file.id == pin) {
                            picked.push(file.clone());
                        }
                    }
                    None => need_latest = true,
                }
            } else {
                // Every context rule pins its own build; cache all of them, and fall back to the
                // latest compatible build when a rule's file is gone.
                for rule in &item.pin_rules {
                    match files.iter().find(|file| file.id == rule.file_id) {
                        Some(file) => picked.push(file.clone()),
                        None => need_latest = true,
                    }
                }
            }
            if need_latest
                && let Ok(file) = select_best_file_filtered(&files, &preset.game_versions, &loaders)
            {
                picked.push(file);
            }

            for file in picked {
                if seen.insert(file.id.clone()) {
                    resolved_files.push((file, item.kind));
                }
            }
        }
        ctx.finish_step("resolve").await;

        let total = resolved_files.len().max(1);
        ctx.begin_step("download").await;
        ctx.set_items(0, total as u32).await;
        for (index, (file, kind)) in resolved_files.into_iter().enumerate() {
            if ctx.control().checkpoint().await.is_err() {
                return Err(AppError::new("task.canceled"));
            }
            if let Err(err) = store.ensure(&downloader, &file, kind).await {
                tracing::warn!(file = %file.file_name, error = %err, "preset cache download failed");
            }
            let done = (index + 1) as u32;
            ctx.set_items(done, total as u32).await;
            ctx.progress(done as f32 / total as f32, 0).await;
        }
        ctx.finish_step("download").await;

        Ok(())
    }
}
