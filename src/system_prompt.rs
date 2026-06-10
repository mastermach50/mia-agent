use anyhow::Result;
use chrono::Local;
use std::fs;
use os_info;

use crate::config::AppConfig;

// TODO setup caching of system prompt
/// Get the updated system prompt
/// Soul + Context + Memory
pub fn get_system_prompt() -> Result<String> {
    let mut system_prompt = String::new();

    // Soul
    let soul = fs::read_to_string(&AppConfig::global().documents.soul)?;
    system_prompt.push_str(&soul);
    system_prompt.push_str("\n");

    // Context
    let executable = std::env::current_exe()?.into_string().unwrap();
    let config_folder = AppConfig::internal().mia_dir.to_string_lossy();
    let model_name = AppConfig::global().model.name.clone();
    system_prompt.push_str(&indoc::formatdoc! {"
        You are an AI agent running on a custom harness called mia-agent.
        When the user asks you to configure something about yourself this is what they are referring to.
        Always use the tools you have if they even slightly seem be useful for the task.
        Don't assume things, always verify, use your tools to do this if needed.
        
        your_executable: {executable}
        your_config_folder: {config_folder}
        the_model_you_are_running: {model_name}
    "});
    system_prompt.push_str("\n");
    let os_name =  os_info::get().to_string();
    let cwd = std::env::current_dir()?.into_string().unwrap();
    let date_and_hour = Local::now().format("%a, %d %b %Y %I%p %z");
    system_prompt.push_str(&indoc::formatdoc! {"
        # Environment Context
        operating_system: {os_name} 
        current_directory: {cwd}
        date_and_hour (use your datetime tool to get precise time): {date_and_hour}
    "});
    system_prompt.push_str("\n");

    // Memory
    let user_memory_file = AppConfig::global().documents.user_memory.clone();
    let user_memory = fs::read_to_string(&user_memory_file)?
        .lines()
        .filter(|&f| f != "§")
        .collect::<Vec<&str>>()
        .join("\n");
    let system_memory_file = AppConfig::global().documents.system_memory.clone();
    let system_memory = fs::read_to_string(&system_memory_file)?
        .lines()
        .filter(|&f| f != "§")
        .collect::<Vec<&str>>()
        .join("\n");
    system_prompt.push_str( &indoc::formatdoc! {"
        # Memory
        Whenever you learn new things about yourself or about the user that will be relevent in future conversations add it to your memory using your memory tool.
        If the user asks you to remember something, then also use your memory tool to save it to your memory.
        Anything that is in your memory will always be included in your system prompt.
        {user_memory_file} is your memory about the user and {system_memory_file} is your memory about yourself.
        Don't include status of currents tasks in your memory. Only add things that will be relevent long term.
        
        ## User Memory ({user_memory_file})
        {user_memory}

        ## System Memory ({system_memory_file})
        {system_memory}
    "});

    Ok(system_prompt)
}