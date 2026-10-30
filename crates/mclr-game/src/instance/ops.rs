//! File and instance lifecycle operations (validate, rename, duplicate, delete, save config).

use std::fs;
use std::path::Path;

use super::scanner::{INSTANCE_CACHE, scan_instances};
use super::{InstanceConfig, InstanceInfo};
use crate::error::GameError;

/// Validates an instance name (PCL FolderNameValidator parity).
/// Max 70 chars, non-empty, no leading/trailing space or trailing dot,
/// forbidden chars: \ / : * ? " < > | ! ;, and no DOS device names.
pub fn validate_instance_name(name: &str) -> Result<(), GameError> {
    if name.is_empty() {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot be empty".to_string(),
        ));
    }
    if name.starts_with(' ') || name.ends_with(' ') {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot start or end with spaces".to_string(),
        ));
    }
    if name.ends_with('.') {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot end with a period".to_string(),
        ));
    }
    if name.chars().count() > 70 {
        return Err(GameError::InvalidInstanceName(
            "Instance name cannot exceed 70 characters".to_string(),
        ));
    }
    const FORBIDDEN: &[char] = &['\\', '/', ':', '*', '?', '"', '<', '>', '|', '!', ';'];
    if name.contains(FORBIDDEN) {
        return Err(GameError::InvalidInstanceName(format!(
            "Instance name contains invalid characters: {name}"
        )));
    }
    const RESERVED_DOS: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let base_name = name.split('.').next().unwrap_or(name);
    for reserved in RESERVED_DOS {
        if base_name.eq_ignore_ascii_case(reserved) {
            return Err(GameError::InvalidInstanceName(format!(
                "Instance name is a reserved system name: {name}"
            )));
        }
    }
    Ok(())
}

/// Renames an instance: moves `<mc_folder>/versions/<id>` to `<mc_folder>/versions/<new_name>`
/// and renames `<id>.json` and `<id>.jar` and `<id>-natives` if present.
pub fn rename_instance(
    mc_folder: &Path,
    id: &str,
    new_name: &str,
) -> Result<InstanceInfo, GameError> {
    let versions_dir = mc_folder.join("versions");
    let old_dir = versions_dir.join(id);
    if !old_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    if id == new_name {
        let list = scan_instances(mc_folder)?;
        return list
            .into_iter()
            .find(|i| i.id == new_name)
            .ok_or_else(|| GameError::InstanceNotFound(new_name.to_string()));
    }

    let case_only = id.eq_ignore_ascii_case(new_name);
    if case_only {
        crate::naming::validate_name(new_name, None)?;
    } else {
        crate::naming::validate_name(new_name, Some(&versions_dir))?;
    }

    let new_dir = versions_dir.join(new_name);
    if !case_only && new_dir.exists() {
        return Err(GameError::InstanceExists(new_name.to_string()));
    }
    if case_only {
        let temp_name = format!(
            "{id}__mclr_temp_rename_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let temp_dir = versions_dir.join(&temp_name);
        fs::rename(&old_dir, &temp_dir).map_err(GameError::Io)?;
        fs::rename(&temp_dir, &new_dir).map_err(GameError::Io)?;
    } else {
        fs::rename(&old_dir, &new_dir).map_err(GameError::Io)?;
    }

    // Rename json file if <id>.json exists
    let old_json = new_dir.join(format!("{id}.json"));
    let new_json = new_dir.join(format!("{new_name}.json"));
    if old_json.exists() {
        let _ = fs::rename(&old_json, &new_json);
        if let Ok(content) = fs::read_to_string(&new_json) {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object_mut() {
                    obj.insert(
                        "id".to_string(),
                        serde_json::Value::String(new_name.to_string()),
                    );
                    if let Ok(updated) = serde_json::to_string_pretty(&val) {
                        let _ = fs::write(&new_json, updated);
                    }
                }
            }
        }
    }

    // Rename jar file if <id>.jar exists
    let old_jar = new_dir.join(format!("{id}.jar"));
    let new_jar = new_dir.join(format!("{new_name}.jar"));
    if old_jar.exists() {
        let _ = fs::rename(old_jar, new_jar);
    }

    // Rename <id>-natives directory if exists
    let old_natives = new_dir.join(format!("{id}-natives"));
    let new_natives = new_dir.join(format!("{new_name}-natives"));
    if old_natives.exists() {
        let _ = fs::rename(old_natives, new_natives);
    }

    // Invalidate cache
    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(id);
            cache.instances.remove(new_name);
        }
    }

    let list = scan_instances(mc_folder)?;
    list.into_iter()
        .find(|i| i.id == new_name)
        .ok_or_else(|| GameError::InstanceNotFound(new_name.to_string()))
}

/// Recursively copies a directory.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_child = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_child)?;
        } else {
            fs::copy(entry.path(), dest_child)?;
        }
    }
    Ok(())
}

/// Duplicates an instance to a new name.
pub fn duplicate_instance(
    mc_folder: &Path,
    id: &str,
    new_name: &str,
) -> Result<InstanceInfo, GameError> {
    let versions_dir = mc_folder.join("versions");
    crate::naming::validate_name(new_name, Some(&versions_dir))?;

    let src_dir = versions_dir.join(id);
    if !src_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    let dst_dir = versions_dir.join(new_name);
    if dst_dir.exists() {
        return Err(GameError::InstanceExists(new_name.to_string()));
    }

    copy_dir_all(&src_dir, &dst_dir).map_err(GameError::Io)?;

    let old_json = dst_dir.join(format!("{id}.json"));
    let new_json = dst_dir.join(format!("{new_name}.json"));
    if old_json.exists() {
        let _ = fs::rename(&old_json, &new_json);
        if let Ok(content) = fs::read_to_string(&new_json) {
            if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = val.as_object_mut() {
                    obj.insert(
                        "id".to_string(),
                        serde_json::Value::String(new_name.to_string()),
                    );
                    if let Ok(updated) = serde_json::to_string_pretty(&val) {
                        let _ = fs::write(&new_json, updated);
                    }
                }
            }
        }
    }

    let old_jar = dst_dir.join(format!("{id}.jar"));
    let new_jar = dst_dir.join(format!("{new_name}.jar"));
    if old_jar.exists() {
        let _ = fs::rename(old_jar, new_jar);
    }

    let old_natives = dst_dir.join(format!("{id}-natives"));
    let new_natives = dst_dir.join(format!("{new_name}-natives"));
    if old_natives.exists() {
        let _ = fs::rename(old_natives, new_natives);
    }

    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(new_name);
        }
    }

    let list = scan_instances(mc_folder)?;
    list.into_iter()
        .find(|i| i.id == new_name)
        .ok_or_else(|| GameError::InstanceNotFound(new_name.to_string()))
}

/// Deletes an instance folder, optionally permanently or to Recycle Bin.
pub fn delete_instance_ext(mc_folder: &Path, id: &str, permanent: bool) -> Result<(), GameError> {
    let instance_dir = mc_folder.join("versions").join(id);
    if !instance_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    if permanent {
        fs::remove_dir_all(&instance_dir).map_err(GameError::Io)?;
    } else if let Err(e) = trash::delete(&instance_dir) {
        tracing::warn!(
            "trash::delete failed for {}: {e}; falling back to remove_dir_all",
            instance_dir.display()
        );
        fs::remove_dir_all(&instance_dir).map_err(GameError::Io)?;
    }

    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(id);
        }
    }

    Ok(())
}

/// Deletes an instance folder. Moves to Windows Recycle Bin via `trash::delete`,
/// falling back to `fs::remove_dir_all`.
pub fn delete_instance(mc_folder: &Path, id: &str) -> Result<(), GameError> {
    delete_instance_ext(mc_folder, id, false)
}

/// Records the last successful launch time, preserving every other config field.
///
/// Written as a single-key patch so a config edited concurrently is not overwritten
/// with a stale snapshot of the other fields.
pub fn touch_instance_last_played(
    mc_folder: &Path,
    id: &str,
    timestamp: &str,
) -> Result<(), GameError> {
    let instance_dir = mc_folder.join("versions").join(id);
    if !instance_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    let path = instance_dir.join("mclr.instance.json");
    let mut value = fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .filter(|v| v.is_object())
        .or_else(|| serde_json::to_value(InstanceConfig::default()).ok());
    let Some(obj) = value.as_mut().and_then(|v| v.as_object_mut()) else {
        return Ok(());
    };
    obj.insert(
        "lastPlayedAt".to_string(),
        serde_json::Value::String(timestamp.to_string()),
    );
    let json_str = serde_json::to_string_pretty(&value)
        .map_err(|e| GameError::InvalidVersionJson(e.to_string()))?;
    fs::write(&path, json_str).map_err(GameError::Io)?;

    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(id);
        }
    }
    Ok(())
}

/// Saves instance configuration to `<instance_dir>/mclr.instance.json`.
pub fn save_instance_config(
    mc_folder: &Path,
    id: &str,
    config: &InstanceConfig,
) -> Result<InstanceInfo, GameError> {
    let instance_dir = mc_folder.join("versions").join(id);
    if !instance_dir.exists() {
        return Err(GameError::InstanceNotFound(id.to_string()));
    }

    let mut to_save = config.clone();
    if to_save.pinned && !to_save.starred {
        to_save.starred = true;
    } else if to_save.starred && !to_save.pinned {
        to_save.pinned = true;
    }

    let config_path = instance_dir.join("mclr.instance.json");
    let json_str = serde_json::to_string_pretty(&to_save)
        .map_err(|e| GameError::InvalidVersionJson(e.to_string()))?;
    fs::write(config_path, json_str).map_err(GameError::Io)?;

    if let Ok(mut cache_map) = INSTANCE_CACHE.lock() {
        if let Some(cache) = cache_map.get_mut(mc_folder) {
            cache.instances.remove(id);
        }
    }

    let list = scan_instances(mc_folder)?;
    list.into_iter()
        .find(|i| i.id == id)
        .ok_or_else(|| GameError::InstanceNotFound(id.to_string()))
}
