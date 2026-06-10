#![feature(iter_intersperse)]
#![feature(pathbuf_into_string)]

use anyhow::{Ok, Result};
use clap::Parser;
use termimad::crossterm::style::Stylize;
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
// mod gateway;

use cli::Cli;
use config::AppConfig;
use log::{info, trace};
use agent_tools::ToolRegistry;

use crate::api::History;

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

    if let Some(command) = cli.command {
        let mut history = History::new();
        history.set_system_prompt(tui::get_tui_system_prompt()?);
        history.add_message(api::Message::new("user", &command));
        agent_loop::run_agent(history, tui::message_printer, tui::thinking_printer).await?;
    }
    
    match cli.sub_command {
        // Some(cli::Commands::Gateway) => {
        //     info!("Starting gateway...");
        //     gateway::whatsapp::start().await?;
        // },
        Some(cli::SubCommands::Tui { new }) => {
            info!("Starting TUI...");
            tui::run(new).await?;
        },
        Some(cli::SubCommands::Tools) => {
            println!("Available Tools:");
            for (tool_name, is_available, reason) in ToolRegistry::tools_status() {
                println!(
                    "    {} {:15} {} {}",
                    ToolRegistry::tool_icon(&tool_name), tool_name, if is_available { "✔".green() } else { "✘".red() }, reason.red()
                );
            }
        },
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}
