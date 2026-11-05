//! Task handlers for specific task kinds.

pub mod comp;
pub mod export;
pub mod install;
pub mod loaders;
pub mod localmods;
pub mod modpack;
pub mod presets;
pub mod tools;
pub mod update;

use mclr_core::tasks::TaskEngine;
use std::sync::Arc;
use tauri::AppHandle;

pub async fn register_all(engine: &TaskEngine, app: &AppHandle) {
    engine
        .register(Arc::new(install::VanillaInstallHandler {
            app: app.clone(),
        }))
        .await;
    engine
        .register(Arc::new(install::CreateInstallHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(install::RepairInstallHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(loaders::LoaderInstallHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(modpack::ModpackImportHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(modpack::ModpackInstallHandler {
            app: app.clone(),
        }))
        .await;
    engine
        .register(Arc::new(comp::CompDownloadHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(localmods::LocalModUpdateHandler {
            app: app.clone(),
        }))
        .await;
    engine
        .register(Arc::new(localmods::LocalModsUpdateManyHandler {
            app: app.clone(),
        }))
        .await;
    engine
        .register(Arc::new(presets::PresetsApplyHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(presets::PresetsCacheHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(tools::ToolsCustomDownloadHandler {
            app: app.clone(),
        }))
        .await;
    engine
        .register(Arc::new(update::UpdateDownloadHandler { app: app.clone() }))
        .await;
    engine
        .register(Arc::new(export::ExportModpackHandler { app: app.clone() }))
        .await;
}
