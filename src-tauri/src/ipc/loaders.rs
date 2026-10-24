//! Mod loader installers IPC commands.

use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_loaders::{
    LoaderInstallContext, LoaderKind, LoaderVersionEntry, install_with_loader, list_loader_versions,
};
use mclr_net::build_http_client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

async fn current_mc_folder(state: &State<'_, SettingsStore>) -> PathBuf {
    let settings = state.get().await;
    if settings.game.current_folder.is_empty() {
        mclr_core::paths::default_minecraft_dir()
    } else {
        PathBuf::from(settings.game.current_folder)
    }
}

#[tauri::command]
#[specta::specta]
pub async fn loaders_versions(
    state: State<'_, SettingsStore>,
    loader: LoaderKind,
    mc_version: String,
) -> Result<Vec<LoaderVersionEntry>, AppError> {
    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    list_loader_versions(&client, loader, &mc_version)
        .await
        .map_err(AppError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn loaders_install(
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    mc_version: String,
    instance_name: String,
    loader: LoaderKind,
    loader_version: String,
    optifine_version: Option<String>,
) -> Result<String, AppError> {
    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    let mc_folder = current_mc_folder(&state).await;
    let net_settings = settings.net.clone();
    let java_settings = settings.java.clone();

    let mut params = HashMap::new();
    params.insert("loader".to_string(), format!("{loader:?}"));
    params.insert("mcVersion".to_string(), mc_version.clone());
    params.insert("name".to_string(), instance_name.clone());
    params.insert("version".to_string(), loader_version.clone());

    let (task_id, _token) = tasks_state
        .create_task(None, "tasks.item.download.group", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        let ctx = LoaderInstallContext {
            client: &client,
            net_settings: &net_settings,
            java_settings: &java_settings,
            mc_folder,
            mc_version,
            instance_name,
            loader_version,
            optifine_version,
            task_ctx: Some((&tc, &tid)),
        };

        match install_with_loader(loader, ctx).await {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}
