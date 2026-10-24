//! Path resolution and canonical directories for MCLR.
//!
//! Root directory defaults to `%APPDATA%/MCLR` (or `$XDG_DATA_HOME/MCLR` / `~/Library/Application Support/MCLR`),
//! but can be overridden with the `MCLR_DATA_DIR` environment variable.

use std::path::PathBuf;

/// Returns the root data directory for MCLR.
///
/// Priority:
/// 1. `MCLR_DATA_DIR` environment variable (if non-empty)
/// 2. `dirs::data_dir().unwrap().join("MCLR")`
pub fn data_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("MCLR_DATA_DIR") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("MCLR")
}

/// Path to `config.json` in the MCLR data directory.
pub fn config_file() -> PathBuf {
    data_dir().join("config.json")
}

/// Path to `accounts.enc` in the MCLR data directory.
pub fn accounts_file() -> PathBuf {
    data_dir().join("accounts.enc")
}

/// Path to `master.key` in the MCLR data directory.
pub fn master_key_file() -> PathBuf {
    data_dir().join("master.key")
}

/// Path to `cache/` directory.
pub fn cache_dir() -> PathBuf {
    data_dir().join("cache")
}

/// Path to `logs/` directory.
pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Path to `runtime/` directory (for downloaded Java runtimes).
pub fn runtime_dir() -> PathBuf {
    data_dir().join("runtime")
}

/// Path to `tools/` directory (e.g. authlib-injector.jar).
pub fn tools_dir() -> PathBuf {
    data_dir().join("tools")
}

/// Path to `.minecraft/` directory (default Minecraft game folder).
pub fn default_minecraft_dir() -> PathBuf {
    data_dir().join(".minecraft")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_default_and_override() {
        let _lock = crate::test_utils::ENV_MUTEX.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let temp_path = temp.path().to_path_buf();

        unsafe {
            std::env::set_var("MCLR_DATA_DIR", temp_path.to_str().unwrap());
        }
        assert_eq!(data_dir(), temp_path);
        assert_eq!(config_file(), temp_path.join("config.json"));
        assert_eq!(accounts_file(), temp_path.join("accounts.enc"));
        assert_eq!(master_key_file(), temp_path.join("master.key"));
        assert_eq!(cache_dir(), temp_path.join("cache"));
        assert_eq!(logs_dir(), temp_path.join("logs"));
        assert_eq!(runtime_dir(), temp_path.join("runtime"));
        assert_eq!(tools_dir(), temp_path.join("tools"));
        assert_eq!(default_minecraft_dir(), temp_path.join(".minecraft"));

        unsafe {
            std::env::remove_var("MCLR_DATA_DIR");
        }
    }
}
