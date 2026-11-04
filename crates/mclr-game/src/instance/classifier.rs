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
fn maven_version(coordinate: &str) -> &str {
    coordinate.split(':').nth(2).unwrap_or("")
}

fn strip_maven_classifier(version: &str) -> &str {
    for suffix in [
        "-universal",
        "-client",
        "-server",
        "-installer",
        "-shim",
        "-extra",
    ] {
        if let Some(stripped) = version.strip_suffix(suffix) {
            return stripped;
        }
    }
    version
}

fn after_mc_prefix(version: &str) -> &str {
    match version.split_once('-') {
        Some((head, tail)) if head.chars().next().is_some_and(|c| c.is_ascii_digit()) => {
            strip_maven_classifier(tail)
        }
        _ => strip_maven_classifier(version),
    }
}

fn strip_build_metadata(version: &str) -> &str {
    version.split('+').next().unwrap_or(version)
}

fn optifine_version(raw: &str) -> String {
    match raw.find("HD_U_") {
        Some(idx) => raw[idx..].to_string(),
        None => raw.to_string(),
    }
}

fn version_from_game_args(version_json: &VersionJson, flag: &str) -> Option<String> {
    if let Some(args) = &version_json.arguments {
        let mut expect_value = false;
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
            if expect_value {
                if !s.is_empty() && !s.starts_with("--") {
                    return Some(s.to_string());
                }
                expect_value = false;
            }
            if s == flag {
                expect_value = true;
            }
        }
    }

    if let Some(raw_args) = &version_json.minecraft_arguments {
        let mut expect_value = false;
        for token in raw_args.split_whitespace() {
            if expect_value {
                if !token.is_empty() && !token.starts_with("--") {
                    return Some(token.to_string());
                }
                expect_value = false;
            }
            if token == flag {
                expect_value = true;
            }
        }
    }
    None
}

fn mc_version_from_neoforge(neoforge_ver: &str) -> Option<String> {
    let clean = strip_maven_classifier(neoforge_ver);
    let mut parts = clean.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    if major >= 26 {
        Some(format!("{major}.{minor}"))
    } else if major >= 20 {
        if minor == 0 {
            Some(format!("1.{major}"))
        } else {
            Some(format!("1.{major}.{minor}"))
        }
    } else {
        None
    }
}

fn is_plausible_mc_version(s: &str) -> bool {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return false;
    }
    MC_VER_REGEX.is_match(trimmed)
}

/// Extract Minecraft version from version JSON, arguments, libraries, and string hints.
pub fn extract_mc_version(version_json: &VersionJson, fallback_id: &str) -> String {
    // 1. Inherits from (if it contains a valid version or isn't self-referential)
    if let Some(inherits) = &version_json.inherits_from {
        if !inherits.is_empty() && inherits != fallback_id && inherits != &version_json.id {
            if let Some(caps) = MC_VER_REGEX.captures(inherits) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
            return inherits.clone();
        }
    }

    // 2. Check game arguments (--fml.mcVersion <ver>)
    if let Some(arg_ver) = version_from_game_args(version_json, "--fml.mcVersion") {
        if let Some(caps) = MC_VER_REGEX.captures(&arg_ver) {
            if let Some(m) = caps.get(1) {
                return m.as_str().to_string();
            }
        }
        if is_plausible_mc_version(&arg_ver) {
            return arg_ver;
        }
    }

    // 3. Check libraries coordinates
    for lib in &version_json.libraries {
        let name = &lib.name;
        let ver = maven_version(name);
        if ver.is_empty() {
            continue;
        }

        // Fabric / LegacyFabric intermediary (e.g. net.fabricmc:intermediary:1.20.1)
        if name.starts_with("net.fabricmc:intermediary")
            || name.starts_with("net.legacyfabric:intermediary")
        {
            if let Some(caps) = MC_VER_REGEX.captures(ver) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
            return ver.to_string();
        }

        // Forge (e.g. net.minecraftforge:forge:1.20.1-47.2.0)
        if (name.starts_with("net.minecraftforge:forge:")
            || name.starts_with("net.neoforged:forge:"))
            && ver.contains('-')
        {
            let mc_part = ver.split('-').next().unwrap_or("");
            if let Some(caps) = MC_VER_REGEX.captures(mc_part) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
        }

        // NeoForge 20.x+ (e.g. net.neoforged:neoforge:21.1.248 -> 1.21.1)
        if name.starts_with("net.neoforged:neoforge:") {
            if let Some(derived) = mc_version_from_neoforge(ver) {
                return derived;
            }
        }

        // Vanilla client / server
        if name.starts_with("net.minecraft:client:") || name.starts_with("com.mojang:minecraft:") {
            if let Some(caps) = MC_VER_REGEX.captures(ver) {
                if let Some(m) = caps.get(1) {
                    return m.as_str().to_string();
                }
            }
        }
    }

    // 4. Try parsing from version_json.id using regex
    if let Some(caps) = MC_VER_REGEX.captures(&version_json.id) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }

    // 5. Try from fallback_id (folder name)
    if let Some(caps) = MC_VER_REGEX.captures(fallback_id) {
        if let Some(m) = caps.get(1) {
            return m.as_str().to_string();
        }
    }

    // 6. Check asset_index id
    if let Some(asset_idx) = &version_json.asset_index {
        if let Some(caps) = MC_VER_REGEX.captures(&asset_idx.id) {
            if let Some(m) = caps.get(1) {
                return m.as_str().to_string();
            }
        }
    }

    // 7. If id itself looks like a Minecraft version
    if is_plausible_mc_version(&version_json.id) {
        version_json.id.clone()
    } else if is_plausible_mc_version(fallback_id) {
        fallback_id.to_string()
    } else {
        "Unknown".to_string()
    }
}
#[derive(Default)]
struct DetectedLoaders {
    fabric: Option<String>,
    legacy_fabric: Option<String>,
    quilt: Option<String>,
    forge: Option<String>,
    neoforge: Option<String>,
    cleanroom: Option<String>,
    optifine: Option<String>,
    liteloader: Option<String>,
    labymod: bool,
}

/// Classify instance kind and extract loaders from version JSON and libraries.
pub fn classify_instance(
    version_json: &VersionJson,
    mc_version: &str,
) -> (InstanceKind, Vec<String>) {
    let mut found = DetectedLoaders::default();

    // 1. Game arguments win: PCL reads the loader version from `--fml.*Version`.
    found.neoforge = version_from_game_args(version_json, "--fml.neoForgeVersion");
    found.forge = version_from_game_args(version_json, "--fml.forgeVersion");

    // 2. Maven coordinates: first hit per loader family wins.
    for lib in &version_json.libraries {
        let name = &lib.name;
        let version = maven_version(name);

        if name.starts_with("net.fabricmc:fabric-loader") {
            found.fabric.get_or_insert_with(|| {
                strip_build_metadata(strip_maven_classifier(version)).to_string()
            });
        } else if name.starts_with("net.legacyfabric") || name.contains("legacyfabric:intermediary")
        {
            found.legacy_fabric.get_or_insert_with(|| {
                strip_build_metadata(strip_maven_classifier(version)).to_string()
            });
        } else if name.starts_with("org.quiltmc:quilt-loader") {
            found.quilt.get_or_insert_with(|| {
                strip_build_metadata(strip_maven_classifier(version)).to_string()
            });
        } else if name.starts_with("net.neoforged.fancymodloader:loader") {
            // FancyModLoader version ≠ NeoForge version; presence only.
            if found.neoforge.is_none() {
                found.neoforge = Some(String::new());
            }
        } else if name.starts_with("net.neoforged:neoforge:")
            || name.starts_with("net.neoforged:forge:")
        {
            // 1.20.2+ coordinates carry no MC prefix (`20.4.80-beta`), while the
            // 1.20.1-era `net.neoforged:forge` coordinate does (`1.20.1-47.1.99`).
            let clean = if name.starts_with("net.neoforged:forge:") {
                after_mc_prefix(version).to_string()
            } else {
                strip_maven_classifier(version).to_string()
            };
            // A concrete versioned coordinate beats the presence-only FML marker.
            if !clean.is_empty() && found.neoforge.as_deref().unwrap_or("").is_empty() {
                found.neoforge = Some(clean);
            }
        } else if name.starts_with("net.minecraftforge:forge:")
            || name.starts_with("net.minecraftforge:minecraftforge:")
            || name.starts_with("net.minecraftforge:fmlloader:")
        {
            let clean = after_mc_prefix(version).to_string();
            if found.forge.is_none() && !clean.is_empty() {
                found.forge = Some(clean);
            }
        } else if name.starts_with("com.cleanroommc:cleanroom") {
            found
                .cleanroom
                .get_or_insert_with(|| strip_maven_classifier(version).to_string());
        } else if name.starts_with("optifine:OptiFine") || name.to_lowercase().contains("optifine")
        {
            if found.optifine.is_none() && !version.is_empty() {
                found.optifine = Some(optifine_version(version));
            }
        } else if name.starts_with("com.mumfrey:liteloader") {
            found
                .liteloader
                .get_or_insert_with(|| strip_maven_classifier(version).to_string());
        } else if name.starts_with("net.labymod") {
            found.labymod = true;
        }
    }

    // 3. mainClass hints only prove presence (no version information).
    if let Some(main) = &version_json.main_class {
        if main.contains("fabricmc") && found.fabric.is_none() && found.quilt.is_none() {
            found.fabric = Some(String::new());
        } else if (main.contains("neoforged") || main.contains("neoforge"))
            && found.neoforge.is_none()
        {
            found.neoforge = Some(String::new());
        } else if (main.contains("minecraftforge") || main.contains("fml"))
            && found.forge.is_none()
            && found.neoforge.is_none()
        {
            found.forge = Some(String::new());
        } else if main.contains("OptiFine") && found.optifine.is_none() {
            found.optifine = Some(String::new());
        }
    }

    // 4. Version id / name hints, last resort.
    let id_lower = version_json.id.to_lowercase();
    if found.neoforge.is_none() && id_lower.contains("neoforge") {
        found.neoforge = Some(String::new());
    }
    if found.forge.is_none() && !id_lower.contains("neoforge") && id_lower.contains("forge") {
        found.forge = Some(String::new());
    }

    fn label(name: &str, version: &Option<String>) -> Option<String> {
        let v = version.as_deref().unwrap_or("").trim();
        if v.is_empty() {
            None
        } else {
            Some(format!("{name} {v}"))
        }
    }
    fn entry(name: &str, version: &Option<String>) -> Option<String> {
        version.as_ref().map(|v| {
            let v = v.trim();
            if v.is_empty() {
                name.to_string()
            } else {
                format!("{name} {v}")
            }
        })
    }

    let mut detected_loaders: Vec<String> = Vec::new();
    for (name, version) in [
        ("Fabric", &found.fabric),
        ("LegacyFabric", &found.legacy_fabric),
        ("Quilt", &found.quilt),
        ("NeoForge", &found.neoforge),
        ("Forge", &found.forge),
        ("Cleanroom", &found.cleanroom),
        ("OptiFine", &found.optifine),
        ("LiteLoader", &found.liteloader),
    ] {
        if let Some(entry) = label(name, version).or_else(|| entry(name, version)) {
            detected_loaders.push(entry);
        }
    }
    if found.labymod {
        detected_loaders.push("LabyMod".to_string());
    }
    detected_loaders.sort();

    let has_fabric = found.fabric.is_some();
    let has_legacy_fabric = found.legacy_fabric.is_some();
    let has_forge = found.forge.is_some();
    let has_neoforge = found.neoforge.is_some();
    let has_cleanroom = found.cleanroom.is_some();
    let has_optifine = found.optifine.is_some();
    let has_liteloader = found.liteloader.is_some();
    let has_labymod = found.labymod;
    let has_quilt = found.quilt.is_some();

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

/// Canonical loader id for a detected label, as the mod platforms index it.
///
/// Detection produces display labels (`"Fabric 0.15.11"`, `"NeoForge 21.1.0"`) and instance
/// kinds (`"forge"`); Modrinth and CurseForge only understand `fabric`, `forge`, `neoforge`,
/// `quilt` and `liteloader`. Filtering with a label silently hides or mislabels every update,
/// so anything that is not indexed is reported as unknown instead of guessed.
pub fn canonical_loader_id(label: &str) -> Option<&'static str> {
    let lower = label.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    // Platforms do not index these flavours: a filter would hide every real update.
    if lower.contains("legacyfabric") || lower.contains("cleanroom") || lower.contains("labymod") {
        return None;
    }
    // `neoforge` before `forge`, because the longer name contains the shorter one.
    ["neoforge", "quilt", "fabric", "forge", "liteloader"]
        .into_iter()
        .find(|id| lower.contains(id))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::{ArgumentEntry, Library, VersionJson};

    #[test]
    fn canonical_loader_id_maps_detection_labels_to_platform_ids() {
        // Detection labels carry a version suffix.
        assert_eq!(canonical_loader_id("Fabric 0.15.11"), Some("fabric"));
        assert_eq!(canonical_loader_id("NeoForge 21.1.0"), Some("neoforge"));
        assert_eq!(canonical_loader_id("Forge 47.2.0"), Some("forge"));
        assert_eq!(canonical_loader_id("Quilt 0.24.0"), Some("quilt"));
        assert_eq!(canonical_loader_id("LiteLoader"), Some("liteloader"));

        // Instance kinds arrive bare and lowercased.
        assert_eq!(canonical_loader_id("forge"), Some("forge"));
        assert_eq!(canonical_loader_id("NEOFORGE"), Some("neoforge"));

        // Flavours the platforms do not index must not filter: an id would hide every update.
        assert_eq!(canonical_loader_id("LegacyFabric 0.14.5"), None);
        assert_eq!(canonical_loader_id("Cleanroom 0.2.4"), None);
        assert_eq!(canonical_loader_id("LabyMod"), None);
        assert_eq!(canonical_loader_id("Vanilla"), None);
        assert_eq!(canonical_loader_id(""), None);
    }

    fn vjson(id: &str, libs: &[&str], main: Option<&str>) -> VersionJson {
        VersionJson {
            id: id.to_string(),
            inherits_from: Some("1.20.1".to_string()),
            main_class: main.map(|s| s.to_string()),
            libraries: libs
                .iter()
                .map(|name| Library {
                    name: name.to_string(),
                    url: None,
                    downloads: None,
                    rules: Vec::new(),
                    natives: Default::default(),
                    extract: None,
                })
                .collect(),
            ..Default::default()
        }
    }

    fn loaders(json: &VersionJson, mc: &str) -> Vec<String> {
        classify_instance(json, mc).1
    }

    fn with_args(mut json: VersionJson, pairs: &[(&str, &str)]) -> VersionJson {
        let mut game: Vec<ArgumentEntry> = Vec::new();
        for (flag, value) in pairs {
            game.push(ArgumentEntry::Plain(flag.to_string()));
            game.push(ArgumentEntry::Plain(value.to_string()));
        }
        json.arguments = Some(crate::version::Arguments {
            game,
            jvm: Vec::new(),
        });
        json
    }

    #[test]
    fn neoforge_uses_argument_version_not_fancymodloader() {
        // 26.3 JSON: FML loader lib comes first, NeoForge coordinate second.
        let json = with_args(
            vjson(
                "26.3-neoforge-26.3.0",
                &[
                    "net.neoforged.fancymodloader:loader:4.0.9",
                    "net.neoforged:neoforge:26.3.0",
                ],
                Some("cpw.mods.bootstraplauncher.BootstrapLauncher"),
            ),
            &[("--fml.neoForgeVersion", "26.3.0")],
        );
        assert_eq!(loaders(&json, "26.3"), vec!["NeoForge 26.3.0"]);
    }

    #[test]
    fn neoforge_keeps_prerelease_suffix() {
        // 1.20.2+ scheme: 20.4.80-beta bundles the suffix in the version.
        let json = vjson(
            "1.20.4-neoforge-20.4.80-beta",
            &["net.neoforged:neoforge:20.4.80-beta"],
            None,
        );
        assert_eq!(loaders(&json, "1.20.4"), vec!["NeoForge 20.4.80-beta"]);
    }

    #[test]
    fn neoforge_1_20_1_uses_mc_prefixed_coordinate() {
        let json = vjson(
            "1.20.1-neoforge-47.1.99",
            &["net.neoforged:forge:1.20.1-47.1.99"],
            None,
        );
        assert_eq!(loaders(&json, "1.20.1"), vec!["NeoForge 47.1.99"]);
    }

    #[test]
    fn fancymodloader_alone_reports_loader_without_version() {
        let json = vjson(
            "26.3-neoforge",
            &["net.neoforged.fancymodloader:loader:4.0.9"],
            None,
        );
        assert_eq!(loaders(&json, "26.3"), vec!["NeoForge"]);
    }

    #[test]
    fn forge_strips_universal_classifier() {
        let json = vjson(
            "1.20.1-forge-47.2.0",
            &["net.minecraftforge:forge:1.20.1-47.2.0-universal"],
            Some("cpw.mods.bootstraplauncher.BootstrapLauncher"),
        );
        assert_eq!(loaders(&json, "1.20.1"), vec!["Forge 47.2.0"]);
    }

    #[test]
    fn forge_1_12_2_coordinate_and_argument_agree() {
        let json = with_args(
            vjson(
                "1.12.2-forge-14.23.5.2860",
                &[
                    "net.minecraftforge:forge:1.12.2-14.23.5.2860",
                    "net.minecraftforge:minecraftforge:1.12.2-14.23.5.2860",
                ],
                Some("net.minecraft.launchwrapper.Launch"),
            ),
            &[("--fml.forgeVersion", "14.23.5.2860")],
        );
        // Exactly one Forge entry, whatever the coordinate flavour.
        assert_eq!(loaders(&json, "1.12.2"), vec!["Forge 14.23.5.2860"]);
    }

    #[test]
    fn fabric_and_quilt_strip_build_metadata_but_keep_pre_tags() {
        let fabric = vjson(
            "1.20.1-fabric",
            &[
                "net.fabricmc:fabric-loader:0.15.11+build.1",
                "net.fabricmc:intermediary:1.20.1",
            ],
            Some("net.fabricmc.loader.impl.launch.knot.KnotClient"),
        );
        assert_eq!(loaders(&fabric, "1.20.1"), vec!["Fabric 0.15.11"]);

        let quilt = vjson(
            "1.20.1-quilt",
            &["org.quiltmc:quilt-loader:0.23.1-beta.3+build.7"],
            Some("org.quiltmc.loader.impl.launch.knot.KnotClient"),
        );
        assert_eq!(loaders(&quilt, "1.20.1"), vec!["Quilt 0.23.1-beta.3"]);
    }

    #[test]
    fn cleanroom_keeps_alpha_suffix() {
        let json = vjson(
            "1.12.2-cleanroom",
            &["com.cleanroommc:cleanroom:0.2.4-alpha"],
            Some("com.cleanroommc.boot.Main"),
        );
        assert_eq!(loaders(&json, "1.12.2"), vec!["Cleanroom 0.2.4-alpha"]);
    }

    #[test]
    fn optifine_reports_hd_u_version() {
        let json = vjson(
            "1.20.1-optifine",
            &["optifine:OptiFine:1.20.1_HD_U_I6"],
            Some("net.minecraft.launchwrapper.Launch"),
        );
        assert_eq!(loaders(&json, "1.20.1"), vec!["OptiFine HD_U_I6"]);
    }

    #[test]
    fn liteloader_and_labymod_keep_their_versions() {
        let lite = vjson(
            "1.12.2-liteloader",
            &["com.mumfrey:liteloader:1.12.2-SNAPSHOT-r5"],
            None,
        );
        assert_eq!(
            loaders(&lite, "1.12.2"),
            vec!["LiteLoader 1.12.2-SNAPSHOT-r5"]
        );

        let laby = vjson("26.3-labymod", &["net.labymod:LabyMod:4"], None);
        assert_eq!(loaders(&laby, "26.3"), vec!["LabyMod"]);
    }

    #[test]
    fn multiple_loaders_each_report_their_own_version_once() {
        let json = vjson(
            "1.20.1-forge-optifine",
            &[
                "net.minecraftforge:forge:1.20.1-47.2.0",
                "optifine:OptiFine:1.20.1_HD_U_I6",
                "net.minecraftforge:fmlloader:1.20.1-47.2.0",
            ],
            Some("cpw.mods.bootstraplauncher.BootstrapLauncher"),
        );
        assert_eq!(
            loaders(&json, "1.20.1"),
            vec!["Forge 47.2.0", "OptiFine HD_U_I6"]
        );
    }

    #[test]
    fn test_extract_mc_version_from_neoforge_and_args() {
        // Tensura Neo Otherworld case: custom modpack id without inheritsFrom
        let tensura_json = VersionJson {
            id: "Tensura Neo Otherworld".to_string(),
            inherits_from: None,
            libraries: vec![Library {
                name: "net.neoforged:neoforge:21.1.248".to_string(),
                url: None,
                downloads: None,
                rules: Vec::new(),
                natives: Default::default(),
                extract: None,
            }],
            ..Default::default()
        };
        assert_eq!(
            extract_mc_version(&tensura_json, "Tensura Neo Otherworld"),
            "1.21.1"
        );

        // From game arguments --fml.mcVersion
        let forge_json = VersionJson {
            id: "Custom Modpack".to_string(),
            inherits_from: None,
            arguments: Some(crate::version::Arguments {
                game: vec![
                    crate::version::ArgumentEntry::Plain("--fml.mcVersion".to_string()),
                    crate::version::ArgumentEntry::Plain("1.20.1".to_string()),
                ],
                jvm: Vec::new(),
            }),
            ..Default::default()
        };
        assert_eq!(extract_mc_version(&forge_json, "Custom Modpack"), "1.20.1");

        // Fabric intermediary library
        let fabric_json = VersionJson {
            id: "My Pack".to_string(),
            inherits_from: None,
            libraries: vec![Library {
                name: "net.fabricmc:intermediary:1.20.4".to_string(),
                url: None,
                downloads: None,
                rules: Vec::new(),
                natives: Default::default(),
                extract: None,
            }],
            ..Default::default()
        };
        assert_eq!(extract_mc_version(&fabric_json, "My Pack"), "1.20.4");

        // Completely custom title with no version signals -> Unknown, not the custom title
        let unknown_json = VersionJson {
            id: "My Totally Custom Pack".to_string(),
            inherits_from: None,
            ..Default::default()
        };
        assert_eq!(
            extract_mc_version(&unknown_json, "My Totally Custom Pack"),
            "Unknown"
        );
    }
}
