//! Saves, datapacks, screenshots, and instance files management.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use fastnbt::Value;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use specta::Type;
use zip::ZipArchive;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::error::SavesError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveSpawn {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SaveInfo {
    pub folder_name: String,
    pub path: String,
    pub name: String,
    pub seed: String,
    pub spawn: SaveSpawn,
    pub game_mode: i32,
    pub difficulty: i32,
    pub play_time: i64,
    pub last_played: i64,
    pub data_version: Option<i32>,
    pub icon_data_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DatapackInfo {
    pub id: String,
    pub file_name: String,
    pub description: String,
    pub pack_format: i32,
    pub enabled: bool,
    pub built_in: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotInfo {
    pub file_name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: String,
    pub data_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceFileItem {
    pub file_name: String,
    pub path: String,
    pub enabled: bool,
    pub size: u64,
    pub modified_at: String,
}

pub(crate) fn format_system_time(st: std::time::SystemTime) -> String {
    let dur = match st.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d,
        Err(_) => return "1970-01-01T00:00:00Z".to_string(),
    };
    let total_secs = dur.as_secs();
    let sec = total_secs % 60;
    let min = (total_secs / 60) % 60;
    let hour = (total_secs / 3600) % 24;
    let mut days = (total_secs / 86400) as i64;

    let mut year = 1970;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if leap { 366 } else { 365 };
        if days >= days_in_year {
            days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let days_in_months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1;
    for &dim in &days_in_months {
        if days >= dim {
            days -= dim;
            month += 1;
        } else {
            break;
        }
    }
    let day = days + 1;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

fn as_i32(val: &Value) -> Option<i32> {
    match val {
        Value::Int(i) => Some(*i),
        Value::Byte(b) => Some(*b as i32),
        Value::Short(s) => Some(*s as i32),
        Value::Long(l) => Some(*l as i32),
        _ => None,
    }
}

fn as_i64(val: &Value) -> Option<i64> {
    match val {
        Value::Long(l) => Some(*l),
        Value::Int(i) => Some(*i as i64),
        Value::Short(s) => Some(*s as i64),
        Value::Byte(b) => Some(*b as i64),
        _ => None,
    }
}

fn as_str(val: &Value) -> Option<&str> {
    match val {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    }
}

fn as_seed_string(val: &Value) -> Option<String> {
    match val {
        Value::Long(l) => Some(l.to_string()),
        Value::Int(i) => Some(i.to_string()),
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn read_legacy_spawn(data: &HashMap<String, Value>) -> SaveSpawn {
    let x = data.get("SpawnX").and_then(as_i32).unwrap_or(0);
    let y = data.get("SpawnY").and_then(as_i32).unwrap_or(0);
    let z = data.get("SpawnZ").and_then(as_i32).unwrap_or(0);
    SaveSpawn { x, y, z }
}

/// Reads `level.dat` from `path`, auto-detecting GZip compression.
pub fn read_level_dat(path: &Path) -> Result<fastnbt::Value, SavesError> {
    let bytes = fs::read(path)?;
    let is_gzip = bytes.starts_with(&[0x1f, 0x8b]);
    let raw = if is_gzip {
        let mut decoder = GzDecoder::new(&bytes[..]);
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        buf
    } else {
        bytes
    };
    let value: fastnbt::Value = fastnbt::from_bytes(&raw)?;
    Ok(value)
}

/// Writes `level.dat` to `path`, preserving existing GZip compression (default GZip).
pub fn write_level_dat(path: &Path, value: &fastnbt::Value) -> Result<(), SavesError> {
    let is_gzip = if path.exists() {
        match fs::read(path) {
            Ok(b) => b.starts_with(&[0x1f, 0x8b]),
            Err(_) => true,
        }
    } else {
        true
    };
    write_level_dat_compressed(path, value, is_gzip)
}

/// Writes `level.dat` to `path` with explicit GZip compression setting.
pub fn write_level_dat_compressed(
    path: &Path,
    value: &fastnbt::Value,
    gzip: bool,
) -> Result<(), SavesError> {
    let raw = fastnbt::to_bytes(value)?;
    let data = if gzip {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw)?;
        encoder.finish()?
    } else {
        raw
    };

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let tmp_path = path.with_extension(format!("tmp.{}", nanos));
    fs::write(&tmp_path, &data)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Extracts `SaveInfo` from a save directory by reading `level.dat` and optional `icon.png`.
pub fn parse_save_info(save_dir: &Path) -> Result<SaveInfo, SavesError> {
    let level_dat = save_dir.join("level.dat");
    if !level_dat.exists() {
        return Err(SavesError::SaveNotFound(
            save_dir.to_string_lossy().to_string(),
        ));
    }
    let nbt = read_level_dat(&level_dat)?;
    parse_save_info_from_nbt(save_dir, &nbt)
}

/// Helper to parse `SaveInfo` given an already loaded root `fastnbt::Value`.
pub fn parse_save_info_from_nbt(
    save_dir: &Path,
    root: &fastnbt::Value,
) -> Result<SaveInfo, SavesError> {
    let folder_name = save_dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let path = save_dir.to_string_lossy().to_string();

    let data = match root {
        Value::Compound(map) => {
            if let Some(Value::Compound(d)) = map.get("Data").or_else(|| map.get("data")) {
                d
            } else {
                map
            }
        }
        _ => {
            return Err(SavesError::InvalidPath(
                "NBT root must be a compound".into(),
            ));
        }
    };

    // 1. DataVersion boundary:
    // Boundary 1: < 100 (or absent): None
    // Boundary 2: 100 <= dv < 1443
    // Boundary 3: 1443 <= dv < 2536
    // Boundary 4: 2536 <= dv < 4774
    // Boundary 5: >= 4774
    let raw_dv = data.get("DataVersion").and_then(as_i32);
    let data_version = raw_dv.filter(|&v| v >= 100);
    let dv_val = raw_dv.unwrap_or(0);

    // 2. LevelName
    let name = data
        .get("LevelName")
        .and_then(as_str)
        .unwrap_or(&folder_name)
        .to_string();

    // 3. Seed:
    // >= 2536: Data.WorldGenSettings.seed
    // >= 4774: also checks <save_dir>/data/minecraft/world_gen_settings.dat
    // < 2536: Data.RandomSeed
    let seed = if dv_val >= 2536 {
        let mut s = None;
        if let Some(Value::Compound(wgs)) = data.get("WorldGenSettings") {
            s = wgs.get("seed").and_then(as_seed_string);
        }
        if s.is_none() && dv_val >= 4774 {
            let external_wgs = save_dir
                .join("data")
                .join("minecraft")
                .join("world_gen_settings.dat");
            if let Ok(Value::Compound(ext_map)) = read_level_dat(&external_wgs) {
                let ext_data = ext_map.get("data").or_else(|| ext_map.get("Data"));
                if let Some(Value::Compound(d)) = ext_data {
                    s = d.get("seed").and_then(as_seed_string);
                }
            }
        }
        s.or_else(|| data.get("RandomSeed").and_then(as_seed_string))
            .unwrap_or_else(|| "0".to_string())
    } else {
        data.get("RandomSeed")
            .and_then(as_seed_string)
            .or_else(|| {
                if let Some(Value::Compound(wgs)) = data.get("WorldGenSettings") {
                    wgs.get("seed").and_then(as_seed_string)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "0".to_string())
    };

    // 4. Spawn:
    // >= 4774: Data.spawn.pos [x, y, z] (IntArray or List)
    // fallback: Data.SpawnX, Data.SpawnY, Data.SpawnZ
    let spawn = if dv_val >= 4774 || data.contains_key("spawn") {
        let spawn_pos = if let Some(Value::Compound(s)) = data.get("spawn") {
            match s.get("pos") {
                Some(Value::IntArray(arr)) if arr.len() >= 3 => Some(SaveSpawn {
                    x: arr[0],
                    y: arr[1],
                    z: arr[2],
                }),
                Some(Value::List(list)) if list.len() >= 3 => {
                    let x = list.first().and_then(as_i32).unwrap_or(0);
                    let y = list.get(1).and_then(as_i32).unwrap_or(0);
                    let z = list.get(2).and_then(as_i32).unwrap_or(0);
                    Some(SaveSpawn { x, y, z })
                }
                _ => None,
            }
        } else {
            None
        };
        spawn_pos.unwrap_or_else(|| read_legacy_spawn(data))
    } else {
        read_legacy_spawn(data)
    };

    // 5. Difficulty:
    // >= 4774: difficulty_settings.difficulty ("peaceful"|"easy"|"normal"|"hard" or int)
    // fallback: Data.Difficulty (Byte/Int)
    let difficulty = if dv_val >= 4774 || data.contains_key("difficulty_settings") {
        let diff_val = if let Some(Value::Compound(ds)) = data.get("difficulty_settings") {
            match ds.get("difficulty") {
                Some(Value::String(s)) => match s.to_ascii_lowercase().as_str() {
                    "peaceful" => Some(0),
                    "easy" => Some(1),
                    "normal" => Some(2),
                    "hard" => Some(3),
                    _ => None,
                },
                Some(val) => as_i32(val),
                None => None,
            }
        } else {
            None
        };
        diff_val
            .or_else(|| data.get("Difficulty").and_then(as_i32))
            .unwrap_or(2)
    } else {
        data.get("Difficulty").and_then(as_i32).unwrap_or(2)
    };

    // 6. GameMode:
    let game_mode = data.get("GameType").and_then(as_i32).unwrap_or(0);

    // 7. PlayTime: Time or DayTime
    let play_time = data
        .get("Time")
        .or_else(|| data.get("DayTime"))
        .and_then(as_i64)
        .unwrap_or(0);

    // 8. LastPlayed:
    let last_played = data.get("LastPlayed").and_then(as_i64).unwrap_or(0);

    // 9. Icon data URL
    let icon_path = save_dir.join("icon.png");
    let icon_data_url = if icon_path.exists() {
        match fs::read(&icon_path) {
            Ok(bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                Some(format!("data:image/png;base64,{}", b64))
            }
            Err(_) => None,
        }
    } else {
        None
    };

    Ok(SaveInfo {
        folder_name,
        path,
        name,
        seed,
        spawn,
        game_mode,
        difficulty,
        play_time,
        last_played,
        data_version,
        icon_data_url,
    })
}

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

fn zip_directory(src_dir: &Path, dest_zip: &Path, prefix: Option<&str>) -> Result<(), SavesError> {
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
    fs::create_dir_all(saves_dir)?;

    let file = fs::File::open(zip_path)?;
    let mut zip = ZipArchive::new(file)?;

    let mut root_level_dat = false;
    let mut folder_prefix: Option<String> = None;

    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() == 1 && parts[0] == "level.dat" {
            root_level_dat = true;
            break;
        } else if parts.len() >= 2 && parts.last() == Some(&"level.dat") && folder_prefix.is_none()
        {
            folder_prefix = Some(parts[..parts.len() - 1].join("/"));
        }
    }

    let (base_name, strip_prefix) = if root_level_dat {
        let stem = zip_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported_save");
        (stem.to_string(), None)
    } else if let Some(prefix) = folder_prefix {
        let first_seg = prefix
            .split('/')
            .next()
            .unwrap_or("imported_save")
            .to_string();
        (first_seg, Some(prefix))
    } else {
        return Err(SavesError::InvalidPath(
            "No level.dat found in save zip".into(),
        ));
    };

    let clean_name: String = base_name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect();
    let clean_name = if clean_name.is_empty() {
        "imported_save".to_string()
    } else {
        clean_name
    };

    let mut dest_dir = saves_dir.join(&clean_name);
    let mut counter = 1;
    while dest_dir.exists() {
        dest_dir = saves_dir.join(format!("{}-{}", clean_name, counter));
        counter += 1;
    }
    fs::create_dir_all(&dest_dir)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().replace('\\', "/");
        let rel_name = if let Some(p) = &strip_prefix {
            let p_slash = format!("{}/", p);
            if let Some(stripped) = name.strip_prefix(&p_slash) {
                stripped
            } else {
                continue;
            }
        } else {
            &name
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

fn extract_datapack_lists(root: &Value) -> (Vec<String>, Vec<String>, Option<i32>) {
    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    let mut data_version = None;

    if let Value::Compound(map) = root {
        let data = map
            .get("Data")
            .or_else(|| map.get("data"))
            .and_then(|v| match v {
                Value::Compound(d) => Some(d),
                _ => None,
            })
            .unwrap_or(map);

        data_version = data.get("DataVersion").and_then(as_i32);

        if let Some(Value::Compound(dp)) = data.get("DataPacks") {
            if let Some(Value::List(e_list)) = dp.get("Enabled") {
                for item in e_list {
                    if let Value::String(s) = item {
                        enabled.push(s.clone());
                    }
                }
            }
            if let Some(Value::List(d_list)) = dp.get("Disabled") {
                for item in d_list {
                    if let Value::String(s) = item {
                        disabled.push(s.clone());
                    }
                }
            }
        }
    }

    (enabled, disabled, data_version)
}

fn parse_pack_mcmeta(json_str: &str) -> (i32, String) {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        let pack = &v["pack"];
        let format = pack["pack_format"].as_i64().unwrap_or(0) as i32;
        let desc = if let Some(s) = pack["description"].as_str() {
            s.to_string()
        } else if let Some(txt) = pack["description"]["text"].as_str() {
            txt.to_string()
        } else if pack["description"].is_object() || pack["description"].is_array() {
            pack["description"].to_string()
        } else {
            String::new()
        };
        (format, desc)
    } else {
        (0, String::new())
    }
}

fn read_datapack_meta(path: &Path) -> (i32, String) {
    if path.is_dir() {
        let meta_file = path.join("pack.mcmeta");
        if let Ok(content) = fs::read_to_string(&meta_file) {
            return parse_pack_mcmeta(&content);
        }
    } else if let Ok(f) = fs::File::open(path)
        && let Ok(mut zip) = ZipArchive::new(f)
        && let Ok(mut entry) = zip.by_name("pack.mcmeta")
    {
        let mut content = String::new();
        if entry.read_to_string(&mut content).is_ok() {
            return parse_pack_mcmeta(&content);
        }
    }
    (0, String::new())
}

/// Scans `<save_dir>/datapacks` and merges with `Data.DataPacks.{Enabled, Disabled}` from `level.dat`.
pub fn list_datapacks(save_dir: &Path) -> Result<Vec<DatapackInfo>, SavesError> {
    let level_dat_path = save_dir.join("level.dat");
    let (enabled_set, disabled_set, data_version) = if level_dat_path.exists() {
        match read_level_dat(&level_dat_path) {
            Ok(root) => extract_datapack_lists(&root),
            Err(_) => (Vec::new(), Vec::new(), None),
        }
    } else {
        (Vec::new(), Vec::new(), None)
    };

    let mut datapacks = Vec::new();
    let is_modern = data_version.unwrap_or(0) >= 1443;

    if is_modern {
        let vanilla_enabled = !disabled_set.iter().any(|s| s == "vanilla");
        datapacks.push(DatapackInfo {
            id: "vanilla".to_string(),
            file_name: "vanilla".to_string(),
            description: "Vanilla data pack".to_string(),
            pack_format: data_version.unwrap_or(0),
            enabled: vanilla_enabled,
            built_in: true,
        });
    }

    let datapacks_dir = save_dir.join("datapacks");
    if datapacks_dir.exists() {
        for entry in fs::read_dir(&datapacks_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_name.starts_with('.') {
                continue;
            }

            let (pack_format, description) = read_datapack_meta(&path);
            let raw_id = file_name
                .strip_suffix(".disabled")
                .unwrap_or(&file_name)
                .to_string();

            let enabled = if !is_modern {
                !file_name.ends_with(".disabled")
            } else {
                let target1 = format!("file/{}", file_name);
                let target2 = file_name.clone();
                let target3 = format!("file/{}", raw_id);
                let target4 = raw_id.clone();

                if disabled_set
                    .iter()
                    .any(|s| s == &target1 || s == &target2 || s == &target3 || s == &target4)
                {
                    false
                } else if enabled_set
                    .iter()
                    .any(|s| s == &target1 || s == &target2 || s == &target3 || s == &target4)
                {
                    true
                } else {
                    !file_name.ends_with(".disabled")
                }
            };

            datapacks.push(DatapackInfo {
                id: raw_id,
                file_name,
                description,
                pack_format,
                enabled,
                built_in: false,
            });
        }
    }

    Ok(datapacks)
}

/// Toggles a datapack in `level.dat` (or renames `.zip` <-> `.zip.disabled` if `DataVersion < 1443`).
pub fn toggle_datapack(
    save_dir: &Path,
    pack_id: &str,
    enabled: bool,
) -> Result<Vec<DatapackInfo>, SavesError> {
    let level_dat_path = save_dir.join("level.dat");
    if !level_dat_path.exists() {
        return Err(SavesError::SaveNotFound(
            save_dir.to_string_lossy().to_string(),
        ));
    }

    let mut root = read_level_dat(&level_dat_path)?;
    let (_, _, dv) = extract_datapack_lists(&root);
    let is_modern = dv.unwrap_or(0) >= 1443;

    if is_modern {
        let (root_key, is_inner) = match &root {
            Value::Compound(map) => {
                if map.contains_key("Data") {
                    ("Data", true)
                } else if map.contains_key("data") {
                    ("data", true)
                } else {
                    ("", false)
                }
            }
            _ => return Err(SavesError::InvalidPath("Invalid NBT compound".into())),
        };

        let target_item = if pack_id == "vanilla" {
            "vanilla".to_string()
        } else if pack_id.starts_with("file/") {
            pack_id.to_string()
        } else {
            format!("file/{}", pack_id)
        };
        let bare_item = pack_id.strip_prefix("file/").unwrap_or(pack_id).to_string();

        let data = match &mut root {
            Value::Compound(map) => {
                if is_inner {
                    map.get_mut(root_key)
                        .and_then(|v| match v {
                            Value::Compound(d) => Some(d),
                            _ => None,
                        })
                        .unwrap()
                } else {
                    map
                }
            }
            _ => unreachable!(),
        };

        if !data.contains_key("DataPacks") {
            data.insert("DataPacks".to_string(), Value::Compound(HashMap::new()));
        }

        if let Some(Value::Compound(dp)) = data.get_mut("DataPacks") {
            if !dp.contains_key("Enabled") {
                dp.insert("Enabled".to_string(), Value::List(Vec::new()));
            }
            if !dp.contains_key("Disabled") {
                dp.insert("Disabled".to_string(), Value::List(Vec::new()));
            }

            let e_list = match dp.get_mut("Enabled") {
                Some(Value::List(l)) => l,
                _ => unreachable!(),
            };
            if enabled {
                if !e_list.iter().any(|v| match v {
                    Value::String(s) => s == &target_item || s == &bare_item,
                    _ => false,
                }) {
                    e_list.push(Value::String(target_item.clone()));
                }
            } else {
                e_list.retain(|v| match v {
                    Value::String(s) => s != &target_item && s != &bare_item,
                    _ => true,
                });
            }

            let d_list = match dp.get_mut("Disabled") {
                Some(Value::List(l)) => l,
                _ => unreachable!(),
            };
            if enabled {
                d_list.retain(|v| match v {
                    Value::String(s) => s != &target_item && s != &bare_item,
                    _ => true,
                });
            } else if !d_list.iter().any(|v| match v {
                Value::String(s) => s == &target_item || s == &bare_item,
                _ => false,
            }) {
                d_list.push(Value::String(target_item.clone()));
            }
        }

        write_level_dat(&level_dat_path, &root)?;
    } else {
        let datapacks_dir = save_dir.join("datapacks");
        if datapacks_dir.exists() {
            if enabled {
                let disabled_path = datapacks_dir.join(format!("{}.disabled", pack_id));
                if disabled_path.exists() {
                    let enabled_path = datapacks_dir.join(pack_id);
                    fs::rename(&disabled_path, &enabled_path)?;
                }
            } else {
                let enabled_path = datapacks_dir.join(pack_id);
                if enabled_path.exists() {
                    let disabled_path = datapacks_dir.join(format!("{}.disabled", pack_id));
                    fs::rename(&enabled_path, &disabled_path)?;
                }
            }
        }
    }

    list_datapacks(save_dir)
}

/// Lists screenshots in `screenshots_dir`, sorted newest first.
pub fn list_screenshots(screenshots_dir: &Path) -> Result<Vec<ScreenshotInfo>, SavesError> {
    if !screenshots_dir.exists() {
        return Ok(Vec::new());
    }
    let mut screenshots = Vec::new();
    for entry in fs::read_dir(screenshots_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if path.is_file() && file_name.to_ascii_lowercase().ends_with(".png") {
            let meta = fs::metadata(&path)?;
            let size = meta.len();
            let modified_at =
                format_system_time(meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH));

            let data_url = match fs::read(&path) {
                Ok(bytes) => {
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                    Some(format!("data:image/png;base64,{}", b64))
                }
                Err(_) => None,
            };

            screenshots.push(ScreenshotInfo {
                file_name,
                path: path.to_string_lossy().to_string(),
                size,
                modified_at,
                data_url,
            });
        }
    }
    screenshots.sort_by_key(|a| std::cmp::Reverse(a.modified_at.clone()));
    Ok(screenshots)
}

/// Deletes a screenshot from `screenshots_dir`.
pub fn delete_screenshot(screenshots_dir: &Path, file_name: &str) -> Result<(), SavesError> {
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(SavesError::InvalidPath("Invalid file name".into()));
    }
    let target = screenshots_dir.join(file_name);
    if !target.exists() {
        return Err(SavesError::InvalidPath(format!(
            "Screenshot not found: {}",
            file_name
        )));
    }
    if let Err(trash_err) = trash::delete(&target) {
        tracing::warn!("trash::delete failed ({trash_err}), falling back to remove_file");
        fs::remove_file(&target)?;
    }
    Ok(())
}

/// Lists files in an instance folder (`resourcepacks`, `shaderpacks`, `schematics`).
pub fn list_instance_files(dir: &Path) -> Result<Vec<InstanceFileItem>, SavesError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with('.') {
            continue;
        }

        let meta = entry.metadata()?;
        let size = if meta.is_file() { meta.len() } else { 0 };
        let modified_at =
            format_system_time(meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH));
        let enabled = !file_name.ends_with(".disabled");

        items.push(InstanceFileItem {
            file_name,
            path: path.to_string_lossy().to_string(),
            enabled,
            size,
            modified_at,
        });
    }
    items.sort_by(|a, b| {
        a.file_name
            .to_ascii_lowercase()
            .cmp(&b.file_name.to_ascii_lowercase())
    });
    Ok(items)
}

/// Toggles an instance file between enabled and disabled by renaming with `.disabled` suffix.
pub fn toggle_instance_file(
    dir: &Path,
    file_name: &str,
    enabled: bool,
) -> Result<InstanceFileItem, SavesError> {
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(SavesError::InvalidPath("Invalid file name".into()));
    }
    let src_path = dir.join(file_name);
    if !src_path.exists() {
        return Err(SavesError::InvalidPath(format!(
            "File not found: {}",
            file_name
        )));
    }

    let is_currently_disabled = file_name.ends_with(".disabled");
    let target_file_name = if enabled && is_currently_disabled {
        file_name.strip_suffix(".disabled").unwrap().to_string()
    } else if !enabled && !is_currently_disabled {
        format!("{}.disabled", file_name)
    } else {
        file_name.to_string()
    };

    let target_path = dir.join(&target_file_name);
    if src_path != target_path {
        fs::rename(&src_path, &target_path)?;
    }

    let meta = fs::metadata(&target_path)?;
    let size = if meta.is_file() { meta.len() } else { 0 };
    let modified_at =
        format_system_time(meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH));

    Ok(InstanceFileItem {
        file_name: target_file_name,
        path: target_path.to_string_lossy().to_string(),
        enabled,
        size,
        modified_at,
    })
}

/// Deletes an instance file or folder.
pub fn delete_instance_file(dir: &Path, file_name: &str) -> Result<(), SavesError> {
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return Err(SavesError::InvalidPath("Invalid file name".into()));
    }
    let target = dir.join(file_name);
    if !target.exists() {
        return Err(SavesError::InvalidPath(format!(
            "File not found: {}",
            file_name
        )));
    }
    if let Err(trash_err) = trash::delete(&target) {
        tracing::warn!("trash::delete failed ({trash_err}), falling back to direct removal");
        if target.is_dir() {
            fs::remove_dir_all(&target)?;
        } else {
            fs::remove_file(&target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_format_system_time() {
        let epoch = std::time::SystemTime::UNIX_EPOCH;
        assert_eq!(format_system_time(epoch), "1970-01-01T00:00:00Z");

        let after = epoch + std::time::Duration::from_secs(3665);
        assert_eq!(format_system_time(after), "1970-01-01T01:01:05Z");
    }

    #[test]
    fn test_nbt_roundtrip_boundaries() {
        let dir = tempdir().unwrap();

        // 1. Pre-100 (<15w32a): absent DataVersion, RandomSeed, SpawnX/Y/Z, Difficulty
        {
            let save_dir = dir.path().join("pre100");
            fs::create_dir_all(&save_dir).unwrap();
            let mut data = HashMap::new();
            data.insert("LevelName".into(), Value::String("AlphaWorld".into()));
            data.insert("RandomSeed".into(), Value::Long(12345));
            data.insert("SpawnX".into(), Value::Int(10));
            data.insert("SpawnY".into(), Value::Int(64));
            data.insert("SpawnZ".into(), Value::Int(20));
            data.insert("Difficulty".into(), Value::Byte(1));
            data.insert("GameType".into(), Value::Int(0));
            data.insert("Time".into(), Value::Long(1000));
            data.insert("LastPlayed".into(), Value::Long(1600000000000));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));
            let root_val = Value::Compound(root);

            let dat_path = save_dir.join("level.dat");
            write_level_dat(&dat_path, &root_val).unwrap();

            let read_val = read_level_dat(&dat_path).unwrap();
            assert_eq!(read_val, root_val);

            let info = parse_save_info(&save_dir).unwrap();
            assert_eq!(info.name, "AlphaWorld");
            assert_eq!(info.seed, "12345");
            assert_eq!(
                info.spawn,
                SaveSpawn {
                    x: 10,
                    y: 64,
                    z: 20
                }
            );
            assert_eq!(info.difficulty, 1);
            assert_eq!(info.game_mode, 0);
            assert_eq!(info.data_version, None);
            assert_eq!(info.play_time, 1000);
            assert_eq!(info.last_played, 1600000000000);
        }

        // 2. 100 <= DataVersion < 1443 (1.12.2, dv = 1343)
        {
            let save_dir = dir.path().join("mc112");
            fs::create_dir_all(&save_dir).unwrap();
            let mut data = HashMap::new();
            data.insert("DataVersion".into(), Value::Int(1343));
            data.insert("LevelName".into(), Value::String("MC112World".into()));
            data.insert("RandomSeed".into(), Value::Long(67890));
            data.insert("SpawnX".into(), Value::Int(100));
            data.insert("SpawnY".into(), Value::Int(70));
            data.insert("SpawnZ".into(), Value::Int(-50));
            data.insert("Difficulty".into(), Value::Byte(2));
            data.insert("GameType".into(), Value::Int(1));
            data.insert("Time".into(), Value::Long(20000));
            data.insert("LastPlayed".into(), Value::Long(1610000000000));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));
            let root_val = Value::Compound(root);

            let dat_path = save_dir.join("level.dat");
            write_level_dat(&dat_path, &root_val).unwrap();

            let read_val = read_level_dat(&dat_path).unwrap();
            assert_eq!(read_val, root_val);

            let info = parse_save_info(&save_dir).unwrap();
            assert_eq!(info.name, "MC112World");
            assert_eq!(info.seed, "67890");
            assert_eq!(
                info.spawn,
                SaveSpawn {
                    x: 100,
                    y: 70,
                    z: -50
                }
            );
            assert_eq!(info.difficulty, 2);
            assert_eq!(info.game_mode, 1);
            assert_eq!(info.data_version, Some(1343));
        }

        // 3. 1443 <= DataVersion < 2536 (1.14.4, dv = 1976 with DataPacks)
        {
            let save_dir = dir.path().join("mc114");
            fs::create_dir_all(&save_dir).unwrap();

            let mut dp = HashMap::new();
            dp.insert(
                "Enabled".into(),
                Value::List(vec![
                    Value::String("vanilla".into()),
                    Value::String("file/custom1.zip".into()),
                ]),
            );
            dp.insert(
                "Disabled".into(),
                Value::List(vec![Value::String("file/custom2.zip".into())]),
            );

            let mut data = HashMap::new();
            data.insert("DataVersion".into(), Value::Int(1976));
            data.insert("LevelName".into(), Value::String("MC114World".into()));
            data.insert("RandomSeed".into(), Value::Long(112233));
            data.insert("SpawnX".into(), Value::Int(-25));
            data.insert("SpawnY".into(), Value::Int(65));
            data.insert("SpawnZ".into(), Value::Int(150));
            data.insert("Difficulty".into(), Value::Byte(3));
            data.insert("DataPacks".into(), Value::Compound(dp));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));
            let root_val = Value::Compound(root);

            let dat_path = save_dir.join("level.dat");
            write_level_dat(&dat_path, &root_val).unwrap();

            let read_val = read_level_dat(&dat_path).unwrap();
            assert_eq!(read_val, root_val);

            let info = parse_save_info(&save_dir).unwrap();
            assert_eq!(info.data_version, Some(1976));
            assert_eq!(info.seed, "112233");
            assert_eq!(info.difficulty, 3);

            // Test toggle_datapack: disable custom1.zip, enable custom2.zip
            let dps = toggle_datapack(&save_dir, "custom1.zip", false).unwrap();
            let c1 = dps
                .iter()
                .find(|d| d.id == "custom1.zip" || d.id == "file/custom1.zip");
            if let Some(c1_info) = c1 {
                assert!(!c1_info.enabled);
            }

            let dps2 = toggle_datapack(&save_dir, "custom2.zip", true).unwrap();
            let c2 = dps2
                .iter()
                .find(|d| d.id == "custom2.zip" || d.id == "file/custom2.zip");
            if let Some(c2_info) = c2 {
                assert!(c2_info.enabled);
            }
        }

        // 4. 2536 <= DataVersion < 4774 (1.20.1, dv = 3465 with WorldGenSettings.seed)
        {
            let save_dir = dir.path().join("mc120");
            fs::create_dir_all(&save_dir).unwrap();

            let mut wgs = HashMap::new();
            wgs.insert("seed".into(), Value::Long(9876543210));

            let mut data = HashMap::new();
            data.insert("DataVersion".into(), Value::Int(3465));
            data.insert("LevelName".into(), Value::String("MC120World".into()));
            data.insert("WorldGenSettings".into(), Value::Compound(wgs));
            data.insert("SpawnX".into(), Value::Int(0));
            data.insert("SpawnY".into(), Value::Int(64));
            data.insert("SpawnZ".into(), Value::Int(0));
            data.insert("Difficulty".into(), Value::Byte(1));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));
            let root_val = Value::Compound(root);

            let dat_path = save_dir.join("level.dat");
            write_level_dat(&dat_path, &root_val).unwrap();

            let info = parse_save_info(&save_dir).unwrap();
            assert_eq!(info.data_version, Some(3465));
            assert_eq!(info.seed, "9876543210");
            assert_eq!(info.difficulty, 1);
        }

        // 5. DataVersion >= 4774 (26.1 snapshot with spawn.pos and difficulty_settings)
        {
            let save_dir = dir.path().join("mc26");
            fs::create_dir_all(&save_dir).unwrap();

            let mut spawn_compound = HashMap::new();
            spawn_compound.insert(
                "pos".into(),
                Value::IntArray(fastnbt::IntArray::new(vec![10, 64, -20])),
            );

            let mut diff_compound = HashMap::new();
            diff_compound.insert("difficulty".into(), Value::String("hard".into()));

            let mut wgs = HashMap::new();
            wgs.insert("seed".into(), Value::Long(555555555));

            let mut data = HashMap::new();
            data.insert("DataVersion".into(), Value::Int(4774));
            data.insert("LevelName".into(), Value::String("MC26World".into()));
            data.insert("WorldGenSettings".into(), Value::Compound(wgs));
            data.insert("spawn".into(), Value::Compound(spawn_compound));
            data.insert("difficulty_settings".into(), Value::Compound(diff_compound));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));
            let root_val = Value::Compound(root);

            let dat_path = save_dir.join("level.dat");
            write_level_dat(&dat_path, &root_val).unwrap();

            let info = parse_save_info(&save_dir).unwrap();
            assert_eq!(info.data_version, Some(4774));
            assert_eq!(info.seed, "555555555");
            assert_eq!(
                info.spawn,
                SaveSpawn {
                    x: 10,
                    y: 64,
                    z: -20
                }
            );
            assert_eq!(info.difficulty, 3); // "hard" -> 3
        }
    }

    #[test]
    fn test_export_and_import_save_zip() {
        let dir = tempdir().unwrap();
        let saves_dir = dir.path().join("saves");
        let world_dir = saves_dir.join("TestWorld");
        fs::create_dir_all(&world_dir).unwrap();

        let mut data = HashMap::new();
        data.insert("DataVersion".into(), Value::Int(3465));
        data.insert("LevelName".into(), Value::String("ExportWorld".into()));
        data.insert("RandomSeed".into(), Value::Long(777));
        data.insert("SpawnX".into(), Value::Int(5));
        data.insert("SpawnY".into(), Value::Int(60));
        data.insert("SpawnZ".into(), Value::Int(7));

        let mut root = HashMap::new();
        root.insert("Data".into(), Value::Compound(data));
        write_level_dat(&world_dir.join("level.dat"), &Value::Compound(root)).unwrap();

        // Write a dummy icon.png
        fs::write(world_dir.join("icon.png"), b"fake_png_data").unwrap();

        let export_zip = dir.path().join("world_export.zip");
        export_save_zip(&saves_dir, "TestWorld", &export_zip).unwrap();
        assert!(export_zip.exists());

        // Import into another saves directory
        let new_saves_dir = dir.path().join("imported_saves");
        let imported_info = import_save_zip(&new_saves_dir, &export_zip).unwrap();
        assert_eq!(imported_info.name, "ExportWorld");
        assert_eq!(imported_info.seed, "777");
        assert_eq!(imported_info.spawn, SaveSpawn { x: 5, y: 60, z: 7 });
        assert!(imported_info.icon_data_url.is_some());
    }

    #[test]
    fn test_screenshots_and_instance_files() {
        let dir = tempdir().unwrap();
        let ss_dir = dir.path().join("screenshots");
        fs::create_dir_all(&ss_dir).unwrap();

        let img_file = ss_dir.join("2024-05-01_12.00.00.png");
        fs::write(&img_file, b"sample_png").unwrap();

        let list = list_screenshots(&ss_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].file_name, "2024-05-01_12.00.00.png");
        assert!(list[0].data_url.is_some());

        delete_screenshot(&ss_dir, "2024-05-01_12.00.00.png").unwrap();
        assert_eq!(list_screenshots(&ss_dir).unwrap().len(), 0);

        // Instance files
        let rp_dir = dir.path().join("resourcepacks");
        fs::create_dir_all(&rp_dir).unwrap();

        let rp_file = rp_dir.join("pack.zip");
        fs::write(&rp_file, b"zip_content").unwrap();

        let files = list_instance_files(&rp_dir).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].enabled);

        let toggled = toggle_instance_file(&rp_dir, "pack.zip", false).unwrap();
        assert!(!toggled.enabled);
        assert_eq!(toggled.file_name, "pack.zip.disabled");

        let re_toggled = toggle_instance_file(&rp_dir, "pack.zip.disabled", true).unwrap();
        assert!(re_toggled.enabled);
        assert_eq!(re_toggled.file_name, "pack.zip");

        delete_instance_file(&rp_dir, "pack.zip").unwrap();
        assert_eq!(list_instance_files(&rp_dir).unwrap().len(), 0);
    }
}
