use std::fs;
use std::sync::OnceLock;

use anyhow::{Context, Result};
use config::{Config, Environment, File};
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
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            name: "owl-alpha".to_string(),
            provider: "openrouter".to_string(),
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
        CONFIG_CACHE
            .get()
            .expect("Failed to load cached config")
    }

    /// Loads config to the cache and returns it.
    /// If the config file doesn't exist, it creates a default one and then loads it.
    pub fn load() -> Result<Self> {
        let home_dir = std::env::home_dir().unwrap();
        let mia_dir = home_dir.join(".mia");
        let config_file = mia_dir.join("config.toml");

        // Create config file if it doesn't exist
        if !mia_dir.exists() || !config_file.exists() {
            fs::create_dir_all(&mia_dir)
                .context("Failed to create agent home dir")?;
            fs::write(&config_file, toml::to_string(&AppConfig::default())?)
                .context("Failed to create default config file")?;
        }

        // Build config with priority
        // Env > Config file
        let config_builder = Config::builder()
            .add_source(
                File::with_name(config_file.to_str().unwrap())
            )
            .add_source(
                Environment::with_prefix("MIA")
                    .separator("_")
            )
            .build()
            .context("Failed to assemble configuration sources");

        let app_config: AppConfig = config_builder?
            .try_deserialize()?;
        
        let _ = CONFIG_CACHE.set(app_config.clone());

        Ok(app_config)
    }
}
