//! Task handler for custom URL downloads.

use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_net::downloader::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Manager};
use url::Url;

pub struct ToolsCustomDownloadHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct ToolsCustomDownloadPayload {
    pub url: String,
    pub dest_path: String,
}

#[async_trait::async_trait]
impl TaskHandler for ToolsCustomDownloadHandler {
    fn kind(&self) -> &'static str {
        "tools.customDownload"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: ToolsCustomDownloadPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let dest = Path::new(&p.dest_path);
        let file_name = dest
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "download.bin".to_string());
        let mut params = HashMap::new();
        params.insert("name".to_string(), file_name);
        Ok(TaskPlan {
            title_key: "tasks.titles.customDownload".into(),
            title_params: params,
            steps: vec![StepPlan {
                key: "download",
                weight: 1.0,
            }],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: ToolsCustomDownloadPayload = ctx.payload()?;
        let settings_state = self.app.state::<SettingsStore>();
        let settings = settings_state.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();

        let parsed_url = Url::parse(&p.url)
            .map_err(|e| AppError::new("tools.download.invalidUrl").with_detail(e.to_string()))?;
        let dest = Path::new(&p.dest_path).to_path_buf();

        let spec = DownloadSpec {
            urls: vec![parsed_url],
            dest,
            size: None,
            sha1: None,
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
