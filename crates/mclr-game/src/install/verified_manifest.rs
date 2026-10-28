//! Per-instance "already verified" fingerprint used by the fast startup repair path.
//!
//! The fingerprint is written next to the instance (`<instance>/.mclr-verified.json`)
//! only after a full repair finished successfully, so it is never trusted for an
//! instance that was interrupted mid-repair. It keys on the resolved version JSON
//! (`mtime`, `len`) plus the asset index id, which are exactly the inputs that make
//! the expensive part of a repair (asset index parse + per-file stat pass) redundant.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::warn;

/// File name of the fingerprint, inside the instance directory.
pub const MANIFEST_FILE: &str = ".mclr-verified.json";

/// Fingerprint of a fully verified instance directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedManifest {
    pub version_json_mtime_ms: u64,
    pub version_json_len: u64,
    pub asset_index_id: String,
    pub lib_count: u32,
}

fn manifest_path(instance_dir: &Path) -> std::path::PathBuf {
    instance_dir.join(MANIFEST_FILE)
}

/// `(mtime_ms, len)` of `path`, or `None` when it cannot be read.
///
/// `mtime_ms` is `0` when the platform cannot report a modification time, which
/// simply makes the fingerprint mismatch on the next run (full repair, then a new
/// fingerprint) instead of trusting a stale one.
pub fn file_fingerprint(path: &Path) -> Option<(u64, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    let mtime_ms = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    Some((mtime_ms, metadata.len()))
}

/// Loads the fingerprint of an instance directory.
///
/// Returns `None` when the file is missing or cannot be parsed; a partial or corrupt
/// file must never be treated as "verified".
pub fn load(instance_dir: &Path) -> Option<VerifiedManifest> {
    let path = manifest_path(instance_dir);
    let data = std::fs::read(&path).ok()?;
    match serde_json::from_slice::<VerifiedManifest>(&data) {
        Ok(manifest) => Some(manifest),
        Err(e) => {
            warn!(
                "Ignoring unreadable verified manifest {}: {}",
                path.display(),
                e
            );
            None
        }
    }
}

/// Writes the fingerprint of an instance directory.
///
/// The JSON is written to a temporary file in the same directory and then renamed,
/// so a crash mid-write can never leave a truncated (and therefore misleading) file.
/// Failures are logged and ignored: the fingerprint is a pure optimisation and the
/// next run simply repairs in full again.
pub fn save(instance_dir: &Path, m: &VerifiedManifest) {
    let path = manifest_path(instance_dir);
    let tmp_path = instance_dir.join(format!("{MANIFEST_FILE}.tmp"));

    let json = match serde_json::to_vec(m) {
        Ok(json) => json,
        Err(e) => {
            warn!(
                "Failed to serialize verified manifest for {}: {}",
                instance_dir.display(),
                e
            );
            return;
        }
    };

    if let Err(e) = std::fs::write(&tmp_path, &json) {
        warn!(
            "Failed to write verified manifest {}: {}",
            tmp_path.display(),
            e
        );
        return;
    }

    if let Err(e) = std::fs::rename(&tmp_path, &path) {
        warn!(
            "Failed to commit verified manifest {}: {}",
            path.display(),
            e
        );
        let _ = std::fs::remove_file(&tmp_path);
    }
}
