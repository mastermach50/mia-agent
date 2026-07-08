use anyhow::{Context, Result};
use chrono::Local;
use log::{debug, trace};
use std::fs;

use crate::config::AppConfig;

// TODO setup caching of system prompt
/// Get the updated system prompt
/// Soul + Context + Memory
pub fn get_system_prompt() -> Result<String> {
    let mut system_prompt = String::new();

    // Soul
    if !AppConfig::internal().soul_file.exists() {
        fs::write(
            &AppConfig::internal().soul_file,
            indoc::indoc! {"
            You are Mia, a personal AI agent running on the user's machine.
            You have tools — use them to accomplish tasks rather than describing what you would do.
            "},
        )
        .context("Failed to create soul file")?;
    };
    let soul =
        fs::read_to_string(&AppConfig::internal().soul_file).context("Failed to read soul file")?;
    system_prompt.push_str(&soul);
    system_prompt.push('\n');

    // Context
    let executable = std::env::current_exe()
        .context("Failed to get current exe")?
        .into_string()
        .unwrap();
    let config_folder = AppConfig::internal().mia_dir.to_string_lossy();
    let model_name = AppConfig::global().model.name.clone();
    system_prompt.push_str(&indoc::formatdoc! {"
    # Agent
    Harness: mia-agent
    Config: {config_folder}
    Model: {model_name}
    Executable: {executable}

    # Operating Principles
    - Do, don't describe. Use tools to act. Report results, not intentions.
    - Verify before assuming. Check files and system state with tools before acting on guesses.
    - Complete tasks fully. Execute all steps; don't stop halfway and hand off to the user.
    - Interpret ambiguity, then act. Attempt the most reasonable reading of unclear requests and state what you did.
    - Fail forward. If a tool call fails, read the error, diagnose, and retry or try an alternative.
    - Chain tools freely. Multiple sequential tool calls to complete a task is correct behavior.
    - Prefer specialized tools. Use fs_read_file over `cat`, fs_grep_files over `grep`. Fall back to exec_shell for everything else.
    - Be concise. Don't narrate upcoming tool calls. Do them, then summarize what you found or did.

    # Tool Discipline
    Destructive operations — file writes, overwrites, shell execution — trigger a built-in confirmation prompt shown to the user. Do not add your own pre-warnings; trust the confirmation system.
    If the user denies a request then don't try to do the same thing another way, instead stop and wait for the user to decide what to do.
    When a task requires multiple lookups, plan what you need before calling tools so you gather information systematically rather than reactively.
    If you are unsure whether a path exists or what it contains, check before acting.

    # User
    - Assume competence. The user is an adult, treat them like so.
    - Don't add unsolicited warnings or disclaimers to standard operations.
    - If something is wrong, say so clearly. Don't soften the feedback.
    - Match length to complexity. A short question gets a short answer.
    "});
    system_prompt.push('\n');
    let os_name = os_info::get().to_string();
    let cwd = std::env::current_dir()
        .context("Failed to get current dir")?
        .into_string()
        .unwrap();
    let date_and_hour = Local::now().format("%a, %d %b %Y %I%p %z");
    system_prompt.push_str(&indoc::formatdoc! {"
    # Environment
    operating_system: {os_name}
    current_directory: {cwd}
    datetime_approx: {date_and_hour}
    note: use the datetime tool when precise time is needed
    "});
    system_prompt.push('\n');

    // Memory
    let user_memory_file = AppConfig::internal().user_memory_file.clone();
    if !user_memory_file.exists() {
        fs::File::create(&user_memory_file).context("Failed to create user memory file")?;
        debug!("Created user memory file {:?}", user_memory_file);
    }
    let user_memory = fs::read_to_string(&user_memory_file)
        .context("Failed to read user memory file")?
        .lines()
        .filter(|&f| f != "§")
        .collect::<Vec<&str>>()
        .join("\n");
    let system_memory_file = AppConfig::internal().system_memory_file.clone();
    if !system_memory_file.exists() {
        fs::File::create(&system_memory_file).context("Failed to create system memory file")?;
        debug!("Created system memory file {:?}", system_memory_file);
    }
    let system_memory = fs::read_to_string(&system_memory_file)
        .context("Failed to read system memory file")?
        .lines()
        .filter(|&f| f != "§")
        .collect::<Vec<&str>>()
        .join("\n");
    system_prompt.push_str( &indoc::formatdoc! {"
    # Memory
    You have persistent memory across sessions. Use it actively.
    - Save: user preferences, name, recurring projects, environment quirks, discovered tool paths, constraints.
    - Save: facts about your own setup — configured tools, paths, model behavior notes.
    - Do not save: task outcomes, what you did today, or anything that won't matter next session.
    - When the user asks you to remember something, call the memory tool immediately.
    - When you discover a memory entry is wrong or stale, delete it.

    ## User Memory ({user_memory_file})
    {user_memory}

    ## System Memory ({system_memory_file})
    {system_memory}
    ", user_memory_file = user_memory_file.to_string_lossy(), system_memory_file=system_memory_file.to_string_lossy()});
    system_prompt.push('\n');

    trace!("Retrieved system prompt");

    Ok(system_prompt)
}

pub fn get_tui_system_prompt(help_msg: Option<&str>) -> Result<String> {
    let mut system_prompt = get_system_prompt()?;
    system_prompt.push_str(&format!(
        "You are in a terminal TUI session with {}.",
        AppConfig::global().tui.username
    ));
    system_prompt.push('\n');
    if let Some(help_msg) = help_msg {
        system_prompt.push_str("Commands available to the user:\n");
        system_prompt.push_str(help_msg);
        system_prompt.push('\n');
    }
    Ok(system_prompt)
}
