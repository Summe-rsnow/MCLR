//! Task handlers for installation and repair operations.

use mclr_core::AppError;
use mclr_core::SettingsStore;
use mclr_core::tasks::{StepPlan, TaskCtx, TaskHandler, TaskPlan};
use mclr_loaders::InstallRequest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Manager};

pub struct VanillaInstallHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct VanillaInstallPayload {
    pub version: String,
    pub name: String,
}

#[async_trait::async_trait]
impl TaskHandler for VanillaInstallHandler {
    fn kind(&self) -> &'static str {
        "install.vanilla"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: VanillaInstallPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("version".to_string(), p.version);
        params.insert("name".to_string(), p.name);
        Ok(TaskPlan {
            title_key: "tasks.item.download.group".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "manifest",
                    weight: 3.0,
                },
                StepPlan {
                    key: "metadata",
                    weight: 5.0,
                },
                StepPlan {
                    key: "verify",
                    weight: 12.0,
                },
                StepPlan {
                    key: "download",
                    weight: 78.0,
                },
                StepPlan {
                    key: "finalize",
                    weight: 2.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: VanillaInstallPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();
        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;

        if ctx.resumed || ctx.attempt > 0 {
            mclr_game::install::remove_installing_dir(&mc_folder, &p.name);
        }

        mclr_game::install::install_vanilla(
            &client,
            &settings.net,
            &mc_folder,
            &p.version,
            &p.name,
            ctx.tc(),
        )
        .await
        .map_err(AppError::from)?;

        Ok(())
    }
}

pub struct CreateInstallHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct CreateInstallPayload {
    pub req: InstallRequest,
}

#[async_trait::async_trait]
impl TaskHandler for CreateInstallHandler {
    fn kind(&self) -> &'static str {
        "install.create"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: CreateInstallPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("mcVersion".to_string(), p.req.mc_version.clone());
        params.insert("name".to_string(), p.req.instance_name.clone());
        Ok(TaskPlan {
            title_key: "tasks.item.install.create".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "staging",
                    weight: 30.0,
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
                    key: "extraMods",
                    weight: 10.0,
                },
                StepPlan {
                    key: "repair",
                    weight: 50.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: CreateInstallPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();
        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;

        if ctx.resumed || ctx.attempt > 0 {
            mclr_game::install::remove_installing_dir(&mc_folder, &p.req.instance_name);
        }

        let env = mclr_loaders::InstallEnv {
            client,
            net_settings: settings.net,
            java_settings: settings.java,
            game_settings: settings.game,
            mc_folder,
            task_ctx: ctx.tc(),
        };

        mclr_loaders::install_instance(p.req, env)
            .await
            .map_err(AppError::from)?;

        Ok(())
    }
}

pub struct RepairInstallHandler {
    pub app: AppHandle,
}

#[derive(Serialize, Deserialize)]
pub struct RepairInstallPayload {
    pub instance_id: String,
}

#[async_trait::async_trait]
impl TaskHandler for RepairInstallHandler {
    fn kind(&self) -> &'static str {
        "install.repair"
    }

    fn plan(&self, payload: &serde_json::Value) -> Result<TaskPlan, AppError> {
        let p: RepairInstallPayload = serde_json::from_value(payload.clone())
            .map_err(|e| AppError::new("task.payloadInvalid").with_detail(e.to_string()))?;
        let mut params = HashMap::new();
        params.insert("instanceId".to_string(), p.instance_id);
        Ok(TaskPlan {
            title_key: "tasks.item.download.group".into(),
            title_params: params,
            steps: vec![
                StepPlan {
                    key: "resolve",
                    weight: 2.0,
                },
                StepPlan {
                    key: "verify",
                    weight: 18.0,
                },
                StepPlan {
                    key: "download",
                    weight: 80.0,
                },
            ],
        })
    }

    async fn run(&self, ctx: &TaskCtx) -> Result<(), AppError> {
        let p: RepairInstallPayload = ctx.payload()?;
        let settings_store = self.app.state::<SettingsStore>();
        let settings = settings_store.get().await;
        let http = self.app.state::<crate::http::HttpState>();
        let client = http.client();
        let mc_folder = crate::ipc::current_mc_folder(&settings_store).await;

        if ctx.resumed || ctx.attempt > 0 {
            mclr_game::install::remove_installing_dir(&mc_folder, &p.instance_id);
        }

        mclr_game::install::repair_instance(
            &client,
            &settings.net,
            &mc_folder,
            &p.instance_id,
            ctx.tc(),
            mclr_game::install::RepairMode::Full,
        )
        .await
        .map_err(AppError::from)?;

        Ok(())
    }
}
