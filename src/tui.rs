use std::io::stdout;
use std::{fs, io::Write};
use anyhow::Result;
use colored::Colorize;
use termimad;

use crate::agent_tools::ToolRegistry;
use crate::utils::{generate_think_lines, load_history, save_history};
use crate::config::AppConfig;
use crate::agent_loop;
use crate::api::{History, Message};

pub async fn run() -> Result<()> {

    // Try to load the history from file
    // If it doesn't exist, create a new one
    let mut history = History::new();
    if let Ok(loaded_history) = load_history("tui-agent-history.json") {
        history = loaded_history;
    } else {
        let mut system_prompt = String::new();
        let soul = fs::read_to_string(&AppConfig::global().documents.soul)?;
        system_prompt.push_str(&soul);
        system_prompt.push_str(&format!("\nYou are talking to {} via a TUI.", AppConfig::global().cli.username));
        history.set_system_prompt(system_prompt);
    }
    
    println!("{} > Use {} to exit the chat, {} to start a new session.", "System".yellow(), "/exit".yellow(), "/new".yellow());
    loop {
        print!("{} > ", "User".blue());
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input = input.trim().to_string();

        match input.as_str() {
            "/exit" => {
                save_history("tui-agent-history.json", &history)?;
                break;
            }
            "/new" => {
                history = History::new();
                let mut system_prompt = String::new();
                let soul = fs::read_to_string(&AppConfig::global().documents.soul)?;
                system_prompt.push_str(&soul);
                system_prompt.push_str(&format!("\nYou are talking to {} via a TUI.", AppConfig::global().cli.username));
                history.set_system_prompt(system_prompt);
            }
            _ => {}
        }

        history.add_message(Message::new("user", input));

        print!("{}  > Thinking...\r", "Mia".red());
        stdout().flush()?;

        history = agent_loop::run_agent(history, message_printer).await?;
    }
    Ok(())
}

pub fn message_printer(message: &Message) {
    let mut output = String::new();
    if let Some(reasoning) = message.reasoning.clone() {
        output += &format!("{}  > 💭             \n", "Mia".red());
        output += &format!("{}\n", generate_think_lines(reasoning.trim()));
    }
    if let Some(content) = message.content.clone() {
        if content.trim() != "" {
            output += &format!("{}  > {}\n", "Mia".red(), content.trim());
        }
    }
    if let Some(tool_calls) = message.tool_calls.clone() {
        for tool_call in tool_calls {
            output += &format!(
                "{}  > {} {}: {}\n",
                "Mia".red(),
                ToolRegistry::tool_icon(&tool_call.function.name),
                tool_call.function.name,
                serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments).unwrap()
            );
        }
    }

    termimad::print_inline(&output);
}