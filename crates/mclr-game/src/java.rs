//! Java discovery, scanning, version parsing, and Mojang runtime downloads.

use crate::error::GameError;
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

/// File specification for downloading Mojang Java runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeFileSpec {
    pub url: String,
    pub dest: PathBuf,
    pub size: u64,
    pub sha1: String,
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

/// Selects the best Java installation matching constraints and settings.
pub fn select_java(
    list: &[JavaInstallation],
    constraint: &JavaConstraint,
    settings: &JavaSettings,
) -> Option<JavaInstallation> {
    // If user specified custom explicit path in settings
    if settings.select != "auto" && !settings.select.is_empty() {
        let sel_norm = settings.select.replace('/', "\\").to_lowercase();
        if let Some(found) = list.iter().find(|j| {
            let p_norm = j.path.replace('/', "\\").to_lowercase();
            let h_norm = j.home.replace('/', "\\").to_lowercase();
            p_norm == sel_norm || h_norm == sel_norm
        }) {
            return Some(found.clone());
        }
    }

    // Filter by constraints
    let filtered: Vec<&JavaInstallation> = list
        .iter()
        .filter(|j| {
            // Check exact majors if specified
            if !constraint.exact_majors.is_empty() && !constraint.exact_majors.contains(&j.major) {
                return false;
            }

            // Check min major
            if let Some(min) = constraint.min_major {
                if j.major < min {
                    return false;
                }
            }

            // Check max major
            if let Some(max) = constraint.max_major {
                if j.major > max {
                    return false;
                }
            }

            true
        })
        .collect();

    if filtered.is_empty() {
        return None;
    }

    // Sort preference:
    // 1. prefer_jdk match
    // 2. 64-bit arch
    // 3. major version (highest or exact)
    let mut sorted = filtered;
    sorted.sort_by(|a, b| {
        let a_jdk = a.is_jdk == settings.prefer_jdk;
        let b_jdk = b.is_jdk == settings.prefer_jdk;

        let a_64 = a.arch.contains("64");
        let b_64 = b.arch.contains("64");

        b_jdk
            .cmp(&a_jdk)
            .then_with(|| b_64.cmp(&a_64))
            .then_with(|| b.major.cmp(&a.major))
            .then_with(|| b.version.cmp(&a.version))
    });

    sorted.first().copied().cloned()
}

/// Manifest structure for Mojang Java runtime.
#[derive(Debug, Deserialize)]
struct MojangAllManifest {
    #[serde(rename = "windows-x64")]
    windows_x64: Option<HashMap<String, Vec<MojangComponentEntry>>>,
}

#[derive(Debug, Deserialize)]
struct MojangComponentEntry {
    manifest: MojangComponentManifestInfo,
}

#[derive(Debug, Deserialize)]
struct MojangComponentManifestInfo {
    url: String,
}

#[derive(Debug, Deserialize)]
struct MojangFileManifest {
    files: HashMap<String, MojangFileEntry>,
}

#[derive(Debug, Deserialize)]
struct MojangFileEntry {
    #[serde(rename = "type")]
    entry_type: String, // "file" or "directory"
    downloads: Option<MojangFileDownloads>,
}

#[derive(Debug, Deserialize)]
struct MojangFileDownloads {
    raw: Option<MojangFileRaw>,
}

#[derive(Debug, Deserialize)]
struct MojangFileRaw {
    sha1: String,
    size: u64,
    url: String,
}

/// Resolves Mojang runtime download file specifications for a given component
/// (e.g. `java-runtime-gamma`, `java-runtime-alpha`, `jre-legacy`).
pub async fn resolve_mojang_runtime_downloads(
    client: &reqwest::Client,
    component: &str,
    dest_dir: &Path,
) -> Result<Vec<RuntimeFileSpec>, GameError> {
    const ALL_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

    let resp = client
        .get(ALL_URL)
        .send()
        .await
        .map_err(|e| GameError::RuntimeManifest(e.to_string()))?;

    let all: MojangAllManifest = resp
        .json()
        .await
        .map_err(|e| GameError::RuntimeManifest(e.to_string()))?;

    let win_x64 = all
        .windows_x64
        .ok_or_else(|| GameError::RuntimeManifest("Missing windows-x64 in all.json".to_string()))?;

    let entries = win_x64.get(component).ok_or_else(|| {
        GameError::RuntimeManifest(format!("Component {component} not found for windows-x64"))
    })?;

    let entry = entries
        .first()
        .ok_or_else(|| GameError::RuntimeManifest(format!("No entries for {component}")))?;

    // Fetch component file manifest
    let manifest_resp = client
        .get(&entry.manifest.url)
        .send()
        .await
        .map_err(|e| GameError::RuntimeManifest(e.to_string()))?;

    let file_manifest: MojangFileManifest = manifest_resp
        .json()
        .await
        .map_err(|e| GameError::RuntimeManifest(e.to_string()))?;

    let mut specs = Vec::new();

    for (rel_path, item) in file_manifest.files {
        if item.entry_type == "file" {
            if let Some(dl) = item.downloads {
                if let Some(raw) = dl.raw {
                    let dest = dest_dir.join(rel_path.replace('/', "\\"));
                    specs.push(RuntimeFileSpec {
                        url: raw.url,
                        dest,
                        size: raw.size,
                        sha1: raw.sha1,
                    });
                }
            }
        }
    }

    Ok(specs)
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

        // Min major 17 (prefers highest, so 25)
        let c_min17 = JavaConstraint {
            min_major: Some(17),
            max_major: None,
            exact_majors: vec![],
        };
        let sel = select_java(&list, &c_min17, &settings).unwrap();
        assert_eq!(sel.major, 25);

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
