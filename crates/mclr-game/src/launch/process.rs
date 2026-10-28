//! Process spawning, pre-launch command execution, log streaming, and watcher integration.

use std::fs;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use mclr_core::account_session::ResolvedAccountSession;
use mclr_core::settings::Settings;
use mclr_core::tasks::TaskCenter;
use tokio::io::AsyncBufReadExt;
use tracing::{debug, error, info, warn};

use super::args::{
    LaunchCommandInput, build_launch_command, ensure_authlib_injector, extract_natives,
};
use super::win32::spawn_window_title_poller;
use super::{
    LaunchProcessInfo, LaunchStartResult, LaunchWatcher, LogLevel, LogLine, check_launch_paths,
    check_login_requirement, classify_log_level, resolve_java_installation_with_component,
    sanitize_log_line, version_java_component, version_java_constraint,
};
use crate::error::GameError;
use crate::install::{load_resolved_version_json, repair_instance};
use crate::instance::{InstanceConfig, extract_mc_version};
use crate::java::scan_java;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct LaunchPipelineParams<'a> {
    pub client: &'a reqwest::Client,
    pub settings: &'a Settings,
    pub mc_folder: &'a Path,
    pub instance_id: &'a str,
    pub session: &'a ResolvedAccountSession,
    pub task_center: Option<(&'a Arc<TaskCenter>, &'a str)>,
    pub watcher: &'a Arc<LaunchWatcher>,
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

/// Executes pre-launch command asynchronously if configured.
pub async fn execute_pre_launch_command_async(
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

    let status = tokio::process::Command::new("cmd")
        .args(["/c", &expanded])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .await?;

    if !status.success() {
        warn!("Pre-launch command exited with status: {:?}", status.code());
    }

    Ok(())
}

/// Full game launch pipeline.
pub async fn launch(params: LaunchPipelineParams<'_>) -> Result<LaunchStartResult, GameError> {
    let LaunchPipelineParams {
        client,
        settings,
        mc_folder,
        instance_id,
        session,
        task_center,
        watcher,
    } = params;

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
    let (selected_java, version_json) = {
        let mc_folder_buf = mc_folder.to_path_buf();
        let inst_id = instance_id.to_string();
        let custom_javas = settings.java.custom.clone();
        let settings_clone = settings.clone();
        let cfg_clone = instance_config.clone();
        tokio::task::spawn_blocking(move || {
            let version_json = load_resolved_version_json(&mc_folder_buf, &inst_id)?;
            let mc_version = extract_mc_version(&version_json, &inst_id);
            let constraint = version_java_constraint(&version_json, &mc_version);
            let mut installed_javas = crate::java::cached_java();
            if installed_javas.is_empty() {
                installed_javas = scan_java(&custom_javas);
            }
            let component = version_java_component(&version_json);
            let selected_java = match resolve_java_installation_with_component(
                &installed_javas,
                &constraint,
                &settings_clone,
                &cfg_clone,
                component.as_deref(),
            ) {
                Ok(java) => java,
                // A stale cache may predate a Java the user installed afterwards:
                // rescan once before reporting the requirement as unmet.
                Err(err) => {
                    debug!("Java resolution from cache failed ({err}); rescanning installs");
                    let rescanned = scan_java(&custom_javas);
                    resolve_java_installation_with_component(
                        &rescanned,
                        &constraint,
                        &settings_clone,
                        &cfg_clone,
                        component.as_deref(),
                    )?
                }
            };
            info!(
                "Launching instance {inst_id} with Java {} ({}) for constraint {:?}",
                selected_java.version, selected_java.path, constraint
            );
            Ok::<_, GameError>((selected_java, version_json))
        })
        .await
        .map_err(|e| GameError::LaunchProcessFailed(e.to_string()))??
    };
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
    let natives_dir = {
        let mc_folder_buf = mc_folder.to_path_buf();
        let inst_id = instance_id.to_string();
        let v_json = version_json.clone();
        tokio::task::spawn_blocking(move || extract_natives(&mc_folder_buf, &inst_id, &v_json))
            .await
            .map_err(|e| GameError::LaunchProcessFailed(e.to_string()))??
    };

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
    let spec = build_launch_command(&LaunchCommandInput {
        mc_folder,
        instance_id,
        version_json: &version_json,
        java: &selected_java,
        launch_settings: &settings.launch,
        game_settings: &settings.game,
        instance_config: &instance_config,
        session,
        natives_dir: &natives_dir,
        authlib_injector_jar: authlib_jar.as_deref(),
    })?;

    // 7. Pre-launch command (instance override wins over the global default)
    let pre_cmd = instance_config
        .pre_launch_command
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(settings.launch.pre_launch_command.as_str());
    if !pre_cmd.trim().is_empty() {
        execute_pre_launch_command_async(
            pre_cmd,
            &spec.java_executable,
            mc_folder,
            &instance_dir,
            &session.username,
        )
        .await?;
    }

    // 7b. Windows GPU preference so the game uses the discrete GPU.
    if settings.launch.require_gpu_high_performance {
        crate::launch::win32::set_gpu_high_performance(&spec.java_executable);
    }

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

    if let Some(pid) = child.id() {
        crate::launch::win32::apply_process_priority(pid, &settings.launch.process_priority);
    }

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
