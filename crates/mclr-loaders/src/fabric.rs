//! Mod loader installers: Fabric, LegacyFabric, LiteLoader, LabyMod, and OptiFine.
#![allow(clippy::collapsible_if)]

use crate::error::LoaderError;
use crate::{LoaderInstallContext, LoaderInstaller, LoaderKind, LoaderVersionEntry};
use async_trait::async_trait;
use mclr_game::{
    ArgumentEntry, InstanceConfig, InstanceInfo, InstanceState, Library, VersionJson,
    classify_instance, install_vanilla, merge_version_json, repair_instance, scan_java,
    select_java, version_java_constraint,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use tracing::{info, warn};

// ============================================================================
// Internal HTTP Helpers
// ============================================================================

async fn fetch_json<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    primary_url: &str,
    fallback_url: Option<&str>,
) -> Result<T, LoaderError> {
    let resp = match client.get(primary_url).send().await {
        Ok(r) if r.status().is_success() => Ok(r),
        Ok(r) => Err(format!("HTTP status {}", r.status())),
        Err(e) => Err(e.to_string()),
    };

    let resp = match resp {
        Ok(r) => r,
        Err(err) => {
            if let Some(fb) = fallback_url {
                warn!("Primary URL {primary_url} failed ({err}), trying fallback {fb}");
                client
                    .get(fb)
                    .send()
                    .await
                    .map_err(|e| LoaderError::Net(e.into()))?
            } else {
                return Err(LoaderError::Metadata(format!(
                    "Failed to fetch {primary_url}: {err}"
                )));
            }
        }
    };

    if !resp.status().is_success() {
        return Err(LoaderError::Metadata(format!(
            "Failed to fetch {primary_url}: HTTP {}",
            resp.status()
        )));
    }

    let bytes = resp.bytes().await.map_err(|e| LoaderError::Net(e.into()))?;

    serde_json::from_slice(&bytes).map_err(LoaderError::Json)
}

async fn fetch_string(
    client: &reqwest::Client,
    primary_url: &str,
    fallback_url: Option<&str>,
) -> Result<String, LoaderError> {
    let resp = match client.get(primary_url).send().await {
        Ok(r) if r.status().is_success() => Ok(r),
        Ok(r) => Err(format!("HTTP status {}", r.status())),
        Err(e) => Err(e.to_string()),
    };

    let resp = match resp {
        Ok(r) => r,
        Err(err) => {
            if let Some(fb) = fallback_url {
                warn!("Primary URL {primary_url} failed ({err}), trying fallback {fb}");
                client
                    .get(fb)
                    .send()
                    .await
                    .map_err(|e| LoaderError::Net(e.into()))?
            } else {
                return Err(LoaderError::Metadata(format!(
                    "Failed to fetch {primary_url}: {err}"
                )));
            }
        }
    };

    if !resp.status().is_success() {
        return Err(LoaderError::Metadata(format!(
            "Failed to fetch {primary_url}: HTTP {}",
            resp.status()
        )));
    }

    resp.text().await.map_err(|e| LoaderError::Net(e.into()))
}

async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
) -> Result<(), LoaderError> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| LoaderError::Net(e.into()))?;

    if !resp.status().is_success() {
        return Err(LoaderError::Metadata(format!(
            "Failed to download {url}: HTTP {}",
            resp.status()
        )));
    }

    let bytes = resp.bytes().await.map_err(|e| LoaderError::Net(e.into()))?;

    std::fs::write(dest, bytes)?;
    Ok(())
}

async fn ensure_vanilla_installed(
    ctx: &LoaderInstallContext<'_>,
) -> Result<VersionJson, LoaderError> {
    let instance_dir = ctx.mc_folder.join("versions").join(&ctx.instance_name);
    let instance_json_file = instance_dir.join(format!("{}.json", ctx.instance_name));

    if instance_json_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&instance_json_file) {
            if let Ok(json) = VersionJson::parse(&content) {
                return Ok(json);
            }
        }
    }

    info!(
        "Base instance JSON not found at {}; installing vanilla {} first",
        instance_json_file.display(),
        ctx.mc_version
    );

    install_vanilla(
        ctx.client,
        ctx.net_settings,
        &ctx.mc_folder,
        &ctx.mc_version,
        &ctx.instance_name,
        ctx.task_ctx,
    )
    .await?;

    let content = std::fs::read_to_string(&instance_json_file)?;
    VersionJson::parse(&content).map_err(LoaderError::Game)
}

fn finish_instance_install(
    ctx: &LoaderInstallContext<'_>,
    merged: &VersionJson,
) -> Result<InstanceInfo, LoaderError> {
    let instance_dir = ctx.mc_folder.join("versions").join(&ctx.instance_name);
    let instance_json_file = instance_dir.join(format!("{}.json", ctx.instance_name));
    std::fs::create_dir_all(&instance_dir)?;

    let serialized = serde_json::to_string_pretty(merged)?;
    std::fs::write(&instance_json_file, serialized)?;

    let (kind, loaders) = classify_instance(merged, &ctx.mc_version);
    let cfg_path = instance_dir.join("mclr.instance.json");
    let config = if cfg_path.exists() {
        std::fs::read_to_string(&cfg_path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        InstanceConfig::default()
    };

    Ok(InstanceInfo {
        id: ctx.instance_name.clone(),
        name: ctx.instance_name.clone(),
        folder: ctx.mc_folder.to_string_lossy().to_string(),
        path: instance_dir.to_string_lossy().to_string(),
        mc_version: ctx.mc_version.clone(),
        kind,
        loaders,
        state: InstanceState::Ok,
        config,
    })
}

// ============================================================================
// Helper Utilities
// ============================================================================

pub fn make_optifine_library(mc_version: &str, optifine_version: &str) -> Library {
    Library {
        name: format!("optifine:OptiFine:{mc_version}_{optifine_version}"),
        url: None,
        downloads: None,
        rules: Vec::new(),
        natives: HashMap::new(),
        extract: None,
    }
}

pub fn format_optifine_version(opt_type: &str, patch: &str) -> String {
    format!("{opt_type}_{patch}")
}

pub fn parse_optifine_version(version_str: &str) -> (String, String) {
    if let Some(pos) = version_str.find("_pre") {
        let (opt_type, patch) = version_str.split_at(pos);
        (
            opt_type.to_string(),
            patch.trim_start_matches('_').to_string(),
        )
    } else if let Some(last_underscore) = version_str.rfind('_') {
        let (opt_type, patch) = version_str.split_at(last_underscore);
        (
            opt_type.to_string(),
            patch.trim_start_matches('_').to_string(),
        )
    } else {
        ("HD_U".to_string(), version_str.to_string())
    }
}

fn parse_mc_minor(mc_version: &str) -> u32 {
    let parts: Vec<&str> = mc_version.split('.').collect();
    if parts.len() >= 2 {
        parts[1].parse().unwrap_or(0)
    } else {
        0
    }
}

// ============================================================================
// 1. FabricInstaller
// ============================================================================

#[derive(Deserialize)]
struct FabricMetaLoader {
    version: String,
    stable: bool,
}

#[derive(Deserialize)]
struct FabricMetaEntry {
    loader: FabricMetaLoader,
}

pub struct FabricInstaller;

#[async_trait]
impl LoaderInstaller for FabricInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let primary =
            format!("https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/{mc_version}");
        let fallback = format!("https://meta.fabricmc.net/v2/versions/loader/{mc_version}");

        let raw_list: Vec<FabricMetaEntry> = fetch_json(client, &primary, Some(&fallback)).await?;
        let entries = raw_list
            .into_iter()
            .map(|item| LoaderVersionEntry {
                loader: LoaderKind::Fabric,
                mc_version: mc_version.to_string(),
                version: item.loader.version,
                stable: item.loader.stable,
                release_time: None,
            })
            .collect();
        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        let vanilla_json = ensure_vanilla_installed(&ctx).await?;

        let primary_profile = format!(
            "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/{}/{}/profile/json",
            ctx.mc_version, ctx.loader_version
        );
        let fallback_profile = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
            ctx.mc_version, ctx.loader_version
        );

        let fabric_profile_str =
            fetch_string(ctx.client, &primary_profile, Some(&fallback_profile)).await?;
        let fabric_profile = VersionJson::parse(&fabric_profile_str).map_err(LoaderError::Game)?;

        let mut merged = merge_version_json(fabric_profile, vanilla_json);
        merged.id = ctx.instance_name.clone();
        merged.inherits_from = None;

        if let Some(of_ver) = &ctx.optifine_version {
            merged
                .libraries
                .insert(0, make_optifine_library(&ctx.mc_version, of_ver));
        }

        let info = finish_instance_install(&ctx, &merged)?;
        repair_instance(
            ctx.client,
            ctx.net_settings,
            &ctx.mc_folder,
            &ctx.instance_name,
            ctx.task_ctx,
        )
        .await?;
        Ok(info)
    }
}

// ============================================================================
// 2. LegacyFabricInstaller
// ============================================================================

pub struct LegacyFabricInstaller;

#[async_trait]
impl LoaderInstaller for LegacyFabricInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let primary = format!("https://meta.legacyfabric.net/v2/versions/loader/{mc_version}");
        let raw_list: Vec<FabricMetaEntry> = fetch_json(client, &primary, None).await?;
        let entries = raw_list
            .into_iter()
            .map(|item| LoaderVersionEntry {
                loader: LoaderKind::LegacyFabric,
                mc_version: mc_version.to_string(),
                version: item.loader.version,
                stable: item.loader.stable,
                release_time: None,
            })
            .collect();
        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        let vanilla_json = ensure_vanilla_installed(&ctx).await?;

        let profile_url = format!(
            "https://meta.legacyfabric.net/v2/versions/loader/{}/{}/profile/json",
            ctx.mc_version, ctx.loader_version
        );

        let profile_str = fetch_string(ctx.client, &profile_url, None).await?;
        let profile = VersionJson::parse(&profile_str).map_err(LoaderError::Game)?;

        let mut merged = merge_version_json(profile, vanilla_json);
        merged.id = ctx.instance_name.clone();
        merged.inherits_from = None;

        if let Some(of_ver) = &ctx.optifine_version {
            merged
                .libraries
                .insert(0, make_optifine_library(&ctx.mc_version, of_ver));
        }

        let info = finish_instance_install(&ctx, &merged)?;
        repair_instance(
            ctx.client,
            ctx.net_settings,
            &ctx.mc_folder,
            &ctx.instance_name,
            ctx.task_ctx,
        )
        .await?;
        Ok(info)
    }
}

// ============================================================================
// 3. LiteLoaderInstaller
// ============================================================================

#[derive(Deserialize)]
struct LiteLoaderRoot {
    #[serde(default)]
    versions: HashMap<String, serde_json::Value>,
}

pub struct LiteLoaderInstaller;

#[async_trait]
impl LoaderInstaller for LiteLoaderInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let primary = "https://bmclapi2.bangbang93.com/maven/com/mumfrey/liteloader/versions.json";
        let fallback = "http://dl.liteloader.com/versions/versions.json";

        let root: LiteLoaderRoot = fetch_json(client, primary, Some(fallback)).await?;

        let mut entries = Vec::new();
        let mut seen = std::collections::HashSet::new();

        if let Some(mc_obj) = root.versions.get(mc_version) {
            for section_key in &["artefacts", "snapshots"] {
                if let Some(section) = mc_obj.get(section_key).and_then(|v| v.as_object()) {
                    if let Some(ll) = section
                        .get("com.mumfrey:liteloader")
                        .and_then(|v| v.as_object())
                    {
                        for (_k, build_val) in ll {
                            if let Some(b_obj) = build_val.as_object() {
                                if let Some(ver) = b_obj.get("version").and_then(|v| v.as_str()) {
                                    if seen.insert(ver.to_string()) {
                                        let stream = b_obj
                                            .get("stream")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("");
                                        let stable = stream.eq_ignore_ascii_case("RELEASE")
                                            || !ver.contains("SNAPSHOT");
                                        let release_time = b_obj.get("timestamp").and_then(|v| {
                                            if let Some(s) = v.as_str() {
                                                Some(s.to_string())
                                            } else {
                                                v.as_i64().map(|n| n.to_string())
                                            }
                                        });
                                        entries.push(LoaderVersionEntry {
                                            loader: LoaderKind::LiteLoader,
                                            mc_version: mc_version.to_string(),
                                            version: ver.to_string(),
                                            stable,
                                            release_time,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        let mut vanilla_json = ensure_vanilla_installed(&ctx).await?;

        vanilla_json.main_class = Some("net.minecraft.launchwrapper.Launch".to_string());
        vanilla_json.id = ctx.instance_name.clone();
        vanilla_json.inherits_from = None;

        let tweak_arg = "com.mumfrey.liteloader.launch.LiteLoaderTweaker";
        if let Some(args) = &mut vanilla_json.minecraft_arguments {
            if !args.contains(tweak_arg) {
                args.push_str(&format!(" --tweakClass {tweak_arg}"));
            }
        } else if let Some(arguments) = &mut vanilla_json.arguments {
            arguments
                .game
                .push(ArgumentEntry::Plain("--tweakClass".to_string()));
            arguments
                .game
                .push(ArgumentEntry::Plain(tweak_arg.to_string()));
        } else {
            vanilla_json.minecraft_arguments = Some(format!("--tweakClass {tweak_arg}"));
        }

        let ll_lib = Library {
            name: format!("com.mumfrey:liteloader:{}", ctx.loader_version),
            url: Some("https://dl.liteloader.com/versions/".to_string()),
            downloads: None,
            rules: Vec::new(),
            natives: HashMap::new(),
            extract: None,
        };
        let lw_lib = Library {
            name: "net.minecraft:launchwrapper:1.12".to_string(),
            url: Some("https://libraries.minecraft.net/".to_string()),
            downloads: None,
            rules: Vec::new(),
            natives: HashMap::new(),
            extract: None,
        };

        vanilla_json.libraries.insert(0, ll_lib);
        vanilla_json.libraries.insert(1, lw_lib);

        if let Some(of_ver) = &ctx.optifine_version {
            vanilla_json
                .libraries
                .insert(0, make_optifine_library(&ctx.mc_version, of_ver));
        }

        let info = finish_instance_install(&ctx, &vanilla_json)?;
        repair_instance(
            ctx.client,
            ctx.net_settings,
            &ctx.mc_folder,
            &ctx.instance_name,
            ctx.task_ctx,
        )
        .await?;
        Ok(info)
    }
}

// ============================================================================
// 4. LabyModInstaller
// ============================================================================

#[derive(Deserialize)]
struct LabyModMinecraftVersion {
    version: String,
    #[serde(default)]
    tag: Option<String>,
    #[serde(default, rename = "customManifestUrl")]
    custom_manifest_url: Option<String>,
}

#[derive(Deserialize)]
struct LabyModManifest {
    #[serde(rename = "labyModVersion")]
    laby_mod_version: String,
    #[serde(rename = "commitReference")]
    commit_reference: String,
    #[serde(default, rename = "releaseTime")]
    release_time: Option<serde_json::Value>,
    #[serde(default, rename = "minecraftVersions")]
    minecraft_versions: Vec<LabyModMinecraftVersion>,
}

pub struct LabyModInstaller;

#[async_trait]
impl LoaderInstaller for LabyModInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let manifest_url = "https://releases.r2.labymod.net/api/v1/manifest/production/latest.json";
        let manifest: LabyModManifest = fetch_json(client, manifest_url, None).await?;

        let supported = manifest
            .minecraft_versions
            .iter()
            .any(|mv| mv.version == mc_version || mv.tag.as_deref() == Some(mc_version));

        if !supported {
            return Ok(Vec::new());
        }

        let release_time = manifest.release_time.map(|v| v.to_string());
        Ok(vec![LoaderVersionEntry {
            loader: LoaderKind::LabyMod,
            mc_version: mc_version.to_string(),
            version: manifest.laby_mod_version,
            stable: true,
            release_time,
        }])
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        let vanilla_json = ensure_vanilla_installed(&ctx).await?;

        let manifest_url = "https://releases.r2.labymod.net/api/v1/manifest/production/latest.json";
        let manifest: LabyModManifest = fetch_json(ctx.client, manifest_url, None).await?;

        let commit_ref = if ctx.loader_version.len() == 8
            && ctx.loader_version.chars().all(|c| c.is_ascii_hexdigit())
        {
            ctx.loader_version.clone()
        } else {
            manifest.commit_reference.clone()
        };

        let custom_url = manifest
            .minecraft_versions
            .iter()
            .find(|mv| mv.version == ctx.mc_version || mv.tag.as_deref() == Some(&ctx.mc_version))
            .and_then(|mv| mv.custom_manifest_url.clone());

        let profile_url = custom_url.unwrap_or_else(|| {
            format!(
                "https://releases.r2.labymod.net/api/v1/download/manifest/labymod4/production/{}/{}.json",
                ctx.mc_version, commit_ref
            )
        });

        let profile_str = fetch_string(ctx.client, &profile_url, None).await?;
        let profile = VersionJson::parse(&profile_str).map_err(LoaderError::Game)?;

        let mut merged = merge_version_json(profile, vanilla_json);
        merged.id = ctx.instance_name.clone();
        merged.inherits_from = None;

        let laby_core_lib = Library {
            name: "net.labymod:LabyMod:4".to_string(),
            url: Some(format!(
                "https://releases.r2.labymod.net/api/v1/download/labymod4/production/{commit_ref}.jar"
            )),
            downloads: None,
            rules: Vec::new(),
            natives: HashMap::new(),
            extract: None,
        };
        merged.libraries.insert(0, laby_core_lib);

        if let Some(of_ver) = &ctx.optifine_version {
            merged
                .libraries
                .insert(0, make_optifine_library(&ctx.mc_version, of_ver));
        }

        let info = finish_instance_install(&ctx, &merged)?;
        repair_instance(
            ctx.client,
            ctx.net_settings,
            &ctx.mc_folder,
            &ctx.instance_name,
            ctx.task_ctx,
        )
        .await?;
        Ok(info)
    }
}

// ============================================================================
// 5. OptiFineInstaller
// ============================================================================

#[derive(Deserialize)]
struct OptiFineListItem {
    #[serde(rename = "type")]
    opt_type: String,
    patch: String,
}

pub struct OptiFineInstaller;

#[async_trait]
impl LoaderInstaller for OptiFineInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let url = format!("https://bmclapi2.bangbang93.com/optifine/{mc_version}");
        let list: Vec<OptiFineListItem> = fetch_json(client, &url, None).await?;

        let entries = list
            .into_iter()
            .map(|item| {
                let formatted = format_optifine_version(&item.opt_type, &item.patch);
                let stable = !item.patch.to_lowercase().contains("pre");
                LoaderVersionEntry {
                    loader: LoaderKind::OptiFine,
                    mc_version: mc_version.to_string(),
                    version: formatted,
                    stable,
                    release_time: None,
                }
            })
            .collect();
        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        let mut vanilla_json = ensure_vanilla_installed(&ctx).await?;

        let (opt_type, patch) = parse_optifine_version(&ctx.loader_version);
        let formatted_ver = format_optifine_version(&opt_type, &patch);

        let lib_dir = ctx
            .mc_folder
            .join("libraries")
            .join("optifine")
            .join("OptiFine")
            .join(format!("{}_{}", ctx.mc_version, formatted_ver));
        std::fs::create_dir_all(&lib_dir)?;

        let installer_jar = lib_dir.join(format!(
            "OptiFine-{}_{}-installer.jar",
            ctx.mc_version, formatted_ver
        ));
        let target_lib_jar =
            lib_dir.join(format!("OptiFine-{}_{}.jar", ctx.mc_version, formatted_ver));

        if !installer_jar.exists() {
            let download_url = format!(
                "https://bmclapi2.bangbang93.com/optifine/{}/{}/{}",
                ctx.mc_version, opt_type, patch
            );
            download_file(ctx.client, &download_url, &installer_jar).await?;
        }

        let minor = parse_mc_minor(&ctx.mc_version);
        if minor >= 14 && installer_jar.exists() {
            let java_list = scan_java(&ctx.java_settings.custom);
            let constraint = version_java_constraint(&vanilla_json, &ctx.mc_version);
            if let Some(java) = select_java(&java_list, &constraint, ctx.java_settings) {
                let mut cmd = Command::new(&java.path);
                cmd.arg(format!("-Duser.home={}", ctx.mc_folder.to_string_lossy()));
                if java.major >= 9 {
                    cmd.arg("--add-exports")
                        .arg("cpw.mods.bootstraplauncher/cpw.mods.bootstraplauncher=ALL-UNNAMED");
                }
                cmd.arg("-cp").arg(&installer_jar).arg("optifine.Installer");
                let _ = cmd.status();
            }
        }

        if !target_lib_jar.exists() && installer_jar.exists() {
            let _ = std::fs::copy(&installer_jar, &target_lib_jar);
        }

        vanilla_json
            .libraries
            .insert(0, make_optifine_library(&ctx.mc_version, &formatted_ver));

        if minor < 13 {
            let lw = Library {
                name: "net.minecraft:launchwrapper:1.12".to_string(),
                url: Some("https://libraries.minecraft.net/".to_string()),
                downloads: None,
                rules: Vec::new(),
                natives: HashMap::new(),
                extract: None,
            };
            vanilla_json.libraries.insert(1, lw);
            vanilla_json.main_class = Some("net.minecraft.launchwrapper.Launch".to_string());
            if let Some(args) = &mut vanilla_json.minecraft_arguments {
                if !args.contains("optifine.OptiFineTweaker") {
                    args.push_str(" --tweakClass optifine.OptiFineTweaker");
                }
            } else {
                vanilla_json.minecraft_arguments =
                    Some("--tweakClass optifine.OptiFineTweaker".to_string());
            }
        } else if let Some(arguments) = &mut vanilla_json.arguments {
            arguments
                .game
                .push(ArgumentEntry::Plain("--tweakClass".to_string()));
            arguments
                .game
                .push(ArgumentEntry::Plain("optifine.OptiFineTweaker".to_string()));
        } else if let Some(args) = &mut vanilla_json.minecraft_arguments {
            if !args.contains("optifine.OptiFineTweaker") {
                args.push_str(" --tweakClass optifine.OptiFineTweaker");
            }
        }

        vanilla_json.id = ctx.instance_name.clone();
        vanilla_json.inherits_from = None;

        let info = finish_instance_install(&ctx, &vanilla_json)?;
        repair_instance(
            ctx.client,
            ctx.net_settings,
            &ctx.mc_folder,
            &ctx.instance_name,
            ctx.task_ctx,
        )
        .await?;
        Ok(info)
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mclr_game::{ArgumentEntry, Arguments, InstanceKind};

    #[test]
    fn test_optifine_version_formatting_and_parsing() {
        assert_eq!(format_optifine_version("HD_U", "I6"), "HD_U_I6");
        assert_eq!(format_optifine_version("HD_U_I5", "pre4"), "HD_U_I5_pre4");

        let (t1, p1) = parse_optifine_version("HD_U_I6");
        assert_eq!(t1, "HD_U");
        assert_eq!(p1, "I6");

        let (t2, p2) = parse_optifine_version("HD_U_I5_pre4");
        assert_eq!(t2, "HD_U_I5");
        assert_eq!(p2, "pre4");
    }

    #[test]
    fn test_fabric_profile_merge_standalone() {
        let vanilla_json = VersionJson {
            id: "1.20.1".to_string(),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            libraries: vec![Library {
                name: "org.lwjgl:lwjgl:3.3.1".to_string(),
                url: None,
                downloads: None,
                rules: Vec::new(),
                natives: HashMap::new(),
                extract: None,
            }],
            arguments: Some(Arguments {
                game: vec![ArgumentEntry::Plain("--username".to_string())],
                jvm: vec![ArgumentEntry::Plain("-Xmx2G".to_string())],
            }),
            ..Default::default()
        };

        let fabric_profile = VersionJson {
            id: "fabric-loader-0.16.10-1.20.1".to_string(),
            inherits_from: Some("1.20.1".to_string()),
            main_class: Some("net.fabricmc.loader.impl.launch.knot.KnotClient".to_string()),
            libraries: vec![
                Library {
                    name: "net.fabricmc:fabric-loader:0.16.10".to_string(),
                    url: Some("https://maven.fabricmc.net/".to_string()),
                    downloads: None,
                    rules: Vec::new(),
                    natives: HashMap::new(),
                    extract: None,
                },
                Library {
                    name: "net.fabricmc:intermediary:1.20.1".to_string(),
                    url: Some("https://maven.fabricmc.net/".to_string()),
                    downloads: None,
                    rules: Vec::new(),
                    natives: HashMap::new(),
                    extract: None,
                },
            ],
            ..Default::default()
        };

        let mut merged = merge_version_json(fabric_profile, vanilla_json);
        merged.id = "my-fabric-inst".to_string();
        merged.inherits_from = None;

        // Apply OptiFine combo
        merged
            .libraries
            .insert(0, make_optifine_library("1.20.1", "HD_U_I6"));

        assert_eq!(merged.id, "my-fabric-inst");
        assert!(merged.inherits_from.is_none());
        assert_eq!(
            merged.main_class.as_deref(),
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient")
        );

        // Classify instance should detect both Fabric and OptiFine
        let (kind, loaders) = classify_instance(&merged, "1.20.1");
        assert_eq!(kind, InstanceKind::Fabric);
        assert!(loaders.iter().any(|l| l.contains("Fabric")));
        assert!(loaders.iter().any(|l| l.contains("OptiFine")));
    }

    #[test]
    fn test_liteloader_synthesis_and_classification() {
        let mut vanilla = VersionJson {
            id: "1.12.2".to_string(),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            minecraft_arguments: Some("--username ${auth_player_name}".to_string()),
            libraries: vec![Library {
                name: "com.mojang:patchy:1.1".to_string(),
                url: None,
                downloads: None,
                rules: Vec::new(),
                natives: HashMap::new(),
                extract: None,
            }],
            ..Default::default()
        };

        vanilla.main_class = Some("net.minecraft.launchwrapper.Launch".to_string());
        vanilla.id = "lite-1.12.2".to_string();
        vanilla.inherits_from = None;

        let tweak = "com.mumfrey.liteloader.launch.LiteLoaderTweaker";
        if let Some(args) = &mut vanilla.minecraft_arguments {
            args.push_str(&format!(" --tweakClass {tweak}"));
        }

        vanilla.libraries.insert(
            0,
            Library {
                name: "com.mumfrey:liteloader:1.12.2-SNAPSHOT".to_string(),
                url: Some("https://dl.liteloader.com/versions/".to_string()),
                downloads: None,
                rules: Vec::new(),
                natives: HashMap::new(),
                extract: None,
            },
        );
        vanilla.libraries.insert(
            1,
            Library {
                name: "net.minecraft:launchwrapper:1.12".to_string(),
                url: Some("https://libraries.minecraft.net/".to_string()),
                downloads: None,
                rules: Vec::new(),
                natives: HashMap::new(),
                extract: None,
            },
        );

        let (kind, loaders) = classify_instance(&vanilla, "1.12.2");
        assert_eq!(kind, InstanceKind::LiteLoader);
        assert!(loaders.iter().any(|l| l.contains("LiteLoader")));
    }

    #[test]
    fn test_optifine_synthesis_and_classification() {
        let mut vanilla = VersionJson {
            id: "1.20.1".to_string(),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            arguments: Some(Arguments::default()),
            libraries: vec![],
            ..Default::default()
        };

        vanilla
            .libraries
            .insert(0, make_optifine_library("1.20.1", "HD_U_I6"));
        if let Some(args) = &mut vanilla.arguments {
            args.game
                .push(ArgumentEntry::Plain("--tweakClass".to_string()));
            args.game
                .push(ArgumentEntry::Plain("optifine.OptiFineTweaker".to_string()));
        }

        let (kind, loaders) = classify_instance(&vanilla, "1.20.1");
        assert_eq!(kind, InstanceKind::OptiFine);
        assert!(loaders.iter().any(|l| l.contains("OptiFine")));
    }
}
