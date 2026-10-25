//! Mod loader installers: Fabric, LegacyFabric, LiteLoader, LabyMod, and OptiFine.
#![allow(clippy::collapsible_if)]

use crate::error::LoaderError;
use crate::pipeline::{LabyModSel, StageCtx, Staged};
use crate::{LoaderInstallContext, LoaderInstaller, LoaderKind, LoaderVersionEntry};
use async_trait::async_trait;
use mclr_core::paths;
use mclr_game::{
    InstanceInfo, Library, VersionJson, scan_java, select_java, version_java_constraint,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use tracing::warn;

// ============================================================================
// Internal HTTP Helpers
// ============================================================================

pub(crate) async fn fetch_json<T: for<'de> Deserialize<'de>>(
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

pub(crate) async fn fetch_string(
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

pub(crate) async fn download_file(
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
    } else if let Some(idx) = version_str.rfind('_') {
        let (opt_type, patch) = version_str.split_at(idx);
        (
            opt_type.to_string(),
            patch.trim_start_matches('_').to_string(),
        )
    } else {
        (version_str.to_string(), String::new())
    }
}

pub(crate) fn parse_mc_minor(mc_version: &str) -> u32 {
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

impl FabricInstaller {
    pub async fn stage(ctx: &StageCtx<'_>, loader_version: &str) -> Result<Staged, LoaderError> {
        let primary_profile = format!(
            "https://bmclapi2.bangbang93.com/fabric-meta/v2/versions/loader/{}/{}/profile/json",
            ctx.mc_version, loader_version
        );
        let fallback_profile = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
            ctx.mc_version, loader_version
        );

        let fabric_profile_str =
            fetch_string(ctx.client, &primary_profile, Some(&fallback_profile)).await?;
        let fabric_profile = VersionJson::parse(&fabric_profile_str).map_err(LoaderError::Game)?;

        Ok(Staged {
            version_json: fabric_profile,
            mods: Vec::new(),
        })
    }
}

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
        crate::install_with_loader(LoaderKind::Fabric, ctx).await
    }
}

// ============================================================================
// 2. LegacyFabricInstaller
// ============================================================================

pub struct LegacyFabricInstaller;

impl LegacyFabricInstaller {
    pub async fn stage(ctx: &StageCtx<'_>, loader_version: &str) -> Result<Staged, LoaderError> {
        let profile_url = format!(
            "https://meta.legacyfabric.net/v2/versions/loader/{}/{}/profile/json",
            ctx.mc_version, loader_version
        );

        let profile_str = fetch_string(ctx.client, &profile_url, None).await?;
        let profile = VersionJson::parse(&profile_str).map_err(LoaderError::Game)?;

        Ok(Staged {
            version_json: profile,
            mods: Vec::new(),
        })
    }
}

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
        crate::install_with_loader(LoaderKind::LegacyFabric, ctx).await
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

impl LiteLoaderInstaller {
    pub async fn stage(ctx: &StageCtx<'_>, loader_version: &str) -> Result<Staged, LoaderError> {
        let resolved_ver = if loader_version == ctx.mc_version || !loader_version.contains('-') {
            if let Ok(list) = Self.versions(ctx.client, ctx.mc_version).await
                && let Some(first) = list.first()
            {
                first.version.clone()
            } else {
                format!("{loader_version}-SNAPSHOT")
            }
        } else {
            loader_version.to_string()
        };
        let tweak_arg = "com.mumfrey.liteloader.launch.LiteLoaderTweaker";
        let ll_lib = Library {
            name: format!("com.mumfrey:liteloader:{resolved_ver}"),
            url: Some("https://bmclapi2.bangbang93.com/maven/".to_string()),
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

        let version_json = VersionJson {
            main_class: Some("net.minecraft.launchwrapper.Launch".to_string()),
            libraries: vec![ll_lib, lw_lib],
            minecraft_arguments: Some(format!("--tweakClass {tweak_arg}")),
            ..Default::default()
        };

        Ok(Staged {
            version_json,
            mods: Vec::new(),
        })
    }
}

#[async_trait]
impl LoaderInstaller for LiteLoaderInstaller {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
        let url = "https://dl.liteloader.com/versions/versions.json";
        let root: LiteLoaderRoot = fetch_json(client, url, None).await?;

        let mut entries = Vec::new();

        if let Some(mc_obj) = root.versions.get(mc_version) {
            for section_key in &["artefacts", "snapshots"] {
                if let Some(section) = mc_obj.get(section_key).and_then(|v| v.as_object()) {
                    if let Some(ll) = section
                        .get("com.mumfrey:liteloader")
                        .and_then(|v| v.as_object())
                    {
                        if let Some(latest) = ll.get("latest").and_then(|v| v.as_object()) {
                            let version = latest
                                .get("version")
                                .and_then(|v| v.as_str())
                                .unwrap_or("latest")
                                .to_string();
                            let stable = *section_key == "artefacts";
                            entries.push(LoaderVersionEntry {
                                loader: LoaderKind::LiteLoader,
                                mc_version: mc_version.to_string(),
                                version,
                                stable,
                                release_time: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(entries)
    }

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError> {
        crate::install_with_loader(LoaderKind::LiteLoader, ctx).await
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
}

#[derive(Deserialize)]
struct LabyModManifest {
    #[serde(rename = "labyModVersion")]
    laby_mod_version: String,
    #[serde(default, rename = "releaseTime")]
    release_time: Option<serde_json::Value>,
    #[serde(default, rename = "minecraftVersions")]
    minecraft_versions: Vec<LabyModMinecraftVersion>,
}

pub struct LabyModInstaller;

impl LabyModInstaller {
    pub async fn stage(ctx: &StageCtx<'_>, sel: &LabyModSel) -> Result<Staged, LoaderError> {
        let profile_url = format!(
            "https://releases.r2.labymod.net/api/v1/download/manifest/labymod4/{}/{}/{}.json",
            sel.channel, ctx.mc_version, sel.commit_ref
        );

        let profile_str = fetch_string(ctx.client, &profile_url, None).await?;
        let mut profile = VersionJson::parse(&profile_str).map_err(LoaderError::Game)?;

        let laby_core_lib = Library {
            name: "net.labymod:LabyMod:4".to_string(),
            url: Some(format!(
                "https://releases.r2.labymod.net/api/v1/download/labymod4/{}/{}.jar",
                sel.channel, sel.commit_ref
            )),
            downloads: None,
            rules: Vec::new(),
            natives: HashMap::new(),
            extract: None,
        };
        profile.libraries.insert(0, laby_core_lib);

        Ok(Staged {
            version_json: profile,
            mods: Vec::new(),
        })
    }
}

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
        crate::install_with_loader(LoaderKind::LabyMod, ctx).await
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

impl OptiFineInstaller {
    pub async fn stage(
        ctx: &StageCtx<'_>,
        optifine_version: &str,
        as_mod: bool,
        with_liteloader: bool,
    ) -> Result<Staged, LoaderError> {
        let (opt_type, patch) = parse_optifine_version(optifine_version);
        let formatted_ver = format_optifine_version(&opt_type, &patch);

        let cache_dir = paths::cache_dir().join("optifine");
        std::fs::create_dir_all(&cache_dir)?;
        let installer_jar = cache_dir.join(format!(
            "OptiFine-{}_{}-installer.jar",
            ctx.mc_version, formatted_ver
        ));

        if !installer_jar.exists() {
            let download_url = format!(
                "https://bmclapi2.bangbang93.com/optifine/{}/{}/{}",
                ctx.mc_version, opt_type, patch
            );
            download_file(ctx.client, &download_url, &installer_jar).await?;
        }

        if as_mod {
            let mods_target = if with_liteloader {
                ctx.staging_dir.join("mods").join(ctx.mc_version)
            } else {
                ctx.staging_dir.join("mods")
            };
            std::fs::create_dir_all(&mods_target)?;
            let target_jar =
                mods_target.join(format!("OptiFine_{}_{}.jar", ctx.mc_version, formatted_ver));
            std::fs::copy(&installer_jar, &target_jar)?;
            return Ok(Staged {
                version_json: VersionJson::default(),
                mods: vec![target_jar],
            });
        }

        let minor = parse_mc_minor(ctx.mc_version);
        let mut version_json = VersionJson::default();
        if minor >= 14 && installer_jar.exists() {
            let installed_javas = scan_java(&ctx.java_settings.custom);
            let dummy_json = VersionJson::default();
            let constraint = version_java_constraint(&dummy_json, ctx.mc_version);
            if let Some(java) = select_java(&installed_javas, &constraint, ctx.java_settings) {
                let mut cmd = Command::new(&java.path);
                cmd.arg(format!("-Duser.home={}", ctx.staging_dir.to_string_lossy()));
                if java.major >= 9 {
                    cmd.arg("--add-exports")
                        .arg("cpw.mods.bootstraplauncher/cpw.mods.bootstraplauncher=ALL-UNNAMED");
                }
                cmd.arg("-cp").arg(&installer_jar).arg("optifine.Installer");
                let _ = cmd.status();
            }

            let versions_dir = ctx.staging_dir.join("versions");
            if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.contains("OptiFine") {
                        let j_path = entry.path().join(format!("{name}.json"));
                        if j_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&j_path) {
                                if let Ok(vj) = VersionJson::parse(&content) {
                                    version_json = vj;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let lib_dir = ctx
                .staging_dir
                .join("libraries")
                .join("optifine")
                .join("OptiFine")
                .join(format!("{}_{}", ctx.mc_version, formatted_ver));
            std::fs::create_dir_all(&lib_dir)?;
            let lib_jar =
                lib_dir.join(format!("OptiFine-{}_{}.jar", ctx.mc_version, formatted_ver));
            std::fs::copy(&installer_jar, &lib_jar)?;

            let of_lib = Library {
                name: format!("optifine:OptiFine:{}_{}", ctx.mc_version, formatted_ver),
                url: None,
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
            version_json.main_class = Some("net.minecraft.launchwrapper.Launch".to_string());
            version_json.libraries = vec![of_lib, lw_lib];
            version_json.minecraft_arguments =
                Some("--tweakClass optifine.OptiFineTweaker".to_string());
        }

        Ok(Staged {
            version_json,
            mods: Vec::new(),
        })
    }
}

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
        crate::install_with_loader(LoaderKind::OptiFine, ctx).await
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use mclr_game::{
        ArgumentEntry, Arguments, InstanceKind, classify_instance, merge_version_json,
    };

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

        let (kind, loaders) = classify_instance(&merged, "1.20.1");
        assert_eq!(kind, InstanceKind::Fabric);
        assert!(loaders.iter().any(|l| l.contains("Fabric")));
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
