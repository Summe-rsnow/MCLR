//! Instance-specific presets ledger (`mclr.presets.json`).
//!
//! The ledger is the only place that records the community project identity of installed files
//! that cannot be identified from disk (CurseForge resource packs, shaders, datapacks, worlds).
//! It is written atomically and is never authoritative for "is the file still installed":
//! callers must check that `rel_path` still exists.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::error::CompError;
use crate::model::{CompPlatform, CompType};
use crate::preset::model::LinkMode;

/// File name of the ledger inside a game directory.
pub const LEDGER_FILE_NAME: &str = "mclr.presets.json";
const LEDGER_VERSION: u32 = 1;

/// One resource applied by a preset to an instance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LedgerEntry {
    pub platform: CompPlatform,
    pub project_id: String,
    pub file_id: String,
    pub sha1: String,
    pub kind: CompType,
    /// `/`-separated path relative to the game directory (e.g. `mods/sodium.jar`).
    pub rel_path: String,
    pub preset_ids: Vec<String>,
    pub link_mode: LinkMode,
    pub applied_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct LedgerDocument {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    entries: Vec<LedgerEntry>,
}

/// Path of the ledger for `game_dir`.
pub fn ledger_path(game_dir: &Path) -> PathBuf {
    game_dir.join(LEDGER_FILE_NAME)
}

/// Loads the ledger for `game_dir`, dropping entries whose `rel_path` escapes the game directory.
///
/// A missing or corrupt ledger yields an empty list: preset bookkeeping must never break instance
/// loading.
pub fn load(game_dir: &Path) -> Vec<LedgerEntry> {
    let path = ledger_path(game_dir);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(_) => return Vec::new(),
    };
    let doc: LedgerDocument = match serde_json::from_slice(&bytes) {
        Ok(doc) => doc,
        Err(err) => {
            tracing::warn!(path = %path.display(), error = %err, "instance preset ledger is corrupt; ignoring it");
            return Vec::new();
        }
    };
    doc.entries
        .into_iter()
        .filter(|entry| is_contained(game_dir, &entry.rel_path))
        .collect()
}

/// Inserts or replaces the entry identified by `(platform, project_id, kind)`.
///
/// Any other entry pointing at the same `rel_path` is dropped, so a replaced file leaves exactly
/// one owner behind.
pub fn upsert(game_dir: &Path, entry: LedgerEntry) -> Result<(), CompError> {
    if !is_contained(game_dir, &entry.rel_path) {
        return Err(CompError::Preset("preset.pathInvalid"));
    }
    let mut entries = load(game_dir);
    entries.retain(|existing| {
        !(existing.platform == entry.platform
            && existing.project_id == entry.project_id
            && existing.kind == entry.kind)
            && existing.rel_path != entry.rel_path
    });
    entries.push(entry);
    write(game_dir, &entries)
}

/// Removes the entry pointing at `rel_path` (no-op when absent).
pub fn remove_by_rel_path(game_dir: &Path, rel_path: &str) -> Result<(), CompError> {
    let mut entries = load(game_dir);
    let before = entries.len();
    entries.retain(|entry| entry.rel_path != rel_path);
    if entries.len() == before {
        return Ok(());
    }
    write(game_dir, &entries)
}

/// True when `rel_path` resolves inside `game_dir`.
fn is_contained(game_dir: &Path, rel_path: &str) -> bool {
    !rel_path.trim().is_empty() && mclr_core::safe_path::safe_join(game_dir, rel_path).is_some()
}

/// Writes the ledger atomically (temp file + rename).
fn write(game_dir: &Path, entries: &[LedgerEntry]) -> Result<(), CompError> {
    fs::create_dir_all(game_dir)?;
    let path = ledger_path(game_dir);
    let tmp_path = path.with_extension("json.tmp");
    let doc = LedgerDocument {
        version: LEDGER_VERSION,
        entries: entries.to_vec(),
    };
    fs::write(&tmp_path, serde_json::to_vec_pretty(&doc)?)?;
    fs::rename(&tmp_path, &path)?;
    Ok(())
}

/// Milliseconds since the Unix epoch, saturating at `0` before 1970.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
