//! Task handler for loader installation.

use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_loaders::{LoaderInstallContext, LoaderKind, install_with_loader};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

pub struct LoaderInstallHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct LoaderInstallPayload {
    pub mc_version: String,
    pub instance_name: String,
    pub loader: LoaderKind,
    pub loader_version: String,
    pub optifine_version: Option<String>,
}

#[async_trait::async_trait]
impl TaskHandler for LoaderInstallHandler {
    fn kind(&self) -> &'static str {
        "loader.install"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: LoaderInstallPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("loader".to_string(), format!("{:?}", p.loader));
        params.insert("mcVersion".to_string(), p.mc_version);
        params.insert("name".to_string(), p.instance_name);
        params.insert("version".to_string(), p.loader_version);
        Ok(TaskPlan {
            title_key: "tasks.item.download.group".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "staging",
                    weight: 35.0,
                },
                StepPlan {
                    key: "merge",
                    weight: 5.0,
                },
                StepPlan {
                    key: "libraries",
                    weight: 5.0,
                },
                StepPlan {
                    key: "repair",
                    weight: 55.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: LoaderInstallPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();
        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;

        if ctx.resumed || ctx.attempt > 0 {
            mclr_game::install::remove_installing_dir(&mc_folder, &p.instance_name);
        }

        let l_ctx = LoaderInstallContext {
            client: &client,
            net_settings: &settings.net,
            java_settings: &settings.java,
            mc_folder,
            mc_version: p.mc_version,
            instance_name: p.instance_name,
            loader_version: p.loader_version,
            optifine_version: p.optifine_version,
            task_ctx: ctx.tc(),
        };

        install_with_loader(p.loader, l_ctx)
            .await
            .map_err(AppError::from)?;

        Ok(())
    }
}
