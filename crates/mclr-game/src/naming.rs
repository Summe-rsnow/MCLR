//! Instance folder / display name validation (PCL FolderNameValidator parity).
//! Owned by slice B1; `instance::validate_instance_name` delegates here.

use crate::error::GameError;
use mclr_core::AppError;
use std::path::Path;

const FORBIDDEN_CHARS: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '!', ';'];

const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub fn validate_instance_name_ext(name: &str, versions_dir: Option<&Path>) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::new("instance.name.empty"));
    }

    if name.chars().count() > 70 {
        return Err(AppError::new("instance.name.tooLong"));
    }

    if name.starts_with(' ') || name.ends_with(' ') || name.contains(FORBIDDEN_CHARS) {
        return Err(AppError::new("instance.name.invalidChar"));
    }

    if name.ends_with('.') {
        return Err(AppError::new("instance.name.trailingDot"));
    }

    let upper = name.to_ascii_uppercase();
    let base = upper.split('.').next().unwrap_or("");
    if RESERVED_NAMES.contains(&upper.as_str()) || RESERVED_NAMES.contains(&base) {
        return Err(AppError::new("instance.name.reserved"));
    }

    if let Some(v_dir) = versions_dir {
        if v_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(v_dir) {
                for entry in entries.flatten() {
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        if let Some(folder_name) = entry.file_name().to_str() {
                            if folder_name.eq_ignore_ascii_case(name) {
                                return Err(AppError::new("instance.name.duplicate"));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn validate_name(name: &str, existing_versions_dir: Option<&Path>) -> Result<(), GameError> {
    validate_instance_name_ext(name, existing_versions_dir)
        .map_err(|e| GameError::InvalidInstanceName(e.code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_instance_name_rules() {
        assert_eq!(
            validate_instance_name_ext("", None).unwrap_err().code,
            "instance.name.empty"
        );

        let long_name = "a".repeat(71);
        assert_eq!(
            validate_instance_name_ext(&long_name, None)
                .unwrap_err()
                .code,
            "instance.name.tooLong"
        );

        assert_eq!(
            validate_instance_name_ext(" hello", None).unwrap_err().code,
            "instance.name.invalidChar"
        );
        assert_eq!(
            validate_instance_name_ext("hello ", None).unwrap_err().code,
            "instance.name.invalidChar"
        );
        for forbidden in FORBIDDEN_CHARS {
            let bad = format!("test{forbidden}inst");
            assert_eq!(
                validate_instance_name_ext(&bad, None).unwrap_err().code,
                "instance.name.invalidChar"
            );
        }

        assert_eq!(
            validate_instance_name_ext("inst.", None).unwrap_err().code,
            "instance.name.trailingDot"
        );

        assert_eq!(
            validate_instance_name_ext("con", None).unwrap_err().code,
            "instance.name.reserved"
        );
        assert_eq!(
            validate_instance_name_ext("PRN", None).unwrap_err().code,
            "instance.name.reserved"
        );
        assert_eq!(
            validate_instance_name_ext("aux.json", None)
                .unwrap_err()
                .code,
            "instance.name.reserved"
        );
        assert_eq!(
            validate_instance_name_ext("com1", None).unwrap_err().code,
            "instance.name.reserved"
        );
        assert_eq!(
            validate_instance_name_ext("LPT9", None).unwrap_err().code,
            "instance.name.reserved"
        );

        assert!(validate_instance_name_ext("1.20.1-Fabric_0.15.11", None).is_ok());
        assert!(validate_instance_name_ext("contact", None).is_ok());
    }

    #[test]
    fn test_validate_instance_name_duplicate() {
        let temp = tempdir().unwrap();
        let versions_dir = temp.path().join("versions");
        std::fs::create_dir_all(versions_dir.join("1.20.1-fabric")).unwrap();

        assert_eq!(
            validate_instance_name_ext("1.20.1-Fabric", Some(&versions_dir))
                .unwrap_err()
                .code,
            "instance.name.duplicate"
        );
        assert_eq!(
            validate_instance_name_ext("1.20.1-FABRIC", Some(&versions_dir))
                .unwrap_err()
                .code,
            "instance.name.duplicate"
        );
        assert!(validate_instance_name_ext("1.20.1-forge", Some(&versions_dir)).is_ok());
    }
}
