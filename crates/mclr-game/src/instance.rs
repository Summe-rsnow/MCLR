//! Minecraft instance model, classification, scan, and file operations.

use crate::error::GameError;
use crate::version::{VersionJson, merge_version_json};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs;
use std::path::Path;

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

/// Per-instance configuration stored in `<instance_dir>/mclr.instance.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub icon: String,
    pub category: String,
    pub pinned: bool,
    pub isolation: String, // "global" | "on" | "off"
    pub java: Option<String>,
    pub memory: Option<u32>,
    pub jvm_args: Option<String>,
    pub game_args: Option<String>,
    pub window_title: Option<String>,
    pub pre_launch_command: Option<String>,
    pub login_requirement: String, // "any" | "microsoft" | "thirdParty" | "microsoftOrThirdParty"
    pub server_quick_play: Option<String>,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            icon: "grass".to_string(),
            category: "Default".to_string(),
            pinned: false,
            isolation: "global".to_string(),
            java: None,
            memory: None,
            jvm_args: None,
            game_args: None,
            window_title: None,
            pre_launch_command: None,
            login_requirement: "any".to_string(),
            server_quick_play: None,
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
}

/// Validates an instance name.
/// Forbidden characters: \ / : * ? " < > | ! ;
pub fn validate_instance_name(name: &str) -> Result<(), GameError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot be empty".to_string(),
        ));
    }
    const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '!', ';'];
    if trimmed.contains(FORBIDDEN) {
        return Err(GameError::InvalidInstanceName(format!(
            "Instance name contains invalid characters: {trimmed}"
        )));
    }
    Ok(())
}

/// Extract Minecraft version from version JSON and string hints.
pub fn extract_mc_version(version_json: &VersionJson, fallback_id: &str) -> String {
    if let Some(inherits) = &version_json.inherits_from {
        if !inherits.is_empty() {
            return inherits.clone();
        }
    }

    // 2. Try parsing from id using regex
    if let Ok(re) = regex::Regex::new(
        r"((?:1|[2-9][0-9])\.[0-9]+(?:\.[0-9]+)?(?:-(?:pre|rc|snapshot)[0-9]*)?|\d+w\d+[a-z])",
    ) {
        if let Some(caps) = re.captures(&version_json.id) {
            if let Some(m) = caps.get(1) {
                return m.as_str().to_string();
            }
        }
    }

    // 3. Try from fallback_id
    if let Ok(re) = regex::Regex::new(
        r"((?:1|[2-9][0-9])\.[0-9]+(?:\.[0-9]+)?(?:-(?:pre|rc|snapshot)[0-9]*)?|\d+w\d+[a-z])",
    ) {
        if let Some(caps) = re.captures(fallback_id) {
            if let Some(m) = caps.get(1) {
                return m.as_str().to_string();
            }
        }
    }

    if !version_json.id.is_empty() {
        version_json.id.clone()
    } else {
        fallback_id.to_string()
    }
}

/// Classify instance kind and extract loaders from version JSON and libraries.
pub fn classify_instance(
    version_json: &VersionJson,
    mc_version: &str,
) -> (InstanceKind, Vec<String>) {
    let mut detected_loaders = Vec::new();
    let mut has_fabric = false;
    let mut has_legacy_fabric = false;
    let mut has_forge = false;
    let mut has_neoforge = false;
    let mut has_cleanroom = false;
    let mut has_optifine = false;
    let mut has_liteloader = false;
    let mut has_labymod = false;
    let mut has_quilt = false;

    // Check libraries Maven coordinates
    for lib in &version_json.libraries {
        let name = &lib.name;
        if name.starts_with("net.fabricmc:fabric-loader") {
            has_fabric = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            let clean_ver = ver.split("+build").next().unwrap_or(ver);
            detected_loaders.push(format!("Fabric {clean_ver}"));
        } else if name.starts_with("net.legacyfabric") || name.contains("legacyfabric:intermediary")
        {
            has_legacy_fabric = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            let clean_ver = ver.split("+build").next().unwrap_or(ver);
            detected_loaders.push(format!("LegacyFabric {clean_ver}"));
        } else if name.starts_with("org.quiltmc:quilt-loader") {
            has_quilt = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            let clean_ver = ver.split("+build").next().unwrap_or(ver);
            detected_loaders.push(format!("Quilt {clean_ver}"));
        } else if name.starts_with("com.cleanroommc:cleanroom") {
            has_cleanroom = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            detected_loaders.push(format!("Cleanroom {ver}"));
        } else if name.starts_with("net.minecraftforge:forge")
            || name.starts_with("net.minecraftforge:minecraftforge")
            || name.starts_with("net.minecraftforge:fmlloader")
        {
            has_forge = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            let clean_ver = ver.split('-').last().unwrap_or(ver);
            detected_loaders.push(format!("Forge {clean_ver}"));
        } else if name.starts_with("net.neoforged:neoforge")
            || name.starts_with("net.neoforged:fancyassembly")
        {
            has_neoforge = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            detected_loaders.push(format!("NeoForge {ver}"));
        } else if name.starts_with("optifine:OptiFine") || name.to_lowercase().contains("optifine")
        {
            has_optifine = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            detected_loaders.push(format!("OptiFine {ver}"));
        } else if name.starts_with("com.mumfrey:liteloader") {
            has_liteloader = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            detected_loaders.push(format!("LiteLoader {ver}"));
        } else if name.starts_with("net.labymod") {
            has_labymod = true;
            detected_loaders.push("LabyMod".to_string());
        }
    }

    // Check mainClass if not detected in libraries
    if let Some(main) = &version_json.main_class {
        if main.contains("fabricmc") && !has_fabric && !has_quilt {
            has_fabric = true;
            detected_loaders.push("Fabric".to_string());
        } else if main.contains("neoforged") && !has_neoforge {
            has_neoforge = true;
            detected_loaders.push("NeoForge".to_string());
        } else if (main.contains("minecraftforge") || main.contains("fml"))
            && !has_forge
            && !has_neoforge
        {
            has_forge = true;
            detected_loaders.push("Forge".to_string());
        } else if main.contains("OptiFine") && !has_optifine {
            has_optifine = true;
            detected_loaders.push("OptiFine".to_string());
        }
    }

    // Deduplicate loaders
    detected_loaders.sort();
    detected_loaders.dedup();

    // Determine primary mod loader count (excluding OptiFine/LiteLoader in multiple loader count)
    let primary_loaders_count = [
        has_fabric,
        has_legacy_fabric,
        has_forge,
        has_neoforge,
        has_cleanroom,
        has_quilt,
        has_labymod,
    ]
    .iter()
    .filter(|&&b| b)
    .count();

    let kind = if primary_loaders_count > 1 {
        InstanceKind::ModdedMultiple
    } else if has_cleanroom {
        InstanceKind::Cleanroom
    } else if has_neoforge {
        InstanceKind::NeoForge
    } else if has_forge {
        InstanceKind::Forge
    } else if has_quilt {
        InstanceKind::Quilt
    } else if has_legacy_fabric {
        InstanceKind::LegacyFabric
    } else if has_fabric {
        InstanceKind::Fabric
    } else if has_labymod {
        InstanceKind::LabyMod
    } else if has_optifine {
        InstanceKind::OptiFine
    } else if has_liteloader {
        InstanceKind::LiteLoader
    } else {
        // Vanilla family
        let ver_type = version_json.version_type.as_deref().unwrap_or("release");
        if ver_type.eq_ignore_ascii_case("snapshot")
            || mc_version.contains('w')
            || mc_version.contains("-pre")
            || mc_version.contains("-rc")
        {
            InstanceKind::Snapshot
        } else if ver_type.eq_ignore_ascii_case("old_beta") || mc_version.starts_with("b1.") {
            InstanceKind::OldBeta
        } else if ver_type.eq_ignore_ascii_case("old_alpha")
            || mc_version.starts_with("a1.")
            || mc_version.starts_with("c0.")
            || mc_version.starts_with("rd-")
        {
            InstanceKind::OldAlpha
        } else {
            InstanceKind::Vanilla
        }
    };

    (kind, detected_loaders)
}

/// Scans instances inside `<mc_folder>/versions`.
pub fn scan_instances(mc_folder: &Path) -> Result<Vec<InstanceInfo>, GameError> {
    let versions_dir = mc_folder.join("versions");
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut instances = Vec::new();
    let entries = fs::read_dir(&versions_dir).map_err(GameError::Io)?;

    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if !file_type.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let instance_path = entry.path();
        let json_path = instance_path.join(format!("{dir_name}.json"));

        // Read config or default
        let config_path = instance_path.join("mclr.instance.json");
        let config = if config_path.exists() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|content| serde_json::from_str::<InstanceConfig>(&content).ok())
                .unwrap_or_default()
        } else {
            InstanceConfig::default()
        };

        if !json_path.exists() {
            instances.push(InstanceInfo {
                id: dir_name.clone(),
                name: dir_name.clone(),
                folder: mc_folder.to_string_lossy().to_string(),
                path: instance_path.to_string_lossy().to_string(),
                mc_version: "Unknown".to_string(),
                kind: InstanceKind::Vanilla,
                loaders: Vec::new(),
                state: InstanceState::Incomplete,
                config,
            });
            continue;
        }

        let json_str = match fs::read_to_string(&json_path) {
            Ok(s) => s,
            Err(_) => {
                instances.push(InstanceInfo {
                    id: dir_name.clone(),
                    name: dir_name.clone(),
                    folder: mc_folder.to_string_lossy().to_string(),
                    path: instance_path.to_string_lossy().to_string(),
                    mc_version: "Unknown".to_string(),
                    kind: InstanceKind::Vanilla,
                    loaders: Vec::new(),
                    state: InstanceState::Broken,
                    config,
                });
                continue;
            }
        };

        let version_json = match VersionJson::parse(&json_str) {
            Ok(v) => v,
            Err(_) => {
                instances.push(InstanceInfo {
                    id: dir_name.clone(),
                    name: dir_name.clone(),
                    folder: mc_folder.to_string_lossy().to_string(),
                    path: instance_path.to_string_lossy().to_string(),
                    mc_version: "Unknown".to_string(),
                    kind: InstanceKind::Vanilla,
                    loaders: Vec::new(),
                    state: InstanceState::Broken,
                    config,
                });
                continue;
            }
        };

        // If inherits_from is present, attempt to merge with parent if available
        let effective_json = if let Some(parent_id) = &version_json.inherits_from {
            let parent_json_path = versions_dir
                .join(parent_id)
                .join(format!("{parent_id}.json"));
            if parent_json_path.exists() {
                if let Ok(parent_str) = fs::read_to_string(&parent_json_path) {
                    if let Ok(parent_v) = VersionJson::parse(&parent_str) {
                        merge_version_json(version_json.clone(), parent_v)
                    } else {
                        version_json.clone()
                    }
                } else {
                    version_json.clone()
                }
            } else {
                version_json.clone()
            }
        } else {
            version_json.clone()
        };

        let mc_version = extract_mc_version(&effective_json, &dir_name);
        let (kind, loaders) = classify_instance(&effective_json, &mc_version);

        instances.push(InstanceInfo {
            id: dir_name.clone(),
            name: dir_name.clone(),
            folder: mc_folder.to_string_lossy().to_string(),
            path: instance_path.to_string_lossy().to_string(),
            mc_version,
            kind,
            loaders,
            state: InstanceState::Ok,
            config,
        });
    }

    Ok(instances)
}

/// Renames an instance: moves `<mc_folder>/versions/<id>` to `<mc_folder>/versions/<new_name>`
/// and renames `<id>.json` and `<id>.jar` if they match the original id.
pub fn rename_instance(
    mc_folder: &Path,
    id: &str,
    new_name: &str,
) -> Result<InstanceInfo, GameError> {
    validate_instance_name(new_name)?;

    let versions_dir = mc_folder.join("versions");
    let old_dir = versions_dir.join(id);
    if !old_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    let new_dir = versions_dir.join(new_name);
    if new_dir.exists() {
        return Err(GameError::InstanceExists(new_name.to_string()));
    }

    // Rename directory
    fs::rename(&old_dir, &new_dir).map_err(GameError::Io)?;

    // Rename json file if <id>.json exists
    let old_json = new_dir.join(format!("{id}.json"));
    let new_json = new_dir.join(format!("{new_name}.json"));
    if old_json.exists() {
        let _ = fs::rename(&old_json, &new_json);
        // Also update internal "id" in json if needed
        if let Ok(content) = fs::read_to_string(&new_json) {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object_mut() {
                    obj.insert(
                        "id".to_string(),
                        serde_json::Value::String(new_name.to_string()),
                    );
                    if let Ok(updated) = serde_json::to_string_pretty(&val) {
                        let _ = fs::write(&new_json, updated);
                    }
                }
            }
        }
    }

    // Rename jar file if <id>.jar exists
    let old_jar = new_dir.join(format!("{id}.jar"));
    let new_jar = new_dir.join(format!("{new_name}.jar"));
    if old_jar.exists() {
        let _ = fs::rename(old_jar, new_jar);
    }

    // Rescan single instance info
    let list = scan_instances(mc_folder)?;
    list.into_iter()
        .find(|i| i.id == new_name)
        .ok_or_else(|| GameError::InstanceNotFound(new_name.to_string()))
}

/// Recursively copies a directory.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), dest_child)?;
        }
    }
    Ok(())
}

/// Duplicates an instance to a new name.
pub fn duplicate_instance(
    mc_folder: &Path,
    id: &str,
    new_name: &str,
) -> Result<InstanceInfo, GameError> {
    validate_instance_name(new_name)?;

    let versions_dir = mc_folder.join("versions");
    let src_dir = versions_dir.join(id);
    if !src_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    let dst_dir = versions_dir.join(new_name);
    if dst_dir.exists() {
        return Err(GameError::InstanceExists(new_name.to_string()));
    }

    copy_dir_all(&src_dir, &dst_dir).map_err(GameError::Io)?;

    // Rename JSON file
    let old_json = dst_dir.join(format!("{id}.json"));
    let new_json = dst_dir.join(format!("{new_name}.json"));
    if old_json.exists() {
        let _ = fs::rename(&old_json, &new_json);
        if let Ok(content) = fs::read_to_string(&new_json) {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object_mut() {
                    obj.insert(
                        "id".to_string(),
                        serde_json::Value::String(new_name.to_string()),
                    );
                    if let Ok(updated) = serde_json::to_string_pretty(&val) {
                        let _ = fs::write(&new_json, updated);
                    }
                }
            }
        }
    }

    // Rename JAR file
    let old_jar = dst_dir.join(format!("{id}.jar"));
    let new_jar = dst_dir.join(format!("{new_name}.jar"));
    if old_jar.exists() {
        let _ = fs::rename(old_jar, new_jar);
    }

    let list = scan_instances(mc_folder)?;
    list.into_iter()
        .find(|i| i.id == new_name)
        .ok_or_else(|| GameError::InstanceNotFound(new_name.to_string()))
}

/// Deletes an instance folder. Moves to Windows Recycle Bin via `trash::delete`,
/// falling back to `fs::remove_dir_all` if `trash::delete` fails (e.g. in test/temp directories).
pub fn delete_instance(mc_folder: &Path, id: &str) -> Result<(), GameError> {
    let instance_dir = mc_folder.join("versions").join(id);
    if !instance_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    if let Err(e) = trash::delete(&instance_dir) {
        tracing::warn!(
            "trash::delete failed for {}: {e}; falling back to remove_dir_all",
            instance_dir.display()
        );
        fs::remove_dir_all(&instance_dir).map_err(GameError::Io)?;
    }

    Ok(())
}

/// Saves instance configuration to `<instance_dir>/mclr.instance.json`.
pub fn save_instance_config(
    mc_folder: &Path,
    id: &str,
    config: &InstanceConfig,
) -> Result<InstanceInfo, GameError> {
    let instance_dir = mc_folder.join("versions").join(id);
    if !instance_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    let config_path = instance_dir.join("mclr.instance.json");
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| GameError::InvalidVersionJson(e.to_string()))?;
    fs::write(&config_path, json).map_err(GameError::Io)?;

    let list = scan_instances(mc_folder)?;
    list.into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| GameError::InstanceNotFound(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_instance_name() {
        assert!(validate_instance_name("1.20.1-Fabric").is_ok());
        assert!(validate_instance_name("My Instance (1)").is_ok());
        assert!(validate_instance_name("").is_err());
        assert!(validate_instance_name("abc/def").is_err());
        assert!(validate_instance_name("abc:def").is_err());
        assert!(validate_instance_name("abc!def").is_err());
        assert!(validate_instance_name("abc;def").is_err());
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

        // ModdedMultiple
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

        // Scan instances
        let scanned = scan_instances(mc_dir).unwrap();
        assert_eq!(scanned.len(), 3);

        let v_info = scanned.iter().find(|i| i.id == "1.20.1").unwrap();
        assert_eq!(v_info.kind, InstanceKind::Vanilla);
        assert_eq!(v_info.state, InstanceState::Ok);

        let f_info = scanned.iter().find(|i| i.id == "1.20.1-fabric").unwrap();
        assert_eq!(f_info.kind, InstanceKind::Fabric);
        assert_eq!(f_info.state, InstanceState::Ok);
        assert_eq!(f_info.mc_version, "1.20.1");

        let b_info = scanned.iter().find(|i| i.id == "broken").unwrap();
        assert_eq!(b_info.state, InstanceState::Broken);

        // Test save config
        let mut cfg = InstanceConfig::default();
        cfg.pinned = true;
        cfg.window_title = Some("Custom Window".to_string());
        let updated = save_instance_config(mc_dir, "1.20.1", &cfg).unwrap();
        assert!(updated.config.pinned);
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

        // Test delete
        delete_instance(mc_dir, "1.20.1-renamed").unwrap();
        assert!(!versions_dir.join("1.20.1-renamed").exists());
    }
}
