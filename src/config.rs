use std::{env, fs};
use std::sync::OnceLock;

use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};

/// Cached config that is loaded on load() and accessed on global()
static CONFIG_CACHE: OnceLock<AppConfig> = OnceLock::new();

/// Config structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub model: ModelConfig,
    pub documents: DocumentConfig,
    pub agent: AgentConfig,
    pub cli: CliConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            documents: DocumentConfig::default(),
            agent: AgentConfig::default(),
            cli: CliConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelConfig {
    pub name: String,
    pub reasoning: String 
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "owl-alpha".to_string(),
            reasoning: "auto".to_string()
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
            max_iterations: 10,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CliConfig {
    pub username: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            username: "user".to_string(),
        }
    }
}

impl AppConfig {
    /// Fetches cached global config.
    pub fn global() -> &'static AppConfig {
        let cached_config = CONFIG_CACHE
            .get()
            .expect("Failed to load cached config");

        trace!("Fetched config from cache");
        cached_config
    }

    /// Loads config to the cache and returns it.
    /// If the config file doesn't exist, it creates a default one and then loads it.
    pub fn load() -> Result<Self> {
        let home_dir = std::env::home_dir().unwrap();
        let mia_dir = home_dir.join(".mia");
        let env_file = mia_dir.join(".env");
        let config_file = mia_dir.join("config.toml");

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

        let _ = CONFIG_CACHE.set(parsed_config.clone());
        debug!("Loaded and cached config");

        Ok(parsed_config)
    }

    /// Executes checks and actions to be done right after config load
    fn post_config_load(mut config: AppConfig) -> Result<AppConfig> {

        // Check if required api keys are present in env
        if env::var("OPENROUTER_API_KEY").is_err() {
            error!("OPENROUTER_API_KEY not set in .env");
            anyhow::bail!("OPENROUTER_API_KEY not set in .env");
        };

        // Make soul, user memory and system memory files if they don't exist
        let mia_dir = std::env::home_dir().unwrap().join(".mia");
        let soul_path = mia_dir.join(config.documents.soul.clone());
        let user_memory_path = mia_dir.join(config.documents.user_memory.clone());
        let system_memory_path = mia_dir.join(config.documents.system_memory.clone());

        let initial_soul = "You are Mia, a personal assistant. Respond accurately and concisely";
        if !soul_path.exists() {
            fs::write(&soul_path, initial_soul)
                .context("Failed to create soul file")?;
            info!("Created soul file at {:?}", soul_path);
        };
        if !user_memory_path.exists() {
            if user_memory_path.parent().is_some() {
                fs::create_dir_all(user_memory_path.parent().unwrap())
                    .context("Failed to create user memory directory")?;
            }
            fs::write(&user_memory_path, "")
                .context("Failed to create user memory file")?;
            info!("Created user memory file at {:?}", user_memory_path);
        };
        if !system_memory_path.exists() {
            if system_memory_path.parent().is_some() {
                fs::create_dir_all(system_memory_path.parent().unwrap())
                    .context("Failed to create system memory directory")?;
            }
            fs::write(&system_memory_path, "")
                .context("Failed to create system memory file")?;
            info!("Created system memory file at {:?}", system_memory_path);
        };

        config.documents.soul = soul_path.to_str().unwrap().to_string();
        config.documents.user_memory = user_memory_path.to_str().unwrap().to_string();
        config.documents.system_memory = system_memory_path.to_str().unwrap().to_string();

        Ok(config)
    }
}
