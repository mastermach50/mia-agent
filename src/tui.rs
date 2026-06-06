use std::io::stdout;
use std::{fs, io::Write};
use anyhow::Result;
use colored::Colorize;

use crate::agent_tools::ToolRegistry;
use crate::utils::generate_think_lines;
use crate::config::AppConfig;
use crate::agent_loop;
use crate::api::{History, Message};

pub async fn run() -> Result<()> {

    let mut history = History::new();
    let mut system_prompt = String::new();
    let soul = fs::read_to_string(&AppConfig::global().documents.soul)?;
    system_prompt.push_str(&soul);
    system_prompt.push_str(&format!("\nYou are talking to {} via a TUI.", AppConfig::global().cli.username));
    history.set_system_prompt(system_prompt);
    
    println!("{} > Use {} to exit the chat", "System".yellow(), "/exit".yellow());
    loop {
        print!("{} > ", "User".blue());
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        input = input.trim().to_string();

        if input == "/exit"  {
            break;
        }

        history.add_message(Message::new("user", input));

        print!("{}  > Thinking...\r", "Mia".red());
        stdout().flush()?;

        history = agent_loop::run_agent(history, message_printer).await?;
    }
    Ok(())
}

pub fn message_printer(message: &Message) {
    if let Some(reasoning) = message.reasoning.clone() {
        println!("{}  > 💭             ", "Mia".red());
        println!("{}", generate_think_lines(reasoning.trim()))
    }
    if let Some(content) = message.content.clone() {
        if content.trim() != "" {
            println!("{}  > {}", "Mia".red(), content.trim());
        }
    }
    if let Some(tool_calls) = message.tool_calls.clone() {
        for tool_call in tool_calls {
            println!(
                "{}  > {} {}: {}",
                "Mia".red(),
                ToolRegistry::tool_icon(&tool_call.function.name),
                tool_call.function.name,
                serde_json::to_string(&tool_call.function.arguments).unwrap()
            );
        }
    }
}