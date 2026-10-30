//! UI configuration settings.

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum MaterialMode {
    #[default]
    Flat,
    Glass,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct UiSettings {
    pub seed: String,
    pub mode: ThemeMode,
    pub material: MaterialMode,
    pub scale: f32,
    pub background_mode: String,
    pub background_path: String,
    pub background_blur: f32,
    pub background_opacity: f32,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            seed: "#3b82f6".to_string(),
            mode: ThemeMode::System,
            material: MaterialMode::Flat,
            scale: 1.0,
            background_mode: "none".to_string(),
            background_path: String::new(),
            background_blur: 0.0,
            background_opacity: 0.3,
        }
    }
}
