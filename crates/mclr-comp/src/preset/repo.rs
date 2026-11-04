//! Resource preset repository persistence and CRUD operations.
//!
//! The repository is a single JSON document (`presets.json`) written atomically through a
//! temp-file rename, mirroring the settings store. A corrupt or unreadable document is moved
//! aside (`.bad`) instead of failing launcher startup: losing presets is recoverable, refusing to
//! boot is not.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::error::CompError;
use crate::model::{CompFile, CompPlatform, CompType};
use crate::preset::model::{AddItemOutcome, AddStatus, PresetItem, ResourcePreset};
use crate::resolver::MatchFailure;

/// Schema version of `presets.json`.
const REPO_VERSION: u32 = 1;
const MAX_NAME_CHARS: usize = 64;
const VALID_LOADERS: [&str; 4] = ["fabric", "forge", "neoforge", "quilt"];

/// On-disk shape of the preset repository.
#[derive(Debug, Serialize, Deserialize)]
struct RepoDocument {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    presets: Vec<ResourcePreset>,
}

/// Named resource presets persisted under the launcher data directory.
#[derive(Debug)]
pub struct PresetRepo {
    path: PathBuf,
    presets: Vec<ResourcePreset>,
}

impl PresetRepo {
    /// Loads the repository at `path`.
    ///
    /// A missing file yields an empty repository. A corrupt file is renamed to
    /// `presets.json.bad` (best effort) and also yields an empty repository; the caller never
    /// sees an error because startup must not depend on user-editable JSON.
    pub fn load(path: PathBuf) -> Self {
        let presets = match fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<RepoDocument>(&bytes) {
                Ok(doc) => doc.presets,
                Err(err) => {
                    tracing::warn!(path = %path.display(), error = %err, "presets.json is corrupt; starting empty");
                    let backup = path.with_extension("json.bad");
                    let _ = fs::rename(&path, &backup);
                    Vec::new()
                }
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => {
                tracing::warn!(path = %path.display(), error = %err, "failed to read presets.json; starting empty");
                Vec::new()
            }
        };
        Self { path, presets }
    }

    /// Returns every preset in insertion order.
    pub fn list(&self) -> Vec<ResourcePreset> {
        self.presets.clone()
    }

    /// Returns the preset with `id`.
    pub fn get(&self, id: &str) -> Option<ResourcePreset> {
        self.presets.iter().find(|p| p.id == id).cloned()
    }

    /// Creates (empty `id`) or updates a preset and persists the repository.
    ///
    /// Filters are normalized (trimmed, lowercased loaders, deduplicated) and validated: names
    /// must be non-empty, at most 64 characters and unique case-insensitively; loaders must be a
    /// subset of `fabric`/`forge`/`neoforge`/`quilt`; modpacks and Modrinth worlds cannot be
    /// preset items.
    pub fn save(&mut self, preset: ResourcePreset) -> Result<ResourcePreset, CompError> {
        if preset.items.iter().any(|item| !kind_supported(item)) {
            return Err(CompError::Preset("preset.kindUnsupported"));
        }

        let name = preset.name.trim().to_string();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
            return Err(CompError::Preset("preset.nameInvalid"));
        }
        let duplicate = self
            .presets
            .iter()
            .any(|p| p.id != preset.id && p.name.trim().eq_ignore_ascii_case(&name));
        if duplicate {
            return Err(CompError::Preset("preset.nameExists"));
        }

        let mut loaders: Vec<String> = Vec::with_capacity(preset.loaders.len());
        for loader in &preset.loaders {
            let id = loader.trim().to_ascii_lowercase();
            if id.is_empty() {
                continue;
            }
            if !VALID_LOADERS.contains(&id.as_str()) {
                return Err(CompError::Preset("preset.loaderInvalid"));
            }
            if !loaders.contains(&id) {
                loaders.push(id);
            }
        }

        let mut game_versions: Vec<String> = Vec::with_capacity(preset.game_versions.len());
        for version in &preset.game_versions {
            let version = version.trim().to_string();
            if !version.is_empty() && !game_versions.contains(&version) {
                game_versions.push(version);
            }
        }

        let now = now_ms();
        let mut saved = preset;
        saved.name = name;
        saved.loaders = loaders;
        saved.game_versions = game_versions;

        match self.presets.iter_mut().find(|p| p.id == saved.id) {
            Some(existing) if !saved.id.is_empty() => {
                saved.created_at = existing.created_at;
                saved.updated_at = now;
                *existing = saved.clone();
            }
            _ => {
                if saved.id.is_empty() {
                    saved.id = uuid::Uuid::new_v4().to_string();
                }
                saved.created_at = now;
                saved.updated_at = now;
                self.presets.push(saved.clone());
            }
        }

        self.persist()?;
        Ok(saved)
    }

    /// Deletes the preset with `id`.
    pub fn delete(&mut self, id: &str) -> Result<(), CompError> {
        let before = self.presets.len();
        self.presets.retain(|p| p.id != id);
        if self.presets.len() == before {
            return Err(CompError::Preset("preset.notFound"));
        }
        self.persist()
    }

    /// Adds items to a preset, resolving each through `checker`.
    ///
    /// `checker` receives the item as supplied and returns the file it resolves to (honouring the
    /// item's pin) or the reason no file matches the preset filters. Items that cannot be added
    /// are reported individually; the repository is written once when at least one item was
    /// added.
    pub fn add_items<F>(
        &mut self,
        preset_id: &str,
        items: Vec<PresetItem>,
        checker: F,
    ) -> Result<Vec<AddItemOutcome>, CompError>
    where
        F: Fn(&PresetItem) -> Result<Option<CompFile>, MatchFailure>,
    {
        let preset = self
            .presets
            .iter_mut()
            .find(|p| p.id == preset_id)
            .ok_or(CompError::Preset("preset.notFound"))?;

        let mut outcomes = Vec::with_capacity(items.len());
        let mut changed = false;

        for item in items {
            let outcome = |status: AddStatus, latest_file_name: Option<String>| AddItemOutcome {
                platform: item.platform,
                project_id: item.project_id.clone(),
                title: item.title.clone(),
                status,
                latest_file_name,
            };

            if !kind_supported(&item) {
                outcomes.push(outcome(AddStatus::Unsupported, None));
                continue;
            }

            if let Some(existing) = preset
                .items
                .iter()
                .find(|p| p.platform == item.platform && p.project_id == item.project_id)
            {
                let status = if existing.pinned_file_id == item.pinned_file_id {
                    AddStatus::AlreadyInPreset
                } else {
                    AddStatus::AlreadyInPresetOtherVersion
                };
                outcomes.push(outcome(status, None));
                continue;
            }

            match checker(&item) {
                Ok(Some(file)) => {
                    let latest_file_name = Some(file.file_name.clone());
                    preset.items.push(item.clone());
                    preset.updated_at = now_ms();
                    changed = true;
                    outcomes.push(outcome(AddStatus::Added, latest_file_name));
                }
                Ok(None) => outcomes.push(outcome(AddStatus::NoFile, None)),
                Err(MatchFailure::NoGameVersion) => {
                    outcomes.push(outcome(AddStatus::NoGameVersion, None))
                }
                Err(MatchFailure::NoLoader) => outcomes.push(outcome(AddStatus::NoLoader, None)),
                Err(MatchFailure::NoFile) => outcomes.push(outcome(AddStatus::NoFile, None)),
            }
        }

        if changed {
            self.persist()?;
        }
        Ok(outcomes)
    }

    /// Removes the item for `(platform, project_id)` from a preset.
    ///
    /// Removing an item that is not present is a no-op so two racing UI actions cannot fail.
    pub fn remove_item(
        &mut self,
        preset_id: &str,
        platform: CompPlatform,
        project_id: &str,
    ) -> Result<(), CompError> {
        let preset = self
            .presets
            .iter_mut()
            .find(|p| p.id == preset_id)
            .ok_or(CompError::Preset("preset.notFound"))?;

        let before = preset.items.len();
        preset
            .items
            .retain(|item| !(item.platform == platform && item.project_id == project_id));
        if preset.items.len() != before {
            preset.updated_at = now_ms();
            self.persist()?;
        }
        Ok(())
    }

    /// Pins an item to one file version, or unpins it when `file_id` is `None`.
    pub fn pin(
        &mut self,
        preset_id: &str,
        platform: CompPlatform,
        project_id: &str,
        file_id: Option<String>,
        file_name: Option<String>,
    ) -> Result<(), CompError> {
        let preset = self
            .presets
            .iter_mut()
            .find(|p| p.id == preset_id)
            .ok_or(CompError::Preset("preset.notFound"))?;
        let item = preset
            .items
            .iter_mut()
            .find(|item| item.platform == platform && item.project_id == project_id)
            .ok_or(CompError::Preset("preset.itemNotFound"))?;

        match file_id {
            Some(id) => {
                item.pinned_file_id = Some(id);
                item.pinned_file_name = file_name;
            }
            None => {
                item.pinned_file_id = None;
                item.pinned_file_name = None;
                item.pin_rules.clear();
            }
        }
        preset.updated_at = now_ms();
        self.persist()
    }

    /// Adds or updates a specific pin rule for an item.
    pub fn pin_rule(
        &mut self,
        preset_id: &str,
        platform: CompPlatform,
        project_id: &str,
        rule: crate::preset::model::PresetPinRule,
    ) -> Result<(), CompError> {
        let preset = self
            .presets
            .iter_mut()
            .find(|p| p.id == preset_id)
            .ok_or(CompError::Preset("preset.notFound"))?;
        let item = preset
            .items
            .iter_mut()
            .find(|item| item.platform == platform && item.project_id == project_id)
            .ok_or(CompError::Preset("preset.itemNotFound"))?;

        let existing = item
            .pin_rules
            .iter_mut()
            .find(|r| r.game_versions == rule.game_versions && r.loaders == rule.loaders);
        if let Some(r) = existing {
            r.file_id = rule.file_id.clone();
            r.file_name = rule.file_name.clone();
        } else {
            item.pin_rules.push(rule.clone());
        }
        item.pinned_file_id = Some(rule.file_id);
        item.pinned_file_name = Some(rule.file_name);
        preset.updated_at = now_ms();
        self.persist()
    }

    /// Removes a pin rule by pinned file id.
    pub fn remove_pin_rule(
        &mut self,
        preset_id: &str,
        platform: CompPlatform,
        project_id: &str,
        file_id: &str,
    ) -> Result<(), CompError> {
        let preset = self
            .presets
            .iter_mut()
            .find(|p| p.id == preset_id)
            .ok_or(CompError::Preset("preset.notFound"))?;
        let item = preset
            .items
            .iter_mut()
            .find(|item| item.platform == platform && item.project_id == project_id)
            .ok_or(CompError::Preset("preset.itemNotFound"))?;

        item.pin_rules.retain(|r| r.file_id != file_id);
        if let Some(first) = item.pin_rules.first() {
            item.pinned_file_id = Some(first.file_id.clone());
            item.pinned_file_name = Some(first.file_name.clone());
        } else {
            item.pinned_file_id = None;
            item.pinned_file_name = None;
        }
        preset.updated_at = now_ms();
        self.persist()
    }

    /// Returns every pinned file id across all presets (used to keep store objects alive).
    pub fn pinned_file_ids(&self) -> std::collections::HashSet<String> {
        self.presets
            .iter()
            .flat_map(|p| p.items.iter())
            .flat_map(|item| {
                let mut ids = Vec::new();
                if let Some(id) = &item.pinned_file_id {
                    ids.push(id.clone());
                }
                for r in &item.pin_rules {
                    ids.push(r.file_id.clone());
                }
                ids
            })
            .collect()
    }

    /// Writes the repository atomically (temp file + rename).
    fn persist(&self) -> Result<(), CompError> {
        write_repo_atomic(&self.path, &self.presets)
    }
}

/// Writes `presets` to `path` atomically.
fn write_repo_atomic(path: &Path, presets: &[ResourcePreset]) -> Result<(), CompError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let doc = RepoDocument {
        version: REPO_VERSION,
        presets: presets.to_vec(),
    };
    let tmp_path = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(&doc)?;
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Presets may only carry mods, resource packs, shaders, datapacks and CurseForge worlds.
fn kind_supported(item: &PresetItem) -> bool {
    match item.kind {
        CompType::Mod | CompType::Resourcepack | CompType::Shader | CompType::Datapack => true,
        CompType::World => item.platform == CompPlatform::Curseforge,
        CompType::Modpack => false,
    }
}

/// Milliseconds since the Unix epoch, saturating at `0` before 1970.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
