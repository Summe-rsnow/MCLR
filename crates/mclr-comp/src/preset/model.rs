//! Preset and store data models.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::model::{CompPlatform, CompType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PresetPinRule {
    /// Target game versions (empty = any version).
    #[serde(default)]
    pub game_versions: Vec<String>,
    /// Target loaders (empty = any loader).
    #[serde(default)]
    pub loaders: Vec<String>,
    /// Pinned release file ID.
    pub file_id: String,
    /// Display name of the pinned file.
    pub file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PresetItem {
    pub platform: CompPlatform,
    pub project_id: String,
    pub kind: CompType,
    pub title: String,
    pub slug: Option<String>,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub pinned_file_id: Option<String>,
    #[serde(default)]
    pub pinned_file_name: Option<String>,
    #[serde(default)]
    pub pin_rules: Vec<PresetPinRule>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub items: Vec<PresetItem>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OtherVersionPolicy {
    Skip,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRequest {
    pub instance_id: String,
    pub preset_ids: Vec<String>,
    pub datapack_world: Option<String>,
    pub include_dependencies: bool,
    pub on_other_version: OtherVersionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LinkMode {
    HardLink,
    Copy,
    Extracted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ApplyStatus {
    Installed,
    Replaced,
    AlreadySame,
    AlreadyOtherVersion,
    NoGameVersion,
    NoLoader,
    NoFile,
    DistributionDisabled,
    NeedsWorld,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyRow {
    pub platform: CompPlatform,
    pub project_id: String,
    pub kind: CompType,
    pub title: String,
    pub zh_title: Option<String>,
    pub status: ApplyStatus,
    pub file_id: Option<String>,
    pub file_name: Option<String>,
    pub installed_file_name: Option<String>,
    pub link_mode: Option<LinkMode>,
    pub from_presets: Vec<String>,
    pub overlap: bool,
    pub version_conflict: bool,
    pub via_dependency_of: Option<String>,
    pub message_code: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum GateReason {
    GameVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PresetGate {
    pub preset_id: String,
    pub name: String,
    pub ok: bool,
    pub reason: Option<GateReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApplyReport {
    pub instance_id: String,
    pub gates: Vec<PresetGate>,
    pub rows: Vec<ApplyRow>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum AddStatus {
    Added,
    AlreadyInPreset,
    AlreadyInPresetOtherVersion,
    NoGameVersion,
    NoLoader,
    NoFile,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AddItemOutcome {
    pub platform: CompPlatform,
    pub project_id: String,
    pub title: String,
    pub status: AddStatus,
    pub latest_file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StoreEntry {
    pub sha1: String,
    pub platform: CompPlatform,
    pub project_id: String,
    pub file_id: String,
    pub file_name: String,
    pub kind: CompType,
    pub size: u64,
    pub added_at: u64,
    pub live_refs: u32,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StoreInfo {
    pub dir: String,
    pub default_dir: String,
    pub object_count: u32,
    pub total_bytes: u64,
    pub saved_bytes: u64,
    pub entries: Vec<StoreEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PruneResult {
    pub removed_count: u32,
    pub freed_bytes: u64,
}
