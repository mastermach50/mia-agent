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
use strum::{EnumIter, IntoEnumIterator};
use termimad::crossterm::style::Stylize;
use toml_edit::{DocumentMut, value};

use crate::{api, config::AppConfig};

#[derive(Debug, Clone, Copy, Selectable, EnumIter, PartialEq)]
pub enum Providers {
    Openrouter,
    Local,
    Groq,
    Cerebras,
    GoogleAIStudio,
}

impl Display for Providers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Providers::Openrouter => write!(f, "Openrouter.ai"),
            Providers::Local => write!(f, "Local LLM (Ollama, LMStudio, llama.cpp etc.)"),
            Providers::Groq => write!(f, "Groq"),
            Providers::Cerebras => write!(f, "Cerebras.ai"),
            Providers::GoogleAIStudio => write!(f, "Google AI Studio"),
        }
    }
}

impl Providers {
    pub fn from_name(name: &str) -> Option<Self> {
        Providers::iter().find(|p| p.name() == name)
    }

    pub fn index(&self) -> usize {
        Providers::iter().position(|p| &p == self).unwrap()
    }

    pub fn name(&self) -> &str {
        match self {
            Providers::Openrouter => "openrouter",
            Providers::Local => "local",
            Providers::Groq => "groq",
            Providers::Cerebras => "cerebras",
            Providers::GoogleAIStudio => "google_ai_studio",
        }
    }

    pub fn api_key_name(&self) -> Option<&str> {
        match self {
            Providers::Openrouter => Some("OPENROUTER_API_KEY"),
            Providers::Local => None,
            Providers::Groq => Some("GROQ_API_KEY"),
            Providers::Cerebras => Some("CEREBRAS_API_KEY"),
            Providers::GoogleAIStudio => Some("GOOGLE_AI_STUDIO_API_KEY"),
        }
    }

    pub fn base_url(&self) -> Result<&str> {
        match self {
            Providers::Openrouter => Ok("https://openrouter.ai/api/v1"),
            Providers::Local => anyhow::bail!("Local provider doesn't have default base url"),
            Providers::Groq => Ok("https://api.groq.com/openai/v1"),
            Providers::Cerebras => Ok("https://api.cerebras.ai/v1"),
            Providers::GoogleAIStudio => {
                Ok("https://generativelanguage.googleapis.com/v1beta/openai/")
            }
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
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    println!("{}", "Model Options".yellow());
    set_model_provider(&mut doc).await?;
    set_model_name(&mut doc).await?;
    set_model_reasoning(&mut doc).await?;

    println!("\n{}", "TUI Options".yellow());
    set_tui_username(&mut doc).await?;
    set_tui_streaming(&mut doc).await?;
    set_tui_reasoning(&mut doc).await?;

    println!("\n{}", "Agent Options".yellow());
    set_max_iterations(&mut doc).await?;

    fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;

    Ok(())
}

async fn set_model_provider(doc: &mut DocumentMut) -> Result<()> {
    let current_provider_name = AppConfig::global().model.provider.clone();
    let current_provider = Providers::from_name(&current_provider_name).unwrap();

    let provider = Providers::select("Provider:")
        .with_starting_cursor(current_provider.index())
        .with_help_message("Select the LLM provider, can be running on device or remotely")
        .prompt()?;

    // Only provide the current base url as default if the provider was already "local"
    let default_base_url = if doc["model"]["provider"].as_str().unwrap() == "local" {
        doc["model"]["base_url"].as_str().unwrap().to_string()
    } else {
        String::new()
    };

    doc["model"]["provider"] = value(provider.name());
    match provider {
        Providers::Local => {
            println!("{}", "No API key required".blue());
            let base_url = Text::new("Provider Base URL")
                .with_validator(required!())
                .with_validator(http_validator)
                .with_validator(min_length!(1))
                .with_default(&default_base_url)
                .with_help_message("Must include the api version at the end, if present. (Like /v1)")
                .prompt()?;
            doc["model"]["base_url"] = value(base_url);
        }
        _ => {
            println!("Base URL found ({})", provider.base_url()?.blue());
            doc["model"]["base_url"] = value(provider.base_url()?);
            set_api_key(provider.api_key_name().unwrap()).await?;
        }
    }

    Ok(())
}

async fn set_api_key(key: &str) -> Result<()> {
    let env = fs::read_to_string(AppConfig::internal().env_file.clone())?;

    // TODO fix
    // Could fail if there is a space before key name
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
        let new_env = env.trim_end().to_owned() + &format!("\n{key}={api_key}");
        fs::write(AppConfig::internal().env_file.clone(), new_env)?;
    }

    Ok(())
}

async fn set_model_name(doc: &mut DocumentMut) -> Result<()> {
    let base_url = doc["model"]["base_url"].as_str().unwrap();
    let provider_name = doc["model"]["provider"].as_str().unwrap();
    let provider = Providers::from_name(provider_name).unwrap();
    let api_key_name = provider.api_key_name();

    // Fetch the models
    print!("{}\r", "Fetching models...".blue());
    io::stdout().flush()?;
    let models = api::models(base_url, api_key_name)
        .await
        .unwrap_or(Vec::new());
    let options = models.iter().map(|m| m.id.clone()).collect::<Vec<String>>();

    let model_suggester = |input: &str| {
        let input_lower = input.to_lowercase();
        Ok(options
            .iter()
            .filter(|o| o.to_lowercase().contains(&input_lower))
            .map(|s| s.to_string())
            .collect())
    };

    let default_model_name = if provider_name == AppConfig::global().model.provider {
        AppConfig::global().model.name.clone()
    } else {
        String::new()
    };

    let model = Text::new("Model name:")
        .with_autocomplete(model_suggester)
        .with_default(&default_model_name)
        .prompt()?;
    doc["model"]["name"] = value(model);

    Ok(())
}

async fn set_model_reasoning(doc: &mut DocumentMut) -> Result<()> {
    let provider = Providers::from_name(doc["model"]["provider"].as_str().unwrap()).unwrap();

    let levels = match provider {
        Providers::Openrouter => vec!["xhigh", "high", "medium", "low", "minimal", "none"],
        Providers::Groq => vec!["high","medium","low","default", "none"],
        Providers::Cerebras => vec!["high", "medium", "low", "none"],
        Providers::GoogleAIStudio => vec!["high", "medium", "low", "none"], // not verified
        Providers::Local => vec!["max", "high", "medium", "low", "none"],
    };
    let current_level = AppConfig::global().model.reasoning.clone();
    let starting_index = levels.iter().position(|i| i == &current_level).unwrap_or(0);

    let reasoning = Select::new("Model reasoning level:", levels)
        .with_starting_cursor(starting_index)
        .prompt()?;
    doc["model"]["reasoning"] = value(reasoning);

    Ok(())
}

async fn set_tui_username(doc: &mut DocumentMut) -> Result<()> {
    let username = Text::new("Username:")
        .with_help_message("This is what the assistant will call you")
        .with_default(&AppConfig::global().tui.username)
        .prompt_skippable()?;
    match username {
        Some(username) => {
            doc["tui"]["username"] = value(username);
        }
        None => {
            println!("Username unchanged");
        }
    }

    Ok(())
}

async fn set_tui_streaming(doc: &mut DocumentMut) -> Result<()> {
    let streaming = Confirm::new("Streaming:")
        .with_help_message("Whether to stream content in the tui")
        .with_default(AppConfig::global().tui.streaming)
        .prompt_skippable()?;
    match streaming {
        Some(streaming) => {
            doc["tui"]["streaming"] = value(streaming);
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_tui_reasoning(doc: &mut DocumentMut) -> Result<()> {
    let reasoning = Confirm::new("Show reasoning:")
        .with_help_message("Show the reasoning content from the model, if any")
        .with_default(AppConfig::global().tui.show_reasoning)
        .prompt_skippable()?;
    match reasoning {
        Some(reasoning) => {
            doc["tui"]["show_reasoning"] = value(reasoning);
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_max_iterations(doc: &mut DocumentMut) -> Result<()> {
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
        .with_help_message("Maximum number of turns the agent can have before stopping and waiting for user input, higher values required for longer tasks")
        .with_validator(num_validator)
        .prompt_skippable()?;
    match max_iter {
        Some(max_iter) => {
            doc["agent"]["max_iterations"] = value(max_iter.parse::<i64>().unwrap());
        }
        None => {
            println!("Max iterations unchanged");
        }
    }

    Ok(())
}

fn http_validator(
    input: &str,
) -> core::result::Result<Validation, Box<dyn std::error::Error + Send + Sync + 'static>> {
    if input.starts_with("http://") || input.starts_with("https://") {
        Ok(Validation::Valid)
    } else {
        Ok(Validation::Invalid("Must be a valid URL".into()))
    }
}
