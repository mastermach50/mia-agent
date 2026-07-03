use std::time::Duration;

use crossterm::{event::{self, Event, KeyCode, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags}, execute};
use ratatui::{Frame, layout::{Constraint, Direction, Layout}, style::{Color, Stylize}, text::{Line, Span, Text}, widgets::{Block, BorderType, Borders, Paragraph, Wrap}};
use anyhow::Result;
use ratatui_textarea::{TextArea, WrapMode};

use crate::{config::AppConfig, sessions::Session};

pub async fn run(new_session: bool) -> Result<()> {

    let mut state = AppState::new();

    let mut session = if new_session {
        state.on_system_message("Started new session.");
        Session::new("user", "tui", "tui")
    } else {
        if let Ok(session) = Session::load_last_session("user", "tui", "tui") {
            state.on_system_message("Loaded last session.");
            session
        } else {
            state.on_system_message("No previous session found, starting new one.");
            Session::new("user", "tui", "tui")
        }
    };

    let mut terminal = ratatui::init();
    execute!(
        std::io::stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        ),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
        )
    )?;

    while !state.exit {
        terminal.draw(|f| state.draw(f))?;
        handle_key_events(&mut state, &mut session)?;
    }

    ratatui::restore();
    execute!(
        std::io::stdout(),
        PopKeyboardEnhancementFlags
    )?;
    Ok(())
}

struct AppState<'a> {
    input: TextArea<'a>,

    status: String,
    model: String,

    complete_messages: Vec<TUIMessage<'a>>,
    partial_message: String,
    scroll_offset: u32,

    exit: bool,
}

enum TUIMessage<'a> {
    Message(Role, Text<'a>),
    ToolCall(String, String, String),
}

enum Role {
    User,
    System,
    Assistant,
}

impl<'a> AppState<'a> {
    fn new() -> Self {
        let mut input = TextArea::default();
        input.set_wrap_mode(WrapMode::WordOrGlyph);
        input.set_block(Block::new().bg(Color::Gray));

        Self {
            input: input,

            status: String::new(),
            model: AppConfig::global().model.name.clone(),

            complete_messages: Vec::new(),
            partial_message: String::new(),
            scroll_offset: 0,

            exit: false,
        }
    }

    fn submit(&mut self) {
        if !self.input.is_empty() {
            self.complete_messages.push(TUIMessage::Message(
                Role::User,
                Text::from(self.input.lines())
            ));
            self.input.clear();
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let input_width = area.width.max(1) as usize;

        let input_height: u16 = self.input.lines().iter()
            .map(|line| {
                let len = line.chars().count().max(1);
                ((len + input_width - 1) / input_width) as u16
            })
            .sum::<u16>()
            .max(1);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(input_height)
            ])
            .split(frame.area());

        let mut full_text: Text = Text::default();
        for item in &self.complete_messages {
            match item {
                TUIMessage::Message(role, text) => {
                    let name = match role {
                        Role::User => "User".green().into(),
                        Role::System => "System".yellow().into(),
                        Role::Assistant => "Mia".cyan().into(),
                    };
                    full_text.push_line(Line::from(vec![
                        name,
                        " ◣".into(),
                    ]));
                    full_text.extend(text.clone().lines);
                }
                _ => {}
            }
        }
        let messages = Paragraph::new(full_text);
        frame.render_widget(messages, chunks[0]);
        frame.render_widget(&self.input, chunks[1]);
    }

    fn on_system_message(&mut self, message: &'a str) {
        self.complete_messages.push(TUIMessage::Message(
            Role::System,
            Text::from(message)
        ));
    }
}

fn handle_key_events(state: &mut AppState, session: &mut Session) -> Result<()> {
    let timeout = Duration::from_millis(50);
    if event::poll(timeout)? {
        match event::read()? {
            Event::Key(key_event) => {
                match key_event.code {
                    KeyCode::Esc => {
                        session.save()?;
                        state.exit = true;
                    }
                    KeyCode::Enter=> {
                        if key_event.modifiers.is_empty() {
                            state.submit();
                        }
                    }
                    _ => {}
                }
                if !(key_event.code == KeyCode::Enter && key_event.modifiers.is_empty()) {
                    state.input.input(key_event);
                }
            }
            _ => {}
        }
    }

    Ok(())
}