//! Modpack override extraction and import pipeline.

use std::collections::HashMap;
use std::path::Path;

use mclr_core::TaskCenter;
use mclr_game::instance::{
    InstanceConfig, InstanceInfo, InstanceKind, InstanceState, scan_instances,
    validate_instance_name,
};
use mclr_loaders::{LoaderInstallContext, LoaderKind, install_with_loader};
use mclr_net::{DownloadSpec, Downloader};
use tracing::{debug, info};
use url::Url;

use super::inspect_modpack;
use crate::curseforge::CurseForgeClient;
use crate::error::CompError;

pub struct ModpackImportContext<'a> {
    pub archive_path: &'a Path,
    pub instance_name: &'a str,
    pub mc_folder: &'a Path,
    pub client: &'a reqwest::Client,
    pub net_settings: &'a mclr_core::NetSettings,
    pub java_settings: Option<&'a mclr_core::JavaSettings>,
    pub cf_client: Option<&'a CurseForgeClient>,
    pub task_ctx: Option<(&'a TaskCenter, &'a str)>,
    pub install_base_game: bool,
}

pub use mclr_core::safe_path::safe_join;

pub fn extract_modpack_overrides(
    archive_path: &Path,
    instance_dir: &Path,
    override_prefixes: &[String],
) -> Result<usize, CompError> {
    let file = std::fs::File::open(archive_path).map_err(CompError::Io)?;
    let mut zip = zip::ZipArchive::new(file).map_err(CompError::Zip)?;
    let mut extracted_count = 0;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(CompError::Zip)?;
        if entry.is_dir() {
            continue;
        }

        let name = entry.name().map_err(CompError::Zip)?.replace('\\', "/");

        for prefix in override_prefixes {
            let normalized_prefix = prefix.replace('\\', "/");
            let matched = if normalized_prefix.is_empty() {
                Some(name.as_str())
            } else {
                name.strip_prefix(&normalized_prefix)
            };

            if let Some(rel) = matched {
                let rel = rel.trim_start_matches('/');
                if rel.is_empty() {
                    continue;
                }

                // Security: reject traversal / absolute entries.
                let Some(target_path) = safe_join(instance_dir, rel) else {
                    debug!("Skipping unsafe path traversal entry: {}", rel);
                    continue;
                };

                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent).map_err(CompError::Io)?;
                }

                let mut out = std::fs::File::create(&target_path).map_err(CompError::Io)?;
                std::io::copy(&mut entry, &mut out).map_err(CompError::Io)?;
                extracted_count += 1;
                break;
            }
        }
    }

    Ok(extracted_count)
}

/// Starts a stage: switches the task's progress window and tells the UI what is running.
/// A missing task context (fixtures, offline import) is not an error.
async fn begin_stage(task_ctx: Option<(&TaskCenter, &str)>, base: f32, span: f32, stage: &str) {
    if let Some((tc, tid)) = task_ctx {
        let _ = tc.set_progress_window(tid, base, span).await;
        let mut params = HashMap::new();
        params.insert("stage".to_string(), stage.to_string());
        let _ = tc.update_title_params(tid, params).await;
    }
}

fn cf_file_destination_folder(file_name: &str) -> &'static str {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".jar") {
        "mods"
    } else if lower.contains("shader") {
        "shaderpacks"
    } else if lower.contains("resource") || lower.contains("texture") {
        "resourcepacks"
    } else {
        "mods"
    }
}

/// Imports a modpack into `<mc_folder>/versions/<instance_name>`.
///
/// Steps:
/// 1. Inspect archive (rejecting Quilt).
/// 2. If nested, extracts inner archive.
/// 3. Installs base game (with loader if specified; or stub version JSON if `install_base_game` is false).
/// 4. Forces `isolation = "on"` in `<instance_dir>/mclr.instance.json`.
/// 5. Resolves CurseForge file IDs if present.
/// 6. Downloads all files via Downloader.
/// 7. Extracts overrides into `instance_dir`.
///
/// A failed import removes the version directory it created, so a broken install cannot linger as
/// a half-written instance holding every file that did download.
pub async fn import_modpack(ctx: ModpackImportContext<'_>) -> Result<InstanceInfo, CompError> {
    let instance_dir = ctx.mc_folder.join("versions").join(ctx.instance_name);
    let existed = instance_dir.exists();

    let result = import_modpack_inner(ctx).await;

    if result.is_err() && !existed {
        let _ = tokio::fs::remove_dir_all(&instance_dir).await;
    }
    result
}

async fn import_modpack_inner(ctx: ModpackImportContext<'_>) -> Result<InstanceInfo, CompError> {
    let ModpackImportContext {
        archive_path,
        instance_name,
        mc_folder,
        client,
        net_settings,
        java_settings,
        cf_client,
        task_ctx,
        install_base_game,
    } = ctx;

    validate_instance_name(instance_name).map_err(CompError::Game)?;

    // Everything below reports its own 0.0..=1.0; `begin_stage` maps each stage into its slice
    // of the bar, so the visible progress never jumps back to 0 between stages.
    begin_stage(task_ctx, 0.0, 0.08, "inspect").await;
    let mut plan = inspect_modpack(archive_path)?;

    // If nested archive, extract it and work with the inner archive
    let (effective_archive_path, _temp_guard) =
        if let Some(nested_entry) = &plan.nested_archive_entry {
            let file = std::fs::File::open(archive_path).map_err(CompError::Io)?;
            let mut zip = zip::ZipArchive::new(file).map_err(CompError::Zip)?;
            let mut inner = zip.by_name(nested_entry).map_err(CompError::Zip)?;
            let mut temp = tempfile::Builder::new()
                .suffix(".zip")
                .tempfile()
                .map_err(CompError::Io)?;
            std::io::copy(&mut inner, &mut temp).map_err(CompError::Io)?;
            let path = temp.path().to_path_buf();
            (path, Some(temp))
        } else {
            (archive_path.to_path_buf(), None)
        };

    let instance_dir = mc_folder.join("versions").join(instance_name);
    std::fs::create_dir_all(&instance_dir).map_err(CompError::Io)?;

    // 1+3. The base game install (client jar, libraries, assets, loader installer) and the
    // CurseForge file-id resolution touch disjoint state — the install writes
    // `versions/<instance>`, `libraries/` and `assets/`, the metadata call writes nothing — so
    // the metadata round trip hides inside the install instead of being added to it.
    let cf_file_ids: Vec<u64> = plan.files.iter().filter_map(|f| f.cf_file_id).collect();

    let base_install = async {
        if !install_base_game {
            // Offline / fixture mode: write minimal valid instance version JSON if missing
            let json_path = instance_dir.join(format!("{instance_name}.json"));
            if !json_path.exists() {
                let dummy_json = serde_json::json!({
                    "id": instance_name,
                    "time": "2026-01-01T00:00:00+00:00",
                    "releaseTime": "2026-01-01T00:00:00+00:00",
                    "type": "release",
                    "mainClass": "net.minecraft.client.main.Main",
                    "inheritsFrom": plan.inspect.mc_version,
                });
                std::fs::write(&json_path, serde_json::to_string_pretty(&dummy_json)?)
                    .map_err(CompError::Io)?;
            }
            return Ok::<(), CompError>(());
        }

        begin_stage(task_ctx, 0.08, 0.62, "baseGame").await;
        match &plan.inspect.loader {
            Some(loader_name) => {
                let loader_kind = match loader_name.to_lowercase().as_str() {
                    "fabric" => Some(LoaderKind::Fabric),
                    "legacyfabric" => Some(LoaderKind::LegacyFabric),
                    "liteloader" => Some(LoaderKind::LiteLoader),
                    "labymod" => Some(LoaderKind::LabyMod),
                    "optifine" => Some(LoaderKind::OptiFine),
                    "forge" => Some(LoaderKind::Forge),
                    "neoforge" => Some(LoaderKind::NeoForge),
                    "cleanroom" => Some(LoaderKind::Cleanroom),
                    "quilt" => return Err(CompError::QuiltUnsupported),
                    _ => None,
                };

                if let Some(kind) = loader_kind {
                    let default_java = mclr_core::JavaSettings::default();
                    let java_cfg = java_settings.unwrap_or(&default_java);
                    let ctx = LoaderInstallContext {
                        client,
                        net_settings,
                        java_settings: java_cfg,
                        mc_folder: mc_folder.to_path_buf(),
                        mc_version: plan.inspect.mc_version.clone(),
                        instance_name: instance_name.to_string(),
                        loader_version: plan.inspect.loader_version.clone().unwrap_or_default(),
                        optifine_version: None,
                        task_ctx,
                    };
                    install_with_loader(kind, ctx).await?;
                } else {
                    mclr_game::install::install_vanilla(
                        client,
                        net_settings,
                        mc_folder,
                        &plan.inspect.mc_version,
                        instance_name,
                        task_ctx,
                    )
                    .await
                    .map_err(CompError::Game)?;
                }
            }
            None => {
                mclr_game::install::install_vanilla(
                    client,
                    net_settings,
                    mc_folder,
                    &plan.inspect.mc_version,
                    instance_name,
                    task_ctx,
                )
                .await
                .map_err(CompError::Game)?;
            }
        }
        Ok(())
    };

    let cf_resolve = async {
        if cf_file_ids.is_empty() {
            return Ok::<_, CompError>(Vec::new());
        }
        let cf = cf_client.ok_or(CompError::NotConfigured)?;
        cf.get_files_batch(&cf_file_ids).await
    };

    let (base_result, cf_result) = tokio::join!(base_install, cf_resolve);
    base_result?;
    let resolved_files = cf_result?;

    // 2. Always force isolation = "on" in mclr.instance.json
    let config_path = instance_dir.join("mclr.instance.json");
    let mut cfg = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|c| serde_json::from_str::<InstanceConfig>(&c).ok())
            .unwrap_or_default()
    } else {
        InstanceConfig::default()
    };
    cfg.isolation = "on".to_string();
    std::fs::write(&config_path, serde_json::to_string_pretty(&cfg)?).map_err(CompError::Io)?;

    // 3. Apply the resolved CurseForge metadata
    if !resolved_files.is_empty() {
        let mut resolved_map = HashMap::new();
        for rf in resolved_files {
            if let Ok(fid) = rf.id.parse::<u64>() {
                resolved_map.insert(fid, rf);
            }
        }
        for file in &mut plan.files {
            if let Some(rf) = file.cf_file_id.and_then(|fid| resolved_map.get(&fid)) {
                let subfolder = cf_file_destination_folder(&rf.file_name);
                file.rel_path = format!("{subfolder}/{}", rf.file_name);
                file.urls = rf.download_urls.clone();
                file.sha1 = rf.hashes.sha1.clone();
                file.size = Some(rf.size);
            }
        }
    }

    // 4. Download modpack files
    let mut specs = Vec::new();
    for file in &plan.files {
        if file.rel_path.is_empty() {
            continue;
        }
        let Some(dest) = safe_join(&instance_dir, file.rel_path.as_str()) else {
            debug!("Skipping unsafe modpack file path: {}", file.rel_path);
            continue;
        };
        let urls: Vec<Url> = file
            .urls
            .iter()
            .filter_map(|u| Url::parse(u).ok())
            .collect();
        if urls.is_empty() {
            continue;
        }
        specs.push(DownloadSpec {
            urls,
            dest,
            size: file.size,
            sha1: file.sha1.clone(),
            sha256: None,
            md5: None,
        });
    }

    if !specs.is_empty() {
        begin_stage(task_ctx, 0.70, 0.28, "files").await;
        info!(
            "Downloading {} modpack files into {}",
            specs.len(),
            instance_dir.display()
        );
        let downloader = Downloader::new(client.clone(), net_settings.clone());
        downloader
            .download_many(specs, task_ctx)
            .await
            .map_err(CompError::Net)?;
    }

    // 5. Extract overrides into instance_dir
    // ZIP extraction is CPU/disk bound synchronous work: keep it off the async
    // runtime's worker threads so downloads and IPC stay responsive.
    begin_stage(task_ctx, 0.98, 0.02, "overrides").await;
    let extract_archive = effective_archive_path;
    let extract_dir = instance_dir.clone();
    let extract_prefixes = std::mem::take(&mut plan.override_prefixes);
    let extracted = tokio::task::spawn_blocking(move || {
        extract_modpack_overrides(&extract_archive, &extract_dir, &extract_prefixes)
    })
    .await
    .map_err(|e| CompError::Io(std::io::Error::other(e)))??;
    info!(
        "Extracted {extracted} override files into {}",
        instance_dir.display()
    );
    if let Some((tc, tid)) = task_ctx {
        let _ = tc.update_progress(tid, 1.0, 0).await;
    }

    // 6. Return refreshed InstanceInfo
    let scan_folder = mc_folder.to_path_buf();
    let instances = tokio::task::spawn_blocking(move || scan_instances(&scan_folder))
        .await
        .map_err(|e| mclr_game::GameError::Io(std::io::Error::other(e)))??;
    if let Some(info) = instances.into_iter().find(|i| i.id == instance_name) {
        Ok(info)
    } else {
        Ok(InstanceInfo {
            id: instance_name.to_string(),
            name: instance_name.to_string(),
            folder: mc_folder.to_string_lossy().to_string(),
            path: instance_dir.to_string_lossy().to_string(),
            mc_version: plan.inspect.mc_version,
            kind: InstanceKind::Vanilla,
            loaders: plan.inspect.loader.into_iter().collect(),
            state: InstanceState::Ok,
            config: cfg,
            ..Default::default()
        })
    }
}
