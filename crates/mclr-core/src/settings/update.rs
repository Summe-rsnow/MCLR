//! Update and announcement settings (`settings.update`).

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct UpdateSettings {
    pub channel: String, // "stable" | "beta"
    pub auto_check: bool,
    pub announcement_url: String,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            channel: "stable".to_string(),
            auto_check: true,
            announcement_url: String::new(),
        }
    }
}
