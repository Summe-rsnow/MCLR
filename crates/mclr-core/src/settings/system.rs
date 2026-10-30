//! System-level configuration settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SystemSettings {
    pub log_level: String,
    pub log_retention_days: u32,
    pub disable_gpu: bool,
    /// Closing the main window hides it to the tray instead of quitting.
    pub keep_in_background: bool,
    /// The first-close prompt was answered with "don't ask again".
    pub close_prompt_dismissed: bool,
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            log_retention_days: 7,
            disable_gpu: false,
            keep_in_background: false,
            close_prompt_dismissed: false,
        }
    }
}
