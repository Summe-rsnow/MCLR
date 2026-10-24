//! Dev and headless smoke CLI for MCLR.

use clap::{Parser, Subcommand};
use mclr_core::AppInfo;

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
}

#[derive(Subcommand)]
enum JavaCommands {
    /// Scan the system for Java installations and print as JSON
    Scan,
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
    }
    Ok(())
}
