//! Dev and headless smoke CLI for MCLR.

use clap::{Parser, Subcommand};
use mclr_core::{AppInfo, NetSettings, paths};
use mclr_game::{install_vanilla, repair_instance};
use mclr_net::build_http_client;
use serde::Serialize;

#[derive(Parser)]
#[command(name = "mclr-cli", version, about = "MCLR headless CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print application info as JSON
    Info,

    /// Java runtime management commands
    Java {
        #[command(subcommand)]
        subcommand: JavaCommands,
    },

    /// Install vanilla Minecraft instance
    Install {
        /// Minecraft version (e.g. 1.20.1)
        mc: String,
        /// Instance name
        name: String,
    },

    /// Launch Minecraft instance
    Launch {
        /// Instance name
        instance: String,
        /// Offline username
        #[arg(long)]
        offline: String,
        /// Stop process cleanly once this pattern appears in stdout/stderr log
        #[arg(long)]
        stop_on_log: Option<String>,
        /// Timeout in seconds before giving up
        #[arg(long, default_value = "45")]
        timeout_secs: u64,
    },
}

#[derive(Subcommand)]
enum JavaCommands {
    /// Scan the system for Java installations and print as JSON
    Scan,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallSummary {
    instance: mclr_game::InstanceInfo,
    repaired_count: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Info => {
            let info = AppInfo::current();
            println!("{}", serde_json::to_string(&info)?);
        }
        Commands::Java { subcommand } => match subcommand {
            JavaCommands::Scan => {
                let installations = mclr_game::java::scan_java(&[]);
                println!("{}", serde_json::to_string_pretty(&installations)?);
            }
        },
        Commands::Install { mc, name } => {
            let default_dir = paths::default_minecraft_dir();
            let net_settings = NetSettings::default();
            let client = build_http_client(&net_settings)?;

            let instance =
                install_vanilla(&client, &net_settings, &default_dir, &mc, &name, None).await?;

            let repaired_count =
                repair_instance(&client, &net_settings, &default_dir, &name, None).await?;

            let summary = InstallSummary {
                instance,
                repaired_count,
            };

            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Commands::Launch {
            instance,
            offline,
            stop_on_log,
            timeout_secs,
        } => {
            let default_dir = paths::default_minecraft_dir();
            let settings = mclr_core::Settings::default();
            let client = build_http_client(&settings.net)?;

            let uuid = mclr_auth::compute_offline_uuid(
                &offline,
                mclr_auth::OfflineUuidMode::Standard,
                None,
            )?;
            let session = mclr_core::account_session::ResolvedAccountSession {
                id: format!("offline-{}", uuid),
                kind: mclr_core::account_session::AccountKind::Offline,
                username: offline.clone(),
                uuid,
                access_token: "0".to_string(),
                user_type: "legacy".to_string(),
                api_location: None,
                xuid: None,
            };

            let watcher = std::sync::Arc::new(mclr_game::LaunchWatcher::new());
            let mut event_rx = watcher.subscribe();

            println!(
                "Launching instance '{}' with offline user '{}'...",
                instance, offline
            );
            let result = mclr_game::launch(
                &client,
                &settings,
                &default_dir,
                &instance,
                &session,
                None,
                &watcher,
            )
            .await?;
            let pid = result.process_id.unwrap_or(0);
            println!("Process launched (PID: {})", pid);

            let stop_pattern = stop_on_log.clone();
            let timeout_duration = std::time::Duration::from_secs(timeout_secs);

            let run_watch = async {
                while let Ok(event) = event_rx.recv().await {
                    match event {
                        mclr_game::WatcherEvent::Log(line) => {
                            println!("[{}] {}", line.level, line.text);
                            if let Some(pat) = &stop_pattern {
                                if line.text.contains(pat) {
                                    println!(
                                        "Target log pattern '{}' matched! Terminating process cleanly...",
                                        pat
                                    );
                                    if pid > 0 {
                                        let _ = watcher.kill(pid).await;
                                    }
                                    return Ok(());
                                }
                            } else if line.text.contains("Setting user:") {
                                println!(
                                    "Default pattern 'Setting user:' matched! Terminating process cleanly..."
                                );
                                if pid > 0 {
                                    let _ = watcher.kill(pid).await;
                                }
                                return Ok(());
                            }
                        }
                        mclr_game::WatcherEvent::Exited(payload) => {
                            println!("Process exited with code: {}", payload.exit_code);
                            if payload.exit_code != 0 {
                                anyhow::bail!(
                                    "Minecraft process crashed with exit code {}",
                                    payload.exit_code
                                );
                            }
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                Ok(())
            };

            match tokio::time::timeout(timeout_duration, run_watch).await {
                Ok(res) => res?,
                Err(_) => {
                    if pid > 0 {
                        let _ = watcher.kill(pid).await;
                    }
                    anyhow::bail!("Launch timed out after {} seconds", timeout_secs);
                }
            }
        }
    }
    Ok(())
}
