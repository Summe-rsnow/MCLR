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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Info => {
            let info = AppInfo::current();
            println!("{}", serde_json::to_string(&info)?);
        }
    }
    Ok(())
}
