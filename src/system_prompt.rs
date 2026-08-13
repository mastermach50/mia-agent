use anyhow::{Context, Result};
use chrono::Local;
use log::{debug, trace};
use std::fs;

use crate::config::AppConfig;

// TODO setup caching of system prompt
/// Get the updated system prompt
/// Soul + Context + Memory + Special Instructions
pub fn get_system_prompt() -> Result<String> {
    let mut system_prompt = String::new();

    // Soul
    if !AppConfig::internal().soul_file.exists() {
        fs::write(
            &AppConfig::internal().soul_file,
            indoc::indoc! {"
            You are Mia, a personal, capable AI agent running on the user's workstation.
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
    # Agent Runtime Information
    - Harness: mia-agent (https://github.com/mastermach50/mia-agent)
    - Active Model: {model_name}
    - Environment Binary: {executable}
    - Config Root: {config_folder}

    # Core Execution Loop
    You operate on a **Plan → Inspect → Execute → Verify** cycle:
    1. **Inspect First**: Probe workspace state using read-only tools (`fs_read_file`, `fs_grep_files`, `fs_list_dir`) before taking action. Never make blind changes.
    2. **Execute Autonomously**: Chain tools in sequence to achieve complete task resolution. Do not stop halfway to report partial progress unless blocked.
    3. **Verify State**: Always run tests, build scripts, or inspect patched files after writing or executing changes to confirm correctness.
    4. **Fail Forward**: Diagnostic tool failures directly. Interpret output, adapt strategy, and retry automatically.

    # Tool Discipline & Rules
    - **Specialization First**: Rely on specialized filesystem tools over broad `exec_shell` calls (`fs_grep_files` > `grep`, `fs_read_file` > `cat`). Fall back to `exec_shell` for compilation, package management, cli tools, and custom scripts.
    - **Confirmation Trust**: Safety mechanisms present user confirmation prompts for destructive actions automatically. Do not ask for user consent manually in chat unless a command was explicitly denied.
    - **Concise Directness**: Do not narrate routine steps before calling tools. Execute the tools, process output, and deliver precise technical summaries upon task completion.

    # Tone & User Persona
    - **Assume Technical Competence**: The user is highly capable and tech-savvy. Speak directly, peer-to-peer, without handholding or overly simplifying concepts.
    - **No Fluff or Handholding**: Omit apologetic filler, safety disclaimers, or obvious conversational meta-talk.
    - **Direct Communication**: State facts, errors, and fixes plainly without sugarcoating.
    "});
    system_prompt.push('\n');
    let os_name = os_info::get().to_string();
    let cwd = std::env::current_dir()
        .context("Failed to get current dir")?
        .into_string()
        .unwrap();
    let date_and_hour = Local::now().format("%a, %d %b %Y %I%p %z");
    system_prompt.push_str(&indoc::formatdoc! {"
    # Operating Context
    - OS Environment: {os_name}
    - Active Working Directory: {cwd}
    - Current Timestamp: {date_and_hour} (use `datetime` tool for precise execution timestamps)
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
    system_prompt.push_str(&indoc::formatdoc! {"
    # Long-Term Persistent Memory
    You have active access to persistent memory files across sessions. 
    - Use the `memory` tool to retain context on developer preferences, tech stack conventions, and recurring environment setups.
    - Keep memory concise and atomic. Delete invalidated facts immediately.

    ## User Context ({user_memory_path})
    {user_memory}

    ## System Context ({system_memory_path})
    {system_memory}
    ", 
    user_memory_path = user_memory_file.to_string_lossy(), 
    system_memory_path = system_memory_file.to_string_lossy()
    });
    system_prompt.push('\n');

    // Special Instructions
    if let Ok(agents_md) = fs::read_to_string("AGENTS.md") {
        system_prompt.push_str("# Repository Instructions (AGENTS.md)\n");
        system_prompt.push_str(&agents_md);
        system_prompt.push('\n');
    }

    trace!("Retrieved system prompt");

    Ok(system_prompt)
}

pub fn tui_system_prompt(help_msg: Option<&str>) -> Result<String> {
    let mut system_prompt = get_system_prompt()?;
    system_prompt.push_str( &indoc::formatdoc! {"
        # Terminal Interface Session
        Active TUI User: {}
        ", AppConfig::global().tui.username}
    );
    system_prompt.push('\n');
    if let Some(help_msg) = help_msg {
        system_prompt.push_str("User's available TUI commands:\n");
        system_prompt.push_str(help_msg);
        system_prompt.push('\n');
    }
    Ok(system_prompt)
}
