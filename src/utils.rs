use log::{debug, info};
use termimad::crossterm::style::ResetColor;
use textwrap;
use std::{cmp::{max, min}, fs, io::{Write, stdout}, path::PathBuf};
use anyhow::Result;
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, highlighting::Style ,util::{LinesWithEndings, as_24_bit_terminal_escaped}};

use crate::api::History;
use crate::config::AppConfig;

pub fn generate_think_lines(thinking: &str) -> String {
    let width = textwrap::termwidth() - 6;
    "    ╎ ".to_string() + &textwrap::wrap(thinking, width).join("\n    ╎ ")
}

pub fn ask_permission(header: impl ToString, content: &str) -> bool {
    let header = header.to_string();
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
    input = input.trim().to_string().to_lowercase();

    if input == "y" || input == "yes" {
        true
    } else {
        false
    }
}

/// Returns colored text based on the file extension from the path
pub fn hilight_text(filename: &str, text: &str) -> String {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Get syntax reference based on file extension
    let pathbuf = PathBuf::from(filename);
    let extension = pathbuf.extension().map(|s|s.to_str().unwrap()).unwrap_or("txt");
    let syntax = ps.find_syntax_by_extension(extension).unwrap_or(ps.find_syntax_plain_text());

    // Highlight the content (copied straight from docs example)
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-eighties.dark"]);
    let mut colored_text = String::new();
    for line in LinesWithEndings::from(text) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        colored_text.push_str(&escaped);
    }
    colored_text.push_str(&ResetColor.to_string());

    return colored_text;
}

#[cfg(test)]
mod tests {
    use super::*;

    use termimad::crossterm::style::Stylize;

    #[test]
    fn test_hilight_text() {
        let text = "
import os
print('hello world')
        ";

        ask_permission(
            "Execute?".red(),
            &hilight_text("some/python.py", text)
        );   
    }

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