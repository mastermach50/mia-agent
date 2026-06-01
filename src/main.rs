use anyhow::{Ok, Result};

mod config;
use config::AppConfig;

fn main() -> Result<()>{
    
    AppConfig::load()?;
    AppConfig::global(); // Access global config to ensure it's loaded
    
    println!("model: {}", AppConfig::global().model.name);
    println!("provider: {}", AppConfig::global().model.provider);
    
    Ok(())
}
