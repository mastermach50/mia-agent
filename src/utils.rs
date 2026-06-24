use anyhow::Context;
use std::{
    cmp::max,
    io::{Read, Write, stdout},
    path::PathBuf,
    process::Child,
    sync::Mutex,
    time::Duration,
};
use syntect::{
    easy::HighlightLines,
    highlighting::Style,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};
use termimad::crossterm::style::{ResetColor, Stylize};
use textwrap::{self, core::display_width, termwidth, wrap};
use tokio::task::JoinHandle;

pub fn generate_think_lines(thinking: &str) -> String {
    let left_gap = "> ";
    let width = termwidth() - 2;
    left_gap.to_string() + &wrap(thinking, width).join(&("\n".to_owned() + left_gap))
}

pub fn ask_permission(prompt: impl ToString, content: &str) -> bool {
    let term_width = termwidth();
    let max_content_width = term_width - 4;

    let wrapped = wrap(content, max_content_width);

    // Prompt's width after wrapping
    let prompt_width = if display_width(&prompt.to_string()) > max_content_width {
        max_content_width
    } else {
        display_width(&prompt.to_string())
    };

    // Content's width after wrapping
    let content_width = wrapped
        .iter()
        .map(|l| display_width(l))
        .max()
        .unwrap_or(max_content_width);

    // The larger of prompt_width or content_width will be the inner width of the box (excluding padding)
    let inner_width = max(prompt_width, content_width);

    for line in wrap(&prompt.to_string(), max_content_width) {
        println!(
            "╭─{}{}─╮",
            line,
            "─".repeat(inner_width - display_width(&line))
        );
    }
    for line in wrapped {
        let line_width = display_width(&line);
        println!("│ {}{} │", line, " ".repeat(inner_width - line_width));
    }
    for line in wrap(&prompt.to_string(), max_content_width) {
        println!(
            "├─{}{}─╯",
            line,
            "─".repeat(inner_width - display_width(&line))
        );
    }
    print!("╰─[y/n]: ");
    stdout().flush().unwrap();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    input = input.trim().to_string().to_lowercase();

    input == "y" || input == "yes" || input.chars().all(|c| c == 'y')
}

/// Returns colored text based on the file extension from the path
pub fn highlight_text(filename: &str, text: &str) -> String {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Get syntax reference based on file extension
    let pathbuf = PathBuf::from(filename);
    let extension = pathbuf
        .extension()
        .map(|s| s.to_str().unwrap())
        .unwrap_or("txt");
    let syntax = ps
        .find_syntax_by_extension(extension)
        .unwrap_or(ps.find_syntax_plain_text());

    // Highlight the content (copied straight from docs example)
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-eighties.dark"]);
    let mut colored_text = String::new();
    for line in LinesWithEndings::from(text) {
        let ranges: Vec<(Style, &str)> = h.highlight_line(line, &ps).unwrap();
        let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
        colored_text.push_str(&escaped);
    }
    colored_text.push_str(&ResetColor.to_string());

    colored_text
}

pub fn stdio_capture_and_print(child: &mut Child) -> (String, String) {
    let mut stdout_captured = String::new();
    let mut stderr_captured = String::new();

    if let Some(mut stdout) = child.stdout.take() {
        let mut buffer = [0; u8::MAX as usize];
        while let Ok(bytes_read) = stdout.read(&mut buffer) {
            if bytes_read == 0 {
                break;
            }
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
            if bytes_read == 0 {
                break;
            }
            if let Ok(text) = std::str::from_utf8(&buffer[..bytes_read]) {
                eprint!("{}", text);
                std::io::Write::flush(&mut std::io::stderr()).unwrap();
                stderr_captured.push_str(text);
            }
        }
    }

    (stdout_captured, stderr_captured)
}

/// Format a number to a human readable form
/// Mainly used for formattiong model context length
pub fn format_number(n: i64) -> String {
    match n {
        n if n >= 1_000_000_000 => format!("{:.1}B", n as f64 / 1_000_000_000.0),
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
        n if n >= 1_000 => format!("{:.1}K", n as f64 / 1_000.0),
        n => n.to_string(),
    }
}

// Convert representations like 3k, 4.5M to i64
pub fn parse_human_number(s: &str) -> anyhow::Result<i64> {
    let s = s.trim();
    let (num_str, multiplier) = match s.chars().last() {
        Some('k') | Some('K') => (&s[..s.len() - 1], 1_000),
        Some('m') | Some('M') => (&s[..s.len() - 1], 1_000_000),
        Some('b') | Some('B') => (&s[..s.len() - 1], 1_000_000_000),
        _ => (s, 1),
    };

    let value: f64 = num_str.parse().context(format!("Invalid number: '{s}'"))?;

    Ok((value * multiplier as f64).round() as i64)
}

/// Join handle to the tokio task showing the spinner
static SPINNER: Mutex<Option<JoinHandle<()>>> = Mutex::new(None);

/// Start showing a thinking spinner
pub fn start_spinner(kind: &str) {
    // Stop any previous spinner
    if let Some(handle) = SPINNER.lock().unwrap().take() {
        handle.abort();
    }

    let mia_colored = format!("{}  {}", "Mia".red(), ">".cyan());
    let frames = ["⠇", "⠋", "⠙", "⠸", "⠼", "⠴", "⠦"];
    let kind = kind.to_string();

    let handle = tokio::spawn(async move {
        let mut i = 0;
        loop {
            print!("\r{} {} {}...", mia_colored, frames[i], kind);
            stdout().flush().unwrap();
            tokio::time::sleep(Duration::from_millis(80)).await;
            i = (i + 1) % frames.len();
        }
    });
    *SPINNER.lock().unwrap() = Some(handle);
}

/// Stop showing the thinking spinner
pub fn stop_spinner() {
    if let Some(handle) = SPINNER.lock().unwrap().take() {
        handle.abort();
        print!("\r{}\r", " ".repeat(20));
        stdout().flush().unwrap();
    }
}