//! Task handlers for modpack import and remote installation.

use mclr_comp::CompFile;
use mclr_comp::curseforge::CurseForgeClient;
use mclr_comp::modpack::{ModpackImportContext, import_modpack};
use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_net::downloader::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use url::Url;

pub struct ModpackImportHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct ModpackImportPayload {
    pub archive_path: String,
    pub instance_name: String,
}

#[async_trait::async_trait]
impl TaskHandler for ModpackImportHandler {
    fn kind(&self) -> &'static str {
        "modpack.import"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: ModpackImportPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("name".to_string(), p.instance_name);
        params.insert("archivePath".to_string(), p.archive_path);
        Ok(TaskPlan {
            title_key: "tasks.titles.modpackImport".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "inspect",
                    weight: 5.0,
                },
                StepPlan {
                    key: "baseGame",
                    weight: 62.0,
                },
                StepPlan {
                    key: "files",
                    weight: 31.0,
                },
                StepPlan {
                    key: "overrides",
                    weight: 2.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: ModpackImportPayload = ctx.payload()?;
        let path = PathBuf::from(&p.archive_path);
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();
        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;
        let cf_client = CurseForgeClient::new(settings.net.source_priority);

        let res = import_modpack(ModpackImportContext {
            archive_path: &path,
            instance_name: &p.instance_name,
            mc_folder: &mc_folder,
            client: &client,
            net_settings: &settings.net,
            java_settings: Some(&settings.java),
            cf_client: Some(&cf_client),
            task_ctx: ctx.tc(),
            install_base_game: true,
        })
        .await
        .map_err(AppError::from);

        let _ = tokio::fs::remove_file(&p.archive_path).await;
        res.map(|_| ())
    }
}

pub struct ModpackInstallHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct ModpackInstallPayload {
    pub file: CompFile,
    pub instance_name: String,
}

#[async_trait::async_trait]
impl TaskHandler for ModpackInstallHandler {
    fn kind(&self) -> &'static str {
        "modpack.install"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: ModpackInstallPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("name".to_string(), p.instance_name);
        params.insert("fileName".to_string(), p.file.file_name);
        Ok(TaskPlan {
            title_key: "tasks.titles.modpackInstall".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "download",
                    weight: 8.0,
                },
                StepPlan {
                    key: "inspect",
                    weight: 4.0,
                },
                StepPlan {
                    key: "baseGame",
                    weight: 58.0,
                },
                StepPlan {
                    key: "files",
                    weight: 28.0,
                },
                StepPlan {
                    key: "overrides",
                    weight: 2.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: ModpackInstallPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();
        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;
        let cf_client = CurseForgeClient::new(settings.net.source_priority);

        let cache_dir = mclr_core::paths::cache_dir().join("modpacks");
        let _ = tokio::fs::create_dir_all(&cache_dir).await;
        let archive_path = cache_dir.join(&p.file.file_name);
        ctx.add_artifact(&archive_path).await;

        let should_download = ctx.begin_step("download").await;
        let archive_exists = archive_path.is_file();
        let size_matches = p.file.size == 0
            || tokio::fs::metadata(&archive_path)
                .await
                .map(|m| m.len() == p.file.size)
                .unwrap_or(false);

        if should_download || !archive_exists || !size_matches {
            let urls: Vec<Url> = p
                .file
                .download_urls
                .iter()
                .filter_map(|u| Url::parse(u).ok())
                .collect();
            let downloader = Downloader::new(client.clone(), settings.net.clone());
            let spec = DownloadSpec {
                urls,
                dest: archive_path.clone(),
                size: if p.file.size > 0 {
                    Some(p.file.size)
                } else {
                    None
                },
                sha1: p.file.hashes.sha1,
                sha256: None,
                md5: None,
            };

            downloader
                .download(spec, ctx.tc())
                .await
                .map_err(AppError::from)?;
            ctx.finish_step("download").await;
        }

        let res = import_modpack(ModpackImportContext {
            archive_path: &archive_path,
            instance_name: &p.instance_name,
            mc_folder: &mc_folder,
            client: &client,
            net_settings: &settings.net,
            java_settings: Some(&settings.java),
            cf_client: Some(&cf_client),
            task_ctx: ctx.tc(),
            install_base_game: true,
        })
        .await
        .map_err(AppError::from);

        let _ = tokio::fs::remove_file(&archive_path).await;
        res.map(|_| ())
    }
}
