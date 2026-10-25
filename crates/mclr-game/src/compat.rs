//! Version compatibility, loader eligibility, April Fools detection, and Java recommendations.

use chrono::{DateTime, Datelike, Duration};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VersionCompatInfo {
    pub is_april_fools: bool,
    pub fool_code: Option<String>,
    pub supported_loaders: Vec<String>,
    pub incompatible_pairs: Vec<(String, String)>,
    pub recommended_java: u32,
}

/// Compares two version strings numerically and lexicographically.
pub fn compare_versions(v1: &str, v2: &str) -> std::cmp::Ordering {
    fn parse_parts(s: &str) -> Vec<Result<u64, &str>> {
        s.split(['.', '-', '_'])
            .filter(|p| !p.is_empty())
            .map(|p| p.parse::<u64>().map_err(|_| p))
            .collect()
    }
    let p1 = parse_parts(v1);
    let p2 = parse_parts(v2);

    let max_len = p1.len().max(p2.len());
    for i in 0..max_len {
        let part1 = p1.get(i);
        let part2 = p2.get(i);

        match (part1, part2) {
            (None, None) => break,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(Ok(n1)), Some(Ok(n2))) => {
                if n1 != n2 {
                    return n1.cmp(n2);
                }
            }
            (Some(Err(s1)), Some(Err(s2))) => {
                if s1 != s2 {
                    return s1.cmp(s2);
                }
            }
            (Some(Ok(_)), Some(Err(_))) => return std::cmp::Ordering::Greater,
            (Some(Err(_)), Some(Ok(_))) => return std::cmp::Ordering::Less,
        }
    }

    std::cmp::Ordering::Equal
}

/// Calculates drop score for Minecraft versions (e.g. 1.20.1 -> 200, 1.14 -> 140).
pub fn drop_of(mc_version: &str) -> u32 {
    let clean = mc_version.trim();
    if let Some(rest) = clean.strip_prefix("1.") {
        let minor = rest
            .split('.')
            .next()
            .and_then(|m| m.parse::<u32>().ok())
            .unwrap_or(0);
        return minor * 10;
    }

    // Modern 2-digit versions e.g. 26.1 -> 260
    if let Some(pos) = clean.find('.') {
        if let Ok(first) = clean[..pos].parse::<u32>() {
            if first >= 26 {
                return first * 10;
            }
        }
    }

    // Snapshots: 24w14a -> 210
    if clean.len() >= 3 && clean.chars().take(2).all(|c| c.is_ascii_digit()) && clean.contains('w')
    {
        let yr = clean[..2].parse::<u32>().unwrap_or(0);
        return match yr {
            y if y >= 24 => 210,
            23 => 200,
            22 => 190,
            21 => 180,
            20 => 160,
            19 => 150,
            18 => 130,
            17 => 120,
            16 => 110,
            15 => 90,
            _ => 0,
        };
    }

    0
}

/// Detects whether a version is an April Fools update and returns the i18n fool code.
pub fn detect_april_fools(
    mc_version: &str,
    release_type: &str,
    release_time: &str,
) -> (bool, Option<String>) {
    let lower = mc_version.trim().to_lowercase();

    if lower.starts_with("2.0") || lower.starts_with("2point0") {
        if lower.ends_with("red") {
            return (true, Some("2.0_red".to_string()));
        }
        if lower.ends_with("blue") {
            return (true, Some("2.0_blue".to_string()));
        }
        if lower.ends_with("purple") {
            return (true, Some("2.0_purple".to_string()));
        }
        return (true, Some("2.0".to_string()));
    }

    if lower == "15w14a" {
        return (true, Some("15w14a".to_string()));
    }
    if lower == "1.rv-pre1" {
        return (true, Some("1.rv-pre1".to_string()));
    }
    if lower == "3d shareware v1.34" || lower == "3d_shareware_v1.34" {
        return (true, Some("3d_shareware".to_string()));
    }
    if lower.starts_with("20w14inf") || lower == "20w14∞" || lower == "20w14infinite" {
        return (true, Some("20w14infinite".to_string()));
    }
    if lower == "22w13oneblockatatime" {
        return (true, Some("22w13oneblockatatime".to_string()));
    }
    if lower == "23w13a_or_b" {
        return (true, Some("23w13a_or_b".to_string()));
    }
    if lower == "24w14potato" {
        return (true, Some("24w14potato".to_string()));
    }
    if lower == "25w14craftmine" {
        return (true, Some("25w14craftmine".to_string()));
    }
    if lower == "26w14a" {
        return (true, Some("26w14a".to_string()));
    }

    if release_type.eq_ignore_ascii_case("special") {
        return (true, Some(lower));
    }

    // Release time check: April 1st (shifted +2h for timezone)
    if !release_time.is_empty() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(release_time) {
            let shifted = dt + Duration::hours(2);
            if shifted.month() == 4 && shifted.day() == 1 {
                return (true, Some(lower));
            }
        }
    }

    (false, None)
}

/// Pure calculation of version compatibility, supported loaders, incompatibilities, and Java version.
pub fn compute_version_compat(
    mc_version: &str,
    release_type: &str,
    release_time: &str,
) -> VersionCompatInfo {
    let (is_april_fools, fool_code) = detect_april_fools(mc_version, release_type, release_time);
    let drop = drop_of(mc_version);

    // Format fit check
    let is_standard_release = mc_version.starts_with("1.")
        || mc_version
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count()
            >= 2;

    let mut supported_loaders = Vec::new();

    // Fabric: >= 1.14 (drop >= 140 or drop >= 130 snapshot)
    if drop >= 130 {
        supported_loaders.push("fabric".to_string());
    }

    // Quilt: >= 1.14
    if drop >= 130 {
        supported_loaders.push("quilt".to_string());
    }

    // NeoForge: >= 1.20.1 (1.20.1 and >= 1.20.2; 1.20.0 is not supported)
    if is_standard_release
        && compare_versions(mc_version, "1.20.1").is_ge()
        && mc_version != "1.20"
        && mc_version != "1.20.0"
    {
        supported_loaders.push("neoforge".to_string());
    }

    // Forge: >= 1.1
    if is_standard_release && compare_versions(mc_version, "1.1").is_ge() {
        supported_loaders.push("forge".to_string());
    }

    // OptiFine: >= 1.7.2
    if compare_versions(mc_version, "1.7.2").is_ge() {
        supported_loaders.push("optifine".to_string());
    }

    // LiteLoader: >= 1.5.2 and <= 1.12.2
    if is_standard_release
        && compare_versions(mc_version, "1.5.2").is_ge()
        && compare_versions(mc_version, "1.12.2").is_le()
    {
        supported_loaders.push("liteloader".to_string());
    }

    // Cleanroom: exactly 1.12.2
    if mc_version == "1.12.2" {
        supported_loaders.push("cleanroom".to_string());
    }

    // LegacyFabric: drop >= 30 and drop <= 130
    if (30..=130).contains(&drop) {
        supported_loaders.push("legacyfabric".to_string());
    }

    // LabyMod: >= 1.8.9
    if is_standard_release && compare_versions(mc_version, "1.8.9").is_ge() {
        supported_loaders.push("labymod".to_string());
    }

    // Incompatible loader pairs
    let mut incompatible_pairs = vec![
        ("fabric".to_string(), "forge".to_string()),
        ("fabric".to_string(), "neoforge".to_string()),
        ("forge".to_string(), "neoforge".to_string()),
        ("cleanroom".to_string(), "optifine".to_string()),
        ("cleanroom".to_string(), "liteloader".to_string()),
        ("neoforge".to_string(), "liteloader".to_string()),
        ("labymod".to_string(), "optifine".to_string()),
        ("labymod".to_string(), "liteloader".to_string()),
        ("legacyfabric".to_string(), "liteloader".to_string()),
    ];

    // Fabric + OptiFine incompatible for >= 1.20.5
    if compare_versions(mc_version, "1.20.5").is_ge() {
        incompatible_pairs.push(("fabric".to_string(), "optifine".to_string()));
    }
    // NeoForge + OptiFine incompatible for >= 1.20.2
    if compare_versions(mc_version, "1.20.2").is_ge() {
        incompatible_pairs.push(("neoforge".to_string(), "optifine".to_string()));
    }

    // Forge + OptiFine incompatible for 1.13 to 1.14.3
    if compare_versions(mc_version, "1.13").is_ge()
        && compare_versions(mc_version, "1.14.3").is_le()
    {
        incompatible_pairs.push(("forge".to_string(), "optifine".to_string()));
    }

    // Recommended Java:
    // >= 1.20.5 (or snapshots >= 24w): 21
    // >= 1.18 (or snapshots >= 21w44a): 17
    // == 1.17: 16
    // < 1.17: 8
    let recommended_java = if drop >= 210 || compare_versions(mc_version, "1.20.5").is_ge() {
        21
    } else if drop >= 180 || compare_versions(mc_version, "1.18").is_ge() {
        17
    } else if drop >= 170 || compare_versions(mc_version, "1.17").is_ge() {
        16
    } else {
        8
    };

    VersionCompatInfo {
        is_april_fools,
        fool_code,
        supported_loaders,
        incompatible_pairs,
        recommended_java,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare_versions() {
        assert_eq!(
            compare_versions("1.20.1", "1.20.1"),
            std::cmp::Ordering::Equal
        );
        assert_eq!(
            compare_versions("1.20.1", "1.20"),
            std::cmp::Ordering::Greater
        );
        assert_eq!(
            compare_versions("1.5.2", "1.12.2"),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_versions("1.20.4", "1.20.5"),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_april_fools_detection() {
        let res = detect_april_fools("24w14potato", "snapshot", "2024-04-01T10:00:00Z");
        assert!(res.0);
        assert_eq!(res.1.as_deref(), Some("24w14potato"));

        let res2 = detect_april_fools("15w14a", "snapshot", "2015-04-01T10:00:00Z");
        assert!(res2.0);
        assert_eq!(res2.1.as_deref(), Some("15w14a"));

        let res3 = detect_april_fools("2.0_red", "special", "");
        assert!(res3.0);
        assert_eq!(res3.1.as_deref(), Some("2.0_red"));

        let res4 = detect_april_fools("1.20.1", "release", "2023-06-12T00:00:00Z");
        assert!(!res4.0);
        assert_eq!(res4.1, None);
    }

    #[test]
    fn test_compute_version_compat() {
        let c1_20_1 = compute_version_compat("1.20.1", "release", "");
        assert!(!c1_20_1.is_april_fools);
        assert_eq!(c1_20_1.recommended_java, 17);
        assert!(c1_20_1.supported_loaders.contains(&"fabric".to_string()));
        assert!(c1_20_1.supported_loaders.contains(&"neoforge".to_string()));

        let c1_20_6 = compute_version_compat("1.20.6", "release", "");
        assert_eq!(c1_20_6.recommended_java, 21);
        assert!(c1_20_6.supported_loaders.contains(&"neoforge".to_string()));
        assert!(
            c1_20_6
                .incompatible_pairs
                .contains(&("fabric".to_string(), "optifine".to_string()))
        );

        let c1_12_2 = compute_version_compat("1.12.2", "release", "");
        assert_eq!(c1_12_2.recommended_java, 8);
        assert!(c1_12_2.supported_loaders.contains(&"cleanroom".to_string()));
        assert!(
            c1_12_2
                .supported_loaders
                .contains(&"liteloader".to_string())
        );
        assert!(c1_12_2.supported_loaders.contains(&"forge".to_string()));
    }
}
