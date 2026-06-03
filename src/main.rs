use anyhow::{Ok, Result};
use clap::Parser;
use env_logger::Env;
use tokio;

mod config;
mod tui;
mod cli;
mod api;
use cli::Cli;
use config::AppConfig;
use log::{info, trace};

#[tokio::main]
async fn main() -> Result<()>{

    // Setup logging
    let env = Env::new().filter_or("MIA_LOG", "info");
    env_logger::init_from_env(env);

    // Parse CLI args
    let cli = Cli::parse();
    
    // Load configs from config.toml (to global cache)
    AppConfig::load()?;

    trace!("Model: {:?}", AppConfig::global().model);
    trace!("Documents: {:?}", AppConfig::global().documents);
    
    match cli.command {
        Some(cli::Commands::Gateway) => {
            info!("Starting gateway...");
            // TODO start gateway server
        },
        Some(cli::Commands::Tui) => {
            info!("Starting TUI...");
            tui::run()?;
        },
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}
