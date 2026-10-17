use std::path::PathBuf;
use std::sync::{LazyLock, RwLock};
use std::{env, fs};

use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use struct_iterable::Iterable;

use crate::setup::Providers;

static APP_CONFIG_CACHE: LazyLock<RwLock<AppConfig>> = LazyLock::new(|| {
    let config = AppConfig::load().unwrap();
    RwLock::new(config)
});

#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub model: ModelConfig,
    pub agent: AgentConfig,
    pub tui: TuiConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelConfig {
    pub provider: String,
    pub base_url: String,
    pub name: String,
    pub reasoning: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: "openrouter".to_string(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            name: "nvidia/nemotron-3.5-super-vl:free".to_string(),
            reasoning: "medium".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentConfig {
    pub max_iterations: i32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self { max_iterations: 20 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TuiConfig {
    pub username: String,
    pub max_history: usize,
    pub show_reasoning: bool,
    pub streaming: bool,
    pub show_spinner: bool,
    pub render_markdown: bool,
    pub show_tool_output: bool,
    pub use_nerdfont: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            username: "user".to_string(),
            max_history: 1000,
            show_reasoning: true,
            streaming: true,
            show_spinner: true,
            render_markdown: true,
            show_tool_output: true,
            use_nerdfont: false,
        }
    }
}

#[derive(Clone, Debug, Iterable)]
pub struct ConfigTree {
    pub home_dir: PathBuf,
    pub mia_dir: PathBuf,
    pub config_file: PathBuf,
    pub env_file: PathBuf,
    pub sessions_dir: PathBuf,
    pub gateways_dir: PathBuf,
    pub soul_file: PathBuf,
    pub memory_dir: PathBuf,
    pub user_memory_file: PathBuf,
    pub system_memory_file: PathBuf,
    pub tui_history_file: PathBuf,
}

impl AppConfig {
    /// Returns the paths to all the files and folders that are required for the
    /// agent to function.
    ///
    /// If the required file or folder does not already exist then creates it.
    pub fn tree() -> ConfigTree {
        let home_dir = std::env::home_dir().unwrap();
        let mia_dir = home_dir.join(".mia");
        let env_file = mia_dir.join(".env");
        let config_file = mia_dir.join("config.toml");
        let sessions_dir = mia_dir.join("sessions");
        let gateways_dir = mia_dir.join("gateways");
        let soul_file = mia_dir.join("SOUL.md");
        let memory_dir = mia_dir.join("memories");
        let user_memory_file = memory_dir.join("USER.md");
        let system_memory_file = memory_dir.join("MEMORY.md");
        let tui_history_file = mia_dir.join(".mia_tui_history");

        let tree = ConfigTree {
            home_dir,
            mia_dir,
            env_file,
            config_file,
            sessions_dir,
            gateways_dir,
            soul_file,
            memory_dir,
            user_memory_file,
            system_memory_file,
            tui_history_file,
        };

        for (k, v) in tree.iter() {
            if let Some(path) = v.downcast_ref::<PathBuf>() {
                if !path.exists() {
                    if k.ends_with("dir") {
                        fs::create_dir(path).unwrap();
                    } else if k.ends_with("file") {
                        fs::File::create(path).unwrap();
                    }
                }
            }
        }

        tree
    }

    /// Loads the app config using
    /// Env variables > config.toml > internal defaults
    /// and returns it.
    pub fn load() -> Result<Self> {
        // Load environment variables from .env file
        let env_file = AppConfig::tree().env_file;
        dotenvy::from_path(AppConfig::tree().env_file).context("Failed to load .env file")?;
        debug!("Loaded .env file from {:?}", env_file);

        let config_file = AppConfig::tree().config_file;
        let default_config_toml = toml::to_string(&AppConfig::default())?;
        let config_builder = Config::builder()
            // The internal defaults
            .add_source(File::from_str(&default_config_toml, FileFormat::Toml))
            // The config file
            .add_source(File::with_name(config_file.to_str().unwrap()))
            // The environment variables
            .add_source(Environment::with_prefix("MIA").separator("__"))
            .build()
            .context("Failed to assemble configuration sources");

        let app_config: AppConfig = config_builder?.try_deserialize()?;
        Self::validate_config(app_config.clone())?;
        debug!("Loaded and cached config");

        Ok(app_config)
    }

    /// Get the app config
    pub fn config() -> AppConfig {
        APP_CONFIG_CACHE.read().unwrap().clone()
    }

    /// Validate the app config
    fn validate_config(config: AppConfig) -> Result<()> {
        // Validate provider and check if api key is present
        let provider_name = config.model.provider.clone();
        let provider = Providers::from_name(&provider_name);
        if let Some(provider) = provider {
            if provider != Providers::Local {
                let api_key_name = provider.api_key_name().unwrap();
                if env::var(api_key_name).is_err() {
                    warn!("{api_key_name} not set in .env");
                }
            }
        } else {
            anyhow::bail!("Unknown provider: {}", provider_name);
        }

        // Check for Tavily API key (warning only, tools will just be unavailable)
        if env::var("TAVILY_API_KEY").is_err() {
            warn!(
                "TAVILY_API_KEY not set in .env - web_search and web_extract tools will be unavailable"
            );
        }

        Ok(())
    }

    /// Process the app config into usable values
    /// These processed values are stored in
    fn process_config(config: AppConfig) -> Result<()> {
        Ok(())
    }
}
