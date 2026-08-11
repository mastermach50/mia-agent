use ratatui::{
    style::Stylize,
    text::{Line, Span, Text},
};

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
