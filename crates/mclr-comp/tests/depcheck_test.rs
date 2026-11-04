//! Dependency analysis and version-range tests.
//!
//! Fixture jars are built with `zip::ZipWriter` straight from metadata text, then parsed with
//! the real `parse_mod_metadata`, so the tests cover the parser and the checker together.

use std::collections::HashSet;
use std::io::{Cursor, Write};

use mclr_comp::depcheck::{DepStatus, ModDepKind, ModDependencyReport, analyze};
use mclr_comp::local::{LocalModInfo, parse_mod_metadata};
use mclr_comp::versionrange::satisfies;
use zip::write::SimpleFileOptions;

fn entry_options() -> SimpleFileOptions {
    SimpleFileOptions::default().last_modified_time(zip::DateTime::default())
}

/// Builds a jar in memory from `(entry name, text)` pairs.
fn jar_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        for (name, text) in entries {
            zip.start_file(*name, entry_options()).unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }
    cursor.into_inner()
}

fn fabric_jar(json: &str) -> Vec<u8> {
    jar_bytes(&[("fabric.mod.json", json)])
}

fn mod_info(file_name: &str, enabled: bool, bytes: &[u8]) -> LocalModInfo {
    let stem = file_name
        .trim_end_matches(".disabled")
        .trim_end_matches(".jar");
    let meta = parse_mod_metadata(bytes, stem);
    LocalModInfo {
        file_name: file_name.to_string(),
        path: format!("/mods/{file_name}"),
        enabled,
        mod_id: meta.mod_id,
        name: meta.name,
        version: meta.version,
        description: meta.description,
        authors: meta.authors,
        loader: meta.loader,
        sha1: format!("sha1-{file_name}"),
        murmur2: 0,
        size: bytes.len() as u64,
        project_id: None,
        update_file: None,
        icon: None,
        dependencies: meta.dependencies,
        provides: meta.provides,
    }
}

/// A mod with `id`/`version` plus a Fabric dependency map body (`"depends": {...}` etc.).
fn fabric_mod(
    file_name: &str,
    enabled: bool,
    id: &str,
    version: &str,
    extra: &str,
) -> LocalModInfo {
    let json = format!(r#"{{"id":"{id}","version":"{version}","name":"{id}"{extra}}}"#);
    mod_info(file_name, enabled, &fabric_jar(&json))
}

fn report<'a>(reports: &'a [ModDependencyReport], file_name: &str) -> &'a ModDependencyReport {
    reports
        .iter()
        .find(|r| r.file_name == file_name)
        .unwrap_or_else(|| panic!("no report for {file_name}"))
}

#[test]
fn versionrange_maven_intervals() {
    assert!(satisfies("1.5", "[1.0,2.0)"));
    assert!(!satisfies("2.0", "[1.0,2.0)"));
    assert!(satisfies("2.0", "[1.0,2.0]"));
    assert!(!satisfies("0.9", "[1.0,2.0]"));
    assert!(satisfies("1.5", "(,1.5]"));
    assert!(!satisfies("1.6", "(,1.5]"));
    assert!(satisfies("1.2", "[1.2]"));
    assert!(!satisfies("1.2.1", "[1.2]"));
    assert!(satisfies("21", "[17,)"));
    assert!(!satisfies("16", "[17,)"));

    // Unions of intervals are one range string.
    assert!(satisfies("1.1", "[1.0,1.2],[1.4,1.6]"));
    assert!(!satisfies("1.3", "[1.0,1.2],[1.4,1.6]"));
    assert!(satisfies("1.5", "[1.0,1.2],[1.4,1.6]"));
}

#[test]
fn versionrange_fabric_predicates() {
    assert!(satisfies("1.20.1", "*"));
    assert!(satisfies("1.2.3", ">=1.2.3"));
    assert!(!satisfies("1.2.2", ">=1.2.3"));
    assert!(satisfies("1.9", "<2"));
    assert!(!satisfies("2.0", "<2"));
    assert!(satisfies("1.2.9", "~1.2"));
    assert!(!satisfies("1.3.0", "~1.2"));
    assert!(satisfies("1.9.0", "^1.2"));
    assert!(!satisfies("2.0.0", "^1.2"));
    assert!(satisfies("0.2.9", "^0.2.3"));
    assert!(!satisfies("0.3.0", "^0.2.3"));
    assert!(satisfies("0.0.3", "^0.0.3"));
    assert!(!satisfies("0.0.4", "^0.0.3"));
    assert!(satisfies("1.20.1", "1.20.x"));
    assert!(!satisfies("1.21", "1.20.x"));
    assert!(satisfies("1.20.5", ">=1.20 <1.21"));
    assert!(!satisfies("1.21", ">=1.20 <1.21"));
    assert!(satisfies("0.15.11", ">=0.14"));
    assert!(!satisfies("0.13", ">=0.14"));

    // `||` alternatives accept any branch.
    assert!(satisfies("3.0", "1.0 || 2.0 || 3.0"));
    assert!(!satisfies("4.0", "1.0 || 2.0 || 3.0"));

    // A pre-release sorts below the matching release.
    assert!(!satisfies("1.0-beta", ">=1.0"));
    assert!(satisfies("1.0-beta", ">=1.0-beta"));
}

#[test]
fn versionrange_never_claims_an_unprovable_mismatch() {
    // Placeholders and garbage are not versions; they must never produce a mismatch.
    assert!(satisfies("1.0", "${file.jarVersion}"));
    assert!(satisfies("${file.jarVersion}", "[1.0,2.0)"));
    assert!(satisfies("1.0", ""));
    assert!(satisfies("", ">=1.0"));
    assert!(satisfies("1.0", ">="));
    assert!(satisfies("1.0", "~"));
    assert!(
        satisfies("1.0", "???"),
        "a non-version token is not a mismatch"
    );
    assert!(satisfies("unspecified", ">=1.0"));
    // A comparable pair still reports a real mismatch.
    assert!(!satisfies("1.0", ">=2.0"));
}

#[test]
fn depcheck_reports_every_status() {
    let mods = vec![
        // The subject mod: one dep of each interesting shape.
        fabric_mod(
            "subject.jar",
            true,
            "subject",
            "1.0.0",
            r#","depends":{"present":"*","old_lib":">=2.0","off_lib":"*","gone_lib":"*","etapack":"*"},"breaks":{"enemy":"*","absent":"*"}"#,
        ),
        fabric_mod("present.jar", true, "present", "1.0.0", ""),
        fabric_mod("old_lib.jar", true, "old_lib", "1.5.0", ""),
        fabric_mod("off_lib.jar.disabled", false, "off_lib", "1.0.0", ""),
        fabric_mod("enemy.jar", true, "enemy", "1.0.0", ""),
        fabric_mod(
            "eta.jar",
            true,
            "eta",
            "1.0.0",
            r#","provides":["etapack"]"#,
        ),
    ];

    let reports = analyze(&mods, &HashSet::new());
    let subject = report(&reports, "subject.jar");

    let dep = |id: &str| {
        subject
            .deps
            .iter()
            .find(|d| d.id == id)
            .unwrap_or_else(|| panic!("missing dep row {id}"))
    };

    assert_eq!(dep("present").status, DepStatus::Installed);
    assert_eq!(dep("present").provider_file.as_deref(), Some("present.jar"));

    assert_eq!(dep("old_lib").status, DepStatus::VersionMismatch);
    assert_eq!(dep("old_lib").provider_version.as_deref(), Some("1.5.0"));

    assert_eq!(dep("off_lib").status, DepStatus::Disabled);
    assert_eq!(
        dep("off_lib").provider_file.as_deref(),
        Some("off_lib.jar.disabled")
    );

    assert_eq!(dep("gone_lib").status, DepStatus::Missing);
    assert!(dep("gone_lib").provider_file.is_none());

    assert_eq!(dep("etapack").status, DepStatus::Installed);
    assert_eq!(dep("etapack").provider_file.as_deref(), Some("eta.jar"));

    // Incompatible dep with an installed provider is a conflict; without one it is omitted.
    assert_eq!(dep("enemy").kind, ModDepKind::Incompatible);
    assert_eq!(dep("enemy").status, DepStatus::Conflict);
    assert!(subject.deps.iter().all(|d| d.id != "absent"));

    // Platform ids never show up.
    assert!(subject.deps.iter().all(|d| {
        !matches!(
            d.id.as_str(),
            "minecraft" | "java" | "forge" | "neoforge" | "fabricloader"
        )
    }));
}

#[test]
fn depcheck_platform_deps_and_dependents() {
    let mods = vec![
        fabric_mod(
            "alpha.jar",
            true,
            "alpha",
            "1.0.0",
            r#","depends":{"minecraft":">=1.20","fabricloader":">=0.14","beta":">=1.0"}"#,
        ),
        fabric_mod("beta.jar", true, "beta", "1.2.0", ""),
        fabric_mod("beta_old.jar.disabled", false, "beta", "0.9.0", ""),
    ];

    let reports = analyze(&mods, &HashSet::new());
    let alpha = report(&reports, "alpha.jar");

    assert_eq!(alpha.deps.len(), 1, "only the real dependency is listed");
    assert_eq!(alpha.deps[0].id, "beta");
    assert_eq!(alpha.deps[0].status, DepStatus::Installed);
    assert_eq!(alpha.deps[0].provider_file.as_deref(), Some("beta.jar"));

    assert_eq!(report(&reports, "beta.jar").dependents, vec!["alpha.jar"]);
    assert!(report(&reports, "alpha.jar").dependents.is_empty());
    // The disabled older copy is not the provider and has no dependents.
    assert!(
        report(&reports, "beta_old.jar.disabled")
            .dependents
            .is_empty()
    );
}

#[test]
fn depcheck_bundled_and_optional() {
    let mods = vec![
        fabric_mod(
            "host.jar",
            true,
            "host",
            "1.0.0",
            r#","depends":{"libx":"*"},"recommends":{"nice_to_have":">=1.0"},"suggests":{"maybe":">=1.0"}"#,
        ),
        // Same declaration through the array form of a Fabric constraint.
        fabric_mod(
            "multi.jar",
            true,
            "multi",
            "1.0.0",
            r#","depends":{"liby":["[1.0,2.0)","[3.0,4.0)"]}"#,
        ),
        fabric_mod("liby.jar", true, "liby", "3.5.0", ""),
    ];

    let mut bundled = HashSet::new();
    bundled.insert("libx".to_string());
    let reports = analyze(&mods, &bundled);

    let host = report(&reports, "host.jar");
    let dep = |id: &str| host.deps.iter().find(|d| d.id == id).unwrap();

    assert_eq!(dep("libx").status, DepStatus::Bundled);
    assert_eq!(dep("libx").kind, ModDepKind::Required);
    assert_eq!(dep("nice_to_have").status, DepStatus::Missing);
    assert_eq!(dep("nice_to_have").kind, ModDepKind::Optional);
    assert_eq!(dep("maybe").kind, ModDepKind::Optional);

    // `["[1.0,2.0)","[3.0,4.0)"]` becomes one `||` range satisfied by 3.5.0.
    let multi = report(&reports, "multi.jar");
    assert_eq!(multi.deps.len(), 1);
    assert_eq!(
        multi.deps[0].range.as_deref(),
        Some("[1.0,2.0) || [3.0,4.0)")
    );
    assert_eq!(multi.deps[0].status, DepStatus::Installed);
}

#[test]
fn depcheck_parses_forge_and_neoforge_tables() {
    let toml = r#"
modLoader = "javafml"
loaderVersion = "[47,)"
license = "MIT"

[[mods]]
modId = "parent"
version = "1.0.0"
displayName = "Parent"

[[dependencies.parent]]
modId = "neoforge"
mandatory = true
versionRange = "[47,)"

[[dependencies.parent]]
modId = "child"
mandatory = true
versionRange = "[2.0,3.0)"

[[dependencies.parent]]
modId = "rival"
type = "optional"
versionRange = "[1.0,)"

[[dependencies.parent]]
modId = "enemy"
type = "incompatible"
versionRange = "*"

[[dependencies.parent]]
modId = "soft"
mandatory = false
versionRange = "[1.0,)"
"#;
    let parent = mod_info(
        "parent.jar",
        true,
        &jar_bytes(&[("META-INF/neoforge.mods.toml", toml)]),
    );
    assert_eq!(parent.loader, "NeoForge");

    let child = fabric_jar(r#"{"id":"child","version":"2.5.0","name":"Child"}"#);
    let enemy = fabric_jar(r#"{"id":"enemy","version":"1.0.0","name":"Enemy"}"#);
    let mods = vec![
        parent,
        mod_info("child.jar", true, &child),
        mod_info("enemy.jar", true, &enemy),
    ];

    let reports = analyze(&mods, &HashSet::new());
    let parent = report(&reports, "parent.jar");
    let dep = |id: &str| parent.deps.iter().find(|d| d.id == id).unwrap();

    assert_eq!(dep("child").status, DepStatus::Installed);
    assert_eq!(dep("child").kind, ModDepKind::Required);
    assert_eq!(dep("rival").kind, ModDepKind::Optional);
    assert_eq!(dep("rival").status, DepStatus::Missing);
    assert_eq!(dep("soft").kind, ModDepKind::Optional);
    assert_eq!(dep("enemy").kind, ModDepKind::Incompatible);
    assert_eq!(dep("enemy").status, DepStatus::Conflict);
    assert!(parent.deps.iter().all(|d| d.id != "neoforge"));
}

#[test]
fn depcheck_parses_quilt_and_legacy_forge_metadata() {
    let quilt = r#"{
        "schema_version": 1,
        "quilt_loader": {
            "group": "org.example",
            "id": "qmod",
            "version": "1.0.0",
            "metadata": {"name": "QMod"},
            "depends": ["qlib >=1.0", {"id": "qother", "versions": ">=2.0", "optional": true}],
            "provides": [{"id": "qpack"}]
        }
    }"#;
    let qmod = mod_info("qmod.jar", true, &jar_bytes(&[("quilt.mod.json", quilt)]));
    assert_eq!(qmod.loader, "Quilt");
    assert_eq!(qmod.provides, vec!["qpack"]);

    let legacy =
        r#"[{"modid":"legacy","name":"Legacy","version":"1.0","requiredMods":["legacylib"]}]"#;
    let legacy_mod = mod_info("legacy.jar", true, &jar_bytes(&[("mcmod.info", legacy)]));

    let mods = vec![qmod, legacy_mod];
    let reports = analyze(&mods, &HashSet::new());

    let qmod = report(&reports, "qmod.jar");
    assert_eq!(qmod.deps.len(), 2);
    assert_eq!(qmod.deps[0].id, "qlib");
    assert_eq!(qmod.deps[0].range.as_deref(), Some(">=1.0"));
    assert_eq!(qmod.deps[0].status, DepStatus::Missing);
    assert_eq!(qmod.deps[1].kind, ModDepKind::Optional);
    assert_eq!(qmod.deps[1].range.as_deref(), Some(">=2.0"));

    let legacy = report(&reports, "legacy.jar");
    assert_eq!(legacy.deps.len(), 1);
    assert_eq!(legacy.deps[0].id, "legacylib");
    assert_eq!(legacy.deps[0].status, DepStatus::Missing);
    assert!(legacy.deps[0].range.is_none());
}
