use arboard::Clipboard;
use indoc::indoc;

use crate::{config::AppConfig, sessions::Session, tui::AppState};
use anyhow::Result;

const COMMANDS: [&str; 8] = [
    // Help message
    "/help",
    // Session
    "/new",
    // Info
    "/model",
    // States
    "/yolo",
    "/reasoning",
    // Actions
    "/copylast",
    // Exit
    "/exit",
    "/bye",
];

pub fn is_valid_command(command: &str) -> bool {
    COMMANDS.contains(&command)
}

pub fn get_help_message() -> String {
    indoc! {"
    Keybinds
        <Esc>     Exit the TUI
        <Ctrl-C>  Stop agent turn
    
    Commands
        / /help     Show this help message
        /exit /bye  Exit the TUI
        /new        Start a new session
        /model      Show the model information
        /yolo       Toggle YOLO mode
        /copylast   Copy the last message to the clipboard
        /reasoning  Toggle reasoning display
    "}
    .to_string()
}

pub fn execute_command(command: &str, state: &mut AppState) -> Result<()> {
    match command {
        "/help" | "/" => {
            state.push_rendered_message(get_help_message().into());
        }
        "/new" => {
            state.session = Session::new("user", "tui", "tui");
            state
                .session
                .history
                .set_system_prompt(crate::tui::tui_system_prompt(None)?);
            state.rendered_messages.clear();
            state.wrapped_line_count = 0;
            state.rendered_partial_message = None;
            state.rendered_partial_message_wrapped_line_count = 0;
            state.partial_tool_output = None;
            state.partial_tool_output_wrapped_line_count = 0;
            state.push_rendered_message(crate::tui::logo::get_logo());
            state.send_harness_message("New session created")?;
        }
        "/model" => {
            let mut text = String::new();
            let model_config = AppConfig::global().model.clone();
            text.push_str(&format!("Model     : {}\n", model_config.name));
            text.push_str(&format!("Provider  : {}\n", model_config.provider));
            text.push_str(&format!("Base URL  : {}\n", model_config.base_url));
            text.push_str(&format!("Reasoning : {}\n", model_config.reasoning));
            state.send_harness_message(&text)?;
        }
        "/yolo" => {
            state.yolo = !state.yolo;
        }
        "/reasoning" => {
            state.show_reasoning = !state.show_reasoning;
            state.re_render_messages = true;
        }
        "/copylast" => {
            if let Some(last_message) = state.session.history.messages.last() {
                let mut clipboard = Clipboard::new()?;
                match clipboard.set_text(last_message.content.as_ref().unwrap()) {
                    Ok(_) => {
                        state.send_harness_message("Last message copied to clipboard")?;
                    }
                    Err(e) => {
                        state
                            .send_harness_message(&format!("Failed to copy to clipboard: {}", e))?;
                    }
                }
            }
        }
        "/exit" | "/bye" => {
            state.exit = true;
        }
        _ => {
            state.send_harness_message("Unknown command")?;
        }
    }

    Ok(())
}

pub fn complete_command(command: &str, state: &mut AppState) {
    let matches: Vec<String> = COMMANDS
        .iter()
        .map(|opt| opt.to_string())
        .filter(|opt| opt.starts_with(command))
        .collect();

    if matches.is_empty() {
        return;
    };

    let first = &matches[0];
    let mut longest_len = first.len();

    for candidate in &matches[1..] {
        // Narrow down matching length character by character
        let common_len = first
            .chars()
            .zip(candidate.chars())
            .take_while(|(a, b)| a == b)
            .count();

        // Convert character count back to byte index boundary
        let byte_idx = first
            .char_indices()
            .map(|(idx, _)| idx)
            .nth(common_len)
            .unwrap_or(first.len());

        longest_len = longest_len.min(byte_idx);
    }

    state.input.clear();
    state.input.insert_str(&first[..longest_len]);
}
