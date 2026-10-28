//! Global application settings model, persistence, and RFC 7386 JSON Merge Patch.

pub mod comp;
pub mod download;
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
pub use download::DownloadSettings;
pub use game::{DefaultIsolation, GameFolderEntry, GameSettings};
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

/// Rewrites launch flags that used to be stored negated (`disableX`) into their
/// positive form (`x`), inverting the value.
///
/// Done on the raw JSON so `deny_unknown_fields` never rejects a config written by
/// an older build — and so the user's choice survives the rename.
fn migrate_negated_launch_flags(value: &mut Value) {
    let Some(launch) = value.get_mut("launch").and_then(|v| v.as_object_mut()) else {
        return;
    };
    for (old, new) in [
        ("disableLegacyFix", "legacyFix"),
        ("disableLwjglUnsafeAgent", "lwjglUnsafeAgent"),
        ("disableCrashAnalysis", "crashAnalysis"),
    ] {
        if let Some(previous) = launch.remove(old)
            && !launch.contains_key(new)
        {
            let disabled = previous.as_bool().unwrap_or(false);
            launch.insert(new.to_string(), Value::Bool(!disabled));
        }
    }
}

/// Removes settings that no longer exist so `deny_unknown_fields` keeps accepting
/// previously written configs.
///
/// `java.select` was the single global "default Java"; Java is now resolved per instance
/// from the version requirement, so the pin is dropped rather than migrated into anything.
fn strip_removed_keys(value: &mut Value) {
    if let Some(java) = value.get_mut("java").and_then(|v| v.as_object_mut()) {
        java.remove("select");
    }
}

/// Current settings schema version; bumped whenever `migrate` changes stored values.
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

/// Upgrades persisted settings in place. Returns `true` when values changed and
/// the caller must write the file back.
fn migrate(settings: &mut Settings) -> bool {
    let mut changed = false;
    if settings.schema_version < 2 {
        // v2 introduced the recommended JVM argument head and the high-performance
        // GPU default; existing profiles would otherwise keep an empty field.
        if settings.launch.jvm_args.trim().is_empty() {
            settings.launch.jvm_args = launch::DEFAULT_JVM_ARGS.to_string();
        }
        settings.launch.require_gpu_high_performance = true;
        settings.schema_version = 2;
        changed = true;
    }
    if settings.schema_version < 3 {
        // v3: ship the launch options that are a net win by default —
        // `java.exe` keeps the game console (and its crash output) attached, and
        // a locked heap (`-Xms == -Xmx`) removes GC jitter on modded packs.
        // Both remain user-toggleable in 高级启动选项.
        settings.launch.use_java_exe = true;
        settings.launch.lock_memory = true;
        settings.schema_version = 3;
        changed = true;
    }
    if settings.schema_version < 4 {
        // v4: the launch switches became positive ("开 = 启用"). Earlier configs
        // stored the negated flags; they were already migrated to the enabled
        // state above, so only the schema marker is needed here.
        settings.schema_version = 4;
        changed = true;
    }
    if settings.schema_version < 5 {
        // v5: the global "default Java" (`java.select`) is gone — Java is resolved per
        // instance from the version's requirement. The raw JSON is scrubbed before
        // deserialization, so only the schema marker is needed here.
        settings.schema_version = 5;
        changed = true;
    }
    changed
}

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
    pub download: DownloadSettings,
    pub update: UpdateSettings,
    pub states: StatesSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            ui: UiSettings::default(),
            language: LanguageSettings::default(),
            system: SystemSettings::default(),
            net: NetSettings::default(),
            game: GameSettings::default(),
            java: JavaSettings::default(),
            launch: LaunchSettings::default(),
            comp: CompSettings::default(),
            download: DownloadSettings::default(),
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
            let parsed = serde_json::from_str::<Value>(&content)
                .map_err(|e| e.to_string())
                .and_then(|mut value| {
                    migrate_negated_launch_flags(&mut value);
                    strip_removed_keys(&mut value);
                    serde_json::from_value::<Settings>(value).map_err(|e| e.to_string())
                });
            parsed.unwrap_or_else(|e| {
                // Never destroy the user's settings silently: keep a copy next to
                // the file (a newer build's config, a stray key or a partial write
                // would otherwise be replaced by defaults on the next patch).
                let backup = file_path.with_extension("json.bak");
                match fs::copy(&file_path, &backup) {
                    Ok(_) => tracing::warn!(
                        "Failed to parse settings at {:?}: {}. Reset to default; original kept at {:?}",
                        file_path,
                        e,
                        backup
                    ),
                    Err(copy_err) => tracing::warn!(
                        "Failed to parse settings at {:?}: {}. Reset to default (backup failed: {copy_err})",
                        file_path,
                        e
                    ),
                }
                Settings::default()
            })
        } else {
            Settings::default()
        };

        let mut settings = settings;
        if migrate(&mut settings) {
            // Persist immediately: the debounced writer only fires on patches.
            if let Err(e) = Self::write_atomic(&file_path, &settings) {
                tracing::warn!("Failed to persist settings migration: {}", e);
            }
        }

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
    async fn corrupt_settings_are_backed_up_instead_of_lost() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let garbage = r#"{"schemaVersion":2,"ui":{"unexpectedKey":true}}"#;
        fs::write(&path, garbage).unwrap();

        let store = SettingsStore::open_path(&path).unwrap();
        // Falls back to defaults...
        assert_eq!(store.get().await.ui.seed, Settings::default().ui.seed);
        // ...but the original bytes survive in `config.json.bak`.
        let backup = path.with_extension("json.bak");
        assert_eq!(fs::read_to_string(&backup).unwrap(), garbage);
    }

    /// The global "default Java" pin was removed: a v4 config carrying `java.select`
    /// must still load (not be discarded as corrupt) and must come back without it.
    #[tokio::test]
    async fn test_legacy_java_select_is_dropped_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(
            &path,
            r##"{"schemaVersion":4,"ui":{"seed":"#123456"},"java":{"custom":["C:\\Java\\jdk8"],"select":"C:\\Java\\jdk21\\bin\\java.exe","preferJdk":true}}"##,
        )
        .unwrap();

        let store = SettingsStore::open_path(&path).unwrap();
        let settings = store.get().await;

        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(settings.ui.seed, "#123456", "user choices must survive");
        assert_eq!(settings.java.custom, vec!["C:\\Java\\jdk8".to_string()]);
        assert!(settings.java.prefer_jdk);
        assert!(
            !path.with_extension("json.bak").exists(),
            "a known legacy key must not trigger the corrupt-config fallback"
        );
    }

    #[tokio::test]
    async fn test_legacy_disabled_flags_are_inverted_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        // v3 config written before the switches became positive.
        fs::write(
            &path,
            r#"{"schemaVersion":3,"launch":{"disableLegacyFix":true,"disableLwjglUnsafeAgent":false,"disableCrashAnalysis":true}}"#,
        )
        .unwrap();

        let store = SettingsStore::open_path(&path).unwrap();
        let launch = store.get().await.launch;

        // The user's intent is preserved, only the polarity flips.
        assert!(!launch.legacy_fix);
        assert!(launch.lwjgl_unsafe_agent);
        assert!(!launch.crash_analysis);
    }

    #[tokio::test]
    async fn test_open_path_migrates_v2_launch_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        // v2 profile: the launch conveniences were still off.
        fs::write(
            &path,
            r#"{"schemaVersion":2,"launch":{"useJavaExe":false,"lockMemory":false}}"#,
        )
        .unwrap();

        let store = SettingsStore::open_path(&path).unwrap();
        let settings = store.get().await;

        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(settings.launch.use_java_exe);
        assert!(settings.launch.lock_memory);
        // Disabling afterwards must stick (no re-migration on the next open).
        let patch = serde_json::json!({"launch": {"useJavaExe": false}});
        store.patch(&patch).await.unwrap();
        // `patch` only schedules the debounced writer.
        store.save_sync().await.unwrap();
        let reopened = SettingsStore::open_path(&path).unwrap();
        assert!(!reopened.get().await.launch.use_java_exe);
    }

    #[tokio::test]
    async fn test_open_path_migrates_v1_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        // v1 profile: empty JVM args, GPU preference off.
        fs::write(
            &path,
            r#"{"schemaVersion":1,"launch":{"jvmArgs":"","requireGpuHighPerformance":false}}"#,
        )
        .unwrap();

        let store = SettingsStore::open_path(&path).unwrap();
        let settings = store.get().await;

        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(settings.launch.jvm_args, launch::DEFAULT_JVM_ARGS);
        assert!(settings.launch.require_gpu_high_performance);

        // Migration is persisted immediately, not only via the debounced writer.
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("-XX:+UseG1GC"));
        assert!(raw.contains(&format!("\"schemaVersion\": {CURRENT_SCHEMA_VERSION}")));
    }

    #[tokio::test]
    async fn test_open_path_keeps_custom_jvm_args_and_clears_are_respected_after_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{"schemaVersion":2,"launch":{"jvmArgs":""}}"#).unwrap();

        let store = SettingsStore::open_path(&path).unwrap();
        assert_eq!(store.get().await.launch.jvm_args, "");

        let path_v1 = dir.path().join("v1.json");
        fs::write(
            &path_v1,
            r#"{"schemaVersion":1,"launch":{"jvmArgs":"-Xmx2g"}}"#,
        )
        .unwrap();
        let store = SettingsStore::open_path(&path_v1).unwrap();
        assert_eq!(store.get().await.launch.jvm_args, "-Xmx2g");
    }

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
