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
use toml_edit::value;

use crate::cli::SetupArgs;
use crate::{api, config::AppConfig};

#[derive(Debug, Clone, Copy, Selectable, EnumIter, PartialEq)]
pub enum Providers {
    Openrouter,
    Local,
    Groq,
    Cerebras,
    GoogleAIStudio,
    OpenCodeZen,
    NvidiaNIM,
}

impl Display for Providers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Providers::Openrouter => write!(f, "Openrouter.ai"),
            Providers::Local => write!(f, "Local LLM (Ollama, LMStudio, llama.cpp etc.)"),
            Providers::Groq => write!(f, "Groq"),
            Providers::Cerebras => write!(f, "Cerebras.ai"),
            Providers::GoogleAIStudio => write!(f, "Google AI Studio"),
            Providers::OpenCodeZen => write!(f, "OpenCode Zen"),
            Providers::NvidiaNIM => write!(f, "Nvidia NIM"),
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
            Providers::OpenCodeZen => "opencode_zen",
            Providers::NvidiaNIM => "nvidia_nim",
        }
    }

    pub fn api_key_name(&self) -> Option<&str> {
        match self {
            Providers::Openrouter => Some("OPENROUTER_API_KEY"),
            Providers::Local => None,
            Providers::Groq => Some("GROQ_API_KEY"),
            Providers::Cerebras => Some("CEREBRAS_API_KEY"),
            Providers::GoogleAIStudio => Some("GOOGLE_AI_STUDIO_API_KEY"),
            Providers::OpenCodeZen => Some("OPENCODE_ZEN_API_KEY"),
            Providers::NvidiaNIM => Some("NVIDIA_NIM_API_KEY"),
        }
    }

    pub fn base_url(&self) -> Result<&str> {
        match self {
            Providers::Openrouter => Ok("https://openrouter.ai/api/v1"),
            Providers::Local => anyhow::bail!("Local provider doesn't have default base url"),
            Providers::Groq => Ok("https://api.groq.com/openai/v1"),
            Providers::Cerebras => Ok("https://api.cerebras.ai/v1"),
            Providers::GoogleAIStudio => {
                Ok("https://generativelanguage.googleapis.com/v1beta/openai")
            }
            Providers::OpenCodeZen => Ok("https://opencode.ai/zen/v1"),
            Providers::NvidiaNIM => Ok("https://integrate.api.nvidia.com/v1"),
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

pub async fn setup(args: SetupArgs) -> Result<()> {
    let noargs = !(args.agent || args.model || args.tui);

    if noargs || args.model {
        println!("{}", "Model Options".yellow());
        set_model_provider().await?;
        set_model_name().await?;
        set_model_reasoning().await?;
    }

    if noargs || args.tui {
        println!("\n{}", "TUI Options".yellow());
        set_tui_username().await?;
        set_tui_streaming().await?;
        set_tui_reasoning().await?;
        set_tui_render_markdown().await?;
        set_tui_show_tool_output().await?;
    }

    if noargs || args.agent {
        println!("\n{}", "Agent Options".yellow());
        set_max_iterations().await?;
    }

    Ok(())
}

async fn set_model_provider() -> Result<()> {
    let current_provider = AppConfig::data().provider;

    let provider = Providers::select("Provider:")
        .with_starting_cursor(current_provider.index())
        .with_help_message("Select the LLM provider, can be running on device or remotely")
        .prompt()?;

    // Only provide the current base url as default if the provider was already "local"
    let default_base_url = if AppConfig::config().model.provider == "local" {
        AppConfig::config().model.base_url
    } else {
        String::new()
    };

    AppConfig::edit_exact(|doc| {
        doc["model"]["provider"] = value(provider.name());
    })?;
    match provider {
        Providers::Local => {
            println!("{}", "No API key required".blue());
            let base_url = Text::new("Provider Base URL")
                .with_validator(required!())
                .with_validator(http_validator)
                .with_validator(min_length!(1))
                .with_default(&default_base_url)
                .with_help_message(
                    "Must include the api version at the end, if present. (Like /v1)",
                )
                .prompt()?;
            AppConfig::edit_exact(|doc| {
                doc["model"]["base_url"] = value(base_url);
            })?;
        }
        _ => {
            println!("Base URL found ({})", provider.base_url()?.blue());
            AppConfig::edit_exact(|doc| {
                doc["model"]["base_url"] = value(provider.base_url().unwrap());
            })?;
            set_api_key(provider.api_key_name().unwrap()).await?;
        }
    }

    Ok(())
}

async fn set_api_key(key: &str) -> Result<()> {
    let env = fs::read_to_string(AppConfig::tree().env_file.clone())?;

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
                fs::write(AppConfig::tree().env_file.clone(), new_env)?;
            }
        }
    } else {
        let api_key = Password::new("Enter API key:").prompt()?;
        let new_env = env.trim_end().to_owned() + &format!("\n{key}={api_key}");
        fs::write(AppConfig::tree().env_file.clone(), new_env)?;
    }

    Ok(())
}

async fn set_model_name() -> Result<()> {
    let base_url = &AppConfig::config().model.base_url;
    let provider = AppConfig::data().provider;
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

    let default_model_name = if provider.name() == AppConfig::config().model.provider {
        AppConfig::config().model.name.clone()
    } else {
        String::new()
    };

    let model = Text::new("Model name:")
        .with_autocomplete(model_suggester)
        .with_default(&default_model_name)
        .prompt()?;
    AppConfig::edit_exact(|doc| {
        doc["model"]["name"] = value(model);
    })?;
    Ok(())
}

async fn set_model_reasoning() -> Result<()> {
    let provider = AppConfig::data().provider;

    let levels = match provider {
        Providers::Openrouter => vec!["xhigh", "high", "medium", "low", "minimal", "none"],
        Providers::Groq => vec!["high", "medium", "low", "default", "none"],
        Providers::Cerebras => vec!["high", "medium", "low", "none"],
        Providers::GoogleAIStudio => vec!["high", "medium", "low", "none"], // not verified
        Providers::Local => vec!["max", "high", "medium", "low", "none"],
        Providers::OpenCodeZen => vec!["xhigh", "high", "medium", "low", "minimal", "none"], // not verified
        Providers::NvidiaNIM => vec!["xhigh", "high", "medium", "low", "minimal", "none"], // not verified
    };
    let current_level = AppConfig::config().model.reasoning.clone();
    let starting_index = levels.iter().position(|i| i == &current_level).unwrap_or(0);

    let reasoning = Select::new("Model reasoning level:", levels)
        .with_starting_cursor(starting_index)
        .prompt()?;
    AppConfig::edit_exact(|doc| {
        doc["model"]["reasoning"] = value(reasoning);
    })?;
    Ok(())
}

async fn set_tui_username() -> Result<()> {
    let username = Text::new("Username:")
        .with_help_message("This is what the assistant will call you")
        .with_default(&AppConfig::config().tui.username)
        .prompt_skippable()?;
    match username {
        Some(username) => {
            AppConfig::edit_exact(|doc| {
                doc["tui"]["username"] = value(username);
            })?;
        }
        None => {
            println!("Username unchanged");
        }
    }

    Ok(())
}

async fn set_tui_streaming() -> Result<()> {
    let streaming = Confirm::new("Streaming:")
        .with_help_message("Whether to stream content in the tui")
        .with_default(AppConfig::config().tui.streaming)
        .prompt_skippable()?;
    match streaming {
        Some(streaming) => {
            AppConfig::edit_exact(|doc| {
                doc["tui"]["streaming"] = value(streaming);
            })?;
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_tui_reasoning() -> Result<()> {
    let reasoning = Confirm::new("Show reasoning:")
        .with_help_message("Show the reasoning content from the model, if any")
        .with_default(AppConfig::config().tui.show_reasoning)
        .prompt_skippable()?;
    match reasoning {
        Some(reasoning) => {
            AppConfig::edit_exact(|doc| {
                doc["tui"]["show_reasoning"] = value(reasoning);
            })?;
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_tui_render_markdown() -> Result<()> {
    let render_markdown = Confirm::new("Render markdown:")
        .with_help_message("Render markdown content with formatting")
        .with_default(AppConfig::config().tui.render_markdown)
        .prompt_skippable()?;
    match render_markdown {
        Some(render_markdown) => {
            AppConfig::edit_exact(|doc| {
                doc["tui"]["render_markdown"] = value(render_markdown);
            })?;
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_tui_show_tool_output() -> Result<()> {
    let show_tool_output = Confirm::new("Show tool output:")
        .with_help_message("Display the ouputs of tools called by the agent")
        .with_default(AppConfig::config().tui.show_tool_output)
        .prompt_skippable()?;
    match show_tool_output {
        Some(show_tool_output) => {
            AppConfig::edit_exact(|doc| {
                doc["tui"]["show_tool_output"] = value(show_tool_output);
            })?;
        }
        None => {
            println!("Value unchanged");
        }
    }

    Ok(())
}

async fn set_max_iterations() -> Result<()> {
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
        .with_default(&AppConfig::config().agent.max_iterations.to_string())
        .with_help_message("Maximum number of turns the agent can have before stopping and waiting for user input, higher values required for longer tasks")
        .with_validator(num_validator)
        .prompt_skippable()?;
    match max_iter {
        Some(max_iter) => {
            AppConfig::edit_exact(|doc| {
                doc["agent"]["max_iterations"] = value(max_iter.parse::<i64>().unwrap());
            })?;
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
