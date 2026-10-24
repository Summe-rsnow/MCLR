//! Community platform (Modrinth / CurseForge) IPC commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mclr_comp::{
    CompError, CompFile, CompPlatform, CompProject, CompSearchQuery, CompSearchResult, CompType,
    CurseForgeClient, ModrinthClient,
};
use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_game::InstanceConfig;
use mclr_net::{DownloadSpec, Downloader, build_http_client};
use tauri::State;
use url::Url;

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
pub async fn comp_search(
    state: State<'_, SettingsStore>,
    query: CompSearchQuery,
) -> Result<CompSearchResult, AppError> {
    let settings = state.get().await;
    match query.platform {
        CompPlatform::Modrinth => {
            let client = ModrinthClient::new(settings.net.source_priority);
            client.search(&query).await.map_err(AppError::from)
        }
        CompPlatform::Curseforge => {
            let client = CurseForgeClient::new(settings.net.source_priority);
            client.search(&query).await.map_err(AppError::from)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn comp_project(
    state: State<'_, SettingsStore>,
    platform: CompPlatform,
    project_id: String,
) -> Result<CompProject, AppError> {
    let settings = state.get().await;
    match platform {
        CompPlatform::Modrinth => {
            let client = ModrinthClient::new(settings.net.source_priority);
            client
                .get_project(&project_id)
                .await
                .map_err(AppError::from)
        }
        CompPlatform::Curseforge => {
            let client = CurseForgeClient::new(settings.net.source_priority);
            client
                .get_project(&project_id)
                .await
                .map_err(AppError::from)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn comp_files(
    state: State<'_, SettingsStore>,
    platform: CompPlatform,
    project_id: String,
    mc_version: Option<String>,
    loader: Option<String>,
) -> Result<Vec<CompFile>, AppError> {
    let settings = state.get().await;
    match platform {
        CompPlatform::Modrinth => {
            let client = ModrinthClient::new(settings.net.source_priority);
            client
                .get_versions(&project_id, mc_version.as_deref(), loader.as_deref())
                .await
                .map_err(AppError::from)
        }
        CompPlatform::Curseforge => {
            let client = CurseForgeClient::new(settings.net.source_priority);
            client
                .get_files(&project_id, mc_version.as_deref(), loader.as_deref())
                .await
                .map_err(AppError::from)
        }
    }
}

#[tauri::command]
#[specta::specta]
pub async fn comp_download(
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    file: CompFile,
    target_instance: String,
    kind: CompType,
) -> Result<String, AppError> {
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

    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    let mc_folder = current_mc_folder(&state).await;

    let instance_dir = mc_folder.join("versions").join(&target_instance);
    let config_path = instance_dir.join("mclr.instance.json");
    let instance_config = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| serde_json::from_str::<InstanceConfig>(&content).ok())
            .unwrap_or_default()
    } else {
        InstanceConfig::default()
    };

    let base_dir = if instance_config.isolation == "on"
        || (instance_config.isolation == "global" && settings.game.default_isolation == "on")
    {
        instance_dir
    } else {
        mc_folder
    };

    let target_dir = base_dir.join(kind.target_subfolder());
    std::fs::create_dir_all(&target_dir).map_err(CompError::Io)?;
    let dest_path = target_dir.join(&file.file_name);

    let mut params = HashMap::new();
    params.insert("name".to_string(), file.name.clone());
    params.insert("fileName".to_string(), file.file_name.clone());
    params.insert("targetInstance".to_string(), target_instance);

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.compDownload", params)
        .await;

    let spec = DownloadSpec {
        urls,
        dest: dest_path,
        size: if file.size > 0 { Some(file.size) } else { None },
        sha1: file.hashes.sha1,
        sha256: None,
        md5: None,
    };

    let downloader = Downloader::new(client, settings.net.clone());
    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match downloader.download(spec, Some((&tc, &tid))).await {
            Ok(()) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}
