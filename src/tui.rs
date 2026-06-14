use anyhow::Result;
use nu_ansi_term::{Color, Style};
use reedline::{
    ColumnarMenu, DefaultCompleter, EditCommand, Emacs, ExampleHighlighter, FileBackedHistory,
    KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptHistorySearchStatus, Reedline, ReedlineEvent,
    ReedlineMenu, Signal, default_emacs_keybindings,
};
use std::io::{Write, stdout};
use termimad::{self, crossterm::style::Stylize};

use crate::agent_loop;
use crate::agent_tools::ToolRegistry;
use crate::api::{History, Message};
use crate::config::AppConfig;
use crate::sessions::{load_session, save_session};
use crate::system_prompt::get_system_prompt;
use crate::utils::{generate_think_lines, start_spinner, stop_spinner};

pub async fn run(new_session: bool) -> Result<()> {
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

    on_system_message(&format!(
        "Use {} to exit the chat, {} to show all commands.",
        "/exit".yellow(),
        "/help".yellow()
    ));

    // Unless a new session was requested load the previous history
    let mut history = History::new();
    if !new_session {
        // Try to load the history from file
        if let Ok(loaded_history) = load_session("tui-agent-history.json") {
            history = loaded_history;
            on_system_message("Loaded previous session history.");
        }
    } else {
        on_system_message("Started new session.");
    }

    // For full featured input powered by reedline
    let (mut rl, prompt) = get_reedline()?;

    loop {
        // Update the system prompt every turn in case the user or system memory changed
        history.set_system_prompt(get_tui_system_prompt()?);

        // A
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
                        save_session("tui-agent-history.json", &history)?;
                        break;
                    }
                    "/new" => {
                        history = History::new();
                        on_system_message("New session started, history cleared.");
                        continue;
                    }
                    "/clear" | "/cls" => {
                        rl.clear_scrollback()?;
                        continue;
                    }
                    "/" | "/help" => {
                        println!("{}", help_message);
                        continue;
                    }
                    "/model" => {
                        let mut line = String::new();
                        line.push_str(&format!(
                            "\nBase URL  {}",
                            AppConfig::global().model.base_url
                        ));
                        line.push_str(&format!("\nName      {}", AppConfig::global().model.name));
                        line.push_str(&format!(
                            "\nReasoning {}",
                            AppConfig::global().model.reasoning
                        ));
                        on_system_message(&line);
                        continue;
                    }
                    _ => {
                        if line.starts_with('/') {
                            on_system_message("Invalid command, use /help for a list of commands.");
                            continue;
                        }
                    }
                }

                history.add_message(Message::new("user", &line));

                // Assistant's response is printed by the printer passed into the agent loop
                history = agent_loop::run_agent(
                    history,
                    on_assistant_message,
                    on_assistant_status_update,
                    on_system_message,
                )
                .await?;

                // Save the session at the end of turn
                save_session("tui-agent-history.json", &history)?;
            }
            Ok(Signal::CtrlC) => {
                println!("^C");
                continue;
            }
            Ok(Signal::CtrlD) => {
                println!("^D");
                save_session("tui-agent-history.json", &history)?;
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
    if let Some(reasoning) = message.reasoning.clone()
        && AppConfig::global().tui.show_reasoning
    {
        output += &format!("{mia_colored} 💭             \n");
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

pub fn get_tui_system_prompt() -> Result<String> {
    let mut system_prompt = get_system_prompt()?;
    system_prompt.push_str(&format!(
        "\nYou are talking to {} via a TUI.",
        AppConfig::global().tui.username
    ));
    Ok(system_prompt)
}

fn get_reedline() -> Result<(Reedline, impl Prompt)> {
    let history = Box::new(
        FileBackedHistory::with_file(1000, AppConfig::global().tui.history_file.clone().into())
            .unwrap_or_else(|_| FileBackedHistory::new(1000).unwrap()),
    );

    let commands = vec![
        "/exit".into(),
        "/bye".into(),
        "/new".into(),
        "/clear".into(),
        "/cls".into(),
    ];

    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_text_style(Style::new().fg(Color::Green)),
    );
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    let mut completer = Box::new(DefaultCompleter::with_inclusions(&['/', '-', '_']));
    completer.insert(commands.clone());

    let mut hilighter = Box::new(ExampleHighlighter::new(commands.clone()));
    hilighter.change_colors(
        nu_ansi_term::Color::Green,
        nu_ansi_term::Color::Default,
        nu_ansi_term::Color::Default,
    );

    let prompt = CustomPrompt;

    let rl = Reedline::create()
        .with_history(history)
        .with_highlighter(hilighter)
        .with_history_exclusion_prefix(Some(" ".into()))
        .with_completer(completer)
        .with_partial_completions(true)
        .with_quick_completions(true)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);
    Ok((rl, prompt))
}

struct CustomPrompt;
impl Prompt for CustomPrompt {
    fn render_prompt_left(&self) -> std::borrow::Cow<'_, str> {
        "User ".blue().to_string().into()
    }
    fn render_prompt_right(&self) -> std::borrow::Cow<'_, str> {
        "".into()
    }
    fn render_prompt_indicator(
        &self,
        _prompt_mode: reedline::PromptEditMode,
    ) -> std::borrow::Cow<'_, str> {
        "> ".into()
    }
    fn render_prompt_multiline_indicator(&self) -> std::borrow::Cow<'_, str> {
        "::: ".into()
    }
    fn render_prompt_history_search_indicator(
        &self,
        history_search: reedline::PromptHistorySearch,
    ) -> std::borrow::Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };

        format!(" ({} reverse-search: {}) ", prefix, history_search.term).into()
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
