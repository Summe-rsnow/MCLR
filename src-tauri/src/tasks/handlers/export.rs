//! Task handler for modpack export.

use mclr_comp::curseforge::CurseForgeClient;
use mclr_comp::export::{ModpackExportOptions, export_modpack};
use mclr_comp::modrinth::ModrinthClient;
use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

pub struct ExportModpackHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct ExportModpackPayload {
    pub options: ModpackExportOptions,
}

#[async_trait::async_trait]
impl TaskHandler for ExportModpackHandler {
    fn kind(&self) -> &'static str {
        "export.modpack"
    }

    fn resumable(&self) -> bool {
        true
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: ExportModpackPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("instanceId".to_string(), p.options.instance_id.clone());
        params.insert("packName".to_string(), p.options.pack_name.clone());
        params.insert("outputPath".to_string(), p.options.output_path.clone());
        Ok(TaskPlan {
            title_key: "task.export.title".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "collect",
                    weight: 10.0,
                },
                StepPlan {
                    key: "hash",
                    weight: 60.0,
                },
                StepPlan {
                    key: "write",
                    weight: 30.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: ExportModpackPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let inst_dir = crate::ipc::game_dir_for(&settings_store, &p.options.instance_id).await;
        let mr_client = ModrinthClient::new(settings.net.source_priority);
        let cf_client = CurseForgeClient::new(settings.net.source_priority);

        export_modpack(
            &inst_dir,
            &p.options,
            Some(&mr_client),
            Some(&cf_client),
            ctx.tc(),
        )
        .await
        .map_err(AppError::from)?;

        Ok(())
    }
}
