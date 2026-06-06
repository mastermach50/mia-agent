use log::info;
use textwrap;
use colored::{ColoredString};
use std::io::{Write, stdout};
use std::fs;
use std::env::home_dir;
use std::cmp::{max, min};
use anyhow::Result;

use crate::api::History;
use crate::config::AppConfig;

pub fn generate_think_lines(thinking: &str) -> String {
    let width = textwrap::termwidth() - 6;
    "    ╎ ".to_string() + &textwrap::wrap(thinking, width).join("\n    ╎ ")
}

pub fn ask_permission(header: impl Into<ColoredString>, content: &str) -> bool {
    let header = header.into();
    let width = textwrap::termwidth() - 4;
    let wrapped = textwrap::wrap(&content, width);

    let max_content = min( width, textwrap::core::display_width(content));
    let maxw = max(header.len(), max_content);

    print!("╭{}╮", "─".repeat(maxw + 2));
    println!("\r╭─{}", header);
    for line in wrapped {
        println!("│ {}{}│", line, " ".repeat(maxw - line.len() + 1));
    }
    print!("╰{}╯", "─".repeat(maxw + 2));
    print!("\r╰[y/n]: ");

    stdout().flush().unwrap();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    if input.trim().to_lowercase() == "y" {
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use colored::Colorize;

    use super::*;

    #[test]
    fn test_permission_prompt() {
        let _ = ask_permission("Execute?".red(), "Hi");
    }
}

pub fn save_history(filename: &str, history: &History) -> Result<()>{
    info!("Saving history to file");
    let sessions_dir = home_dir().unwrap().join(".mia/sessions");
    if !sessions_dir.exists() {
        fs::create_dir_all(&sessions_dir).unwrap();
    }
    let history_file = sessions_dir.join(filename);
    fs::write(history_file, serde_json::to_string_pretty(history).unwrap())?;
    Ok(())
}

pub fn load_history(filename: &str) -> Result<History> {
    info!("Loading history from file");
    let history_file = home_dir().unwrap().join(".mia/sessions").join(filename);
    if history_file.exists() {
        let history = fs::read_to_string(history_file)?;
        return Ok(serde_json::from_str(&history)?)
    } else {
        info!("History file not found");
        anyhow::bail!("History file not found");
    }
}

pub fn generate_system_prompt(system_prompt: &mut String) -> Result<&mut String> {
    let soul = fs::read_to_string(&AppConfig::global().documents.soul)?;
    system_prompt.push_str(&soul);
    let user_memory = fs::read_to_string(&AppConfig::global().documents.user_memory)?;
    let system_memory = fs::read_to_string(&AppConfig::global().documents.system_memory)?;
    system_prompt.push_str("\n");
    system_prompt.push_str("# Memory\n");
    system_prompt.push_str("## User Memory\n");
    system_prompt.push_str(&format!("These are the contents of the user related memory (USER.md)\n{}\n", user_memory));
    system_prompt.push_str("## System Memory\n");
    system_prompt.push_str(&format!("These are the contents of the system related memory (MEMORY.md)\n{}\n", system_memory));
    return Ok(system_prompt);
}