//! Dependency resolution for mod community projects (Modrinth and CurseForge).

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::curseforge::CurseForgeClient;
use crate::error::CompError;
use crate::model::{
    CompDependency, CompDependencyKind, CompFile, CompPlatform, CompProject, CompReleaseChannel,
};
use crate::modrinth::ModrinthClient;

pub const MAX_RESOLVER_DEPTH: usize = 32;

/// A resolved dependency item containing project metadata, the chosen file, and whether it is already installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedDependencyItem {
    pub project: CompProject,
    pub file: CompFile,
    pub installed: bool,
}

/// True when a file can be installed into an instance running `loader`.
///
/// An empty loader list means the artifact does not declare one (datapacks, loader-agnostic
/// files) and is treated as compatible. A file that only declares other loaders is not.
pub fn file_supports_loader(loaders: &[String], loader: &str) -> bool {
    let loader = loader.trim();
    loader.is_empty()
        || loaders.is_empty()
        || loaders.iter().any(|l| l.eq_ignore_ascii_case(loader))
}

/// True when a file can be installed into an instance running any of `instance_loaders`.
///
/// An instance may legitimately carry more than one loader (Forge + OptiFine, Cleanroom + Forge),
/// and a build tagged with any of them is installable.
pub fn file_supports_any_loader(loaders: &[String], instance_loaders: &[&str]) -> bool {
    instance_loaders.is_empty()
        || loaders.is_empty()
        || loaders
            .iter()
            .any(|l| instance_loaders.iter().any(|id| l.eq_ignore_ascii_case(id)))
}

/// Selects the best compatible file for the given Minecraft version and loader.
///
/// Both filters are strict: a candidate must match the game version (when one is known) and the
/// loader. There is no "some files matched, so relax the other filter" fallback — that fallback is
/// what made a Forge-only build look like an update for a Fabric instance. When nothing is
/// compatible the answer is `None`, which callers surface as "no update".
///
/// Among compatible files: `Release` > `Beta` > `Alpha`, then newest release date.
pub fn select_best_file(files: &[CompFile], mc_version: &str, loader: &str) -> Option<CompFile> {
    select_best_file_for(files, mc_version, &[loader])
}

/// Same as [`select_best_file`] for an instance that runs several loaders at once.
pub fn select_best_file_for(
    files: &[CompFile],
    mc_version: &str,
    instance_loaders: &[&str],
) -> Option<CompFile> {
    let mc_clean = mc_version.trim();

    let mut candidates: Vec<&CompFile> = files
        .iter()
        .filter(|f| {
            mc_clean.is_empty()
                || f.mc_versions
                    .iter()
                    .any(|v| v.eq_ignore_ascii_case(mc_clean))
        })
        .filter(|f| file_supports_any_loader(&f.loaders, instance_loaders))
        .collect();

    candidates.sort_by(|a, b| {
        let channel_order = |c: CompReleaseChannel| match c {
            CompReleaseChannel::Release => 0,
            CompReleaseChannel::Beta => 1,
            CompReleaseChannel::Alpha => 2,
        };
        let c_cmp = channel_order(a.channel).cmp(&channel_order(b.channel));
        if c_cmp != std::cmp::Ordering::Equal {
            return c_cmp;
        }

        b.release_date.cmp(&a.release_date)
    });

    candidates.first().copied().cloned()
}

struct ResolverContext<'a> {
    platform: CompPlatform,
    mc_version: &'a str,
    loader: &'a str,
    installed_project_ids: &'a HashSet<String>,
    installed_mod_ids: &'a HashSet<String>,
    modrinth: &'a ModrinthClient,
    curseforge: Option<&'a CurseForgeClient>,
}

/// Recursively resolves dependencies using DFS with cycle prevention, max depth 32,
/// and checks against already installed mods.
pub async fn resolve_dependencies(
    root_file: &CompFile,
    mc_version: &str,
    loader: &str,
    installed_project_ids: &HashSet<String>,
    installed_mod_ids: &HashSet<String>,
    modrinth: &ModrinthClient,
    curseforge: Option<&CurseForgeClient>,
) -> Result<Vec<ResolvedDependencyItem>, CompError> {
    let mut results = Vec::new();
    let mut visited = HashSet::new();

    if !root_file.project_id.is_empty() {
        visited.insert(root_file.project_id.to_ascii_lowercase());
    }

    let ctx = ResolverContext {
        platform: root_file.platform,
        mc_version,
        loader,
        installed_project_ids,
        installed_mod_ids,
        modrinth,
        curseforge,
    };

    resolve_dependencies_dfs(&root_file.dependencies, &ctx, &mut visited, &mut results, 0).await?;

    Ok(results)
}

fn resolve_dependencies_dfs<'a>(
    dependencies: &'a [CompDependency],
    ctx: &'a ResolverContext<'a>,
    visited: &'a mut HashSet<String>,
    results: &'a mut Vec<ResolvedDependencyItem>,
    depth: usize,
) -> Pin<Box<dyn Future<Output = Result<(), CompError>> + Send + 'a>> {
    Box::pin(async move {
        if depth > MAX_RESOLVER_DEPTH {
            return Ok(());
        }

        for dep in dependencies {
            // Only follow Required dependencies
            if dep.kind != CompDependencyKind::Required {
                continue;
            }

            let dep_id_lower = dep.project_id.to_ascii_lowercase();
            if visited.contains(&dep_id_lower) {
                continue;
            }
            visited.insert(dep_id_lower.clone());

            let is_installed = ctx.installed_project_ids.contains(&dep_id_lower)
                || ctx.installed_mod_ids.contains(&dep_id_lower);

            // Fetch the project metadata
            let project_res = match ctx.platform {
                CompPlatform::Modrinth => ctx.modrinth.get_project(&dep.project_id).await,
                CompPlatform::Curseforge => {
                    if let Some(cf) = ctx.curseforge {
                        cf.get_project(&dep.project_id).await
                    } else {
                        Err(CompError::NotConfigured)
                    }
                }
            };

            let project = match project_res {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch dependency project {}: {}",
                        dep.project_id,
                        e
                    );
                    continue;
                }
            };

            // Also check if project slug matches installed mod IDs
            let is_installed = is_installed
                || ctx
                    .installed_mod_ids
                    .contains(&project.slug.to_ascii_lowercase())
                || ctx
                    .installed_project_ids
                    .contains(&project.id.to_ascii_lowercase());

            if is_installed {
                // Dependency is already installed. Include in results with installed: true, and do NOT recurse.
                results.push(ResolvedDependencyItem {
                    project: project.clone(),
                    file: CompFile {
                        id: String::new(),
                        project_id: dep.project_id.clone(),
                        platform: ctx.platform,
                        name: project.title.clone(),
                        file_name: String::new(),
                        mc_versions: vec![ctx.mc_version.to_string()],
                        loaders: vec![ctx.loader.to_string()],
                        channel: CompReleaseChannel::Release,
                        release_date: String::new(),
                        size: 0,
                        hashes: Default::default(),
                        download_urls: Vec::new(),
                        distribution_disabled: false,
                        dependencies: Vec::new(),
                    },
                    installed: true,
                });
                continue;
            }

            // Fetch candidate files for this project
            let files_res = match ctx.platform {
                CompPlatform::Modrinth => {
                    ctx.modrinth
                        .get_versions(&dep.project_id, Some(ctx.mc_version), Some(ctx.loader))
                        .await
                }
                CompPlatform::Curseforge => {
                    if let Some(cf) = ctx.curseforge {
                        cf.get_files(&dep.project_id, Some(ctx.mc_version), Some(ctx.loader))
                            .await
                    } else {
                        Err(CompError::NotConfigured)
                    }
                }
            };

            let files = match files_res {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!(
                        "Failed to fetch files for dependency {}: {}",
                        dep.project_id,
                        e
                    );
                    continue;
                }
            };

            if let Some(best_file) = select_best_file(&files, ctx.mc_version, ctx.loader) {
                let nested_deps = best_file.dependencies.clone();
                results.push(ResolvedDependencyItem {
                    project,
                    file: best_file,
                    installed: false,
                });

                // Recurse for nested dependencies
                resolve_dependencies_dfs(&nested_deps, ctx, visited, results, depth + 1).await?;
            }
        }

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn make_test_file(
        id: &str,
        mc: &[&str],
        loaders: &[&str],
        channel: CompReleaseChannel,
        date: &str,
    ) -> CompFile {
        CompFile {
            id: id.to_string(),
            project_id: "test_proj".to_string(),
            platform: CompPlatform::Modrinth,
            name: format!("File {id}"),
            file_name: format!("{id}.jar"),
            mc_versions: mc.iter().map(|s| s.to_string()).collect(),
            loaders: loaders.iter().map(|s| s.to_string()).collect(),
            channel,
            release_date: date.to_string(),
            size: 1000,
            hashes: CompFileHashes::default(),
            download_urls: vec!["https://example.com".to_string()],
            distribution_disabled: false,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn test_select_best_file_channel_and_date_order() {
        let files = vec![
            make_test_file(
                "alpha_new",
                &["1.20.1"],
                &["fabric"],
                CompReleaseChannel::Alpha,
                "2024-02-01",
            ),
            make_test_file(
                "beta_mid",
                &["1.20.1"],
                &["fabric"],
                CompReleaseChannel::Beta,
                "2024-01-15",
            ),
            make_test_file(
                "rel_old",
                &["1.20.1"],
                &["fabric"],
                CompReleaseChannel::Release,
                "2023-12-01",
            ),
            make_test_file(
                "rel_new",
                &["1.20.1"],
                &["fabric"],
                CompReleaseChannel::Release,
                "2024-01-01",
            ),
        ];

        let best = select_best_file(&files, "1.20.1", "fabric").unwrap();
        // Release is prioritized over Beta and Alpha, and 2024-01-01 is newer than 2023-12-01
        assert_eq!(best.id, "rel_new");
    }

    #[test]
    fn test_select_best_file_version_and_loader_filtering() {
        let files = vec![
            make_test_file(
                "f1_wrong_mc",
                &["1.19.2"],
                &["fabric"],
                CompReleaseChannel::Release,
                "2024-01-01",
            ),
            make_test_file(
                "f2_wrong_loader",
                &["1.20.1"],
                &["forge"],
                CompReleaseChannel::Release,
                "2024-01-02",
            ),
            make_test_file(
                "f3_match",
                &["1.20.1"],
                &["fabric"],
                CompReleaseChannel::Release,
                "2024-01-01",
            ),
        ];

        let best = select_best_file(&files, "1.20.1", "fabric").unwrap();
        assert_eq!(best.id, "f3_match");
    }

    #[test]
    fn select_best_file_never_falls_back_to_another_loader_or_version() {
        let other_loader = vec![make_test_file(
            "forge_only",
            &["1.20.1"],
            &["forge"],
            CompReleaseChannel::Release,
            "2024-05-01",
        )];
        // A Fabric instance must not be told a Forge-only build is its update.
        assert!(select_best_file(&other_loader, "1.20.1", "fabric").is_none());

        let other_version = vec![make_test_file(
            "fabric_121",
            &["1.21"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-05-01",
        )];
        assert!(select_best_file(&other_version, "1.20.1", "fabric").is_none());

        // Loader-agnostic artifacts (empty loader list) still qualify.
        let agnostic = vec![make_test_file(
            "datapack",
            &["1.20.1"],
            &[],
            CompReleaseChannel::Release,
            "2024-05-01",
        )];
        assert_eq!(
            select_best_file(&agnostic, "1.20.1", "fabric").map(|f| f.id),
            Some("datapack".to_string())
        );
    }

    #[test]
    fn test_resolver_visited_cycle_and_installed_satisfied() {
        // Pure algorithmic test simulating the DFS resolution state
        let mut visited = HashSet::new();
        let mut installed_project_ids = HashSet::new();
        let _installed_mod_ids: HashSet<String> = HashSet::new();

        installed_project_ids.insert("already_installed".to_string());

        let dep_cycle_1 = CompDependency {
            project_id: "cycle_a".to_string(),
            kind: CompDependencyKind::Required,
        };
        let dep_cycle_2 = CompDependency {
            project_id: "cycle_a".to_string(),
            kind: CompDependencyKind::Required,
        };
        let dep_installed = CompDependency {
            project_id: "already_installed".to_string(),
            kind: CompDependencyKind::Required,
        };

        // First visit cycle_a
        assert!(visited.insert(dep_cycle_1.project_id.clone()));
        // Second visit to cycle_a must be detected as duplicate
        assert!(!visited.insert(dep_cycle_2.project_id.clone()));

        // Installed check
        assert!(installed_project_ids.contains(&dep_installed.project_id));
    }
}
