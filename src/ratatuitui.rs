use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{
        self, Event, KeyCode, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use ratatui_textarea::TextArea;

use crate::{api::Message, sessions::Session};

pub async fn run(new_session: bool) -> Result<()> {
    let mut state = AppState {
        session: Session::default(),
        input: TextArea::default(),
        messages: Vec::new(),
        partial_message: String::new(),
        exit: false,
    };

    let session = if new_session {
        state.on_system_message("Started new session.".to_string());
        Session::new("user", "tui", "tui")
    } else {
        if let Ok(session) = Session::load_last_session("user", "tui", "tui") {
            state.on_system_message("Loaded last session.".to_string());
            session
        } else {
            state.on_system_message("No previous session found, starting new one.".to_string());
            Session::new("user", "tui", "tui")
        }
    };

    state.session = session;

    let mut terminal = ratatui::init();
    execute!(
        std::io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
    )?;

    while !state.exit {
        terminal.draw(|f| state.draw(f))?;
        handle_key_events(&mut state).await?;
    }

    ratatui::restore();
    execute!(std::io::stdout(), PopKeyboardEnhancementFlags)?;
    Ok(())
}

struct AppState {
    session: Session,

    input: TextArea<'static>,

    messages: Vec<RenderAwareTUIMessage>,
    partial_message: String,

    exit: bool,
}

struct RenderAwareTUIMessage {
    message: TUIMessage,
    cached_text: Text<'static>,
    rendered: bool,
}

enum TUIMessage {
    TextMessage {
        role: Role,
        reasoning: String,
        content: String,
    },

    ToolCallNotifier {
        icon: String,
        name: String,
        shorthand: String,
    },
}

#[derive(Clone, Copy)]
enum Role {
    User,
    Assistant,
    System,
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

        frame.render_widget(&self.input, chunks[2]);

        self.pre_render_messages(frame);

        let mut display_lines = Vec::new();
        for ra_message in &self.messages {
            display_lines.extend(ra_message.cached_text.lines.clone());
        }
        let messages_paragraph = Paragraph::new(display_lines).wrap(Wrap { trim: false });

        frame.render_widget(messages_paragraph, chunks[0]);
    }

    /// Take in the messages and then prerender it and store it in the cache.
    /// Only needs to render uncached messsages
    fn pre_render_messages(&mut self, frame: &mut Frame) {
        let area_width = frame.area().width as usize;

        for ra_message in &mut self.messages {
            if ra_message.rendered {
                continue;
            }

            match &mut ra_message.message {
                TUIMessage::TextMessage {
                    role,
                    reasoning,
                    content,
                } => {
                    let sender = match role {
                        Role::User => "User".green(),
                        Role::Assistant => "Mia".cyan(),
                        Role::System => "System".yellow(),
                    };

                    let short_line = reasoning.is_empty()
                        && !content.contains("\n")
                        && content.chars().count() < (area_width - sender.width() - 10);

                    if short_line {
                        ra_message.cached_text.push_line(Line::from(vec![
                            sender,
                            " > ".into(),
                            content.clone().into(),
                        ]));
                        ra_message.rendered = true;
                        continue;
                    } else {
                        ra_message
                            .cached_text
                            .push_line(Line::from(vec![sender, " ◣".into()]));
                    }

                    if !reasoning.is_empty() {
                        let text_width = (frame.area().width - 2) as usize;
                        let wrapped = textwrap::wrap(reasoning, text_width);

                        for line in wrapped {
                            ra_message
                                .cached_text
                                .push_line(Line::from(vec!["| ".into(), line.into_owned().into()]));
                        }
                    }
                    if !content.is_empty() {
                        for line in content.split("\n") {
                            ra_message
                                .cached_text
                                .push_line(Line::from(line.to_string()));
                        }
                    }

                    ra_message.rendered = true;
                }
                TUIMessage::ToolCallNotifier {
                    icon,
                    name,
                    shorthand,
                } => {
                    ra_message.cached_text.push_line(Line::from(vec![
                        "Mia".cyan(),
                        " > ".into(),
                        icon.to_string().into(),
                        name.to_string().into(),
                        shorthand.to_string().into(),
                    ]));
                    ra_message.rendered = true;
                }
            }
        }
    }

    /// Take in the current contents of the input and make it into a new message
    fn submit(&mut self) -> Result<()> {
        if !self.input.is_empty() {
            let lines = self.input.lines();
            let text = lines.join("\n");

            self.messages.push(RenderAwareTUIMessage {
                message: TUIMessage::TextMessage {
                    role: Role::User,
                    reasoning: String::new(),
                    content: text.clone(),
                },
                cached_text: Text::default(),
                rendered: false,
            });

            self.session.history.add_message(Message::new("user", text));

            self.input.clear();
        }

        Ok(())
    }

    fn on_system_message(&mut self, message: String) {
        self.messages.push(RenderAwareTUIMessage {
            message: TUIMessage::TextMessage {
                role: Role::System,
                reasoning: String::new(),
                content: message,
            },
            cached_text: Text::default(),
            rendered: false,
        });
    }
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
                            state.submit()?;
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
