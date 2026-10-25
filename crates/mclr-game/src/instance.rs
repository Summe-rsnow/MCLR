//! Minecraft instance model, classification, scan, and file operations.

use crate::error::GameError;
use crate::version::{VersionJson, merge_version_json};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

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

/// Validates an instance name (PCL FolderNameValidator parity).
/// Max 70 chars, non-empty, no leading/trailing space or trailing dot,
/// forbidden chars: \ / : * ? " < > | ! ;, and no DOS device names.
pub fn validate_instance_name(name: &str) -> Result<(), GameError> {
    if name.is_empty() {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot be empty".to_string(),
        ));
    }
    if name.starts_with(' ') || name.ends_with(' ') {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot start or end with spaces".to_string(),
        ));
    }
    if name.ends_with('.') {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot end with a period".to_string(),
        ));
    }
    if name.chars().count() > 70 {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot exceed 70 characters".to_string(),
        ));
    }
    const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '!', ';'];
    if name.contains(FORBIDDEN) {
        return Err(GameError::InvalidInstanceName(format!(
            "Instance name contains invalid characters: {name}"
        )));
    }
    const RESERVED_DOS: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let base_name = name.split('.').next().unwrap_or(name);
    for reserved in RESERVED_DOS {
        if base_name.eq_ignore_ascii_case(reserved) {
            return Err(GameError::InvalidInstanceName(format!(
                "Instance name is a reserved system name: {name}"
            )));
        }
    }
    Ok(())
}

static MC_VER_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"((?:1|[2-9][0-9])\.[0-9]+(?:\.[0-9]+)?(?:-(?:pre|rc|snapshot)[0-9]*)?|\d+w\d+[a-z])",
    )
    .expect("valid regex")
});

/// Extract Minecraft version from version JSON and string hints.
pub fn extract_mc_version(version_json: &VersionJson, fallback_id: &str) -> String {
    if let Some(inherits) = &version_json.inherits_from {
        if !inherits.is_empty() {
            return inherits.clone();
        }
    }

    // 2. Try parsing from id using regex
    if let Some(caps) = MC_VER_REGEX.captures(&version_json.id) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }

    // 3. Try from fallback_id
    if let Some(caps) = MC_VER_REGEX.captures(fallback_id) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
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

pub fn extract_loader_versions(loaders: &[String]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for l in loaders {
        let trimmed = l.trim();
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        let (key, ver) = match parts[0].to_lowercase().as_str() {
            "fabric" => ("fabric", parts.get(1).copied().unwrap_or("")),
            "legacyfabric" => ("legacyFabric", parts.get(1).copied().unwrap_or("")),
            "quilt" => ("quilt", parts.get(1).copied().unwrap_or("")),
            "forge" => ("forge", parts.get(1).copied().unwrap_or("")),
            "neoforge" => ("neoforge", parts.get(1).copied().unwrap_or("")),
            "cleanroom" => ("cleanroom", parts.get(1).copied().unwrap_or("")),
            "liteloader" => ("liteloader", parts.get(1).copied().unwrap_or("")),
            "optifine" => ("optifine", parts.get(1).copied().unwrap_or("")),
            "labymod" => ("labymod", parts.get(1).copied().unwrap_or("present")),
            _ => (parts[0], parts.get(1).copied().unwrap_or("")),
        };
        map.insert(key.to_string(), ver.to_string());
    }
    map
}

pub fn is_modable_instance(loaders: &[String], card_type: Option<CardKind>) -> bool {
    if card_type == Some(CardKind::Api) {
        return true;
    }
    loaders.iter().any(|l| {
        let lower = l.to_lowercase();
        lower.starts_with("fabric")
            || lower.starts_with("legacyfabric")
            || lower.starts_with("quilt")
            || lower.starts_with("forge")
            || lower.starts_with("neoforge")
            || lower.starts_with("cleanroom")
            || lower.starts_with("liteloader")
    })
}

pub fn is_april_fools(mc_version: &str, id: &str, release_time: Option<&str>) -> bool {
    let id_lower = id.to_lowercase();
    let ver_lower = mc_version.to_lowercase();
    const FOOL_PATTERNS: &[&str] = &[
        "15w14a",
        "1.rv-pre1",
        "3d shareware v1.34",
        "20w14",
        "22w13oneblockatatime",
        "23w13a_or_b",
        "24w14potato",
        "25w14craftmine",
        "26w14a",
    ];
    if id_lower.starts_with("2.0") || ver_lower.starts_with("2.0") {
        return true;
    }
    for pat in FOOL_PATTERNS {
        if id_lower.contains(pat) || ver_lower.contains(pat) {
            return true;
        }
    }
    if let Some(rt) = release_time {
        if rt.contains("-04-01") {
            return true;
        }
    }
    false
}

fn loader_priority_score(loaders: &[String]) -> u32 {
    let mut score = 99;
    for l in loaders {
        let lower = l.to_lowercase();
        let s = if lower.starts_with("fabric") {
            1
        } else if lower.starts_with("quilt") {
            2
        } else if lower.starts_with("legacyfabric") {
            3
        } else if lower.starts_with("neoforge") {
            4
        } else if lower.starts_with("forge") {
            5
        } else if lower.starts_with("cleanroom") {
            6
        } else if lower.starts_with("labymod") {
            7
        } else if lower.starts_with("optifine") {
            8
        } else if lower.starts_with("liteloader") {
            9
        } else {
            50
        };
        if s < score {
            score = s;
        }
    }
    score
}

fn top_loader_version(loaders: &[String]) -> String {
    let best_score = loader_priority_score(loaders);
    for l in loaders {
        let lower = l.to_lowercase();
        let s = if lower.starts_with("fabric") {
            1
        } else if lower.starts_with("quilt") {
            2
        } else if lower.starts_with("legacyfabric") {
            3
        } else if lower.starts_with("neoforge") {
            4
        } else if lower.starts_with("forge") {
            5
        } else if lower.starts_with("cleanroom") {
            6
        } else if lower.starts_with("labymod") {
            7
        } else if lower.starts_with("optifine") {
            8
        } else if lower.starts_with("liteloader") {
            9
        } else {
            50
        };
        if s == best_score {
            let parts: Vec<&str> = l.split_whitespace().collect();
            return parts.get(1).unwrap_or(&"").to_string();
        }
    }
    String::new()
}

pub fn card_display_order(card: CardKind) -> u32 {
    match card {
        CardKind::Star => 0,
        CardKind::Api => 1,
        CardKind::Regular => 2,
        CardKind::Rubbish => 3,
        CardKind::Fool => 4,
        CardKind::Error => 5,
        CardKind::Hidden => 6,
    }
}

fn version_drop(ver: &str) -> u32 {
    let trimmed = ver.trim_start_matches(|c: char| !c.is_ascii_digit());
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() >= 2 && parts[0] == "1" {
        if let Ok(n) = parts[1].parse::<u32>() {
            return n * 10;
        }
    }
    0
}

/// Classifies card kind and modability for all instances and sorts them
/// per PCL-CE rules.
pub fn classify_cards(instances: &mut [InstanceInfo]) {
    for info in instances.iter_mut() {
        info.modable = is_modable_instance(&info.loaders, info.config.card_type);

        if info.config.card_type == Some(CardKind::Hidden) {
            info.card = CardKind::Hidden;
        } else if info.state != InstanceState::Ok {
            info.card = CardKind::Error;
        } else if info.config.starred || info.config.pinned {
            info.card = CardKind::Star;
        } else if let Some(user_card) = info.config.card_type {
            info.card = user_card;
        } else if is_april_fools(&info.mc_version, &info.id, info.release_time.as_deref()) {
            info.card = CardKind::Fool;
        } else if !info.loaders.is_empty()
            && info.loaders.iter().any(|l| {
                let lower = l.to_lowercase();
                lower.starts_with("fabric")
                    || lower.starts_with("quilt")
                    || lower.starts_with("legacyfabric")
                    || lower.starts_with("neoforge")
                    || lower.starts_with("forge")
                    || lower.starts_with("cleanroom")
                    || lower.starts_with("liteloader")
                    || lower.starts_with("labymod")
            })
        {
            info.card = CardKind::Api;
        } else if info.kind == InstanceKind::OldBeta
            || info.kind == InstanceKind::OldAlpha
            || info.mc_version.starts_with("b1.")
            || info.mc_version.starts_with("a1.")
            || info.mc_version.starts_with("c0.")
            || info.mc_version.starts_with("rd-")
            || info.kind == InstanceKind::Snapshot
        {
            info.card = CardKind::Rubbish;
        } else {
            info.card = CardKind::Regular;
        }
    }

    // Per-drop OptiFine/vanilla: all but newest in drop are Rubbish
    let mut drop_groups: HashMap<u32, Vec<usize>> = HashMap::new();
    for (idx, info) in instances.iter().enumerate() {
        if info.card == CardKind::Regular {
            let drop = version_drop(&info.mc_version);
            if drop > 0 {
                drop_groups.entry(drop).or_default().push(idx);
            }
        }
    }

    for indices in drop_groups.values() {
        if indices.len() > 1 {
            let mut newest_idx = indices[0];
            for &idx in &indices[1..] {
                let curr = &instances[idx];
                let best = &instances[newest_idx];
                let is_newer = match (&curr.release_time, &best.release_time) {
                    (Some(ca), Some(ba)) => ca > ba,
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => curr.mc_version > best.mc_version,
                };
                if is_newer {
                    newest_idx = idx;
                }
            }
            for &idx in indices {
                if idx != newest_idx && instances[idx].config.card_type.is_none() {
                    instances[idx].card = CardKind::Rubbish;
                }
            }
        }
    }

    instances.sort_by(|a, b| {
        let card_order_a = card_display_order(a.card);
        let card_order_b = card_display_order(b.card);
        if card_order_a != card_order_b {
            return card_order_a.cmp(&card_order_b);
        }
        match (&a.release_time, &b.release_time) {
            (Some(ra), Some(rb)) if ra != rb => return rb.cmp(ra),
            (Some(_), None) => return std::cmp::Ordering::Less,
            (None, Some(_)) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        let lp_a = loader_priority_score(&a.loaders);
        let lp_b = loader_priority_score(&b.loaders);
        if lp_a != lp_b {
            return lp_a.cmp(&lp_b);
        }
        let lv_a = top_loader_version(&a.loaders);
        let lv_b = top_loader_version(&b.loaders);
        if lv_a != lv_b {
            return lv_b.cmp(&lv_a);
        }
        b.name.cmp(&a.name)
    });
}

#[derive(Debug, Clone)]
struct CachedInstance {
    json_mtime: Option<SystemTime>,
    config_mtime: Option<SystemTime>,
    info: InstanceInfo,
}

#[derive(Default)]
struct ScanCache {
    instances: HashMap<String, CachedInstance>,
}

static INSTANCE_CACHE: LazyLock<Mutex<HashMap<PathBuf, ScanCache>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Ensures `<mc_folder>/versions` directory and `launcher_profiles.json` exist.
pub fn ensure_game_folder(path: &Path) -> Result<(), GameError> {
    let versions_dir = path.join("versions");
    if !versions_dir.exists() {
        fs::create_dir_all(&versions_dir).map_err(GameError::Io)?;
    }
    let profiles_json = path.join("launcher_profiles.json");
    if !profiles_json.exists() {
        let _ = fs::write(profiles_json, r#"{"profiles":{}}"#);
    }
    Ok(())
}

/// Scans instances inside `<mc_folder>/versions`.
pub fn scan_instances(mc_folder: &Path) -> Result<Vec<InstanceInfo>, GameError> {
    let versions_dir = mc_folder.join("versions");
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&versions_dir).map_err(GameError::Io)?;
    let mut active_ids = std::collections::HashSet::new();
    let mut instances = Vec::new();

    let mut cache_guard = INSTANCE_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let folder_cache = cache_guard.entry(mc_folder.to_path_buf()).or_default();

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

        // Skip cache directory, .mclr directories, and folders containing .mclr-installing marker
        if dir_name == "cache"
            || dir_name.starts_with(".mclr")
            || instance_path.join(".mclr-installing").exists()
        {
            continue;
        }

        // Skip empty directories
        let is_empty = fs::read_dir(&instance_path)
            .map(|mut it| it.next().is_none())
            .unwrap_or(true);
        if is_empty {
            continue;
        }

        active_ids.insert(dir_name.clone());

        let json_path = instance_path.join(format!("{dir_name}.json"));
        let config_path = instance_path.join("mclr.instance.json");

        let json_mtime = fs::metadata(&json_path)
            .ok()
            .and_then(|m| m.modified().ok());
        let config_mtime = fs::metadata(&config_path)
            .ok()
            .and_then(|m| m.modified().ok());

        // Check cache hit
        if let Some(cached) = folder_cache.instances.get(&dir_name) {
            if cached.json_mtime == json_mtime && cached.config_mtime == config_mtime {
                instances.push(cached.info.clone());
                continue;
            }
        }

        // Read config or default
        let mut config = if config_path.exists() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|content| serde_json::from_str::<InstanceConfig>(&content).ok())
                .unwrap_or_default()
        } else {
            InstanceConfig::default()
        };
        if config.pinned && !config.starred {
            config.starred = true;
        } else if config.starred && !config.pinned {
            config.pinned = true;
        }

        if !json_path.exists() {
            let info = InstanceInfo {
                id: dir_name.clone(),
                name: dir_name.clone(),
                folder: mc_folder.to_string_lossy().to_string(),
                path: instance_path.to_string_lossy().to_string(),
                mc_version: "Unknown".to_string(),
                kind: InstanceKind::Vanilla,
                loaders: Vec::new(),
                state: InstanceState::Incomplete,
                config,
                modable: false,
                card: CardKind::Error,
                release_time: None,
                loader_versions: BTreeMap::new(),
            };
            folder_cache.instances.insert(
                dir_name,
                CachedInstance {
                    json_mtime,
                    config_mtime,
                    info: info.clone(),
                },
            );
            instances.push(info);
            continue;
        }

        let json_str = match fs::read_to_string(&json_path) {
            Ok(s) => s,
            Err(_) => {
                let info = InstanceInfo {
                    id: dir_name.clone(),
                    name: dir_name.clone(),
                    folder: mc_folder.to_string_lossy().to_string(),
                    path: instance_path.to_string_lossy().to_string(),
                    mc_version: "Unknown".to_string(),
                    kind: InstanceKind::Vanilla,
                    loaders: Vec::new(),
                    state: InstanceState::Broken,
                    config,
                    modable: false,
                    card: CardKind::Error,
                    release_time: None,
                    loader_versions: BTreeMap::new(),
                };
                folder_cache.instances.insert(
                    dir_name,
                    CachedInstance {
                        json_mtime,
                        config_mtime,
                        info: info.clone(),
                    },
                );
                instances.push(info);
                continue;
            }
        };

        let version_json = match VersionJson::parse(&json_str) {
            Ok(v) => v,
            Err(_) => {
                let info = InstanceInfo {
                    id: dir_name.clone(),
                    name: dir_name.clone(),
                    folder: mc_folder.to_string_lossy().to_string(),
                    path: instance_path.to_string_lossy().to_string(),
                    mc_version: "Unknown".to_string(),
                    kind: InstanceKind::Vanilla,
                    loaders: Vec::new(),
                    state: InstanceState::Broken,
                    config,
                    modable: false,
                    card: CardKind::Error,
                    release_time: None,
                    loader_versions: BTreeMap::new(),
                };
                folder_cache.instances.insert(
                    dir_name,
                    CachedInstance {
                        json_mtime,
                        config_mtime,
                        info: info.clone(),
                    },
                );
                instances.push(info);
                continue;
            }
        };

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
        let release_time = effective_json
            .release_time
            .clone()
            .or_else(|| effective_json.time.clone());
        let loader_versions = extract_loader_versions(&loaders);
        let modable = is_modable_instance(&loaders, config.card_type);

        let info = InstanceInfo {
            id: dir_name.clone(),
            name: dir_name.clone(),
            folder: mc_folder.to_string_lossy().to_string(),
            path: instance_path.to_string_lossy().to_string(),
            mc_version,
            kind,
            loaders,
            state: InstanceState::Ok,
            config,
            modable,
            card: CardKind::Regular,
            release_time,
            loader_versions,
        };

        folder_cache.instances.insert(
            dir_name,
            CachedInstance {
                json_mtime,
                config_mtime,
                info: info.clone(),
            },
        );
        instances.push(info);
    }

    folder_cache
        .instances
        .retain(|id, _| active_ids.contains(id));

    classify_cards(&mut instances);

    for inst in &instances {
        if let Some(cached) = folder_cache.instances.get_mut(&inst.id) {
            cached.info = inst.clone();
        }
    }

    Ok(instances)
}

/// Renames an instance: moves `<mc_folder>/versions/<id>` to `<mc_folder>/versions/<new_name>`
/// and renames `<id>.json` and `<id>.jar` and `<id>-natives` if present.
pub fn rename_instance(
    mc_folder: &Path,
    id: &str,
    new_name: &str,
) -> Result<InstanceInfo, GameError> {
    let versions_dir = mc_folder.join("versions");
    let old_dir = versions_dir.join(id);
    if !old_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    if id == new_name {
        let list = scan_instances(mc_folder)?;
        return list
            .into_iter()
            .find(|i| i.id == new_name)
            .ok_or_else(|| GameError::InstanceNotFound(new_name.to_string()));
    }

    let case_only = id.eq_ignore_ascii_case(new_name);
    if case_only {
        crate::naming::validate_name(new_name, None)?;
    } else {
        crate::naming::validate_name(new_name, Some(&versions_dir))?;
    }

    let new_dir = versions_dir.join(new_name);
    if !case_only && new_dir.exists() {
        return Err(GameError::InstanceExists(new_name.to_string()));
    }
    if case_only {
        let temp_name = format!(
            "{id}__mclr_temp_rename_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let temp_dir = versions_dir.join(&temp_name);
        fs::rename(&old_dir, &temp_dir).map_err(GameError::Io)?;
        fs::rename(&temp_dir, &new_dir).map_err(GameError::Io)?;
    } else {
        fs::rename(&old_dir, &new_dir).map_err(GameError::Io)?;
    }

    // Rename json file if <id>.json exists
    let old_json = new_dir.join(format!("{id}.json"));
    let new_json = new_dir.join(format!("{new_name}.json"));
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

    // Rename jar file if <id>.jar exists
    let old_jar = new_dir.join(format!("{id}.jar"));
    let new_jar = new_dir.join(format!("{new_name}.jar"));
    if old_jar.exists() {
        let _ = fs::rename(old_jar, new_jar);
    }

    // Rename <id>-natives directory if exists
    let old_natives = new_dir.join(format!("{id}-natives"));
    let new_natives = new_dir.join(format!("{new_name}-natives"));
    if old_natives.exists() {
        let _ = fs::rename(old_natives, new_natives);
    }

    // Invalidate cache
    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(id);
            cache.instances.remove(new_name);
        }
    }

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
    let versions_dir = mc_folder.join("versions");
    crate::naming::validate_name(new_name, Some(&versions_dir))?;

    let src_dir = versions_dir.join(id);
    if !src_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    let dst_dir = versions_dir.join(new_name);
    if dst_dir.exists() {
        return Err(GameError::InstanceExists(new_name.to_string()));
    }

    copy_dir_all(&src_dir, &dst_dir).map_err(GameError::Io)?;

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

    let old_jar = dst_dir.join(format!("{id}.jar"));
    let new_jar = dst_dir.join(format!("{new_name}.jar"));
    if old_jar.exists() {
        let _ = fs::rename(old_jar, new_jar);
    }

    let old_natives = dst_dir.join(format!("{id}-natives"));
    let new_natives = dst_dir.join(format!("{new_name}-natives"));
    if old_natives.exists() {
        let _ = fs::rename(old_natives, new_natives);
    }

    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(new_name);
        }
    }

    let list = scan_instances(mc_folder)?;
    list.into_iter()
        .find(|i| i.id == new_name)
        .ok_or_else(|| GameError::InstanceNotFound(new_name.to_string()))
}

/// Deletes an instance folder, optionally permanently or to Recycle Bin.
pub fn delete_instance_ext(mc_folder: &Path, id: &str, permanent: bool) -> Result<(), GameError> {
    let instance_dir = mc_folder.join("versions").join(id);
    if !instance_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    if permanent {
        fs::remove_dir_all(&instance_dir).map_err(GameError::Io)?;
    } else if let Err(e) = trash::delete(&instance_dir) {
        tracing::warn!(
            "trash::delete failed for {}: {e}; falling back to remove_dir_all",
            instance_dir.display()
        );
        fs::remove_dir_all(&instance_dir).map_err(GameError::Io)?;
    }

    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(id);
        }
    }

    Ok(())
}

/// Deletes an instance folder. Moves to Windows Recycle Bin via `trash::delete`,
/// falling back to `fs::remove_dir_all`.
pub fn delete_instance(mc_folder: &Path, id: &str) -> Result<(), GameError> {
    delete_instance_ext(mc_folder, id, false)
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

    let mut config_to_save = config.clone();
    // Synchronize pinned and starred for backward compatibility
    config_to_save.pinned = config_to_save.starred;

    let config_path = instance_dir.join("mclr.instance.json");
    let json = serde_json::to_string_pretty(&config_to_save)
        .map_err(|e| GameError::InvalidVersionJson(e.to_string()))?;
    fs::write(&config_path, json).map_err(GameError::Io)?;

    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(id);
        }
    }

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

        // Older vanilla in same drop (1.20) was reclassified as Rubbish
        assert_eq!(instances[3].id, "inst-vanilla-old");
        assert_eq!(instances[3].card, CardKind::Rubbish);

        assert_eq!(instances[4].id, "inst-fool");
        assert_eq!(instances[4].card, CardKind::Fool);

        assert_eq!(instances[5].id, "inst-hidden");
        assert_eq!(instances[5].card, CardKind::Hidden);
    }
}
