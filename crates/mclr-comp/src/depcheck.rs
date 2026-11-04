//! Mod dependency analysis and checking.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::local::LocalModInfo;
use crate::versionrange::satisfies;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ModDepKind {
    Required,
    Optional,
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModDep {
    pub id: String,
    pub range: Option<String>,
    pub kind: ModDepKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DepStatus {
    Installed,
    Disabled,
    Bundled,
    VersionMismatch,
    Missing,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DepItem {
    pub id: String,
    pub kind: ModDepKind,
    pub range: Option<String>,
    pub status: DepStatus,
    pub provider_file: Option<String>,
    pub provider_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModDependencyReport {
    pub file_name: String,
    pub mod_id: String,
    pub deps: Vec<DepItem>,
    pub dependents: Vec<String>,
}

/// Ids the platforms own: every mod depends on them, so listing them would bury the real
/// problems in noise. Compared lowercase.
const IGNORED_IDS: &[&str] = &[
    "minecraft",
    "java",
    "forge",
    "neoforge",
    "fml",
    "javafml",
    "lowcodefml",
    "fabricloader",
    "fabric-loader",
    "fabric_loader",
    "fabric",
    "quilt_loader",
    "quilt-loader",
    "mcp",
    "mod_resources",
];

/// Analyses the dependency graph of the installed mods.
///
/// `bundled_ids` holds mod ids that ship inside other jars (Jar-in-Jar), so a requirement they
/// satisfy is reported as `Bundled` instead of `Missing`. The result has one entry per mod in
/// input order; `deps` keeps the declared order minus ignored platform ids and `Incompatible`
/// rows with no installed provider (nothing to warn about).
pub fn analyze(mods: &[LocalModInfo], bundled_ids: &HashSet<String>) -> Vec<ModDependencyReport> {
    analyze_with_loader(mods, bundled_ids, None)
}

/// Analyses the dependency graph of installed mods, taking the active instance's mod loader into
/// account so multi-loader jars running on NeoForge/Forge don't falsely report missing Fabric
/// loader APIs, and vice versa.
pub fn analyze_with_loader(
    mods: &[LocalModInfo],
    bundled_ids: &HashSet<String>,
    instance_loader: Option<&str>,
) -> Vec<ModDependencyReport> {
    let providers = provider_index(mods);
    let bundled: HashSet<String> = bundled_ids.iter().map(|id| id.to_lowercase()).collect();

    let is_neoforge_or_forge = instance_loader
        .map(|l| {
            let lower = l.to_ascii_lowercase();
            lower.contains("neoforge") || lower.contains("forge")
        })
        .unwrap_or(false);

    let is_fabric_or_quilt = instance_loader
        .map(|l| {
            let lower = l.to_ascii_lowercase();
            lower.contains("fabric") || lower.contains("quilt")
        })
        .unwrap_or(false);

    mods.iter()
        .enumerate()
        .map(|(own, target)| ModDependencyReport {
            file_name: target.file_name.clone(),
            mod_id: target.mod_id.clone(),
            deps: dep_rows(
                own,
                target,
                mods,
                &providers,
                &bundled,
                is_neoforge_or_forge,
                is_fabric_or_quilt,
            ),
            dependents: dependents_of(own, target, mods),
        })
        .collect()
}

/// Lowercase `mod_id` and every `provides` entry → the mods that can satisfy that id.
fn provider_index(mods: &[LocalModInfo]) -> HashMap<String, Vec<usize>> {
    let mut index: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, m) in mods.iter().enumerate() {
        let mut keys = Vec::with_capacity(1 + m.provides.len());
        keys.push(m.mod_id.to_lowercase());
        keys.extend(m.provides.iter().map(|p| p.to_lowercase()));
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            if !key.is_empty() {
                index.entry(key).or_default().push(i);
            }
        }
    }
    index
}

fn dep_rows(
    own: usize,
    target: &LocalModInfo,
    mods: &[LocalModInfo],
    providers: &HashMap<String, Vec<usize>>,
    bundled: &HashSet<String>,
    is_neoforge_or_forge: bool,
    is_fabric_or_quilt: bool,
) -> Vec<DepItem> {
    let mut rows = Vec::new();
    let mut seen: HashSet<(String, u8)> = HashSet::new();

    for dep in &target.dependencies {
        let id = dep.id.trim().to_lowercase();
        if id.is_empty() || IGNORED_IDS.contains(&id.as_str()) {
            continue;
        }
        if is_neoforge_or_forge
            && (id == "fabric"
                || id.starts_with("fabric-")
                || id.starts_with("fabric_")
                || id == "fabricloader"
                || id.starts_with("quilt"))
        {
            continue;
        }
        if is_fabric_or_quilt
            && (id == "forge"
                || id == "neoforge"
                || id == "fml"
                || id == "javafml"
                || id == "lowcodefml")
        {
            continue;
        }
        if !seen.insert((id.clone(), kind_key(dep.kind))) {
            continue;
        }
        let candidates: Vec<usize> = providers
            .get(&id)
            .map(|owners| owners.iter().copied().filter(|&i| i != own).collect())
            .unwrap_or_default();

        let row =
            match dep.kind {
                ModDepKind::Required | ModDepKind::Optional => {
                    let enabled: Vec<usize> = candidates
                        .iter()
                        .copied()
                        .filter(|&i| mods[i].enabled)
                        .collect();
                    let matching = enabled.iter().copied().find(|&i| {
                        dep.range
                            .as_deref()
                            .is_none_or(|range| satisfies(&mods[i].version, range))
                    });

                    let (status, provider) = if let Some(i) = matching {
                        (DepStatus::Installed, Some(i))
                    } else if let Some(i) = enabled.first().copied() {
                        // A provider runs, but it is not the declared version.
                        (DepStatus::VersionMismatch, Some(i))
                    } else if let Some(i) = candidates.first().copied() {
                        (DepStatus::Disabled, Some(i))
                    } else if bundled.contains(&id) {
                        (DepStatus::Bundled, None)
                    } else {
                        (DepStatus::Missing, None)
                    };

                    Some(DepItem {
                        id: dep.id.clone(),
                        kind: dep.kind,
                        range: dep.range.clone(),
                        status,
                        provider_file: provider.map(|i| mods[i].file_name.clone()),
                        provider_version: provider.map(|i| mods[i].version.clone()),
                    })
                }
                ModDepKind::Incompatible => candidates
                    .iter()
                    .copied()
                    .find(|&i| mods[i].enabled)
                    .map(|i| DepItem {
                        id: dep.id.clone(),
                        kind: dep.kind,
                        range: dep.range.clone(),
                        status: DepStatus::Conflict,
                        provider_file: Some(mods[i].file_name.clone()),
                        provider_version: Some(mods[i].version.clone()),
                    }),
            };

        if let Some(row) = row {
            rows.push(row);
        }
    }

    rows
}

/// File names of every other mod whose `Required` dependency this mod satisfies.
fn dependents_of(own: usize, target: &LocalModInfo, mods: &[LocalModInfo]) -> Vec<String> {
    let mut own_ids: Vec<String> = Vec::with_capacity(1 + target.provides.len());
    own_ids.push(target.mod_id.to_lowercase());
    own_ids.extend(target.provides.iter().map(|p| p.to_lowercase()));

    let mut dependents: Vec<String> = Vec::new();
    for (i, other) in mods.iter().enumerate() {
        if i == own {
            continue;
        }
        let declared = other.dependencies.iter().any(|dep| {
            dep.kind == ModDepKind::Required
                && own_ids.contains(&dep.id.trim().to_lowercase())
                && dep
                    .range
                    .as_deref()
                    .is_none_or(|range| satisfies(&target.version, range))
        });
        if declared {
            dependents.push(other.file_name.clone());
        }
    }
    dependents
}

/// Cheap discriminator for de-duplicating identical `(id, kind)` declarations.
fn kind_key(kind: ModDepKind) -> u8 {
    match kind {
        ModDepKind::Required => 0,
        ModDepKind::Optional => 1,
        ModDepKind::Incompatible => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mod(file: &str, id: &str, deps: Vec<ModDep>) -> LocalModInfo {
        LocalModInfo {
            path: file.to_string(),
            file_name: file.to_string(),
            mod_id: id.to_string(),
            name: id.to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: Vec::new(),
            loader: "Multi".to_string(),
            sha1: "sha".to_string(),
            murmur2: 0,
            size: 100,
            project_id: None,
            update_file: None,
            icon: None,
            dependencies: deps,
            provides: Vec::new(),
            enabled: true,
        }
    }

    #[test]
    fn test_neoforge_loader_filters_fabric_deps() {
        let multi_mod = make_mod(
            "multi.jar",
            "multi_mod",
            vec![
                ModDep {
                    id: "fabric".to_string(),
                    range: None,
                    kind: ModDepKind::Required,
                },
                ModDep {
                    id: "fabric-api".to_string(),
                    range: None,
                    kind: ModDepKind::Required,
                },
                ModDep {
                    id: "actual_dep".to_string(),
                    range: None,
                    kind: ModDepKind::Required,
                },
            ],
        );

        let reports = analyze_with_loader(&[multi_mod], &HashSet::new(), Some("neoforge 20.4.80"));
        assert_eq!(reports.len(), 1);
        // fabric and fabric-api are filtered out; only actual_dep remains as missing
        assert_eq!(reports[0].deps.len(), 1);
        assert_eq!(reports[0].deps[0].id, "actual_dep");
        assert_eq!(reports[0].deps[0].status, DepStatus::Missing);
    }

    #[test]
    fn test_fabric_loader_filters_forge_deps() {
        let multi_mod = make_mod(
            "multi.jar",
            "multi_mod",
            vec![
                ModDep {
                    id: "forge".to_string(),
                    range: None,
                    kind: ModDepKind::Required,
                },
                ModDep {
                    id: "neoforge".to_string(),
                    range: None,
                    kind: ModDepKind::Required,
                },
                ModDep {
                    id: "actual_dep".to_string(),
                    range: None,
                    kind: ModDepKind::Required,
                },
            ],
        );

        let reports = analyze_with_loader(&[multi_mod], &HashSet::new(), Some("fabric 0.15.11"));
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].deps.len(), 1);
        assert_eq!(reports[0].deps[0].id, "actual_dep");
    }
}
