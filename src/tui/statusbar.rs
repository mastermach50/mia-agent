use ratatui::{
    style::Stylize,
    text::Line,
    widgets::{Block, BorderType, Borders},
};

use crate::{config::AppConfig, tui::AppState};

// Create a statusbar block based on the app state
pub fn create_statusbar(state: &mut AppState) -> Block<'static> {
    let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

    let border_type = if state.auto_scroll {
        BorderType::Plain
    } else {
        BorderType::LightDoubleDashed
    };

    let mut statusbar = Block::new().border_type(border_type).borders(Borders::TOP);

    if !state.status.is_empty() {
        if AppConfig::global().tui.show_spinner && state.permission_request.is_none() {
            statusbar = statusbar
                .title(Line::from(spinner_frames[state.spinner_idx].cyan()).left_aligned());
            state.spinner_idx = (state.spinner_idx + 1) % spinner_frames.len();
        }

        if !state.status.is_empty() {
            statusbar = statusbar.title(Line::from(state.status.clone().yellow()).left_aligned());
        }
    };

    if state.yolo {
        statusbar = statusbar.title(Line::from("[yolo]".red()).left_aligned());
    }

    if state.completion_tokens > 0 && state.prompt_tokens > 0 {
        statusbar = statusbar.title(
            Line::from(vec![
                "(".yellow(),
                state.prompt_tokens.to_string().blue(),
                "|".yellow(),
                state.completion_tokens.to_string().blue(),
                "|".yellow(),
                state.total_tokens.to_string().blue(),
                ")".yellow(),
            ])
            .right_aligned(),
        );
    }

    statusbar = statusbar.title(Line::from(state.model.clone().yellow()).right_aligned());

    statusbar
}
