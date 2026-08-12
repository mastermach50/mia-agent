use std::time::Duration;

use ansi_to_tui::IntoText;
use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyModifiers, KeyboardEnhancementFlags, MouseButton, MouseEventKind,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::supports_keyboard_enhancement,
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};
use ratatui_textarea::{TextArea, WrapMode};
use reedline::KeyCode;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    agent_loop::{AgentEvent, AgentHandle, PermissionRequest},
    api::Message,
    config::AppConfig,
    sessions::Session,
    system_prompt::tui_system_prompt,
    tui::{
        commands::{complete_command, execute_command, is_valid_command},
        logo::get_logo,
        message_renderer::{render_all_messages, render_message},
        statusbar::create_statusbar,
    },
};

mod commands;
mod logo;
mod message_renderer;
mod statusbar;

pub async fn run(new_session: bool) -> Result<()> {
    let mut state = AppState::new();

    if new_session {
        state.session = Session::new("user", "tui", "tui");
        state
            .session
            .history
            .set_system_prompt(tui_system_prompt(None)?);
        state.push_rendered_message(get_logo());
        state.send_harness_message("New session creted")?;
    } else {
        match Session::load_last_session("user", "tui", "tui") {
            Ok(session) => {
                state.session = session;
                render_all_messages(&mut state)?;
                state.send_harness_message("Loaded last session")?;
            }
            Err(_) => {
                state.session = Session::new("user", "tui", "tui");
                state
                    .session
                    .history
                    .set_system_prompt(tui_system_prompt(None)?);
                state.push_rendered_message(get_logo());
                state.send_harness_message("No previous session found")?;
                state.send_harness_message("New session creted")?;
            }
        };
    };

    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableBracketedPaste, EnableMouseCapture)
        .context("Failed to enable bracketed paste")?;

    if supports_keyboard_enhancement()? {
        execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            ),
        )
        .context("Failed to push keyboard enhancement flags")?;
    }

    while !state.exit {
        state.handle_agent_events()?;
        state.handle_input_events()?;

        if state.redraw_once {
            terminal.clear()?;
            state.redraw_once = false;
        }

        if state.re_render_messages {
            render_all_messages(&mut state)?;
            state.re_render_messages = false;
        }

        if state.term_size_changed {
            state.term_size_changed = false;
            state.chat_area_width = state.term_width - 1;
            state.chat_area_height = state.term_height - 1 - state.get_input_height();
            render_all_messages(&mut state)?;
        }

        terminal.draw(|f| state.draw_frame(f).expect("Failed to draw frame"))?;
    }

    state.session.save()?;
    execute!(
        std::io::stdout(),
        DisableBracketedPaste,
        DisableMouseCapture
    )
    .context("Failed to disable bracketed paste")?;
    if supports_keyboard_enhancement()? {
        execute!(std::io::stdout(), PopKeyboardEnhancementFlags)
            .context("Failed to pop keyboard enhancement flags")?;
    }
    ratatui::restore();

    Ok(())
}

struct AppState {
    // Data
    session: Session, // Contains the actual messages
    agent_handle: AgentHandle,
    agent_event_rx: UnboundedReceiver<AgentEvent>,

    // Chat Area
    chat_area_height: usize,
    chat_area_width: usize,

    rendered_messages: Vec<Text<'static>>, // For storing rendered messages
    wrapped_line_count: usize,

    partial_message: Option<Message>,
    rendered_partial_message: Option<Text<'static>>,
    rendered_partial_message_wrapped_line_count: usize,

    partial_tool_output: Option<String>,
    partial_tool_output_wrapped_line_count: usize,

    permission_request: Option<PermissionRequest>,
    rendered_permission_request: Option<Text<'static>>,
    rendered_permission_request_wrapped_line_count: usize,

    show_reasoning: bool,

    // Scrollbar
    scroll_offset: usize,
    auto_scroll: bool,
    scrollbar_state: ScrollbarState,

    // Statusbar
    spinner_idx: usize,
    status: String,
    model: String,
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,

    // Input Area
    input: TextArea<'static>,

    // Other
    yolo: bool,
    selection_mode: bool,
    term_width: usize,
    term_height: usize,
    term_size_changed: bool,
    re_render_messages: bool,
    redraw_once: bool,
    exit: bool,
}

impl AppState {
    fn new() -> Self {
        let mut input = TextArea::default();
        input.set_wrap_mode(WrapMode::WordOrGlyph);
        input.set_placeholder_text("Type something...");

        let (agent_event_rx, agent_handle) = AgentHandle::new();

        let model = AppConfig::global().model.name.clone();

        let (term_width, term_height) =
            crossterm::terminal::size().expect("Failed to get terminal size");

        let chat_area_width = term_width.saturating_sub(1) as usize;
        let chat_area_height = term_height.saturating_sub(2) as usize;

        let scrollbar_state = ScrollbarState::new(0);

        Self {
            session: Session::default(),
            agent_handle,
            agent_event_rx,
            chat_area_height,
            chat_area_width,
            rendered_messages: Vec::new(),
            wrapped_line_count: 0,
            partial_message: None,
            rendered_partial_message: None,
            rendered_partial_message_wrapped_line_count: 0,
            partial_tool_output: None,
            partial_tool_output_wrapped_line_count: 0,
            permission_request: None,
            rendered_permission_request: None,
            rendered_permission_request_wrapped_line_count: 0,
            show_reasoning: AppConfig::global().tui.show_reasoning,
            scroll_offset: 0,
            auto_scroll: true,
            scrollbar_state,
            spinner_idx: 0,
            status: String::new(),
            model,
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            input,
            yolo: false,
            selection_mode: false,
            term_width: (term_width as usize),
            term_height: (term_height as usize),
            term_size_changed: false,
            re_render_messages: false,
            redraw_once: false,
            exit: false,
        }
    }

    /// Submit the input. The input is either a message to the agent or a command to the harness.
    fn submit(&mut self) -> Result<()> {
        // Ignore if input is empty
        if self.input.is_empty() {
            return Ok(());
        }

        let full_input = self.input.lines().join("\n");

        // Don't treat C style comments as commands
        if full_input.starts_with("/") && !full_input.starts_with("//") {
            execute_command(&full_input, self)?;
            self.input.clear();

            return Ok(());
        }

        // Consider submission to permission request if it exists
        if let Some(permission_request) = self.permission_request.take()
            && !permission_request.response.is_closed()
        {
            self.input.clear();

            let permission_granted = if full_input == "yes" || full_input.chars().all(|c| c == 'y')
            {
                true
            } else {
                false
            };

            if permission_request
                .response
                .send(permission_granted)
                .is_err()
            {
                self.send_harness_message("Failed to send permission response")?;
            }

            self.permission_request = None;
            self.rendered_permission_request = None;
            self.rendered_permission_request_wrapped_line_count = 0;
            self.input
                .set_placeholder_text("Executing, <Ctrl-C> to cancel");
            self.status.clear();

            return Ok(());
        }

        // Note: After cancellation the cancellation token is reset instead of creating a new agent handle
        // to avoid a race condition where the event_rx is dropped and redefined before the agent thread
        // can send a harness message to it declaring the cancellation.

        // Cancel any existing agent turn
        self.agent_handle.cancel.cancel();

        // Get the input content
        let lines = self.input.lines();
        let content = lines.join("\n");
        self.input.clear();

        // Add the message to history and also render it
        let message = Message::new("user", content);
        if let Some(rendered_message) =
            render_message(&message, self.term_width, self.show_reasoning)?
        {
            self.push_rendered_message(rendered_message);
        }
        self.session.history.add_message(message);
        self.session.save()?;

        // Snap to the end of the chat
        self.auto_scroll = true;
        self.recalculate_scroll_offset();

        // Call the agent
        self.agent_handle.reset_cancellation();
        let history = self.session.history.clone();
        let session_id = self.session.get_extended_session_id();
        let stream = AppConfig::global().tui.streaming;
        let handle = self.agent_handle.clone();
        tokio::spawn(async move {
            crate::agent_loop::run_agent(history, &session_id, stream, handle)
                .await
                .unwrap();
        });

        Ok(())
    }

    /// Draw the frame based on current app state.
    ///
    /// The state taken is mutable (for now) to achieve the following
    /// - update spinner frame index
    /// - update chat area dimensions
    /// - recalculate scroll offset
    fn draw_frame(&mut self, frame: &mut Frame) -> Result<()> {
        let area = frame.area();

        let chunks = Layout::default()
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(self.get_input_height() as u16),
            ])
            .split(area);
        let chat_chunk = chunks[0];
        let statusbar_area = chunks[1];
        let input_area = chunks[2];

        // Chat Area
        let chat_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(chat_chunk);
        let chat_area = chat_chunks[0];
        let scrollbar_area = chat_chunks[1];

        self.chat_area_height = chat_area.height as usize;
        self.chat_area_width = chat_area.width as usize;
        self.recalculate_scroll_offset();

        // Scrollbar
        if !self.selection_mode {
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight).track_symbol(Some("│")),
                scrollbar_area,
                &mut self.scrollbar_state,
            );
        }

        let mut display_lines = Vec::new();
        for rendered_message in self.rendered_messages.iter() {
            display_lines.extend(rendered_message.lines.clone());
        }

        if let Some(rendered_partial_message) = &self.rendered_partial_message {
            display_lines.extend(rendered_partial_message.lines.clone())
        }
        if let Some(partial_tool_output) = &self.partial_tool_output {
            display_lines.extend(Text::from(partial_tool_output.clone()));
        }
        if let Some(rendered_permission_request) = &self.rendered_permission_request {
            display_lines.extend(rendered_permission_request.lines.clone());
        }

        let chat = Paragraph::new(display_lines)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll_offset as u16, 0));
        frame.render_widget(chat, chat_area);

        // Statusbar
        let statusbar = create_statusbar(self);
        frame.render_widget(statusbar, statusbar_area);

        // Input Area
        frame.render_widget(&self.input, input_area);

        Ok(())
    }

    /// Get the input size based on its contents. Has no side effects.
    fn get_input_height(&self) -> usize {
        let input_width = self.term_width;

        self.input
            .lines()
            .iter()
            .map(|l| {
                let len = l.chars().count().max(1);
                len.div_ceil(input_width)
            })
            .sum::<usize>()
            .max(1)
    }

    /// Recalculate the scroll offset and set the scrollbar state.
    ///
    /// The total content height is the sum of
    /// - rendered messages wrapped line count
    /// - partial message wrapped line count
    /// - partial tool call wrapped line count
    /// - permission request wrapped line count
    fn recalculate_scroll_offset(&mut self) {
        // Calculate content and viewport height
        let content_height = self.wrapped_line_count
            + self.rendered_partial_message_wrapped_line_count
            + self.partial_tool_output_wrapped_line_count
            + self.rendered_permission_request_wrapped_line_count;
        let viewport_height = self.chat_area_height;

        // Set the max scroll before updating position
        let max_scroll = content_height.saturating_sub(viewport_height);
        self.scrollbar_state = self.scrollbar_state.content_length(max_scroll);

        if self.auto_scroll {
            self.scroll_offset = max_scroll;
            self.scrollbar_state.last();
        }

        if self.scroll_offset >= max_scroll {
            self.scroll_offset = max_scroll;
            self.auto_scroll = true;
            self.scrollbar_state.last();
        }

        self.scrollbar_state = self.scrollbar_state.position(self.scroll_offset);
    }

    /// Send a new harness message to the chat
    ///
    /// Handles the whole process of turning the str into a message, rendering it,
    /// placing it in the rendered_messages cache and updating the wrapped_line_count
    fn send_harness_message(&mut self, message: &str) -> Result<()> {
        let message = Message::new("harness", message);
        if let Some(rendered_message) =
            render_message(&message, self.chat_area_width, self.show_reasoning)?
        {
            self.push_rendered_message(rendered_message);
        }
        Ok(())
    }

    /// Insert a rendered message into the rendered_messages cache, while also doing necessary operations.
    ///
    /// Necessary operations
    /// - update wrapped line count
    /// - update scrollbar state context length
    fn push_rendered_message(&mut self, rendered_message: Text<'static>) {
        let line_count = wrapped_text_height(&rendered_message, self.chat_area_width);
        self.wrapped_line_count += line_count;
        self.rendered_messages.push(rendered_message);
        self.recalculate_scroll_offset();
    }

    /// Handle any agents events from `self.agent_event_rx`
    fn handle_agent_events(&mut self) -> Result<()> {
        while let Ok(event) = self.agent_event_rx.try_recv() {
            match event {
                AgentEvent::AssistantMessage(msg) => {
                    // Clear the partial message
                    self.partial_message = None;
                    self.rendered_partial_message = None;
                    self.rendered_partial_message_wrapped_line_count = 0;

                    // Clear permission prompt
                    self.permission_request = None;
                    self.rendered_permission_request = None;
                    self.rendered_permission_request_wrapped_line_count = 0;

                    self.recalculate_scroll_offset();

                    // Render and display the message
                    if let Some(rendered_message) =
                        render_message(&msg, self.chat_area_width, self.show_reasoning)?
                    {
                        self.push_rendered_message(rendered_message);
                    };

                    // Calculate token usage
                    if let Some(usage) = &msg.usage {
                        self.prompt_tokens = usage.prompt_tokens;
                        self.completion_tokens += usage.completion_tokens;
                        self.total_tokens = usage.total_tokens;
                    }

                    // Clear any previous status if this message does not have any tool calls
                    if msg.tool_calls.is_none() {
                        self.status.clear();
                        self.input.set_placeholder_text("Type Something...");
                    }

                    // Append the message to the session history and save it
                    self.session.history.add_message(msg);
                    self.session.save()?;
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

                    if let Some(rendered_partial_message) = render_message(
                        &self.partial_message.as_ref().unwrap(),
                        self.chat_area_width,
                        self.show_reasoning,
                    )? {
                        self.rendered_partial_message_wrapped_line_count =
                            wrapped_text_height(&rendered_partial_message, self.chat_area_width);
                        self.rendered_partial_message = Some(rendered_partial_message);
                        self.recalculate_scroll_offset();
                    }
                }
                AgentEvent::AssistantStatusUpdate(msg) => {
                    self.status = msg;

                    if !self.status.is_empty() {
                        self.input
                            .set_placeholder_text("Executing, <Ctrl-C> to cancel");
                    } else {
                        self.input.set_placeholder_text("Type Something...");
                    }
                }
                AgentEvent::ToolResponseMessage(msg) => {
                    // This message is not actually displayed in the chat
                    self.session.history.add_message(msg);
                    self.session.save()?;
                }
                AgentEvent::HarnessMessage(msg) => {
                    self.send_harness_message(&msg)?;
                }
                AgentEvent::HistoryUpdate(history) => {
                    // Just to sync the history
                    self.session.history = history;
                    self.session.save()?;
                }
                AgentEvent::PermissionRequest(request) => {
                    if self.yolo {
                        request.response.send(true).unwrap();
                        continue;
                    }

                    let mut text = Text::default();
                    let wrapped = textwrap::wrap(&request.content, self.chat_area_width - 2)
                        .iter()
                        .map(|line| line.to_string())
                        .collect::<Vec<String>>()
                        .join("\n");

                    text.push_line(Line::from(vec![
                        "╭─".into(),
                        request.header.clone().red().bold(),
                    ]));
                    for mut line in wrapped.into_text()?.lines {
                        line.spans.insert(0, "│ ".into());
                        text.push_line(line);
                    }
                    text.push_line(Line::from(vec![
                        "╰─".into(),
                        request.header.clone().red().bold(),
                    ]));

                    self.rendered_permission_request_wrapped_line_count =
                        wrapped_text_height(&text, self.chat_area_width);
                    self.rendered_permission_request = Some(text);
                    self.recalculate_scroll_offset();

                    self.permission_request = Some(request);
                    self.status = "Permission Required".to_string();
                    self.input.set_placeholder_text("y/n, <Ctrl-C> to cancel");
                }
                AgentEvent::PartialToolOutput { stdout, stderr } => {
                    if AppConfig::global().tui.show_tool_output {
                        if self.partial_tool_output.is_none() {
                            self.partial_tool_output = Some(String::new());
                        }

                        if let Some(stdout) = stdout {
                            self.partial_tool_output.as_mut().unwrap().push_str(&stdout);
                        }

                        if let Some(stderr) = stderr {
                            self.partial_tool_output.as_mut().unwrap().push_str(&stderr);
                        }

                        self.partial_tool_output_wrapped_line_count = wrapped_string_height(
                            self.partial_tool_output.as_ref().unwrap(),
                            self.chat_area_width,
                        );
                        self.recalculate_scroll_offset();
                    }
                }
                AgentEvent::ToolOutput { stdout, stderr } => {
                    if AppConfig::global().tui.show_tool_output {
                        self.partial_tool_output = None;
                        self.partial_tool_output_wrapped_line_count = 0;
                        self.recalculate_scroll_offset();

                        let mut output = Text::default();
                        output.extend(
                            stdout
                                .into_text()
                                .context("Failed to parse toos stdout as ansi")?,
                        );
                        output.extend(
                            stderr
                                .into_text()
                                .context("Failed to parse toos stderr as ansi")?,
                        );
                        self.push_rendered_message(output);
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle any input events directly from crossterm
    fn handle_input_events(&mut self) -> Result<()> {
        let timeout = Duration::from_millis(16);
        if event::poll(timeout)? {
            let mut consumed = false;

            match event::read()? {
                Event::Key(key_event) => {
                    match key_event.code {
                        KeyCode::Esc => {
                            self.session.save()?;
                            self.exit = true;
                        }
                        KeyCode::F(5) => {
                            self.redraw_once = true;
                            consumed = true;
                        }
                        KeyCode::Tab => {
                            let full_input = self.input.lines().join("\n");
                            if full_input.starts_with("/") && !full_input.starts_with("//") {
                                complete_command(&full_input, self);
                                consumed = true;
                            }
                        }
                        KeyCode::Enter => {
                            if key_event.modifiers.is_empty() {
                                self.submit()?;

                                consumed = true;
                            }
                        }
                        KeyCode::Char('c') => {
                            if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                                // Stop running agent if any
                                self.agent_handle.cancel.cancel();

                                // Clear all temporary buffers
                                self.partial_message = None;
                                self.rendered_partial_message = None;
                                self.rendered_partial_message_wrapped_line_count = 0;
                                self.partial_tool_output = None;
                                self.partial_tool_output_wrapped_line_count = 0;
                                self.permission_request = None;
                                self.rendered_permission_request = None;
                                self.rendered_permission_request_wrapped_line_count = 0;

                                self.recalculate_scroll_offset();

                                self.input.set_placeholder_text("Type Something...");
                                self.status.clear();

                                self.session.save()?;

                                consumed = true;
                            }
                        }
                        // Kinda weird, but works for natural user interaction
                        KeyCode::Up => {
                            self.selection_mode = false;
                            execute!(std::io::stdout(), EnableMouseCapture)?;
                        }
                        KeyCode::Down => {
                            self.selection_mode = false;
                            execute!(std::io::stdout(), EnableMouseCapture)?;
                        }
                        _ => {}
                    }

                    if !consumed {
                        self.input.input(key_event);
                    }

                    // Highlight commands
                    let full_input = self.input.lines().join("\n");
                    if full_input.starts_with("/") && !full_input.starts_with("//") {
                        // Could be a command
                        if is_valid_command(&self.input.lines().join("\n")) {
                            // Is an actual command
                            self.input.set_style(Style::default().green().bold());
                        } else {
                            // Not an actual command
                            self.input.set_style(Style::default().yellow());
                        }
                    } else {
                        self.input.set_style(Style::default())
                    }
                }
                Event::Mouse(mouse_event) => match mouse_event.kind {
                    MouseEventKind::ScrollUp => {
                        self.auto_scroll = false;
                        self.scroll_offset = self.scroll_offset.saturating_sub(2);
                        self.recalculate_scroll_offset();
                    }
                    MouseEventKind::ScrollDown => {
                        self.scroll_offset = self.scroll_offset.saturating_add(2);
                        self.recalculate_scroll_offset();
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.selection_mode = true;
                        execute!(std::io::stdout(), DisableMouseCapture)?;
                    }
                    _ => {}
                },
                Event::Paste(text) => {
                    self.input.insert_str(&text);
                }
                Event::Resize(width, height) => {
                    self.term_width = width as usize;
                    self.term_height = height as usize;
                    self.term_size_changed = true;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn wrapped_text_height(text: &Text, width: usize) -> usize {
    let mut height = 0;
    for line in text.lines.iter() {
        if line.width() <= width {
            height += 1;
        } else {
            height += line.width().div_ceil(width).max(1);
        }
    }

    height
}

fn wrapped_string_height(string: &String, width: usize) -> usize {
    let mut height = 0;
    for line in string.lines() {
        if line.chars().count() <= width {
            height += 1;
        } else {
            height += line.chars().count().div_ceil(width).max(1);
        }
    }

    height
}
