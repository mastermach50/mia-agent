use std::fmt::Display;
use anyhow::Result;

use inquire::{Password, Text, required, validator::Validation};
use inquire_derive::Selectable;
use itertools::Itertools;
use toml_edit::{DocumentMut, value};
use std::fs;

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, Selectable)]
enum Providers {
    Openrouter,
    Local
}

impl Display for Providers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Providers::Openrouter => write!(f, "Openrouter.ai"),
            Providers::Local => write!(f, "Local LLM (Ollama, LMStudio, llama.cpp etc.)")
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
            EditModes::Change => write!(f, "Change")
        }
    }
}

pub fn setup() -> Result<()> {
    println!("Select LLM Provider");
    let provider = Providers::select("Provider:").prompt()?;

    match provider {
        Providers::Openrouter => {
            println!("Base URL found");
            set_provider_base_url(Some("https://openrouter.ai/api/v1"))?;
            set_api_key("Openrouter", "OPENROUTER_API_KEY")?;
        }
        Providers::Local => {
            println!("No API key required");
            set_provider_base_url(None)?;
        }
    }

    set_tui_username()?;
    set_max_iterations()?;

    Ok(())
}

fn set_provider_base_url(url: Option<&str>) -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;
    if let Some(base_url) = url {
        doc["model"]["base_url"] = value(base_url);
    } else {
        println!("Enter provider base url");
        let base_url = Text::new("Provider Base URL")
            .with_validator(required!())
            .prompt()?;
        doc["model"]["base_url"] = value(base_url);
    }

    fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;

    Ok(())
}

fn set_api_key(provider: &str, key: &str) -> Result<()>{
    let env = fs::read_to_string(AppConfig::internal().env_file.clone())?;

    if env.contains(&format!("{key}=")) {
        println!("{} key found, do you want to keep it?", provider);
        match EditModes::select("Edit:").prompt()? {
            EditModes::Keep => {
                println!("Keeping current API key");
            },
            EditModes::Change => {
                let api_key = Password::new("Enter API key:").prompt()?;
                let new_env = env.lines()
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

fn set_tui_username() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let username = Text::new("Username (what your assistant should call you):")
        .with_default(&AppConfig::global().tui.username)
        .prompt_skippable()?;
    match username {
        Some(username) => {
            doc["tui"]["username"] = value(username);
            fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;
        },
        None => {
            println!("Username unchanged");
        }
    }

    Ok(())
}

fn set_max_iterations() -> Result<()> {
    let config = fs::read_to_string(AppConfig::internal().config_file.clone())?;
    let mut doc = config.parse::<DocumentMut>()?;

    let validator = |input: &str| {
        if input.parse::<u64>().is_ok() {
            Ok(Validation::Valid)
        } else {
            Ok(Validation::Invalid("Value must be a positive number".into()))
        }
    };

    let max_iter = Text::new("Max iterations:")
        .with_default(&AppConfig::global().agent.max_iterations.to_string())
        .with_validator(validator)
        .prompt_skippable()?;
    match max_iter {
        Some(max_iter) => {
            doc["agent"]["max_iterations"] = value(max_iter.parse::<i64>().unwrap());
            fs::write(AppConfig::internal().config_file.clone(), doc.to_string())?;
        },
        None => {
            println!("Max iterations unchanged");
        }
    }

    Ok(())
}