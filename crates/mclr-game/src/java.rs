//! Java discovery, scanning, version parsing, and Mojang runtime downloads.

use mclr_core::paths;
use mclr_core::settings::JavaSettings;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Description of an installed Java runtime on the machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JavaInstallation {
    pub path: String,        // path to java.exe
    pub home: String,        // java home directory
    pub version: String,     // full version string e.g. "25.0.3", "1.8.0_402"
    pub major: u32,          // major version e.g. 25, 8
    pub implementor: String, // implementor / vendor
    pub arch: String,        // "x86_64", "x86", etc.
    pub is_jdk: bool,        // true if javac.exe exists
    pub source: String,      // "registry", "path", "env", "defaultPath", "custom", "cache"
}

/// Constraints used when selecting Java for an instance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct JavaConstraint {
    pub min_major: Option<u32>,
    pub max_major: Option<u32>,
    #[serde(default)]
    pub exact_majors: Vec<u32>,
}

/// Parses Java version string to major version number.
/// e.g. "1.8.0_402" -> 8, "17.0.10" -> 17, "25.0.3" -> 25
pub fn parse_major_version(version: &str) -> u32 {
    let clean = version.trim().trim_matches('"');
    let mut parts = clean.split('.');
    if let Some(first) = parts.next() {
        if first == "1" {
            if let Some(second) = parts.next() {
                let minor_str = second.split('_').next().unwrap_or(second);
                return minor_str.parse::<u32>().unwrap_or(8);
            }
        } else {
            let major_str = first.split('_').next().unwrap_or(first);
            return major_str.parse::<u32>().unwrap_or(0);
        }
    }
    0
}

/// Parses the `<home>/release` file content into (version, major, implementor, arch).
pub fn parse_release_content(content: &str) -> Option<(String, u32, String, String)> {
    let mut version = None;
    let mut implementor = None;
    let mut arch = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').to_string();
            match key {
                "JAVA_VERSION" => version = Some(val),
                "IMPLEMENTOR" => implementor = Some(val),
                "OS_ARCH" => arch = Some(val),
                _ => {}
            }
        }
    }

    if let Some(ver) = version {
        let major = parse_major_version(&ver);
        let impl_str = implementor.unwrap_or_else(|| "Unknown".to_string());
        let arch_str = arch.unwrap_or_else(|| "x86_64".to_string());
        Some((ver, major, impl_str, arch_str))
    } else {
        None
    }
}

/// Inspects a `java.exe` path and determines its installation details.
pub fn inspect_java(java_exe: &Path, source: &str) -> Option<JavaInstallation> {
    if !java_exe.exists() {
        return None;
    }

    let bin_dir = java_exe.parent()?;
    let home_dir = bin_dir.parent()?;

    let release_path = home_dir.join("release");
    let is_jdk = bin_dir.join("javac.exe").exists();

    if release_path.exists() {
        if let Ok(content) = fs::read_to_string(&release_path) {
            if let Some((ver, major, implementor, arch)) = parse_release_content(&content) {
                return Some(JavaInstallation {
                    path: java_exe.to_string_lossy().to_string(),
                    home: home_dir.to_string_lossy().to_string(),
                    version: ver,
                    major,
                    implementor,
                    arch,
                    is_jdk,
                    source: source.to_string(),
                });
            }
        }
    }

    // Fallback: execute `<java_exe> -version`
    let mut cmd = Command::new(java_exe);
    cmd.arg("-version");
    cmd.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = cmd.output() {
        let text = String::from_utf8_lossy(&output.stderr).to_string()
            + &String::from_utf8_lossy(&output.stdout);
        // regex match "version \"([^\"]+)\""
        if let Ok(re) = regex::Regex::new(r#"version "([^"]+)""#) {
            if let Some(caps) = re.captures(&text) {
                if let Some(ver_match) = caps.get(1) {
                    let ver = ver_match.as_str().to_string();
                    let major = parse_major_version(&ver);
                    let arch = if text.contains("64-Bit")
                        || text.contains("x86_64")
                        || text.contains("amd64")
                    {
                        "x86_64".to_string()
                    } else {
                        "x86".to_string()
                    };
                    let implementor = if text.contains("GraalVM") {
                        "GraalVM".to_string()
                    } else if text.contains("Zulu") {
                        "Azul Systems".to_string()
                    } else if text.contains("Temurin") || text.contains("Adoptium") {
                        "Eclipse Adoptium".to_string()
                    } else if text.contains("Corretto") {
                        "Amazon Corretto".to_string()
                    } else if text.contains("Liberica") {
                        "BellSoft".to_string()
                    } else if text.contains("Microsoft") {
                        "Microsoft".to_string()
                    } else if text.contains("IBM") || text.contains("Semeru") {
                        "IBM Semeru".to_string()
                    } else {
                        "Unknown".to_string()
                    };

                    return Some(JavaInstallation {
                        path: java_exe.to_string_lossy().to_string(),
                        home: home_dir.to_string_lossy().to_string(),
                        version: ver,
                        major,
                        implementor,
                        arch,
                        is_jdk,
                        source: source.to_string(),
                    });
                }
            }
        }
    }

    None
}

/// Registry scanner on Windows HKLM and HKCU.
fn scan_registry(results: &mut HashMap<String, JavaInstallation>) {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    let hives = [(HKEY_LOCAL_MACHINE, "HKLM"), (HKEY_CURRENT_USER, "HKCU")];

    let prefixes = [
        r"SOFTWARE\JavaSoft\JDK",
        r"SOFTWARE\JavaSoft\JRE",
        r"SOFTWARE\JavaSoft\Java Development Kit",
        r"SOFTWARE\JavaSoft\Java Runtime Environment",
        r"SOFTWARE\Azul Systems\Zulu",
        r"SOFTWARE\Eclipse Adoptium",
        r"SOFTWARE\Eclipse Foundation\JDK",
        r"SOFTWARE\Microsoft\JDK",
        r"SOFTWARE\AdoptOpenJDK",
        r"SOFTWARE\Amazon Corretto",
        r"SOFTWARE\BellSoft\Liberica",
        // WOW6432Node
        r"SOFTWARE\WOW6432Node\JavaSoft\JDK",
        r"SOFTWARE\WOW6432Node\JavaSoft\JRE",
        r"SOFTWARE\WOW6432Node\JavaSoft\Java Development Kit",
        r"SOFTWARE\WOW6432Node\JavaSoft\Java Runtime Environment",
    ];

    for (hive_hkey, _hive_name) in hives {
        let base_hive = RegKey::predef(hive_hkey);
        for prefix in prefixes {
            let key = match base_hive.open_subkey(prefix) {
                Ok(k) => k,
                Err(_) => continue,
            };

            // Enumerate subkeys
            for subkey_name in key.enum_keys().filter_map(|k| k.ok()) {
                if let Ok(subkey) = key.open_subkey(&subkey_name) {
                    // Check JavaHome value
                    let java_home: Result<String, _> = subkey.get_value("JavaHome");
                    if let Ok(home) = java_home {
                        let path = PathBuf::from(&home).join("bin").join("java.exe");
                        if path.exists() {
                            let norm = path.to_string_lossy().to_string();
                            if !results.contains_key(&norm) {
                                if let Some(inst) = inspect_java(&path, "registry") {
                                    results.insert(norm, inst);
                                }
                            }
                        }
                    }

                    // Check Path value
                    let path_val: Result<String, _> = subkey.get_value("Path");
                    if let Ok(p) = path_val {
                        let pbuf = PathBuf::from(&p);
                        let candidate = if pbuf.file_name().is_some_and(|n| n == "java.exe") {
                            pbuf
                        } else {
                            pbuf.join("bin").join("java.exe")
                        };
                        if candidate.exists() {
                            let norm = candidate.to_string_lossy().to_string();
                            if !results.contains_key(&norm) {
                                if let Some(inst) = inspect_java(&candidate, "registry") {
                                    results.insert(norm, inst);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// PATH and environment variables scanner.
fn scan_env(results: &mut HashMap<String, JavaInstallation>) {
    // 1. JAVA_HOME
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let exe = PathBuf::from(&java_home).join("bin").join("java.exe");
        if exe.exists() {
            let norm = exe.to_string_lossy().to_string();
            if !results.contains_key(&norm) {
                if let Some(inst) = inspect_java(&exe, "env") {
                    results.insert(norm, inst);
                }
            }
        }
    }

    // 2. PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for entry in std::env::split_paths(&path_var) {
            let candidate = entry.join("java.exe");
            if candidate.exists() {
                let norm = candidate.to_string_lossy().to_string();
                if !results.contains_key(&norm) {
                    if let Some(inst) = inspect_java(&candidate, "path") {
                        results.insert(norm, inst);
                    }
                }
            }
        }
    }
}

/// Filesystem standard directories scanner.
fn scan_default_paths(results: &mut HashMap<String, JavaInstallation>) {
    let mut check_dirs = Vec::new();

    // Program Files and Program Files (x86)
    for env_var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(pf) = std::env::var(env_var) {
            let pf_path = PathBuf::from(pf);
            for sub in [
                "Java",
                "Eclipse Adoptium",
                "Zulu",
                "Microsoft",
                "Amazon Corretto",
                "BellSoft",
                "Semeru",
            ] {
                check_dirs.push(pf_path.join(sub));
            }
        }
    }

    // Default MC runtime and MCLR runtime
    let default_mc_runtime = paths::default_minecraft_dir().join("runtime");
    check_dirs.push(default_mc_runtime);

    let mclr_runtime = paths::runtime_dir();
    check_dirs.push(mclr_runtime);

    // Also check common developer JDK folders
    for drive_letter in ["C:\\env\\JDK", "C:\\Java", "C:\\JDK"] {
        let p = PathBuf::from(drive_letter);
        if p.exists() {
            check_dirs.push(p);
        }
    }

    for dir in check_dirs {
        if !dir.exists() {
            continue;
        }

        // Search depth 3
        search_java_exe(&dir, 3, results);
    }
}

/// Recursive helper to find `bin/java.exe` up to `max_depth`.
fn search_java_exe(dir: &Path, max_depth: u32, results: &mut HashMap<String, JavaInstallation>) {
    if max_depth == 0 {
        return;
    }

    let direct_bin_java = dir.join("bin").join("java.exe");
    if direct_bin_java.exists() {
        let norm = direct_bin_java.to_string_lossy().to_string();
        if !results.contains_key(&norm) {
            if let Some(inst) = inspect_java(&direct_bin_java, "defaultPath") {
                results.insert(norm, inst);
            }
        }
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if let Ok(ft) = entry.file_type() {
            if ft.is_dir() {
                search_java_exe(&entry.path(), max_depth - 1, results);
            }
        }
    }
}

/// Reads cached Java installations from `<data_dir>/cache/java.json`.
pub fn cached_java() -> Vec<JavaInstallation> {
    read_cache()
}

fn read_cache() -> Vec<JavaInstallation> {
    let cache_file = paths::cache_dir().join("java.json");
    if cache_file.exists() {
        if let Ok(content) = fs::read_to_string(cache_file) {
            if let Ok(list) = serde_json::from_str::<Vec<JavaInstallation>>(&content) {
                return list
                    .into_iter()
                    .filter(|j| Path::new(&j.path).exists())
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Writes cached Java installations to `<data_dir>/cache/java.json`.
fn write_cache(list: &[JavaInstallation]) {
    let cache_dir = paths::cache_dir();
    let _ = fs::create_dir_all(&cache_dir);
    let cache_file = cache_dir.join("java.json");
    if let Ok(json) = serde_json::to_string_pretty(list) {
        let _ = fs::write(cache_file, json);
    }
}

/// Scans the system for Java installations, caching and returning the deduplicated list.
pub fn scan_java(custom_paths: &[String]) -> Vec<JavaInstallation> {
    let mut map: HashMap<String, JavaInstallation> = HashMap::new();

    // 1. Add cached entries if valid
    for cached in read_cache() {
        map.insert(cached.path.clone(), cached);
    }

    // 2. Custom paths from settings
    for custom in custom_paths {
        let p = PathBuf::from(custom);
        let exe = if p.is_dir() {
            p.join("bin").join("java.exe")
        } else {
            p
        };
        if exe.exists() {
            let norm = exe.to_string_lossy().to_string();
            if let Some(inst) = inspect_java(&exe, "custom") {
                map.insert(norm, inst);
            }
        }
    }

    // 3. Registry scan
    scan_registry(&mut map);

    // 4. Environment scan
    scan_env(&mut map);

    // 5. Default filesystem paths
    scan_default_paths(&mut map);

    let mut result: Vec<JavaInstallation> = map.into_values().collect();

    // Sort by major version descending, then is_jdk descending
    result.sort_by(|a, b| {
        b.major
            .cmp(&a.major)
            .then_with(|| b.is_jdk.cmp(&a.is_jdk))
            .then_with(|| a.path.cmp(&b.path))
    });

    write_cache(&result);

    result
}

/// Selects the best installed Java for a version requirement.
///
/// There is no global "default Java": the choice is derived from the version's own
/// requirement. Ranking, in order:
/// 1. closest to the requirement — an exact major match wins, otherwise the **smallest**
///    major above it (a Java 17 instance must not be handed a Java 25 runtime);
/// 2. 64-bit over 32-bit;
/// 3. preferred JDK/JRE kind;
/// 4. newest build of the same major.
///
/// Installations below the requirement are never candidates.
pub fn select_java(
    list: &[JavaInstallation],
    constraint: &JavaConstraint,
    settings: &JavaSettings,
) -> Option<JavaInstallation> {
    let target = constraint.min_major.or(constraint.max_major);

    let mut filtered: Vec<&JavaInstallation> = list
        .iter()
        .filter(|j| {
            if !constraint.exact_majors.is_empty() && !constraint.exact_majors.contains(&j.major) {
                return false;
            }
            if let Some(min) = constraint.min_major
                && j.major < min
            {
                return false;
            }
            if let Some(max) = constraint.max_major
                && j.major > max
            {
                return false;
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        return None;
    }

    filtered.sort_by(|a, b| {
        let distance = |j: &JavaInstallation| match target {
            Some(t) => (j.major as i64 - t as i64).abs(),
            None => 0,
        };
        let a_exact = target.is_some_and(|t| a.major == t);
        let b_exact = target.is_some_and(|t| b.major == t);
        let a_64 = a.arch.contains("64");
        let b_64 = b.arch.contains("64");

        b_exact
            .cmp(&a_exact)
            .then_with(|| distance(a).cmp(&distance(b)))
            .then_with(|| b_64.cmp(&a_64))
            .then_with(|| (b.is_jdk == settings.prefer_jdk).cmp(&(a.is_jdk == settings.prefer_jdk)))
            .then_with(|| b.version.cmp(&a.version))
            .then_with(|| a.path.cmp(&b.path))
    });

    filtered.first().copied().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_major_version() {
        assert_eq!(parse_major_version("25.0.3"), 25);
        assert_eq!(parse_major_version("17.0.10"), 17);
        assert_eq!(parse_major_version("1.8.0_402"), 8);
        assert_eq!(parse_major_version("1.7.0_80"), 7);
        assert_eq!(parse_major_version("21"), 21);
    }

    #[test]
    fn test_parse_release_content() {
        let content = r#"
JAVA_VERSION="25.0.3"
IMPLEMENTOR="Oracle Corporation"
OS_ARCH="x86_64"
OS_NAME="Windows"
"#;
        let (ver, major, implementor, arch) = parse_release_content(content).unwrap();
        assert_eq!(ver, "25.0.3");
        assert_eq!(major, 25);
        assert_eq!(implementor, "Oracle Corporation");
        assert_eq!(arch, "x86_64");

        let content_legacy = r#"
JAVA_VERSION="1.8.0_312"
IMPLEMENTOR="Eclipse Adoptium"
OS_ARCH="x86_64"
"#;
        let (ver8, major8, implementor8, _) = parse_release_content(content_legacy).unwrap();
        assert_eq!(ver8, "1.8.0_312");
        assert_eq!(major8, 8);
        assert_eq!(implementor8, "Eclipse Adoptium");
    }

    #[test]
    fn test_select_java() {
        let j8 = JavaInstallation {
            path: "C:\\Java\\jdk8\\bin\\java.exe".to_string(),
            home: "C:\\Java\\jdk8".to_string(),
            version: "1.8.0_402".to_string(),
            major: 8,
            implementor: "Adoptium".to_string(),
            arch: "x86_64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        };

        let j17 = JavaInstallation {
            path: "C:\\Java\\jdk17\\bin\\java.exe".to_string(),
            home: "C:\\Java\\jdk17".to_string(),
            version: "17.0.10".to_string(),
            major: 17,
            implementor: "Microsoft".to_string(),
            arch: "x86_64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        };

        let j25 = JavaInstallation {
            path: "C:\\Java\\jdk25\\bin\\java.exe".to_string(),
            home: "C:\\Java\\jdk25".to_string(),
            version: "25.0.3".to_string(),
            major: 25,
            implementor: "Oracle Corporation".to_string(),
            arch: "x86_64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        };

        let list = vec![j8.clone(), j17.clone(), j25.clone()];

        let settings = JavaSettings::default();

        // Exact match 17
        let c17 = JavaConstraint {
            min_major: None,
            max_major: None,
            exact_majors: vec![17],
        };
        let sel = select_java(&list, &c17, &settings).unwrap();
        assert_eq!(sel.major, 17);

        // Min major 17: the closest match wins, so 17 — not the newest (25).
        // Handing a Java 17 instance a Java 25 runtime breaks agents and old mixins.
        let c_min17 = JavaConstraint {
            min_major: Some(17),
            max_major: None,
            exact_majors: vec![],
        };
        let sel = select_java(&list, &c_min17, &settings).unwrap();
        assert_eq!(sel.major, 17);

        // Only newer runtimes available: the smallest one above the requirement wins.
        let newer_only = vec![j25.clone(), {
            let mut j = j25.clone();
            j.major = 30;
            j.version = "30.0.1".to_string();
            j.path = "C:\\Java\\jdk30\\bin\\java.exe".to_string();
            j.home = "C:\\Java\\jdk30".to_string();
            j
        }];
        let sel = select_java(&newer_only, &c_min17, &settings).unwrap();
        assert_eq!(sel.major, 25, "Java 25 is closer to 17 than Java 30");

        // A lone installation must still honour the constraint: Java 21 cannot run MC 26.x
        // (previously the single-entry shortcut returned it anyway and the JVM threw
        // UnsupportedClassVersionError at startup).
        let only21 = vec![JavaInstallation {
            path: "C:\\Java\\jdk21\\bin\\java.exe".to_string(),
            home: "C:\\Java\\jdk21".to_string(),
            version: "21.0.5".to_string(),
            major: 21,
            implementor: "Microsoft".to_string(),
            arch: "x86_64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        }];
        let c_min25 = JavaConstraint {
            min_major: Some(25),
            max_major: None,
            exact_majors: vec![],
        };
        assert!(
            select_java(&only21, &c_min25, &settings).is_none(),
            "a single Java 21 must not satisfy a Java 25+ requirement"
        );

        // The same lone installation is still used when it does satisfy the requirement.
        let sel = select_java(&only21, &c_min17, &settings).unwrap();
        assert_eq!(sel.major, 21);

        // Max major 8
        let c_max8 = JavaConstraint {
            min_major: None,
            max_major: Some(8),
            exact_majors: vec![],
        };
        let sel = select_java(&list, &c_max8, &settings).unwrap();
        assert_eq!(sel.major, 8);
    }

    #[test]
    fn test_system_java_discovered() {
        let scanned = scan_java(&[]);
        assert!(
            !scanned.is_empty(),
            "Expected at least one Java installation to be found on system"
        );
        assert!(
            scanned
                .iter()
                .any(|j| j.major >= 17 || j.major == 8 || j.major == 25)
        );
    }
}
