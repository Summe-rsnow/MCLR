//! Launch-time Java agents that are not part of the vanilla version JSON.
//!
//! Both agents are embedded in the binary and materialized under
//! `<data>/tools/agents/` on first use, so launches never depend on a download
//! and never write into the game folder.
//!
//! * `lwjgl-unsafe-agent.jar` — restores the `sun.misc.Unsafe` native-memory
//!   fast path for LWJGL **3.4.1** (Minecraft 26.1+), which otherwise goes
//!   through the much slower FFM path on JDK 25+.
//! * `legacyfix.jar` — compatibility shims for Minecraft **1.5.x and older**
//!   (release time before 2013-06-25) running on a modern JRE.
//!
//! Both come from PCL-CE (Apache-2.0); see `THIRD_PARTY_NOTICES.md`.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use mclr_core::paths;
use tracing::warn;

/// Disambiguates concurrent temp files inside `ensure_agent`.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Version of LWJGL that benefits from the unsafe agent.
pub const LWJGL_UNSAFE_TARGET_VERSION: &str = "3.4.1";

/// Release date (exclusive upper bound) below which LegacyFix applies.
pub const LEGACY_FIX_CUTOFF: &str = "2013-06-25";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchAgent {
    /// `lwjgl-unsafe-agent.jar`
    LwjglUnsafe,
    /// `legacyfix.jar`
    LegacyFix,
}

impl LaunchAgent {
    pub fn file_name(self) -> &'static str {
        match self {
            LaunchAgent::LwjglUnsafe => "lwjgl-unsafe-agent.jar",
            LaunchAgent::LegacyFix => "legacyfix.jar",
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            LaunchAgent::LwjglUnsafe => include_bytes!("../../assets/lwjgl-unsafe-agent.jar"),
            LaunchAgent::LegacyFix => include_bytes!("../../assets/legacyfix.jar"),
        }
    }
}

/// Writes the agent to `<data>/tools/agents/<name>` when missing or truncated.
///
/// Never fails the launch: a write error is logged and `None` is returned so the
/// caller simply skips the `-javaagent` argument.
pub fn ensure_agent(agent: LaunchAgent) -> Option<PathBuf> {
    let dir = paths::tools_dir().join("agents");
    let path = dir.join(agent.file_name());
    let bytes = agent.bytes();

    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() as usize == bytes.len() {
            return Some(path);
        }
    }

    if let Err(e) = fs::create_dir_all(&dir) {
        warn!("Failed to create agent directory {:?}: {e}", dir);
        return None;
    }

    // Write-then-rename so a crashed write can never leave a half jar behind.
    // The temp name is unique per call: two concurrent launches (or parallel
    // tests) must never write into the same temp file.
    let tmp = dir.join(format!(
        "{}.{}-{}.tmp",
        agent.file_name(),
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if let Err(e) = fs::write(&tmp, bytes) {
        warn!("Failed to write agent {:?}: {e}", tmp);
        return None;
    }
    if let Err(e) = fs::rename(&tmp, &path) {
        let _ = fs::remove_file(&tmp);
        // A concurrent writer may have moved an identical jar into place first.
        if let Ok(meta) = fs::metadata(&path)
            && meta.len() as usize == bytes.len()
        {
            return Some(path);
        }
        warn!("Failed to move agent into place {:?}: {e}", path);
        return None;
    }
    Some(path)
}

/// Resolves the `org.lwjgl` version an instance depends on (`3.4.1`, `3.3.3`, …).
pub fn lwjgl_version(libraries: &[crate::version::Library]) -> Option<String> {
    for lib in libraries {
        let mut parts = lib.name.split(':');
        let group = parts.next().unwrap_or_default();
        if group != "org.lwjgl" {
            continue;
        }
        if let Some(version) = parts.nth(1) {
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}

/// True when the instance predates the LegacyFix cutoff (2013-06-25).
///
/// Uses the ISO `releaseTime` string: a plain lexicographic comparison on
/// `YYYY-MM-DD` is correct for the format Mojang publishes.
pub fn needs_legacy_fix(release_time: Option<&str>) -> bool {
    let Some(raw) = release_time else {
        return false;
    };
    // Guard against placeholder timestamps (1970/0001) that some packers emit.
    if raw.len() < 10 || !raw.starts_with(|c: char| c.is_ascii_digit()) {
        return false;
    }
    let date = &raw[..10];
    if date.starts_with("1970") || date.starts_with("0000") {
        return false;
    }
    date < LEGACY_FIX_CUTOFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::Library;

    fn lib(name: &str) -> Library {
        Library {
            name: name.to_string(),
            url: None,
            downloads: None,
            rules: Vec::new(),
            natives: Default::default(),
            extract: None,
        }
    }

    #[test]
    fn detects_lwjgl_version_from_libraries() {
        let libs = vec![
            lib("com.mojang:brigadier:1.0.18"),
            lib("org.lwjgl:lwjgl:3.4.1"),
            lib("org.lwjgl:lwjgl-glfw:3.4.1"),
        ];
        assert_eq!(lwjgl_version(&libs).as_deref(), Some("3.4.1"));
        assert_eq!(
            lwjgl_version(&[lib("org.lwjgl:lwjgl:3.3.3")]).as_deref(),
            Some("3.3.3")
        );
        assert_eq!(lwjgl_version(&[lib("com.mojang:brigadier:1.0.18")]), None);
    }

    #[test]
    fn legacy_fix_applies_only_to_pre_2013_06_25() {
        assert!(needs_legacy_fix(Some("2013-06-24T15:00:00+00:00")));
        assert!(needs_legacy_fix(Some("2011-11-18T00:00:00+00:00")));
        assert!(!needs_legacy_fix(Some("2013-06-25T00:00:00+00:00")));
        assert!(!needs_legacy_fix(Some("2026-03-01T00:00:00+00:00")));
        assert!(!needs_legacy_fix(None));
        assert!(!needs_legacy_fix(Some("1970-01-01T00:00:00+00:00")));
    }

    #[test]
    fn embedded_agents_are_real_jars() {
        for agent in [LaunchAgent::LwjglUnsafe, LaunchAgent::LegacyFix] {
            let bytes = agent.bytes();
            assert!(bytes.len() > 1000, "{:?} too small", agent.file_name());
            // ZIP local file header
            assert_eq!(
                &bytes[..4],
                b"PK\x03\x04",
                "{:?} not a jar",
                agent.file_name()
            );
        }
    }
}
