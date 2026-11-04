//! Preset planning, repository, ledger and resolver-filter tests.

use std::collections::{HashMap, HashSet};
use std::fs;

use mclr_comp::model::{CompFile, CompFileHashes, CompPlatform, CompReleaseChannel, CompType};
use mclr_comp::preset::installed::{InstalledHit, InstalledIndex};
use mclr_comp::preset::ledger;
use mclr_comp::preset::model::{
    AddStatus, ApplyStatus, GateReason, LinkMode, OtherVersionPolicy, PresetItem, ResourcePreset,
};
use mclr_comp::preset::plan::{InstanceCtx, decide, merge_presets};
use mclr_comp::preset::repo::PresetRepo;
use mclr_comp::resolver::{
    MatchFailure, expand_loaders, file_supports_mc, select_best_file_filtered,
};
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

fn merged_of(
    presets: &[ResourcePreset],
    context: &InstanceCtx,
    project_id: &str,
) -> mclr_comp::preset::plan::MergedItem {
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
        expand_loaders(&["neoforge".into()], "1.20.1"),
        vec!["neoforge".to_string(), "forge".to_string()]
    );
    assert_eq!(
        expand_loaders(&["neoforge".into()], "1.20.4"),
        vec!["neoforge".to_string()]
    );
    assert_eq!(
        expand_loaders(&["quilt".into()], "1.20.1"),
        vec!["quilt".to_string(), "fabric".to_string()]
    );
    assert_eq!(
        expand_loaders(&["cleanroom".into()], "1.12.2"),
        vec!["forge".to_string()]
    );
    assert_eq!(
        expand_loaders(&["legacyfabric".into()], "1.8.9"),
        vec!["fabric".to_string()]
    );
    assert_eq!(
        expand_loaders(&["Fabric".into(), "fabric".into()], "1.20.1"),
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
    let context = ctx("1.20.1", &["fabric"]);
    let merged = merged_of(&[a, b], &context, "sodium");
    assert!(!merged.version_conflict);
    assert_eq!(merged.item.pinned_file_id.as_deref(), Some("f2"));
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
fn decide_preset_loader_filter_uses_intersection_with_instance() {
    let mod_item = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    let shader_item = item(CompPlatform::Modrinth, "bsl", CompType::Shader, None);
    let p = preset("a", "A", &[], &["forge"], vec![mod_item, shader_item]);
    let context = ctx("1.20.1", &["fabric"]);

    let (gates, items) = merge_presets(&[p], &context);
    assert!(gates[0].ok);
    assert_eq!(items.len(), 2);

    let files = vec![file(
        "f1",
        "sodium",
        &["1.20.1"],
        &["fabric"],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("aa"),
    )];
    let mod_decision = decide(
        &items[0],
        &files,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(mod_decision.status, ApplyStatus::NoLoader);

    // Shader items are not gated by preset loaders at all.
    let shader_files = vec![file(
        "f2",
        "bsl",
        &["1.20.1"],
        &[],
        CompReleaseChannel::Release,
        "2024-01-01T00:00:00Z",
        Some("bb"),
    )];
    let shader_decision = decide(
        &items[1],
        &shader_files,
        &context,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(shader_decision.status, ApplyStatus::Installed);
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
fn decide_pin_rules_and_unmatched_falls_back_to_latest() {
    let mut i = item(CompPlatform::Modrinth, "sodium", CompType::Mod, None);
    i.pin_rules.push(mclr_comp::preset::model::PresetPinRule {
        game_versions: vec!["1.19.2".to_string()],
        loaders: vec!["fabric".to_string()],
        file_id: "old".to_string(),
        file_name: "sodium-old.jar".to_string(),
    });

    let p = preset("a", "A", &[], &[], vec![i]);

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

    // Case 1: 1.19.2 Fabric matches the rule -> installs pinned "old" file
    let ctx_119 = ctx("1.19.2", &["fabric"]);
    let merged_119 = merged_of(std::slice::from_ref(&p), &ctx_119, "sodium");
    let decision_119 = decide(
        &merged_119,
        &files,
        &ctx_119,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision_119.status, ApplyStatus::Installed);
    assert_eq!(decision_119.file.map(|f| f.id), Some("old".to_string()));

    // Case 2: 1.20.1 Fabric does not match any rule -> automatically falls back to latest version "new"
    let ctx_120 = ctx("1.20.1", &["fabric"]);
    let merged_120 = merged_of(&[p], &ctx_120, "sodium");
    let decision_120 = decide(
        &merged_120,
        &files,
        &ctx_120,
        &empty_installed(),
        OtherVersionPolicy::Skip,
        false,
    );
    assert_eq!(decision_120.status, ApplyStatus::Installed);
    assert_eq!(decision_120.file.map(|f| f.id), Some("new".to_string()));
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
