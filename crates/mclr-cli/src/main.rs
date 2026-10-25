#![allow(clippy::collapsible_if)]
//! Dev and headless smoke CLI for MCLR.

use clap::{Parser, Subcommand};
use mclr_core::{AppInfo, paths};
use mclr_net::build_http_client;
use serde::Serialize;

async fn resolve_loader_ver(
    client: &reqwest::Client,
    loader: mclr_loaders::LoaderKind,
    mc: &str,
    req_ver: Option<String>,
    name: &str,
) -> anyhow::Result<Option<String>> {
    if let Some(v) = req_ver {
        if v == "latest" || v.is_empty() {
            println!("Fetching latest {} version for Minecraft {}...", name, mc);
            let versions = mclr_loaders::list_loader_versions(client, loader, mc).await?;
            if versions.is_empty() {
                anyhow::bail!("No {} versions found for Minecraft {}", name, mc);
            }
            let picked = versions[0].version.clone();
            println!("Selected {} version {}", name, picked);
            Ok(Some(picked))
        } else {
            Ok(Some(v))
        }
    } else {
        Ok(None)
    }
}

async fn fetch_modrinth_extra(
    client: &reqwest::Client,
    slug: &str,
    mc: &str,
    loader: Option<&str>,
) -> anyhow::Result<Option<mclr_loaders::ExtraMod>> {
    println!("Fetching add-on {} for Minecraft {}...", slug, mc);
    let mut url =
        format!("https://api.modrinth.com/v2/project/{slug}/version?game_versions=[\"{mc}\"]");
    if let Some(l) = loader {
        url.push_str(&format!("&loaders=[\"{l}\"]"));
    }
    let res = client.get(&url).send().await?;
    if !res.status().is_success() {
        return Ok(None);
    }
    let versions: serde_json::Value = res.json().await?;
    if let Some(arr) = versions.as_array() {
        if let Some(first) = arr.first() {
            if let Some(files) = first.get("files").and_then(|f| f.as_array()) {
                let picked_file = files
                    .iter()
                    .find(|f| f.get("primary").and_then(|p| p.as_bool()).unwrap_or(false))
                    .or_else(|| files.first());
                if let Some(file_obj) = picked_file {
                    let file_name = file_obj
                        .get("filename")
                        .and_then(|f| f.as_str())
                        .unwrap_or("")
                        .to_string();
                    let url_str = file_obj.get("url").and_then(|f| f.as_str()).unwrap_or("");
                    let size = file_obj.get("size").and_then(|s| s.as_u64());
                    let sha1 = file_obj
                        .get("hashes")
                        .and_then(|h| h.get("sha1"))
                        .and_then(|s| s.as_str())
                        .map(ToString::to_string);
                    if let Ok(parsed_url) = url::Url::parse(url_str) {
                        return Ok(Some(mclr_loaders::ExtraMod {
                            file_name,
                            urls: vec![parsed_url.to_string()],
                            sha1,
                            size,
                        }));
                    }
                }
            }
        }
    }
    Ok(None)
}

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

    /// Instance management commands
    Instances {
        #[command(subcommand)]
        subcommand: InstancesCommands,
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
        /// Install Legacy Fabric loader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        legacy_fabric: Option<String>,
        /// Install Forge loader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        forge: Option<String>,
        /// Install NeoForge loader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        neoforge: Option<String>,
        /// Install Cleanroom loader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        cleanroom: Option<String>,
        /// Install OptiFine (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        optifine: Option<String>,
        /// Install LiteLoader (optional version, defaults to latest)
        #[arg(long, num_args = 0..=1, default_missing_value = "latest")]
        liteloader: Option<String>,
        /// Install Fabric API add-on
        #[arg(long)]
        fabric_api: bool,
        /// Install Legacy Fabric API add-on
        #[arg(long)]
        legacy_fabric_api: bool,
        /// Install OptiFabric add-on
        #[arg(long)]
        optifabric: bool,
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

#[derive(Subcommand)]
enum InstancesCommands {
    /// List instances in the default Minecraft folder
    List,
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
        Commands::Instances { subcommand } => match subcommand {
            InstancesCommands::List => {
                let dir = paths::default_minecraft_dir();
                mclr_game::instance::ensure_game_folder(&dir)?;
                for inst in mclr_game::scan_instances(&dir)? {
                    println!("{}\t{}\t{:?}", inst.id, inst.mc_version, inst.kind);
                }
            }
        },
        Commands::Install {
            mc,
            name,
            fabric,
            legacy_fabric,
            forge,
            neoforge,
            cleanroom,
            optifine,
            liteloader,
            fabric_api,
            legacy_fabric_api,
            optifabric,
        } => {
            let default_dir = paths::default_minecraft_dir();
            let settings = mclr_core::Settings::default();
            let client = build_http_client(&settings.net)?;

            let fabric_ver = resolve_loader_ver(
                &client,
                mclr_loaders::LoaderKind::Fabric,
                &mc,
                fabric,
                "Fabric",
            )
            .await?;
            let legacy_fabric_ver = resolve_loader_ver(
                &client,
                mclr_loaders::LoaderKind::LegacyFabric,
                &mc,
                legacy_fabric,
                "Legacy Fabric",
            )
            .await?;
            let forge_ver = resolve_loader_ver(
                &client,
                mclr_loaders::LoaderKind::Forge,
                &mc,
                forge,
                "Forge",
            )
            .await?;
            let neoforge_ver = resolve_loader_ver(
                &client,
                mclr_loaders::LoaderKind::NeoForge,
                &mc,
                neoforge,
                "NeoForge",
            )
            .await?;
            let cleanroom_ver = resolve_loader_ver(
                &client,
                mclr_loaders::LoaderKind::Cleanroom,
                &mc,
                cleanroom,
                "Cleanroom",
            )
            .await?;
            let optifine_ver = resolve_loader_ver(
                &client,
                mclr_loaders::LoaderKind::OptiFine,
                &mc,
                optifine,
                "OptiFine",
            )
            .await?;
            let liteloader_ver = resolve_loader_ver(
                &client,
                mclr_loaders::LoaderKind::LiteLoader,
                &mc,
                liteloader,
                "LiteLoader",
            )
            .await?;

            let mut extra_mods = Vec::new();
            if fabric_api {
                if let Some(extra) =
                    fetch_modrinth_extra(&client, "fabric-api", &mc, Some("fabric")).await?
                {
                    println!("Added Fabric API: {}", extra.file_name);
                    extra_mods.push(extra);
                }
            }
            if legacy_fabric_api {
                if let Some(extra) =
                    fetch_modrinth_extra(&client, "legacy-fabric-api", &mc, None).await?
                {
                    println!("Added Legacy Fabric API: {}", extra.file_name);
                    extra_mods.push(extra);
                }
            }
            if optifabric {
                if let Some(extra) =
                    fetch_modrinth_extra(&client, "optifabric", &mc, Some("fabric")).await?
                {
                    println!("Added OptiFabric: {}", extra.file_name);
                    extra_mods.push(extra);
                }
            }

            let req = mclr_loaders::InstallRequest {
                mc_version: mc.clone(),
                instance_name: name.clone(),
                forge: forge_ver,
                neoforge: neoforge_ver,
                cleanroom: cleanroom_ver,
                fabric: fabric_ver,
                legacy_fabric: legacy_fabric_ver,
                labymod: None,
                liteloader: liteloader_ver,
                optifine: optifine_ver,
                extra_mods,
                mode: mclr_loaders::InstallMode::Create,
            };

            let env = mclr_loaders::InstallEnv {
                client: client.clone(),
                net_settings: settings.net.clone(),
                java_settings: settings.java.clone(),
                game_settings: settings.game.clone(),
                mc_folder: default_dir.clone(),
                task_ctx: None,
            };

            let instance = mclr_loaders::install_instance(req, env).await?;

            let summary = InstallSummary {
                instance,
                repaired_count: 0,
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
            let result = mclr_game::launch(mclr_game::LaunchPipelineParams {
                client: &client,
                settings: &settings,
                mc_folder: &default_dir,
                instance_id: &instance,
                session: &session,
                task_center: None,
                watcher: &watcher,
            })
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
