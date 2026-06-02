use std::fs;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use config::{Config, Environment, File};
use log::{debug, info};
use serde::{Deserialize, Serialize};

/// Cached config that is loaded on load() and accessed on global()
static CONFIG_CACHE: OnceLock<AppConfig> = OnceLock::new();

/// Config structure
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub model: ModelConfig,
    pub documents: DocumentConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig::default(),
            documents: DocumentConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub api_key: Option<String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "owl-alpha".to_string(),
            provider: "openrouter".to_string(),
            api_key: None,
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
            user_memory: "USER.md".to_string(),
            system_memory: "MEMORY.md".to_string(),
        }
    }
}

impl AppConfig {
    /// Fetches cached global config.
    pub fn global() -> &'static AppConfig {
        let cached_config = CONFIG_CACHE
            .get()
            .expect("Failed to load cached config");

        debug!("Fetched config from cache");
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
            .add_source(
                File::with_name(config_file.to_str().unwrap())
            )
            .add_source(
                Environment::with_prefix("MIA")
                    .separator("__")
            )
            .build()
            .context("Failed to assemble configuration sources");

        let app_config: AppConfig = config_builder?
            .try_deserialize()?;
        
        let _ = CONFIG_CACHE.set(app_config.clone());
        debug!("Loaded and cached config");

        Ok(app_config)
    }
}
