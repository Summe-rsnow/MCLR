//! Dev and headless smoke CLI for MCLR.

use clap::{Parser, Subcommand};
use mclr_core::{AppInfo, paths};
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

    /// Install vanilla or modded Minecraft instance
    Install {
        /// Minecraft version (e.g. 1.20.1)
        mc: String,
        /// Instance name
        name: String,
        /// Install Fabric loader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        fabric: Option<String>,
        /// Install Forge loader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        forge: Option<String>,
        /// Install NeoForge loader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        neoforge: Option<String>,
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

    /// Update management commands
    Update {
        #[command(subcommand)]
        subcommand: UpdateCommands,
    },
}

#[derive(Subcommand)]
enum UpdateCommands {
    /// Check for launcher updates
    Check {
        /// Channel override (stable or beta)
        #[arg(long, default_value = "stable")]
        channel: String,
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
        Commands::Install {
            mc,
            name,
            fabric,
            forge,
            neoforge,
        } => {
            let default_dir = paths::default_minecraft_dir();
            let settings = mclr_core::Settings::default();
            let client = build_http_client(&settings.net)?;

            let loader_spec = if let Some(ver) = fabric {
                Some((mclr_loaders::LoaderKind::Fabric, ver, "Fabric"))
            } else if let Some(ver) = forge {
                Some((mclr_loaders::LoaderKind::Forge, ver, "Forge"))
            } else {
                neoforge.map(|ver| (mclr_loaders::LoaderKind::NeoForge, ver, "NeoForge"))
            };

            let instance = if let Some((loader_kind, ver_req, loader_name)) = loader_spec {
                let loader_version = if ver_req == "latest" || ver_req.is_empty() {
                    println!(
                        "Fetching latest {} version for Minecraft {}...",
                        loader_name, mc
                    );
                    let versions =
                        mclr_loaders::list_loader_versions(&client, loader_kind, &mc).await?;
                    if versions.is_empty() {
                        anyhow::bail!("No {} versions found for Minecraft {}", loader_name, mc);
                    }
                    let picked = versions[0].version.clone();
                    println!("Selected {} version {}", loader_name, picked);
                    picked
                } else {
                    ver_req
                };

                let ctx = mclr_loaders::LoaderInstallContext {
                    client: &client,
                    net_settings: &settings.net,
                    java_settings: &settings.java,
                    mc_folder: default_dir.clone(),
                    mc_version: mc.clone(),
                    instance_name: name.clone(),
                    loader_version,
                    optifine_version: None,
                    task_ctx: None,
                };

                mclr_loaders::install_with_loader(loader_kind, ctx).await?
            } else {
                install_vanilla(&client, &settings.net, &default_dir, &mc, &name, None).await?
            };

            let repaired_count =
                repair_instance(&client, &settings.net, &default_dir, &name, None).await?;

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
        Commands::Update { subcommand } => match subcommand {
            UpdateCommands::Check { channel } => {
                match mclr_update::UpdateConfig::from_runtime(&channel) {
                    Ok(config) => {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(15))
                            .build()?;
                        let current_version = env!("CARGO_PKG_VERSION");
                        match mclr_update::check_update_manifest(&client, &config, current_version)
                            .await
                        {
                            Ok(result) => {
                                println!("{}", serde_json::to_string_pretty(&result)?);
                            }
                            Err(e) => {
                                let app_error: mclr_core::AppError = e.into();
                                println!("{}", serde_json::to_string_pretty(&app_error)?);
                            }
                        }
                    }
                    Err(e) => {
                        let app_error: mclr_core::AppError = e.into();
                        println!("{}", serde_json::to_string_pretty(&app_error)?);
                    }
                }
            }
        },
    }
    Ok(())
}
