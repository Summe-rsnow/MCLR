//! Task handlers for single and batch local mod updates.

use futures_util::stream::{self, StreamExt};
use mclr_comp::modname;
use mclr_comp::naming;
use mclr_comp::{CompFile, CompType};
use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_net::downloader::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::{AppHandle, Manager};
use url::Url;

use crate::ipc::localmods::LocalModUpdateItem;
use mclr_comp::delete_local_mod;

pub struct LocalModUpdateHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct LocalModUpdatePayload {
    pub instance_id: String,
    pub new_file: CompFile,
    pub old_file_name: String,
}

#[async_trait::async_trait]
impl TaskHandler for LocalModUpdateHandler {
    fn kind(&self) -> &'static str {
        "localmods.update"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: LocalModUpdatePayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("name".to_string(), p.new_file.name.clone());
        params.insert("fileName".to_string(), p.new_file.file_name.clone());
        params.insert("targetInstance".to_string(), p.instance_id);
        Ok(TaskPlan {
            title_key: "tasks.titles.compDownload".into(),
            title_params: params,
            steps: vec![StepPlan {
                key: "download",
                weight: 1.0,
            }],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: LocalModUpdatePayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();

        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;
        let mods_dir = mc_folder.join("versions").join(&p.instance_id).join("mods");
        let _ = tokio::fs::create_dir_all(&mods_dir).await;

        let urls: Vec<Url> = p
            .new_file
            .download_urls
            .iter()
            .filter_map(|u| Url::parse(u).ok())
            .collect();
        if urls.is_empty() {
            return Err(AppError::new("comp.download.invalidUrls"));
        }

        let zh_name = if settings.comp.show_zh_name {
            let scan_dir = mods_dir.clone();
            let current = p.old_file_name.clone();
            tokio::task::spawn_blocking(move || {
                let jar_path = scan_dir.join(&current);
                let meta = mclr_comp::local::parse_mod_metadata_path(&jar_path, "")?;
                let entry = modname::lookup_by_slug(&meta.mod_id)?;
                Some(modname::display_zh(entry).to_string())
            })
            .await
            .unwrap_or_default()
        } else {
            None
        };

        let dest_name = naming::format_download_name(
            &p.new_file.file_name,
            zh_name.as_deref(),
            CompType::Mod,
            settings.comp.filename_format,
        );
        let dest_path = mods_dir.join(&dest_name);

        let spec = DownloadSpec {
            urls,
            dest: dest_path,
            size: if p.new_file.size > 0 {
                Some(p.new_file.size)
            } else {
                None
            },
            sha1: p.new_file.hashes.sha1,
            sha256: None,
            md5: None,
        };

        let downloader = Downloader::new(client, settings.net);
        downloader
            .download(spec, ctx.tc())
            .await
            .map_err(AppError::from)?;

        if !p.old_file_name.is_empty() && p.old_file_name != dest_name {
            let old_path = mods_dir.join(&p.old_file_name);
            if old_path.exists() {
                let _ = delete_local_mod(&mods_dir, &p.old_file_name);
            }
        }

        Ok(())
    }
}

pub struct LocalModsUpdateManyHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct LocalModsUpdateManyPayload {
    pub instance_id: String,
    pub updates: Vec<LocalModUpdateItem>,
}

#[async_trait::async_trait]
impl TaskHandler for LocalModsUpdateManyHandler {
    fn kind(&self) -> &'static str {
        "localmods.updateMany"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: LocalModsUpdateManyPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("targetInstance".to_string(), p.instance_id);
        params.insert("count".to_string(), p.updates.len().to_string());
        Ok(TaskPlan {
            title_key: "tasks.titles.compDownload".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "resolve",
                    weight: 5.0,
                },
                StepPlan {
                    key: "download",
                    weight: 95.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: LocalModsUpdateManyPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();

        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;
        let mods_dir = mc_folder.join("versions").join(&p.instance_id).join("mods");
        let _ = tokio::fs::create_dir_all(&mods_dir).await;

        let concurrency = settings.net.max_concurrent_files.clamp(1, 16) as usize;
        let downloader = Arc::new(Downloader::new(client, settings.net.clone()));

        ctx.begin_step("resolve").await;
        let filename_format = settings.comp.filename_format;
        let zh_by_file: Arc<HashMap<String, String>> = if settings.comp.show_zh_name {
            let names: Vec<String> = p
                .updates
                .iter()
                .map(|u| u.current_file_name.clone())
                .filter(|n| !n.is_empty())
                .collect();
            let scan_dir = mods_dir.clone();
            Arc::new(
                tokio::task::spawn_blocking(move || {
                    let mut map = HashMap::new();
                    for name in names {
                        let jar_path = scan_dir.join(&name);
                        let Some(meta) = mclr_comp::local::parse_mod_metadata_path(&jar_path, "")
                        else {
                            continue;
                        };
                        if let Some(entry) = modname::lookup_by_slug(&meta.mod_id) {
                            map.insert(name, modname::display_zh(entry).to_string());
                        }
                    }
                    map
                })
                .await
                .unwrap_or_default(),
            )
        } else {
            Arc::new(HashMap::new())
        };
        ctx.finish_step("resolve").await;

        ctx.begin_step("download").await;
        let total = p.updates.len().max(1);
        ctx.set_items(0, total as u32).await;
        let completed = Arc::new(AtomicUsize::new(0));

        let control = ctx.control().clone();

        let results: Vec<Result<(), AppError>> = stream::iter(p.updates.into_iter().map(|item| {
            let completed = Arc::clone(&completed);
            let control = control.clone();
            let mods_dir = mods_dir.clone();
            let zh_by_file = Arc::clone(&zh_by_file);
            let downloader = Arc::clone(&downloader);

            async move {
                if control.checkpoint().await.is_err() {
                    return Err(AppError::new("task.canceled"));
                }

                let urls: Vec<Url> = item
                    .new_file
                    .download_urls
                    .iter()
                    .filter_map(|u| Url::parse(u).ok())
                    .collect();
                if urls.is_empty() {
                    return Err(AppError::new("comp.download.invalidUrls"));
                }

                let zh_name = zh_by_file.get(&item.current_file_name).map(|s| s.as_str());
                let dest_name = naming::format_download_name(
                    &item.new_file.file_name,
                    zh_name,
                    CompType::Mod,
                    filename_format,
                );
                let dest_path = mods_dir.join(&dest_name);

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

                downloader
                    .download(spec, None)
                    .await
                    .map_err(AppError::from)?;

                if !item.current_file_name.is_empty() && item.current_file_name != dest_name {
                    let old_path = mods_dir.join(&item.current_file_name);
                    if old_path.exists() {
                        let _ = delete_local_mod(&mods_dir, &item.current_file_name);
                    }
                }

                let _done = completed.fetch_add(1, Ordering::SeqCst) + 1;
                Ok(())
            }
        }))
        .buffer_unordered(concurrency)
        .collect()
        .await;

        for r in results {
            r?;
        }

        ctx.set_items(total as u32, total as u32).await;
        ctx.finish_step("download").await;
        Ok(())
    }
}
