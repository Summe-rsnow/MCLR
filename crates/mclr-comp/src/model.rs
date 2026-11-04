//! Unified data model for Modrinth and CurseForge projects, files, and search queries.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum CompPlatform {
    #[default]
    Modrinth,
    Curseforge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum CompType {
    #[default]
    Mod,
    Modpack,
    Resourcepack,
    Shader,
    Datapack,
    World,
}

impl CompType {
    /// Returns the subfolder inside the Minecraft instance/game directory where files of this type belong.
    pub fn target_subfolder(&self) -> &'static str {
        match self {
            CompType::Mod => "mods",
            CompType::Modpack => "modpacks",
            CompType::Resourcepack => "resourcepacks",
            CompType::Shader => "shaderpacks",
            CompType::Datapack => "datapacks",
            CompType::World => "saves",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum CompReleaseChannel {
    #[default]
    Release,
    Beta,
    Alpha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum CompDependencyKind {
    #[default]
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompDependency {
    pub project_id: String,
    pub kind: CompDependencyKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompFileHashes {
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub murmur2: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompFile {
    pub id: String,
    pub project_id: String,
    pub platform: CompPlatform,
    pub name: String,
    pub file_name: String,
    pub mc_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub channel: CompReleaseChannel,
    pub release_date: String,
    pub size: u64,
    pub hashes: CompFileHashes,
    pub download_urls: Vec<String>,
    pub distribution_disabled: bool,
    pub dependencies: Vec<CompDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompProject {
    pub platform: CompPlatform,
    pub id: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub author: String,
    pub downloads: u64,
    pub follows: u64,
    pub categories: Vec<String>,
    pub mc_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub comp_type: CompType,
    pub updated_at: String,
    pub website_url: Option<String>,
    pub zh_name: Option<String>,
    pub wiki_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub enum CompSort {
    #[default]
    Relevance,
    Downloads,
    Follows,
    Newest,
    Updated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompSearchQuery {
    pub platform: CompPlatform,
    pub comp_type: CompType,
    pub query: String,
    pub mc_version: Option<String>,
    pub loader: Option<String>,
    pub category: Option<String>,
    pub sort: CompSort,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CompSearchResult {
    pub hits: Vec<CompProject>,
    pub total_hits: u32,
    pub offset: u32,
    pub limit: u32,
}
