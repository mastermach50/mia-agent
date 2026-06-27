use std::path::PathBuf;
use std::sync::OnceLock;
use std::{env, fs};

use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use log::{debug, info, trace, warn};
use serde::{Deserialize, Serialize};

use crate::setup::Providers;

/// Cached config that is loaded on load() and accessed on global()
static APP_CONFIG_CACHE: OnceLock<AppConfig> = OnceLock::new();

/// Cached internal config
static INTERNAL_CONFIG_CACHE: OnceLock<InternalConfig> = OnceLock::new();

/// App config structure for the config stored in config.toml
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
            name: "openrouter/owl-alpha".to_string(),
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
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            username: "user".to_string(),
            // history_file: ".mia_tui_history".to_string(),
            max_history: 1000,
            show_reasoning: true,
            streaming: true,
            show_spinner: true,
        }
    }
}

/// Internal config structure that defines paths of files and folders
/// that needs to be accessed in several places in the code
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct InternalConfig {
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
    /// Fetches cached global config.
    pub fn global() -> &'static AppConfig {
        let cached_config = APP_CONFIG_CACHE
            .get()
            .expect("Failed to load cached config");

        trace!("Fetched config from cache");
        cached_config
    }

    /// Used to access the internal config
    /// Also handles caching of it
    pub fn internal() -> &'static InternalConfig {
        // Return cached config if it exists
        if let Some(cached) = INTERNAL_CONFIG_CACHE.get() {
            return cached;
        }

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

        INTERNAL_CONFIG_CACHE
            .set(InternalConfig {
                home_dir,
                mia_dir,
                config_file,
                env_file,
                sessions_dir,
                gateways_dir,
                soul_file,
                memory_dir,
                user_memory_file,
                system_memory_file,
                tui_history_file,
            })
            .unwrap();

        INTERNAL_CONFIG_CACHE.get().unwrap()
    }

    /// Loads config to the cache and returns it.
    /// If the config file doesn't exist, it creates a default one and then loads it.
    /// Also creates necessary paths for intial startup.
    pub fn load() -> Result<Self> {
        // Create home dir, mia dir, config.toml and .env if they don't exist
        // These are the only paths necessary for startup
        // Other paths can be created later
        let home_dir = Self::internal().home_dir.clone();
        if !home_dir.exists() {
            fs::create_dir_all(&home_dir).context("Failed to create home dir")?;
            info!("Created home directory at {:?}", home_dir);
        }
        let mia_dir = Self::internal().mia_dir.clone();
        if !mia_dir.exists() {
            fs::create_dir_all(&mia_dir).context("Failed to create agent home dir")?;
            info!("Created agent home directory at {:?}", mia_dir);
        }
        let env_file = Self::internal().env_file.clone();
        if !env_file.exists() {
            fs::write(&env_file, "")?;
            info!("Created default .env file at {:?}", env_file);
        }
        let config_file = Self::internal().config_file.clone();
        if !config_file.exists() {
            fs::write(&config_file, toml::to_string(&AppConfig::default())?)
                .context("Failed to create default config file")?;
            info!("Created default config file at {:?}", config_file);
        }

        // Load .env to environment variables
        dotenvy::from_path(&env_file).context("Failed to load .env file")?;
        debug!("Loaded .env file from {:?}", env_file);

        // Build config with priority
        // Env > Config file
        let config_builder = Config::builder()
            // Hardcoded default config
            .add_source(
                File::from_str(&toml::to_string(&AppConfig::default())?, FileFormat::Toml)
                    .required(true),
            )
            // Config file
            .add_source(File::with_name(config_file.to_str().unwrap()))
            // Environment variables
            .add_source(Environment::with_prefix("MIA").separator("__"))
            .build()
            .context("Failed to assemble configuration sources");

        let app_config: AppConfig = config_builder?.try_deserialize()?;

        let parsed_config = Self::post_config_load(app_config)?;

        let _ = APP_CONFIG_CACHE.set(parsed_config.clone());
        debug!("Loaded and cached config");

        Ok(parsed_config)
    }

    /// Executes checks and actions to be done right after config load
    fn post_config_load(mut config: AppConfig) -> Result<AppConfig> {
        // Validate provider and check if api key is present
        let provider_name = config.model.provider.clone();
        let provider = Providers::from_name(&provider_name);
        if let Some(provider) = provider {
            if provider != Providers::Local {
                let api_key_name = provider.api_key_name().unwrap();
                if env::var(api_key_name).is_err() {
                    anyhow::bail!("{api_key_name} not set in .env");
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

        // Make required folders if they don't exist
        let sessions_dir = Self::internal().sessions_dir.clone();
        if !sessions_dir.exists() {
            fs::create_dir(&sessions_dir).context("Failed to create sessions dir")?;
            info!("Created sessions directory at {:?}", sessions_dir)
        }
        let gateways_dir = Self::internal().gateways_dir.clone();
        if !gateways_dir.exists() {
            fs::create_dir(&gateways_dir).context("Failed to create gateways dir")?;
            info!("Created gateways directory at {:?}", gateways_dir)
        }

        // Make sure there is no / at the end of the base url
        if config.model.base_url.ends_with('/') {
            config.model.base_url.pop();
        }

        Ok(config)
    }
}
