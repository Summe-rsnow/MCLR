//! Preset application decision and planning engine.
//!
//! Everything here is pure: the caller fetches project files and the installed index, then feeds
//! them through [`merge_presets`] and [`decide`]. No filesystem or network access happens in this
//! module, which keeps every status transition unit-testable.

use crate::model::{CompFile, CompType};

use crate::preset::installed::{InstalledHit, InstalledIndex};
use crate::preset::model::{
    ApplyStatus, OtherVersionPolicy, PresetGate, PresetItem, ResourcePreset,
};
use crate::resolver::{MatchFailure, file_supports_any_loader, select_best_file_filtered};

/// The instance a preset is applied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceCtx {
    /// Minecraft version of the instance (`""` when unknown).
    pub mc_version: String,
    /// Canonical loader ids after [`mclr_core::loader::expand_mod_loaders`].
    pub loaders: Vec<String>,
}

/// A preset item after cross-preset merging, carrying which presets requested it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedItem {
    pub item: PresetItem,
    /// Names of the presets requesting this item, in request (priority) order.
    pub from_presets: Vec<String>,
    /// Two presets pinned different file ids; the first pin wins.
    pub version_conflict: bool,
}

/// The planner's answer for one merged item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub status: ApplyStatus,
    pub file: Option<CompFile>,
    pub existing: Option<InstalledHit>,
}

/// Splits presets into the accepted ones and the merged item list.
///
/// Presets are universal resource lists: their legacy `game_versions`/`loaders` fields are
/// ignored, so every requested preset is accepted and its files are matched against the target
/// instance at apply time ([`decide`]). Items are merged by `(platform, project_id)` in request
/// order: the first preset's metadata wins, the first explicit pin wins, and differing explicit
/// pins raise `version_conflict`.
pub fn merge_presets(presets: &[ResourcePreset]) -> (Vec<PresetGate>, Vec<MergedItem>) {
    let mut gates = Vec::with_capacity(presets.len());
    let mut merged: Vec<MergedItem> = Vec::new();

    for preset in presets {
        gates.push(PresetGate {
            preset_id: preset.id.clone(),
            name: preset.name.clone(),
            ok: true,
            reason: None,
        });

        for item in &preset.items {
            let existing = merged
                .iter_mut()
                .find(|m| m.item.platform == item.platform && m.item.project_id == item.project_id);

            let Some(existing) = existing else {
                merged.push(MergedItem {
                    item: item.clone(),
                    from_presets: vec![preset.name.clone()],
                    version_conflict: false,
                });
                continue;
            };

            if !existing
                .from_presets
                .iter()
                .any(|name| name == &preset.name)
            {
                existing.from_presets.push(preset.name.clone());
            }

            if let Some(pin) = item.pinned_file_id.clone() {
                match existing.item.pinned_file_id.clone() {
                    None => {
                        existing.item.pinned_file_name = item.pinned_file_name.clone();
                        existing.item.pinned_file_id = Some(pin);
                    }
                    Some(current) if current != pin => existing.version_conflict = true,
                    Some(_) => {}
                }
            }
        }
    }

    (gates, merged)
}

/// Decides what to do with one merged item.
///
/// Order of checks: datapack world requirement, pinned file resolution, best-file selection,
/// distribution policy, then installed-state comparison. `files` must be the project's full,
/// unfiltered file list.
pub fn decide(
    merged: &MergedItem,
    files: &[CompFile],
    ctx: &InstanceCtx,
    installed: &InstalledIndex,
    policy: OtherVersionPolicy,
    datapack_world_set: bool,
) -> Decision {
    let kind = merged.item.kind;

    if kind == CompType::Datapack && !datapack_world_set {
        return Decision {
            status: ApplyStatus::NeedsWorld,
            file: None,
            existing: None,
        };
    }

    let loader_filter = if kind == CompType::Mod {
        ctx.loaders.clone()
    } else {
        Vec::new()
    };

    let file = match resolve_pinned_file_for_context(&merged.item, files, ctx) {
        Some(pinned) => pinned.clone(),
        None => {
            // Without a usable pin, take the latest file compatible with the instance.
            let game_versions: &[String] = if ctx.mc_version.trim().is_empty() {
                &[]
            } else {
                std::slice::from_ref(&ctx.mc_version)
            };
            match select_best_file_filtered(files, game_versions, &loader_filter) {
                Ok(file) => file,
                Err(failure) => {
                    return Decision {
                        status: status_for_failure(failure),
                        file: None,
                        existing: None,
                    };
                }
            }
        }
    };

    if file.distribution_disabled {
        return Decision {
            status: ApplyStatus::DistributionDisabled,
            file: Some(file),
            existing: None,
        };
    }

    let sha1 = file
        .hashes
        .sha1
        .as_deref()
        .map(str::to_ascii_lowercase)
        .filter(|s| !s.is_empty());

    if let Some(sha1) = sha1.as_deref()
        && let Some(hit) = installed.by_sha1(sha1)
    {
        return Decision {
            status: ApplyStatus::AlreadySame,
            file: Some(file),
            existing: Some(hit.clone()),
        };
    }

    let other = installed
        .by_project(merged.item.platform, &merged.item.project_id)
        .iter()
        .find(|hit| Some(&hit.sha1) != sha1.as_ref())
        .cloned();
    if let Some(hit) = other {
        let status = match policy {
            OtherVersionPolicy::Skip => ApplyStatus::AlreadyOtherVersion,
            OtherVersionPolicy::Replace => ApplyStatus::Replaced,
        };
        return Decision {
            status,
            file: Some(file),
            existing: Some(hit),
        };
    }

    Decision {
        status: ApplyStatus::Installed,
        file: Some(file),
        existing: None,
    }
}

fn status_for_failure(failure: MatchFailure) -> ApplyStatus {
    match failure {
        MatchFailure::NoGameVersion => ApplyStatus::NoGameVersion,
        MatchFailure::NoLoader => ApplyStatus::NoLoader,
        MatchFailure::NoFile => ApplyStatus::NoFile,
    }
}

/// Resolves the item's single pinned file for the target instance.
///
/// The pin wins over version matching: it is only rejected when the file no longer exists in the
/// project's file list, or (for mods) when it does not support any of the instance's loaders. A
/// rejected or absent pin returns `None`, which makes the planner pick the latest compatible file.
fn resolve_pinned_file_for_context<'a>(
    item: &PresetItem,
    files: &'a [CompFile],
    ctx: &InstanceCtx,
) -> Option<&'a CompFile> {
    let pin_id = item.pinned_file_id.as_deref()?;
    let file = files.iter().find(|f| f.id == pin_id)?;

    if item.kind == CompType::Mod && !ctx.loaders.is_empty() {
        let loaders: Vec<&str> = ctx.loaders.iter().map(String::as_str).collect();
        if !file_supports_any_loader(&file.loaders, &loaders) {
            return None;
        }
    }

    Some(file)
}
