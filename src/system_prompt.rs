use anyhow::Result;
use chrono::Local;
use std::fs;

use crate::config::AppConfig;

// TODO setup caching of system prompt
/// Get the updated system prompt
/// Soul + Context + Memory
pub fn get_system_prompt() -> Result<String> {
    let mut system_prompt = String::new();

    // Soul
    let soul = fs::read_to_string(&AppConfig::internal().soul_file)?;
    system_prompt.push_str(&soul);
    system_prompt.push('\n');

    // Context
    let executable = std::env::current_exe()?.into_string().unwrap();
    let config_folder = AppConfig::internal().mia_dir.to_string_lossy();
    let model_name = AppConfig::global().model.name.clone();
    system_prompt.push_str(&indoc::formatdoc! {"
        You are an AI agent running on a custom harness called mia-agent.
        When the user asks you to configure something about yourself this is what they are referring to.
        All your config files are present in your_config_folder.
        If any tool you have seems useful for any task then always use them.
        Don't assume things, always verify with the help of your tools.
        
        your_executable: {executable}
        your_config_folder: {config_folder}
        the_model_you_are_running: {model_name}
    "});
    system_prompt.push('\n');
    let os_name = os_info::get().to_string();
    let cwd = std::env::current_dir()?.into_string().unwrap();
    let date_and_hour = Local::now().format("%a, %d %b %Y %I%p %z");
    system_prompt.push_str(&indoc::formatdoc! {"
        # Environment Context
        operating_system: {os_name} 
        current_directory: {cwd}
        date_and_hour (use your datetime tool to get precise time): {date_and_hour}
    "});
    system_prompt.push('\n');

    // Memory
    let user_memory_file = AppConfig::internal().user_memory_file.clone();
    let user_memory = fs::read_to_string(&user_memory_file)?
        .lines()
        .filter(|&f| f != "§")
        .collect::<Vec<&str>>()
        .join("\n");
    let system_memory_file = AppConfig::internal().system_memory_file.clone();
    let system_memory = fs::read_to_string(&system_memory_file)?
        .lines()
        .filter(|&f| f != "§")
        .collect::<Vec<&str>>()
        .join("\n");
    system_prompt.push_str( &indoc::formatdoc! {"
        # Memory
        Whenever you learn new things about yourself or about the user that will be relevent later, add it to your memory using your memory tool.
        If the user asks you to remember something, then also use your memory tool to save it to your memory.
        Anything that is in your memory will always be included in your system prompt.
        {user_memory_file} is your memory about the user and {system_memory_file} is your memory about yourself.
        Don't include status of tasks, or what tasks were done in your memory. Only add things that will be relevent long term.
        
        ## User Memory ({user_memory_file})
        {user_memory}

        ## System Memory ({system_memory_file})
        {system_memory}
    ", user_memory_file = user_memory_file.to_string_lossy(), system_memory_file=system_memory_file.to_string_lossy()});

    Ok(system_prompt)
}

pub fn get_tui_system_prompt(help_msg: Option<&str>) -> Result<String> {
    let mut system_prompt = get_system_prompt()?;
    system_prompt.push_str(&format!(
        "\nYou are talking to {} via a TUI.",
        AppConfig::global().tui.username
    ));
    if let Some(help_msg) = help_msg {
        system_prompt.push('\n');
        system_prompt.push_str("This are the options available to the user in the TUI\n");
        system_prompt.push_str(&help_msg);
    }
    Ok(system_prompt)
}
