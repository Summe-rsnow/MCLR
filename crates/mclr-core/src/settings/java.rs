//! Java discovery and selection settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct JavaSettings {
    pub custom: Vec<String>,
    pub select: String,
    pub prefer_jdk: bool,
}

impl Default for JavaSettings {
    fn default() -> Self {
        Self {
            custom: Vec::new(),
            select: "auto".to_string(),
            prefer_jdk: false,
        }
    }
}
