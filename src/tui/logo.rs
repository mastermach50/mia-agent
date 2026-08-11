use ratatui::{style::Stylize, text::{Line, Span, Text}};

use crate::tui::{AppState, wrapped_text_height};


pub fn get_logo() -> Text<'static> {
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

/// Push the logo to the rendered messages
pub fn push_logo(state: &mut AppState) {
    // Push the logo to the rendered messages while also updating wrapped line_count
    // This is the only place where state.rendered_messages must be modified directly
    let logo = get_logo();
    state.wrapped_line_count += wrapped_text_height(&logo, state.chat_area_width);
    state.rendered_messages.push(logo);
}