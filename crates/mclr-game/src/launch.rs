//! Minecraft launch pipeline, arguments builder, natives extraction, and process watcher.

use crate::error::GameError;
use crate::install::{load_resolved_version_json, maven_coordinate_to_path, repair_instance};
use crate::instance::{InstanceConfig, extract_mc_version};
use crate::java::{JavaConstraint, JavaInstallation, inspect_java, scan_java, select_java};
use crate::version::{ArgumentEntry, RuleContext, VersionJson, evaluate_rules};
use mclr_core::account_session::{AccountKind, ResolvedAccountSession};
use mclr_core::paths;
use mclr_core::settings::{GameSettings, LaunchSettings, NetSettings, Settings};
use mclr_core::tasks::TaskCenter;
use mclr_net::{DownloadSpec, Downloader};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};
use url::Url;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, SetWindowTextW,
};
use windows::core::PCWSTR;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const MAX_LOG_LINES: usize = 5000;
const COMMAND_LINE_LIMIT: usize = 8000;

/// Log severity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
    Fatal,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Fatal => write!(f, "FATAL"),
        }
    }
}

/// A single line of Minecraft stdout or stderr log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub line_id: u64,
    pub process_id: u32,
    pub instance_id: String,
    pub level: LogLevel,
    pub text: String,
    pub timestamp: String,
}

/// Runtime metadata for a launched Minecraft process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LaunchProcessInfo {
    pub process_id: u32,
    pub instance_id: String,
    pub account_name: String,
    pub started_at: String,
    pub running: bool,
    pub exit_code: Option<i32>,
}

/// Result returned to IPC callers immediately upon launching.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LaunchStartResult {
    pub task_id: String,
    pub process_id: Option<u32>,
}

/// Event payload emitted on process exit or crash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LaunchExitPayload {
    pub process_id: u32,
    pub instance_id: String,
    pub exit_code: i32,
}

/// Internal broadcast events emitted by the launch watcher.
#[derive(Debug, Clone)]
pub enum WatcherEvent {
    Started(LaunchProcessInfo),
    Log(LogLine),
    Exited(LaunchExitPayload),
    Crashed(LaunchExitPayload),
}

/// Complete launch command specification prepared before spawning.
#[derive(Debug, Clone)]
pub struct LaunchCommandSpec {
    pub java_executable: PathBuf,
    pub java_home: PathBuf,
    pub game_dir: PathBuf,
    pub args: Vec<String>,
    pub used_argfile: bool,
    pub argfile_path: Option<PathBuf>,
}

/// Global registry tracking active processes and log history.
pub struct LaunchWatcher {
    processes: RwLock<HashMap<u32, LaunchProcessInfo>>,
    logs: RwLock<VecDeque<LogLine>>,
    line_id_counter: AtomicU64,
    event_tx: broadcast::Sender<WatcherEvent>,
}

impl Default for LaunchWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl LaunchWatcher {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            processes: RwLock::new(HashMap::new()),
            logs: RwLock::new(VecDeque::with_capacity(MAX_LOG_LINES)),
            line_id_counter: AtomicU64::new(1),
            event_tx,
        }
    }

    /// Subscribes to watcher events (started, log lines, exited, crashed).
    pub fn subscribe(&self) -> broadcast::Receiver<WatcherEvent> {
        self.event_tx.subscribe()
    }

    /// Returns a snapshot of all tracked processes.
    pub async fn list_processes(&self) -> Vec<LaunchProcessInfo> {
        let procs = self.processes.read().await;
        let mut list: Vec<_> = procs.values().cloned().collect();
        list.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        list
    }

    /// Returns log history, optionally filtered by instance id.
    pub async fn get_logs(&self, instance_id: Option<&str>) -> Vec<LogLine> {
        let logs = self.logs.read().await;
        if let Some(target) = instance_id {
            logs.iter()
                .filter(|l| l.instance_id == target)
                .cloned()
                .collect()
        } else {
            logs.iter().cloned().collect()
        }
    }

    /// Kills a process by PID.
    pub async fn kill(&self, process_id: u32) -> Result<(), GameError> {
        let mut killed = false;
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, process_id) {
                if TerminateProcess(handle, 1).is_ok() {
                    killed = true;
                }
                let _ = windows::Win32::Foundation::CloseHandle(handle);
            }
        }

        if !killed {
            let status = std::process::Command::new("taskkill")
                .args(["/F", "/PID", &process_id.to_string()])
                .creation_flags(CREATE_NO_WINDOW)
                .status();
            if status.is_ok() {
                killed = true;
            }
        }

        let mut procs = self.processes.write().await;
        if let Some(p) = procs.get_mut(&process_id) {
            p.running = false;
            p.exit_code = Some(-1);
        }

        if killed {
            Ok(())
        } else {
            Err(GameError::LaunchProcessNotFound(process_id))
        }
    }

    /// Registers a newly started process and broadcasts the `Started` event.
    pub async fn register_process(&self, info: LaunchProcessInfo) {
        {
            let mut procs = self.processes.write().await;
            procs.insert(info.process_id, info.clone());
        }
        let _ = self.event_tx.send(WatcherEvent::Started(info));
    }

    /// Appends a log line to the ring buffer and broadcasts the `Log` event.
    pub async fn append_log(&self, line: LogLine) {
        {
            let mut logs = self.logs.write().await;
            if logs.len() >= MAX_LOG_LINES {
                logs.pop_front();
            }
            logs.push_back(line.clone());
        }
        let _ = self.event_tx.send(WatcherEvent::Log(line));
    }

    /// Marks a process as exited and broadcasts `Exited` and optionally `Crashed`.
    pub async fn mark_exited(&self, process_id: u32, instance_id: &str, exit_code: i32) {
        {
            let mut procs = self.processes.write().await;
            if let Some(p) = procs.get_mut(&process_id) {
                p.running = false;
                p.exit_code = Some(exit_code);
            }
        }

        let payload = LaunchExitPayload {
            process_id,
            instance_id: instance_id.to_string(),
            exit_code,
        };

        if exit_code != 0 {
            let _ = self.event_tx.send(WatcherEvent::Crashed(payload.clone()));
        }
        let _ = self.event_tx.send(WatcherEvent::Exited(payload));
    }
}

/// Precheck: rejects game folder or instance path containing `!` or `;`.
pub fn check_launch_paths(mc_folder: &Path, instance_id: &str) -> Result<(), GameError> {
    let mc_str = mc_folder.to_string_lossy();
    if mc_str.contains('!') || mc_str.contains(';') {
        return Err(GameError::LaunchInvalidPath(format!(
            "Game folder path contains '!' or ';': {mc_str}"
        )));
    }
    if instance_id.contains('!') || instance_id.contains(';') {
        return Err(GameError::LaunchInvalidPath(format!(
            "Instance ID contains '!' or ';': {instance_id}"
        )));
    }
    Ok(())
}

/// Precheck: enforces `instance.config.login_requirement`.
pub fn check_login_requirement(
    requirement: &str,
    account_kind: AccountKind,
) -> Result<(), GameError> {
    let satisfied = match requirement {
        "any" | "" => true,
        "microsoft" => account_kind == AccountKind::Microsoft,
        "thirdParty" => {
            account_kind == AccountKind::Authlib || account_kind == AccountKind::YggdrasilConnect
        }
        "microsoftOrThirdParty" => account_kind != AccountKind::Offline,
        _ => true,
    };

    if !satisfied {
        let actual_str = match account_kind {
            AccountKind::Microsoft => "microsoft",
            AccountKind::Offline => "offline",
            AccountKind::Authlib => "thirdParty",
            AccountKind::YggdrasilConnect => "thirdParty",
        };
        return Err(GameError::LaunchLoginRequirement {
            required: requirement.to_string(),
            actual: actual_str.to_string(),
        });
    }

    Ok(())
}

/// Helper to parse Minecraft minor and patch version numbers from version string.
/// E.g. "1.20.5" -> (20, 5), "1.18" -> (18, 0), "1.5.2" -> (5, 2).
pub fn parse_mc_minor_patch(mc_version: &str) -> (u32, u32) {
    let clean = mc_version.trim();
    if let Some(rest) = clean.strip_prefix("1.") {
        let mut parts = rest.split('.');
        if let Some(minor_part) = parts.next() {
            let minor_num: String = minor_part
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            let minor = minor_num.parse::<u32>().unwrap_or(0);
            let patch = if let Some(patch_part) = parts.next() {
                let patch_num: String = patch_part
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                patch_num.parse::<u32>().unwrap_or(0)
            } else {
                0
            };
            return (minor, patch);
        }
    }

    // Modern snapshot support like "24w14a" -> minor 21
    if clean.len() >= 3 && clean.chars().take(2).all(|c| c.is_ascii_digit()) {
        let year: u32 = clean[..2].parse().unwrap_or(0);
        if year >= 24 {
            return (21, 0);
        } else if year >= 22 {
            return (19, 0);
        }
    }

    (0, 0)
}

/// Resolves Java version constraints from version JSON and Minecraft version string.
pub fn version_java_constraint(version_json: &VersionJson, mc_version: &str) -> JavaConstraint {
    if let Some(jv) = &version_json.java_version {
        if jv.major_version > 0 {
            return JavaConstraint {
                min_major: Some(jv.major_version),
                max_major: None,
                exact_majors: Vec::new(),
            };
        }
    }

    let (minor, patch) = parse_mc_minor_patch(mc_version);

    if minor > 20 || (minor == 20 && patch >= 5) {
        JavaConstraint {
            min_major: Some(21),
            max_major: None,
            exact_majors: Vec::new(),
        }
    } else if minor >= 18 {
        JavaConstraint {
            min_major: Some(17),
            max_major: None,
            exact_majors: Vec::new(),
        }
    } else if minor == 17 {
        JavaConstraint {
            min_major: Some(16),
            max_major: None,
            exact_majors: Vec::new(),
        }
    } else if minor >= 12 {
        JavaConstraint {
            min_major: Some(8),
            max_major: None,
            exact_majors: Vec::new(),
        }
    } else if minor <= 5 && minor > 0 {
        JavaConstraint {
            min_major: None,
            max_major: Some(8),
            exact_majors: Vec::new(),
        }
    } else {
        JavaConstraint {
            min_major: Some(8),
            max_major: None,
            exact_majors: Vec::new(),
        }
    }
}

/// Resolves the Java installation to use, checking per-instance override first, then settings.
pub fn resolve_java_installation(
    installed_list: &[JavaInstallation],
    constraint: &JavaConstraint,
    settings: &Settings,
    instance_config: &InstanceConfig,
) -> Result<JavaInstallation, GameError> {
    // 1. Per-instance override
    if let Some(custom) = &instance_config.java {
        let trimmed = custom.trim();
        if !trimmed.is_empty() && trimmed != "auto" {
            let custom_p = PathBuf::from(trimmed);
            let exe = if custom_p.is_dir() {
                custom_p.join("bin").join("java.exe")
            } else {
                custom_p
            };
            if exe.exists() {
                if let Some(inst) = inspect_java(&exe, "instanceCustom") {
                    return Ok(inst);
                }
            }
            let sel_norm = trimmed.replace('/', "\\").to_lowercase();
            if let Some(found) = installed_list.iter().find(|j| {
                j.path.replace('/', "\\").to_lowercase() == sel_norm
                    || j.home.replace('/', "\\").to_lowercase() == sel_norm
            }) {
                return Ok(found.clone());
            }
        }
    }

    // 2. Settings selection
    if let Some(found) = select_java(installed_list, constraint, &settings.java) {
        return Ok(found);
    }

    let major = constraint.min_major.or(constraint.max_major).unwrap_or(8);
    Err(GameError::LaunchJavaNotFound { major })
}

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

/// Extracts native DLLs into `<mc_folder>/versions/<instance_id>/<instance_id>-natives/`.
pub fn extract_natives(
    mc_folder: &Path,
    instance_id: &str,
    version_json: &VersionJson,
) -> Result<PathBuf, GameError> {
    let natives_dir = mc_folder
        .join("versions")
        .join(instance_id)
        .join(format!("{instance_id}-natives"));
    fs::create_dir_all(&natives_dir)?;

    let rule_ctx = RuleContext::windows_default();
    let mut current_dlls = HashSet::new();

    for lib in &version_json.libraries {
        if !evaluate_rules(&lib.rules, &rule_ctx) {
            continue;
        }

        // Determine if this library contains Windows natives
        let is_modern_native = lib.name.contains(":natives-windows")
            && !lib.name.contains("arm64")
            && !lib.name.contains("x86");

        let has_classifier_native = lib
            .downloads
            .as_ref()
            .and_then(|d| d.classifiers.get("natives-windows"))
            .is_some()
            || lib.natives.contains_key("windows");

        if !is_modern_native && !has_classifier_native {
            continue;
        }

        // Find native jar path
        let jar_path = if let Some(artifact) = lib
            .downloads
            .as_ref()
            .and_then(|d| d.classifiers.get("natives-windows"))
        {
            if let Some(p) = &artifact.path {
                mc_folder
                    .join("libraries")
                    .join(p.replace('/', std::path::MAIN_SEPARATOR_STR))
            } else {
                let rel = maven_coordinate_to_path(&lib.name);
                mc_folder
                    .join("libraries")
                    .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
            }
        } else if let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            if let Some(p) = &artifact.path {
                mc_folder
                    .join("libraries")
                    .join(p.replace('/', std::path::MAIN_SEPARATOR_STR))
            } else {
                let rel = maven_coordinate_to_path(&lib.name);
                mc_folder
                    .join("libraries")
                    .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
            }
        } else {
            let rel = maven_coordinate_to_path(&lib.name);
            mc_folder
                .join("libraries")
                .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
        };

        if !jar_path.exists() {
            continue;
        }

        let file = match fs::File::open(&jar_path) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to open native jar {}: {e}", jar_path.display());
                continue;
            }
        };

        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => {
                warn!("Failed to read zip in {}: {e}", jar_path.display());
                continue;
            }
        };

        for i in 0..archive.len() {
            let mut entry = match archive.by_index(i) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_name = entry.name().to_string();
            if !entry_name.to_lowercase().ends_with(".dll") || entry_name.contains("META-INF/") {
                continue;
            }

            if let Some(extract) = &lib.extract {
                if extract.exclude.iter().any(|ex| entry_name.starts_with(ex)) {
                    continue;
                }
            }

            let file_name = match Path::new(&entry_name).file_name() {
                Some(f) => f.to_string_lossy().to_string(),
                None => continue,
            };

            current_dlls.insert(file_name.to_lowercase());
            let dest_file = natives_dir.join(&file_name);

            // Skip extracting if identical size already exists
            if dest_file.exists() {
                if let Ok(meta) = fs::metadata(&dest_file) {
                    if meta.len() == entry.size() {
                        continue;
                    }
                }
            }

            let mut out = match fs::File::create(&dest_file) {
                Ok(f) => f,
                Err(e) => {
                    warn!("Failed to create DLL file {}: {e}", dest_file.display());
                    continue;
                }
            };
            let _ = std::io::copy(&mut entry, &mut out);
        }
    }

    // Purge stale DLLs
    if let Ok(entries) = fs::read_dir(&natives_dir) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    let path = entry.path();
                    if path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
                    {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if !current_dlls.contains(&name.to_lowercase()) {
                                let _ = fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(natives_dir)
}

#[derive(Deserialize)]
struct AuthlibArtifactResponse {
    download_url: String,
    checksums: HashMap<String, String>,
}

/// Ensures `tools/authlib-injector.jar` exists, downloading and verifying SHA256 if missing.
pub async fn ensure_authlib_injector(
    client: &reqwest::Client,
    net_settings: &NetSettings,
) -> Result<PathBuf, GameError> {
    let tools_dir = paths::tools_dir();
    fs::create_dir_all(&tools_dir)?;
    let target = tools_dir.join("authlib-injector.jar");

    if target.exists() {
        return Ok(target);
    }

    info!("Downloading authlib-injector.jar...");
    let urls = [
        "https://bmclapi2.bangbang93.com/mirrors/authlib-injector/artifact/latest.json",
        "https://authlib-injector.yushi.moe/artifact/latest.json",
    ];

    let mut response_meta: Option<AuthlibArtifactResponse> = None;
    for url in urls {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                if let Ok(meta) = resp.json::<AuthlibArtifactResponse>().await {
                    response_meta = Some(meta);
                    break;
                }
            }
        }
    }

    let meta = response_meta.ok_or_else(|| {
        GameError::InstallFailed("Failed to fetch authlib-injector artifact metadata".to_string())
    })?;

    let download_url = Url::parse(&meta.download_url).map_err(|e| {
        GameError::InstallFailed(format!("Invalid authlib-injector download url: {e}"))
    })?;

    let sha256 = meta.checksums.get("sha256").cloned();
    let spec = DownloadSpec {
        urls: vec![download_url],
        dest: target.clone(),
        size: None,
        sha1: None,
        sha256,
        md5: None,
    };

    let downloader = Downloader::new(client.clone(), net_settings.clone());
    downloader.download(spec, None).await.map_err(|e| {
        GameError::InstallFailed(format!("Failed to download authlib-injector: {e}"))
    })?;

    Ok(target)
}

/// Helper substituting placeholders in launch argument strings.
pub fn substitute_placeholders(arg: &str, subs: &HashMap<&str, String>) -> String {
    let mut result = arg.to_string();
    for (key, val) in subs {
        if result.contains(key) {
            result = result.replace(key, val);
        }
    }
    result
}

/// Builds the full command line arguments, handles placeholder substitution,
/// and applies `@argfile` if on Java >= 9 and joined length > 8000 characters.
#[allow(clippy::too_many_arguments)]
pub fn build_launch_command(
    mc_folder: &Path,
    instance_id: &str,
    version_json: &VersionJson,
    java: &JavaInstallation,
    launch_settings: &LaunchSettings,
    game_settings: &GameSettings,
    instance_config: &InstanceConfig,
    session: &ResolvedAccountSession,
    natives_dir: &Path,
    authlib_injector_jar: Option<&Path>,
) -> Result<LaunchCommandSpec, GameError> {
    let instance_dir = mc_folder.join("versions").join(instance_id);

    // 1. Determine isolated game directory
    let game_dir = if instance_config.isolation == "on"
        || (instance_config.isolation == "global" && game_settings.default_isolation == "on")
    {
        instance_dir.clone()
    } else {
        mc_folder.to_path_buf()
    };
    fs::create_dir_all(&game_dir)?;

    // 2. Ensure launcher_profiles.json exists
    let launcher_profiles = mc_folder.join("launcher_profiles.json");
    if !launcher_profiles.exists() {
        let _ = fs::write(&launcher_profiles, r#"{"profiles":{}}"#);
    }

    let rule_ctx = RuleContext::windows_default();

    // 3. JVM Arguments
    let mut jvm_args: Vec<String> = Vec::new();

    // Custom JVM args
    let custom_jvm = instance_config
        .jvm_args
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&launch_settings.jvm_args);
    if !custom_jvm.trim().is_empty() {
        jvm_args.extend(custom_jvm.split_whitespace().map(|s| s.to_string()));
    }

    jvm_args.push("-Dlog4j2.formatMsgNoLookups=true".to_string());

    let max_mb = compute_ram_mb(launch_settings, instance_config, &game_dir);
    let mn_mb = ((0.15 * max_mb as f64) as u32).max(128);
    jvm_args.push(format!("-Xmn{mn_mb}m"));
    jvm_args.push(format!("-Xmx{max_mb}m"));

    jvm_args.push("-Dfile.encoding=COMPAT".to_string());
    jvm_args.push("-Dstdout.encoding=UTF-8".to_string());
    jvm_args.push("-Dstderr.encoding=UTF-8".to_string());

    if launch_settings.preferred_ip_stack == "ipv4" {
        jvm_args.push("-Djava.net.preferIPv4Stack=true".to_string());
    } else if launch_settings.preferred_ip_stack == "ipv6" {
        jvm_args.push("-Djava.net.preferIPv6Addresses=true".to_string());
    }

    if let Some(api_url) = &session.api_location {
        if let Some(jar) = authlib_injector_jar {
            jvm_args.push(format!("-javaagent:{}={}", jar.display(), api_url));
            jvm_args.push("-Dauthlibinjector.side=client".to_string());
        }
    }

    if let Some(args) = &version_json.arguments {
        if !args.jvm.is_empty() {
            for entry in &args.jvm {
                match entry {
                    ArgumentEntry::Plain(s) => jvm_args.push(s.clone()),
                    ArgumentEntry::Conditional(c) => {
                        if evaluate_rules(&c.rules, &rule_ctx) {
                            jvm_args.extend(c.value.as_slice());
                        }
                    }
                }
            }
        } else {
            jvm_args.push("-Djava.library.path=${natives_directory}".to_string());
            jvm_args.push("-cp".to_string());
            jvm_args.push("${classpath}".to_string());
        }
    } else {
        jvm_args.push("-Djava.library.path=${natives_directory}".to_string());
        jvm_args.push("-cp".to_string());
        jvm_args.push("${classpath}".to_string());
    }

    // 4. Classpath
    let mut cp_entries: Vec<PathBuf> = Vec::new();
    for lib in &version_json.libraries {
        if !evaluate_rules(&lib.rules, &rule_ctx) {
            continue;
        }

        let jar_path =
            if let Some(artifact) = lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
                if let Some(p) = &artifact.path {
                    mc_folder
                        .join("libraries")
                        .join(p.replace('/', std::path::MAIN_SEPARATOR_STR))
                } else {
                    let rel = maven_coordinate_to_path(&lib.name);
                    mc_folder
                        .join("libraries")
                        .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
                }
            } else {
                let rel = maven_coordinate_to_path(&lib.name);
                mc_folder
                    .join("libraries")
                    .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
            };

        if jar_path.exists() && !cp_entries.contains(&jar_path) {
            cp_entries.push(jar_path);
        }
    }

    // Separate OptiFine jar so it precedes the client jar
    let mut regular_cp = Vec::new();
    let mut optifine_cp = Vec::new();
    for p in cp_entries {
        let name_lower = p.to_string_lossy().to_lowercase();
        if name_lower.contains("optifine") {
            optifine_cp.push(p);
        } else {
            regular_cp.push(p);
        }
    }

    // Add client jar
    let direct_client_jar = instance_dir.join(format!("{instance_id}.jar"));
    let client_jar = if direct_client_jar.exists() {
        direct_client_jar
    } else if let Some(parent) = &version_json.inherits_from {
        let parent_jar = mc_folder
            .join("versions")
            .join(parent)
            .join(format!("{parent}.jar"));
        if parent_jar.exists() {
            parent_jar
        } else {
            direct_client_jar
        }
    } else {
        direct_client_jar
    };

    regular_cp.extend(optifine_cp);
    regular_cp.push(client_jar);

    let classpath = regular_cp
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(";");

    // 5. Game Arguments
    let mut game_args: Vec<String> = Vec::new();
    if let Some(args) = &version_json.arguments {
        if !args.game.is_empty() {
            for entry in &args.game {
                match entry {
                    ArgumentEntry::Plain(s) => game_args.push(s.clone()),
                    ArgumentEntry::Conditional(c) => {
                        if evaluate_rules(&c.rules, &rule_ctx) {
                            game_args.extend(c.value.as_slice());
                        }
                    }
                }
            }
        } else if let Some(mc_args) = &version_json.minecraft_arguments {
            game_args.extend(mc_args.split_whitespace().map(|s| s.to_string()));
        }
    } else if let Some(mc_args) = &version_json.minecraft_arguments {
        game_args.extend(mc_args.split_whitespace().map(|s| s.to_string()));
    }

    if launch_settings.fullscreen {
        game_args.push("--fullscreen".to_string());
    } else {
        game_args.push("--width".to_string());
        game_args.push(launch_settings.width.to_string());
        game_args.push("--height".to_string());
        game_args.push(launch_settings.height.to_string());
    }

    if let Some(addr) = &instance_config.server_quick_play {
        let trimmed = addr.trim();
        if !trimmed.is_empty() {
            let (minor, _) = parse_mc_minor_patch(&extract_mc_version(version_json, instance_id));
            if minor >= 20 {
                game_args.push("--quickPlayMultiplayer".to_string());
                game_args.push(trimmed.to_string());
            } else {
                let (host, port) = if let Some((h, p)) = trimmed.split_once(':') {
                    (h, p.parse::<u16>().unwrap_or(25565))
                } else {
                    (trimmed, 25565)
                };
                game_args.push("--server".to_string());
                game_args.push(host.to_string());
                game_args.push("--port".to_string());
                game_args.push(port.to_string());
            }
        }
    }

    let custom_game = instance_config
        .game_args
        .as_ref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&launch_settings.game_args);
    if !custom_game.trim().is_empty() {
        game_args.extend(custom_game.split_whitespace().map(|s| s.to_string()));
    }

    // 6. Substitution map
    let mut subs: HashMap<&str, String> = HashMap::new();
    subs.insert(
        "${natives_directory}",
        natives_dir.to_string_lossy().to_string(),
    );
    subs.insert("${launcher_name}", "MCLR".to_string());
    subs.insert("${launcher_version}", env!("CARGO_PKG_VERSION").to_string());
    subs.insert("${version_name}", instance_id.to_string());
    subs.insert(
        "${version_type}",
        version_json
            .version_type
            .clone()
            .unwrap_or_else(|| "release".to_string()),
    );
    subs.insert("${game_directory}", game_dir.to_string_lossy().to_string());
    subs.insert(
        "${assets_root}",
        mc_folder.join("assets").to_string_lossy().to_string(),
    );
    subs.insert(
        "${assets_index_name}",
        version_json
            .asset_index
            .as_ref()
            .map(|a| a.id.clone())
            .or_else(|| version_json.assets.clone())
            .unwrap_or_else(|| "legacy".to_string()),
    );
    subs.insert("${auth_player_name}", session.username.clone());
    subs.insert("${auth_uuid}", session.uuid.clone());
    subs.insert("${auth_access_token}", session.access_token.clone());
    subs.insert("${user_type}", session.user_type.clone());
    subs.insert("${user_properties}", "{}".to_string());
    subs.insert("${clientid}", "MCLR".to_string());
    subs.insert(
        "${auth_xuid}",
        session.xuid.clone().unwrap_or_else(|| "0".to_string()),
    );
    subs.insert("${resolution_width}", launch_settings.width.to_string());
    subs.insert("${resolution_height}", launch_settings.height.to_string());
    subs.insert("${classpath}", classpath);
    subs.insert("${classpath_separator}", ";".to_string());
    subs.insert(
        "${library_directory}",
        mc_folder.join("libraries").to_string_lossy().to_string(),
    );

    let final_jvm_args: Vec<String> = jvm_args
        .iter()
        .map(|a| substitute_placeholders(a, &subs))
        .collect();

    let final_game_args: Vec<String> = game_args
        .iter()
        .map(|a| substitute_placeholders(a, &subs))
        .collect();

    let main_class = version_json
        .main_class
        .clone()
        .unwrap_or_else(|| "net.minecraft.client.main.Main".to_string());

    let java_bin = PathBuf::from(&java.home).join("bin");
    let javaw_exe = java_bin.join("javaw.exe");
    let java_executable = if javaw_exe.exists() {
        javaw_exe
    } else {
        PathBuf::from(&java.path)
    };

    // Calculate full command string length to detect Windows command line limit
    let total_len: usize = java_executable.to_string_lossy().len()
        + 1
        + final_jvm_args.iter().map(|s| s.len() + 1).sum::<usize>()
        + main_class.len()
        + 1
        + final_game_args.iter().map(|s| s.len() + 1).sum::<usize>();

    if java.major >= 9 && total_len > COMMAND_LINE_LIMIT {
        // Write @argfile
        let argfile_path = instance_dir.join("mclr-launch.args");
        let mut argfile_content = String::new();
        for arg in &final_jvm_args {
            let escaped = format!("\"{}\"\n", arg.replace('\\', "\\\\").replace('"', "\\\""));
            argfile_content.push_str(&escaped);
        }
        fs::write(&argfile_path, argfile_content)?;

        let mut final_args = vec![format!("@{}", argfile_path.display()), main_class];
        final_args.extend(final_game_args);

        Ok(LaunchCommandSpec {
            java_executable,
            java_home: PathBuf::from(&java.home),
            game_dir,
            args: final_args,
            used_argfile: true,
            argfile_path: Some(argfile_path),
        })
    } else {
        let mut final_args = final_jvm_args;
        final_args.push(main_class);
        final_args.extend(final_game_args);

        Ok(LaunchCommandSpec {
            java_executable,
            java_home: PathBuf::from(&java.home),
            game_dir,
            args: final_args,
            used_argfile: false,
            argfile_path: None,
        })
    }
}

/// Executes pre-launch command if configured.
pub fn execute_pre_launch_command(
    pre_launch_cmd: &str,
    java_exe: &Path,
    mc_folder: &Path,
    instance_dir: &Path,
    username: &str,
) -> Result<(), GameError> {
    if pre_launch_cmd.trim().is_empty() {
        return Ok(());
    }

    let now = chrono::Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H:%M:%S").to_string();

    let expanded = pre_launch_cmd
        .replace("{java}", &format!("\"{}\"", java_exe.display()))
        .replace("{minecraft}", &format!("\"{}\"", mc_folder.display()))
        .replace("{version_path}", &format!("\"{}\"", instance_dir.display()))
        .replace("{user}", username)
        .replace("{date}", &date_str)
        .replace("{time}", &time_str);

    let status = std::process::Command::new("cmd")
        .args(["/c", &expanded])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;

    if !status.success() {
        warn!("Pre-launch command exited with status: {:?}", status.code());
    }

    Ok(())
}

/// Sanitizes access token and Windows username in log lines.
pub fn sanitize_log_line(line: &str, access_token: &str, windows_username: &str) -> String {
    let mut result = line.to_string();
    if access_token.len() >= 4 && result.contains(access_token) {
        result = result.replace(access_token, "***");
    }
    if windows_username.len() >= 2 && result.contains(windows_username) {
        result = result.replace(windows_username, "***");
    }
    result
}

/// Classifies log level from raw log text.
pub fn classify_log_level(line: &str) -> LogLevel {
    if line.contains("/FATAL]") || line.contains("[FATAL]") || line.starts_with("FATAL:") {
        LogLevel::Fatal
    } else if line.contains("/ERROR]")
        || line.contains("[ERROR]")
        || line.contains("Exception in thread")
        || line.contains("Exception:")
        || line.contains("STDERR")
    {
        LogLevel::Error
    } else if line.contains("/WARN]") || line.contains("[WARN]") || line.starts_with("WARN:") {
        LogLevel::Warn
    } else if line.contains("/DEBUG]") || line.contains("[DEBUG]") || line.starts_with("DEBUG:") {
        LogLevel::Debug
    } else {
        LogLevel::Info
    }
}

/// Finds the main window handle (HWND) belonging to the given process ID.
pub fn find_window_for_pid(pid: u32) -> Option<HWND> {
    struct SearchContext {
        target_pid: u32,
        found_hwnd: Option<HWND>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let ctx = &mut *(lparam.0 as *mut SearchContext);
            let mut window_pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
            if window_pid == ctx.target_pid {
                let length = GetWindowTextLengthW(hwnd);
                if length > 0 {
                    let mut buf = vec![0u16; (length + 1) as usize];
                    let copied = GetWindowTextW(hwnd, &mut buf);
                    if copied > 0 {
                        let title = String::from_utf16_lossy(&buf[..copied as usize]);
                        if !title.starts_with("GLFW") && title != "PopupMessageWindow" {
                            ctx.found_hwnd = Some(hwnd);
                            return BOOL(0);
                        }
                    }
                }
            }
            BOOL(1)
        }
    }

    let mut ctx = SearchContext {
        target_pid: pid,
        found_hwnd: None,
    };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut _ as isize));
    }
    ctx.found_hwnd
}

/// Sets the window text for the top-level window of the given PID.
pub fn set_window_title_for_pid(pid: u32, title: &str) -> bool {
    if let Some(hwnd) = find_window_for_pid(pid) {
        let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe { SetWindowTextW(hwnd, PCWSTR::from_raw(wide.as_ptr())).is_ok() }
    } else {
        false
    }
}

/// Spawns background task to set Minecraft window title when ready.
fn spawn_window_title_poller(pid: u32, template: String) {
    if template.trim().is_empty() {
        return;
    }

    tokio::spawn(async move {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
        while tokio::time::Instant::now() < deadline {
            let now = chrono::Local::now();
            let expanded = template
                .replace("{date}", &now.format("%Y-%m-%d").to_string())
                .replace("{time}", &now.format("%H:%M:%S").to_string());

            let success =
                tokio::task::spawn_blocking(move || set_window_title_for_pid(pid, &expanded))
                    .await
                    .unwrap_or(false);

            if success {
                // Keep polling briefly to ensure title sticks
                for _ in 0..5 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    let now2 = chrono::Local::now();
                    let expanded2 = template
                        .replace("{date}", &now2.format("%Y-%m-%d").to_string())
                        .replace("{time}", &now2.format("%H:%M:%S").to_string());
                    let _ = tokio::task::spawn_blocking(move || {
                        set_window_title_for_pid(pid, &expanded2)
                    })
                    .await;
                }
                break;
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
}

/// Full game launch pipeline.
#[allow(clippy::too_many_arguments)]
pub async fn launch(
    client: &reqwest::Client,
    settings: &Settings,
    mc_folder: &Path,
    instance_id: &str,
    session: &ResolvedAccountSession,
    task_center: Option<(&Arc<TaskCenter>, &str)>,
    watcher: &Arc<LaunchWatcher>,
) -> Result<LaunchStartResult, GameError> {
    // 1. Precheck
    check_launch_paths(mc_folder, instance_id)?;

    let instance_dir = mc_folder.join("versions").join(instance_id);
    let instance_cfg_file = instance_dir.join("mclr.instance.json");
    let instance_config: InstanceConfig = if instance_cfg_file.exists() {
        if let Ok(c) = fs::read_to_string(&instance_cfg_file) {
            serde_json::from_str(&c).unwrap_or_default()
        } else {
            InstanceConfig::default()
        }
    } else {
        InstanceConfig::default()
    };

    check_login_requirement(&instance_config.login_requirement, session.kind)?;

    if let Some((tc, tid)) = task_center {
        let _ = tc.update_progress(tid, 0.1, 0).await;
    }

    // 2. Load resolved version JSON & Java resolution
    let version_json = load_resolved_version_json(mc_folder, instance_id)?;
    let mc_version = extract_mc_version(&version_json, instance_id);
    let constraint = version_java_constraint(&version_json, &mc_version);

    let installed_javas = scan_java(&settings.java.custom);
    let selected_java =
        resolve_java_installation(&installed_javas, &constraint, settings, &instance_config)?;

    if let Some((tc, tid)) = task_center {
        let _ = tc.update_progress(tid, 0.3, 0).await;
    }

    // 3. Instance repair
    let _ = repair_instance(
        client,
        &settings.net,
        mc_folder,
        instance_id,
        task_center.map(|(tc, tid)| (tc.as_ref(), tid)),
    )
    .await?;

    if let Some((tc, tid)) = task_center {
        let _ = tc.update_progress(tid, 0.6, 0).await;
    }

    // 4. Extract natives
    let natives_dir = extract_natives(mc_folder, instance_id, &version_json)?;

    // 5. Authlib injector if required
    let authlib_jar = if session.api_location.is_some() {
        Some(ensure_authlib_injector(client, &settings.net).await?)
    } else {
        None
    };

    if let Some((tc, tid)) = task_center {
        let _ = tc.update_progress(tid, 0.8, 0).await;
    }

    // 6. Build command line
    let spec = build_launch_command(
        mc_folder,
        instance_id,
        &version_json,
        &selected_java,
        &settings.launch,
        &settings.game,
        &instance_config,
        session,
        &natives_dir,
        authlib_jar.as_deref(),
    )?;

    // 7. Pre-launch command
    let pre_cmd = instance_config
        .pre_launch_command
        .as_deref()
        .unwrap_or_default();
    execute_pre_launch_command(
        pre_cmd,
        &spec.java_executable,
        mc_folder,
        &instance_dir,
        &session.username,
    )?;

    // 8. Spawn process
    let current_path = std::env::var("PATH").unwrap_or_default();
    let prepended_path = format!("{};{}", spec.java_home.join("bin").display(), current_path);

    let mut cmd = tokio::process::Command::new(&spec.java_executable);
    cmd.args(&spec.args)
        .current_dir(&spec.game_dir)
        .env("PATH", prepended_path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd.spawn().map_err(|e| {
        GameError::LaunchProcessFailed(format!(
            "Failed to spawn {}: {e}",
            spec.java_executable.display()
        ))
    })?;

    let process_id = child.id().unwrap_or(0);
    info!(
        "Spawned Minecraft process {process_id} for instance {instance_id} (argfile: {})",
        spec.used_argfile
    );

    let process_info = LaunchProcessInfo {
        process_id,
        instance_id: instance_id.to_string(),
        account_name: session.username.clone(),
        started_at: chrono::Local::now().to_rfc3339(),
        running: true,
        exit_code: None,
    };

    watcher.register_process(process_info).await;

    // Window title poller
    let window_title = instance_config
        .window_title
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(&settings.launch.window_title)
        .to_string();
    spawn_window_title_poller(process_id, window_title);

    // 9. Stream stdout and stderr logs in background
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let watcher_clone = Arc::clone(watcher);
    let instance_id_str = instance_id.to_string();
    let access_token = session.access_token.clone();
    let windows_user = std::env::var("USERNAME").unwrap_or_default();

    tokio::spawn(async move {
        let (log_tx, mut log_rx) = tokio::sync::mpsc::channel::<(String, bool)>(512);

        if let Some(out) = stdout {
            let tx1 = log_tx.clone();
            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(out).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if tx1.send((line, false)).await.is_err() {
                        break;
                    }
                }
            });
        }

        if let Some(err) = stderr {
            let tx2 = log_tx.clone();
            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(err).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if tx2.send((line, true)).await.is_err() {
                        break;
                    }
                }
            });
        }

        drop(log_tx);

        while let Some((raw_line, is_stderr)) = log_rx.recv().await {
            let sanitized = sanitize_log_line(&raw_line, &access_token, &windows_user);
            let level = if is_stderr {
                let classified = classify_log_level(&sanitized);
                if classified == LogLevel::Info {
                    LogLevel::Error
                } else {
                    classified
                }
            } else {
                classify_log_level(&sanitized)
            };

            let line_id = watcher_clone.line_id_counter.fetch_add(1, Ordering::SeqCst);
            let log_line = LogLine {
                line_id,
                process_id,
                instance_id: instance_id_str.clone(),
                level,
                text: sanitized,
                timestamp: chrono::Local::now().to_rfc3339(),
            };
            watcher_clone.append_log(log_line).await;
        }

        let exit_code = match child.wait().await {
            Ok(status) => status.code().unwrap_or(0),
            Err(e) => {
                error!("Process wait error: {e}");
                -1
            }
        };

        info!("Minecraft process {process_id} exited with code {exit_code}");
        watcher_clone
            .mark_exited(process_id, &instance_id_str, exit_code)
            .await;
    });

    if let Some((tc, tid)) = task_center {
        let _ = tc.complete_task(tid).await;
    }

    let task_id = task_center
        .map(|(_, tid)| tid.to_string())
        .unwrap_or_default();
    Ok(LaunchStartResult {
        task_id,
        process_id: Some(process_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_java_constraint() {
        let empty_json = VersionJson::default();

        // 1.20.5 -> 21
        let c1 = version_java_constraint(&empty_json, "1.20.5");
        assert_eq!(c1.min_major, Some(21));
        assert_eq!(c1.max_major, None);

        // 1.20.1 -> 17
        let c2 = version_java_constraint(&empty_json, "1.20.1");
        assert_eq!(c2.min_major, Some(17));
        assert_eq!(c2.max_major, None);

        // 1.16.5 -> 8
        let c3 = version_java_constraint(&empty_json, "1.16.5");
        assert_eq!(c3.min_major, Some(8));
        assert_eq!(c3.max_major, None);

        // 1.5.2 -> max 8
        let c4 = version_java_constraint(&empty_json, "1.5.2");
        assert_eq!(c4.min_major, None);
        assert_eq!(c4.max_major, Some(8));
    }

    #[test]
    fn test_sanitize_and_classify_log() {
        let token = "eyJhbGciOiJIUzI1NiJ9.s3cr3tT0k3n";
        let win_user = "test_developer";

        let raw = format!(
            "[12:00:00] [main/INFO]: User logged in with token {token} at C:\\Users\\{win_user}\\AppData"
        );
        let sanitized = sanitize_log_line(&raw, token, win_user);

        assert!(!sanitized.contains(token));
        assert!(!sanitized.contains(win_user));
        assert!(sanitized.contains("***"));
        assert_eq!(classify_log_level(&sanitized), LogLevel::Info);

        let warn_line = "[12:00:01] [Render thread/WARN]: Something might be wrong";
        assert_eq!(classify_log_level(warn_line), LogLevel::Warn);

        let error_line = "[12:00:02] [Server thread/ERROR]: Encountered an unexpected exception";
        assert_eq!(classify_log_level(error_line), LogLevel::Error);

        let fatal_line = "[FATAL] Failed to initialize Minecraft engine";
        assert_eq!(classify_log_level(fatal_line), LogLevel::Fatal);
    }

    #[test]
    fn test_argfile_generation_over_8000_chars() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mc_folder = temp_dir.path();
        let instance_id = "test-instance";
        let instance_dir = mc_folder.join("versions").join(instance_id);
        fs::create_dir_all(&instance_dir).unwrap();

        // Write a fake instance json
        fs::write(
            instance_dir.join(format!("{instance_id}.json")),
            r#"{"id":"test-instance","mainClass":"net.minecraft.client.main.Main"}"#,
        )
        .unwrap();

        let mut long_jvm_args = String::new();
        for i in 0..600 {
            long_jvm_args.push_str(&format!("-Dcustom.property.number.{i}=value_{i} "));
        }

        let launch_settings = LaunchSettings {
            jvm_args: long_jvm_args,
            ..Default::default()
        };
        let game_settings = GameSettings::default();
        let instance_config = InstanceConfig::default();
        let session = ResolvedAccountSession {
            id: "test-acc".to_string(),
            kind: AccountKind::Offline,
            username: "Steve".to_string(),
            uuid: "00000000000000000000000000000000".to_string(),
            access_token: "fake-token".to_string(),
            user_type: "legacy".to_string(),
            api_location: None,
            xuid: None,
        };

        let natives_dir = instance_dir.join("test-natives");
        fs::create_dir_all(&natives_dir).unwrap();

        let version_json = VersionJson {
            id: instance_id.to_string(),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            ..Default::default()
        };

        let java21 = JavaInstallation {
            path: "C:\\Program Files\\Java\\jdk-21\\bin\\java.exe".to_string(),
            home: "C:\\Program Files\\Java\\jdk-21".to_string(),
            version: "21.0.2".to_string(),
            major: 21,
            implementor: "Oracle".to_string(),
            arch: "x64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        };

        let spec21 = build_launch_command(
            mc_folder,
            instance_id,
            &version_json,
            &java21,
            &launch_settings,
            &game_settings,
            &instance_config,
            &session,
            &natives_dir,
            None,
        )
        .unwrap();

        assert!(spec21.used_argfile);
        assert!(spec21.argfile_path.is_some());
        let argfile = spec21.argfile_path.unwrap();
        assert!(argfile.exists());
        assert!(spec21.args[0].starts_with('@'));

        // For Java 8, direct args must be used even over 8000 chars
        let java8 = JavaInstallation {
            path: "C:\\Program Files\\Java\\jre1.8.0\\bin\\java.exe".to_string(),
            home: "C:\\Program Files\\Java\\jre1.8.0".to_string(),
            version: "1.8.0_402".to_string(),
            major: 8,
            implementor: "Oracle".to_string(),
            arch: "x64".to_string(),
            is_jdk: false,
            source: "test".to_string(),
        };

        let spec8 = build_launch_command(
            mc_folder,
            instance_id,
            &version_json,
            &java8,
            &launch_settings,
            &game_settings,
            &instance_config,
            &session,
            &natives_dir,
            None,
        )
        .unwrap();

        assert!(!spec8.used_argfile);
        assert!(spec8.argfile_path.is_none());
        assert!(!spec8.args[0].starts_with('@'));
    }

    #[test]
    fn test_auto_ram_calculation() {
        // Base 2048 + 25 * mods
        // 0 mods -> 2048
        assert_eq!(compute_ram_mb_auto(0, 16384), 2048);

        // 10 mods -> 2048 + 250 = 2298
        assert_eq!(compute_ram_mb_auto(10, 16384), 2298);

        // 100 mods -> 2048 + 2500 = 4548
        assert_eq!(compute_ram_mb_auto(100, 16384), 4548);

        // High mod count capped at 16384
        assert_eq!(compute_ram_mb_auto(1000, 32768), 16384);

        // Capped at 0.8 * free_ram_mb
        // free = 3000 -> 0.8 * 3000 = 2400
        // mods = 100 (needs 4548) -> capped at 2400
        assert_eq!(compute_ram_mb_auto(100, 3000), 2400);

        // Minimum 2048 even if 0.8 * free is lower
        assert_eq!(compute_ram_mb_auto(10, 1000), 2048);
    }
}
