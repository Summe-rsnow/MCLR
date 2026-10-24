//! Modpack import IPC commands.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use mclr_comp::{
    CompError, CompFile, CurseForgeClient, ModpackInspectResult, import_modpack, inspect_modpack,
};
use mclr_core::{AppError, SettingsStore, TaskCenter};
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
pub async fn modpack_inspect(archive_path: String) -> Result<ModpackInspectResult, AppError> {
    let path = PathBuf::from(&archive_path);
    let plan = inspect_modpack(&path).map_err(AppError::from)?;
    Ok(plan.inspect)
}

#[tauri::command]
#[specta::specta]
pub async fn modpack_import_file(
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    archive_path: String,
    instance_name: String,
) -> Result<String, AppError> {
    mclr_game::instance::validate_instance_name(&instance_name).map_err(AppError::from)?;
    let path = PathBuf::from(&archive_path);
    if !path.exists() {
        return Err(CompError::FileNotFound(archive_path.clone()).into());
    }

    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    let mc_folder = current_mc_folder(&state).await;
    let cf_client = CurseForgeClient::new(settings.net.source_priority);

    let mut params = HashMap::new();
    params.insert("name".to_string(), instance_name.clone());
    params.insert("archivePath".to_string(), archive_path.clone());

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.modpackImport", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();
    let net_settings = settings.net.clone();
    let java_settings = settings.java.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match import_modpack(
            &path,
            &instance_name,
            &mc_folder,
            &client,
            &net_settings,
            Some(&java_settings),
            Some(&cf_client),
            Some((&tc, &tid)),
            true,
        )
        .await
        {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, e.into()).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn modpack_install_remote(
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
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

    let settings = state.get().await;
    let client = build_http_client(&settings.net).map_err(AppError::from)?;
    let mc_folder = current_mc_folder(&state).await;
    let cf_client = CurseForgeClient::new(settings.net.source_priority);

    let cache_dir = mclr_core::paths::cache_dir().join("modpacks");
    std::fs::create_dir_all(&cache_dir).map_err(CompError::Io)?;
    let archive_path = cache_dir.join(&file.file_name);

    let mut params = HashMap::new();
    params.insert("name".to_string(), instance_name.clone());
    params.insert("fileName".to_string(), file.file_name.clone());

    let (task_id, _cancel_token) = tasks_state
        .create_task(None, "tasks.titles.modpackInstall", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();
    let net_settings = settings.net.clone();
    let java_settings = settings.java.clone();
    let downloader = Downloader::new(client.clone(), net_settings.clone());

    let spec = DownloadSpec {
        urls,
        dest: archive_path.clone(),
        size: if file.size > 0 { Some(file.size) } else { None },
        sha1: file.hashes.sha1,
        sha256: None,
        md5: None,
    };

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        // Step 1: download the modpack archive
        if let Err(e) = downloader.download(spec, Some((&tc, &tid))).await {
            let _ = tc.fail_task(&tid, e.into()).await;
            return;
        }

        // Step 2: import the modpack
        match import_modpack(
            &archive_path,
            &instance_name,
            &mc_folder,
            &client,
            &net_settings,
            Some(&java_settings),
            Some(&cf_client),
            Some((&tc, &tid)),
            true,
        )
        .await
        {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                let _ = tc.fail_task(&tid, e.into()).await;
            }
        }
    });

    Ok(task_id)
}
