use mclr_crash::export::export_crash_report_zip;
use std::fs::{self, File};
use std::io::Read;
use zip::ZipArchive;

#[test]
fn test_export_crash_report_zip() {
    let temp = tempfile::tempdir().unwrap();
    let instance_dir = temp.path().join("test_instance");
    fs::create_dir_all(instance_dir.join("mods")).unwrap();
    fs::create_dir_all(instance_dir.join("logs")).unwrap();
    fs::create_dir_all(instance_dir.join("crash-reports")).unwrap();

    // 1. Write dummy logs
    let latest_log_path = instance_dir.join("logs/latest.log");
    fs::write(
        &latest_log_path,
        "[12:00:00] [main/INFO]: Starting Minecraft\n[12:00:01] [main/FATAL]: Something crashed\n",
    )
    .unwrap();

    let crash_report_path = instance_dir.join("crash-reports/crash-2026-10-24.txt");
    fs::write(
        &crash_report_path,
        "---- Minecraft Crash Report ----\n// Manually triggered debug crash\nDescription: Manually triggered debug crash\n",
    )
    .unwrap();

    // 2. Write mclr-launch.args
    let args_path = instance_dir.join("mclr-launch.args");
    fs::write(
        &args_path,
        "-Xmx4G\n-Dlog4j2.formatMsgNoLookups=true\n--accessToken secret_access_token_12345\n",
    )
    .unwrap();

    // 3. Write a dummy mod jar
    let mod_jar_path = instance_dir.join("mods/test_mod.jar");
    fs::write(&mod_jar_path, b"PK\x03\x04dummy_jar_bytes").unwrap();

    let watcher_lines = vec![
        "[12:00:00] [main/INFO]: Watcher line 1".to_string(),
        "[12:00:01] [main/ERROR]: Watcher line 2".to_string(),
    ];

    let dest_zip = temp.path().join("crash-report.zip");
    let result_path =
        export_crash_report_zip(&instance_dir, "test-instance", &watcher_lines, &dest_zip)
            .expect("export crash report zip should succeed");

    assert_eq!(result_path, dest_zip);
    assert!(dest_zip.exists(), "Destination zip should exist");

    // 4. Verify contents of destination zip
    let file = File::open(&dest_zip).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();

    let expected_files = [
        "launch-debug.bat",
        "mclr-launch.args",
        "environment.json",
        "mods.json",
        "watcher-output.log",
        "logs/latest.log",
        "crash-reports/crash-2026-10-24.txt",
    ];

    for expected in &expected_files {
        let zip_entry = archive.by_name(expected);
        assert!(
            zip_entry.is_ok(),
            "Expected file '{expected}' was not found in crash report zip"
        );
    }

    // 5. Verify environment.json content
    {
        let mut env_file = archive.by_name("environment.json").unwrap();
        let mut env_content = String::new();
        env_file.read_to_string(&mut env_content).unwrap();
        let env_json: serde_json::Value = serde_json::from_str(&env_content).unwrap();
        assert_eq!(env_json["instanceId"], "test-instance");
        assert_eq!(env_json["launcher"]["name"], "MCLR");
        assert!(env_json["analysis"]["causes"].is_array());
        let causes = env_json["analysis"]["causes"].as_array().unwrap();
        assert!(
            causes.iter().any(|c| c["code"] == "manualDebugCrash"),
            "Analysis in environment.json should find manualDebugCrash"
        );
    }

    // 6. Verify mclr-launch.args has masked access token
    {
        let mut args_entry = archive.by_name("mclr-launch.args").unwrap();
        let mut args_content = String::new();
        args_entry.read_to_string(&mut args_content).unwrap();
        assert!(!args_content.contains("secret_access_token_12345"));
        assert!(args_content.contains("--accessToken ***"));
    }

    // 7. Verify watcher-output.log
    {
        let mut watcher_entry = archive.by_name("watcher-output.log").unwrap();
        let mut watcher_content = String::new();
        watcher_entry.read_to_string(&mut watcher_content).unwrap();
        assert!(watcher_content.contains("Watcher line 1"));
        assert!(watcher_content.contains("Watcher line 2"));
    }
}
