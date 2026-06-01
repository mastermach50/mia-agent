use anyhow::{Ok, Result};
use clap::Parser;

mod config;
mod tui;
mod cli;
use cli::Cli;
use config::AppConfig;

fn main() -> Result<()>{
    let cli = Cli::parse();
    
    AppConfig::load()?;
    
    match cli.command {
        Some(cli::Commands::Gateway) => {
            println!("Starting gateway...");
            // TODO start gateway server
        },
        Some(cli::Commands::Tui) => {
            println!("Starting TUI...");
            tui::run()?;
        },
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}
