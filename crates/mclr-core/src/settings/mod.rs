//! Global application settings model, persistence, and RFC 7386 JSON Merge Patch.

pub mod comp;
pub mod game;
pub mod java;
pub mod language;
pub mod launch;
pub mod net;
pub mod states;
pub mod system;
pub mod ui;
pub mod update;

use crate::error::CoreError;
use crate::paths;
pub use comp::{CompFavoriteEntry, CompSettings};
pub use game::{GameFolderEntry, GameSettings};
pub use java::JavaSettings;
pub use language::LanguageSettings;
pub use launch::LaunchSettings;
pub use net::{NetSettings, ProxyMode, SourcePriority};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use specta::Type;
pub use states::StatesSettings;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
pub use system::SystemSettings;
use tokio::sync::{Notify, RwLock};
use tokio::time::{Duration, sleep};
pub use ui::{MaterialMode, ThemeMode, UiSettings};
pub use update::UpdateSettings;

/// Complete MCLR user settings.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub ui: UiSettings,
    pub language: LanguageSettings,
    pub system: SystemSettings,
    pub net: NetSettings,
    pub game: GameSettings,
    pub java: JavaSettings,
    pub launch: LaunchSettings,
    pub comp: CompSettings,
    pub update: UpdateSettings,
    pub states: StatesSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            ui: UiSettings::default(),
            language: LanguageSettings::default(),
            system: SystemSettings::default(),
            net: NetSettings::default(),
            game: GameSettings::default(),
            java: JavaSettings::default(),
            launch: LaunchSettings::default(),
            comp: CompSettings::default(),
            update: UpdateSettings::default(),
            states: StatesSettings::default(),
        }
    }
}

/// RFC 7386 JSON Merge Patch implementation.
///
/// If patch is not an object, target is replaced by patch.
/// If patch is an object, for each key:
/// - if value is null, remove key from target
/// - if value is object, recursively merge_patch
/// - otherwise, target[key] = value
pub fn merge_patch(target: &mut Value, patch: &Value) {
    if let Value::Object(patch_obj) = patch {
        if !target.is_object() {
            *target = Value::Object(serde_json::Map::new());
        }
        let target_obj = target.as_object_mut().unwrap();

        for (key, value) in patch_obj {
            if value.is_null() {
                target_obj.remove(key);
            } else {
                let entry = target_obj.entry(key).or_insert(Value::Null);
                merge_patch(entry, value);
            }
        }
    } else {
        *target = patch.clone();
    }
}

/// Applies RFC 7386 merge patch to a `Settings` instance and validates that the result
/// strictly conforms to `Settings` (denying unknown fields and type mismatches).
pub fn apply_patch(current: &Settings, patch: &Value) -> Result<Settings, CoreError> {
    let mut current_val = serde_json::to_value(current).map_err(CoreError::Json)?;
    merge_patch(&mut current_val, patch);

    let updated: Settings = serde_json::from_value(current_val)
        .map_err(|e| CoreError::SettingsInvalid(e.to_string()))?;

    Ok(updated)
}

/// Thread-safe settings store with debounced asynchronous save and synchronous fallback.
pub struct SettingsStore {
    file_path: PathBuf,
    settings: Arc<RwLock<Settings>>,
    dirty_notify: Arc<Notify>,
}

impl SettingsStore {
    /// Opens or initializes settings from the canonical `config.json` path.
    pub fn open() -> Result<Self, CoreError> {
        Self::open_path(paths::config_file())
    }

    /// Opens or initializes settings from a specified path.
    pub fn open_path(path: impl Into<PathBuf>) -> Result<Self, CoreError> {
        let file_path = path.into();
        let settings = if file_path.exists() {
            let content = fs::read_to_string(&file_path)?;
            serde_json::from_str(&content).unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to parse settings at {:?}: {}, resetting to default",
                    file_path,
                    e
                );
                Settings::default()
            })
        } else {
            Settings::default()
        };

        let settings = Arc::new(RwLock::new(settings));
        let dirty_notify = Arc::new(Notify::new());

        // Spawn background debounced save task
        let bg_settings = Arc::clone(&settings);
        let bg_path = file_path.clone();
        let bg_notify = Arc::clone(&dirty_notify);

        tokio::spawn(async move {
            loop {
                bg_notify.notified().await;
                // Debounce 300ms
                loop {
                    tokio::select! {
                        _ = bg_notify.notified() => {
                            // Reset debounce window on rapid notifications
                            continue;
                        }
                        _ = sleep(Duration::from_millis(300)) => {
                            break;
                        }
                    }
                }

                let current = bg_settings.read().await.clone();
                if let Err(e) = Self::write_atomic(&bg_path, &current) {
                    tracing::error!("Failed to write settings atomically: {}", e);
                }
            }
        });

        Ok(Self {
            file_path,
            settings,
            dirty_notify,
        })
    }

    /// Gets a cloned snapshot of current settings.
    pub async fn get(&self) -> Settings {
        self.settings.read().await.clone()
    }

    /// Applies an RFC 7386 patch, schedules debounced save, and returns the updated settings.
    pub async fn patch(&self, patch: &Value) -> Result<Settings, CoreError> {
        let mut lock = self.settings.write().await;
        let updated = apply_patch(&lock, patch)?;
        *lock = updated.clone();
        drop(lock);

        self.dirty_notify.notify_one();
        Ok(updated)
    }

    /// Writes settings immediately to disk atomically.
    pub async fn save_sync(&self) -> Result<(), CoreError> {
        let current = self.settings.read().await.clone();
        Self::write_atomic(&self.file_path, &current)
    }

    /// Atomic write helper: writes to `.tmp` file and renames it.
    fn write_atomic(path: &Path, settings: &Settings) -> Result<(), CoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp_path = path.with_extension("tmp");
        let json_bytes = serde_json::to_vec_pretty(settings).map_err(CoreError::Json)?;
        fs::write(&tmp_path, json_bytes)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_settings_default_and_merge_patch() {
        let current = Settings::default();
        assert_eq!(current.ui.seed, "#3b82f6");
        assert_eq!(current.ui.mode, ThemeMode::System);

        // Valid patch
        let patch = json!({
            "ui": {
                "seed": "#ff0000",
                "mode": "dark"
            }
        });
        let updated = apply_patch(&current, &patch).expect("valid patch should succeed");
        assert_eq!(updated.ui.seed, "#ff0000");
        assert_eq!(updated.ui.mode, ThemeMode::Dark);
        assert_eq!(updated.ui.material, MaterialMode::Flat); // unchanged

        // Invalid patch: unknown field
        let invalid_patch = json!({
            "ui": {
                "nonExistentField": 123
            }
        });
        let err =
            apply_patch(&current, &invalid_patch).expect_err("unknown field should be rejected");
        match err {
            CoreError::SettingsInvalid(_) => {}
            other => panic!("expected SettingsInvalid, got: {:?}", other),
        }

        // Invalid patch: wrong type
        let wrong_type_patch = json!({
            "system": {
                "logRetentionDays": "not-a-number"
            }
        });
        let err2 =
            apply_patch(&current, &wrong_type_patch).expect_err("wrong type should be rejected");
        match err2 {
            CoreError::SettingsInvalid(_) => {}
            other => panic!("expected SettingsInvalid, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_settings_store_save_and_reload() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("config.json");

        let store = SettingsStore::open_path(&path).unwrap();
        let initial = store.get().await;
        assert_eq!(initial.ui.seed, "#3b82f6");

        let patch = json!({
            "ui": {
                "seed": "#10b981"
            }
        });
        let updated = store.patch(&patch).await.unwrap();
        assert_eq!(updated.ui.seed, "#10b981");

        // Save immediately
        store.save_sync().await.unwrap();
        assert!(path.exists());

        // Reload in new store
        let store2 = SettingsStore::open_path(&path).unwrap();
        let loaded = store2.get().await;
        assert_eq!(loaded.ui.seed, "#10b981");
    }
}
