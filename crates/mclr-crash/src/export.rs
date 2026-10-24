//! Crash report zip packager.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::json;
use zip::write::{SimpleFileOptions, ZipWriter};

use crate::collector::collect_crash_logs;
use crate::error::CrashError;
use crate::preparer::prepare_text;
use crate::rules::analyze_crash;

/// Packages a full diagnostic zip containing launch debug script, environment info,
/// mod inventory with JarInJar scan, watcher output, and log files.
pub fn export_crash_report_zip(
    instance_dir: &Path,
    instance_id: &str,
    watcher_lines: &[String],
    dest_zip: &Path,
) -> Result<PathBuf, CrashError> {
    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(dest_zip)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. Collect logs and run crash analysis
    let logs = collect_crash_logs(instance_dir, None, watcher_lines, None)?;
    let analysis = analyze_crash(instance_id, instance_dir, &logs);

    // 2. launch-debug.bat
    let launch_script = generate_debug_launch_script(instance_dir, instance_id);
    zip.start_file("launch-debug.bat", options)?;
    zip.write_all(launch_script.as_bytes())?;

    // Also include mclr-launch.args if present
    let args_file = instance_dir.join("mclr-launch.args");
    if args_file.exists()
        && let Ok(content) = fs::read_to_string(&args_file)
    {
        let sanitized = prepare_text(&content);
        zip.start_file("mclr-launch.args", options)?;
        zip.write_all(sanitized.as_bytes())?;
    }

    // 3. environment.json
    let env_json = json!({
        "instanceId": instance_id,
        "instanceDir": instance_dir.to_string_lossy(),
        "exportedAt": Utc::now().to_rfc3339(),
        "os": {
            "family": std::env::consts::FAMILY,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
        },
        "launcher": {
            "name": "MCLR",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "analysis": analysis,
    });
    let env_text = serde_json::to_string_pretty(&env_json)?;
    zip.start_file("environment.json", options)?;
    zip.write_all(env_text.as_bytes())?;

    // 4. mods.json (local mods scan + JarInJar)
    let mods_dir = instance_dir.join("mods");
    let mut mods_list = Vec::new();
    if mods_dir.exists()
        && let Ok(scanned) = mclr_comp::scan_local_mods(&mods_dir)
    {
        for m in scanned {
            let jar_path = mods_dir.join(&m.file_name);
            let jar_in_jar = mclr_comp::scan_jar_in_jar(&jar_path).unwrap_or_default();
            mods_list.push(json!({
                "fileName": m.file_name,
                "modId": m.mod_id,
                "name": m.name,
                "version": m.version,
                "enabled": m.enabled,
                "jarInJar": jar_in_jar,
            }));
        }
    }
    let mods_json_text = serde_json::to_string_pretty(&mods_list)?;
    zip.start_file("mods.json", options)?;
    zip.write_all(mods_json_text.as_bytes())?;

    // 5. watcher-output.log (last 200 lines)
    let watcher_tail: Vec<String> = watcher_lines
        .iter()
        .rev()
        .take(200)
        .rev()
        .cloned()
        .collect();
    let watcher_content = prepare_text(&watcher_tail.join("\n"));
    zip.start_file("watcher-output.log", options)?;
    zip.write_all(watcher_content.as_bytes())?;

    // 6. Collected crash-reports/*
    let crash_reports_dir = instance_dir.join("crash-reports");
    if crash_reports_dir.exists()
        && let Ok(entries) = fs::read_dir(&crash_reports_dir)
    {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && let Some(file_name) = p.file_name()
                && let Ok(content) = fs::read_to_string(&p)
            {
                let name = file_name.to_string_lossy();
                let prepared = prepare_text(&content);
                zip.start_file(format!("crash-reports/{name}"), options)?;
                zip.write_all(prepared.as_bytes())?;
            }
        }
    }

    // 7. logs/latest.log and logs/debug.log
    let latest_path = instance_dir.join("logs").join("latest.log");
    if latest_path.exists()
        && let Ok(content) = fs::read_to_string(&latest_path)
    {
        let prepared = prepare_text(&content);
        zip.start_file("logs/latest.log", options)?;
        zip.write_all(prepared.as_bytes())?;
    }
    let debug_path = instance_dir.join("logs").join("debug.log");
    if debug_path.exists()
        && let Ok(content) = fs::read_to_string(&debug_path)
    {
        let prepared = prepare_text(&content);
        zip.start_file("logs/debug.log", options)?;
        zip.write_all(prepared.as_bytes())?;
    }

    // 8. hs_err_pid*.log
    if let Ok(entries) = fs::read_dir(instance_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file()
                && let Some(name) = p.file_name()
            {
                let name_str = name.to_string_lossy();
                if name_str.starts_with("hs_err_pid")
                    && name_str.ends_with(".log")
                    && let Ok(content) = fs::read_to_string(&p)
                {
                    let prepared = prepare_text(&content);
                    zip.start_file(&name_str, options)?;
                    zip.write_all(prepared.as_bytes())?;
                }
            }
        }
    }

    zip.finish()?;
    Ok(dest_zip.to_path_buf())
}

fn generate_debug_launch_script(instance_dir: &Path, instance_id: &str) -> String {
    let script_path = instance_dir.join("mclr-launch.bat");
    if script_path.exists()
        && let Ok(content) = fs::read_to_string(&script_path)
    {
        return prepare_text(&content);
    }

    let has_argfile = instance_dir.join("mclr-launch.args").exists();
    if has_argfile {
        format!(
            "@echo off\r\n\
            chcp 65001 > nul\r\n\
            echo ========================================================\r\n\
            echo  MCLR Reproducible Launch Debug Script\r\n\
            echo  Instance: {instance_id}\r\n\
            echo ========================================================\r\n\
            cd /d \"%~dp0\"\r\n\
            java @mclr-launch.args\r\n\
            pause\r\n"
        )
    } else {
        format!(
            "@echo off\r\n\
            chcp 65001 > nul\r\n\
            echo ========================================================\r\n\
            echo  MCLR Diagnostic Launch Script\r\n\
            echo  Instance: {instance_id}\r\n\
            echo ========================================================\r\n\
            cd /d \"%~dp0\"\r\n\
            rem Launch parameters not recorded in standalone batch file\r\n\
            pause\r\n"
        )
    }
}
