//! Task handler for component (mod/resourcepack/shader) downloads.

use mclr_comp::{CompFile, CompType};
use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_net::downloader::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};
use url::Url;

pub struct CompDownloadHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct CompDownloadPayload {
    pub file: CompFile,
    pub target_instance: String,
    pub kind: CompType,
    pub dest_name: Option<String>,
}

#[async_trait::async_trait]
impl TaskHandler for CompDownloadHandler {
    fn kind(&self) -> &'static str {
        "comp.download"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: CompDownloadPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("name".to_string(), p.file.name.clone());
        params.insert("fileName".to_string(), p.file.file_name.clone());
        params.insert("targetInstance".to_string(), p.target_instance);
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
        let p: CompDownloadPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();
        let base_dir = crate::ipc::game_dir_for(&settings_store, &p.target_instance).await;
        let target_dir = base_dir.join(p.kind.target_subfolder());
        let _ = tokio::fs::create_dir_all(&target_dir).await;
        let dest_name = p.dest_name.unwrap_or(p.file.file_name);
        let dest_path = target_dir.join(dest_name);

        let urls: Vec<Url> = p
            .file
            .download_urls
            .iter()
            .filter_map(|u| Url::parse(u).ok())
            .collect();
        if urls.is_empty() {
            return Err(AppError::new("comp.download.invalidUrls"));
        }

        let spec = DownloadSpec {
            urls,
            dest: dest_path,
            size: if p.file.size > 0 {
                Some(p.file.size)
            } else {
                None
            },
            sha1: p.file.hashes.sha1,
            sha256: None,
            md5: None,
        };

        let downloader = Downloader::new(client, settings.net);
        downloader
            .download(spec, ctx.tc())
            .await
            .map_err(AppError::from)?;

        Ok(())
    }
}
