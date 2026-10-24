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
    pub font_family: String,
    pub background_mode: String,
    pub background_path: String,
    pub background_blur: f32,
    pub background_opacity: f32,
    pub music_enabled: bool,
    pub music_folder: String,
    pub music_volume: u8,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            seed: "#3b82f6".to_string(),
            mode: ThemeMode::System,
            material: MaterialMode::Flat,
            scale: 1.0,
            font_family: String::new(),
            background_mode: "none".to_string(),
            background_path: String::new(),
            background_blur: 0.0,
            background_opacity: 0.3,
            music_enabled: false,
            music_folder: String::new(),
            music_volume: 50,
        }
    }
}
