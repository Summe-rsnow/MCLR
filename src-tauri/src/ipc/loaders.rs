//! Mod loader installers IPC commands.

use mclr_core::AppError;
use mclr_loaders::{LoaderKind, LoaderVersionEntry, list_loader_versions};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn loaders_versions(
    http: State<'_, crate::http::HttpState>,
    loader: LoaderKind,
    mc_version: String,
) -> Result<Vec<LoaderVersionEntry>, AppError> {
    let client = http.client();
    list_loader_versions(&client, loader, &mc_version)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn loaders_install(
    engine: State<'_, Arc<mclr_core::tasks::TaskEngine>>,
    mc_version: String,
    instance_name: String,
    loader: LoaderKind,
    loader_version: String,
    optifine_version: Option<String>,
) -> Result<String, AppError> {
    engine
        .submit(
            "loader.install",
            serde_json::to_value(crate::tasks::handlers::loaders::LoaderInstallPayload {
                mc_version,
                instance_name,
                loader,
                loader_version,
                optifine_version,
            })?,
        )
        .await
}
