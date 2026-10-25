//! Single source of truth for an instance's game directory (isolation resolution).
//!
//! Rule (PCL parity): `isolation = "on"` → `<instance dir>`, `"off"` → `<mc folder>`,
//! `"global"` → on when `<instance>/mods` has files or `<instance>/saves` has sub-folders,
//! otherwise decided by the global `game.defaultIsolation`. The first computed `"global"`
//! decision is persisted once as `"on"` / `"off"` in `mclr.instance.json`.

use crate::instance::{InstanceConfig, classify_instance, extract_mc_version};
use crate::version::{VersionJson, merge_version_json};
use mclr_core::settings::{DefaultIsolation, GameSettings};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "mclr.instance.json";

/// Loader name prefixes (as produced by `classify_instance`) that make an instance moddable.
const MODABLE_LOADERS: &[&str] = &[
    "Fabric",
    "LegacyFabric",
    "Quilt",
    "Forge",
    "NeoForge",
    "Cleanroom",
    "LiteLoader",
];

/// `true` when any detected loader accepts mods.
pub fn is_modable_loaders(loaders: &[String]) -> bool {
    loaders.iter().any(|l| {
        let name = l.split_whitespace().next().unwrap_or("");
        MODABLE_LOADERS.contains(&name)
    })
}

/// `true` for plain release ids (`1.20.1`, `1.7`, `26.1`), `false` for snapshots/pre-releases/old versions.
pub fn is_release_version(mc_version: &str) -> bool {
    let mut parts = mc_version.split('.');
    let (Some(a), Some(b)) = (parts.next(), parts.next()) else {
        return false;
    };
    let rest = parts.next();
    parts.next().is_none()
        && !a.is_empty()
        && a.bytes().all(|c| c.is_ascii_digit())
        && !b.is_empty()
        && b.bytes().all(|c| c.is_ascii_digit())
        && rest.is_none_or(|c| !c.is_empty() && c.bytes().all(|d| d.is_ascii_digit()))
}

/// Pure decision from the isolation setting of an instance, never touching the disk.
pub fn decide_default(default: DefaultIsolation, modable: bool, is_release: bool) -> bool {
    match default {
        DefaultIsolation::Off => false,
        DefaultIsolation::Modded => modable,
        DefaultIsolation::NonRelease => !is_release,
        DefaultIsolation::ModdedOrNonRelease => modable || !is_release,
        DefaultIsolation::All => true,
    }
}

fn dir_has_files(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .any(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        })
        .unwrap_or(false)
}

fn dir_has_subdirs(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .any(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        })
        .unwrap_or(false)
}

fn read_config(instance_dir: &Path) -> InstanceConfig {
    fs::read_to_string(instance_dir.join(CONFIG_FILE))
        .ok()
        .and_then(|s| serde_json::from_str::<InstanceConfig>(&s).ok())
        .unwrap_or_default()
}

/// Persist only the `isolation` field, keeping every other field of the config file intact.
fn persist_isolation(instance_dir: &Path, isolated: bool) {
    let path = instance_dir.join(CONFIG_FILE);
    let mut value = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .filter(|v| v.is_object())
        .or_else(|| serde_json::to_value(InstanceConfig::default()).ok());
    let Some(obj) = value.as_mut().and_then(|v| v.as_object_mut()) else {
        return;
    };
    obj.insert(
        "isolation".to_string(),
        serde_json::Value::String(if isolated { "on" } else { "off" }.to_string()),
    );
    if let Ok(text) = serde_json::to_string_pretty(&value) {
        if let Err(e) = fs::write(&path, text) {
            tracing::warn!(path = %path.display(), error = %e, "persisting isolation failed");
        }
    }
}

/// Loaders + MC version of one instance (merging `inheritsFrom` when the parent exists).
fn instance_profile(mc_folder: &Path, id: &str) -> (bool, bool) {
    let versions = mc_folder.join("versions");
    let Ok(text) = fs::read_to_string(versions.join(id).join(format!("{id}.json"))) else {
        return (false, false);
    };
    let Ok(json) = VersionJson::parse(&text) else {
        return (false, false);
    };
    let effective = match json.inherits_from.as_deref() {
        Some(parent) if !parent.is_empty() => {
            let parent_json =
                fs::read_to_string(versions.join(parent).join(format!("{parent}.json")))
                    .ok()
                    .and_then(|s| VersionJson::parse(&s).ok());
            match parent_json {
                Some(p) => merge_version_json(json.clone(), p),
                None => json,
            }
        }
        _ => json,
    };
    let mc_version = extract_mc_version(&effective, id);
    let (_, loaders) = classify_instance(&effective, &mc_version);
    (
        is_modable_loaders(&loaders),
        is_release_version(&mc_version),
    )
}

/// Resolve the game directory of instance `id`, persisting the one-time `"global"` decision.
pub fn resolve_game_dir(mc_folder: &Path, id: &str, defaults: &GameSettings) -> PathBuf {
    let instance_dir = mc_folder.join("versions").join(id);
    let cfg = read_config(&instance_dir);
    let isolated = match cfg.isolation.as_str() {
        "on" => true,
        "off" => false,
        _ => {
            let decided = if dir_has_files(&instance_dir.join("mods"))
                || dir_has_subdirs(&instance_dir.join("saves"))
            {
                true
            } else {
                let (modable, release) = instance_profile(mc_folder, id);
                decide_default(defaults.default_isolation, modable, release)
            };
            if instance_dir.is_dir() {
                persist_isolation(&instance_dir, decided);
            }
            decided
        }
    };
    if isolated {
        instance_dir
    } else {
        mc_folder.to_path_buf()
    }
}

/// Sub-paths of a game directory addressable from the UI. `None` for anything else, which
/// also rejects path traversal.
pub fn game_subdir(game_dir: &Path, sub: &str) -> Option<PathBuf> {
    match sub {
        "" => Some(game_dir.to_path_buf()),
        "mods" | "resourcepacks" | "shaderpacks" | "schematics" | "saves" | "screenshots"
        | "servers.dat" | "backups" | "datapacks" => Some(game_dir.join(sub)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults(d: DefaultIsolation) -> GameSettings {
        GameSettings {
            default_isolation: d,
            ..GameSettings::default()
        }
    }

    fn write_instance(mc: &Path, id: &str, json: &str, isolation: Option<&str>) -> PathBuf {
        let dir = mc.join("versions").join(id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{id}.json")), json).unwrap();
        if let Some(iso) = isolation {
            let mut cfg = InstanceConfig::default();
            cfg.isolation = iso.to_string();
            fs::write(dir.join(CONFIG_FILE), serde_json::to_string(&cfg).unwrap()).unwrap();
        }
        dir
    }

    const VANILLA: &str = r#"{"id":"1.20.1","type":"release","libraries":[]}"#;
    const SNAPSHOT: &str = r#"{"id":"24w14a","type":"snapshot","libraries":[]}"#;
    const FABRIC: &str = r#"{"id":"f","inheritsFrom":"1.20.1","libraries":[{"name":"net.fabricmc:fabric-loader:0.15.11"}]}"#;

    #[test]
    fn release_detection() {
        assert!(is_release_version("1.20.1"));
        assert!(is_release_version("1.7"));
        assert!(is_release_version("26.1.2"));
        assert!(!is_release_version("24w14a"));
        assert!(!is_release_version("1.20-pre1"));
        assert!(!is_release_version("b1.7.3"));
    }

    #[test]
    fn explicit_on_off_ignore_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let mc = tmp.path();
        let on = write_instance(mc, "a", VANILLA, Some("on"));
        write_instance(mc, "b", VANILLA, Some("off"));
        assert_eq!(
            resolve_game_dir(mc, "a", &defaults(DefaultIsolation::Off)),
            on
        );
        assert_eq!(
            resolve_game_dir(mc, "b", &defaults(DefaultIsolation::All)),
            mc
        );
    }

    #[test]
    fn default_modes() {
        let cases = [
            (DefaultIsolation::Off, VANILLA, false),
            (DefaultIsolation::Off, FABRIC, false),
            (DefaultIsolation::Modded, VANILLA, false),
            (DefaultIsolation::Modded, FABRIC, true),
            (DefaultIsolation::NonRelease, VANILLA, false),
            (DefaultIsolation::NonRelease, SNAPSHOT, true),
            (DefaultIsolation::NonRelease, FABRIC, false),
            (DefaultIsolation::ModdedOrNonRelease, VANILLA, false),
            (DefaultIsolation::ModdedOrNonRelease, SNAPSHOT, true),
            (DefaultIsolation::ModdedOrNonRelease, FABRIC, true),
            (DefaultIsolation::All, VANILLA, true),
        ];
        for (mode, json, expect_isolated) in cases {
            let tmp = tempfile::tempdir().unwrap();
            let mc = tmp.path();
            let dir = write_instance(mc, "x", json, None);
            let got = resolve_game_dir(mc, "x", &defaults(mode));
            let want = if expect_isolated {
                dir
            } else {
                mc.to_path_buf()
            };
            assert_eq!(got, want, "{mode:?} / {json}");
        }
    }

    #[test]
    fn non_empty_mods_forces_isolation_and_decision_is_persisted() {
        let tmp = tempfile::tempdir().unwrap();
        let mc = tmp.path();
        let dir = write_instance(mc, "x", VANILLA, Some("global"));
        assert_eq!(
            resolve_game_dir(mc, "x", &defaults(DefaultIsolation::Off)),
            mc
        );
        // Persisted as "off": adding mods afterwards does not flip an already decided instance.
        assert_eq!(read_config(&dir).isolation, "off");
        fs::create_dir_all(dir.join("mods")).unwrap();
        fs::write(dir.join("mods/a.jar"), b"x").unwrap();
        assert_eq!(
            resolve_game_dir(mc, "x", &defaults(DefaultIsolation::All)),
            mc
        );

        let dir2 = write_instance(mc, "y", VANILLA, Some("global"));
        fs::create_dir_all(dir2.join("mods")).unwrap();
        fs::write(dir2.join("mods/a.jar"), b"x").unwrap();
        assert_eq!(
            resolve_game_dir(mc, "y", &defaults(DefaultIsolation::Off)),
            dir2
        );
        assert_eq!(read_config(&dir2).isolation, "on");
    }

    #[test]
    fn saves_subfolder_forces_isolation() {
        let tmp = tempfile::tempdir().unwrap();
        let mc = tmp.path();
        let dir = write_instance(mc, "x", VANILLA, None);
        fs::create_dir_all(dir.join("saves/World1")).unwrap();
        assert_eq!(
            resolve_game_dir(mc, "x", &defaults(DefaultIsolation::Off)),
            dir
        );
    }

    #[test]
    fn subdir_whitelist() {
        let g = Path::new("g");
        assert_eq!(game_subdir(g, "mods"), Some(g.join("mods")));
        assert_eq!(game_subdir(g, ""), Some(g.to_path_buf()));
        assert_eq!(game_subdir(g, "../x"), None);
    }
}
