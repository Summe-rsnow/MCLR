//! Community platform settings (`settings.comp`).

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CompFavoriteEntry {
    pub platform: String,
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub icon_url: Option<String>,
    pub comp_type: String,
}

impl Default for CompFavoriteEntry {
    fn default() -> Self {
        Self {
            platform: "modrinth".to_string(),
            project_id: String::new(),
            slug: String::new(),
            title: String::new(),
            icon_url: None,
            comp_type: "mod".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct CompSettings {
    pub favorites: Vec<CompFavoriteEntry>,
}
