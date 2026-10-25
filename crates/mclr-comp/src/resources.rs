#![allow(clippy::collapsible_if)]
//! Unified resource management: scanning, metadata extraction, toggling, import/export.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use flate2::read::GzDecoder;
use image::imageops::FilterType;
use mclr_core::paths;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use specta::Type;
use zip::ZipArchive;

use crate::error::CompError;
use crate::model::CompFile;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    Mod,
    ResourcePack,
    Shader,
    Schematic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ResourceState {
    Ok,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResourceEntry {
    pub path: String,
    pub rel_path: String,
    pub file_name: String,
    pub is_dir: bool,
    pub size: u64,
    pub created_ms: Option<u64>,
    pub modified_ms: Option<u64>,
    pub enabled: bool,
    pub state: ResourceState,
    pub title: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub icon: Option<String>,
    pub tags: Vec<String>,
    pub mod_id: Option<String>,
    pub project_id: Option<String>,
    pub latest_file: Option<CompFile>,
    pub update_available: bool,
    pub dependents: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SchematicInfo {
    pub name: String,
    pub author: Option<String>,
    pub size: [i32; 3],
    pub total_volume: i64,
    pub total_blocks: i64,
    pub region_count: usize,
    pub created_ms: Option<i64>,
    pub modified_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OverwritePolicy {
    Ask,
    Overwrite,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImportResourcesResult {
    pub imported: Vec<String>,
    pub conflicts: Vec<String>,
}

fn file_md5(path: &Path) -> std::io::Result<[u8; 16]> {
    let data = fs::read(path)?;
    let mut hasher = Md5::new();
    hasher.update(&data);
    Ok(hasher.finalize().into())
}

fn to_ms(time: std::io::Result<SystemTime>) -> Option<u64> {
    time.ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as u64)
}

fn strip_formatting_codes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '§' {
            let _ = chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

fn flatten_text_component(val: &serde_json::Value, out: &mut String) {
    match val {
        serde_json::Value::String(s) => out.push_str(s),
        serde_json::Value::Array(arr) => {
            for item in arr {
                flatten_text_component(item, out);
            }
        }
        serde_json::Value::Object(obj) => {
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                out.push_str(text);
            }
            if let Some(extra) = obj.get("extra") {
                flatten_text_component(extra, out);
            }
        }
        _ => {}
    }
}

fn extract_pack_mcmeta_description(bytes: &[u8]) -> Option<String> {
    let val: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let pack = val.get("pack")?;
    let desc_val = pack.get("description")?;
    let mut text = String::new();
    flatten_text_component(desc_val, &mut text);
    let cleaned = strip_formatting_codes(text.trim());
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn cache_pack_png(raw_png: &[u8], key: &str) -> Option<String> {
    let mut hasher = Sha1::new();
    sha1::Digest::update(&mut hasher, key.as_bytes());
    let hash = hex::encode(sha1::Digest::finalize(hasher));

    let icons_dir = paths::cache_dir().join("packicons");
    let _ = fs::create_dir_all(&icons_dir);
    let dest_path = icons_dir.join(format!("{hash}.png"));

    if dest_path.exists() {
        return Some(dest_path.to_string_lossy().to_string());
    }

    if let Ok(img) = image::load_from_memory(raw_png) {
        let resized = if img.width() > 128 || img.height() > 128 {
            img.resize(128, 128, FilterType::Triangle)
        } else {
            img
        };
        if resized
            .save_with_format(&dest_path, image::ImageFormat::Png)
            .is_ok()
        {
            return Some(dest_path.to_string_lossy().to_string());
        }
    }
    None
}

/// Scans the specified resource folder for entries of the given kind.
pub fn scan_resources(
    game_dir: &Path,
    mc_version: Option<&str>,
    kind: ResourceKind,
    rel: Option<&str>,
) -> Result<Vec<ResourceEntry>, CompError> {
    let base_folder = match kind {
        ResourceKind::Mod => game_dir.join("mods"),
        ResourceKind::ResourcePack => game_dir.join("resourcepacks"),
        ResourceKind::Shader => game_dir.join("shaderpacks"),
        ResourceKind::Schematic => {
            let s = game_dir.join("schematics");
            if let Some(r) = rel {
                if !r.is_empty() { s.join(r) } else { s }
            } else {
                s
            }
        }
    };

    if !base_folder.exists() {
        if kind == ResourceKind::Schematic {
            return Err(CompError::FileNotFound("schematics".to_string()));
        }
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    scan_dir_entries(&base_folder, &base_folder, kind, &mut entries)?;

    // For legacy Forge mods, also scan mods/<mc_version>/ if present
    if kind == ResourceKind::Mod {
        if let Some(ver) = mc_version {
            let legacy_dir = base_folder.join(ver);
            if legacy_dir.is_dir() {
                scan_dir_entries(&legacy_dir, &base_folder, kind, &mut entries)?;
            }
        }
    }

    Ok(entries)
}

fn scan_dir_entries(
    dir: &Path,
    root_dir: &Path,
    kind: ResourceKind,
    out: &mut Vec<ResourceEntry>,
) -> Result<(), CompError> {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    for entry_res in read_dir {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name.starts_with('.') || file_name.contains("$RECYCLE.BIN") {
            continue;
        }

        let rel_path = path
            .strip_prefix(root_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = metadata.len();
        let created_ms = to_ms(metadata.created());
        let modified_ms = to_ms(metadata.modified());

        match kind {
            ResourceKind::Mod => {
                if file_type.is_dir() {
                    continue;
                }
                let lower = file_name.to_ascii_lowercase();
                let is_mod_ext = lower.ends_with(".jar")
                    || lower.ends_with(".zip")
                    || lower.ends_with(".litemod")
                    || lower.ends_with(".jar.disabled")
                    || lower.ends_with(".zip.disabled")
                    || lower.ends_with(".litemod.disabled")
                    || lower.ends_with(".jar.old")
                    || lower.ends_with(".zip.old")
                    || lower.ends_with(".litemod.old");

                if !is_mod_ext {
                    continue;
                }

                let enabled = !lower.ends_with(".disabled") && !lower.ends_with(".old");
                let base_name = if let Some(s) = file_name.strip_suffix(".disabled") {
                    s
                } else if let Some(s) = file_name.strip_suffix(".old") {
                    s
                } else {
                    &file_name
                };

                let stem = Path::new(base_name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                out.push(ResourceEntry {
                    path: path.to_string_lossy().to_string(),
                    rel_path,
                    file_name,
                    is_dir: false,
                    size,
                    created_ms,
                    modified_ms,
                    enabled,
                    state: ResourceState::Ok,
                    title: Some(stem),
                    description: None,
                    version: None,
                    icon: None,
                    tags: Vec::new(),
                    mod_id: None,
                    project_id: None,
                    latest_file: None,
                    update_available: false,
                    dependents: Vec::new(),
                });
            }
            ResourceKind::ResourcePack => {
                let lower = file_name.to_ascii_lowercase();
                let is_zip = lower.ends_with(".zip");
                let is_dir = file_type.is_dir();

                if !is_zip && !is_dir {
                    continue;
                }

                let (description, icon) = if is_zip {
                    let mut desc = None;
                    let mut icon_path = None;
                    if let Ok(file) = fs::File::open(&path) {
                        if let Ok(mut archive) = ZipArchive::new(file) {
                            if let Ok(mut entry) = archive.by_name("pack.mcmeta") {
                                let mut bytes = Vec::new();
                                if entry.read_to_end(&mut bytes).is_ok() {
                                    desc = extract_pack_mcmeta_description(&bytes);
                                }
                            }
                            if let Ok(mut entry) = archive.by_name("pack.png") {
                                let mut bytes = Vec::new();
                                if entry.read_to_end(&mut bytes).is_ok() {
                                    let key = format!(
                                        "{}:{}:{}",
                                        path.display(),
                                        size,
                                        modified_ms.unwrap_or(0)
                                    );
                                    icon_path = cache_pack_png(&bytes, &key);
                                }
                            }
                        }
                    }
                    (desc, icon_path)
                } else {
                    let mcmeta_path = path.join("pack.mcmeta");
                    let desc = if mcmeta_path.is_file() {
                        fs::read(&mcmeta_path)
                            .ok()
                            .and_then(|b| extract_pack_mcmeta_description(&b))
                    } else {
                        None
                    };

                    let png_path = path.join("pack.png");
                    let icon_path = if png_path.is_file() {
                        fs::read(&png_path).ok().and_then(|b| {
                            let key =
                                format!("{}:{}:{}", path.display(), size, modified_ms.unwrap_or(0));
                            cache_pack_png(&b, &key)
                        })
                    } else {
                        None
                    };
                    (desc, icon_path)
                };

                let stem = Path::new(&file_name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                out.push(ResourceEntry {
                    path: path.to_string_lossy().to_string(),
                    rel_path,
                    file_name,
                    is_dir,
                    size,
                    created_ms,
                    modified_ms,
                    enabled: true,
                    state: ResourceState::Ok,
                    title: Some(stem),
                    description,
                    version: None,
                    icon,
                    tags: Vec::new(),
                    mod_id: None,
                    project_id: None,
                    latest_file: None,
                    update_available: false,
                    dependents: Vec::new(),
                });
            }
            ResourceKind::Shader => {
                let lower = file_name.to_ascii_lowercase();
                let is_zip = lower.ends_with(".zip");
                let is_dir = file_type.is_dir();

                if !is_zip && !is_dir {
                    continue;
                }

                let stem = Path::new(&file_name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                out.push(ResourceEntry {
                    path: path.to_string_lossy().to_string(),
                    rel_path,
                    file_name,
                    is_dir,
                    size,
                    created_ms,
                    modified_ms,
                    enabled: true,
                    state: ResourceState::Ok,
                    title: Some(stem),
                    description: None,
                    version: None,
                    icon: None,
                    tags: Vec::new(),
                    mod_id: None,
                    project_id: None,
                    latest_file: None,
                    update_available: false,
                    dependents: Vec::new(),
                });
            }
            ResourceKind::Schematic => {
                let is_dir = file_type.is_dir();
                if is_dir {
                    out.push(ResourceEntry {
                        path: path.to_string_lossy().to_string(),
                        rel_path,
                        file_name: file_name.clone(),
                        is_dir: true,
                        size,
                        created_ms,
                        modified_ms,
                        enabled: true,
                        state: ResourceState::Ok,
                        title: Some(file_name),
                        description: None,
                        version: None,
                        icon: None,
                        tags: vec!["folder".to_string()],
                        mod_id: None,
                        project_id: None,
                        latest_file: None,
                        update_available: false,
                        dependents: Vec::new(),
                    });
                    continue;
                }

                let lower = file_name.to_ascii_lowercase();
                let ext = if lower.ends_with(".litematic") {
                    "litematic"
                } else if lower.ends_with(".nbt") {
                    "nbt"
                } else if lower.ends_with(".schematic") {
                    "schematic"
                } else if lower.ends_with(".schem") {
                    "schem"
                } else {
                    continue;
                };

                let stem = Path::new(&file_name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                out.push(ResourceEntry {
                    path: path.to_string_lossy().to_string(),
                    rel_path,
                    file_name,
                    is_dir: false,
                    size,
                    created_ms,
                    modified_ms,
                    enabled: true,
                    state: ResourceState::Ok,
                    title: Some(stem),
                    description: None,
                    version: None,
                    icon: None,
                    tags: vec![ext.to_string()],
                    mod_id: None,
                    project_id: None,
                    latest_file: None,
                    update_available: false,
                    dependents: Vec::new(),
                });
            }
        }
    }

    Ok(())
}

fn get_i32(val: &fastnbt::Value) -> Option<i32> {
    match val {
        fastnbt::Value::Int(n) => Some(*n),
        fastnbt::Value::Short(n) => Some(*n as i32),
        fastnbt::Value::Byte(n) => Some(*n as i32),
        fastnbt::Value::Long(n) => Some(*n as i32),
        _ => None,
    }
}

fn get_i64(val: &fastnbt::Value) -> Option<i64> {
    match val {
        fastnbt::Value::Long(n) => Some(*n),
        fastnbt::Value::Int(n) => Some(*n as i64),
        fastnbt::Value::Short(n) => Some(*n as i64),
        fastnbt::Value::Byte(n) => Some(*n as i64),
        _ => None,
    }
}

fn get_string(val: &fastnbt::Value) -> Option<String> {
    match val {
        fastnbt::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Parses gzip NBT schematic metadata (.litematic, .nbt, .schematic, .schem).
pub fn schematic_info(path: &Path) -> Result<SchematicInfo, CompError> {
    let bytes = fs::read(path)?;
    let raw = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        decompressed
    } else {
        bytes
    };

    let nbt: fastnbt::Value = fastnbt::from_bytes(&raw)
        .map_err(|e| CompError::InvalidParams(format!("NBT parse error: {e}")))?;

    let default_name = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let meta = match &nbt {
        fastnbt::Value::Compound(root) => {
            if let Some(fastnbt::Value::Compound(meta_map)) = root.get("Metadata") {
                // .litematic format
                let name = meta_map
                    .get("Name")
                    .and_then(get_string)
                    .unwrap_or(default_name);
                let author = meta_map.get("Author").and_then(get_string);
                let mut size = [0, 0, 0];
                if let Some(fastnbt::Value::Compound(enclosing)) = meta_map.get("EnclosingSize") {
                    size[0] = enclosing.get("x").and_then(get_i32).unwrap_or(0).abs();
                    size[1] = enclosing.get("y").and_then(get_i32).unwrap_or(0).abs();
                    size[2] = enclosing.get("z").and_then(get_i32).unwrap_or(0).abs();
                }
                let total_volume = meta_map
                    .get("TotalVolume")
                    .and_then(get_i64)
                    .unwrap_or_else(|| size[0] as i64 * size[1] as i64 * size[2] as i64);
                let total_blocks = meta_map
                    .get("TotalBlocks")
                    .and_then(get_i64)
                    .unwrap_or(total_volume);
                let region_count =
                    meta_map.get("RegionCount").and_then(get_i64).unwrap_or(1) as usize;
                let created_ms = meta_map.get("TimeCreated").and_then(get_i64);
                let modified_ms = meta_map.get("TimeModified").and_then(get_i64);

                SchematicInfo {
                    name,
                    author,
                    size,
                    total_volume,
                    total_blocks,
                    region_count,
                    created_ms,
                    modified_ms,
                }
            } else if let Some(fastnbt::Value::List(size_list)) = root.get("size") {
                // Vanilla structure (.nbt)
                let mut size = [0, 0, 0];
                for (i, val) in size_list.iter().take(3).enumerate() {
                    size[i] = get_i32(val).unwrap_or(0).abs();
                }
                let author = root.get("author").and_then(get_string);
                let total_volume = size[0] as i64 * size[1] as i64 * size[2] as i64;
                let total_blocks = if let Some(fastnbt::Value::List(blocks)) = root.get("blocks") {
                    blocks.len() as i64
                } else {
                    total_volume
                };

                SchematicInfo {
                    name: default_name,
                    author,
                    size,
                    total_volume,
                    total_blocks,
                    region_count: 1,
                    created_ms: None,
                    modified_ms: None,
                }
            } else if root.contains_key("Width")
                && root.contains_key("Height")
                && root.contains_key("Length")
            {
                // .schematic or .schem
                let width = root.get("Width").and_then(get_i32).unwrap_or(0).abs();
                let height = root.get("Height").and_then(get_i32).unwrap_or(0).abs();
                let length = root.get("Length").and_then(get_i32).unwrap_or(0).abs();
                let size = [width, height, length];
                let total_volume = width as i64 * height as i64 * length as i64;

                let (name, author) =
                    if let Some(fastnbt::Value::Compound(meta_map)) = root.get("Metadata") {
                        let n = meta_map
                            .get("Name")
                            .and_then(get_string)
                            .unwrap_or_else(|| default_name.clone());
                        let a = meta_map.get("Author").and_then(get_string);
                        (n, a)
                    } else {
                        (default_name, None)
                    };

                SchematicInfo {
                    name,
                    author,
                    size,
                    total_volume,
                    total_blocks: total_volume,
                    region_count: 1,
                    created_ms: None,
                    modified_ms: None,
                }
            } else {
                SchematicInfo {
                    name: default_name,
                    author: None,
                    size: [0, 0, 0],
                    total_volume: 0,
                    total_blocks: 0,
                    region_count: 1,
                    created_ms: None,
                    modified_ms: None,
                }
            }
        }
        _ => SchematicInfo {
            name: default_name,
            author: None,
            size: [0, 0, 0],
            total_volume: 0,
            total_blocks: 0,
            region_count: 1,
            created_ms: None,
            modified_ms: None,
        },
    };

    Ok(meta)
}

/// Toggles a resource file between enabled and disabled (.disabled / .old).
pub fn toggle_resource(path: &Path) -> Result<(), CompError> {
    if !path.exists() {
        return Err(CompError::FileNotFound(path.display().to_string()));
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| CompError::FileNotFound(path.display().to_string()))?
        .to_string_lossy();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));

    if let Some(base) = file_name.strip_suffix(".disabled") {
        let target = parent.join(base);
        if target.exists() {
            let hash_src = file_md5(path)?;
            let hash_dst = file_md5(&target)?;
            if hash_src != hash_dst {
                return Err(CompError::ToggleConflict);
            }
            let _ = fs::remove_file(path);
        } else {
            fs::rename(path, &target)?;
        }
    } else if let Some(base) = file_name.strip_suffix(".old") {
        let target = parent.join(base);
        if target.exists() {
            let hash_src = file_md5(path)?;
            let hash_dst = file_md5(&target)?;
            if hash_src != hash_dst {
                return Err(CompError::ToggleConflict);
            }
            let _ = fs::remove_file(path);
        } else {
            fs::rename(path, &target)?;
        }
    } else {
        // Currently enabled: check if .old exists, otherwise use .disabled
        let old_target = parent.join(format!("{file_name}.old"));
        let disabled_target = parent.join(format!("{file_name}.disabled"));

        let target = if old_target.exists() {
            old_target
        } else {
            disabled_target
        };

        if target.exists() {
            let hash_src = file_md5(path)?;
            let hash_dst = file_md5(&target)?;
            if hash_src != hash_dst {
                return Err(CompError::ToggleConflict);
            }
            let _ = fs::remove_file(path);
        } else {
            fs::rename(path, &target)?;
        }
    }

    Ok(())
}

/// Deletes resources permanently or into Recycle Bin, removing disabled/old counterparts.
pub fn delete_resources(paths: &[PathBuf], permanent: bool) -> Result<(), CompError> {
    for path in paths {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = match path.file_name() {
            Some(f) => f.to_string_lossy(),
            None => continue,
        };

        let base_name = if let Some(b) = file_name.strip_suffix(".disabled") {
            b
        } else if let Some(b) = file_name.strip_suffix(".old") {
            b
        } else {
            &file_name
        };

        let candidates = [
            parent.join(base_name),
            parent.join(format!("{base_name}.disabled")),
            parent.join(format!("{base_name}.old")),
        ];

        for c in candidates {
            if !c.exists() {
                continue;
            }
            if permanent {
                if c.is_dir() {
                    let _ = fs::remove_dir_all(&c);
                } else {
                    let _ = fs::remove_file(&c);
                }
            } else if let Err(_e) = trash::delete(&c) {
                if c.is_dir() {
                    let _ = fs::remove_dir_all(&c);
                } else {
                    let _ = fs::remove_file(&c);
                }
            }
        }
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Imports resource files into the game directory.
pub fn import_resources(
    kind: ResourceKind,
    game_dir: &Path,
    modable: bool,
    sources: &[PathBuf],
    policy: OverwritePolicy,
) -> Result<ImportResourcesResult, CompError> {
    if kind == ResourceKind::Mod && !modable {
        return Err(CompError::NotModable);
    }

    let target_dir = match kind {
        ResourceKind::Mod => game_dir.join("mods"),
        ResourceKind::ResourcePack => game_dir.join("resourcepacks"),
        ResourceKind::Shader => game_dir.join("shaderpacks"),
        ResourceKind::Schematic => game_dir.join("schematics"),
    };

    fs::create_dir_all(&target_dir)?;

    let mut imported = Vec::new();
    let mut conflicts = Vec::new();

    for src in sources {
        let src_str = src.to_string_lossy();
        if src_str.contains("$RECYCLE.BIN") || !src.exists() {
            continue;
        }

        let file_name = match src.file_name() {
            Some(f) => f.to_string_lossy(),
            None => continue,
        };

        let dest_name = if kind == ResourceKind::Mod {
            if let Some(b) = file_name.strip_suffix(".disabled") {
                b.to_string()
            } else if let Some(b) = file_name.strip_suffix(".old") {
                b.to_string()
            } else {
                file_name.to_string()
            }
        } else {
            file_name.to_string()
        };

        let dest_path = target_dir.join(&dest_name);
        if dest_path.exists() {
            match policy {
                OverwritePolicy::Ask => {
                    conflicts.push(dest_name);
                    continue;
                }
                OverwritePolicy::Skip => continue,
                OverwritePolicy::Overwrite => {}
            }
        }

        if src.is_dir() {
            copy_dir_all(src, &dest_path)?;
        } else {
            fs::copy(src, &dest_path)?;
        }
        imported.push(dest_name);
    }

    Ok(ImportResourcesResult {
        imported,
        conflicts,
    })
}

fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Exports a list of resource entries to TXT or CSV.
pub fn export_resource_list(entries: &[ResourceEntry], format: &str) -> String {
    if format.eq_ignore_ascii_case("csv") {
        let mut out = String::from("name,file,version,loader,mc,id,source,enabled,size\n");
        for e in entries {
            let name = escape_csv_field(e.title.as_deref().unwrap_or(&e.file_name));
            let file = escape_csv_field(&e.file_name);
            let version = escape_csv_field(e.version.as_deref().unwrap_or(""));
            let loader = escape_csv_field(&e.tags.join(";"));
            let mc = "";
            let id = escape_csv_field(e.mod_id.as_deref().unwrap_or(""));
            let source = if e.project_id.is_some() {
                "remote"
            } else {
                "local"
            };
            let enabled = if e.enabled { "true" } else { "false" };
            let size = e.size.to_string();

            out.push_str(&format!(
                "{name},{file},{version},{loader},{mc},{id},{source},{enabled},{size}\n"
            ));
        }
        out
    } else {
        let mut out = String::new();
        out.push_str("Minecraft Resources Export\n");
        out.push_str("=========================\n\n");
        for e in entries {
            let title = e.title.as_deref().unwrap_or(&e.file_name);
            let ver = e
                .version
                .as_deref()
                .map(|v| format!(" v{v}"))
                .unwrap_or_default();
            let status = if e.enabled { "[Enabled]" } else { "[Disabled]" };
            out.push_str(&format!("- {title}{ver} ({}) {status}\n", e.file_name));
            if !e.tags.is_empty() {
                out.push_str(&format!("  Tags: {}\n", e.tags.join(", ")));
            }
            if let Some(desc) = &e.description {
                out.push_str(&format!("  Description: {desc}\n"));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_toggle_and_delete_resource() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_mod.jar");
        fs::write(&file_path, b"dummy mod content").unwrap();

        // Toggle to disabled
        toggle_resource(&file_path).unwrap();
        let disabled_path = dir.path().join("test_mod.jar.disabled");
        assert!(!file_path.exists());
        assert!(disabled_path.exists());

        // Toggle back to enabled
        toggle_resource(&disabled_path).unwrap();
        assert!(file_path.exists());
        assert!(!disabled_path.exists());

        // Delete permanently
        delete_resources(std::slice::from_ref(&file_path), true).unwrap();
        assert!(!file_path.exists());
    }

    #[test]
    fn test_toggle_conflict() {
        let dir = tempdir().unwrap();
        let enabled = dir.path().join("conflict.jar");
        let disabled = dir.path().join("conflict.jar.disabled");
        fs::write(&enabled, b"version 1").unwrap();
        fs::write(&disabled, b"version 2").unwrap();

        let res = toggle_resource(&enabled);
        assert!(matches!(res, Err(CompError::ToggleConflict)));
    }

    #[test]
    fn test_export_csv() {
        let entries = vec![ResourceEntry {
            path: "/path/to/mod.jar".to_string(),
            rel_path: "mod.jar".to_string(),
            file_name: "mod.jar".to_string(),
            is_dir: false,
            size: 1024,
            created_ms: Some(1000),
            modified_ms: Some(2000),
            enabled: true,
            state: ResourceState::Ok,
            title: Some("My Mod".to_string()),
            description: Some("Test mod description".to_string()),
            version: Some("1.0.0".to_string()),
            icon: None,
            tags: vec!["Fabric".to_string()],
            mod_id: Some("mymod".to_string()),
            project_id: Some("proj123".to_string()),
            latest_file: None,
            update_available: false,
            dependents: Vec::new(),
        }];

        let csv = export_resource_list(&entries, "csv");
        assert!(csv.contains("name,file,version,loader,mc,id,source,enabled,size"));
        assert!(csv.contains("My Mod,mod.jar,1.0.0,Fabric,,mymod,remote,true,1024"));
    }
}
