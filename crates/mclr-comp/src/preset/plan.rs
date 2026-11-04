//! Preset application decision and planning engine.
//!
//! Everything here is pure: the caller fetches project files and the installed index, then feeds
//! them through [`merge_presets`] and [`decide`]. No filesystem or network access happens in this
//! module, which keeps every status transition unit-testable.

use crate::model::{CompFile, CompType};

use crate::preset::installed::{InstalledHit, InstalledIndex};
use crate::preset::model::{
    ApplyStatus, GateReason, OtherVersionPolicy, PresetGate, PresetItem, ResourcePreset,
};
use crate::resolver::{
    MatchFailure, file_supports_any_loader, file_supports_mc, select_best_file_filtered,
};

/// The instance a preset is applied to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceCtx {
    /// Minecraft version of the instance (`""` when unknown).
    pub mc_version: String,
    /// Canonical loader ids after [`crate::resolver::expand_loaders`].
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
    /// Union of the loader filters of the requesting presets. `None` when at least one of them
    /// has no loader filter (i.e. it applies to every loader the instance runs).
    pub preset_loaders: Option<Vec<String>>,
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
/// A preset whose game-version filter does not include the instance's version is gated with
/// [`GateReason::GameVersion`] and contributes no items. Accepted presets are merged by
/// `(platform, project_id)` in request order: the first preset's metadata wins, the first explicit
/// pin wins, and differing explicit pins raise `version_conflict`.
pub fn merge_presets(
    presets: &[ResourcePreset],
    ctx: &InstanceCtx,
) -> (Vec<PresetGate>, Vec<MergedItem>) {
    let mut gates = Vec::with_capacity(presets.len());
    let mut merged: Vec<MergedItem> = Vec::new();

    for preset in presets {
        let ok = preset.game_versions.is_empty()
            || preset
                .game_versions
                .iter()
                .any(|v| v.trim().eq_ignore_ascii_case(ctx.mc_version.trim()));
        gates.push(PresetGate {
            preset_id: preset.id.clone(),
            name: preset.name.clone(),
            ok,
            reason: if ok {
                None
            } else {
                Some(GateReason::GameVersion)
            },
        });
        if !ok {
            continue;
        }

        let preset_loaders = (!preset.loaders.is_empty()).then(|| preset.loaders.clone());

        for item in &preset.items {
            let existing = merged
                .iter_mut()
                .find(|m| m.item.platform == item.platform && m.item.project_id == item.project_id);

            let Some(existing) = existing else {
                merged.push(MergedItem {
                    item: item.clone(),
                    from_presets: vec![preset.name.clone()],
                    version_conflict: false,
                    preset_loaders: preset_loaders.clone(),
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

            // A preset without a loader filter lifts the restriction for this item.
            match &preset_loaders {
                None => existing.preset_loaders = None,
                Some(loaders) => {
                    let union = existing.preset_loaders.get_or_insert_with(Vec::new);
                    for loader in loaders {
                        if !union.iter().any(|l| l.eq_ignore_ascii_case(loader)) {
                            union.push(loader.clone());
                        }
                    }
                }
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

            for rule in &item.pin_rules {
                if !existing
                    .item
                    .pin_rules
                    .iter()
                    .any(|r| r.game_versions == rule.game_versions && r.loaders == rule.loaders)
                {
                    existing.item.pin_rules.push(rule.clone());
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

    let loader_filter = match kind {
        CompType::Mod => match effective_loaders(merged, ctx) {
            Some(loaders) => loaders,
            None => {
                return Decision {
                    status: ApplyStatus::NoLoader,
                    file: None,
                    existing: None,
                };
            }
        },
        _ => Vec::new(),
    };

    let file = match resolve_pinned_file_for_context(&merged.item, files, ctx, &loader_filter) {
        Some(pinned) => pinned.clone(),
        None => {
            // When no pin rule matches the target instance (or for versions/loaders not covered by
            // any rule), fall back to selecting the best / latest compatible version.
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

/// Loaders a mod item may be installed for.
///
/// `None` means the instance's loaders and the preset filters are disjoint, i.e. no build of this
/// mod can be installed. An instance with no known loader has no restriction.
fn effective_loaders(merged: &MergedItem, ctx: &InstanceCtx) -> Option<Vec<String>> {
    if ctx.loaders.is_empty() {
        return Some(Vec::new());
    }
    match &merged.preset_loaders {
        None => Some(ctx.loaders.clone()),
        Some(preset_loaders) => {
            let intersection: Vec<String> = ctx
                .loaders
                .iter()
                .filter(|loader| {
                    preset_loaders
                        .iter()
                        .any(|p| p.eq_ignore_ascii_case(loader))
                })
                .cloned()
                .collect();
            (!intersection.is_empty()).then_some(intersection)
        }
    }
}

fn status_for_failure(failure: MatchFailure) -> ApplyStatus {
    match failure {
        MatchFailure::NoGameVersion => ApplyStatus::NoGameVersion,
        MatchFailure::NoLoader => ApplyStatus::NoLoader,
        MatchFailure::NoFile => ApplyStatus::NoFile,
    }
}

/// Resolves the pinned file matching the target instance's context according to configured rules.
/// If no rule matches, returns `None`, which causes the planner to select the latest compatible version.
fn resolve_pinned_file_for_context<'a>(
    item: &PresetItem,
    files: &'a [CompFile],
    ctx: &InstanceCtx,
    loader_filter: &[String],
) -> Option<&'a CompFile> {
    // 1. Check configured pin rules.
    // A rule applies only when it specifically matches the instance's game version and loader.
    for rule in &item.pin_rules {
        if rule.game_versions.is_empty() && rule.loaders.is_empty() {
            continue;
        }

        let version_matches = !rule.game_versions.is_empty()
            && !ctx.mc_version.trim().is_empty()
            && rule
                .game_versions
                .iter()
                .any(|v| v.trim().eq_ignore_ascii_case(ctx.mc_version.trim()));

        let loader_matches = !rule.loaders.is_empty()
            && !loader_filter.is_empty()
            && rule.loaders.iter().any(|l| {
                loader_filter
                    .iter()
                    .any(|inst_ldr| inst_ldr.eq_ignore_ascii_case(l))
            });

        let matches = match (!rule.game_versions.is_empty(), !rule.loaders.is_empty()) {
            (true, true) => version_matches && loader_matches,
            (true, false) => version_matches,
            (false, true) => loader_matches,
            (false, false) => false,
        };

        if matches && let Some(file) = files.iter().find(|f| f.id == rule.file_id) {
            let mc_ok = ctx.mc_version.trim().is_empty() || file_supports_mc(file, &ctx.mc_version);
            let ldr_ok = item.kind != CompType::Mod
                || loader_filter.is_empty()
                || file_supports_any_loader(
                    &file.loaders,
                    &loader_filter.iter().map(String::as_str).collect::<Vec<_>>(),
                );
            if mc_ok && ldr_ok {
                return Some(file);
            }
        }
    }

    // 2. Legacy fallback for single pinned_file_id
    if let Some(pin_id) = &item.pinned_file_id
        && let Some(file) = files.iter().find(|f| &f.id == pin_id)
    {
        let mc_ok = ctx.mc_version.trim().is_empty() || file_supports_mc(file, &ctx.mc_version);
        let ldr_ok = item.kind != CompType::Mod
            || loader_filter.is_empty()
            || file_supports_any_loader(
                &file.loaders,
                &loader_filter.iter().map(String::as_str).collect::<Vec<_>>(),
            );
        if mc_ok && ldr_ok {
            return Some(file);
        }
    }

    None
}
