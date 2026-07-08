use anyhow::Result;
use reedline::Signal;
use std::io::{Write, stdout};
use termimad::{self, crossterm::style::Stylize};
use tokio::sync::mpsc::UnboundedReceiver;

mod custom_reedline;

use crate::agent_loop::{self, AgentEvent, AgentHandle};
use crate::agent_tools::ToolRegistry;
use crate::api::{Message, PartialMessage};
use crate::config::AppConfig;
use crate::sessions::Session;
use crate::system_prompt::tui_system_prompt;
use crate::utils::{generate_think_lines, start_spinner, stdio_ask_permission, stop_spinner};
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

    on_harness_message(&format!(
        "Use {} to exit the chat, {} to show all commands.",
        "/exit".yellow(),
        "/help".yellow()
    ));

    let stream = AppConfig::global().tui.streaming;

    // Unless a new session was requested load the previous history
    let mut session: Session;
    if new_session {
        session = Session::new("user", "tui", "tui");
        on_harness_message("Started new session.");
    } else {
        if let Ok(s) = Session::load_last_session("user", "tui", "tui") {
            session = s;
            on_harness_message("Loaded last session.");
        } else {
            on_harness_message("No previous session found.");
            session = Session::new("user", "tui", "tui");
            on_harness_message("Started new session.");
        }
    }
    let session_id = session.get_extended_session_id();

    // For full featured input powered by reedline
    // The _terminal_lifecycle is needed to support kitty protocol stuff
    let (mut rl, prompt, kitty_protocol) = get_reedline(commands)?;

    loop {
        // Update the system prompt every turn in case the user or system memory changed
        session
            .history
            .set_system_prompt(tui_system_prompt(Some(help_message))?);

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
                        session.save()?;
                        break;
                    }
                    "/new" => {
                        session = Session::new("user", "tui", "tui");
                        on_harness_message("New session started, history cleared.");
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
                        on_harness_message(&line);
                        continue;
                    }
                    _ => {
                        // Show invalid command message but respect C style comments
                        if line.starts_with('/') && !line.starts_with("//") {
                            on_harness_message(
                                "Invalid command, use /help for a list of commands.",
                            );
                            continue;
                        }
                    }
                }

                session.history.add_message(Message::new("user", &line));

                // Suspend the kitty protocol input handling before agent_loop::run_agent
                // for the Ctrl-C handlers in it to work properly
                kitty_protocol.suspend();

                // Assistant's response is printed by the printer passed into the agent loop
                let (mut event_rx, handle) = AgentHandle::new();
                let thread_session_id = session_id.clone();
                let thread_history = session.history.clone();
                tokio::spawn(async move {
                    agent_loop::run_agent(thread_history, &thread_session_id, stream, handle)
                        .await
                        .unwrap();
                });

                // Blocks until it recieves a history update event
                // i.e end of turn of agent
                handle_agent_events(&mut event_rx, &mut session);

                // Resume the kitty protocol input handling
                kitty_protocol.resume();

                // Save the session at the end of turn
                session.save()?;
            }
            Ok(Signal::CtrlC) => {
                println!("^C");
                continue;
            }
            Ok(Signal::CtrlD) => {
                println!("^D");
                session.save()?;
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
    let mut output = String::new();

    // Print the reasoning and content, only if it was not streamed
    if AppConfig::global().tui.streaming {
        output.push('\n');
    } else {
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

    if let Some(reasoning) = &message.reasoning
        && AppConfig::global().tui.show_reasoning
    {
        if message.reasoning_chunk_index == 0 {
            stop_spinner();
            println!("{mia_colored} 💭");
        }
        print!("{}", reasoning.clone().dark_grey().to_string());
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

pub fn on_harness_message(message: &str) {
    stop_spinner();
    let system_colored = format!("\r{} {}", "Harness".yellow(), ">".cyan());
    println!("{} {}", system_colored, message);
}

fn handle_agent_events(event_rx: &mut UnboundedReceiver<AgentEvent>, session: &mut Session) {
    'outer: loop {
        while let Ok(event) = event_rx.try_recv() {
            match event {
                AgentEvent::AssistantMessage(msg) => {
                    on_assistant_message(&msg);
                }
                AgentEvent::PartialAssistantMessage(msg) => {
                    on_partial_assistant_message(&msg);
                }
                AgentEvent::AssistantStatusUpdate(msg) => {
                    on_assistant_status_update(&msg);
                }
                AgentEvent::ToolCallResponseMessage(msg) => {
                    session.history.add_message(msg);
                }
                AgentEvent::HarnessMessage(msg) => {
                    on_harness_message(&msg);
                }
                AgentEvent::HistoryUpdate(history) => {
                    session.history = history;
                    break 'outer;
                }
                AgentEvent::PermissionRequest {
                    header,
                    content,
                    response,
                } => {
                    response
                        .send(stdio_ask_permission(header, &content))
                        .unwrap();
                }
            }
        }
    }
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
