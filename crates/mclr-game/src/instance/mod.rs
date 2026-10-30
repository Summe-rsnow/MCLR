//! Minecraft instance model, classification, scan, and file operations.

pub mod classifier;
pub mod ops;
pub mod scanner;

use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeMap;

pub use classifier::{
    card_display_order, classify_cards, classify_instance, extract_loader_versions,
    extract_mc_version, is_april_fools, is_modable_instance,
};
pub use ops::{
    delete_instance, delete_instance_ext, duplicate_instance, rename_instance,
    save_instance_config, touch_instance_last_played, validate_instance_name,
};
pub use scanner::{ensure_game_folder, scan_instances};

/// Classification of a Minecraft instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum InstanceKind {
    Vanilla,
    Snapshot,
    OldBeta,
    OldAlpha,
    Fabric,
    LegacyFabric,
    Forge,
    NeoForge,
    Cleanroom,
    OptiFine,
    LiteLoader,
    LabyMod,
    Quilt,
    ModdedMultiple,
}

/// State of an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum InstanceState {
    Ok,
    Broken,
    Incomplete,
}

/// Card category for grouping Minecraft instances (PCL parity).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type, Default,
)]
#[serde(rename_all = "camelCase")]
pub enum CardKind {
    Star,
    Api,
    #[default]
    Regular,
    Rubbish,
    Fool,
    Error,
    Hidden,
}

/// Per-instance configuration stored in `<instance_dir>/mclr.instance.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub icon: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub starred: bool,
    #[serde(default)]
    pub card_type: Option<CardKind>,
    pub isolation: String, // "global" | "on" | "off"
    pub java: Option<String>,
    pub memory: Option<u32>,
    pub jvm_args: Option<String>,
    pub game_args: Option<String>,
    pub window_title: Option<String>,
    pub pre_launch_command: Option<String>,
    pub login_requirement: String, // "any" | "microsoft" | "thirdParty" | "microsoftOrThirdParty"
    pub server_quick_play: Option<String>,
    /// RFC 3339 timestamp of the last successful launch, set by the launcher.
    #[serde(default)]
    pub last_played_at: Option<String>,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            icon: "grass".to_string(),
            category: "Default".to_string(),
            pinned: false,
            starred: false,
            card_type: None,
            isolation: "global".to_string(),
            java: None,
            memory: None,
            jvm_args: None,
            game_args: None,
            window_title: None,
            pre_launch_command: None,
            login_requirement: "any".to_string(),
            server_quick_play: None,
            last_played_at: None,
        }
    }
}

/// Information describing a Minecraft instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub path: String,
    pub mc_version: String,
    pub kind: InstanceKind,
    pub loaders: Vec<String>,
    pub state: InstanceState,
    pub config: InstanceConfig,
    #[serde(default)]
    pub modable: bool,
    #[serde(default)]
    pub card: CardKind,
    #[serde(default)]
    pub release_time: Option<String>,
    #[serde(default)]
    pub loader_versions: BTreeMap<String, String>,
}

impl Default for InstanceInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            folder: String::new(),
            path: String::new(),
            mc_version: "Unknown".to_string(),
            kind: InstanceKind::Vanilla,
            loaders: Vec::new(),
            state: InstanceState::Ok,
            config: InstanceConfig::default(),
            modable: false,
            card: CardKind::Regular,
            release_time: None,
            loader_versions: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::VersionJson;
    use std::fs;

    #[test]
    fn test_validate_instance_name() {
        assert!(validate_instance_name("1.20.1-Fabric").is_ok());
        assert!(validate_instance_name("My Instance (1)").is_ok());
        assert!(validate_instance_name("").is_err());
        assert!(validate_instance_name(" ").is_err());
        assert!(validate_instance_name(" leading").is_err());
        assert!(validate_instance_name("trailing ").is_err());
        assert!(validate_instance_name("dot.").is_err());
        assert!(validate_instance_name("abc/def").is_err());
        assert!(validate_instance_name("abc:def").is_err());
        assert!(validate_instance_name("abc!def").is_err());
        assert!(validate_instance_name("abc;def").is_err());
        assert!(validate_instance_name("CON").is_err());
        assert!(validate_instance_name("aux.txt").is_err());
        assert!(validate_instance_name(&"a".repeat(71)).is_err());
    }

    #[test]
    fn test_classify_instance() {
        let json_str = r#"{
            "id": "1.20.1-Fabric",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                { "name": "net.fabricmc:fabric-loader:0.15.11" }
            ]
        }"#;
        let v = VersionJson::parse(json_str).unwrap();
        let (kind, loaders) = classify_instance(&v, "1.20.1");
        assert_eq!(kind, InstanceKind::Fabric);
        assert_eq!(loaders, vec!["Fabric 0.15.11"]);

        let multi_str = r#"{
            "id": "1.20.1-Multi",
            "libraries": [
                { "name": "net.fabricmc:fabric-loader:0.15.11" },
                { "name": "net.minecraftforge:forge:1.20.1-47.2.0" }
            ]
        }"#;
        let multi_v = VersionJson::parse(multi_str).unwrap();
        let (multi_kind, multi_loaders) = classify_instance(&multi_v, "1.20.1");
        assert_eq!(multi_kind, InstanceKind::ModdedMultiple);
        assert_eq!(multi_loaders.len(), 2);
    }

    #[test]
    fn test_scan_and_instance_crud() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mc_dir = temp_dir.path();
        let versions_dir = mc_dir.join("versions");

        // 1. Setup Vanilla 1.20.1
        let v_dir = versions_dir.join("1.20.1");
        fs::create_dir_all(&v_dir).unwrap();
        let v_json = r#"{
            "id": "1.20.1",
            "type": "release",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": []
        }"#;
        fs::write(v_dir.join("1.20.1.json"), v_json).unwrap();

        // 2. Setup Fabric 1.20.1 (inheritsFrom 1.20.1)
        let f_dir = versions_dir.join("1.20.1-fabric");
        fs::create_dir_all(&f_dir).unwrap();
        let f_json = r#"{
            "id": "1.20.1-fabric",
            "inheritsFrom": "1.20.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [
                { "name": "net.fabricmc:fabric-loader:0.15.11" }
            ]
        }"#;
        fs::write(f_dir.join("1.20.1-fabric.json"), f_json).unwrap();

        // 3. Setup Broken instance
        let b_dir = versions_dir.join("broken");
        fs::create_dir_all(&b_dir).unwrap();
        fs::write(b_dir.join("broken.json"), "{ invalid-json").unwrap();

        // 4. Setup empty dir and installing dir (should be skipped)
        let empty_dir = versions_dir.join("empty-dir");
        fs::create_dir_all(&empty_dir).unwrap();
        let inst_dir = versions_dir.join("installing-dir");
        fs::create_dir_all(&inst_dir).unwrap();
        fs::write(inst_dir.join(".mclr-installing"), "").unwrap();
        fs::write(inst_dir.join("dummy.json"), "{}").unwrap();

        // Scan instances
        let scanned = scan_instances(mc_dir).unwrap();
        assert_eq!(scanned.len(), 3);

        let v_info = scanned.iter().find(|i| i.id == "1.20.1").unwrap();
        assert_eq!(v_info.kind, InstanceKind::Vanilla);
        assert_eq!(v_info.state, InstanceState::Ok);
        assert_eq!(v_info.card, CardKind::Regular);
        assert!(!v_info.modable);

        let f_info = scanned.iter().find(|i| i.id == "1.20.1-fabric").unwrap();
        assert_eq!(f_info.kind, InstanceKind::Fabric);
        assert_eq!(f_info.state, InstanceState::Ok);
        assert_eq!(f_info.mc_version, "1.20.1");
        assert_eq!(f_info.card, CardKind::Api);
        assert!(f_info.modable);

        let b_info = scanned.iter().find(|i| i.id == "broken").unwrap();
        assert_eq!(b_info.state, InstanceState::Broken);
        assert_eq!(b_info.card, CardKind::Error);

        // Test save config & star
        let mut cfg = InstanceConfig::default();
        cfg.starred = true;
        cfg.window_title = Some("Custom Window".to_string());
        let updated = save_instance_config(mc_dir, "1.20.1", &cfg).unwrap();
        assert!(updated.config.starred);
        assert!(updated.config.pinned);
        assert_eq!(updated.card, CardKind::Star);
        assert_eq!(
            updated.config.window_title.as_deref(),
            Some("Custom Window")
        );

        // Test duplicate
        let dup = duplicate_instance(mc_dir, "1.20.1", "1.20.1-copy").unwrap();
        assert_eq!(dup.id, "1.20.1-copy");
        assert!(
            versions_dir
                .join("1.20.1-copy")
                .join("1.20.1-copy.json")
                .exists()
        );

        // Test rename
        let renamed = rename_instance(mc_dir, "1.20.1-copy", "1.20.1-renamed").unwrap();
        assert_eq!(renamed.id, "1.20.1-renamed");
        assert!(!versions_dir.join("1.20.1-copy").exists());
        assert!(
            versions_dir
                .join("1.20.1-renamed")
                .join("1.20.1-renamed.json")
                .exists()
        );

        // Test case-only rename
        let case_renamed = rename_instance(mc_dir, "1.20.1-renamed", "1.20.1-RENAMED").unwrap();
        assert_eq!(case_renamed.id, "1.20.1-RENAMED");
        assert!(versions_dir.join("1.20.1-RENAMED").exists());

        // Test delete permanent
        delete_instance_ext(mc_dir, "1.20.1-RENAMED", true).unwrap();
        assert!(!versions_dir.join("1.20.1-RENAMED").exists());
    }

    #[test]
    fn test_ensure_game_folder() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mc_dir = temp_dir.path().join("game_root");
        assert!(!mc_dir.exists());
        ensure_game_folder(&mc_dir).unwrap();
        assert!(mc_dir.join("versions").is_dir());
        assert!(mc_dir.join("launcher_profiles.json").is_file());
    }

    #[test]
    fn test_card_classification_and_sorting() {
        let mut instances = vec![
            InstanceInfo {
                id: "inst-vanilla-old".to_string(),
                name: "Vanilla Old".to_string(),
                mc_version: "1.20.1".to_string(),
                release_time: Some("2023-06-12T14:00:00Z".to_string()),
                ..Default::default()
            },
            InstanceInfo {
                id: "inst-vanilla-new".to_string(),
                name: "Vanilla New".to_string(),
                mc_version: "1.20.4".to_string(),
                release_time: Some("2023-12-07T14:00:00Z".to_string()),
                ..Default::default()
            },
            InstanceInfo {
                id: "inst-fabric".to_string(),
                name: "Fabric Inst".to_string(),
                mc_version: "1.20.1".to_string(),
                loaders: vec!["Fabric 0.15.11".to_string()],
                release_time: Some("2023-06-12T14:00:00Z".to_string()),
                ..Default::default()
            },
            InstanceInfo {
                id: "inst-star".to_string(),
                name: "Starred Inst".to_string(),
                mc_version: "1.20.1".to_string(),
                config: InstanceConfig {
                    starred: true,
                    pinned: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            InstanceInfo {
                id: "inst-fool".to_string(),
                name: "Fool Inst".to_string(),
                mc_version: "24w14potato".to_string(),
                ..Default::default()
            },
            InstanceInfo {
                id: "inst-hidden".to_string(),
                name: "Hidden Inst".to_string(),
                config: InstanceConfig {
                    card_type: Some(CardKind::Hidden),
                    ..Default::default()
                },
                ..Default::default()
            },
        ];

        classify_cards(&mut instances);

        assert_eq!(instances[0].id, "inst-star");
        assert_eq!(instances[0].card, CardKind::Star);

        assert_eq!(instances[1].id, "inst-fabric");
        assert_eq!(instances[1].card, CardKind::Api);
        assert!(instances[1].modable);

        assert_eq!(instances[2].id, "inst-vanilla-new");
        assert_eq!(instances[2].card, CardKind::Regular);

        // Vanilla instances remain Regular (no PCL rubbish demotion)
        assert_eq!(instances[3].id, "inst-vanilla-old");
        assert_eq!(instances[3].card, CardKind::Regular);

        assert_eq!(instances[4].id, "inst-fool");
        assert_eq!(instances[4].card, CardKind::Fool);

        assert_eq!(instances[5].id, "inst-hidden");
        assert_eq!(instances[5].card, CardKind::Hidden);
    }
}
