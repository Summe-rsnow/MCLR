#![allow(clippy::collapsible_if)]

//! Forge, NeoForge, and Cleanroom mod loader installers.

use crate::error::LoaderError;
use crate::{LoaderInstallContext, LoaderInstaller, LoaderKind, LoaderVersionEntry};
use async_trait::async_trait;
use mclr_core::paths;
use mclr_core::settings::SourcePriority;
use mclr_game::install::maven_coordinate_to_path;
use mclr_game::instance::InstanceInfo;
use mclr_game::launch::version_java_constraint;
use mclr_game::version::{Library, VersionJson};
use mclr_game::{scan_java, select_java};
use mclr_net::{DownloadSpec, Downloader, SourceKind, SourceOrigin, rewrite_url};
use serde::Deserialize;
use sha1::Digest;
use std::cmp::Ordering;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use url::Url;

/// Compares two loader version strings descending (e.g. "47.3.0" > "47.0.1", "20.4.237" > "20.4.5").
pub fn compare_loader_versions(a: &str, b: &str) -> Ordering {
    let parts_a: Vec<&str> = a.split(['.', '-', '_']).collect();
    let parts_b: Vec<&str> = b.split(['.', '-', '_']).collect();

    let max_len = parts_a.len().max(parts_b.len());
    for i in 0..max_len {
        let seg_a = parts_a.get(i).copied().unwrap_or("");
        let seg_b = parts_b.get(i).copied().unwrap_or("");

        let num_a = seg_a.parse::<u64>().ok();
        let num_b = seg_b.parse::<u64>().ok();

        match (num_a, num_b) {
            (Some(na), Some(nb)) => {
                if na != nb {
                    return na.cmp(&nb);
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => {
                if seg_a != seg_b {
                    return seg_a.cmp(seg_b);
                }
            }
        }
    }
    Ordering::Equal
}

fn is_file_valid(path: &Path, expected_size: Option<u64>, expected_sha1: Option<&str>) -> bool {
    if !path.exists() {
        return false;
    }
    if let Some(expected_len) = expected_size {
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() != expected_len {
                return false;
            }
        } else {
            return false;
        }
    }
    if let Some(expected_hash) = expected_sha1 {
        if let Ok(bytes) = std::fs::read(path) {
            let actual_hash = hex::encode(sha1::Sha1::digest(&bytes));
            if !actual_hash.eq_ignore_ascii_case(expected_hash) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn build_library_urls(original_url: &str, rel_path: &str, priority: SourcePriority) -> Vec<Url> {
    let mut mirror_urls = Vec::new();
    let mut official_urls = Vec::new();

    // Direct BMCLAPI mirror for maven paths:
    let bmclapi_maven = format!("https://bmclapi2.bangbang93.com/maven/{}", rel_path);
    if let Ok(u) = Url::parse(&bmclapi_maven) {
        mirror_urls.push(u);
    }

    if let Ok(orig) = Url::parse(original_url) {
        for (origin, u) in rewrite_url(&orig, SourceKind::Libraries) {
            match origin {
                SourceOrigin::Mirror => {
                    if !mirror_urls.contains(&u) {
                        mirror_urls.push(u);
                    }
                }
                SourceOrigin::Official => {
                    if !official_urls.contains(&u) {
                        official_urls.push(u);
                    }
                }
            }
        }
    }

    match priority {
        SourcePriority::OfficialFirst => {
            let mut res = official_urls;
            res.extend(mirror_urls);
            res
        }
        _ => {
            let mut res = mirror_urls;
            res.extend(official_urls);
            res
        }
    }
}

async fn download_installer_jar(
    client: &reqwest::Client,
    net_settings: &mclr_core::NetSettings,
    urls: Vec<Url>,
    dest: PathBuf,
    task_ctx: Option<(&mclr_core::TaskCenter, &str)>,
) -> Result<PathBuf, LoaderError> {
    if dest.exists() {
        if let Ok(file) = std::fs::File::open(&dest) {
            if zip::ZipArchive::new(file).is_ok() {
                debug!("Reusing cached installer jar at {}", dest.display());
                return Ok(dest);
            }
        }
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let spec = DownloadSpec {
        urls,
        dest: dest.clone(),
        size: None,
        sha1: None,
        sha256: None,
        md5: None,
    };

    let downloader = Downloader::new(client.clone(), net_settings.clone());
    downloader.download(spec, task_ctx).await?;

    Ok(dest)
}

fn extract_maven_entries_from_zip(
    archive: &mut zip::ZipArchive<std::fs::File>,
    mc_folder: &Path,
) -> Result<(), LoaderError> {
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();
        if name.starts_with("maven/") && !file.is_dir() {
            let rel_path = &name["maven/".len()..];
            let dest = mc_folder
                .join("libraries")
                .join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut file, &mut out)?;
        }
    }
    Ok(())
}

async fn predownload_installer_libraries(
    client: &reqwest::Client,
    net_settings: &mclr_core::NetSettings,
    archive: &mut zip::ZipArchive<std::fs::File>,
    mc_folder: &Path,
    task_ctx: Option<(&mclr_core::TaskCenter, &str)>,
) -> Result<(), LoaderError> {
    let mut all_libs = Vec::new();

    // From install_profile.json
    if let Ok(mut file) = archive.by_name("install_profile.json") {
        let mut content = String::new();
        if file.read_to_string(&mut content).is_ok() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = val.get("libraries").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Ok(lib) = serde_json::from_value::<Library>(item.clone()) {
                            all_libs.push(lib);
                        }
                    }
                }
            }
        }
    }

    // From version.json
    if let Ok(mut file) = archive.by_name("version.json") {
        let mut content = String::new();
        if file.read_to_string(&mut content).is_ok() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(arr) = val.get("libraries").and_then(|v| v.as_array()) {
                    for item in arr {
                        if let Ok(lib) = serde_json::from_value::<Library>(item.clone()) {
                            all_libs.push(lib);
                        }
                    }
                }
            }
        }
    }

    let mut specs = Vec::new();
    for lib in all_libs {
        if let Some(downloads) = &lib.downloads {
            if let Some(art) = &downloads.artifact {
                let rel_path = art
                    .path
                    .clone()
                    .unwrap_or_else(|| maven_coordinate_to_path(&lib.name));
                let dest = mc_folder
                    .join("libraries")
                    .join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
                if !is_file_valid(&dest, art.size, art.sha1.as_deref()) {
                    let mut urls = Vec::new();
                    if let Some(u_str) = &art.url {
                        urls.extend(build_library_urls(
                            u_str,
                            &rel_path,
                            net_settings.source_priority,
                        ));
                    }
                    if !urls.is_empty() {
                        specs.push(DownloadSpec {
                            urls,
                            dest,
                            size: art.size,
                            sha1: art.sha1.clone(),
                            sha256: None,
                            md5: None,
                        });
                    }
                }
                continue;
            }
        }

        let rel_path = maven_coordinate_to_path(&lib.name);
        let dest = mc_folder
            .join("libraries")
            .join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !dest.exists() {
            let base_url = lib
                .url
                .as_deref()
                .unwrap_or("https://libraries.minecraft.net/");
            let joined = format!("{}/{}", base_url.trim_end_matches('/'), rel_path);
            let urls = build_library_urls(&joined, &rel_path, net_settings.source_priority);
            if !urls.is_empty() {
                specs.push(DownloadSpec {
                    urls,
                    dest,
                    size: None,
                    sha1: None,
                    sha256: None,
                    md5: None,
                });
            }
        }
    }

    if !specs.is_empty() {
        debug!(
            "Pre-downloading {} libraries for Forge installer",
            specs.len()
        );
        let downloader = Downloader::new(client.clone(), net_settings.clone());
        downloader.download_many(specs, task_ctx).await?;
    }

    Ok(())
}

/// Runs the official headless Forge/NeoForge/Cleanroom installer jar with `--installClient <mc_folder>`.
///
/// Executes with `CREATE_NO_WINDOW` on Windows.
/// If execution fails with `IllegalAccess`, retries with `--add-exports cpw.mods.bootstraplauncher/cpw.mods.bootstraplauncher=ALL-UNNAMED`.
pub fn run_headless_installer_jar(
    java_exe: &Path,
    installer_jar: &Path,
    mc_folder: &Path,
) -> Result<String, LoaderError> {
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    let mut cmd = Command::new(java_exe);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

    cmd.arg("-jar")
        .arg(installer_jar)
        .arg("--installClient")
        .arg(mc_folder);

    let output = cmd.output().map_err(LoaderError::Io)?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        return Ok(stdout);
    }

    // Check if IllegalAccess error occurred, requiring --add-exports
    if stderr.contains("IllegalAccess") || stdout.contains("IllegalAccess") {
        info!("Installer encountered IllegalAccess; retrying with --add-exports");
        let mut retry_cmd = Command::new(java_exe);
        #[cfg(windows)]
        retry_cmd.creation_flags(0x08000000);

        retry_cmd
            .arg("--add-exports")
            .arg("cpw.mods.bootstraplauncher/cpw.mods.bootstraplauncher=ALL-UNNAMED")
            .arg("-jar")
            .arg(installer_jar)
            .arg("--installClient")
            .arg(mc_folder);

        let retry_output = retry_cmd.output().map_err(LoaderError::Io)?;
        let retry_stdout = String::from_utf8_lossy(&retry_output.stdout).to_string();
        let retry_stderr = String::from_utf8_lossy(&retry_output.stderr).to_string();

        if retry_output.status.success() {
            return Ok(retry_stdout);
        }

        return Err(LoaderError::InstallerFailed(format!(
            "Installer retry failed (code {:?}): {}\n{}",
            retry_output.status.code(),
            retry_stdout,
            retry_stderr
        )));
    }

    Err(LoaderError::InstallerFailed(format!(
        "Installer failed (code {:?}): {}\n{}",
        output.status.code(),
        stdout,
        stderr
    )))
}

fn find_and_load_generated_forge_json(
    mc_folder: &Path,
    installer_path: &Path,
    known_versions_before: &[String],
) -> Result<VersionJson, LoaderError> {
    let versions_dir = mc_folder.join("versions");

    // 1. Check for newly created directories in versions/
    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        let mut new_dirs = Vec::new();
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !known_versions_before.contains(&name) {
                        new_dirs.push((name, entry.path()));
                    }
                }
            }
        }

        for (name, path) in new_dirs {
            let json_file = path.join(format!("{}.json", name));
            if json_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&json_file) {
                    if let Ok(parsed) = serde_json::from_str::<VersionJson>(&content) {
                        // Clean up the temporary generated version directory if it differs
                        let _ = std::fs::remove_dir_all(&path);
                        return Ok(parsed);
                    }
                }
            }
        }
    }

    // 2. Scan existing directories in versions/ that look like forge/neoforge
    if let Ok(entries) = std::fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let lower = name.to_lowercase();
                    if lower.contains("forge") || lower.contains("cleanroom") {
                        let json_file = entry.path().join(format!("{}.json", name));
                        if json_file.exists() {
                            if let Ok(content) = std::fs::read_to_string(&json_file) {
                                if let Ok(parsed) = serde_json::from_str::<VersionJson>(&content) {
                                    return Ok(parsed);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Fallback: read version.json directly from the installer jar
    let file = std::fs::File::open(installer_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    if let Ok(mut vfile) = archive.by_name("version.json") {
        let mut content = String::new();
        vfile.read_to_string(&mut content)?;
        let parsed = serde_json::from_str::<VersionJson>(&content)?;
        return Ok(parsed);
    }

    // 4. Fallback: read versionInfo from install_profile.json
    if let Ok(mut ip_file) = archive.by_name("install_profile.json") {
        let mut content = String::new();
        ip_file.read_to_string(&mut content)?;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(v_info) = val.get("versionInfo") {
                let parsed = serde_json::from_value::<VersionJson>(v_info.clone())?;
                return Ok(parsed);
            }
        }
    }

    Err(LoaderError::Metadata(
        "Could not find generated Forge/NeoForge version.json".to_string(),
    ))
}

// ============================================================================
// Forge Installer
// ============================================================================

impl ForgeInstaller {
    pub async fn stage(
        ctx: &crate::pipeline::StageCtx<'_>,
        loader_version: &str,
    ) -> Result<crate::pipeline::Staged, LoaderError> {
        let cache_file = paths::cache_dir().join(format!(
            "forge-{}-{}-installer.jar",
            ctx.mc_version, loader_version
        ));
        let urls = vec![
            Url::parse(&format!(
                "https://bmclapi2.bangbang93.com/forge/download?mcversion={}&version={}&category=installer&format=jar",
                ctx.mc_version, loader_version
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
            Url::parse(&format!(
                "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/{mc}-{ver}/forge-{mc}-{ver}-installer.jar",
                mc = ctx.mc_version,
                ver = loader_version
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
            Url::parse(&format!(
                "https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{ver}/forge-{mc}-{ver}-installer.jar",
                mc = ctx.mc_version,
                ver = loader_version
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
        ];

        let installer_path =
            download_installer_jar(ctx.client, ctx.net_settings, urls, cache_file, ctx.task_ctx)
                .await?;

        let file = std::fs::File::open(&installer_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let mut install_profile_val: Option<serde_json::Value> = None;
        if let Ok(mut ip_file) = archive.by_name("install_profile.json") {
            let mut content = String::new();
            if ip_file.read_to_string(&mut content).is_ok() {
                install_profile_val = serde_json::from_str::<serde_json::Value>(&content).ok();
            }
        }

        let is_legacy = install_profile_val.as_ref().is_some_and(|val| {
            val.get("install").and_then(|v| v.as_object()).is_some()
                && val.get("versionInfo").and_then(|v| v.as_object()).is_some()
        });

        let version_json = if is_legacy {
            let val = install_profile_val.as_ref().unwrap();
            let install_obj = val["install"].as_object().unwrap();
            let file_path = install_obj
                .get("filePath")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let maven_path_coord = install_obj
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let rel_maven_path = maven_coordinate_to_path(maven_path_coord);
            let dest = ctx
                .staging_dir
                .join("libraries")
                .join(rel_maven_path.replace('/', std::path::MAIN_SEPARATOR_STR));
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let trimmed = file_path.trim_start_matches('/');
            let name_to_find = if archive.by_name(file_path).is_ok() {
                file_path
            } else {
                trimmed
            };
            if let Ok(mut zip_entry) = archive.by_name(name_to_find) {
                let mut out = std::fs::File::create(&dest)?;
                std::io::copy(&mut zip_entry, &mut out)?;
            }

            let forge_json: VersionJson = serde_json::from_value(val["versionInfo"].clone())?;
            forge_json
        } else {
            extract_maven_entries_from_zip(&mut archive, ctx.staging_dir)?;
            predownload_installer_libraries(
                ctx.client,
                ctx.net_settings,
                &mut archive,
                ctx.staging_dir,
                ctx.task_ctx,
            )
            .await?;

            let dummy_json = VersionJson::default();
            let constraint = version_java_constraint(&dummy_json, ctx.mc_version);
            let installed_javas = scan_java(&ctx.java_settings.custom);
            let java_inst = select_java(&installed_javas, &constraint, ctx.java_settings)
                .ok_or(LoaderError::JavaNotFound)?;
            let java_exe = PathBuf::from(&java_inst.path);

            let mut known_versions = Vec::new();
            if let Ok(entries) = std::fs::read_dir(ctx.staging_dir.join("versions")) {
                for entry in entries.flatten() {
                    known_versions.push(entry.file_name().to_string_lossy().to_string());
                }
            }

            run_headless_installer_jar(&java_exe, &installer_path, ctx.staging_dir)?;

            find_and_load_generated_forge_json(ctx.staging_dir, &installer_path, &known_versions)?
        };

        Ok(crate::pipeline::Staged {
            version_json,
            mods: Vec::new(),
        })
    }
}

pub struct ForgeInstaller;

#[derive(Debug, Deserialize)]
struct BmclapiForgeEntry {
    pub version: String,
    #[serde(default)]
    pub modified: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
    #[serde(default)]
    pub files: Option<Vec<BmclapiForgeFile>>,
}

#[derive(Debug, Deserialize)]
struct BmclapiForgeFile {
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

#[async_trait]
impl LoaderInstaller for ForgeInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let clean_mc = mc_version.replace('-', "_");
        let url = format!("https://bmclapi2.bangbang93.com/forge/minecraft/{clean_mc}");
        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| LoaderError::Net(e.into()))?;
        if !res.status().is_success() {
            return Ok(Vec::new());
        }

        let raw_entries: Vec<BmclapiForgeEntry> =
            res.json().await.map_err(|e| LoaderError::Net(e.into()))?;

        let mut entries: Vec<LoaderVersionEntry> = raw_entries
            .into_iter()
            .filter(|e| {
                if let Some(files) = &e.files {
                    if files.is_empty() {
                        return true;
                    }
                    files.iter().any(|f| {
                        f.category.as_deref() == Some("installer")
                            && f.format.as_deref() == Some("jar")
                    })
                } else {
                    true
                }
            })
            .map(|e| LoaderVersionEntry {
                loader: LoaderKind::Forge,
                mc_version: mc_version.to_string(),
                version: e.version,
                stable: e.branch.as_deref().unwrap_or("").is_empty(),
                release_time: e.modified,
            })
            .collect();

        entries.sort_by(|a, b| compare_loader_versions(&b.version, &a.version));
        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        crate::install_with_loader(LoaderKind::Forge, ctx).await
    }
}

// ============================================================================
// NeoForge Installer
// ============================================================================

impl NeoForgeInstaller {
    pub async fn stage(
        ctx: &crate::pipeline::StageCtx<'_>,
        loader_version: &str,
    ) -> Result<crate::pipeline::Staged, LoaderError> {
        let cache_file = paths::cache_dir().join(format!(
            "neoforge-{}-{}-installer.jar",
            ctx.mc_version, loader_version
        ));
        let ver = loader_version;
        let mut urls = vec![
            Url::parse(&format!(
                "https://bmclapi2.bangbang93.com/neoforge/version/{ver}/download/installer.jar"
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
            Url::parse(&format!(
                "https://bmclapi2.bangbang93.com/maven/net/neoforged/neoforge/{ver}/neoforge-{ver}-installer.jar"
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
            Url::parse(&format!(
                "https://maven.neoforged.net/releases/net/neoforged/neoforge/{ver}/neoforge-{ver}-installer.jar"
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
        ];

        if ctx.mc_version == "1.20.1" {
            if let Ok(legacy_u) = Url::parse(&format!(
                "https://maven.neoforged.net/releases/net/neoforged/forge/{ver}/forge-{ver}-installer.jar"
            )) {
                urls.push(legacy_u);
            }
        }

        let installer_path =
            download_installer_jar(ctx.client, ctx.net_settings, urls, cache_file, ctx.task_ctx)
                .await?;

        let file = std::fs::File::open(&installer_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        extract_maven_entries_from_zip(&mut archive, ctx.staging_dir)?;
        predownload_installer_libraries(
            ctx.client,
            ctx.net_settings,
            &mut archive,
            ctx.staging_dir,
            ctx.task_ctx,
        )
        .await?;

        let dummy_json = VersionJson::default();
        let constraint = version_java_constraint(&dummy_json, ctx.mc_version);
        let installed_javas = scan_java(&ctx.java_settings.custom);
        let java_inst = select_java(&installed_javas, &constraint, ctx.java_settings)
            .ok_or(LoaderError::JavaNotFound)?;
        let java_exe = PathBuf::from(&java_inst.path);

        let mut known_versions = Vec::new();
        if let Ok(entries) = std::fs::read_dir(ctx.staging_dir.join("versions")) {
            for entry in entries.flatten() {
                known_versions.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        run_headless_installer_jar(&java_exe, &installer_path, ctx.staging_dir)?;

        let neoforge_json =
            find_and_load_generated_forge_json(ctx.staging_dir, &installer_path, &known_versions)?;

        Ok(crate::pipeline::Staged {
            version_json: neoforge_json,
            mods: Vec::new(),
        })
    }
}

pub struct NeoForgeInstaller;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum NeoForgeApiEntry {
    Object { version: String },
    String(String),
}

impl NeoForgeApiEntry {
    fn version(&self) -> &str {
        match self {
            NeoForgeApiEntry::Object { version } => version.as_str(),
            NeoForgeApiEntry::String(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct NeoForgeMavenVersions {
    #[serde(default)]
    versions: Vec<String>,
}

#[async_trait]
impl LoaderInstaller for NeoForgeInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let bmclapi_url = format!("https://bmclapi2.bangbang93.com/neoforge/list/{mc_version}");
        let mut version_strings: Vec<String> = Vec::new();

        if let Ok(res) = client.get(&bmclapi_url).send().await {
            if res.status().is_success() {
                if let Ok(list) = res.json::<Vec<NeoForgeApiEntry>>().await {
                    for item in list {
                        version_strings.push(item.version().to_string());
                    }
                }
            }
        }

        // Fallback to official maven if empty
        if version_strings.is_empty() {
            let maven_url =
                "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";
            if let Ok(res) = client.get(maven_url).send().await {
                if res.status().is_success() {
                    if let Ok(resp) = res.json::<NeoForgeMavenVersions>().await {
                        let clean_mc = mc_version.strip_prefix("1.").unwrap_or(mc_version);
                        let prefix1 = format!("{}.", clean_mc);
                        let prefix2 = format!("{}-", mc_version);
                        let prefix3 = format!("{}.", mc_version);

                        for v in resp.versions {
                            if v.starts_with(&prefix1)
                                || v.starts_with(&prefix2)
                                || v.starts_with(&prefix3)
                            {
                                version_strings.push(v);
                            }
                        }
                    }
                }
            }
        }

        let mut entries: Vec<LoaderVersionEntry> = version_strings
            .into_iter()
            .map(|v| {
                let is_stable = !v.to_lowercase().contains("beta")
                    && !v.to_lowercase().contains("alpha")
                    && !v.to_lowercase().contains("preview");
                LoaderVersionEntry {
                    loader: LoaderKind::NeoForge,
                    mc_version: mc_version.to_string(),
                    version: v,
                    stable: is_stable,
                    release_time: None,
                }
            })
            .collect();

        entries.sort_by(|a, b| compare_loader_versions(&b.version, &a.version));
        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        crate::install_with_loader(LoaderKind::NeoForge, ctx).await
    }
}

// ============================================================================
// Cleanroom Installer
// ============================================================================

impl CleanroomInstaller {
    pub async fn stage(
        ctx: &crate::pipeline::StageCtx<'_>,
        loader_version: &str,
    ) -> Result<crate::pipeline::Staged, LoaderError> {
        let cache_file = paths::cache_dir().join(format!(
            "cleanroom-{}-{}-installer.jar",
            ctx.mc_version, loader_version
        ));
        let ver = loader_version;
        let urls = vec![
            Url::parse(&format!(
                "https://github.com/CleanroomMC/Cleanroom/releases/download/{ver}/cleanroom-{ver}-installer.jar"
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
            Url::parse(&format!(
                "https://github.com/CleanroomMC/Cleanroom/releases/download/v{ver}/cleanroom-{ver}-installer.jar"
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
            Url::parse(&format!(
                "https://github.com/CleanroomMC/Cleanroom/releases/download/{ver}/Cleanroom-{ver}-installer.jar"
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
            Url::parse(&format!(
                "https://github.com/CleanroomMC/Cleanroom/releases/download/v{ver}/Cleanroom-{ver}-installer.jar"
            ))
            .map_err(|e| LoaderError::Metadata(e.to_string()))?,
        ];

        let installer_path =
            download_installer_jar(ctx.client, ctx.net_settings, urls, cache_file, ctx.task_ctx)
                .await?;

        let file = std::fs::File::open(&installer_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        extract_maven_entries_from_zip(&mut archive, ctx.staging_dir)?;
        predownload_installer_libraries(
            ctx.client,
            ctx.net_settings,
            &mut archive,
            ctx.staging_dir,
            ctx.task_ctx,
        )
        .await?;

        let dummy_json = VersionJson::default();
        let constraint = version_java_constraint(&dummy_json, ctx.mc_version);
        let installed_javas = scan_java(&ctx.java_settings.custom);
        let java_inst = select_java(&installed_javas, &constraint, ctx.java_settings)
            .ok_or(LoaderError::JavaNotFound)?;
        let java_exe = PathBuf::from(&java_inst.path);

        let mut known_versions = Vec::new();
        if let Ok(entries) = std::fs::read_dir(ctx.staging_dir.join("versions")) {
            for entry in entries.flatten() {
                known_versions.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        run_headless_installer_jar(&java_exe, &installer_path, ctx.staging_dir)?;

        let cleanroom_json =
            find_and_load_generated_forge_json(ctx.staging_dir, &installer_path, &known_versions)?;

        Ok(crate::pipeline::Staged {
            version_json: cleanroom_json,
            mods: Vec::new(),
        })
    }
}

pub struct CleanroomInstaller;

const CLEANROOM_FALLBACK_VERSIONS: &[&str] = &[
    "0.7.12-alpha",
    "0.7.11-alpha",
    "0.7.10-alpha",
    "0.3.1-alpha",
    "0.3.0-alpha",
    "0.2.4-alpha",
    "0.2.3-alpha",
    "0.2.2-alpha",
    "0.2.1-alpha",
    "0.2.0-alpha",
];

#[derive(Debug, Deserialize)]
struct GithubRelease {
    pub tag_name: String,
    pub prerelease: bool,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub assets: Vec<GithubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubReleaseAsset {
    pub name: String,
}

#[async_trait]
impl LoaderInstaller for CleanroomInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        if mc_version != "1.12.2" {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let gh_url = "https://api.github.com/repos/CleanroomMC/Cleanroom/releases";
        if let Ok(res) = client
            .get(gh_url)
            .header(reqwest::header::USER_AGENT, "MCLR")
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(releases) = res.json::<Vec<GithubRelease>>().await {
                    for r in releases {
                        let has_installer =
                            r.assets.iter().any(|a| a.name.ends_with("-installer.jar"));
                        if has_installer || r.assets.is_empty() {
                            let clean_ver = r.tag_name.trim_start_matches('v').to_string();
                            entries.push(LoaderVersionEntry {
                                loader: LoaderKind::Cleanroom,
                                mc_version: mc_version.to_string(),
                                version: clean_ver,
                                stable: !r.prerelease,
                                release_time: r.published_at,
                            });
                        }
                    }
                }
            }
        }

        // Fallback if empty
        if entries.is_empty() {
            for v in CLEANROOM_FALLBACK_VERSIONS {
                entries.push(LoaderVersionEntry {
                    loader: LoaderKind::Cleanroom,
                    mc_version: mc_version.to_string(),
                    version: v.to_string(),
                    stable: !v.contains("alpha") && !v.contains("beta"),
                    release_time: None,
                });
            }
        }

        entries.sort_by(|a, b| compare_loader_versions(&b.version, &a.version));
        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        crate::install_with_loader(LoaderKind::Cleanroom, ctx).await
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_compare_loader_versions() {
        assert_eq!(
            compare_loader_versions("47.3.0", "47.0.1"),
            Ordering::Greater
        );
        assert_eq!(compare_loader_versions("47.0.1", "47.0.1"), Ordering::Equal);
        assert_eq!(compare_loader_versions("47.0.0", "47.0.1"), Ordering::Less);
        assert_eq!(
            compare_loader_versions("20.4.237-beta", "20.4.5-beta"),
            Ordering::Greater
        );
    }

    #[test]
    fn test_legacy_forge_install_profile_extraction_and_merging() {
        let temp = tempdir().unwrap();
        let mc_folder = temp.path();

        // 1. Create vanilla version json
        let versions_dir = mc_folder.join("versions").join("1.12.2");
        std::fs::create_dir_all(&versions_dir).unwrap();
        let vanilla_json = serde_json::json!({
            "id": "1.12.2",
            "time": "2017-09-18T08:39:46+00:00",
            "releaseTime": "2017-09-18T08:39:46+00:00",
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "minecraftArguments": "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --assetsDir ${assets_root} --assetIndex ${assets_index_name} --uuid ${auth_uuid} --accessToken ${auth_access_token} --userType ${user_type} --versionType ${version_type}",
            "libraries": [
                {
                    "name": "com.mojang:patchy:1.1"
                }
            ]
        });
        std::fs::write(
            versions_dir.join("1.12.2.json"),
            serde_json::to_string(&vanilla_json).unwrap(),
        )
        .unwrap();

        // 2. Build mock legacy installer zip in memory
        let zip_bytes = {
            let mut buf = std::io::Cursor::new(Vec::new());
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            // Add install_profile.json
            zip.start_file("install_profile.json", options).unwrap();
            let install_profile = serde_json::json!({
                "install": {
                    "profileName": "forge",
                    "target": "1.12.2-forge1.12.2-14.23.5.2860",
                    "path": "net.minecraftforge:forge:1.12.2-14.23.5.2860",
                    "version": "forge 14.23.5.2860",
                    "filePath": "forge-1.12.2-14.23.5.2860-universal.jar",
                    "minecraft": "1.12.2"
                },
                "versionInfo": {
                    "id": "1.12.2-forge1.12.2-14.23.5.2860",
                    "time": "2020-01-01T00:00:00Z",
                    "releaseTime": "2020-01-01T00:00:00Z",
                    "type": "release",
                    "mainClass": "net.minecraft.launchwrapper.Launch",
                    "minecraftArguments": "--username ${auth_player_name} --tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker",
                    "libraries": [
                        {
                            "name": "net.minecraftforge:forge:1.12.2-14.23.5.2860"
                        }
                    ]
                }
            });
            zip.write_all(serde_json::to_string(&install_profile).unwrap().as_bytes())
                .unwrap();

            // Add forge-1.12.2-14.23.5.2860-universal.jar
            zip.start_file("forge-1.12.2-14.23.5.2860-universal.jar", options)
                .unwrap();
            zip.write_all(b"mock universal forge jar content").unwrap();

            zip.finish().unwrap();
            buf.into_inner()
        };

        // 3. Inspect and extract legacy forge
        let cursor = std::io::Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();

        let mut ip_content = String::new();
        {
            let mut ip_file = archive.by_name("install_profile.json").unwrap();
            ip_file.read_to_string(&mut ip_content).unwrap();
        }
        let ip_val: serde_json::Value = serde_json::from_str(&ip_content).unwrap();

        let install_obj = ip_val["install"].as_object().unwrap();
        let file_path = install_obj["filePath"].as_str().unwrap();
        let maven_path_coord = install_obj["path"].as_str().unwrap();

        let rel_maven_path = maven_coordinate_to_path(maven_path_coord);
        let dest = mc_folder
            .join("libraries")
            .join(rel_maven_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();

        let mut jar_entry = archive.by_name(file_path).unwrap();
        let mut out = std::fs::File::create(&dest).unwrap();
        std::io::copy(&mut jar_entry, &mut out).unwrap();

        assert!(dest.exists());
        assert_eq!(
            std::fs::read_to_string(&dest).unwrap(),
            "mock universal forge jar content"
        );

        // 4. Merge version json
        let forge_json: VersionJson =
            serde_json::from_value(ip_val["versionInfo"].clone()).unwrap();
        let vanilla_v_json: VersionJson = serde_json::from_value(vanilla_json).unwrap();
        let mut merged = mclr_game::version::merge_version_json(forge_json, vanilla_v_json);
        merged.id = "test-instance".to_string();
        merged.inherits_from = None;

        assert_eq!(merged.id, "test-instance");
        assert_eq!(merged.inherits_from, None);
        assert_eq!(
            merged.main_class,
            Some("net.minecraft.launchwrapper.Launch".to_string())
        );
    }

    #[test]
    fn test_run_headless_installer_jar() {
        // If java is found on system, run a mock installer jar that exits 0
        let javas = scan_java(&[]);
        if let Some(java) = javas.first() {
            let temp = tempdir().unwrap();
            let java_exe = PathBuf::from(&java.path);
            let installer_jar = temp.path().join("mock-installer.jar");

            // Build a valid executable jar with Main-Class in MANIFEST.MF
            // We can compile a tiny Java class if javac is available
            let java_home = PathBuf::from(&java.home);
            let javac_exe = java_home.join("bin").join("javac.exe");

            if javac_exe.exists() {
                let java_src = temp.path().join("MockInstaller.java");
                std::fs::write(
                    &java_src,
                    r#"public class MockInstaller {
                        public static void main(String[] args) {
                            System.out.println("Installer ran with args: " + java.util.Arrays.toString(args));
                        }
                    }"#,
                )
                .unwrap();

                let mut javac_cmd = std::process::Command::new(&javac_exe);
                javac_cmd.arg(&java_src);
                let javac_res = javac_cmd.output();
                if let Ok(out) = javac_res {
                    if out.status.success() {
                        let class_bytes =
                            std::fs::read(temp.path().join("MockInstaller.class")).unwrap();

                        let mut file = std::fs::File::create(&installer_jar).unwrap();
                        let mut zip = zip::ZipWriter::new(&mut file);
                        let options = zip::write::SimpleFileOptions::default()
                            .compression_method(zip::CompressionMethod::Stored);

                        zip.start_file("META-INF/MANIFEST.MF", options).unwrap();
                        zip.write_all(
                            b"Manifest-Version: 1.0\r\nMain-Class: MockInstaller\r\n\r\n",
                        )
                        .unwrap();

                        zip.start_file("MockInstaller.class", options).unwrap();
                        zip.write_all(&class_bytes).unwrap();
                        zip.finish().unwrap();

                        let result =
                            run_headless_installer_jar(&java_exe, &installer_jar, temp.path());
                        assert!(result.is_ok(), "Installer execution failed: {:?}", result);
                        let stdout = result.unwrap();
                        assert!(
                            stdout.contains("--installClient"),
                            "stdout should contain --installClient argument: {}",
                            stdout
                        );
                    }
                }
            }
        }
    }
}
