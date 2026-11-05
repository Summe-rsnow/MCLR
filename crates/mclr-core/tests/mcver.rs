use mclr_core::mcver::*;
use std::cmp::Ordering;

#[test]
fn test_is_newer() {
    assert!(is_newer("0.0.12", "0.0.11"));
    assert!(is_newer("0.1.0", "0.0.11"));
    assert!(is_newer("1.0.0", "0.9.9"));
    assert!(!is_newer("0.0.11", "0.0.11"));
    assert!(!is_newer("0.0.10", "0.0.11"));
    assert!(!is_newer("0.0.11-beta", "0.0.11"));
    assert!(is_newer("0.0.11", "0.0.11-beta"));
    assert!(is_newer("0.0.11-beta.2", "0.0.11-beta.1"));
}

#[test]
fn test_compare() {
    assert_eq!(compare("1.20", "1.20.0"), Ordering::Equal);
    assert_eq!(compare("1.20.0", "1.20"), Ordering::Equal);
    assert_eq!(compare("1.20.1", "1.20"), Ordering::Greater);
    assert_eq!(compare("1.20", "1.20.1"), Ordering::Less);
    assert_eq!(compare("1.20.1", "1.20.10"), Ordering::Less);
    assert_eq!(compare("1.9", "1.10"), Ordering::Less);
    assert_eq!(compare("1.0-rc1", "1.0"), Ordering::Less);
    assert_eq!(compare("1.0", "1.0-rc1"), Ordering::Greater);
    assert_eq!(compare("47.3.0", "47.0.1"), Ordering::Greater);
    assert_eq!(compare("20.4.237-beta", "20.4.5-beta"), Ordering::Greater);
}

#[test]
fn test_compare_strict() {
    assert_eq!(compare_strict("1.20", "1.20.0"), Some(Ordering::Equal));
    assert_eq!(compare_strict("any", "1.20"), None);
    assert_eq!(compare_strict("1.20", ""), None);
    assert_eq!(compare_strict("[1.20]", "1.20"), None);
}

#[test]
fn test_is_release() {
    assert!(is_release("1.20.1"));
    assert!(is_release("1.7"));
    assert!(is_release("26.1.2"));
    assert!(is_release("26.1"));
    assert!(!is_release("24w14a"));
    assert!(!is_release("1.20-pre1"));
    assert!(!is_release("b1.7.3"));
}

#[test]
fn test_drop_of() {
    assert_eq!(drop_of("1.20.1"), 200);
    assert_eq!(drop_of("1.14"), 140);
    assert_eq!(drop_of("26.1"), 260);
    assert_eq!(drop_of("24w14a"), 210);
    assert_eq!(drop_of("15w14a"), 90);
}

#[test]
fn test_major_key() {
    assert_eq!(major_key("1.20.1"), Some("1.20"));
    assert_eq!(major_key("1.20"), Some("1.20"));
    assert_eq!(major_key("26.1.2"), Some("26.1"));
    assert_eq!(major_key("26.1"), Some("26.1"));
    assert_eq!(major_key("24w14a"), None);
    assert_eq!(major_key("1.20-pre1"), None);
}

#[test]
fn test_known() {
    assert_eq!(known(" 1.20.1 "), Some("1.20.1"));
    assert_eq!(known("Unknown"), None);
    assert_eq!(known("unknown"), None);
    assert_eq!(known(""), None);
    assert_eq!(known("   "), None);
}
