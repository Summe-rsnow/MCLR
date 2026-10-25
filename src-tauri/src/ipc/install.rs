//! Minecraft installation, loader configurator, and instance repair IPC commands.
//! Owned by slice B1.

use mclr_comp::model::CompFile;
use mclr_comp::modrinth::ModrinthClient;
use mclr_core::{AppError, SettingsStore, TaskCenter};
use mclr_game::gamedir::resolve_game_dir;
use mclr_game::instance::classify_instance;
use mclr_game::{
    VersionCompatInfo, VersionManifestList, compute_version_compat, fetch_version_manifest,
    install_vanilla as game_install_vanilla, load_resolved_version_json,
    repair_instance as game_repair_instance,
};
use mclr_loaders::{InstallEnv, InstallRequest};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::State;

use super::current_mc_folder;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExtraCandidates {
    pub fabric_api: Vec<CompFile>,
    pub legacy_fabric_api: Vec<CompFile>,
    pub opti_fabric: Vec<CompFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CurrentExtras {
    pub fabric_api: Option<String>,
    pub legacy_fabric_api: Option<String>,
    pub optifabric: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CurrentInstallInfo {
    pub loaders: Vec<String>,
    pub extras: CurrentExtras,
}

#[tauri::command]
#[specta::specta]
pub async fn install_manifest(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    force_refresh: bool,
) -> Result<VersionManifestList, AppError> {
    let settings = state.get().await;
    let client = http.client();
    fetch_version_manifest(&client, settings.net.source_priority, force_refresh)
        .await
        .map_err(AppError::from)
}
#[tauri::command]
#[specta::specta]
pub async fn install_version_compat(
    mc_version: String,
    release_type: String,
    release_time: String,
) -> Result<VersionCompatInfo, AppError> {
    Ok(compute_version_compat(
        &mc_version,
        &release_type,
        &release_time,
    ))
}

#[tauri::command]
#[specta::specta]
pub async fn install_create(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    req: InstallRequest,
) -> Result<String, AppError> {
    let settings = state.get().await;
    let client = http.client();
    let mc_folder = current_mc_folder(&state).await;

    let mut params = HashMap::new();
    params.insert("mcVersion".to_string(), req.mc_version.clone());
    params.insert("name".to_string(), req.instance_name.clone());

    let (task_id, _token) = tasks_state
        .create_task(None, "tasks.item.install.create", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();
    let net_settings = settings.net.clone();
    let java_settings = settings.java.clone();
    let game_settings = settings.game.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        let env = InstallEnv {
            client,
            net_settings,
            java_settings,
            game_settings,
            mc_folder,
            task_ctx: Some((&tc, tid.as_str())),
        };

        match mclr_loaders::install_instance(req, env).await {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                tracing::error!("Installation failed for {}: {:?}", tid, e);
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn install_extras(
    state: State<'_, SettingsStore>,
    mc_version: String,
    fabric: bool,
    legacy_fabric: bool,
    optifabric: bool,
) -> Result<ExtraCandidates, AppError> {
    let settings = state.get().await;
    let modrinth = ModrinthClient::new(settings.net.source_priority);

    let fabric_api = if fabric {
        modrinth
            .get_versions("fabric-api", Some(&mc_version), Some("fabric"))
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let legacy_fabric_api = if legacy_fabric {
        modrinth
            .get_versions("legacy-fabric-api", Some(&mc_version), None)
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let opti_fabric = if optifabric {
        modrinth
            .get_versions("optifabric", Some(&mc_version), Some("fabric"))
            .await
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(ExtraCandidates {
        fabric_api,
        legacy_fabric_api,
        opti_fabric,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn install_current_info(
    state: State<'_, SettingsStore>,
    instance_id: String,
) -> Result<CurrentInstallInfo, AppError> {
    let settings = state.get().await;
    let mc_folder = current_mc_folder(&state).await;
    let game_settings = settings.game.clone();

    super::run_blocking(move || {
        let version_json =
            load_resolved_version_json(&mc_folder, &instance_id).map_err(AppError::from)?;
        let mc_ver = mclr_game::instance::extract_mc_version(&version_json, &instance_id);
        let (_, loaders) = classify_instance(&version_json, &mc_ver);

        let game_dir = resolve_game_dir(&mc_folder, &instance_id, &game_settings);
        let mods_dir = game_dir.join("mods");

        let mut fabric_api = None;
        let mut legacy_fabric_api = None;
        let mut optifabric = None;

        if mods_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&mods_dir) {
                for entry in entries.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let lower = file_name.to_lowercase();
                    if lower.contains("legacy") && lower.contains("fabric-api") {
                        legacy_fabric_api = Some(file_name);
                    } else if lower.contains("fabric-api") || lower.contains("fabric_api") {
                        fabric_api = Some(file_name);
                    } else if lower.contains("optifabric") {
                        optifabric = Some(file_name);
                    }
                }
            }
        }

        Ok::<_, AppError>(CurrentInstallInfo {
            loaders,
            extras: CurrentExtras {
                fabric_api,
                legacy_fabric_api,
                optifabric,
            },
        })
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn install_validate_name(
    state: State<'_, SettingsStore>,
    name: String,
) -> Result<(), AppError> {
    let mc_folder = current_mc_folder(&state).await;
    let versions_dir = mc_folder.join("versions");

    super::run_blocking(move || {
        mclr_game::naming::validate_instance_name_ext(&name, Some(&versions_dir))
    })
    .await
}

#[tauri::command]
#[specta::specta]
pub async fn install_vanilla(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    version: String,
    name: String,
) -> Result<String, AppError> {
    let settings = state.get().await;
    let client = http.client();
    let mc_folder = current_mc_folder(&state).await;
    let net_settings = settings.net.clone();

    let mut params = HashMap::new();
    params.insert("version".to_string(), version.clone());
    params.insert("name".to_string(), name.clone());

    let (task_id, _token) = tasks_state
        .create_task(None, "tasks.item.download.group", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match game_install_vanilla(
            &client,
            &net_settings,
            &mc_folder,
            &version,
            &name,
            Some((&tc, &tid)),
        )
        .await
        {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                tracing::error!("Installation failed for {}: {:?}", tid, e);
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}

#[tauri::command]
#[specta::specta]
pub async fn install_repair(
    http: State<'_, crate::http::HttpState>,
    state: State<'_, SettingsStore>,
    tasks_state: State<'_, Arc<TaskCenter>>,
    instance_id: String,
) -> Result<String, AppError> {
    let settings = state.get().await;
    let client = http.client();
    let mc_folder = current_mc_folder(&state).await;
    let net_settings = settings.net.clone();

    let mut params = HashMap::new();
    params.insert("instanceId".to_string(), instance_id.clone());

    let (task_id, _token) = tasks_state
        .create_task(None, "tasks.item.download.group", params)
        .await;

    let tc = Arc::clone(&tasks_state);
    let tid = task_id.clone();

    tokio::spawn(async move {
        let _ = tc.start_task(&tid).await;
        match game_repair_instance(
            &client,
            &net_settings,
            &mc_folder,
            &instance_id,
            Some((&tc, &tid)),
        )
        .await
        {
            Ok(_) => {
                let _ = tc.complete_task(&tid).await;
            }
            Err(e) => {
                tracing::error!("Installation failed for {}: {:?}", tid, e);
                let _ = tc.fail_task(&tid, AppError::from(e)).await;
            }
        }
    });

    Ok(task_id)
}
