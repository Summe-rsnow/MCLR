//! Saves, datapacks, screenshots, and instance files management.

pub mod archive;
pub mod level_nbt;
pub mod screenshots;

use serde::{Deserialize, Serialize};
use specta::Type;

pub use archive::{backup_save, delete_save, export_save_zip, import_save_zip, list_saves};
pub use level_nbt::{
    list_datapacks, parse_save_info, parse_save_info_from_nbt, read_level_dat, toggle_datapack,
    write_level_dat, write_level_dat_compressed,
};
pub use screenshots::{
    copy_image_to_clipboard, delete_screenshot, delete_screenshot_path, generate_screenshot_thumb,
    list_screenshots_paged,
};

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
    pub icon_path: Option<String>,
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
pub struct ScreenshotMeta {
    pub file_name: String,
    pub path: String,
    pub size: u64,
    pub created_ms: u64,
    pub modified_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotsPagedResult {
    pub items: Vec<ScreenshotMeta>,
    pub total: usize,
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

#[cfg(test)]
mod tests {
    use fastnbt::Value;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    use super::*;

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
                    Value::String("file/custom.zip".into()),
                ]),
            );
            dp.insert("Disabled".into(), Value::List(vec![]));

            let mut data = HashMap::new();
            data.insert("DataVersion".into(), Value::Int(1976));
            data.insert("LevelName".into(), Value::String("MC114World".into()));
            data.insert("RandomSeed".into(), Value::Long(11111));
            data.insert("DataPacks".into(), Value::Compound(dp));
            data.insert("SpawnX".into(), Value::Int(0));
            data.insert("SpawnY".into(), Value::Int(64));
            data.insert("SpawnZ".into(), Value::Int(0));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));

            let dat_path = save_dir.join("level.dat");
            write_level_dat(&dat_path, &Value::Compound(root)).unwrap();

            let packs = list_datapacks(&save_dir).unwrap();
            assert_eq!(packs.len(), 1);
            assert_eq!(packs[0].id, "vanilla");
            assert!(packs[0].enabled);
            assert!(packs[0].built_in);

            // Toggle vanilla off
            let after = toggle_datapack(&save_dir, "vanilla", false).unwrap();
            assert_eq!(after[0].id, "vanilla");
            assert!(!after[0].enabled);
        }

        // 4. 2536 <= DataVersion < 4774 (1.20.4, dv = 3700 with WorldGenSettings.seed)
        {
            let save_dir = dir.path().join("mc120");
            fs::create_dir_all(&save_dir).unwrap();

            let mut wgs = HashMap::new();
            wgs.insert("seed".into(), Value::Long(999999999999999));

            let mut data = HashMap::new();
            data.insert("DataVersion".into(), Value::Int(3700));
            data.insert("LevelName".into(), Value::String("MC120World".into()));
            data.insert("WorldGenSettings".into(), Value::Compound(wgs));
            data.insert("SpawnX".into(), Value::Int(50));
            data.insert("SpawnY".into(), Value::Int(65));
            data.insert("SpawnZ".into(), Value::Int(-25));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));
            write_level_dat(&save_dir.join("level.dat"), &Value::Compound(root)).unwrap();

            let info = parse_save_info(&save_dir).unwrap();
            assert_eq!(info.seed, "999999999999999");
        }

        // 5. DataVersion >= 4774 (1.21.4+, Data.spawn.pos and difficulty_settings)
        {
            let save_dir = dir.path().join("mc121");
            fs::create_dir_all(&save_dir).unwrap();

            let mut spawn_compound = HashMap::new();
            spawn_compound.insert(
                "pos".into(),
                Value::IntArray(fastnbt::IntArray::new(vec![123, 80, -456])),
            );

            let mut diff_compound = HashMap::new();
            diff_compound.insert("difficulty".into(), Value::String("hard".into()));

            let mut data = HashMap::new();
            data.insert("DataVersion".into(), Value::Int(4800));
            data.insert("LevelName".into(), Value::String("MC121World".into()));
            data.insert("RandomSeed".into(), Value::Long(42));
            data.insert("spawn".into(), Value::Compound(spawn_compound));
            data.insert("difficulty_settings".into(), Value::Compound(diff_compound));

            let mut root = HashMap::new();
            root.insert("Data".into(), Value::Compound(data));
            write_level_dat(&save_dir.join("level.dat"), &Value::Compound(root)).unwrap();

            let info = parse_save_info(&save_dir).unwrap();
            assert_eq!(
                info.spawn,
                SaveSpawn {
                    x: 123,
                    y: 80,
                    z: -456
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
        assert!(imported_info.icon_path.is_some());
    }

    #[test]
    fn test_screenshots() {
        let dir = tempdir().unwrap();
        let ss_dir = dir.path().join("screenshots");
        fs::create_dir_all(&ss_dir).unwrap();

        let img_file = ss_dir.join("2024-05-01_12.00.00.png");
        fs::write(&img_file, b"sample_png").unwrap();

        let paged = list_screenshots_paged(&ss_dir, 0, 10).unwrap();
        assert_eq!(paged.total, 1);
        assert_eq!(paged.items[0].file_name, "2024-05-01_12.00.00.png");

        delete_screenshot(&ss_dir, "2024-05-01_12.00.00.png").unwrap();
        let paged_after = list_screenshots_paged(&ss_dir, 0, 10).unwrap();
        assert_eq!(paged_after.total, 0);
    }
}
