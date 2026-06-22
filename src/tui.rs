use anyhow::Result;
use reedline::Signal;
use termimad::crossterm::terminal::{Clear, ClearType};
use termimad::crossterm::execute;
use termimad::crossterm::cursor::{RestorePosition, SavePosition};
use std::io::{Write, stdout};
use termimad::{self, crossterm::style::Stylize};

mod custom_reedline;

use crate::agent_loop;
use crate::agent_tools::ToolRegistry;
use crate::api::{Message, PartialMessage};
use crate::config::AppConfig;
use crate::sessions::{Session, create_new_session, get_last_session, save_session};
use crate::system_prompt::get_tui_system_prompt;
use crate::utils::{generate_think_lines, start_spinner, stop_spinner};
use custom_reedline::get_reedline;

pub async fn run(new_session: bool) -> Result<()> {
    // Help message
    let help_message = indoc::indoc! {"
    Commands:
        /help         Show this help message
        /exit /bye    Exit the tui
        /new          Create a new session
        /clear /cls   Clear screen
        /model        Show model information
    
    Keybinds:
        <Ctrl-C>      Cancel assistant/user message
        <Ctrl-D>      Exit
    "};

    // All commands
    let commands = vec![
        "/help".into(),
        "/exit".into(),
        "/bye".into(),
        "/new".into(),
        "/clear".into(),
        "/cls".into(),
        "/model".into(),
    ];

    on_system_message(&format!(
        "Use {} to exit the chat, {} to show all commands.",
        "/exit".yellow(),
        "/help".yellow()
    ));

    let stream = AppConfig::global().tui.streaming;

    // Unless a new session was requested load the previous history
    let mut session: Session;
    if new_session {
        session = create_new_session("tui", "user")?;
        on_system_message("Started new session.");
    } else {
        if let Some(s) = get_last_session("tui", "user")? {
            session = s;
            on_system_message("Loaded last session.");
        } else {
            on_system_message("No previous session found.");
            session = create_new_session("tui", "user")?;
            on_system_message("Started new session.");
        }
    }

    // For full featured input powered by reedline
    // The _terminal_lifecycle is needed to support kitty protocol stuff
    let (mut rl, prompt, kitty_protocol) = get_reedline(commands)?;

    loop {
        // Update the system prompt every turn in case the user or system memory changed
        session
            .history
            .set_system_prompt(get_tui_system_prompt(Some(help_message))?);

        // Handle inputs using reedline
        println!("{}", "─".repeat(textwrap::termwidth()));
        match rl.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                let line = line.trim().to_string();
                if line.is_empty() {
                    continue;
                }

                // Match for commands
                match line.as_str() {
                    "/exit" | "/bye" => {
                        save_session(&session)?;
                        break;
                    }
                    "/new" => {
                        session = create_new_session("tui", "user")?;
                        on_system_message("New session started, history cleared.");
                        continue;
                    }
                    "/clear" | "/cls" => {
                        rl.clear_screen()?;
                        continue;
                    }
                    "/" | "/help" => {
                        println!("{}", help_message);
                        continue;
                    }
                    "/model" => {
                        let mut line = String::new();
                        let model_config = AppConfig::global().model.clone();
                        line.push_str(&indoc::formatdoc! {"
                        
                        Model     : {}
                        Base URL  : {}
                        Reasoning : {}",
                        model_config.name, model_config.base_url, model_config.reasoning});
                        on_system_message(&line);
                        continue;
                    }
                    _ => {
                        // Show invalid command message but respect C style comments
                        if line.starts_with('/') && !line.starts_with("//") {
                            on_system_message("Invalid command, use /help for a list of commands.");
                            continue;
                        }
                    }
                }

                session.history.add_message(Message::new("user", &line));

                // Suspend the kitty protocol input handling before agent_loop::run_agent
                // for the Ctrl-C handlers in it to work properly
                kitty_protocol.suspend();

                // Assistant's response is printed by the printer passed into the agent loop
                session.history = agent_loop::run_agent(
                    session.history,
                    stream,
                    on_assistant_message,
                    on_partial_assistant_message,
                    on_assistant_status_update,
                    on_system_message,
                )
                .await?;

                // Resume the kitty protocol input handling
                kitty_protocol.resume();

                // Save the session at the end of turn
                save_session(&session)?;
            }
            Ok(Signal::CtrlC) => {
                println!("^C");
                continue;
            }
            Ok(Signal::CtrlD) => {
                println!("^D");
                save_session(&session)?;
                println!("Exiting...");
                break;
            }
            Ok(_) => {}
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

pub fn on_assistant_message(message: &Message) {
    stop_spinner();
    let mia_colored = format!("\r{}  {}", "Mia".red(), ">".cyan());

    // If streaming, restore cursor position and clear lines before printing
    if AppConfig::global().tui.streaming {
        execute!(stdout(), RestorePosition, Clear(ClearType::FromCursorDown)).unwrap();
    }

    let mut output = String::new();
    if let Some(reasoning) = message.reasoning.clone()
        && AppConfig::global().tui.show_reasoning
    {
        output += &format!("{mia_colored} 💭\n");
        output += &format!("{}\n", generate_think_lines(reasoning.trim()));
    }
    if let Some(content) = message.content.clone()
        && content.trim() != ""
    {
        output += &format!("{mia_colored} {}\n", content.trim());
    }

    if let Some(tool_calls) = message.tool_calls.clone() {
        for tool_call in tool_calls {
            output += &format!(
                "{mia_colored} {} {}: {}\n",
                ToolRegistry::tool_icon(&tool_call.function.name),
                tool_call.function.name,
                ToolRegistry::tool_short(&tool_call.function.name, &tool_call.function.arguments),
            );
        }
    }

    termimad::print_text(&output);
}

pub fn on_partial_assistant_message(message: &PartialMessage) {
    let mia_colored = format!("\r{}  {}", "Mia".red(), ">".cyan());

    // Before printing the very first chunk save cursor state
    if message.reasoning_chunk_index == 0 // first reasoning chunk
    || (message.reasoning_chunk_index == -1 && message.content_chunk_index == 0) // First content chunk without recieving a reasoning chunk
    {
        execute!(stdout(), SavePosition).unwrap();
    }

    if let Some(reasoning) = &message.reasoning
    && AppConfig::global().tui.show_reasoning {
        if message.reasoning_chunk_index == 0 {
            stop_spinner();
            println!("{mia_colored} 💭");
        }
        print!("{reasoning}");
    }

    if let Some(content) = &message.content {
        if message.content_chunk_index == 0 {
            stop_spinner();
            print!("{mia_colored} ");
        }
        print!("{content}");
    }

    stdout().flush().unwrap();
}

pub fn on_assistant_status_update(kind: &str) {
    if AppConfig::global().tui.show_spinner {
        start_spinner(kind);
    } else {
        let mia_colored = format!("{}  {}", "Mia".red(), ">".cyan());
        print!("{} {}...", mia_colored, kind);
        stdout().flush().unwrap();
    }
}

pub fn on_system_message(message: &str) {
    stop_spinner();
    let system_colored = format!("\r{} {}", "System".yellow(), ">".cyan());
    println!("{} {}", system_colored, message);
}

#[cfg(test)]
mod tests {
    #[test]
    fn print_inline() {
        let output = "|---|---|
| Tool | What it would do |
|------|-----------------|
| **file_info** | Get file metadata (size, permissions, timestamps, mime type) |
| **directory_tree** | Recursive tree view of directories with sizes/permissions |
| **process_list** | List running processes, PIDs, CPU/memory usage |
| **system_info** | OS, kernel version, uptime, memory/disk stats |
| **head/tail** | Read first/last N lines of large files efficiently |
| **wc** | Count lines/words/bytes in files |
| **git_log** | Commit history, changes, blame info |
| **diff** | Compare two files or show uncommitted changes |
|---|---|";
        let skin = termimad::MadSkin::default_dark();
        skin.print_text(output);
    }
}
