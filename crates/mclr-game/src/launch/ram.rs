//! Memory calculation, RAM limits, and free physical memory querying.

use std::fs;
use std::path::Path;

use mclr_core::settings::LaunchSettings;
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

use crate::instance::InstanceConfig;

/// Queries free physical memory in MB on Windows.
pub fn get_free_physical_memory_mb() -> Option<u64> {
    unsafe {
        let mut mem_status = MEMORYSTATUSEX {
            dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
            ..Default::default()
        };
        if GlobalMemoryStatusEx(&mut mem_status).is_ok() {
            Some(mem_status.ullAvailPhys / (1024 * 1024))
        } else {
            None
        }
    }
}

/// Counts `.jar` files in a directory.
pub fn count_mods_in_dir(mods_dir: &Path) -> u32 {
    if !mods_dir.exists() {
        return 0;
    }
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(mods_dir) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    let path = entry.path();
                    if path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("jar"))
                    {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// Calculates memory in MB according to auto-RAM formula:
/// `required = 2048 + mod_count * 25`, capped at `0.8 * free_ram_mb` and `16384`, minimum `2048`.
pub fn compute_ram_mb_auto(mod_count: u32, free_ram_mb: u64) -> u32 {
    let required = 2048 + mod_count * 25;
    let cap = ((0.8 * (free_ram_mb as f64)) as u32).min(16384);
    required.min(cap).max(2048)
}

/// Calculates RAM allocation in MB.
pub fn compute_ram_mb(
    launch_settings: &LaunchSettings,
    instance_config: &InstanceConfig,
    game_dir: &Path,
) -> u32 {
    if let Some(m) = instance_config.memory {
        if m > 0 {
            return m;
        }
    }

    if launch_settings.ram_mode == "custom" {
        return launch_settings.ram_custom_mb.clamp(512, 32768);
    }

    let mod_count = count_mods_in_dir(&game_dir.join("mods"));
    let free_ram_mb = get_free_physical_memory_mb().unwrap_or(8192);
    compute_ram_mb_auto(mod_count, free_ram_mb)
}
