use std::time::Duration;

use ansi_to_tui::IntoText;
use anyhow::Result;
use crossterm::{
    event::{
        self, Event, KeyCode, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
};
use log::error;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::Stylize,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};
use ratatui_textarea::TextArea;
use termimad::MadSkin;
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::{
    agent_loop,
    agent_tools::ToolRegistry,
    api::{History, Message, PartialMessage},
    config::AppConfig,
    sessions::Session, system_prompt::get_tui_system_prompt,
};

pub async fn run(new_session: bool) -> Result<()> {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvents>();

    let mut state = AppState {
        event_tx: event_tx,
        session: Session::default(),
        input: TextArea::default(),
        status: "".to_string(),
        model: AppConfig::global().model.name.clone(),
        messages: Vec::new(),
        partial_message: None,
        scroll_offset: 0,
        auto_scroll: true,
        exit: false,
    };

    if new_session {
        state.send_harness_message("Started new session.")?;
        state.session = Session::new("user", "tui", "tui");
        state.session.history.set_system_prompt(get_tui_system_prompt(None)?);
    } else {
        if let Ok(session) = Session::load_last_session("user", "tui", "tui") {
            for message in &session.history.messages {
                let rendered_message = render_message(message)?;
                state.messages.push(rendered_message);
            }
            state.send_harness_message("Loaded last session.")?;
            state.session = session;
            
        } else {
            state.send_harness_message("No previous session found, starting new one.")?;
            state.session = Session::new("user", "tui", "tui");
            state.session.history.set_system_prompt(get_tui_system_prompt(None)?);
        }
    };

    let mut terminal = ratatui::init();
    execute!(
        std::io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
    )?;

    while !state.exit {
        terminal.draw(|f| state.draw(f))?;
        handle_key_events(&mut state).await?;
        while let Ok(event) = event_rx.try_recv() {
            state.handle_event(event)?;
        }
    }

    ratatui::restore();
    execute!(std::io::stdout(), PopKeyboardEnhancementFlags)?;
    Ok(())
}

struct AppState {
    event_tx: UnboundedSender<AppEvents>,

    session: Session,

    input: TextArea<'static>,

    status: String,
    model: String,

    messages: Vec<Text<'static>>,
    partial_message: Option<Message>,
    scroll_offset: u16,
    auto_scroll: bool,

    exit: bool,
}

enum AppEvents {
    AssistantMessage(Message),
    PartialAssistantMessage(PartialMessage),
    StatusUpdate(String),
    SystemMessage(String),
    HistoryUpdate(History),
}

impl AppState {
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
        frame.render_widget(&self.input, chunks[2]);

        // Render status bar
        let border_type = if self.auto_scroll {
            BorderType::Plain
        } else {
            BorderType::LightDoubleDashed
        };
        let status_bar = Block::new()
            .border_type(border_type)
            .borders(Borders::TOP)
            .title(Line::from(vec![self.status.clone().yellow()]).alignment(Alignment::Left))
            .title(Line::from(vec![self.model.clone().yellow()]).alignment(Alignment::Right));
        frame.render_widget(status_bar, chunks[1]);

        // Render chat
        let mut display_lines = Vec::new();
        for message in &self.messages {
            display_lines.extend(message.lines.clone());
        }
        if let Some(partial_message) = &self.partial_message {
            let rendered_message =
                render_message(partial_message).expect("Failed to render partial message");
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
    async fn submit(&mut self) -> Result<()> {
        // Ignore if empty
        if self.input.is_empty() {
            return Ok(());
        }

        let lines = self.input.lines();
        let text = lines.join("\n");

        let message = Message::new("user", text);
        let rendered_message = render_message(&message)?;
        self.messages.push(rendered_message);
        self.session.history.add_message(message);
        self.session.save()?;

        self.input.clear();

        let stream = AppConfig::global().tui.streaming;
        let session_id = self.session.get_extended_session_id();
        let tx1 = self.event_tx.clone();
        let tx2 = self.event_tx.clone();
        let tx3 = self.event_tx.clone();
        let tx4 = self.event_tx.clone();
        let tx5 = self.event_tx.clone();
        let history = self.session.history.clone(); // Fix history clone
        tokio::spawn(async move {
            let history = agent_loop::run_agent(
                history,
                &session_id,
                stream,
                move |msg: &Message| {
                    tx1.send(AppEvents::AssistantMessage(msg.clone()))
                        .unwrap();
                },
                move |msg: &PartialMessage| {
                    tx2.send(AppEvents::PartialAssistantMessage(msg.clone()))
                        .unwrap();
                },
                move |kind: &str| {
                    tx3.send(AppEvents::StatusUpdate(kind.to_string()))
                        .unwrap();
                },
                move |msg: &str| {
                    tx4.send(AppEvents::SystemMessage(msg.to_string()))
                        .unwrap();
                },
            )
            .await.unwrap();

            tx5.send(AppEvents::HistoryUpdate(history))
        });

        Ok(())
    }

    /// Send messages about the state of the agent
    fn send_harness_message(&mut self, message: &str) -> Result<()> {
        let message = Message::new("harness", message);
        let rendered_message = render_message(&message)?;
        self.messages.push(rendered_message);

        Ok(())
    }

    /// Handle message events
    fn handle_event(&mut self, event: AppEvents) -> Result<()> {
        match event {
            AppEvents::AssistantMessage(msg) => {
                // Clear the partial message
                self.partial_message = None;

                // Render and display the message
                let rendered_message = render_message(&msg)?;
                self.messages.push(rendered_message);

                // Append the message to the session history and save it
                self.session.history.add_message(msg);
                self.session.save()?;

                // Clear any previous status
                self.status.clear();
            }
            AppEvents::PartialAssistantMessage(msg) => {
                if (msg.reasoning_chunk_index == 0 && msg.content_chunk_index == -1)
                    || (msg.reasoning_chunk_index == -1 && msg.content_chunk_index == 0)
                {
                    self.partial_message = Some(Message {
                        role: msg.role.clone(),
                        reasoning: Some(String::new()),
                        content: Some(String::new()),
                        tool_calls: None,
                        tool_call_id: None,
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
            AppEvents::StatusUpdate(kind) => {
                self.status = kind;
            }
            AppEvents::SystemMessage(msg) => {
                let rendered_message = render_message(&Message::new("harness", msg))?;
                self.messages.push(rendered_message);
                self.status.clear();
            }
            AppEvents::HistoryUpdate(history) => {
                self.session.history = history;
                self.session.save()?;
            }
        }

        Ok(())
    }
}

/// Render out a single Message into Text
/// The rendered message is not wrapped to the width of the terminal
fn render_message(message: &Message) -> Result<Text<'static>> {
    // Ignore actual system and tool response messages
    if message.role == "system" || message.role == "tool" {
        return Ok(Text::default());
    }

    let mut text = Text::default();

    let sender = match message.role.as_str() {
        "user" => "User".green(),
        "assistant" => "Mia".cyan(),
        "harness" => "System".yellow(),
        _ => {
            error!("Unknown role: {}", message.role);
            anyhow::bail!("Unknown role: {}", message.role);
        }
    };

    let short_message = message.reasoning.is_none()
    && message.content.is_some()
    && message.content.as_ref().unwrap().chars().count() < 100;

    if short_message {
        text.push_line(Line::from(vec![
            sender,
            " > ".into(),
            message.content.as_ref().unwrap().to_string().into(),
        ]));
        return Ok(text);
    }

    text.push_line(Line::from(vec![sender, " ◣".into()]));

    if let Some(reasoning) = &message.reasoning
        && !reasoning.is_empty()
    {
        for line in reasoning.split("\n") {
            text.push_line(line.to_string().dark_gray().italic());
        }
    }

    if let Some(content) = &message.content
        && !content.is_empty()
    {
        let skin = MadSkin::default_dark();
        let formatted = skin.text(content, None);
        let ansi_string = formatted.to_string();
        text.extend(ansi_string.into_text()?);
    }

    if let Some(tool_calls) = &message.tool_calls {
        for tool_call in tool_calls {
            text.push_line(Line::from(vec![
                "Mia".cyan(),
                " > ".into(),
                ToolRegistry::tool_icon(&tool_call.function.name)
                    .to_string()
                    .into(),
                " ".into(),
                tool_call.function.name.clone().into(),
                " ".into(),
                ToolRegistry::tool_short(&tool_call.function.name, &tool_call.function.arguments)
                    .into(),
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

async fn handle_key_events(state: &mut AppState) -> Result<()> {
    let timeout = Duration::from_millis(50);
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key_event) => {
                match key_event.code {
                    KeyCode::Esc => {
                        state.session.save()?;
                        state.exit = true;
                    }
                    KeyCode::Enter => {
                        if key_event.modifiers.is_empty() {
                            state.submit().await?;
                        }
                    }
                    KeyCode::Up => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(1);
                        state.auto_scroll = false;
                    }
                    KeyCode::Down => {
                        state.scroll_offset = state.scroll_offset.saturating_add(1);
                    }
                    KeyCode::PageUp => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(10);
                        state.auto_scroll = false;
                    }
                    KeyCode::PageDown => {
                        state.scroll_offset = state.scroll_offset.saturating_add(10);
                    }
                    _ => {}
                }
                let is_scroll_key = matches!(
                    key_event.code,
                    KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
                );

                if !(key_event.code == KeyCode::Enter && key_event.modifiers.is_empty())
                    && !is_scroll_key
                {
                    state.input.input(key_event);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
