#![feature(iter_intersperse)]
#![feature(pathbuf_into_string)]

use anyhow::{Ok, Result};
use clap::Parser;
use rust_decimal::Decimal;
use tabled::{builder::Builder, settings::Style};
use termimad::crossterm::style::Stylize;
use env_logger::Env;
use tokio;

mod agent_loop;
mod agent_tools;
mod api;
mod cli;
mod config;
// mod gateway;
mod sessions;
mod system_prompt;
mod tui;
mod utils;

use cli::Cli;
use config::AppConfig;
use log::{info, trace};
use agent_tools::ToolRegistry;

use crate::{api::History, utils::{format_number, parse_human_number}};

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
        agent_loop::run_agent(
            history,
            tui::on_assistant_message,
            tui::on_assistant_status_update,
            tui::on_system_message
        ).await?;
    }
    
    match cli.sub_command {
        // Some(cli::Commands::Gateway) => {
        //     info!("Starting gateway...");
        //     gateway::whatsapp::start().await?;
        // },
        Some(cli::MainSubCommands::Model { sub_command }) => {
            match sub_command {
                Some(cli::ModelSubCommands::List { max_price, min_context }) => {
                    list_models(max_price, min_context).await?;
                },
                Some(cli::ModelSubCommands::Show) => {
                    let mut table = Builder::new();
                    table.push_record(["Base URL", &AppConfig::global().model.base_url]);
                    table.push_record(["Name", &AppConfig::global().model.name]);
                    table.push_record(["Reasoning", &AppConfig::global().model.reasoning]);

                    println!("{}", table.build().with(Style::blank()));
                },
                None => {
                    println!("No subcommand provided. Use --help for usage.");
                }
            }
        },
        Some(cli::MainSubCommands::Tools) => {
            println!("Available Tools:");
            for (tool_name, is_available, reason) in ToolRegistry::tools_status() {
                println!(
                    "    {} {:15} {} {}",
                    ToolRegistry::tool_icon(&tool_name), tool_name, if is_available { "✔".green() } else { "✘".red() }, reason.red()
                );
            }
        },
        Some(cli::MainSubCommands::Tui { new }) => {
            info!("Starting TUI...");
            tui::run(new).await?;
        },
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}


async fn list_models(max_price: Option<f64>, min_context: Option<String>) -> Result<()>{
    let models = api::models().await?;

    if models.is_empty() {
        println!("No models available.");
        return Ok(());
    }

    let has_context = models.iter().any(|m| m.context_length.is_some());
    let has_pricing = models.iter().any(|m| m.pricing.is_some());

    let mut table = Builder::new();

    let mut headers = vec!["ID".to_string()];
    if has_context { headers.push("Context Length".to_string()); }
    if has_pricing { headers.push("Price/M out".to_string()); }
    table.push_record(headers);

    for model in models {
        let mut record = vec![model.id];

        if has_context {
            // Skip items that have low context length
            if let Some(min_context) = &min_context {
                if let Some(context_length) = model.context_length {
                    if context_length < parse_human_number(min_context)? {
                        continue;
                    }
                }
            }

            record.push(match model.context_length {
                Some(n) => format_number(n),
                None => "".to_string(),
            });
        }

        if has_pricing {
            // Skip items that have high pricing
            if let Some(max_price) = &max_price {
                if let Some(pricing) = &model.pricing {
                    if pricing.completion > max_price.to_string() {
                        continue;
                    }
                }
            }

            record.push(
                model.pricing
                .and_then(|p| p.completion.parse::<Decimal>().ok())
                .map(|p| if p == Decimal::from(-1) { "".to_string() } else { format!("${}", (p * Decimal::from(1_000_000)).normalize()) })
                .unwrap_or_default()
            );
        }

        table.push_record(record);
    }

    println!("{}", table.build().with(Style::rounded()));

    Ok(())
}