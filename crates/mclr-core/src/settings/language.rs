//! Language and localization settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct LanguageSettings {
    pub locale: String,
    pub format_culture: String,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            locale: "zh-CN".to_string(),
            format_culture: "zh-CN".to_string(),
        }
    }
}
