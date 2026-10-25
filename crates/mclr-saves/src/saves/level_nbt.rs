//! Level.dat NBT parsing, version boundary handling, data serialization, and datapack management.

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use base64::Engine;
use fastnbt::Value;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use super::{DatapackInfo, SaveInfo, SaveSpawn};
use crate::error::SavesError;

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
    let icon_file = save_dir.join("icon.png");
    let icon_path_str = if icon_file.exists() {
        Some(icon_file.to_string_lossy().to_string())
    } else {
        None
    };
    let icon_data_url = if icon_file.exists() {
        match fs::read(&icon_file) {
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
        icon_path: icon_path_str,
        icon_data_url,
    })
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
        && let Ok(mut zip) = zip::ZipArchive::new(f)
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
