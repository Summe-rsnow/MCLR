//! Minecraft launch pipeline, arguments builder, natives extraction, and process watcher.

pub mod agents;
pub mod args;
pub mod process;
pub mod ram;
pub mod win32;

use std::collections::{HashMap, VecDeque};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;

use mclr_core::account_session::AccountKind;
use mclr_core::settings::Settings;
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::sync::{RwLock, broadcast};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

pub use args::{
    LaunchCommandInput, build_launch_command, ensure_authlib_injector, extract_natives,
    substitute_placeholders,
};
pub use process::{
    LaunchPipelineParams, execute_pre_launch_command, execute_pre_launch_command_async, launch,
};
pub use ram::{
    compute_ram_mb, compute_ram_mb_auto, count_mods_in_dir, get_free_physical_memory_mb,
};
pub use win32::{find_window_for_pid, set_window_title_for_pid};

use crate::error::GameError;
use crate::instance::InstanceConfig;
use crate::java::{JavaConstraint, JavaInstallation, inspect_java, select_java};
use crate::version::VersionJson;
use tracing::warn;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const MAX_LOG_LINES: usize = 5000;

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

/// Mojang runtime component that supplies the Java runtime for a version JSON
/// (e.g. `java-runtime-epsilon` for Java 25). `None` when the JSON omits it.
pub fn version_java_component(version_json: &VersionJson) -> Option<String> {
    version_json
        .java_version
        .as_ref()
        .map(|jv| jv.component.trim().to_string())
        .filter(|c| !c.is_empty())
}

/// Detects Mojang's post-"1.x" version scheme and returns its release series.
/// E.g. "26.3" -> 26, "26.4-snapshot-6" -> 26, "1.21.4" -> None.
pub fn new_scheme_series(mc_version: &str) -> Option<u32> {
    let clean = mc_version.trim();
    let digits: String = clean.chars().take_while(|c| c.is_ascii_digit()).collect();
    let rest = clean.get(digits.len()..)?;
    // Legacy "1.x" keeps a series of 1, and "24w14a" snapshots start with "24w".
    if digits.is_empty() || !rest.starts_with('.') {
        return None;
    }
    let series = digits.parse::<u32>().ok()?;
    (series >= 20).then_some(series)
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

    // Mojang's post-1.x numbering ("26.3", "26.4-snapshot-6") always ships Java 25.
    // Without this, "26.3" fell through to the legacy branches below and resolved to Java 8/17,
    // launching with an older JRE and failing with UnsupportedClassVersionError.
    if let Some(series) = new_scheme_series(mc_version) {
        if series >= 26 {
            return JavaConstraint {
                min_major: Some(25),
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

/// Whether `java` can run a version with `constraint` (never a JVM older than the class files).
pub fn java_satisfies(java: &JavaInstallation, constraint: &JavaConstraint) -> bool {
    if !constraint.exact_majors.is_empty() && !constraint.exact_majors.contains(&java.major) {
        return false;
    }
    if let Some(min) = constraint.min_major
        && java.major < min
    {
        return false;
    }
    if let Some(max) = constraint.max_major
        && java.major > max
    {
        return false;
    }
    true
}

/// Resolves the Java installation to use, checking per-instance override first, then settings.
pub fn resolve_java_installation(
    installed_list: &[JavaInstallation],
    constraint: &JavaConstraint,
    settings: &Settings,
    instance_config: &InstanceConfig,
) -> Result<JavaInstallation, GameError> {
    resolve_java_installation_with_component(
        installed_list,
        constraint,
        settings,
        instance_config,
        None,
    )
}

/// Same as [`resolve_java_installation`], but carries the Mojang runtime component that
/// satisfies the constraint so the caller can offer/perform the right download.
pub fn resolve_java_installation_with_component(
    installed_list: &[JavaInstallation],
    constraint: &JavaConstraint,
    settings: &Settings,
    instance_config: &InstanceConfig,
    component: Option<&str>,
) -> Result<JavaInstallation, GameError> {
    // 1. Per-instance override — honoured only when it can actually run this version.
    if let Some(custom) = &instance_config.java {
        let trimmed = custom.trim();
        if !trimmed.is_empty() && trimmed != "auto" {
            let custom_p = PathBuf::from(trimmed);
            let exe = if custom_p.is_dir() {
                custom_p.join("bin").join("java.exe")
            } else {
                custom_p
            };
            let candidate = if exe.exists() {
                inspect_java(&exe, "instanceCustom")
            } else {
                let sel_norm = trimmed.replace('/', "\\").to_lowercase();
                installed_list
                    .iter()
                    .find(|j| {
                        j.path.replace('/', "\\").to_lowercase() == sel_norm
                            || j.home.replace('/', "\\").to_lowercase() == sel_norm
                    })
                    .cloned()
            };

            if let Some(found) = candidate {
                if java_satisfies(&found, constraint) {
                    return Ok(found);
                }
                // The pinned runtime cannot load this version's class files; a compatible
                // install beats a guaranteed JVM crash.
                if let Some(auto) = select_java(installed_list, constraint, &settings.java) {
                    warn!(
                        "Instance Java {} ({}) cannot run this version (requires {:?}); using {}",
                        found.version, found.path, constraint, auto.version
                    );
                    return Ok(auto);
                }
                return Err(GameError::LaunchJavaTooOld {
                    required: constraint.min_major.unwrap_or(found.major),
                    actual: found.major,
                });
            }
        }
    }

    // 2. Smart selection: no global default exists, ranking comes from the requirement.
    if let Some(found) = select_java(installed_list, constraint, &settings.java) {
        return Ok(found);
    }

    let major = constraint.min_major.or(constraint.max_major).unwrap_or(8);
    Err(GameError::LaunchJavaNotFound {
        major,
        component: component.map(str::to_string),
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use mclr_core::account_session::ResolvedAccountSession;
    use mclr_core::settings::{GameSettings, LaunchSettings};
    use std::fs;

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

    /// MC 26.x renamed the version scheme; without the new-scheme branch "26.3" resolved to
    /// the legacy Java 8/17 rules and launched with an older JRE (UnsupportedClassVersionError).
    #[test]
    fn test_version_java_constraint_new_scheme() {
        let empty_json = VersionJson::default();

        for id in ["26.3", "26.1", "26.4-snapshot-6", "26.3-pre-2"] {
            let c = version_java_constraint(&empty_json, id);
            assert_eq!(c.min_major, Some(25), "{id} must require Java 25");
            assert_eq!(c.max_major, None, "{id} must not cap the major version");
        }

        // Legacy 1.x and old weekly snapshots keep their established requirements.
        assert_eq!(
            version_java_constraint(&empty_json, "1.21.4").min_major,
            Some(21)
        );
        assert_eq!(
            version_java_constraint(&empty_json, "24w14a").min_major,
            Some(21)
        );
        assert_eq!(new_scheme_series("1.21.4"), None);
        assert_eq!(new_scheme_series("24w14a"), None);
        assert_eq!(new_scheme_series("26.4-snapshot-6"), Some(26));

        // The manifest's own javaVersion always wins over the guesswork.
        let declared = VersionJson {
            java_version: Some(crate::version::JavaVersionInfo {
                component: "java-runtime-epsilon".to_string(),
                major_version: 25,
            }),
            ..Default::default()
        };
        assert_eq!(
            version_java_constraint(&declared, "26.3").min_major,
            Some(25)
        );
        assert_eq!(
            version_java_component(&declared).as_deref(),
            Some("java-runtime-epsilon")
        );
        assert_eq!(version_java_component(&empty_json), None);
    }

    /// A per-instance Java pin that cannot run the version must not be honoured:
    /// a compatible install is used instead, or the requirement is reported.
    #[test]
    fn test_pinned_instance_java_does_not_break_newer_versions() {
        let java_install = |major: u32, version: &str| JavaInstallation {
            path: format!("C:\\Java\\jdk{major}\\bin\\java.exe"),
            home: format!("C:\\Java\\jdk{major}"),
            version: version.to_string(),
            major,
            implementor: "Test".to_string(),
            arch: "x86_64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        };
        let j21 = java_install(21, "21.0.5");
        let j25 = java_install(25, "25.0.3");
        let constraint = JavaConstraint {
            min_major: Some(25),
            max_major: None,
            exact_majors: vec![],
        };
        let settings = Settings::default();
        let mut config = InstanceConfig::default();
        config.java = Some(j21.path.clone());

        let picked =
            resolve_java_installation(&[j21.clone(), j25.clone()], &constraint, &settings, &config)
                .expect("a compatible Java 25 is installed");
        assert_eq!(picked.major, 25);

        // Only the incompatible pin exists: fail loudly with the requirement, not with a
        // JVM crash at startup.
        let err =
            resolve_java_installation(std::slice::from_ref(&j21), &constraint, &settings, &config)
                .expect_err("Java 21 cannot run a Java 25 instance");
        match err {
            GameError::LaunchJavaTooOld { required, actual } => {
                assert_eq!((required, actual), (25, 21));
            }
            other => panic!("unexpected error: {other:?}"),
        }
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

    /// Builds a minimal spec so launch-option tests stay focused on argument content.
    fn build_test_spec(
        launch_settings: LaunchSettings,
        java_home: &Path,
    ) -> crate::launch::LaunchCommandSpec {
        let temp_dir = tempfile::tempdir().unwrap();
        let mc_folder = temp_dir.path().to_path_buf();
        let instance_id = "opt-instance";
        let instance_dir = mc_folder.join("versions").join(instance_id);
        fs::create_dir_all(&instance_dir).unwrap();
        fs::write(
            instance_dir.join(format!("{instance_id}.json")),
            r#"{"id":"opt-instance","mainClass":"net.minecraft.client.main.Main"}"#,
        )
        .unwrap();
        let natives_dir = instance_dir.join("natives");
        fs::create_dir_all(&natives_dir).unwrap();

        let bin = java_home.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("java.exe"), b"stub").unwrap();
        fs::write(bin.join("javaw.exe"), b"stub").unwrap();

        let java = JavaInstallation {
            path: bin.join("java.exe").to_string_lossy().to_string(),
            home: java_home.to_string_lossy().to_string(),
            version: "21.0.2".to_string(),
            major: 21,
            implementor: "Test".to_string(),
            arch: "x64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        };
        let version_json = VersionJson {
            id: instance_id.to_string(),
            main_class: Some("net.minecraft.client.main.Main".to_string()),
            ..Default::default()
        };
        let session = ResolvedAccountSession {
            id: "opt-acc".to_string(),
            kind: AccountKind::Offline,
            username: "Steve".to_string(),
            uuid: "00000000000000000000000000000000".to_string(),
            access_token: "fake-token".to_string(),
            user_type: "legacy".to_string(),
            api_location: None,
            xuid: None,
        };
        let game_settings = GameSettings::default();
        let instance_config = InstanceConfig::default();

        // Leak the tempdir so the paths stay valid for the returned spec.
        std::mem::forget(temp_dir);
        build_launch_command(&LaunchCommandInput {
            mc_folder: &mc_folder,
            instance_id,
            version_json: &version_json,
            java: &java,
            launch_settings: &launch_settings,
            game_settings: &game_settings,
            instance_config: &instance_config,
            session: &session,
            natives_dir: &natives_dir,
            authlib_injector_jar: None,
        })
        .unwrap()
    }

    /// Builds a spec from a raw version JSON so launch-time agents can be asserted.
    fn build_spec_with_json(
        json: &str,
        settings: LaunchSettings,
    ) -> crate::launch::LaunchCommandSpec {
        build_spec_with_version(VersionJson::parse(json).unwrap(), settings)
    }

    fn build_spec_with_version(
        version_json: VersionJson,
        settings: LaunchSettings,
    ) -> crate::launch::LaunchCommandSpec {
        let temp_dir = tempfile::tempdir().unwrap();
        let mc_folder = temp_dir.path().to_path_buf();
        let instance_id = "agent-instance";
        let instance_dir = mc_folder.join("versions").join(instance_id);
        fs::create_dir_all(&instance_dir).unwrap();
        fs::write(
            instance_dir.join(format!("{instance_id}.json")),
            serde_json::to_string(&version_json).unwrap(),
        )
        .unwrap();
        let natives_dir = instance_dir.join("natives");
        fs::create_dir_all(&natives_dir).unwrap();

        let java_home = mc_folder.join("jdk21");
        let java = JavaInstallation {
            path: java_home
                .join("bin")
                .join("java.exe")
                .to_string_lossy()
                .to_string(),
            home: java_home.to_string_lossy().to_string(),
            version: "21.0.2".to_string(),
            major: 21,
            implementor: "Test".to_string(),
            arch: "x64".to_string(),
            is_jdk: true,
            source: "test".to_string(),
        };
        let session = ResolvedAccountSession {
            id: "agent-acc".to_string(),
            kind: AccountKind::Offline,
            username: "Steve".to_string(),
            uuid: "00000000000000000000000000000000".to_string(),
            access_token: "fake-token".to_string(),
            user_type: "legacy".to_string(),
            api_location: None,
            xuid: None,
        };
        let game_settings = GameSettings::default();
        let instance_config = InstanceConfig::default();

        std::mem::forget(temp_dir);
        build_launch_command(&LaunchCommandInput {
            mc_folder: &mc_folder,
            instance_id,
            version_json: &version_json,
            java: &java,
            launch_settings: &settings,
            game_settings: &game_settings,
            instance_config: &instance_config,
            session: &session,
            natives_dir: &natives_dir,
            authlib_injector_jar: None,
        })
        .unwrap()
    }

    const LWJGL_341_JSON: &str = r#"{
        "id":"26.1-vanilla",
        "mainClass":"net.minecraft.client.main.Main",
        "releaseTime":"2026-03-01T00:00:00+00:00",
        "libraries":[{"name":"org.lwjgl:lwjgl:3.4.1"},{"name":"org.lwjgl:lwjgl-glfw:3.4.1"}]
    }"#;

    const LEGACY_JSON: &str = r#"{
        "id":"1.5.2-vanilla",
        "mainClass":"net.minecraft.client.main.Main",
        "releaseTime":"2013-05-02T00:00:00+00:00",
        "libraries":[{"name":"org.lwjgl:lwjgl:2.9.0"}]
    }"#;

    fn agent_args(spec: &crate::launch::LaunchCommandSpec) -> Vec<String> {
        spec.args
            .iter()
            .filter(|a| a.starts_with("-javaagent:"))
            .cloned()
            .collect()
    }

    #[test]
    fn injects_lwjgl_unsafe_agent_only_for_3_4_1() {
        let spec = build_spec_with_json(LWJGL_341_JSON, LaunchSettings::default());
        let agents = agent_args(&spec);
        assert_eq!(agents.len(), 1, "{agents:?}");
        assert!(agents[0].ends_with("lwjgl-unsafe-agent.jar"), "{agents:?}");

        // Older LWJGL stays untouched.
        let spec = build_spec_with_json(LEGACY_JSON, LaunchSettings::default());
        assert!(
            agent_args(&spec)
                .iter()
                .all(|a| !a.contains("lwjgl-unsafe-agent")),
            "3.4.1-only agent leaked into an old instance"
        );
    }

    #[test]
    fn agents_apply_to_instances_that_inherit_their_parent_json() {
        // Modded instance: no releaseTime / lwjgl of its own.
        let child = VersionJson::parse(
            r#"{"id":"26.3-neoforge","inheritsFrom":"26.3",
                "libraries":[{"name":"net.neoforged:neoforge:26.3.0"}]}"#,
        )
        .unwrap();
        let parent = VersionJson::parse(LWJGL_341_JSON).unwrap();
        let merged = crate::version::merge_version_json(child, parent);

        let spec = build_spec_with_version(merged.clone(), LaunchSettings::default());
        let agents = agent_args(&spec);
        assert_eq!(agents.len(), 1, "{agents:?}");
        assert!(agents[0].ends_with("lwjgl-unsafe-agent.jar"), "{agents:?}");

        let legacy_child = VersionJson::parse(
            r#"{"id":"1.5.2-forge","inheritsFrom":"1.5.2","libraries":[{"name":"net.minecraftforge:forge:1.5.2-7.8.1.738"}]}"#,
        )
        .unwrap();
        let legacy_parent = VersionJson::parse(LEGACY_JSON).unwrap();
        let merged = crate::version::merge_version_json(legacy_child, legacy_parent);
        let spec = build_spec_with_version(merged, LaunchSettings::default());
        let agents = agent_args(&spec);
        assert_eq!(agents.len(), 1, "{agents:?}");
        assert!(agents[0].ends_with("legacyfix.jar"), "{agents:?}");
    }

    #[test]
    fn lwjgl_unsafe_agent_can_be_disabled() {
        let settings = LaunchSettings {
            lwjgl_unsafe_agent: false,
            ..Default::default()
        };
        let spec = build_spec_with_json(LWJGL_341_JSON, settings);
        assert!(agent_args(&spec).is_empty(), "{:?}", agent_args(&spec));
    }

    #[test]
    fn injects_legacy_fix_only_for_old_releases() {
        let spec = build_spec_with_json(LEGACY_JSON, LaunchSettings::default());
        let agents = agent_args(&spec);
        assert_eq!(agents.len(), 1, "{agents:?}");
        assert!(agents[0].ends_with("legacyfix.jar"), "{agents:?}");

        let spec = build_spec_with_json(LWJGL_341_JSON, LaunchSettings::default());
        assert!(
            agent_args(&spec).iter().all(|a| !a.contains("legacyfix")),
            "legacyfix leaked into a modern instance"
        );

        let settings = LaunchSettings {
            legacy_fix: false,
            ..Default::default()
        };
        let spec = build_spec_with_json(LEGACY_JSON, settings);
        assert!(agent_args(&spec).is_empty());
    }

    #[test]
    fn test_lock_memory_mirrors_xmx_into_xms() {
        let temp = tempfile::tempdir().unwrap();
        let settings = LaunchSettings {
            ram_mode: "custom".to_string(),
            ram_custom_mb: 4096,
            lock_memory: true,
            ..Default::default()
        };
        let spec = build_test_spec(settings, temp.path());
        let xmx = spec
            .args
            .iter()
            .find(|a| a.starts_with("-Xmx"))
            .expect("-Xmx present")
            .clone();
        let xms = spec
            .args
            .iter()
            .find(|a| a.starts_with("-Xms"))
            .expect("-Xms added by lock_memory")
            .clone();
        assert_eq!(xms, xmx.replace("-Xmx", "-Xms"));
    }

    #[test]
    fn test_lock_memory_respects_user_supplied_xms() {
        let temp = tempfile::tempdir().unwrap();
        let settings = LaunchSettings {
            jvm_args: "-Xms512m".to_string(),
            lock_memory: true,
            ..Default::default()
        };
        let spec = build_test_spec(settings, temp.path());
        let xms_count = spec.args.iter().filter(|a| a.starts_with("-Xms")).count();
        assert_eq!(xms_count, 1, "user -Xms must not be duplicated");
    }

    #[test]
    fn test_use_java_exe_switches_binary() {
        let temp = tempfile::tempdir().unwrap();
        // Default is `java.exe` (console attached); the switch turns it off.
        assert_eq!(
            build_test_spec(LaunchSettings::default(), temp.path())
                .java_executable
                .file_name()
                .unwrap()
                .to_string_lossy(),
            "java.exe"
        );
        let launch = |use_java_exe: bool| {
            let settings = LaunchSettings {
                use_java_exe,
                ..Default::default()
            };
            build_test_spec(settings, temp.path())
                .java_executable
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        };
        assert_eq!(launch(false), "javaw.exe");
        assert_eq!(launch(true), "java.exe");
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

        let spec21 = build_launch_command(&LaunchCommandInput {
            mc_folder,
            instance_id,
            version_json: &version_json,
            java: &java21,
            launch_settings: &launch_settings,
            game_settings: &game_settings,
            instance_config: &instance_config,
            session: &session,
            natives_dir: &natives_dir,
            authlib_injector_jar: None,
        })
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

        let spec8 = build_launch_command(&LaunchCommandInput {
            mc_folder,
            instance_id,
            version_json: &version_json,
            java: &java8,
            launch_settings: &launch_settings,
            game_settings: &game_settings,
            instance_config: &instance_config,
            session: &session,
            natives_dir: &natives_dir,
            authlib_injector_jar: None,
        })
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

        // The 2 GB floor must not override the safety cap: with only ~1 GB free,
        // promising 2048 MB would exceed physical memory.
        assert_eq!(compute_ram_mb_auto(10, 1000), 800);
        assert_eq!(compute_ram_mb_auto(0, 2048), 1638);
    }
}
