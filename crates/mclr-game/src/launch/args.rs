//! Launch command specification, argument substitution, native DLL extraction, and authlib-injector.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use mclr_core::account_session::ResolvedAccountSession;
use mclr_core::paths;
use mclr_core::settings::{GameSettings, LaunchSettings, NetSettings};
use mclr_net::{DownloadSpec, Downloader};
use serde::Deserialize;
use tracing::{info, warn};
use url::Url;

use super::ram::compute_ram_mb;
use crate::error::GameError;
use crate::install::maven_coordinate_to_path;
use crate::instance::{InstanceConfig, extract_mc_version};
use crate::java::JavaInstallation;
use crate::launch::{LaunchCommandSpec, parse_mc_minor_patch};
use crate::version::{ArgumentEntry, RuleContext, VersionJson, evaluate_rules};

const COMMAND_LINE_LIMIT: usize = 8000;

pub struct LaunchCommandInput<'a> {
    pub mc_folder: &'a Path,
    pub instance_id: &'a str,
    pub version_json: &'a VersionJson,
    pub java: &'a JavaInstallation,
    pub launch_settings: &'a LaunchSettings,
    pub game_settings: &'a GameSettings,
    pub instance_config: &'a InstanceConfig,
    pub session: &'a ResolvedAccountSession,
    pub natives_dir: &'a Path,
    pub authlib_injector_jar: Option<&'a Path>,
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
            // A short write would leave a truncated DLL that the size check above
            // then treats as valid forever, surfacing as UnsatisfiedLinkError.
            match std::io::copy(&mut entry, &mut out) {
                Ok(written) if written == entry.size() => {}
                Ok(written) => {
                    warn!(
                        "Short read extracting {}: {written}/{} bytes",
                        dest_file.display(),
                        entry.size()
                    );
                    drop(out);
                    let _ = fs::remove_file(&dest_file);
                }
                Err(e) => {
                    warn!("Failed to extract {}: {e}", dest_file.display());
                    drop(out);
                    let _ = fs::remove_file(&dest_file);
                }
            }
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
/// Builds the `-javaagent:` argument for the bundled Mesa bridge, if present.
///
/// The agent lives at `<data>/tools/mesa-loader/Loader.jar`; the option is a no-op
/// when the file is missing so enabling it can never break a launch.
fn renderer_agent_arg(renderer: &str) -> Option<String> {
    let mode = match renderer {
        "software" => "llvmpipe",
        "d3d12" => "d3d12",
        "vulkan" => "zink",
        _ => return None,
    };
    let jar = paths::tools_dir().join("mesa-loader").join("Loader.jar");
    if !jar.is_file() {
        warn!(
            "Renderer option '{mode}' requested but {} is missing; using the game default",
            jar.display()
        );
        return None;
    }
    Some(format!("-javaagent:\"{}\"={mode}", jar.display()))
}

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
pub fn build_launch_command(
    input: &LaunchCommandInput<'_>,
) -> Result<LaunchCommandSpec, GameError> {
    let LaunchCommandInput {
        mc_folder,
        instance_id,
        version_json,
        java,
        launch_settings,
        game_settings,
        instance_config,
        session,
        natives_dir,
        authlib_injector_jar,
    } = *input;

    let instance_dir = mc_folder.join("versions").join(instance_id);

    // 1. Determine isolated game directory
    let game_dir = crate::gamedir::resolve_game_dir(mc_folder, instance_id, game_settings);
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

    // Bundled compatibility agents, injected at the head of the JVM arguments
    // (PCL order). Both are best-effort: if the jar cannot be materialized the
    // launch simply proceeds without it.
    //
    // * LegacyFix: Minecraft 1.5.x and older on a modern JRE.
    // * LWJGL unsafe agent: LWJGL 3.4.1 (26.1+) regains the `sun.misc.Unsafe`
    //   native-memory path instead of the slow FFM path on JDK 25+.
    if launch_settings.legacy_fix
        && super::agents::needs_legacy_fix(version_json.release_time.as_deref())
    {
        if let Some(path) = super::agents::ensure_agent(super::agents::LaunchAgent::LegacyFix) {
            jvm_args.insert(0, format!("-javaagent:{}", path.display()));
        }
    }
    let lwjgl_matches = super::agents::lwjgl_version(&version_json.libraries)
        .is_some_and(|v| v == super::agents::LWJGL_UNSAFE_TARGET_VERSION);
    if launch_settings.lwjgl_unsafe_agent && lwjgl_matches {
        if let Some(path) = super::agents::ensure_agent(super::agents::LaunchAgent::LwjglUnsafe) {
            jvm_args.insert(0, format!("-javaagent:{}", path.display()));
        }
    }

    // Software renderer bridge (Mesa): only injected when the agent jar exists,
    // so the option is inert instead of breaking launches when it is absent.
    if let Some(agent_arg) = renderer_agent_arg(&launch_settings.renderer) {
        jvm_args.insert(0, agent_arg);
    }

    jvm_args.push("-Dlog4j2.formatMsgNoLookups=true".to_string());

    let max_mb = compute_ram_mb(launch_settings, instance_config, &game_dir);
    let mn_mb = ((0.15 * max_mb as f64) as u32).max(128);
    jvm_args.push(format!("-Xmn{mn_mb}m"));
    jvm_args.push(format!("-Xmx{max_mb}m"));

    // Lock memory: mirror -Xmx into -Xms unless the user already pinned it.
    if launch_settings.lock_memory
        && !jvm_args
            .iter()
            .any(|a| a.to_ascii_lowercase().starts_with("-xms"))
    {
        jvm_args.push(format!("-Xms{max_mb}m"));
    }

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
    let console_exe = java_bin.join("java.exe");
    // `use_java_exe` keeps a console window attached so stdout/logs stay visible.
    let java_executable = if launch_settings.use_java_exe && console_exe.exists() {
        console_exe
    } else if javaw_exe.exists() {
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
