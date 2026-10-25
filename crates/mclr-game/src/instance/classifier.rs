//! Instance version and loader classification, modability, and card categorization.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use super::{CardKind, InstanceInfo, InstanceKind, InstanceState};
use crate::version::VersionJson;

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
        } else if name.starts_with("net.minecraftforge:forge:")
            || name.starts_with("net.minecraftforge:minecraftforge:")
        {
            has_forge = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            let clean_ver = ver.split('-').last().unwrap_or(ver);
            detected_loaders.push(format!("Forge {clean_ver}"));
        } else if name.starts_with("net.neoforged.fancymodloader:loader:")
            || name.starts_with("net.neoforged:neoforge:")
            || name.starts_with("net.neoforged:forge:")
        {
            has_neoforge = true;
            let ver = name.split(':').nth(2).unwrap_or("");
            let clean_ver = ver.split('-').last().unwrap_or(ver);
            detected_loaders.push(if clean_ver.is_empty() {
                "NeoForge".to_string()
            } else {
                format!("NeoForge {clean_ver}")
            });
        } else if name.starts_with("net.neoforged") {
            has_neoforge = true;
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
        } else if (main.contains("neoforged") || main.contains("neoforge")) && !has_neoforge {
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

    // Check arguments for NeoForge / Forge if not detected yet
    if !has_neoforge {
        let mut found_ver = None;
        if let Some(args) = &version_json.arguments {
            let mut next_is_ver = false;
            for game_arg in &args.game {
                let s = match game_arg {
                    crate::version::ArgumentEntry::Plain(s) => s.as_str(),
                    crate::version::ArgumentEntry::Conditional(cond) => {
                        if let crate::version::ArgumentValue::Single(s) = &cond.value {
                            s.as_str()
                        } else {
                            ""
                        }
                    }
                };
                if next_is_ver && !s.is_empty() && !s.starts_with('-') {
                    found_ver = Some(s.to_string());
                    break;
                }
                if s == "--fml.neoForgeVersion" {
                    next_is_ver = true;
                }
            }
        }
        if let Some(v) = found_ver {
            has_neoforge = true;
            detected_loaders.push(format!("NeoForge {v}"));
        } else if version_json.id.to_lowercase().contains("neoforge")
            || version_json
                .main_class
                .as_deref()
                .unwrap_or("")
                .contains("neoforge")
        {
            has_neoforge = true;
            detected_loaders.push("NeoForge".to_string());
        }
    }

    if has_neoforge && !detected_loaders.iter().any(|l| l.starts_with("NeoForge")) {
        let mut ver_str = None;
        for lib in &version_json.libraries {
            if lib.name.starts_with("net.neoforged") {
                let v = lib.name.split(':').nth(2).unwrap_or("");
                if !v.is_empty() {
                    ver_str = Some(v.to_string());
                    break;
                }
            }
        }
        detected_loaders.push(match ver_str {
            Some(v) => format!("NeoForge {v}"),
            None => "NeoForge".to_string(),
        });
    }

    // Keep only the single most specific loader entry per loader family
    let mut clean_loaders: Vec<String> = Vec::new();
    let mut seen_families: std::collections::HashSet<String> = std::collections::HashSet::new();
    detected_loaders.sort_by_key(|a| std::cmp::Reverse(a.len()));
    for l in detected_loaders {
        let family = l.split_whitespace().next().unwrap_or(&l).to_string();
        if seen_families.insert(family) {
            clean_loaders.push(l);
        }
    }
    clean_loaders.sort();
    detected_loaders = clean_loaders;

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

pub(crate) fn loader_priority_score(loaders: &[String]) -> u32 {
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

pub(crate) fn top_loader_version(loaders: &[String]) -> String {
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
