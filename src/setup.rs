use anyhow::Result;
use inquire::Confirm;
use inquire::{Password, Select, Text, min_length, required, validator::Validation};
use inquire_derive::Selectable;
use itertools::Itertools;
use std::fs;
use std::{
    fmt::Display,
    io::{self, Write},
};
use termimad::crossterm::style::Stylize;
use toml_edit::{DocumentMut, value};

use crate::{api, config::AppConfig};

#[derive(Debug, Clone, Copy, Selectable)]
enum Providers {
    Openrouter,
    Local,
}

impl Display for Providers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Providers::Openrouter => write!(f, "Openrouter.ai"),
            Providers::Local => write!(f, "Local LLM (Ollama, LMStudio, llama.cpp etc.)"),
        }
    }
}

#[derive(Debug, Clone, Copy, Selectable)]
enum EditModes {
    Keep,
    Change,
}

impl Display for EditModes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EditModes::Keep => write!(f, "Keep"),
            EditModes::Change => write!(f, "Change"),
        }
    }
}

pub async fn setup() -> Result<()> {
    println!("{}", "Model Options".yellow());
    let provider = Providers::select("Provider:").prompt()?;
    match provider {
        Providers::Openrouter => {
            println!("{}", "Base URL found".blue());
            set_provider_base_url(Some("https://openrouter.ai/api/v1")).await?;
            set_api_key("OPENROUTER_API_KEY").await?;
        }
        Providers::Local => {
            println!("{}", "No API key required".blue());
            set_provider_base_url(None).await?;
        }
    }
    set_model_name().await?;
    set_model_reasoning().await?;

    println!("\n{}", "TUI Options".yellow());
    set_tui_username().await?;
    set_tui_streaming().await?;
    set_tui_reasoning().await?;

    println!("\n{}", "Agent Options".yellow());
    set_max_iterations().await?;

    Ok(())
}

async fn set_provider_base_url(url: Option<&str>) -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let http_validator = |input: &str| {
        if input.starts_with("http://") || input.starts_with("https://") {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid("Must be a valid URL".into()))
        }
    };

    if let Some(base_url) = url {
        doc["model"]["base_url"] = value(base_url);
    } else {
        println!("Enter provider base url");
        let base_url = Text::new("Provider Base URL")
            .with_validator(required!())
            .with_validator(http_validator)
            .with_validator(min_length!(1))
            .prompt()?;
        doc["model"]["base_url"] = value(base_url);
    }

    fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;

    Ok(())
}

async fn set_api_key(key: &str) -> Result<()> {
    let env = fs::read_to_string(AppConfig::internal().env_file.clone())?;

    if env.contains(&format!("{key}=")) {
        match EditModes::select("Keep existing key:").prompt()? {
            EditModes::Keep => {}
            EditModes::Change => {
                let api_key = Password::new("Enter API key:").prompt()?;
                let new_env = env
                    .lines()
                    .map(|l| {
                        if l.starts_with(&format!("{key}=")) {
                            format!("{key}={api_key}")
                        } else {
                            l.to_string()
                        }
                    })
                    .join("\n");
                fs::write(AppConfig::internal().env_file.clone(), new_env)?;
            }
        }
    } else {
        let api_key = Password::new("Enter API key:").prompt()?;
        let new_env = env.trim_end().to_owned() + &format!("{key}={api_key}");
        fs::write(AppConfig::internal().env_file.clone(), new_env)?;
    }

    Ok(())
}

async fn set_model_name() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let current_model_name = &AppConfig::global().model.name.clone();

    // Fetch the models
    print!("{}\r", "Fetching models...".blue());
    io::stdout().flush()?;
    let models = api::models().await.unwrap_or(Vec::new());
    let options = models.iter().map(|m| m.id.clone()).collect::<Vec<String>>();

    let model_suggester = |input: &str| {
        let input_lower = input.to_lowercase();
        Ok(options
            .iter()
            .filter(|o| o.contains(&input_lower))
            .map(|s| s.to_string())
            .collect())
    };

    let model = Text::new("Model name:")
        .with_autocomplete(model_suggester)
        .with_default(current_model_name)
        .prompt()?;
    doc["model"]["name"] = value(model);
    fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;

    Ok(())
}

async fn set_model_reasoning() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let levels = vec!["xhigh", "high", "medium", "low", "minimal", "none"];
    let current_level = AppConfig::global().model.reasoning.clone();
    let starting_index = levels.iter().position(|i| i == &current_level).unwrap_or(0);

    let reasoning = Select::new("Model reasoning level:", levels)
        .with_starting_cursor(starting_index)
        .prompt()?;
    doc["model"]["reasoning"] = value(reasoning);
    fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;

    Ok(())
}

async fn set_tui_username() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let username = Text::new("Username:")
        .with_help_message("This is what the assistant will call you")
        .with_default(&AppConfig::global().tui.username)
        .prompt_skippable()?;
    match username {
        Some(username) => {
            doc["tui"]["username"] = value(username);
            fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;
        }
        None => {
            println!("Username unchanged");
        }
    }

    Ok(())
}

async fn set_tui_streaming() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let streaming = Confirm::new("Streaming:")
        .with_help_message("Whether to stream content in the tui")
        .with_default(AppConfig::global().tui.streaming)
        .prompt_skippable()?;
    match streaming {
        Some(streaming) => {
            doc["tui"]["streaming"] = value(streaming);
            fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_tui_reasoning() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let reasoning = Confirm::new("Show reasoning:")
        .with_help_message("Show the reasoning content from the model, if any")
        .with_default(AppConfig::global().tui.show_reasoning)
        .prompt_skippable()?;
    match reasoning {
        Some(reasoning) => {
            doc["tui"]["show_reasoning"] = value(reasoning);
            fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_max_iterations() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let num_validator = |input: &str| {
        if input.parse::<u64>().is_ok() {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid(
                "Value must be a positive number".into(),
            ))
        }
    };

    let max_iter = Text::new("Max iterations:")
        .with_default(&AppConfig::global().agent.max_iterations.to_string())
        .with_validator(num_validator)
        .prompt_skippable()?;
    match max_iter {
        Some(max_iter) => {
            doc["agent"]["max_iterations"] = value(max_iter.parse::<i64>().unwrap());
            fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;
        }
        None => {
            println!("Max iterations unchanged");
        }
    }

    Ok(())
}
