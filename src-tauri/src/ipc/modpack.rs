//! Modpack import IPC commands.

use std::path::PathBuf;
use std::sync::Arc;

use mclr_comp::{CompError, CompFile, ModpackInspectResult, inspect_modpack};
use mclr_core::AppError;
use tauri::State;
use url::Url;

#[tauri::command]
#[specta::specta]
pub async fn modpack_inspect(archive_path: String) -> Result<ModpackInspectResult, AppError> {
    let target = crate::security::ensure_not_system_path(std::path::Path::new(&archive_path))?;
    super::run_blocking(move || inspect_modpack(target.as_path()).map(|p| p.inspect)).await
}

#[tauri::command]
#[specta::specta]
pub async fn modpack_import_file(
    engine: State<'_, Arc<mclr_core::tasks::TaskEngine>>,
    archive_path: String,
    instance_name: String,
) -> Result<String, AppError> {
    mclr_game::instance::validate_instance_name(&instance_name).map_err(AppError::from)?;
    let path = PathBuf::from(&archive_path);
    if !path.exists() {
        return Err(CompError::FileNotFound(archive_path.clone()).into());
    }

    engine
        .submit(
            "modpack.import",
            serde_json::to_value(crate::tasks::handlers::modpack::ModpackImportPayload {
                archive_path,
                instance_name,
            })?,
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn modpack_install_remote(
    engine: State<'_, Arc<mclr_core::tasks::TaskEngine>>,
    file: CompFile,
    instance_name: String,
) -> Result<String, AppError> {
    mclr_game::instance::validate_instance_name(&instance_name).map_err(AppError::from)?;
    if file.distribution_disabled || file.download_urls.is_empty() {
        return Err(CompError::DistributionDisabled {
            project_id: file.project_id,
            file_id: file.id,
            website_url: None,
        }
        .into());
    }

    let urls: Vec<Url> = file
        .download_urls
        .iter()
        .filter_map(|u| Url::parse(u).ok())
        .collect();
    if urls.is_empty() {
        return Err(
            CompError::InvalidParams("No valid download URLs in CompFile".to_string()).into(),
        );
    }

    engine
        .submit(
            "modpack.install",
            serde_json::to_value(crate::tasks::handlers::modpack::ModpackInstallPayload {
                file,
                instance_name,
            })?,
        )
        .await
}
