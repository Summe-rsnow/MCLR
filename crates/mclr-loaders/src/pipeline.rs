//! Multi-loader installation pipeline and staging environment (PCL parity).
//! Owned by slice B1.

use crate::error::LoaderError;
use crate::fabric::{
    FabricInstaller, LabyModInstaller, LegacyFabricInstaller, LiteLoaderInstaller,
    OptiFineInstaller,
};
use crate::forge::{CleanroomInstaller, ForgeInstaller, NeoForgeInstaller};
use mclr_core::paths;
use mclr_core::settings::{GameSettings, JavaSettings, NetSettings};
use mclr_core::tasks::TaskCenter;
use mclr_game::gamedir::resolve_game_dir;
use mclr_game::install::{fetch_version_manifest, repair_instance};
use mclr_game::instance::{InstanceConfig, InstanceInfo, InstanceState, classify_instance};
use mclr_game::version::{VersionJson, merge_version_json};
use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::{Path, PathBuf};
use tracing::info;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LabyModSel {
    pub channel: String,
    pub commit_ref: String,
    pub base_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExtraMod {
    pub file_name: String,
    pub urls: Vec<String>,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum InstallMode {
    Create,
    Modify {
        #[serde(default)]
        remove_mod_files: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub mc_version: String,
    pub instance_name: String,
    pub forge: Option<String>,
    pub neoforge: Option<String>,
    pub cleanroom: Option<String>,
    pub fabric: Option<String>,
    pub legacy_fabric: Option<String>,
    pub labymod: Option<LabyModSel>,
    pub liteloader: Option<String>,
    pub optifine: Option<String>,
    pub extra_mods: Vec<ExtraMod>,
    pub mode: InstallMode,
}

pub struct InstallEnv<'a> {
    pub client: reqwest::Client,
    pub net_settings: NetSettings,
    pub java_settings: JavaSettings,
    pub game_settings: GameSettings,
    pub mc_folder: PathBuf,
    pub task_ctx: Option<(&'a TaskCenter, &'a str)>,
}

pub struct StageCtx<'a> {
    pub client: &'a reqwest::Client,
    pub net_settings: &'a NetSettings,
    pub java_settings: &'a JavaSettings,
    pub staging_dir: &'a Path,
    pub mc_folder: &'a Path,
    pub mc_version: &'a str,
    pub instance_name: &'a str,
    pub task_ctx: Option<(&'a TaskCenter, &'a str)>,
}

pub struct Staged {
    pub version_json: VersionJson,
    pub mods: Vec<PathBuf>,
}

fn copy_dir_recursive_skip_existing(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());
        if entry_path.is_dir() {
            copy_dir_recursive_skip_existing(&entry_path, &target_path)?;
        } else if !target_path.exists() {
            if let Some(p) = target_path.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(&entry_path, &target_path)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    if !src.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let entry_path = entry.path();
        let target_path = dst.join(entry.file_name());
        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &target_path)?;
        } else {
            if let Some(p) = target_path.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::copy(&entry_path, &target_path)?;
        }
    }
    Ok(())
}

async fn prepare_vanilla_staging(
    client: &reqwest::Client,
    net_settings: &NetSettings,
    mc_version: &str,
    staging_dir: &Path,
    task_ctx: Option<(&TaskCenter, &str)>,
) -> Result<VersionJson, LoaderError> {
    let cache_dir = paths::cache_dir().join("vanilla").join(mc_version);
    std::fs::create_dir_all(&cache_dir)?;
    let cached_json_path = cache_dir.join(format!("{mc_version}.json"));
    let cached_jar_path = cache_dir.join(format!("{mc_version}.jar"));

    let version_json_str = if cached_json_path.exists() {
        std::fs::read_to_string(&cached_json_path)?
    } else {
        let manifest = fetch_version_manifest(client, net_settings.source_priority, false).await?;
        let entry = manifest
            .versions
            .iter()
            .find(|v| v.id == mc_version)
            .ok_or_else(|| LoaderError::VersionNotFound(mc_version.to_string()))?;

        let res = client
            .get(&entry.url)
            .send()
            .await
            .map_err(mclr_net::error::NetError::Http)?;
        let text = res.text().await.map_err(mclr_net::error::NetError::Http)?;
        std::fs::write(&cached_json_path, &text)?;
        text
    };

    let version_json = VersionJson::parse(&version_json_str).map_err(LoaderError::Game)?;

    if !cached_jar_path.exists() {
        if let Some(downloads) = &version_json.downloads {
            if let Some(client_download) = &downloads.client {
                if let Some(url_str) = &client_download.url {
                    let downloader = Downloader::new(client.clone(), net_settings.clone());
                    let spec = DownloadSpec {
                        urls: vec![
                            Url::parse(url_str)
                                .map_err(|e| LoaderError::Metadata(e.to_string()))?,
                        ],
                        dest: cached_jar_path.clone(),
                        size: client_download.size,
                        sha1: client_download.sha1.clone(),
                        sha256: None,
                        md5: None,
                    };
                    downloader.download_many(vec![spec], task_ctx).await?;
                }
            }
        }
    }

    let staging_version_dir = staging_dir.join("versions").join(mc_version);
    std::fs::create_dir_all(&staging_version_dir)?;
    let staging_json_path = staging_version_dir.join(format!("{mc_version}.json"));
    let staging_jar_path = staging_version_dir.join(format!("{mc_version}.jar"));

    std::fs::copy(&cached_json_path, &staging_json_path)?;
    if cached_jar_path.exists() {
        std::fs::copy(&cached_jar_path, &staging_jar_path)?;
    }

    Ok(version_json)
}

/// Executes the staged multi-loader installation pipeline.
pub async fn install_instance<'a>(
    req: InstallRequest,
    env: InstallEnv<'a>,
) -> Result<InstanceInfo, LoaderError> {
    info!(
        "Starting staged install for instance '{}' (MC {})",
        req.instance_name, req.mc_version
    );

    let now_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let hex_suffix = format!("{:08x}", (now_nanos & 0xffff_ffff) as u32);

    let staging_dir = env
        .mc_folder
        .join(".mclr-staging")
        .join(format!("{}-{}", req.instance_name, hex_suffix));
    let instance_dir = env.mc_folder.join("versions").join(&req.instance_name);

    std::fs::create_dir_all(&instance_dir)?;
    let marker_path = instance_dir.join(".mclr-installing");
    std::fs::write(&marker_path, b"")?;

    let backup_dir = instance_dir.join(".mclr-backups");
    if matches!(req.mode, InstallMode::Modify { .. }) {
        std::fs::create_dir_all(&backup_dir)?;
        let inst_json = instance_dir.join(format!("{}.json", req.instance_name));
        let inst_jar = instance_dir.join(format!("{}.jar", req.instance_name));
        if inst_json.exists() {
            let _ = std::fs::copy(
                &inst_json,
                backup_dir.join(format!("{}.json", req.instance_name)),
            );
        }
        if inst_jar.exists() {
            let _ = std::fs::copy(
                &inst_jar,
                backup_dir.join(format!("{}.jar", req.instance_name)),
            );
        }
    }

    std::fs::create_dir_all(&staging_dir)?;
    std::fs::create_dir_all(staging_dir.join("libraries"))?;
    std::fs::write(
        staging_dir.join("launcher_profiles.json"),
        r#"{"profiles":{}}"#,
    )?;

    let task_ctx_ref = env.task_ctx;

    let res = run_pipeline_steps(&req, &env, &staging_dir, &instance_dir, task_ctx_ref).await;

    match res {
        Ok(info) => {
            let _ = std::fs::remove_file(&marker_path);
            let _ = std::fs::remove_dir_all(&staging_dir);
            if matches!(req.mode, InstallMode::Modify { .. }) {
                let _ = std::fs::remove_dir_all(&backup_dir);
            }
            Ok(info)
        }
        Err(err) => {
            let _ = std::fs::remove_dir_all(&staging_dir);
            match req.mode {
                InstallMode::Create => {
                    let _ = std::fs::remove_dir_all(&instance_dir);
                }
                InstallMode::Modify { .. } => {
                    let b_json = backup_dir.join(format!("{}.json", req.instance_name));
                    let b_jar = backup_dir.join(format!("{}.jar", req.instance_name));
                    if b_json.exists() {
                        let _ = std::fs::copy(
                            &b_json,
                            instance_dir.join(format!("{}.json", req.instance_name)),
                        );
                    }
                    if b_jar.exists() {
                        let _ = std::fs::copy(
                            &b_jar,
                            instance_dir.join(format!("{}.jar", req.instance_name)),
                        );
                    }
                    let _ = std::fs::remove_file(&marker_path);
                    let _ = std::fs::remove_dir_all(&backup_dir);
                }
            }
            Err(err)
        }
    }
}

async fn run_pipeline_steps(
    req: &InstallRequest,
    env: &InstallEnv<'_>,
    staging_dir: &Path,
    instance_dir: &Path,
    task_ctx: Option<(&TaskCenter, &str)>,
) -> Result<InstanceInfo, LoaderError> {
    // 1. Vanilla
    let vanilla_json = prepare_vanilla_staging(
        &env.client,
        &env.net_settings,
        &req.mc_version,
        staging_dir,
        task_ctx,
    )
    .await?;

    let stage_ctx = StageCtx {
        client: &env.client,
        net_settings: &env.net_settings,
        java_settings: &env.java_settings,
        staging_dir,
        mc_folder: &env.mc_folder,
        mc_version: &req.mc_version,
        instance_name: &req.instance_name,
        task_ctx,
    };

    let opti_fine_as_mod = req.optifine.is_some()
        && (req.fabric.is_some()
            || req.legacy_fabric.is_some()
            || req.forge.is_some()
            || req.neoforge.is_some()
            || req.liteloader.is_some());

    // 2. Stage loaders
    let optifine_staged = if let Some(of_ver) = &req.optifine {
        Some(
            OptiFineInstaller::stage(
                &stage_ctx,
                of_ver,
                opti_fine_as_mod,
                req.liteloader.is_some(),
            )
            .await?,
        )
    } else {
        None
    };

    let forge_staged = if let Some(ver) = &req.forge {
        Some(ForgeInstaller::stage(&stage_ctx, ver).await?)
    } else {
        None
    };

    let neoforge_staged = if let Some(ver) = &req.neoforge {
        Some(NeoForgeInstaller::stage(&stage_ctx, ver).await?)
    } else {
        None
    };

    let cleanroom_staged = if let Some(ver) = &req.cleanroom {
        Some(CleanroomInstaller::stage(&stage_ctx, ver).await?)
    } else {
        None
    };

    let liteloader_staged = if let Some(ver) = &req.liteloader {
        Some(LiteLoaderInstaller::stage(&stage_ctx, ver).await?)
    } else {
        None
    };

    let fabric_staged = if let Some(ver) = &req.fabric {
        Some(FabricInstaller::stage(&stage_ctx, ver).await?)
    } else {
        None
    };

    let legacy_fabric_staged = if let Some(ver) = &req.legacy_fabric {
        Some(LegacyFabricInstaller::stage(&stage_ctx, ver).await?)
    } else {
        None
    };

    let labymod_staged = if let Some(sel) = &req.labymod {
        Some(LabyModInstaller::stage(&stage_ctx, sel).await?)
    } else {
        None
    };

    // 3. Merge order: Vanilla -> OptiFine(if merged) -> Forge -> NeoForge -> Cleanroom -> LiteLoader -> Fabric -> LegacyFabric -> LabyMod
    let mut merged = vanilla_json;

    if let Some(staged) = optifine_staged {
        if !opti_fine_as_mod {
            merged = merge_version_json(staged.version_json, merged);
        }
    }

    if let Some(staged) = forge_staged {
        merged = merge_version_json(staged.version_json, merged);
    }

    if let Some(staged) = neoforge_staged {
        merged = merge_version_json(staged.version_json, merged);
    }

    if let Some(staged) = cleanroom_staged {
        merged = merge_version_json(staged.version_json, merged);
    }

    if let Some(staged) = liteloader_staged {
        merged = merge_version_json(staged.version_json, merged);
    }

    if let Some(staged) = fabric_staged {
        merged = merge_version_json(staged.version_json, merged);
    }

    if let Some(staged) = legacy_fabric_staged {
        merged = merge_version_json(staged.version_json, merged);
    }

    if let Some(staged) = labymod_staged {
        merged = merge_version_json(staged.version_json, merged);
    }

    merged.id = req.instance_name.clone();
    merged.inherits_from = None;

    // Strip LWJGL2 when LWJGL3 is present
    let has_lwjgl3 = merged.libraries.iter().any(|lib| {
        lib.name.starts_with("org.lwjgl:lwjgl:3") || lib.name.starts_with("org.lwjgl3:")
    });
    if has_lwjgl3 {
        merged.libraries.retain(|lib| {
            !lib.name.starts_with("org.lwjgl.lwjgl:lwjgl:2")
                && !lib.name.starts_with("org.lwjgl:lwjgl:2")
        });
    }

    // Copy vanilla jar to <name>.jar
    let staging_jar = staging_dir
        .join("versions")
        .join(&req.mc_version)
        .join(format!("{}.jar", req.mc_version));
    let target_jar = instance_dir.join(format!("{}.jar", req.instance_name));
    if staging_jar.exists() {
        std::fs::copy(&staging_jar, &target_jar)?;
    }

    // Write final version json with clientVersion injected
    let mut json_val = serde_json::to_value(&merged)?;
    json_val["clientVersion"] = serde_json::Value::String(req.mc_version.clone());
    let final_json_str = serde_json::to_string_pretty(&json_val)?;
    std::fs::write(
        instance_dir.join(format!("{}.json", req.instance_name)),
        final_json_str,
    )?;

    // 4. Merge staging libraries into real mc_folder libraries (skip existing)
    copy_dir_recursive_skip_existing(
        &staging_dir.join("libraries"),
        &env.mc_folder.join("libraries"),
    )?;

    // 5. Game directory resolution & mods
    let game_dir = resolve_game_dir(&env.mc_folder, &req.instance_name, &env.game_settings);
    let mods_dir = game_dir.join("mods");
    let resourcepacks_dir = game_dir.join("resourcepacks");
    std::fs::create_dir_all(&resourcepacks_dir)?;
    std::fs::create_dir_all(&mods_dir)?;

    if let InstallMode::Modify { remove_mod_files } = &req.mode {
        for rm_file in remove_mod_files {
            let f_path = mods_dir.join(rm_file);
            if f_path.exists() {
                let _ = std::fs::remove_file(f_path);
            }
            let disabled_path = mods_dir.join(format!("{rm_file}.disabled"));
            if disabled_path.exists() {
                let _ = std::fs::remove_file(disabled_path);
            }
        }
    }

    // Download extra_mods into staging mods
    if !req.extra_mods.is_empty() {
        let staging_mods = staging_dir.join("mods");
        std::fs::create_dir_all(&staging_mods)?;
        let mut specs = Vec::new();
        for extra in &req.extra_mods {
            let dest = staging_mods.join(&extra.file_name);
            let parsed_urls: Vec<Url> = extra
                .urls
                .iter()
                .filter_map(|u| Url::parse(u).ok())
                .collect();
            if !parsed_urls.is_empty() {
                specs.push(DownloadSpec {
                    urls: parsed_urls,
                    dest,
                    size: extra.size,
                    sha1: extra.sha1.clone(),
                    sha256: None,
                    md5: None,
                });
            }
        }
        let downloader = Downloader::new(env.client.clone(), env.net_settings.clone());
        downloader.download_many(specs, task_ctx).await?;
    }

    // Copy staging mods into game_dir/mods
    let staging_mods = staging_dir.join("mods");
    if staging_mods.is_dir() {
        copy_dir_recursive(&staging_mods, &mods_dir)?;
    }

    // 6. Repair instance (download remaining libraries and assets)
    repair_instance(
        &env.client,
        &env.net_settings,
        &env.mc_folder,
        &req.instance_name,
        task_ctx,
    )
    .await?;

    if let Ok(list) = mclr_game::scan_instances(&env.mc_folder)
        && let Some(found) = list.into_iter().find(|i| i.id == req.instance_name)
    {
        return Ok(found);
    }
    let (kind, loaders) = classify_instance(&merged, &req.mc_version);
    let modable = mclr_game::gamedir::is_modable_loaders(&loaders);
    Ok(InstanceInfo {
        id: req.instance_name.clone(),
        name: req.instance_name.clone(),
        folder: env.mc_folder.to_string_lossy().to_string(),
        path: instance_dir.to_string_lossy().to_string(),
        mc_version: req.mc_version.clone(),
        kind,
        loaders,
        state: InstanceState::Ok,
        config: InstanceConfig::default(),
        modable,
        ..Default::default()
    })
}
