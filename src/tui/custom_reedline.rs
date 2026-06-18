// Honestly most of this is pieced together from ai generated code, I don't know how reedline properly works,
// but it is too good not to use in this project

use anyhow::Result;
use nu_ansi_term::{Color, Style};
use reedline::{
    ColumnarMenu, DefaultCompleter, EditCommand, Emacs, ExampleHighlighter, FileBackedHistory,
    KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptHistorySearchStatus, Reedline, ReedlineEvent,
    ReedlineMenu, default_emacs_keybindings, kitty_protocol_available,
};
use termimad::crossterm::{
    event::{
        DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    style::Stylize,
};

use crate::config::AppConfig;

pub fn get_reedline(commands: Vec<String>) -> Result<(Reedline, impl Prompt, KittyProtocol)> {
    execute!(std::io::stdout(), EnableBracketedPaste)?;

    // Initialize the Kitty protocol before building Reedline
    let kitty_enabled = kitty_protocol_available();
    if kitty_enabled {
        execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }
    let kitty_protocol = KittyProtocol { kitty_enabled };

    // History handling
    let history = Box::new(
        FileBackedHistory::with_file(1000, AppConfig::global().tui.history_file.clone().into())
            .unwrap_or_else(|_| FileBackedHistory::new(1000).unwrap()),
    );

    // Completion Menu
    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_text_style(Style::new().fg(Color::Green)),
    );

    // Edit Mode
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    // Always submit on enter
    keybindings.add_binding(KeyModifiers::NONE, KeyCode::Enter, ReedlineEvent::Submit);
    // Newline on shift+enter (only on terminals with kitty protocol)
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    // Newline on alt+enter (backup for terminals without kitty protocol)
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    // Handle Ctrl+C inside Reedline if it passes through the terminal
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('c'),
        ReedlineEvent::CtrlC,
    );

    // Handle Ctrl+D (EOF / Exit) inside Reedline
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('d'),
        ReedlineEvent::CtrlD,
    );
    let edit_mode = Box::new(Emacs::new(keybindings));

    // Custom Completer
    let mut completer = Box::new(DefaultCompleter::with_inclusions(&['/', '-', '_']));
    completer.insert(commands.clone());

    // Custom Highlighter
    let mut hilighter = Box::new(ExampleHighlighter::new(commands.clone()));
    hilighter.change_colors(
        nu_ansi_term::Color::Green,
        nu_ansi_term::Color::Default,
        nu_ansi_term::Color::Default,
    );

    // Custom Prompt
    let prompt = CustomPrompt;

    let rl = Reedline::create()
        .with_history(history)
        .with_highlighter(hilighter)
        .with_history_exclusion_prefix(Some(" ".into()))
        .with_completer(completer)
        .with_partial_completions(true)
        .with_quick_completions(true)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    Ok((rl, prompt, kitty_protocol))
}

struct CustomPrompt;
impl Prompt for CustomPrompt {
    fn render_prompt_left(&self) -> std::borrow::Cow<'_, str> {
        "User ".blue().to_string().into()
    }
    fn render_prompt_right(&self) -> std::borrow::Cow<'_, str> {
        "".into()
    }
    fn render_prompt_indicator(
        &self,
        _prompt_mode: reedline::PromptEditMode,
    ) -> std::borrow::Cow<'_, str> {
        "> ".into()
    }
    fn render_prompt_multiline_indicator(&self) -> std::borrow::Cow<'_, str> {
        "   ::: ".into()
    }
    fn render_prompt_history_search_indicator(
        &self,
        history_search: reedline::PromptHistorySearch,
    ) -> std::borrow::Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };

        format!(" ({} reverse-search: {}) ", prefix, history_search.term).into()
    }
}

pub struct KittyProtocol {
    kitty_enabled: bool,
}
impl KittyProtocol {
    pub fn suspend(&self) {
        if self.kitty_enabled {
            let _ = execute!(std::io::stdout(), DisableBracketedPaste);
            let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
        }
    }
    pub fn resume(&self) {
        let _ = execute!(std::io::stdout(), EnableBracketedPaste);
        if self.kitty_enabled {
            let _ = execute!(
                std::io::stdout(),
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            );
        }
    }
}
impl Drop for KittyProtocol {
    fn drop(&mut self) {
        let _ = execute!(std::io::stdout(), EnableBracketedPaste);
        if self.kitty_enabled {
            let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
        }
    }
}
