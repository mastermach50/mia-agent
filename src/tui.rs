use std::time::Duration;

use ansi_to_tui::IntoText;
use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode, KeyModifiers,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use log::error;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use ratatui_textarea::TextArea;
use reedline::kitty_protocol_available;
use termimad::MadSkin;
use tokio::sync::{mpsc::UnboundedReceiver, oneshot};

use crate::{
    agent_loop::{self, AgentEvent, AgentHandle},
    agent_tools::ToolRegistry,
    api::{History, Message},
    config::AppConfig,
    sessions::Session,
    system_prompt::tui_system_prompt,
};

/// Entry point for starting the TUI
pub async fn run(new_session: bool) -> Result<()> {
    // Create a new app state
    let mut state = AppState::new();

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableBracketedPaste)
        .context("Failed to enable bracketed paste")?;

    if kitty_protocol_available() {
        execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            )
        )
        .context("Failed to push keyboard enhancement flags")?;
    }

    state.term_width = terminal.get_frame().area().width as usize;

    if new_session {
        state.session = Session::new("user", "tui", "tui");
        state
            .session
            .history
            .set_system_prompt(tui_system_prompt(Some(&state.help_message))?);
        state.messages.push(get_logo());
        state.send_harness_message("Started new session.")?;
    } else {
        if let Ok(session) = Session::load_last_session("user", "tui", "tui") {
            (
                state.messages,
                state.prompt_tokens,
                state.completion_tokens,
                state.total_tokens,
            ) = render_full_chat(&session.history, Some(state.term_width))?;
            state.session = session;
            state.send_harness_message("Loaded last session.")?;
        } else {
            state.session = Session::new("user", "tui", "tui");
            state
                .session
                .history
                .set_system_prompt(tui_system_prompt(Some(&state.help_message))?);
            state.messages.push(get_logo());
            state.send_harness_message("No previous session found, started new one.")?;
        }
    };

    while !state.exit {
        if state.redraw {
            terminal.clear()?;
            state.redraw = false;
        }
        terminal.draw(|f| state.draw(f))?;
        state.handle_input_events().await?;
        state.handle_agent_events()?;
    }

    state.session.save()?;
    execute!(std::io::stdout(), DisableBracketedPaste)
        .context("Failed to disable bracketed paste")?;
    if kitty_protocol_available() {
        execute!(std::io::stdout(), PopKeyboardEnhancementFlags)
            .context("Failed to pop keyboard enhancement flags")?;
    }
    ratatui::restore();
    Ok(())
}

const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

struct AppState {
    // Shared values
    agent_handle: AgentHandle,
    event_rx: UnboundedReceiver<AgentEvent>,
    term_width: usize,
    help_message: String,

    // Session
    session: Session,

    // Input
    input: TextArea<'static>,
    input_placeholder: String,

    // Permissions
    permission_request: Option<oneshot::Sender<bool>>,
    yolo: bool,

    // Status Bar
    spinner_idx: usize,
    status: String,
    model: String,

    // Chat
    messages: Vec<Text<'static>>,
    partial_message: Option<Message>,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,

    // Scroll
    scroll_offset: u16,
    auto_scroll: bool,

    // Other
    redraw: bool,
    exit: bool,
}

impl AppState {
    fn new() -> Self {
        let (event_rx, agent_handle) = AgentHandle::new();
        let help_message = indoc::indoc! {"
        Commands:
            /help         Show this help message
            /exit /bye    Exit the tui
            /new          Create a new session
            /model        Show model information
            /yolo         Toggle yolo mode (accept all permission requests)
        
        Keybinds:
            <Esc>  Quit
        "}
        .to_string();

        Self {
            agent_handle,
            event_rx,
            session: Session::default(),
            help_message,
            term_width: 0,

            input: TextArea::default(),
            input_placeholder: "Type Something...".to_string(),

            permission_request: None,
            yolo: false,

            spinner_idx: 0,
            status: "".to_string(),
            model: AppConfig::global().model.name.clone(),

            messages: Vec::new(),
            partial_message: None,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,

            scroll_offset: 0,
            auto_scroll: true,

            redraw: false,
            exit: false,
        }
    }

    fn reset_messages(&mut self) {
        self.messages.clear();
        self.messages.push(get_logo());

        self.partial_message = None;
        self.prompt_tokens = 0;
        self.completion_tokens = 0;
        self.total_tokens = 0;
    }

    fn reset_input(&mut self) {
        self.input.clear();
        self.input.set_style(Style::default());
        self.input_placeholder = "Type Something...".to_string();
    }

    /// Draw the frame from the current state
    /// It requires a mutable self because it also calls pre_render_messages()
    /// that requires a mutable self inorder to cache the rendered messages
    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Calculate the height required by the input based on its contents
        let input_width = area.width as usize;
        let input_height = self
            .input
            .lines()
            .iter()
            .map(|l| {
                let len = l.chars().count().max(1);
                ((len + input_width - 1) / input_width) as u16
            })
            .sum::<u16>()
            .max(1);

        // Divide up the total frame area
        let chunks = Layout::default()
            .constraints(vec![
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(input_height),
            ])
            .split(area);

        // Render input box
        self.input.set_placeholder_text(&self.input_placeholder);
        frame.render_widget(&self.input, chunks[2]);

        // Render status bar
        let border_type = if self.auto_scroll {
            BorderType::Plain
        } else {
            BorderType::LightDoubleDashed
        };
        let mut status_bar = Block::new().border_type(border_type).borders(Borders::TOP);

        if !self.status.is_empty() {
            let spinner =
                if AppConfig::global().tui.show_spinner && self.permission_request.is_none() {
                    self.spinner_idx = (self.spinner_idx + 1) % SPINNER_FRAMES.len();
                    format!("{} ", SPINNER_FRAMES[self.spinner_idx])
                } else {
                    String::new()
                };
            status_bar = status_bar.title(
                Line::from(vec![spinner.cyan(), self.status.clone().yellow()])
                    .alignment(Alignment::Left),
            );
        }
        if self.yolo {
            status_bar =
                status_bar.title(Line::from("[yolo]".red().bold()).alignment(Alignment::Left));
        }

        if self.completion_tokens > 0 && self.prompt_tokens > 0 {
            status_bar = status_bar.title(
                Line::from(vec![
                    "(".yellow(),
                    self.prompt_tokens.to_string().blue(),
                    "|".yellow(),
                    self.completion_tokens.to_string().blue(),
                    "|".yellow(),
                    self.total_tokens.to_string().blue(),
                    ")".yellow(),
                ])
                .alignment(Alignment::Right),
            );
        }

        status_bar = status_bar
            .title(Line::from(vec![self.model.clone().yellow()]).alignment(Alignment::Right));
        frame.render_widget(status_bar, chunks[1]);

        // Render chat
        let mut display_lines = Vec::new();
        for message in &self.messages {
            display_lines.extend(message.lines.clone());
        }
        if let Some(partial_message) = &self.partial_message {
            let rendered_message = render_message(partial_message, Some(self.term_width))
                .expect("Failed to render partial message");
            display_lines.extend(rendered_message.lines.clone());
        }

        let visible_height = chunks[0].height;
        let total_lines = wrapped_line_count(&display_lines, chunks[0].width as usize);
        let max_scroll = total_lines.saturating_sub(visible_height);
        if !self.auto_scroll && self.scroll_offset >= max_scroll {
            self.auto_scroll = true;
        }

        if self.auto_scroll {
            self.scroll_offset = max_scroll;
        } else {
            // Just in case of terminal resize
            self.scroll_offset = self.scroll_offset.min(max_scroll);
        }

        let messages_paragraph = Paragraph::new(display_lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset, 0));

        frame.render_widget(messages_paragraph, chunks[0]);
    }

    /// Take in the current contents of the input and make it into a new message
    fn submit(&mut self) -> Result<()> {
        // Ignore if empty
        if self.input.is_empty() {
            return Ok(());
        }

        let lines = self.input.lines();
        let text = lines.join("\n");

        // Process permission requests
        if let Some(response) = self.permission_request.take() {
            if text == "yes" || text == "y" {
                response.send(true).unwrap();
            } else {
                response.send(false).unwrap();
            }

            self.messages.pop();
            self.permission_request = None;

            self.status.clear();
            self.reset_input();

            return Ok(());
        };

        // Handle commands
        if text.trim().starts_with("/") && !text.trim().starts_with("//") {
            match text.trim() {
                "/exit" | "/bye" => {
                    self.exit = true;
                }
                "/new" => {
                    self.session = Session::new("user", "tui", "tui");
                    self.session
                        .history
                        .set_system_prompt(tui_system_prompt(None)?);
                    self.reset_messages();
                    self.send_harness_message("New session started, history cleared.")?;
                }
                "/model" => {
                    let mut text = String::new();
                    let model_config = AppConfig::global().model.clone();
                    text.push_str(&format!("Model     : {}\n", model_config.name));
                    text.push_str(&format!("Provider  : {}\n", model_config.provider));
                    text.push_str(&format!("Base URL  : {}\n", model_config.base_url));
                    text.push_str(&format!("Reasoning : {}\n", model_config.reasoning));
                    self.send_harness_message(&text)?;
                }
                "/yolo" => {
                    self.yolo = !self.yolo;
                    self.send_harness_message(&format!(
                        "Yolo mode {}",
                        if self.yolo { "enabled" } else { "disabled" }
                    ))?;
                }
                "/" | "/help" => {
                    let help_message = self.help_message.clone();
                    self.send_harness_message(&help_message)?;
                }
                _ => {
                    self.send_harness_message(
                        "Invalid command, use /help for a list of commands.",
                    )?;
                }
            }
            self.reset_input();
            return Ok(());
        }

        // Cancel any running agent turn
        self.agent_handle.cancel.cancel();

        // Display the user message, add it to history and save session
        let message = Message::new("user", text);
        let rendered_message = render_message(&message, Some(self.term_width))?;
        self.messages.push(rendered_message);
        self.session.history.add_message(message);
        self.session.save()?;

        self.input.clear();

        // Run the agent
        let stream = AppConfig::global().tui.streaming;
        let session_id = self.session.get_extended_session_id();
        let history = self.session.history.clone(); // Fix history clone
        self.agent_handle.reset_cancellation();
        let handle = self.agent_handle.clone();
        tokio::spawn(async move {
            agent_loop::run_agent(history, &session_id, stream, handle)
                .await
                .unwrap();
        });

        Ok(())
    }

    /// Send messages about the state of the agent
    fn send_harness_message(&mut self, message: &str) -> Result<()> {
        let message = Message::new("harness", message);
        let rendered_message = render_message(&message, Some(self.term_width))?;
        self.messages.push(rendered_message);

        Ok(())
    }

    async fn handle_input_events(&mut self) -> Result<()> {
        let timeout = Duration::from_millis(25);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key_event) => {
                    match key_event.code {
                        KeyCode::Esc => {
                            self.session.save()?;
                            self.exit = true;
                        }
                        KeyCode::Enter => {
                            if key_event.modifiers.is_empty() {
                                self.submit()?;
                            }
                        }
                        KeyCode::F(5) => {
                            self.redraw = true;
                        }
                        KeyCode::Char('c') => {
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                // Stop runnint agent
                                self.agent_handle.cancel.cancel();
                                self.partial_message = None;

                                // Clear input and status
                                self.status.clear();
                                self.input_placeholder = "Type Something...".to_string();
                                self.permission_request = None;

                                // Save the session
                                self.session.save()?;
                            }
                        }
                        KeyCode::Up => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(1);
                            self.auto_scroll = false;
                        }
                        KeyCode::Down => {
                            self.scroll_offset = self.scroll_offset.saturating_add(1);
                        }
                        KeyCode::PageUp => {
                            self.scroll_offset = self.scroll_offset.saturating_sub(10);
                            self.auto_scroll = false;
                        }
                        KeyCode::PageDown => {
                            self.scroll_offset = self.scroll_offset.saturating_add(10);
                        }
                        _ => {}
                    }

                    let is_scroll_key = matches!(
                        key_event.code,
                        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
                    );

                    let is_action_keybind =
                        key_event.code == KeyCode::Enter && key_event.modifiers.is_empty();

                    if !is_action_keybind && !is_scroll_key {
                        let commands = ["/", "/help", "/exit", "/bye", "/new", "/model", "/yolo"];

                        self.input.input(key_event);
                        let text = self.input.lines().join("\n");
                        if commands.contains(&text.trim()) {
                            self.input.set_style(Style::new().green());
                        } else {
                            self.input.set_style(Style::default());
                        }
                    }
                }
                Event::Paste(text) => {
                    self.input.insert_str(&text);
                }
                Event::Resize(cols, _rows) => {
                    self.term_width = cols as usize;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle message events
    fn handle_agent_events(&mut self) -> Result<()> {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                AgentEvent::AssistantMessage(msg) => {
                    // Clear the partial message
                    self.partial_message = None;

                    // Render and display the message
                    let rendered_message = render_message(&msg, Some(self.term_width))?;
                    self.messages.push(rendered_message);

                    // Calculate token usage
                    if let Some(usage) = &msg.usage {
                        self.prompt_tokens = usage.prompt_tokens;
                        self.completion_tokens += usage.completion_tokens;
                        self.total_tokens = usage.total_tokens;
                    }

                    // Append the message to the session history and save it
                    self.session.history.add_message(msg);
                    self.session.save()?;

                    // Clear any previous status
                    self.status.clear();
                    self.input_placeholder = "Type Something...".to_string();
                }
                AgentEvent::PartialAssistantMessage(msg) => {
                    if (msg.reasoning_chunk_index == 0 && msg.content_chunk_index == -1)
                        || (msg.reasoning_chunk_index == -1 && msg.content_chunk_index == 0)
                    {
                        self.partial_message = Some(Message {
                            role: msg.role.clone(),
                            reasoning: Some(String::new()),
                            content: Some(String::new()),
                            tool_calls: None,
                            tool_call_id: None,
                            usage: None,
                        });
                    }

                    if let Some(reasoning) = &msg.reasoning
                        && let Some(partial_message) = &mut self.partial_message
                        && let Some(partial_reasoning) = &mut partial_message.reasoning
                    {
                        partial_reasoning.push_str(reasoning);
                    }

                    if let Some(content) = &msg.content
                        && let Some(partial_message) = &mut self.partial_message
                        && let Some(partial_content) = &mut partial_message.content
                    {
                        partial_content.push_str(content);
                    }
                }
                AgentEvent::AssistantStatusUpdate(kind) => {
                    self.status = kind;

                    if !self.status.is_empty() {
                        self.input_placeholder = "Executing, <Ctrl-C> to cancel".to_string();
                    }
                }
                AgentEvent::ToolCallResponseMessage(msg) => {
                    self.session.history.add_message(msg);
                    self.session.save()?;
                }
                AgentEvent::HarnessMessage(msg) => {
                    let rendered_message =
                        render_message(&Message::new("harness", msg), Some(self.term_width))?;
                    self.messages.push(rendered_message);
                    self.status.clear();
                }
                AgentEvent::HistoryUpdate(history) => {
                    self.session.history = history;
                    self.session.save()?;
                }
                AgentEvent::PermissionRequest {
                    header,
                    content,
                    response,
                } => {
                    if self.yolo {
                        response.send(true).unwrap();
                        return Ok(());
                    };

                    let mut text = Text::default();

                    let wrapped = textwrap::wrap(&content, self.term_width - 2)
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<String>>()
                        .join("\n");

                    text.push_line(Line::from(vec!["╭─".into(), header.clone().red().bold()]));
                    for mut line in wrapped.into_text()?.lines {
                        line.spans.insert(0, "│ ".into());
                        text.push_line(line);
                    }
                    text.push_line(Line::from(vec!["╰─".into(), header.clone().red().bold()]));

                    self.messages.push(text);
                    self.permission_request = Some(response);
                    self.status = "Waiting For Permission".to_string();
                    self.input_placeholder = "y/n | <Ctrl-C> to cancel".to_string()
                }
            }
        }
        Ok(())
    }
}

/// Render all the messages
/// Returns the rendered messages vec and the completion and prompt tokens
fn render_full_chat(
    history: &History,
    term_width: Option<usize>,
) -> Result<(Vec<Text<'static>>, u64, u64, u64)> {
    let mut chat = Vec::new();
    let mut prompt_tokens = 0;
    let mut completion_tokens = 0;
    let mut total_tokens = 0;

    chat.push(get_logo());
    for message in &history.messages {
        let rendered_message = render_message(&message, term_width)?;
        chat.push(rendered_message);

        if let Some(usage) = &message.usage {
            prompt_tokens = usage.prompt_tokens;
            completion_tokens += usage.completion_tokens;
            total_tokens = usage.total_tokens;
        }
    }
    Ok((chat, prompt_tokens, completion_tokens, total_tokens))
}

/// Render out a single Message into Text
/// The rendered message is not wrapped to the width of the terminal
fn render_message(message: &Message, term_width: Option<usize>) -> Result<Text<'static>> {
    // Ignore actual system and tool response messages
    if message.role == "system" || message.role == "tool" {
        return Ok(Text::default());
    }

    let mut text = Text::default();

    let sender = match message.role.as_str() {
        "user" => AppConfig::global().tui.username.clone().green(),
        "assistant" => "Mia".cyan(),
        "harness" => "Harness".yellow(),
        _ => {
            error!("Unknown role: {}", message.role);
            anyhow::bail!("Unknown role: {}", message.role);
        }
    };

    let short_message = message.reasoning.is_none()
        && message.content.is_some()
        && !message.content.as_ref().unwrap().contains("\n")
        && message.content.as_ref().unwrap().chars().count() < 100;

    if short_message {
        text.push_line(Line::from(vec![
            sender,
            " ▶ ".into(),
            message.content.as_ref().unwrap().to_string().into(),
        ]));
        return Ok(text);
    }

    let thoughts = if message.reasoning.is_some() && !AppConfig::global().tui.show_reasoning {
        "Thoughts...".dark_gray().italic()
    } else {
        "".into()
    };
    text.push_line(Line::from(vec![sender, " ▼ ".into(), thoughts]));

    if let Some(reasoning) = &message.reasoning
        && !reasoning.is_empty()
        && AppConfig::global().tui.show_reasoning
    {
        for line in reasoning.split("\n") {
            text.push_line(line.to_string().dark_gray().italic());
        }
    }

    if let Some(content) = &message.content
        && !content.is_empty()
    {
        let skin = MadSkin::default_dark();
        let formatted = skin.text(content, term_width);
        let ansi_string = formatted.to_string();
        text.extend(ansi_string.into_text()?);
    }

    if let Some(tool_calls) = &message.tool_calls {
        for tool_call in tool_calls {
            text.push_line(Line::from(vec![
                "[ ".into(),
                ToolRegistry::tool_icon(&tool_call.function.name)
                    .to_string()
                    .into(),
                " ".into(),
                tool_call.function.name.clone().into(),
                ": ".into(),
                ToolRegistry::tool_short(&tool_call.function.name, &tool_call.function.arguments)
                    .into(),
                " ]".into(),
            ]));
        }
    }

    Ok(text)
}

/// Used to find the wrapped line count of given lines
fn wrapped_line_count(lines: &[Line], width: usize) -> u16 {
    if width == 0 {
        return lines.len() as u16;
    }

    lines
        .iter()
        .map(|line| {
            let content: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if content.is_empty() {
                1
            } else {
                textwrap::wrap(&content, width).len().max(1)
            }
        })
        .sum::<usize>() as u16
}

fn get_logo() -> Text<'static> {
    // The left and top padding are part of the design
    let logo = "

    ██╷     ██╷ ██╷   ██╷
    ████╷ ████│ ██│ ██┌─██╷
    ██┌─██┌─██│ ██│ ██████│
    ██│ └─┘ ██│ ██│ ██┌─██│
    └─┘     └─┘ └─┘ └─┘ └─┘
    ";

    let mut out = Text::default();
    for line in logo.split('\n') {
        let mut colored_line = Line::default();
        for ch in line.chars() {
            let colored_char: Span<'static> = if ['█', '▄', '▀'].contains(&ch) {
                ch.to_string().magenta()
            } else if ['─', '│', '┘', '└', '┌', '┐', '╷', '╶'].contains(&ch) {
                ch.to_string().light_green()
            } else {
                ch.to_string().into()
            };
            colored_line.push_span(colored_char);
        }
        out.push_line(colored_line);
    }
    out
}
