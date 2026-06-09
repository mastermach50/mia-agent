#![feature(iter_intersperse)]
#![feature(pathbuf_into_string)]

use anyhow::{Ok, Result};
use clap::Parser;
use colored::Colorize;
use env_logger::Env;
use tokio;

mod config;
mod tui;
mod cli;
mod api;
mod utils;
mod agent_loop;
mod agent_tools;
mod system_prompt;
mod gateway;

use cli::Cli;
use config::AppConfig;
use log::{info, trace};
use agent_tools::ToolRegistry;

#[tokio::main]
async fn main() -> Result<()>{

    // Setup logging
    let env = Env::new().filter_or("MIA_LOG", "info");
    env_logger::init_from_env(env);

    // Parse CLI args
    let cli = Cli::parse();
    
    // Load configs from config.toml (to global cache)
    AppConfig::load()?;

    // Load agent tools
    ToolRegistry::init();

    // Print config
    trace!("Model: {:?}", AppConfig::global().model);
    trace!("Documents: {:?}", AppConfig::global().documents);
    
    match cli.command {
        Some(cli::Commands::Gateway) => {
            info!("Starting gateway...");
            gateway::whatsapp::start().await?;
        },
        Some(cli::Commands::Tui) => {
            info!("Starting TUI...");
            tui::run().await?;
        },
        Some(cli::Commands::Tools) => {
            println!("Available Tools:");
            for (tool_name, is_available) in ToolRegistry::tools_status() {
                println!(
                    "    {} {:15} {}",
                    ToolRegistry::tool_icon(&tool_name), tool_name, if is_available { "✔".green() } else { "✘".red() }
                );
            }
        },
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}
