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

use mclr_core::mcver::{compare, drop_of};

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

    let supported_loaders = mclr_core::loader::supported_for(mc_version)
        .into_iter()
        .map(|l| l.id().to_string())
        .collect();

    let incompatible_pairs = mclr_core::loader::incompatible_pairs(mc_version)
        .into_iter()
        .map(|(a, b)| (a.id().to_string(), b.id().to_string()))
        .collect();
    // Recommended Java:
    // >= 1.20.5 (or snapshots >= 24w): 21
    // >= 1.18 (or snapshots >= 21w44a): 17
    // == 1.17: 16
    // < 1.17: 8
    let recommended_java = if drop >= 210 || compare(mc_version, "1.20.5").is_ge() {
        21
    } else if drop >= 180 || compare(mc_version, "1.18").is_ge() {
        17
    } else if drop >= 170 || compare(mc_version, "1.17").is_ge() {
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
