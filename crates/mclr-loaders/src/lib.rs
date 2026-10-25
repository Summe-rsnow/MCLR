#![allow(clippy::collapsible_if)]
//! Mod loader installers (Fabric, LegacyFabric, LiteLoader, LabyMod, OptiFine, Forge, NeoForge, Cleanroom).

pub mod error;
pub mod fabric;
pub mod forge;
pub mod pipeline;

pub use error::LoaderError;
pub use fabric::{
    FabricInstaller, LabyModInstaller, LegacyFabricInstaller, LiteLoaderInstaller,
    OptiFineInstaller,
};
pub use forge::{CleanroomInstaller, ForgeInstaller, NeoForgeInstaller};
pub use pipeline::{
    ExtraMod, InstallEnv, InstallMode, InstallRequest, LabyModSel, StageCtx, Staged,
    install_instance,
};

use async_trait::async_trait;
use mclr_core::settings::GameSettings;
use mclr_core::{JavaSettings, NetSettings, TaskCenter};
use mclr_game::InstanceInfo;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LoaderKind {
    Fabric,
    LegacyFabric,
    LiteLoader,
    LabyMod,
    OptiFine,
    Forge,
    NeoForge,
    Cleanroom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersionEntry {
    pub loader: LoaderKind,
    pub mc_version: String,
    pub version: String,
    pub stable: bool,
    pub release_time: Option<String>,
}

pub struct LoaderInstallContext<'a> {
    pub client: &'a reqwest::Client,
    pub net_settings: &'a NetSettings,
    pub java_settings: &'a JavaSettings,
    pub mc_folder: PathBuf,
    pub mc_version: String,
    pub instance_name: String,
    pub loader_version: String,
    pub optifine_version: Option<String>,
    pub task_ctx: Option<(&'a TaskCenter, &'a str)>,
}

#[async_trait]
pub trait LoaderInstaller: Send + Sync {
    async fn versions(
        &self,
        client: &reqwest::Client,
        mc_version: &str,
    ) -> Result<Vec<LoaderVersionEntry>, LoaderError>;

    async fn install(&self, ctx: LoaderInstallContext<'_>) -> Result<InstanceInfo, LoaderError>;
}

pub async fn list_loader_versions(
    client: &reqwest::Client,
    loader: LoaderKind,
    mc_version: &str,
) -> Result<Vec<LoaderVersionEntry>, LoaderError> {
    match loader {
        LoaderKind::Fabric => FabricInstaller.versions(client, mc_version).await,
        LoaderKind::LegacyFabric => LegacyFabricInstaller.versions(client, mc_version).await,
        LoaderKind::LiteLoader => LiteLoaderInstaller.versions(client, mc_version).await,
        LoaderKind::LabyMod => LabyModInstaller.versions(client, mc_version).await,
        LoaderKind::OptiFine => OptiFineInstaller.versions(client, mc_version).await,
        LoaderKind::Forge => ForgeInstaller.versions(client, mc_version).await,
        LoaderKind::NeoForge => NeoForgeInstaller.versions(client, mc_version).await,
        LoaderKind::Cleanroom => CleanroomInstaller.versions(client, mc_version).await,
    }
}

pub async fn install_with_loader(
    loader: LoaderKind,
    ctx: LoaderInstallContext<'_>,
) -> Result<InstanceInfo, LoaderError> {
    let mut req = InstallRequest {
        mc_version: ctx.mc_version.clone(),
        instance_name: ctx.instance_name.clone(),
        forge: None,
        neoforge: None,
        cleanroom: None,
        fabric: None,
        legacy_fabric: None,
        labymod: None,
        liteloader: None,
        optifine: ctx.optifine_version.clone(),
        extra_mods: Vec::new(),
        mode: InstallMode::Create,
    };
    match loader {
        LoaderKind::Fabric => req.fabric = Some(ctx.loader_version.clone()),
        LoaderKind::LegacyFabric => req.legacy_fabric = Some(ctx.loader_version.clone()),
        LoaderKind::LiteLoader => req.liteloader = Some(ctx.loader_version.clone()),
        LoaderKind::LabyMod => {
            req.labymod = Some(LabyModSel {
                channel: "production".to_string(),
                commit_ref: ctx.loader_version.clone(),
                base_version: ctx.loader_version.clone(),
            })
        }
        LoaderKind::OptiFine => req.optifine = Some(ctx.loader_version.clone()),
        LoaderKind::Forge => req.forge = Some(ctx.loader_version.clone()),
        LoaderKind::NeoForge => req.neoforge = Some(ctx.loader_version.clone()),
        LoaderKind::Cleanroom => req.cleanroom = Some(ctx.loader_version.clone()),
    }
    let env = InstallEnv {
        client: ctx.client.clone(),
        net_settings: ctx.net_settings.clone(),
        java_settings: ctx.java_settings.clone(),
        game_settings: GameSettings::default(),
        mc_folder: ctx.mc_folder,
        task_ctx: ctx.task_ctx,
    };
    install_instance(req, env).await
}
