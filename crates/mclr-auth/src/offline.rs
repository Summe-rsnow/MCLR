//! Offline Minecraft account UUID generation and default skin model.

use crate::error::AuthError;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OfflineUuidMode {
    Standard,
    Legacy,
    Custom,
}

impl std::str::FromStr for OfflineUuidMode {
    type Err = AuthError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "standard" => Ok(OfflineUuidMode::Standard),
            "legacy" => Ok(OfflineUuidMode::Legacy),
            "custom" => Ok(OfflineUuidMode::Custom),
            other => Err(AuthError::Other(format!("unknown uuid mode: {other}"))),
        }
    }
}

/// Computes the offline account UUID string (32 lowercase hex characters without hyphens).
pub fn compute_offline_uuid(
    name: &str,
    mode: OfflineUuidMode,
    custom: Option<&str>,
) -> Result<String, AuthError> {
    match mode {
        OfflineUuidMode::Standard => {
            let mut hasher = Md5::new();
            hasher.update(format!("OfflinePlayer:{name}").as_bytes());
            let mut hash = hasher.finalize();
            hash[6] = (hash[6] & 0x0F) | 0x30;
            hash[8] = (hash[8] & 0x3F) | 0x80;
            Ok(hex::encode(hash))
        }
        OfflineUuidMode::Legacy => {
            let len_hex = format!("{:0>16X}", name.chars().count());
            let mut h: u64 = 5381;
            for c in name.encode_utf16() {
                h = (h << 5) ^ h ^ (c as u64);
            }
            let hash_val = h ^ 0xA98F501BC684032F;
            let hash_hex = format!("{:0>16X}", hash_val);
            let full = format!("{len_hex}{hash_hex}");
            let value = format!("{}3{}9{}", &full[0..12], &full[13..16], &full[17..32]);
            Ok(value.to_ascii_lowercase())
        }
        OfflineUuidMode::Custom => {
            let raw = custom.ok_or_else(|| AuthError::Other("custom UUID required".to_string()))?;
            let cleaned: String = raw
                .chars()
                .filter(|c| *c != '-' && !c.is_whitespace())
                .collect();
            if cleaned.len() != 32 || !cleaned.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(AuthError::Other(format!(
                    "invalid custom UUID: '{raw}', expected 32 hex chars"
                )));
            }
            Ok(cleaned.to_ascii_lowercase())
        }
    }
}

/// Determines whether the default skin model for the given 32-char hex UUID is `"slim"` (Alex) or `"classic"` (Steve).
pub fn default_skin_model(uuid_hex_32: &str) -> &'static str {
    let cleaned: String = uuid_hex_32
        .chars()
        .filter(|c| *c != '-' && !c.is_whitespace())
        .collect();
    if cleaned.len() != 32 {
        return "classic";
    }

    let nibble = |idx: usize| -> u8 {
        cleaned
            .get(idx..idx + 1)
            .and_then(|s| u8::from_str_radix(s, 16).ok())
            .unwrap_or(0)
    };

    let a = nibble(7);
    let b = nibble(15);
    let c = nibble(23);
    let d = nibble(31);

    if ((a ^ b ^ c ^ d) % 2) != 0 {
        "slim"
    } else {
        "classic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_offline_uuid_steve() {
        let uuid = compute_offline_uuid("Steve", OfflineUuidMode::Standard, None).unwrap();
        // Minecraft standard offline UUID for Steve
        assert_eq!(uuid, "5627dd98e6be3c21b8a8e92344183641");
    }

    #[test]
    fn test_legacy_offline_uuid() {
        let uuid = compute_offline_uuid("Steve", OfflineUuidMode::Legacy, None).unwrap();
        assert_eq!(uuid.len(), 32);
        assert_eq!(&uuid[12..13], "3");
        assert_eq!(&uuid[16..17], "9");
    }

    #[test]
    fn test_custom_offline_uuid() {
        let raw = "12345678-1234-1234-1234-123456789abc";
        let uuid = compute_offline_uuid("Steve", OfflineUuidMode::Custom, Some(raw)).unwrap();
        assert_eq!(uuid, "12345678123412341234123456789abc");

        let bad = "invalid-uuid";
        assert!(compute_offline_uuid("Steve", OfflineUuidMode::Custom, Some(bad)).is_err());
    }

    #[test]
    fn test_default_skin_model() {
        // Steve: 5627dd98e6be3c21b8a8e92344183641
        // indices: 7='8' (8), 15='1' (1), 23='3' (3), 31='1' (1)
        // (8 ^ 1 ^ 3 ^ 1) = 8 ^ 3 = 11. 11 % 2 = 1 -> "slim"
        assert_eq!(
            default_skin_model("5627dd98e6be3c21b8a8e92344183641"),
            "slim"
        );
        // Test with classic
        // Let's craft one where xor is even: e.g. all 0
        let zeros = "00000000000000000000000000000000";
        assert_eq!(default_skin_model(zeros), "classic");
    }
}
