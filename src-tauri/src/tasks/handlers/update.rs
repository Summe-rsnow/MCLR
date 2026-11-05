//! Task handler for application updater download and installation.

use mclr_core::AppError;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_update::UpdateConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

pub struct UpdateDownloadHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateDownloadPayload {
    pub channel: String,
}

#[async_trait::async_trait]
impl TaskHandler for UpdateDownloadHandler {
    fn kind(&self) -> &'static str {
        "update.download"
    }

    fn resumable(&self) -> bool {
        false
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: UpdateDownloadPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("channel".to_string(), p.channel);
        Ok(TaskPlan {
            title_key: "task.update.download".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "check",
                    weight: 2.0,
                },
                StepPlan {
                    key: "download",
                    weight: 98.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: UpdateDownloadPayload = ctx.payload()?;
        let config = UpdateConfig::from_runtime(&p.channel)?;

        let updater = self
            .app
            .updater_builder()
            .endpoints(vec![config.endpoint.clone()])
            .map_err(|e| AppError::new("update.invalidUrl").with_detail(e.to_string()))?
            .pubkey(config.pubkey.clone())
            .build()
            .map_err(|e| AppError::new("update.http").with_detail(e.to_string()))?;

        ctx.begin_step("check").await;
        let check_res = updater
            .check()
            .await
            .map_err(|e| AppError::new("update.http").with_detail(e.to_string()))?;
        ctx.finish_step("check").await;

        let Some(update) = check_res else {
            return Ok(());
        };

        ctx.begin_step("download").await;
        let downloaded_bytes = Arc::new(AtomicU64::new(0));
        let downloaded_clone = Arc::clone(&downloaded_bytes);
        let tid = ctx.id.clone();
        let center = ctx.center();

        let download_res = update
            .download_and_install(
                move |chunk_length, content_length| {
                    let cur = downloaded_clone.fetch_add(chunk_length as u64, Ordering::Relaxed)
                        + chunk_length as u64;
                    if let Some(total) = content_length {
                        if total > 0 {
                            let prog = (cur as f32 / total as f32).clamp(0.0, 1.0);
                            let center = Arc::clone(&center);
                            let t = tid.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = center.update_progress(&t, prog, 0).await;
                            });
                        }
                    }
                },
                || {},
            )
            .await;

        download_res.map_err(|e| AppError::new("update.install").with_detail(e.to_string()))?;
        ctx.finish_step("download").await;
        Ok(())
    }
}
