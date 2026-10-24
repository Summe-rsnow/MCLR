//! Global game launch settings (`settings.launch`).

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct LaunchSettings {
    pub ram_mode: String, // "auto" | "custom"
    pub ram_custom_mb: u32,
    pub jvm_args: String,
    pub game_args: String,
    pub window_title: String,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub preferred_ip_stack: String,      // "auto" | "ipv4" | "ipv6"
    pub process_priority: String,        // "normal" | "aboveNormal" | "high"
    pub close_launcher_on_start: String, // "keep" | "hide" | "exit"
    pub disable_crash_analysis: bool,
}

impl Default for LaunchSettings {
    fn default() -> Self {
        Self {
            ram_mode: "auto".to_string(),
            ram_custom_mb: 4096,
            jvm_args: String::new(),
            game_args: String::new(),
            window_title: String::new(),
            fullscreen: false,
            width: 854,
            height: 480,
            preferred_ip_stack: "auto".to_string(),
            process_priority: "normal".to_string(),
            close_launcher_on_start: "keep".to_string(),
            disable_crash_analysis: false,
        }
    }
}
