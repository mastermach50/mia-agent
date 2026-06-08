use std::io::stdout;
use std::io::Write;
use anyhow::Result;
use colored::Colorize;
use nu_ansi_term::{Color, Style};
use reedline::EditCommand;
use reedline::{ColumnarMenu, DefaultCompleter, Emacs, ExampleHighlighter, FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptHistorySearchStatus, Reedline, ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings};
use termimad;

use crate::agent_tools::ToolRegistry;
use crate::utils::{generate_think_lines, load_session, save_session};
use crate::system_prompt::get_system_prompt;
use crate::config::AppConfig;
use crate::agent_loop;
use crate::api::{History, Message};

pub async fn run() -> Result<()> {
    let system_colored = format!("{} {}", "System".yellow(), ">".cyan());
    let mia_colored = format!("{}  {}", "Mia".red(), ">".cyan());

    let help_message = indoc::indoc! {"
    /help         Show this help message
    /exit /bye    Exit the tui
    /new          Create a new session
    /clear /cls   Clear screen
    "};

    // Try to load the history from file
    // If it doesn't exist, create a new one
    let mut history = History::new();
    if let Ok(loaded_history) = load_session("tui-agent-history.json") {
        history = loaded_history;
    }

    // For full featured input powered by reedline
    let (mut rl, prompt) = get_reedline()?;
    
    println!(
        "{} Use {} to exit the chat, {} to start a new session.",
        system_colored, "/exit".yellow(), "/new".yellow()
    );
    loop {
        // Update the system prompt every turn in case the user or system memory changed
        history.set_system_prompt(get_tui_system_prompt()?);

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
                        println!("{system_colored} New session started, history cleared.");
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
                    _ => {
                        if line.starts_with('/') {
                            println!("{system_colored} Invalid command, use /help for a list of commands.");
                            continue;
                        }
                    }
                }

                history.add_message(Message::new("user", &line));

                print!("{mia_colored} Thinking...\r");
                stdout().flush()?;

                history = agent_loop::run_agent(history, message_printer).await?;

                // Save the session at the end of turn
                save_session("tui-agent-history.json", &history)?;
            }
            Ok(Signal::CtrlC) => {
                println!("<CTRL-C>");
                continue;
            },
            Ok(Signal::CtrlD) => {
                println!("<CTRL-D>");
                save_session("tui-agent-history.json", &history)?;
                println!("Exiting...");
                break;
            },
            Ok(_) => {},
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }
    Ok(())
}

pub fn message_printer(message: &Message) {
    let mia_colored = format!("{}  {}", "Mia".red(), ">".cyan());

    let mut output = String::new();
    if let Some(reasoning) = message.reasoning.clone() {
        output += &format!("{mia_colored} 💭             \n");
        output += &format!("{}\n", generate_think_lines(reasoning.trim()));
    }
    if let Some(content) = message.content.clone() {
        if content.trim() != "" {
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

    termimad::print_inline(&output);
}

fn get_tui_system_prompt() -> Result<String> {
    let mut system_prompt = get_system_prompt()?;
    system_prompt.push_str(&format!(
        "\nYou are talking to {} via a TUI.",
        AppConfig::global().tui.username
    ));
    Ok(system_prompt)
}

fn get_reedline() -> Result<(Reedline, impl Prompt)> {
    let history = Box::new(
        FileBackedHistory::with_file(
            1000,
            AppConfig::global().tui.history_file.clone().into()
        )
        .unwrap_or_else(|_| FileBackedHistory::new(1000).unwrap())
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
            .with_text_style(Style::new().fg(Color::Green))
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
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline])
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    let mut completer = Box::new(
        DefaultCompleter::with_inclusions(&['/', '-', '_'])
    );
    completer.insert(commands.clone());

    let mut hilighter = Box::new(ExampleHighlighter::new(commands.clone()));
    hilighter.change_colors(
        nu_ansi_term::Color::Green,
        nu_ansi_term::Color::Default,
        nu_ansi_term::Color::Default
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
    fn render_prompt_indicator(&self, _prompt_mode: reedline::PromptEditMode) -> std::borrow::Cow<'_, str> {
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

        format!(
            " ({} reverse-search: {}) ",
            prefix, history_search.term
        ).into()
    }
}