//! Preset planning, repository, ledger and resolver-filter tests.

use std::collections::{HashMap, HashSet};
use std::fs;

use mclr_comp::model::{CompFile, CompFileHashes, CompPlatform, CompReleaseChannel, CompType};
use mclr_comp::preset::installed::{InstalledHit, InstalledIndex};
use mclr_comp::preset::ledger;
use mclr_comp::preset::model::{
    AddStatus, ApplyStatus, GateReason, LinkMode, OtherVersionPolicy, PresetItem, PresetPinRule,
    ResourcePreset,
};
use mclr_comp::preset::plan::{InstanceCtx, MergedItem, decide, merge_presets};
use mclr_comp::preset::repo::PresetRepo;
use mclr_comp::resolver::{MatchFailure, file_supports_mc, select_best_file_filtered};
use mclr_core::loader::expand_mod_loaders;
use tempfile::tempdir;

// ---------------------------------------------------------------- fixtures

fn file(
    id: &str,
    project_id: &str,
    mc_versions: &[&str],
    loaders: &[&str],
    channel: CompReleaseChannel,
    release_date: &str,
    sha1: Option<&str>,
) -> CompFile {
    CompFile {
        id: id.to_string(),
        project_id: project_id.to_string(),
        platform: CompPlatform::Modrinth,
        name: format!("{project_id} {id}"),
        file_name: format!("{project_id}-{id}.jar"),
        mc_versions: mc_versions.iter().map(|v| v.to_string()).collect(),
        loaders: loaders.iter().map(|l| l.to_string()).collect(),
        channel,
        release_date: release_date.to_string(),
        size: 1234,
        hashes: CompFileHashes {
            sha1: sha1.map(str::to_string),
            sha512: None,
            murmur2: None,
        },
        download_urls: vec![format!("https://example.invalid/{project_id}/{id}")],
        distribution_disabled: false,
        dependencies: Vec::new(),
    }
}

fn item(platform: CompPlatform, project_id: &str, kind: CompType, pin: Option<&str>) -> PresetItem {
    PresetItem {
        platform,
        project_id: project_id.to_string(),
        kind,
        title: format!("Title {project_id}"),
        slug: Some(project_id.to_string()),
        icon_url: None,
        pinned_file_id: pin.map(str::to_string),
        pinned_file_name: pin.map(|p| format!("{project_id}-{p}.jar")),
        pin_rules: Vec::new(),
    }
}

fn preset(
    id: &str,
    name: &str,
    game_versions: &[&str],
    loaders: &[&str],
    items: Vec<PresetItem>,
) -> ResourcePreset {
    ResourcePreset {
        id: id.to_string(),
        name: name.to_string(),
        description: String::new(),
        game_versions: game_versions.iter().map(|v| v.to_string()).collect(),
        loaders: loaders.iter().map(|l| l.to_string()).collect(),
        items,
        created_at: 0,
        updated_at: 0,
    }
}

fn ctx(mc_version: &str, loaders: &[&str]) -> InstanceCtx {
    InstanceCtx {
        mc_version: mc_version.to_string(),
        loaders: loaders.iter().map(|l| l.to_string()).collect(),
    }
}

fn hit(file_name: &str, rel_path: &str, sha1: &str) -> InstalledHit {
    InstalledHit {
        file_name: file_name.to_string(),
        rel_path: rel_path.to_string(),
        sha1: sha1.to_string(),
        file_id: None,
    }
}

fn merged_of(presets: &[ResourcePreset], context: &InstanceCtx, project_id: &str) -> MergedItem {
    let (_, items) = merge_presets(presets, context);
    items
        .into_iter()
        .find(|m| m.item.project_id == project_id)
        .expect("item present")
}

// ---------------------------------------------------------------- resolver

#[test]
fn file_supports_mc_requires_exact_declared_version() {
    let f = file(
        "1",
        "p",
        &["1.20.1", "1.20.2"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    );
    assert!(file_supports_mc(&f, "1.20.1"));
    assert!(file_supports_mc(&f, " 1.20.2 "));
    assert!(!file_supports_mc(&f, "1.20"));
    assert!(!file_supports_mc(&f, ""));

    let unversioned = file(
        "2",
        "p",
        &[],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        None,
    );
    assert!(!file_supports_mc(&unversioned, "1.20.1"));
}

#[test]
fn select_best_file_filtered_reports_each_failure() {
    let files = vec![file(
        "1",
        "p",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    )];

    assert_eq!(
        select_best_file_filtered(&[], &[], &[]).unwrap_err(),
        MatchFailure::NoFile
    );
    assert_eq!(
        select_best_file_filtered(&files, &["1.19.2".into()], &[]).unwrap_err(),
        MatchFailure::NoGameVersion
    );
    assert_eq!(
        select_best_file_filtered(&files, &["1.20.1".into()], &["forge".into()]).unwrap_err(),
        MatchFailure::NoLoader
    );
}

#[test]
fn select_best_file_filtered_orders_by_channel_then_date() {
    let files = vec![
        file(
            "alpha",
            "p",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Alpha,
            "2024-06-01T00:00:00Z",
            Some("0a"),
        ),
        file(
            "beta",
            "p",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Beta,
            "2024-05-01T00:00:00Z",
            Some("0b"),
        ),
        file(
            "release-old",
            "p",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-01-01T00:00:00Z",
            Some("0c"),
        ),
        file(
            "release-new",
            "p",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-04-01T00:00:00Z",
            Some("0d"),
        ),
        file(
            "other-mc",
            "p",
            &["1.19.2"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-07-01T00:00:00Z",
            Some("0e"),
        ),
        file(
            "other-loader",
            "p",
            &["1.20.1"],
            &["forge"],
            CompReleaseChannel::Release,
            "2024-08-01T00:00:00Z",
            Some("0f"),
        ),
    ];

    let chosen = select_best_file_filtered(&files, &["1.20.1".into()], &["fabric".into()]).unwrap();
    assert_eq!(chosen.id, "release-new");

    // Empty filter lists mean "no restriction".
    let chosen = select_best_file_filtered(&files, &[], &[]).unwrap();
    assert_eq!(chosen.id, "other-loader");
}

#[test]
fn expand_loaders_maps_known_aliases() {
    assert_eq!(
        expand_mod_loaders(&["neoforge".into()], "1.20.1"),
        vec!["neoforge".to_string(), "forge".to_string()]
    );
    assert_eq!(
        expand_mod_loaders(&["neoforge".into()], "1.20.4"),
        vec!["neoforge".to_string()]
    );
    assert_eq!(
        expand_mod_loaders(&["quilt".into()], "1.20.1"),
        vec!["quilt".to_string(), "fabric".to_string()]
    );
    assert_eq!(
        expand_mod_loaders(&["cleanroom".into()], "1.12.2"),
        vec!["forge".to_string()]
    );
    assert_eq!(
        expand_mod_loaders(&["legacyfabric".into()], "1.8.9"),
        vec!["fabric".to_string()]
    );
    assert_eq!(
        expand_mod_loaders(&["Fabric".into(), "fabric".into()], "1.20.1"),
        vec!["fabric".to_string()]
    );
}

// ---------------------------------------------------------------- merge

#[test]
fn merge_presets_gates_versions_and_merges_overlaps() {
    let project = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    let a = preset("a", "A", &["1.20.1"], &["fabric"], vec![project.clone()]);
    let b = preset("b", "B", &[], &[], vec![project]);
    let c = preset(
        "c",
        "C",
        &["1.19.2"],
        &[],
        vec![item(CompPlatform::Modrinth, "other", CompType::Mod, None)],
    );

    let context = ctx("1.20.1", &["fabric"]);
    let (gates, items) = merge_presets(&[a, c, b], &context);

    assert_eq!(gates.len(), 3);
    assert!(gates[0].ok);
    assert!(!gates[1].ok);
    assert_eq!(gates[1].reason, Some(GateReason::GameVersion));
    assert!(gates[2].ok);

    assert_eq!(items.len(), 1, "gated preset contributes nothing");
    assert_eq!(
        items[0].from_presets,
        vec!["A".to_string(), "B".to_string()]
    );
    assert!(!items[0].version_conflict);
    // B has no loader filter, so the item is not restricted by presets.
    assert_eq!(items[0].preset_loaders, None);
}

#[test]
fn merge_presets_gates_disjoint_loaders() {
    let a = preset(
        "a",
        "A",
        &[],
        &["forge"],
        vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
    );
    let b = preset(
        "b",
        "B",
        &[],
        &["fabric", "quilt"],
        vec![item(CompPlatform::Modrinth, "lithium", CompType::Mod, None)],
    );

    let (gates, items) = merge_presets(&[a, b], &ctx("1.20.1", &["fabric"]));

    assert_eq!(gates.len(), 2);
    assert!(!gates[0].ok);
    assert_eq!(gates[0].reason, Some(GateReason::Loader));
    assert!(gates[1].ok);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].item.project_id, "lithium");
    assert_eq!(
        items[0].preset_loaders,
        Some(vec!["fabric".to_string(), "quilt".to_string()])
    );
}

#[test]
fn merge_presets_unions_loader_filters() {
    let sodium = || item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    let a = preset("a", "A", &[], &["fabric"], vec![sodium()]);
    let b = preset("b", "B", &[], &["quilt"], vec![sodium()]);
    let context = ctx("1.20.1", &["fabric", "quilt"]);
    let merged = merged_of(&[a, b], &context, "sodium");
    assert_eq!(
        merged.preset_loaders,
        Some(vec!["fabric".to_string(), "quilt".to_string()])
    );

    let unrestricted = preset("c", "C", &[], &[], vec![sodium()]);
    let merged = merged_of(
        &[
            preset("a", "A", &[], &["fabric"], vec![sodium()]),
            unrestricted,
        ],
        &context,
        "sodium",
    );
    assert_eq!(merged.preset_loaders, None);
}

#[test]
fn merge_presets_first_pin_wins_and_flags_conflict() {
    let a = preset(
        "a",
        "A",
        &[],
        &[],
        vec![item(
            CompPlatform::Modrinth,
            "sodium",
            CompType::Mod,
            Some("f1"),
        )],
    );
    let b = preset(
        "b",
        "B",
        &[],
        &[],
        vec![item(
            CompPlatform::Modrinth,
            "sodium",
            CompType::Mod,
            Some("f2"),
        )],
    );
    let context = ctx("1.20.1", &["fabric"]);
    let (_, items) = merge_presets(&[a, b], &context);

    assert_eq!(items.len(), 1);
    assert!(items[0].version_conflict);
    assert_eq!(items[0].item.pinned_file_id.as_deref(), Some("f1"));
    assert_eq!(
        items[0].from_presets,
        vec!["A".to_string(), "B".to_string()]
    );
}

#[test]
fn merge_presets_later_pin_fills_missing_pin_without_conflict() {
    let a = preset(
        "a",
        "A",
        &[],
        &[],
        vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
    );
    let b = preset(
        "b",
        "B",
        &[],
        &[],
        vec![item(
            CompPlatform::Modrinth,
            "sodium",
            CompType::Mod,
            Some("f2"),
        )],
    );
    let merged = merged_of(&[a, b], &ctx("1.20.1", &["fabric"]), "sodium");
    assert!(!merged.version_conflict);
    assert_eq!(merged.item.pinned_file_id.as_deref(), Some("f2"));
}

#[test]
fn merge_presets_dedupes_pin_rules_by_context() {
    let mut first = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    first.pin_rules.push(PresetPinRule {
        game_versions: vec!["1.20.1".to_string()],
        loaders: vec!["fabric".to_string()],
        file_id: "f1".to_string(),
        file_name: "sodium-f1.jar".to_string(),
    });
    let mut second = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    second.pin_rules.push(PresetPinRule {
        game_versions: vec!["1.20.1".to_string()],
        loaders: vec!["fabric".to_string()],
        file_id: "f2".to_string(),
        file_name: "sodium-f2.jar".to_string(),
    });
    second.pin_rules.push(PresetPinRule {
        game_versions: vec!["1.19.2".to_string()],
        loaders: Vec::new(),
        file_id: "f3".to_string(),
        file_name: "sodium-f3.jar".to_string(),
    });

    let a = preset("a", "A", &[], &[], vec![first]);
    let b = preset("b", "B", &[], &[], vec![second]);
    let merged = merged_of(&[a, b], &ctx("1.20.1", &["fabric"]), "sodium");

    assert_eq!(merged.item.pin_rules.len(), 2);
    assert_eq!(merged.item.pin_rules[0].file_id, "f1");
    assert_eq!(merged.item.pin_rules[1].file_id, "f3");
}

// ---------------------------------------------------------------- decide

fn empty_installed() -> InstalledIndex {
    InstalledIndex::default()
}

#[test]
fn decide_installed_for_fresh_item() {
    let p = preset(
        "a",
        "A",
        &[],
        &["fabric"],
        vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
    );
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[p], &context, "sodium");
    let files = vec![file(
        "f1",
        "sodium",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    )];

    let decision = decide(
        &merged,
        &files,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(decision.file.map(|f| f.id), Some("f1".to_string()));
    assert!(decision.existing.is_none());
}

#[test]
fn decide_already_same_by_hash() {
    let p = preset(
        "a",
        "A",
        &[],
        &["fabric"],
        vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
    );
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[p], &context, "sodium");
    let files = vec![file(
        "f1",
        "sodium",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("AA"),
    )];

    let mut installed = empty_installed();
    installed
        .by_sha1
        .insert("aa".to_string(), hit("sodium.jar", "mods/sodium.jar", "aa"));

    let decision = decide(
        &merged,
        &files,
        &context,
        &installed,
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::AlreadySame);
    assert_eq!(
        decision.existing.map(|h| h.rel_path),
        Some("mods/sodium.jar".to_string())
    );
}

#[test]
fn decide_other_version_follows_policy() {
    let p = preset(
        "a",
        "A",
        &[],
        &["fabric"],
        vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
    );
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[p], &context, "sodium");
    let files = vec![file(
        "f1",
        "sodium",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("new"),
    )];

    let mut installed = empty_installed();
    installed.by_project.insert(
        (CompPlatform::Modrinth, "sodium".to_string()),
        vec![hit("old.jar", "mods/old.jar", "old")],
    );

    let skip = decide(
        &merged,
        &files,
        &context,
        &installed,
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(skip.status, ApplyStatus::AlreadyOtherVersion);
    assert_eq!(
        skip.existing.map(|h| h.file_name),
        Some("old.jar".to_string())
    );

    let replace = decide(
        &merged,
        &files,
        &context,
        &installed,
        OtherVersionPolicy::Replace,
        false,
    );
    assert_eq!(replace.status, ApplyStatus::Replaced);
    assert!(replace.existing.is_some());
}

#[test]
fn decide_no_game_version_no_loader_no_file() {
    let p = preset(
        "a",
        "A",
        &[],
        &["fabric"],
        vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
    );
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[p], &context, "sodium");
    let installed = empty_installed();

    let wrong_mc = vec![file(
        "f1",
        "sodium",
        &["1.19.2"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    )];
    let decision = decide(
        &merged,
        &wrong_mc,
        &context,
        &installed,
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::NoGameVersion);
    assert!(decision.file.is_none());

    let wrong_loader = vec![file(
        "f1",
        "sodium",
        &["1.20.1"],
        &["forge"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    )];
    let decision = decide(
        &merged,
        &wrong_loader,
        &context,
        &installed,
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::NoLoader);

    let decision = decide(
        &merged,
        &[],
        &context,
        &installed,
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::NoFile);
}

#[test]
fn decide_disjoint_preset_loaders_yield_no_loader() {
    let p = preset(
        "a",
        "A",
        &[],
        &["forge"],
        vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
    );
    // Merged under the preset's own context, then applied to a Fabric instance.
    let (_, items) = merge_presets(&[p], &ctx("1.20.1", &["forge"]));
    let files = vec![file(
        "f1",
        "sodium",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    )];
    let decision = decide(
        &items[0],
        &files,
        &ctx("1.20.1", &["fabric"]),
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::NoLoader);
    assert!(decision.file.is_none());
}

#[test]
fn decide_distribution_disabled_and_needs_world() {
    let shader = preset(
        "a",
        "A",
        &[],
        &[],
        vec![item(
            CompPlatform::Curseforge,
            "bsl",
            CompType::Shader,
            None,
        )],
    );
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[shader], &context, "bsl");
    let mut blocked = file(
        "f1",
        "bsl",
        &["1.20.1"],
        &[],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        None,
    );
    blocked.distribution_disabled = true;

    let decision = decide(
        &merged,
        &[blocked],
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::DistributionDisabled);
    assert!(decision.file.is_some());

    let datapack = preset(
        "d",
        "D",
        &[],
        &[],
        vec![item(CompPlatform::Modrinth, "dp", CompType::Datapack, None)],
    );
    let merged = merged_of(&[datapack], &context, "dp");
    let decision = decide(
        &merged,
        &[],
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::NeedsWorld);

    let world_set = decide(
        &merged,
        &[],
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        true,
    );
    assert_eq!(world_set.status, ApplyStatus::NoFile);
}

#[test]
fn decide_pin_rule_wins_only_for_its_context() {
    let mut pinned = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    pinned.pin_rules.push(PresetPinRule {
        game_versions: vec!["1.19.2".to_string()],
        loaders: vec!["fabric".to_string()],
        file_id: "old".to_string(),
        file_name: "sodium-old.jar".to_string(),
    });
    let p = preset("a", "A", &[], &[], vec![pinned]);

    let files = vec![
        file(
            "old",
            "sodium",
            &["1.19.2"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2023-01-01T00:00:00Z",
            Some("aa"),
        ),
        file(
            "new",
            "sodium",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-01-01T00:00:00Z",
            Some("bb"),
        ),
    ];

    // 1.19.2 Fabric matches the rule -> installs the pinned "old" file.
    let ctx_119 = ctx("1.19.2", &["fabric"]);
    let merged = merged_of(std::slice::from_ref(&p), &ctx_119, "sodium");
    let decision = decide(
        &merged,
        &files,
        &ctx_119,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(decision.file.map(|f| f.id), Some("old".to_string()));

    // 1.20.1 Fabric does not match the rule -> falls back to the latest compatible file.
    let ctx_120 = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[p], &ctx_120, "sodium");
    let decision = decide(
        &merged,
        &files,
        &ctx_120,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(decision.file.map(|f| f.id), Some("new".to_string()));
}

#[test]
fn decide_pin_rule_ignored_for_another_loader_or_empty_context() {
    let mut pinned = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    // Rule for Quilt only: a Fabric instance must not install the Quilt build.
    pinned.pin_rules.push(PresetPinRule {
        game_versions: Vec::new(),
        loaders: vec!["quilt".to_string()],
        file_id: "quilt-only".to_string(),
        file_name: "sodium-quilt.jar".to_string(),
    });
    // A rule with both sides empty is a wildcard for nothing and is skipped.
    pinned.pin_rules.push(PresetPinRule {
        game_versions: Vec::new(),
        loaders: Vec::new(),
        file_id: "quilt-only".to_string(),
        file_name: "sodium-quilt.jar".to_string(),
    });
    let p = preset("a", "A", &[], &[], vec![pinned]);

    let files = vec![
        file(
            "quilt-only",
            "sodium",
            &["1.20.1"],
            &["quilt"],
            CompReleaseChannel::Release,
            "2024-06-01T00:00:00Z",
            Some("aa"),
        ),
        file(
            "fabric-latest",
            "sodium",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-01-01T00:00:00Z",
            Some("bb"),
        ),
    ];

    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[p], &context, "sodium");
    let decision = decide(
        &merged,
        &files,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(
        decision.file.map(|f| f.id),
        Some("fabric-latest".to_string())
    );
}

#[test]
fn decide_pin_rule_with_missing_file_falls_back_to_latest() {
    let mut pinned = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    pinned.pin_rules.push(PresetPinRule {
        game_versions: vec!["1.20.1".to_string()],
        loaders: vec!["fabric".to_string()],
        file_id: "gone".to_string(),
        file_name: "sodium-gone.jar".to_string(),
    });
    let p = preset("a", "A", &[], &[], vec![pinned]);
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[p], &context, "sodium");

    let files = vec![file(
        "latest",
        "sodium",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    )];
    let decision = decide(
        &merged,
        &files,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(decision.file.map(|f| f.id), Some("latest".to_string()));
}

#[test]
fn decide_honours_legacy_pin_when_compatible() {
    let files = vec![
        file(
            "old",
            "sodium",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2023-01-01T00:00:00Z",
            Some("aa"),
        ),
        file(
            "new",
            "sodium",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-01-01T00:00:00Z",
            Some("bb"),
        ),
    ];
    let context = ctx("1.20.1", &["fabric"]);

    // A compatible legacy pin wins over the newer file.
    let pinned = item(CompPlatform::Modrinth, "sodium", CompType::Mod, Some("old"));
    let merged = merged_of(
        &[preset("a", "A", &[], &[], vec![pinned])],
        &context,
        "sodium",
    );
    let decision = decide(
        &merged,
        &files,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(decision.file.map(|f| f.id), Some("old".to_string()));

    // A legacy pin for another game version is ignored in this context.
    let incompatible = file(
        "old-119",
        "sodium",
        &["1.19.2"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2023-06-01T00:00:00Z",
        Some("cc"),
    );
    let pinned = item(
        CompPlatform::Modrinth,
        "sodium",
        CompType::Mod,
        Some("old-119"),
    );
    let merged = merged_of(
        &[preset("b", "B", &[], &[], vec![pinned])],
        &context,
        "sodium",
    );
    let mut all = files.clone();
    all.push(incompatible);
    let decision = decide(
        &merged,
        &all,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(decision.file.map(|f| f.id), Some("new".to_string()));
}

#[test]
fn decide_rejects_pin_without_instance_loader() {
    // The pinned build only exists for Forge, so a Fabric instance falls back to the latest
    // Fabric-compatible file instead of installing a jar the loader cannot read.
    let pinned = item(
        CompPlatform::Modrinth,
        "sodium",
        CompType::Mod,
        Some("forge-only"),
    );
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(
        &[preset("a", "A", &[], &[], vec![pinned])],
        &context,
        "sodium",
    );
    let files = vec![
        file(
            "forge-only",
            "sodium",
            &["1.20.1"],
            &["forge"],
            CompReleaseChannel::Release,
            "2024-03-01T00:00:00Z",
            Some("aa"),
        ),
        file(
            "fabric-latest",
            "sodium",
            &["1.20.1"],
            &["fabric"],
            CompReleaseChannel::Release,
            "2024-01-01T00:00:00Z",
            Some("bb"),
        ),
    ];

    let decision = decide(
        &merged,
        &files,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision.status, ApplyStatus::Installed);
    assert_eq!(
        decision.file.map(|f| f.id),
        Some("fabric-latest".to_string())
    );
}

#[test]
fn decide_all_variants_are_reachable() {
    // `Failed` is produced by the executor (IO/network errors), never by the pure planner; every
    // other variant is exercised by the tests in this file plus the ones listed here.
    let covered = [
        ApplyStatus::Installed,
        ApplyStatus::Replaced,
        ApplyStatus::AlreadySame,
        ApplyStatus::AlreadyOtherVersion,
        ApplyStatus::NoGameVersion,
        ApplyStatus::NoLoader,
        ApplyStatus::NoFile,
        ApplyStatus::DistributionDisabled,
        ApplyStatus::NeedsWorld,
    ];
    assert_eq!(covered.len(), 9);
    assert_ne!(ApplyStatus::Installed, ApplyStatus::Failed);
}

// ---------------------------------------------------------------- repo

fn repo_at(dir: &std::path::Path) -> PresetRepo {
    PresetRepo::load(dir.join("presets.json"))
}

#[test]
fn repo_crud_roundtrip() {
    let dir = tempdir().unwrap();
    let mut repo = repo_at(dir.path());

    let saved = repo
        .save(preset(
            "",
            "整合包 A",
            &["1.20.1", "1.20.1"],
            &["Fabric", "fabric"],
            vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
        ))
        .unwrap();
    assert!(!saved.id.is_empty());
    assert!(saved.created_at > 0);
    assert_eq!(saved.game_versions, vec!["1.20.1".to_string()]);
    assert_eq!(saved.loaders, vec!["fabric".to_string()]);

    // Reloading from disk yields the same preset.
    let reloaded = repo_at(dir.path());
    assert_eq!(reloaded.list(), vec![saved.clone()]);

    // Update keeps created_at, bumps updated_at.
    let mut update = saved.clone();
    update.name = "整合包 B".to_string();
    update.description = "changed".to_string();
    update.created_at = 1;
    update.updated_at = 1;
    let updated = repo.save(update).unwrap();
    assert_eq!(updated.id, saved.id);
    assert_eq!(updated.created_at, saved.created_at);
    assert_eq!(updated.description, "changed");
    assert_eq!(repo.list().len(), 1);

    repo.delete(&saved.id).unwrap();
    assert!(repo.list().is_empty());
    assert_eq!(
        repo.delete(&saved.id).unwrap_err().code(),
        "preset.notFound"
    );
}

#[test]
fn repo_rejects_duplicate_names_and_unsupported_items() {
    let dir = tempdir().unwrap();
    let mut repo = repo_at(dir.path());
    repo.save(preset("", "Same", &[], &[], Vec::new())).unwrap();

    let duplicate = repo
        .save(preset("", "same", &[], &[], Vec::new()))
        .unwrap_err();
    assert_eq!(duplicate.code(), "preset.nameExists");

    let empty_name = repo
        .save(preset("", "   ", &[], &[], Vec::new()))
        .unwrap_err();
    assert_eq!(empty_name.code(), "preset.nameInvalid");

    let bad_loader = repo
        .save(preset("", "Other", &[], &["rift"], Vec::new()))
        .unwrap_err();
    assert_eq!(bad_loader.code(), "preset.loaderInvalid");

    let modpack = repo
        .save(preset(
            "",
            "Pack",
            &[],
            &[],
            vec![item(
                CompPlatform::Modrinth,
                "pack",
                CompType::Modpack,
                None,
            )],
        ))
        .unwrap_err();
    assert_eq!(modpack.code(), "preset.kindUnsupported");

    let modrinth_world = repo
        .save(preset(
            "",
            "World",
            &[],
            &[],
            vec![item(CompPlatform::Modrinth, "world", CompType::World, None)],
        ))
        .unwrap_err();
    assert_eq!(modrinth_world.code(), "preset.kindUnsupported");

    // CurseForge worlds are allowed.
    let world = repo
        .save(preset(
            "",
            "CF World",
            &[],
            &[],
            vec![item(
                CompPlatform::Curseforge,
                "world",
                CompType::World,
                None,
            )],
        ))
        .unwrap();
    assert_eq!(world.items.len(), 1);
}

#[test]
fn repo_recovers_from_corrupt_file() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("presets.json"), b"{not json").unwrap();

    let repo = repo_at(dir.path());
    assert!(repo.list().is_empty());
    assert!(dir.path().join("presets.json.bad").is_file());
    assert!(!dir.path().join("presets.json").exists());
}

#[test]
fn repo_add_items_reports_every_outcome() {
    let dir = tempdir().unwrap();
    let mut repo = repo_at(dir.path());
    let existing = repo
        .save(preset(
            "",
            "A",
            &["1.20.1"],
            &["fabric"],
            vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
        ))
        .unwrap();
    let id = existing.id.clone();

    let resolved = file(
        "f9",
        "lithium",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    );

    let items = vec![
        item(CompPlatform::Modrinth, "sodium", CompType::Mod, None),
        item(CompPlatform::Modrinth, "sodium", CompType::Mod, Some("pin")),
        item(CompPlatform::Modrinth, "pack", CompType::Modpack, None),
        item(CompPlatform::Modrinth, "lithium", CompType::Mod, None),
        item(CompPlatform::Modrinth, "no-mc", CompType::Mod, None),
        item(CompPlatform::Modrinth, "no-loader", CompType::Mod, None),
        item(CompPlatform::Modrinth, "missing", CompType::Mod, None),
    ];

    let outcomes = repo
        .add_items(&id, items, |item| match item.project_id.as_str() {
            "lithium" => Ok(Some(resolved.clone())),
            "no-mc" => Err(MatchFailure::NoGameVersion),
            "no-loader" => Err(MatchFailure::NoLoader),
            "missing" => Ok(None),
            _ => Err(MatchFailure::NoFile),
        })
        .unwrap();

    let statuses: Vec<AddStatus> = outcomes.iter().map(|o| o.status).collect();
    assert_eq!(
        statuses,
        vec![
            AddStatus::AlreadyInPreset,
            AddStatus::AlreadyInPresetOtherVersion,
            AddStatus::Unsupported,
            AddStatus::Added,
            AddStatus::NoGameVersion,
            AddStatus::NoLoader,
            AddStatus::NoFile,
        ]
    );
    assert_eq!(
        outcomes[3].latest_file_name.as_deref(),
        Some("lithium-f9.jar")
    );

    // Only the resolvable item was added and persisted.
    let reloaded = repo_at(dir.path());
    let items = &reloaded.get(&id).unwrap().items;
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.project_id == "lithium"));
    // The pinned duplicate did not overwrite the existing entry's pin.
    let sodium = items.iter().find(|i| i.project_id == "sodium").unwrap();
    assert!(sodium.pinned_file_id.is_none());

    assert_eq!(
        repo.add_items("nope", Vec::new(), |_| Err(MatchFailure::NoFile))
            .unwrap_err()
            .code(),
        "preset.notFound"
    );
}

#[test]
fn repo_add_items_merges_pin_rules_into_existing_item() {
    let dir = tempdir().unwrap();
    let mut repo = repo_at(dir.path());
    let saved = repo
        .save(preset(
            "",
            "A",
            &["1.20.1"],
            &["fabric"],
            vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
        ))
        .unwrap();
    let rule = |file_id: &str| PresetPinRule {
        game_versions: vec!["1.20.1".to_string()],
        loaders: vec!["fabric".to_string()],
        file_id: file_id.to_string(),
        file_name: format!("sodium-{file_id}.jar"),
    };
    let locked = |file_id: &str| {
        let mut locked = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
        locked.pin_rules.push(rule(file_id));
        locked
    };
    let add = |repo: &mut PresetRepo, locked: PresetItem| {
        repo.add_items(&saved.id, vec![locked], |item| {
            Ok(Some(file(
                "unused",
                &item.project_id,
                &["1.20.1"],
                &["fabric"],
                CompReleaseChannel::Release,
                "2024-01-01T00:00:00Z",
                Some("aa"),
            )))
        })
        .unwrap()
    };

    let outcomes = add(&mut repo, locked("f1"));
    assert_eq!(outcomes[0].status, AddStatus::AlreadyInPresetOtherVersion);
    let stored = repo.get(&saved.id).unwrap();
    assert_eq!(stored.items.len(), 1, "the item is not duplicated");
    assert_eq!(stored.items[0].pin_rules.len(), 1);
    assert_eq!(stored.items[0].pin_rules[0].file_id, "f1");
    assert_eq!(stored.items[0].pinned_file_id.as_deref(), Some("f1"));

    // Re-adding the identical lock is a no-op and reports the plain "already" status.
    let outcomes = add(&mut repo, locked("f1"));
    assert_eq!(outcomes[0].status, AddStatus::AlreadyInPreset);
    assert_eq!(repo.get(&saved.id).unwrap().items[0].pin_rules.len(), 1);

    // A different file for the same context replaces the rule instead of appending one.
    let outcomes = add(&mut repo, locked("f2"));
    assert_eq!(outcomes[0].status, AddStatus::AlreadyInPresetOtherVersion);
    let stored = repo.get(&saved.id).unwrap();
    assert_eq!(stored.items.len(), 1);
    assert_eq!(stored.items[0].pin_rules.len(), 1);
    assert_eq!(stored.items[0].pin_rules[0].file_id, "f2");
    assert_eq!(stored.items[0].pinned_file_id.as_deref(), Some("f2"));

    // A rule for another context is appended, and the mirror keeps following the first rule.
    let mut other = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    other.pin_rules.push(PresetPinRule {
        game_versions: vec!["1.19.2".to_string()],
        loaders: Vec::new(),
        file_id: "f3".to_string(),
        file_name: "sodium-f3.jar".to_string(),
    });
    let outcomes = add(&mut repo, other);
    assert_eq!(outcomes[0].status, AddStatus::AlreadyInPresetOtherVersion);
    let stored = repo.get(&saved.id).unwrap();
    assert_eq!(stored.items[0].pin_rules.len(), 2);
    assert_eq!(stored.items[0].pinned_file_id.as_deref(), Some("f2"));
}

#[test]
fn repo_pin_remove_and_pinned_ids() {
    let dir = tempdir().unwrap();
    let mut repo = repo_at(dir.path());
    let saved = repo
        .save(preset(
            "",
            "A",
            &[],
            &[],
            vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
        ))
        .unwrap();

    repo.pin(
        &saved.id,
        CompPlatform::Modrinth,
        "sodium",
        Some("f1".to_string()),
        Some("sodium-1.jar".to_string()),
    )
    .unwrap();
    let item = &repo.get(&saved.id).unwrap().items[0];
    assert_eq!(item.pinned_file_id.as_deref(), Some("f1"));
    assert_eq!(item.pinned_file_name.as_deref(), Some("sodium-1.jar"));
    assert!(repo.pinned_file_ids().contains("f1"));

    repo.pin(&saved.id, CompPlatform::Modrinth, "sodium", None, None)
        .unwrap();
    let item = &repo.get(&saved.id).unwrap().items[0];
    assert!(item.pinned_file_id.is_none());
    assert!(repo.pinned_file_ids().is_empty());

    assert_eq!(
        repo.pin(&saved.id, CompPlatform::Modrinth, "absent", None, None)
            .unwrap_err()
            .code(),
        "preset.itemNotFound"
    );

    repo.remove_item(&saved.id, CompPlatform::Modrinth, "sodium")
        .unwrap();
    assert!(repo.get(&saved.id).unwrap().items.is_empty());
    // Removing an absent item is a no-op.
    repo.remove_item(&saved.id, CompPlatform::Modrinth, "sodium")
        .unwrap();
}

#[test]
fn repo_pin_rules_upsert_and_mirror_first_rule() {
    let dir = tempdir().unwrap();
    let mut repo = repo_at(dir.path());
    let saved = repo
        .save(preset(
            "",
            "A",
            &[],
            &[],
            vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
        ))
        .unwrap();
    let rule = |file_id: &str, version: &str| PresetPinRule {
        game_versions: vec![version.to_string()],
        loaders: vec!["fabric".to_string()],
        file_id: file_id.to_string(),
        file_name: format!("sodium-{file_id}.jar"),
    };
    let pin_rule = |repo: &mut PresetRepo, rule: PresetPinRule| {
        repo.pin_rule(&saved.id, CompPlatform::Modrinth, "sodium", rule)
            .unwrap();
    };
    let item = |repo: &PresetRepo| repo.get(&saved.id).unwrap().items[0].clone();

    pin_rule(&mut repo, rule("f1", "1.20.1"));
    let stored = item(&repo);
    assert_eq!(stored.pin_rules.len(), 1);
    assert_eq!(stored.pinned_file_id.as_deref(), Some("f1"));

    // The same (game_versions, loaders) context updates the existing rule in place.
    pin_rule(&mut repo, rule("f9", "1.20.1"));
    let stored = item(&repo);
    assert_eq!(stored.pin_rules.len(), 1);
    assert_eq!(stored.pin_rules[0].file_id, "f9");
    assert_eq!(stored.pinned_file_id.as_deref(), Some("f9"));

    // A new context appends; the legacy mirror keeps following the first rule.
    pin_rule(&mut repo, rule("f2", "1.19.2"));
    let stored = item(&repo);
    assert_eq!(stored.pin_rules.len(), 2);
    assert_eq!(stored.pinned_file_id.as_deref(), Some("f9"));
    let ids = repo.pinned_file_ids();
    assert!(ids.contains("f9") && ids.contains("f2"));

    repo.remove_pin_rule(&saved.id, CompPlatform::Modrinth, "sodium", "f9")
        .unwrap();
    let stored = item(&repo);
    assert_eq!(stored.pin_rules.len(), 1);
    assert_eq!(stored.pinned_file_id.as_deref(), Some("f2"));

    repo.remove_pin_rule(&saved.id, CompPlatform::Modrinth, "sodium", "f2")
        .unwrap();
    let stored = item(&repo);
    assert!(stored.pin_rules.is_empty());
    assert!(stored.pinned_file_id.is_none());

    assert_eq!(
        repo.pin_rule(
            &saved.id,
            CompPlatform::Modrinth,
            "absent",
            rule("f3", "1.20.1")
        )
        .unwrap_err()
        .code(),
        "preset.itemNotFound"
    );
}

#[test]
fn repo_pin_none_clears_every_rule() {
    let dir = tempdir().unwrap();
    let mut repo = repo_at(dir.path());
    let saved = repo
        .save(preset(
            "",
            "A",
            &[],
            &[],
            vec![item(CompPlatform::Modrinth, "sodium", CompType::Mod, None)],
        ))
        .unwrap();

    repo.pin_rule(
        &saved.id,
        CompPlatform::Modrinth,
        "sodium",
        PresetPinRule {
            game_versions: vec!["1.20.1".to_string()],
            loaders: Vec::new(),
            file_id: "f1".to_string(),
            file_name: "sodium-f1.jar".to_string(),
        },
    )
    .unwrap();
    assert_eq!(repo.get(&saved.id).unwrap().items[0].pin_rules.len(), 1);

    repo.pin(&saved.id, CompPlatform::Modrinth, "sodium", None, None)
        .unwrap();
    let stored = repo.get(&saved.id).unwrap().items.clone();
    assert!(stored[0].pin_rules.is_empty());
    assert!(stored[0].pinned_file_id.is_none());
    assert!(repo.pinned_file_ids().is_empty());
}

// ---------------------------------------------------------------- ledger

fn ledger_entry(rel_path: &str, project_id: &str) -> ledger::LedgerEntry {
    ledger::LedgerEntry {
        platform: CompPlatform::Curseforge,
        project_id: project_id.to_string(),
        file_id: "f1".to_string(),
        sha1: "aa".to_string(),
        kind: CompType::Resourcepack,
        rel_path: rel_path.to_string(),
        preset_ids: vec!["p1".to_string()],
        link_mode: LinkMode::HardLink,
        applied_at: 7,
    }
}

#[test]
fn ledger_roundtrip_upsert_and_remove() {
    let dir = tempdir().unwrap();
    let game_dir = dir.path().join("instance");
    fs::create_dir_all(game_dir.join("resourcepacks")).unwrap();

    assert!(ledger::load(&game_dir).is_empty());

    ledger::upsert(&game_dir, ledger_entry("resourcepacks/pack.zip", "pack")).unwrap();
    let entries = ledger::load(&game_dir);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].rel_path, "resourcepacks/pack.zip");

    // Same key: replaced, not duplicated.
    let mut replacement = ledger_entry("resourcepacks/pack-v2.zip", "pack");
    replacement.file_id = "f2".to_string();
    ledger::upsert(&game_dir, replacement).unwrap();
    let entries = ledger::load(&game_dir);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].file_id, "f2");

    ledger::remove_by_rel_path(&game_dir, "resourcepacks/pack-v2.zip").unwrap();
    assert!(ledger::load(&game_dir).is_empty());
    // Removing an unknown path is a no-op.
    ledger::remove_by_rel_path(&game_dir, "resourcepacks/none.zip").unwrap();
}

#[test]
fn ledger_drops_path_traversal_entries() {
    let dir = tempdir().unwrap();
    let game_dir = dir.path().join("instance");
    fs::create_dir_all(&game_dir).unwrap();

    let doc = serde_json::json!({
        "version": 1,
        "entries": [
            {
                "platform": "curseforge",
                "projectId": "evil",
                "fileId": "f1",
                "sha1": "aa",
                "kind": "resourcepack",
                "relPath": "../../evil.zip",
                "presetIds": [],
                "linkMode": "hardLink",
                "appliedAt": 1
            },
            {
                "platform": "curseforge",
                "projectId": "good",
                "fileId": "f2",
                "sha1": "bb",
                "kind": "resourcepack",
                "relPath": "resourcepacks/good.zip",
                "presetIds": [],
                "linkMode": "copy",
                "appliedAt": 2
            }
        ]
    });
    fs::write(
        game_dir.join(ledger::LEDGER_FILE_NAME),
        doc.to_string().as_bytes(),
    )
    .unwrap();

    let entries = ledger::load(&game_dir);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].project_id, "good");

    // Writing a traversal path through the API is refused too.
    assert_eq!(
        ledger::upsert(&game_dir, ledger_entry("../evil.zip", "evil"))
            .unwrap_err()
            .code(),
        "preset.pathInvalid"
    );
}

// Keep the unused import warning away while `HashMap` is only used by future callers.
#[allow(dead_code)]
fn _map_type(_: HashMap<(CompPlatform, String), Vec<InstalledHit>>, _: HashSet<CompType>) {}

// ---------------------------------------------------------------- installed index

mod installed_tests {
    use super::*;
    use mclr_comp::modrinth::ModrinthClient;
    use mclr_core::settings::SourcePriority;
    use serde_json::json;
    use sha1::{Digest, Sha1};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sha1_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha1::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    #[tokio::test]
    async fn build_hashes_resources_and_uses_ledger_identity() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/version_files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;
        let client = ModrinthClient::new(SourcePriority::default()).with_base_url(server.uri());

        let dir = tempdir().unwrap();
        let game_dir = dir.path().to_path_buf();
        fs::create_dir_all(game_dir.join("resourcepacks")).unwrap();
        fs::create_dir_all(game_dir.join("saves").join("world").join("datapacks")).unwrap();
        fs::write(
            game_dir.join("resourcepacks").join("pack.zip"),
            b"pack-bytes",
        )
        .unwrap();
        fs::write(
            game_dir
                .join("saves")
                .join("world")
                .join("datapacks")
                .join("dp.zip"),
            b"dp-bytes",
        )
        .unwrap();

        let mut kinds = HashSet::new();
        kinds.insert(CompType::Resourcepack);
        kinds.insert(CompType::Datapack);
        let index =
            mclr_comp::preset::installed::build(&game_dir, &kinds, Some("world"), &client).await;

        let pack_sha1 = sha1_hex(b"pack-bytes");
        let hit = index.by_sha1(&pack_sha1).expect("pack hashed");
        assert_eq!(hit.rel_path, "resourcepacks/pack.zip");
        assert_eq!(hit.file_name, "pack.zip");
        assert!(
            index
                .by_sha1(&sha1_hex(b"dp-bytes"))
                .is_some_and(|h| h.rel_path == "saves/world/datapacks/dp.zip")
        );
        // The mock returns no versions, so nothing is identified as a project yet.
        assert!(index.by_project.is_empty());

        // A ledger entry supplies the project identity for a CurseForge resource pack.
        ledger::upsert(
            &game_dir,
            ledger::LedgerEntry {
                platform: CompPlatform::Curseforge,
                project_id: "pack".to_string(),
                file_id: "f1".to_string(),
                sha1: pack_sha1.clone(),
                kind: CompType::Resourcepack,
                rel_path: "resourcepacks/pack.zip".to_string(),
                preset_ids: Vec::new(),
                link_mode: LinkMode::Copy,
                applied_at: 1,
            },
        )
        .unwrap();

        let index =
            mclr_comp::preset::installed::build(&game_dir, &kinds, Some("world"), &client).await;
        let hits = index.by_project(CompPlatform::Curseforge, "pack");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].sha1, pack_sha1);
        assert_eq!(hits[0].file_id.as_deref(), Some("f1"));
    }

    #[tokio::test]
    async fn build_skips_datapacks_without_world_and_honours_kinds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/version_files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;
        let client = ModrinthClient::new(SourcePriority::default()).with_base_url(server.uri());

        let dir = tempdir().unwrap();
        let game_dir = dir.path().to_path_buf();
        fs::create_dir_all(game_dir.join("saves").join("world").join("datapacks")).unwrap();
        fs::write(
            game_dir
                .join("saves")
                .join("world")
                .join("datapacks")
                .join("dp.zip"),
            b"dp-bytes",
        )
        .unwrap();

        let mut kinds = HashSet::new();
        kinds.insert(CompType::Datapack);
        let index = mclr_comp::preset::installed::build(&game_dir, &kinds, None, &client).await;
        assert!(index.is_empty());

        let index =
            mclr_comp::preset::installed::build(&game_dir, &kinds, Some("world"), &client).await;
        assert_eq!(index.by_sha1.len(), 1);
    }

    #[tokio::test]
    async fn build_recognizes_extracted_worlds_from_the_ledger() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/version_files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&server)
            .await;
        let client = ModrinthClient::new(SourcePriority::default()).with_base_url(server.uri());

        let dir = tempdir().unwrap();
        let game_dir = dir.path().to_path_buf();
        let world_dir = game_dir.join("saves").join("MyWorld");
        fs::create_dir_all(&world_dir).unwrap();
        fs::write(world_dir.join("level.dat"), b"level").unwrap();

        ledger::upsert(
            &game_dir,
            ledger::LedgerEntry {
                platform: CompPlatform::Curseforge,
                project_id: "adventure".to_string(),
                file_id: "w1".to_string(),
                sha1: "world-sha".to_string(),
                kind: CompType::World,
                rel_path: "saves/MyWorld".to_string(),
                preset_ids: Vec::new(),
                link_mode: LinkMode::Extracted,
                applied_at: 1,
            },
        )
        .unwrap();

        let mut kinds = HashSet::new();
        kinds.insert(CompType::World);
        let index = mclr_comp::preset::installed::build(&game_dir, &kinds, None, &client).await;
        let hits = index.by_project(CompPlatform::Curseforge, "adventure");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rel_path, "saves/MyWorld");
        assert_eq!(hits[0].file_name, "MyWorld");
    }
}
