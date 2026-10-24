//! Mod loader installers (Fabric, LegacyFabric, LiteLoader, LabyMod, OptiFine, Forge, NeoForge, Cleanroom).

pub mod error;
pub mod fabric;
pub mod forge;

pub use error::LoaderError;
pub use fabric::{
    FabricInstaller, LabyModInstaller, LegacyFabricInstaller, LiteLoaderInstaller,
    OptiFineInstaller,
};
pub use forge::{CleanroomInstaller, ForgeInstaller, NeoForgeInstaller};

use async_trait::async_trait;
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
    match loader {
        LoaderKind::Fabric => FabricInstaller.install(ctx).await,
        LoaderKind::LegacyFabric => LegacyFabricInstaller.install(ctx).await,
        LoaderKind::LiteLoader => LiteLoaderInstaller.install(ctx).await,
        LoaderKind::LabyMod => LabyModInstaller.install(ctx).await,
        LoaderKind::OptiFine => OptiFineInstaller.install(ctx).await,
        LoaderKind::Forge => ForgeInstaller.install(ctx).await,
        LoaderKind::NeoForge => NeoForgeInstaller.install(ctx).await,
        LoaderKind::Cleanroom => CleanroomInstaller.install(ctx).await,
    }
}
