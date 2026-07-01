#![feature(iter_intersperse)]
#![feature(pathbuf_into_string)]
#![feature(file_buffered)]

use anyhow::{Context, Ok, Result};
use clap::Parser;
use env_logger::Env;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use tabled::{Table, builder::Builder, settings::Style};
use termimad::crossterm::style::Stylize;

mod agent_loop;
mod agent_tools;
mod api;
mod cli;
mod config;
mod sessions;
mod setup;
mod system_prompt;
mod tui;
mod utils;

mod ratatuitui;

use agent_tools::ToolRegistry;
use cli::Cli;
use config::AppConfig;
use log::{info, trace};
use uuid::Uuid;

use crate::{
    api::History,
    sessions::list_sessions,
    setup::Providers,
    system_prompt::get_tui_system_prompt,
    utils::{format_number, parse_human_number},
};

#[tokio::main]
async fn main() -> Result<()> {
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

    if let Some(command) = cli.command {
        let mut history = History::new();
        history.set_system_prompt(get_tui_system_prompt(None)?);
        history.add_message(api::Message::new("user", &command));
        let session_id = format!("mia-agent_{}", Uuid::now_v7().to_string());
        agent_loop::run_agent(
            history,
            &session_id,
            AppConfig::global().tui.streaming,
            tui::on_assistant_message,
            tui::on_partial_assistant_message,
            tui::on_assistant_status_update,
            tui::on_system_message,
        )
        .await?;
    }

    match cli.sub_command {
        Some(cli::MainSubCommands::Model { sub_command }) => match sub_command {
            Some(cli::ModelSubCommands::List(args)) => {
                list_models(args).await?;
            }
            Some(cli::ModelSubCommands::Show) => {
                println!("Model     : {}", AppConfig::global().model.name);
                println!("Reasoning : {}", AppConfig::global().model.reasoning);
                println!("Base URL  : {}", AppConfig::global().model.base_url);
            }
            None => {
                println!("No subcommand provided. Use --help for usage.");
            }
        },
        Some(cli::MainSubCommands::Session { sub_command }) => match sub_command {
            Some(cli::SessionSubCommands::List) => {
                let sessions = list_sessions(true)?;
                let table = Table::builder(sessions);
                println!("{}", table.build().with(Style::rounded()));
            }
            Some(cli::SessionSubCommands::Clear) => {
                let clear = inquire::Confirm::new("Clear all sessions?")
                    .with_default(false)
                    .prompt()?;
                if clear {
                    let sessions_dir = AppConfig::internal().sessions_dir.clone();
                    for file in sessions_dir.read_dir()? {
                        let file = file?;
                        std::fs::remove_file(file.path()).context(format!(
                            "Failed to delete {}",
                            file.file_name().to_string_lossy()
                        ))?;
                    }
                    println!("All sessions cleared.");
                }
            }
            None => {
                println!("No subcommand provided. Use --help for usage.");
            }
        },
        Some(cli::MainSubCommands::Setup) => {
            setup::setup().await?;
        }
        Some(cli::MainSubCommands::Tools) => {
            println!("Available Tools:");
            for (tool_name, is_available, reason) in ToolRegistry::tools_status() {
                println!(
                    "{} {:15} {} {}",
                    ToolRegistry::tool_icon(&tool_name),
                    tool_name,
                    if is_available {
                        "✔".green()
                    } else {
                        "✘".red()
                    },
                    reason.red()
                );
            }
        }
        Some(cli::MainSubCommands::Tui { new }) => {
            info!("Starting TUI...");
            tui::run(new).await?;
        }
        Some(cli::MainSubCommands::Ratatui { new }) => {
            info!("Starting Ratatui...");
            ratatuitui::run(new).await?;
        }
        None => {
            println!("No command provided. Use --help for usage.");
        }
    }

    Ok(())
}

async fn list_models(args: cli::ModelListArgs) -> Result<()> {
    let base_url = AppConfig::global().model.base_url.clone();
    let provider_name = AppConfig::global().model.provider.clone();
    let provider = Providers::from_name(&provider_name).unwrap();

    let models = api::models(&base_url, provider.api_key_name()).await?;

    if models.is_empty() {
        println!("No models available.");
        return Ok(());
    }

    let has_context = models.iter().any(|m| m.context_length.is_some());
    let has_pricing = models.iter().any(|m| m.pricing.is_some());

    let mut table = Builder::new();

    let mut headers = vec!["ID".to_string()];
    if has_context {
        headers.push("Context Length".to_string());
    }
    if has_pricing {
        headers.push("Price/M out".to_string());
    }
    table.push_record(headers);

    for model in models {
        let mut record = vec![model.id];

        if has_context {
            // Skip items that have low context length
            if let Some(min_context) = &args.min_context // Min ctxt arg is given
            && let Some(context_length) = model.context_length // Model has ctxt len info
            && context_length < parse_human_number(min_context)?
            {
                continue;
            }

            record.push(match model.context_length {
                Some(n) => format_number(n),
                None => "".to_string(),
            });
        }

        if has_pricing {
            // Skip items that have price greater than max_price
            let max_price = args
                .max_price
                .unwrap_or(if args.free { 0.0 } else { f64::INFINITY });
            if let Some(pricing) = &model.pricing // Model has pricing info
            && let Some(completion_price) = pricing.completion.parse::<Decimal>().ok()
            {
                // The price can be parsed properly
                let completion_price_per_mil = completion_price * Decimal::from(1_000_000);
                let max_price_per_mil =
                    Decimal::from_f64(max_price).expect("Failed to convert max price to decimal");
                if completion_price_per_mil > max_price_per_mil {
                    continue;
                }
            }

            record.push(
                model
                    .pricing
                    .and_then(|p| p.completion.parse::<Decimal>().ok())
                    .map(|p| {
                        if p == Decimal::from(-1) {
                            "".to_string()
                        } else {
                            format!("${}", (p * Decimal::from(1_000_000)).normalize())
                        }
                    })
                    .unwrap_or_default(),
            );
        }

        table.push_record(record);
    }

    println!("{}", table.build().with(Style::rounded()));

    Ok(())
}
