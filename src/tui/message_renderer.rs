use ansi_to_tui::IntoText;
use anyhow::{Context, Ok, Result};
use log::error;
use ratatui::{
    style::Stylize,
    text::{Line, Text},
};
use termimad::MadSkin;

use crate::{agent_tools::ToolRegistry, api::Message, config::AppConfig, tui::{AppState, logo::{get_logo}}};

/// Renders an `api::Message` into a `ratatui::Text`.
///
/// Only returns a text if the message is not a system or tool response message.
/// Only contents of the messages are ever hard wrapped, and that too only if markdown rendering is enabled.
pub fn render_message(message: &Message, width: usize) -> Result<Option<Text<'static>>> {
    // Ignore system and tool response messages
    if message.role == "system" || message.role == "tool" {
        return Ok(Some(Text::default()));
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
        && sender.width() + 3 + message.content.as_ref().unwrap().chars().count() < width;

    if short_message {
        text.push_line(Line::from(vec![
            sender,
            " ▶ ".into(),
            message.content.as_ref().unwrap().to_string().into(),
        ]));
        return Ok(Some(text));
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
        if AppConfig::global().tui.render_markdown {
            let skin = MadSkin::default_dark();
            let formatted = skin.text(content, Some(width));
            let ansi_string = formatted.to_string();
            text.extend(
                ansi_string
                    .into_text()
                    .context("Failed to convert ansi to ratatui text")?,
            );
        } else {
            for line in content.split("\n") {
                text.push_line(line.to_string());
            }
        }
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

    Ok(Some(text))
}

/// Render all the messages in the session to the rendered_messages
pub fn render_all_messages(state: &mut AppState) -> Result<()> {
    state.rendered_messages.clear();
    state.wrapped_line_count = 0;

    state.push_rendered_message(get_logo());

    for message in state.session.history.messages.clone() {
        if let Some(rendered_message) = render_message(&message, state.chat_area_width)? {
            state.push_rendered_message(rendered_message);
        }
    }

    Ok(())
}