//! Minecraft installation, loader configurator, and instance repair IPC commands.
//! Owned by slice B1.

use mclr_comp::model::CompFile;
use mclr_comp::modrinth::ModrinthClient;
use mclr_core::{AppError, SettingsStore};
use mclr_game::gamedir::resolve_game_dir;
use mclr_game::instance::classify_instance;
use mclr_game::{
    VersionCompatInfo, VersionManifestList, compute_version_compat, fetch_version_manifest,
    load_resolved_version_json,
};
use mclr_loaders::InstallRequest;
use serde::{Deserialize, Serialize};
use specta::Type;
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
    engine: State<'_, Arc<mclr_core::tasks::TaskEngine>>,
    req: InstallRequest,
) -> Result<String, AppError> {
    engine
        .submit(
            "install.create",
            serde_json::to_value(crate::tasks::handlers::install::CreateInstallPayload { req })?,
        )
        .await
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
    engine: State<'_, Arc<mclr_core::tasks::TaskEngine>>,
    version: String,
    name: String,
) -> Result<String, AppError> {
    engine
        .submit(
            "install.vanilla",
            serde_json::to_value(crate::tasks::handlers::install::VanillaInstallPayload {
                version,
                name,
            })?,
        )
        .await
}

#[tauri::command]
#[specta::specta]
pub async fn install_repair(
    engine: State<'_, Arc<mclr_core::tasks::TaskEngine>>,
    instance_id: String,
) -> Result<String, AppError> {
    engine
        .submit(
            "install.repair",
            serde_json::to_value(crate::tasks::handlers::install::RepairInstallPayload {
                instance_id,
            })?,
        )
        .await
}
