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
    }
    Ok(())
}
