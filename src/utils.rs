use log::{debug, info};
use textwrap;
use colored::{ColoredString};
use std::io::{Write, stdout};
use std::fs;
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
        let line_width = textwrap::core::display_width(&line);
        let padding = maxw.saturating_sub(line_width) + 1;
        println!("│ {}{}│", line, " ".repeat(padding));
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

pub fn save_session(filename: &str, history: &History) -> Result<()>{
    debug!("Saving history to file");
    let history_file = AppConfig::internal().sessions_dir.join(filename);
    fs::write(history_file, serde_json::to_string_pretty(history).unwrap())?;
    Ok(())
}

pub fn load_session(filename: &str) -> Result<History> {
    debug!("Loading history from file");
    let history_file = AppConfig::internal().mia_dir.join("sessions").join(filename);
    if history_file.exists() {
        let history = fs::read_to_string(history_file)?;
        return Ok(serde_json::from_str(&history)?)
    } else {
        info!("History file not found");
        anyhow::bail!("History file not found");
    }
}