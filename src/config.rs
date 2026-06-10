use std::path::PathBuf;
use std::{env, fs};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use log::{debug, warn, info, trace};
use serde::{Deserialize, Serialize};

/// Cached config that is loaded on load() and accessed on global()
static APP_CONFIG_CACHE: OnceLock<AppConfig> = OnceLock::new();

/// Cached internal config
static INTERNAL_CONFIG_CACHE: OnceLock<InternalConfig> = OnceLock::new();

/// Config structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub model: ModelConfig,
    pub documents: DocumentConfig,
    pub agent: AgentConfig,
    pub tui: TuiConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            documents: DocumentConfig::default(),
            agent: AgentConfig::default(),
            tui: TuiConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelConfig {
    pub base_url: String,
    pub name: String,
    pub reasoning: String 
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            base_url: "https://openrouter.ai/api/v1".to_string(),
            name: "openrouter/owl-alpha".to_string(),
            reasoning: "medium".to_string()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DocumentConfig {
    pub soul: String,
    pub user_memory: String,
    pub system_memory: String,
}

impl Default for DocumentConfig {
    fn default() -> Self {
        Self {
            soul: "SOUL.md".to_string(),
            user_memory: "memories/USER.md".to_string(),
            system_memory: "memories/MEMORY.md".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentConfig {
    pub max_iterations: i32,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TuiConfig {
    pub username: String,
    pub history_file: String,
    pub show_reasoning: bool,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            username: "user".to_string(),
            history_file: ".mia_tui_history".to_string(),
            show_reasoning: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InternalConfig {
    pub home_dir: PathBuf,
    pub mia_dir: PathBuf,
    pub config_file: PathBuf,
    pub env_file: PathBuf,
    pub sessions_dir: PathBuf,
    #[allow(dead_code)] // TODO remove when gateways implemented
    pub gateways_dir: PathBuf,
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

    pub fn internal() -> &'static InternalConfig {
        if let Some(cached) = INTERNAL_CONFIG_CACHE.get() {
            return cached;
        }

        let home_dir = std::env::home_dir().unwrap();
        let mia_dir = home_dir.join(".mia");
        let env_file = mia_dir.join(".env");
        let config_file = mia_dir.join("config.toml");
        let sessions_dir = mia_dir.join("sessions");
        let gateways_dir = mia_dir.join("gateways");

        INTERNAL_CONFIG_CACHE.set(InternalConfig {
            home_dir,
            mia_dir,
            config_file,
            env_file,
            sessions_dir,
            gateways_dir,
        }).unwrap();

        INTERNAL_CONFIG_CACHE.get().unwrap()
    }

    /// Loads config to the cache and returns it.
    /// If the config file doesn't exist, it creates a default one and then loads it.
    pub fn load() -> Result<Self> {
        let home_dir = Self::internal().home_dir.clone();
        let mia_dir = Self::internal().mia_dir.clone();
        let env_file = Self::internal().env_file.clone();
        let config_file = Self::internal().config_file.clone();

        // Create home dir, mia dir, config.toml and .env if they don't exist
        if !home_dir.exists() {
            fs::create_dir_all(&home_dir)
                .context("Failed to create home dir")?;
            info!("Created home directory at {:?}", home_dir);
        }
        if !mia_dir.exists() {
            fs::create_dir_all(&mia_dir)
                .context("Failed to create agent home dir")?;
            info!("Created agent home directory at {:?}", mia_dir);
        }
        if !env_file.exists() {
            fs::write(&env_file, "")?;
            info!("Created default .env file at {:?}", env_file);
        }
        if !config_file.exists() {
            fs::write(&config_file, toml::to_string(&AppConfig::default())?)
                .context("Failed to create default config file")?;
            info!("Created default config file at {:?}", config_file);
        }

        // Load .env to environment variables
        dotenvy::from_path(&env_file)
            .context("Failed to load .env file")?;
        debug!("Loaded .env file from {:?}", env_file);

        // Build config with priority
        // Env > Config file
        let config_builder = Config::builder()
            // Hardcoded default config
            .add_source(
                File::from_str(&toml::to_string(&AppConfig::default())?, FileFormat::Toml).required(true)
            )

            // Config file
            .add_source(
                File::with_name(config_file.to_str().unwrap())
            )

            // Environment variables
            .add_source(
                Environment::with_prefix("MIA")
                    .separator("__")
            )

            .build()
            .context("Failed to assemble configuration sources");

        let app_config: AppConfig = config_builder?
            .try_deserialize()?;
        
        let parsed_config = Self::post_config_load(app_config)?;

        let _ = APP_CONFIG_CACHE.set(parsed_config.clone());
        debug!("Loaded and cached config");

        Ok(parsed_config)
    }

    /// Executes checks and actions to be done right after config load
    fn post_config_load(mut config: AppConfig) -> Result<AppConfig> {

        // Check if required api keys are present in env
        if config.model.base_url.contains("openrouter.ai") && env::var("OPENROUTER_API_KEY").is_err() {
            warn!("OPENROUTER_API_KEY not set in .env");
        };

        // Check for Tavily API key (warning only, tools will just be unavailable)
        if env::var("TAVILY_API_KEY").is_err() {
            warn!("TAVILY_API_KEY not set in .env - web_search and web_extract tools will be unavailable");
        }

        // Make required folders if they don't exist
        let sessions_dir = Self::internal().mia_dir.join("sessions");
        if !sessions_dir.exists() {
            fs::create_dir(&sessions_dir)
                .context("Failed to create sessions dir")?;
            info!("Created sessions directory at {:?}", sessions_dir)
        }
        let gateways_dir = Self::internal().mia_dir.join("gateways");
        if !gateways_dir.exists() {
            fs::create_dir(&gateways_dir)
                .context("Failed to create gateways dir")?;
            info!("Created gateways directory at {:?}", gateways_dir)
        }

        // In the config expand the paths
        let mia_dir = std::env::home_dir().unwrap().join(".mia");
        config.documents.soul = mia_dir.join(config.documents.soul).to_str().unwrap().to_string();
        config.documents.user_memory = mia_dir.join(config.documents.user_memory).to_str().unwrap().to_string();
        config.documents.system_memory = mia_dir.join(config.documents.system_memory).to_str().unwrap().to_string();
        config.tui.history_file = mia_dir.join(config.tui.history_file).to_str().unwrap().to_string();

        // Make all necessary files if they don't exist
        let paths: Vec<PathBuf> = vec![
            config.documents.soul.parse()?,
            config.documents.user_memory.parse()?,
            config.documents.system_memory.parse()?,
            config.tui.history_file.parse()?,
        ];
        for path in paths {
            if !path.exists() {
                // Create parent directories if they don't exist
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                    .context(format!("Failed to create directory {:?}", parent))?;
                }
                // Create file if it doesn't exist
                fs::File::create(&path)
                .context(format!("Failed to create file {:?}", path))?;

                // Write the initial soul if the file had to be created
                if path == config.documents.soul {
                    let initial_soul = "You are Mia, a personal assistant. Respond accurately and concisely";
                    fs::write(path, initial_soul)?;
                }
            }
        }

        // Make sure there is no / at the end of the base url
        if config.model.base_url.ends_with('/') {
            config.model.base_url.pop();
        }

        Ok(config)
    }
}
