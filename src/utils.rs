use termimad::crossterm::style::ResetColor;
use textwrap;
use std::{cmp::{max, min}, io::{Read, Write, stdout}, path::PathBuf, process::Child};
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet, highlighting::Style ,util::{LinesWithEndings, as_24_bit_terminal_escaped}};


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
pub fn highlight_text(filename: &str, text: &str) -> String {
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
            &highlight_text("some/python.py", text)
        );   
    }

    #[test]
    fn test_permission_prompt() {
        let _ = ask_permission("Execute?".red(), "Hi");
    }
}

pub fn stdio_capture_and_print(child: &mut Child) -> (String, String) {
    let mut stdout_captured = String::new();
    let mut stderr_captured = String::new();

    if let Some(mut stdout) = child.stdout.take() {
        let mut buffer = [0; u8::MAX as usize];
        while let Ok(bytes_read) = stdout.read(&mut buffer) {
            if bytes_read == 0 { break; }
            if let Ok(text) = std::str::from_utf8(&buffer[..bytes_read]) {
                print!("{}", text);
                std::io::Write::flush(&mut std::io::stdout()).unwrap(); // Force instant print
                stdout_captured.push_str(text);
            }
        }
    }

    if let Some(mut stderr) = child.stderr.take() {
        let mut buffer = [0; u8::MAX as usize];
        while let Ok(bytes_read) = stderr.read(&mut buffer) {
            if bytes_read == 0 { break; }
            if let Ok(text) = std::str::from_utf8(&buffer[..bytes_read]) {
                eprint!("{}", text);
                std::io::Write::flush(&mut std::io::stderr()).unwrap();
                stderr_captured.push_str(text);
            }
        }
    }

    (stdout_captured, stderr_captured)
}