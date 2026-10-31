//! Screenshot enumeration, pagination, thumbnail generation, and clipboard integration.

use sha1::Digest;
use std::fs;
use std::path::Path;

use super::{ScreenshotMeta, ScreenshotsPagedResult, format_system_time};
use crate::error::SavesError;

const SUPPORTED_SCREENSHOT_EXTS: &[&str] = &["png", "jpg", "jpeg", "bmp", "webp", "tiff"];

/// Lists screenshots with pagination, supporting common image formats and sorting newest first.
///
/// Every call rescans the directory. A directory-mtime cache used to live here, but NTFS only
/// bumps a directory's timestamp lazily: writing `d.png` into a folder leaves its mtime
/// unchanged often enough that a freshly saved screenshot stayed invisible until something else
/// touched the folder. Reading `read_dir` plus one `metadata` per file costs a few milliseconds
/// for a realistic screenshot folder, which is cheap next to handing the user a stale list.
pub fn list_screenshots_paged(
    screenshots_dir: &Path,
    offset: usize,
    limit: usize,
) -> Result<ScreenshotsPagedResult, SavesError> {
    if !screenshots_dir.exists() {
        let _ = fs::create_dir_all(screenshots_dir);
        return Ok(ScreenshotsPagedResult {
            items: Vec::new(),
            total: 0,
        });
    }

    let mut screenshots = Vec::new();
    for entry in fs::read_dir(screenshots_dir)? {
        let entry = entry?;
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };
        if !file_type.is_file() && !file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !SUPPORTED_SCREENSHOT_EXTS.contains(&ext.as_str()) {
            continue;
        }

        // `entry.metadata()` reuses the data the directory enumeration already returned;
        // `fs::metadata` on the joined path costs a second full stat (measured ~160x slower
        // over 1000 files on Windows). It never traverses a link, so links fall back to the
        // path stat to keep symlinked screenshots visible.
        let meta = match entry.metadata() {
            Ok(m) if m.is_file() => m,
            Ok(_) => match fs::metadata(&path) {
                Ok(m) if m.is_file() => m,
                _ => continue,
            },
            Err(_) => continue,
        };
        let size = meta.len();
        if size == 0 {
            continue;
        }

        let created_time = meta
            .created()
            .unwrap_or_else(|_| meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH));
        let created_ms = created_time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let modified_at =
            format_system_time(meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH));

        screenshots.push(ScreenshotMeta {
            file_name,
            path: path.to_string_lossy().to_string(),
            size,
            created_ms,
            modified_at,
        });
    }

    screenshots.sort_by_key(|a| std::cmp::Reverse(a.created_ms));

    let total = screenshots.len();
    let items = screenshots
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    Ok(ScreenshotsPagedResult { items, total })
}

/// Deletes a screenshot file by direct path.
pub fn delete_screenshot_path(path: &Path) -> Result<(), SavesError> {
    if !path.exists() {
        return Err(SavesError::InvalidPath(format!(
            "Screenshot not found: {}",
            path.display()
        )));
    }
    if let Err(trash_err) = trash::delete(path) {
        tracing::warn!("trash::delete failed ({trash_err}), falling back to remove_file");
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Deletes a screenshot from `screenshots_dir`.
pub fn delete_screenshot(screenshots_dir: &Path, file_name: &str) -> Result<(), SavesError> {
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(SavesError::InvalidPath("Invalid file name".into()));
    }
    let target = screenshots_dir.join(file_name);
    delete_screenshot_path(&target)
}

/// Generates and caches a 400px-wide JPEG thumbnail of the screenshot.
pub fn generate_screenshot_thumb(image_path: &str) -> Result<String, SavesError> {
    let path = Path::new(image_path);
    if !path.exists() {
        return Err(SavesError::InvalidPath(format!(
            "File not found: {}",
            image_path
        )));
    }
    let meta = fs::metadata(path)?;
    let mtime_ms = meta
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let hash_input = format!("{}:{}", image_path, mtime_ms);
    let mut hasher = sha1::Sha1::new();
    sha1::Digest::update(&mut hasher, hash_input.as_bytes());
    let hash_hex = hex::encode(sha1::Digest::finalize(hasher));

    let thumbs_dir = mclr_core::paths::cache_dir().join("thumbs");
    if !thumbs_dir.exists() {
        let _ = fs::create_dir_all(&thumbs_dir);
    }
    let thumb_file = thumbs_dir.join(format!("{}.jpg", hash_hex));
    if thumb_file.exists() {
        return Ok(thumb_file.to_string_lossy().to_string());
    }

    let img =
        image::open(path).map_err(|e| SavesError::Io(std::io::Error::other(e.to_string())))?;
    let orig_w = img.width();
    let orig_h = img.height();
    let target_w = 400;
    let target_h = if orig_w > 0 {
        ((orig_h as f64) * (target_w as f64 / orig_w as f64))
            .round()
            .max(1.0) as u32
    } else {
        400
    };
    let resized = img.resize(target_w, target_h, image::imageops::FilterType::Triangle);
    resized
        .save(&thumb_file)
        .map_err(|e| SavesError::Io(std::io::Error::other(e.to_string())))?;

    Ok(thumb_file.to_string_lossy().to_string())
}

/// Decodes image to RGBA and copies to the system clipboard via `arboard::Clipboard` (with up to 5 retries 100ms apart).
pub fn copy_image_to_clipboard(image_path: &str) -> Result<(), SavesError> {
    let path = Path::new(image_path);
    if !path.exists() {
        return Err(SavesError::InvalidPath(format!(
            "File not found: {}",
            image_path
        )));
    }
    let img = image::open(path)
        .map_err(|e| SavesError::Io(std::io::Error::other(e.to_string())))?
        .to_rgba8();
    let width = img.width() as usize;
    let height = img.height() as usize;
    let bytes = img.into_raw();
    let img_data = arboard::ImageData {
        width,
        height,
        bytes: std::borrow::Cow::Owned(bytes),
    };

    let mut last_err = None;
    for attempt in 0..5 {
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_image(img_data.clone())) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }
    Err(SavesError::Io(std::io::Error::other(format!(
        "Failed to copy image to clipboard after 5 attempts: {:?}",
        last_err
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_paged_listing_is_stable_across_calls() {
        let dir = tempdir().unwrap();
        let ss_dir = dir.path().join("screenshots");
        fs::create_dir_all(&ss_dir).unwrap();
        for name in ["a.png", "b.png", "c.png"] {
            fs::write(ss_dir.join(name), b"png-bytes").unwrap();
        }

        let first_page = list_screenshots_paged(&ss_dir, 0, 2).unwrap();
        let second_page = list_screenshots_paged(&ss_dir, 2, 2).unwrap();
        assert_eq!(first_page.total, 3);
        assert_eq!(second_page.total, 3);
        assert_eq!(first_page.items.len(), 2);
        assert_eq!(second_page.items.len(), 1);

        // Repeating a query must return an identical page (the listing is not allowed to drift).
        assert_eq!(list_screenshots_paged(&ss_dir, 0, 2).unwrap(), first_page);
        assert_eq!(list_screenshots_paged(&ss_dir, 2, 2).unwrap(), second_page);

        // A new file changes the directory mtime, forcing a fresh scan.
        fs::write(ss_dir.join("d.png"), b"png-bytes").unwrap();
        let after = list_screenshots_paged(&ss_dir, 0, 10).unwrap();
        assert_eq!(after.total, 4);
        assert!(
            after.items.iter().any(|s| s.file_name == "d.png"),
            "newly added screenshot must show up: {:?}",
            after.items
        );
    }
}
