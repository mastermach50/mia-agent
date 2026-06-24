use ratatui::layout::{Constraint, Direction, Layout};
use ratatui_textarea::{Input, Key, TextArea};

struct App<'a> {
    messages: Vec<(String, String)>,
    textarea: TextArea<'a>,
    scroll_offset: usize,
    total_lines: usize,
    viewport_height: usize,
    status: String,
}

impl<'a> App<'a> {
    fn new() -> Self {
        let mut textarea = TextArea::default();

        // styling

        Self {
            messages: vec![
                ("Star".into(), "Hi therre".into()),
                ("Marco".into(), "Hi the\n\n\n\n\n\n\n\rre".into()),
                ("Genie".into(), "yooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooo".into()),
                ("Alice".into(), "Hi therre".into()),
            ],
            textarea,
            scroll_offset: 0,
            total_lines: 0,
            viewport_height: 0,
            status: "50% used".into()
        }
    }

    fn submit(&mut self) {
        let text = self.textarea.lines().join("\n");
        let trimmed = text.trim().to_string();
        if !trimmed.is_empty() {
            self.messages.push(("user".into(), trimmed));
            self.scroll_offset = 0;
        }
    }

    fn ui(f: &mut ratatui::Frame, app: &mut App) {
        let total_area = f.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(total_area);

        let msg_area = chunks[0];
        let bar_area = chunks[1];
        let input_area = chunks[2];

        // render_messages(f, app, msg_area);
        // render_bar(f, app, bar_area);
        f.render_widget(&app.textarea, input_area);
    }
}
