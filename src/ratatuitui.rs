use anyhow::Result;
use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Direction, Layout},
    style::Stylize,
    text::{Line, Text},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};
use ratatui_textarea::TextArea;
use reedline::KeyModifiers;

pub async fn run(new_session: bool) -> Result<()> {
    ratatui::run(|terminal| App::init().run(terminal))?;
    Ok(())
}

struct App {
    messages: Vec<(String, String)>,
    textbox: TextArea<'static>,
    status_message: String,
    exit: bool,
}
impl App {
    fn init() -> Self {
        let mut textbox = TextArea::default();
        textbox.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        );

        Self {
            messages: vec![],
            textbox: textbox,
            status_message: String::new(),
            exit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Min(1), Constraint::Length(3)])
            .split(frame.area());

        frame.render_widget(Paragraph::new("Messages"), sections[0]);

        self.textbox.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(Line::from(self.status_message.clone().yellow()).right_aligned()),
        );

        frame.render_widget(&self.textbox, sections[1]);
    }

    fn handle_events(&mut self) -> Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                match key_event.code {
                    KeyCode::Esc => {
                        self.exit = true;
                    }
                    KeyCode::Enter => {
                        if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                            self.textbox.insert_newline();
                        } else {
                            let text = self.textbox.lines().join("\n");
                            self.status_message = text;
                            self.textbox = TextArea::default();
                        }
                    }
                    _ => {
                        self.textbox.input(key_event);
                    }
                }
            }
            _ => {}
        };

        Ok(())
    }
}
