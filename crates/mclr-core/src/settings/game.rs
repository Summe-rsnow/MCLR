//! Game folders and instance management settings.

use crate::paths;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct GameFolderEntry {
    pub name: String,
    pub path: String,
}

impl Default for GameFolderEntry {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            path: paths::default_minecraft_dir().to_string_lossy().to_string(),
        }
    }
}

/// Global default for instances whose own isolation is `"global"`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum DefaultIsolation {
    Off,
    Modded,
    NonRelease,
    ModdedOrNonRelease,
    /// Legacy `config.json` files stored `"on"`.
    #[default]
    #[serde(alias = "on")]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct GameSettings {
    pub folders: Vec<GameFolderEntry>,
    pub current_folder: String,
    pub default_isolation: DefaultIsolation,
    pub sort_by: String,
}

impl Default for GameSettings {
    fn default() -> Self {
        let default_path = paths::default_minecraft_dir().to_string_lossy().to_string();
        Self {
            folders: vec![GameFolderEntry {
                name: "Default".to_string(),
                path: default_path.clone(),
            }],
            current_folder: default_path,
            default_isolation: DefaultIsolation::All,
            sort_by: "name".to_string(),
        }
    }
}
