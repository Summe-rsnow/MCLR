//! Save directory listing, deletion, backup, export, and zip import.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use zip::ZipArchive;
use zip::write::{SimpleFileOptions, ZipWriter};

use super::SaveInfo;
use super::level_nbt::parse_save_info;
use crate::error::SavesError;

/// Lists all valid saves in the `saves_dir` directory.
pub fn list_saves(saves_dir: &Path) -> Result<Vec<SaveInfo>, SavesError> {
    if !saves_dir.exists() {
        return Ok(Vec::new());
    }
    let mut saves = Vec::new();
    for entry in fs::read_dir(saves_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && path.join("level.dat").exists()
            && let Ok(info) = parse_save_info(&path)
        {
            saves.push(info);
        }
    }
    saves.sort_by_key(|a| std::cmp::Reverse(a.last_played));
    Ok(saves)
}

/// Deletes a save directory via recycle bin with directory removal fallback.
pub fn delete_save(saves_dir: &Path, folder_name: &str) -> Result<(), SavesError> {
    if folder_name.contains("..") || folder_name.contains('/') || folder_name.contains('\\') {
        return Err(SavesError::InvalidPath("Invalid folder name".into()));
    }
    let target = saves_dir.join(folder_name);
    if !target.exists() {
        return Err(SavesError::SaveNotFound(folder_name.to_string()));
    }
    if let Err(trash_err) = trash::delete(&target) {
        tracing::warn!("trash::delete failed ({trash_err}), falling back to remove_dir_all");
        fs::remove_dir_all(&target)?;
    }
    Ok(())
}

/// Zips `<saves_dir>/<folder_name>` into `<backups_dir>/<folder_name>-backup.zip`.
pub fn backup_save(
    saves_dir: &Path,
    backups_dir: &Path,
    folder_name: &str,
) -> Result<PathBuf, SavesError> {
    if folder_name.contains("..") || folder_name.contains('/') || folder_name.contains('\\') {
        return Err(SavesError::InvalidPath("Invalid folder name".into()));
    }
    let src = saves_dir.join(folder_name);
    if !src.exists() {
        return Err(SavesError::SaveNotFound(folder_name.to_string()));
    }
    fs::create_dir_all(backups_dir)?;
    let backup_path = backups_dir.join(format!("{}-backup.zip", folder_name));
    zip_directory(&src, &backup_path, Some(folder_name))?;
    Ok(backup_path)
}

/// Exports a save folder as a zip archive at `dest_zip`.
pub fn export_save_zip(
    saves_dir: &Path,
    folder_name: &str,
    dest_zip: &Path,
) -> Result<(), SavesError> {
    if folder_name.contains("..") || folder_name.contains('/') || folder_name.contains('\\') {
        return Err(SavesError::InvalidPath("Invalid folder name".into()));
    }
    let src = saves_dir.join(folder_name);
    if !src.exists() {
        return Err(SavesError::SaveNotFound(folder_name.to_string()));
    }
    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent)?;
    }
    zip_directory(&src, dest_zip, Some(folder_name))?;
    Ok(())
}

pub(crate) fn zip_directory(
    src_dir: &Path,
    dest_zip: &Path,
    prefix: Option<&str>,
) -> Result<(), SavesError> {
    let file = fs::File::create(dest_zip)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    fn walk_dir<W: Write + std::io::Seek>(
        zip: &mut ZipWriter<W>,
        root: &Path,
        current: &Path,
        prefix: Option<&str>,
        options: SimpleFileOptions,
    ) -> Result<(), SavesError> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .map_err(|e| SavesError::InvalidPath(e.to_string()))?;
            let rel_str = relative.to_string_lossy().replace('\\', "/");
            let zip_path = match prefix {
                Some(p) => format!("{}/{}", p, rel_str),
                None => rel_str,
            };

            if path.is_dir() {
                zip.add_directory(&zip_path, options)?;
                walk_dir(zip, root, &path, prefix, options)?;
            } else {
                zip.start_file(&zip_path, options)?;
                let mut f = fs::File::open(&path)?;
                std::io::copy(&mut f, zip)?;
            }
        }
        Ok(())
    }

    walk_dir(&mut zip, src_dir, src_dir, prefix, options)?;
    zip.finish()?;
    Ok(())
}

/// Imports a save from a zip archive into `saves_dir`, supporting root or single-folder level.dat layout.
pub fn import_save_zip(saves_dir: &Path, zip_path: &Path) -> Result<SaveInfo, SavesError> {
    if !zip_path.exists() {
        return Err(SavesError::InvalidPath(format!(
            "Zip path not found: {}",
            zip_path.display()
        )));
    }
    let file = fs::File::open(zip_path)?;
    let mut zip = ZipArchive::new(file)?;

    // Detect layout: find level.dat
    let mut level_dat_entry = None;
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        let name = entry.name()?.replace('\\', "/");
        if name == "level.dat" || name.ends_with("/level.dat") {
            level_dat_entry = Some(name);
            break;
        }
    }

    let level_dat_name = level_dat_entry
        .ok_or_else(|| SavesError::InvalidPath("No level.dat found inside zip archive".into()))?;

    // Determine strip prefix and destination folder name
    let (strip_prefix, default_name) = if level_dat_name == "level.dat" {
        (
            "",
            zip_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("imported_world"),
        )
    } else {
        let prefix = level_dat_name.strip_suffix("level.dat").unwrap();
        let folder = prefix.trim_end_matches('/').split('/').next().unwrap();
        (prefix, folder)
    };

    // Ensure non-colliding destination folder
    fs::create_dir_all(saves_dir)?;
    let mut candidate_name = default_name.to_string();
    let mut counter = 1;
    while saves_dir.join(&candidate_name).exists() {
        candidate_name = format!("{}_{}", default_name, counter);
        counter += 1;
    }

    let dest_dir = saves_dir.join(&candidate_name);
    fs::create_dir_all(&dest_dir)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let raw_name = entry.name()?.replace('\\', "/");

        let rel_name = if strip_prefix.is_empty() {
            raw_name
        } else if let Some(stripped) = raw_name.strip_prefix(strip_prefix) {
            stripped.to_string()
        } else {
            continue;
        };

        if rel_name.is_empty() || rel_name.contains("..") {
            continue;
        }

        let out_path = dest_dir.join(rel_name);
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out_file = fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut out_file)?;
        }
    }

    parse_save_info(&dest_dir)
}
