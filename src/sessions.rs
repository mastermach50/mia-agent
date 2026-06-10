use anyhow::Result;
use log::{debug, info};
use std::fs;

use crate::api::History;
use crate::config::AppConfig;

pub fn save_session(filename: &str, history: &History) -> Result<()>{
    debug!("Saving history to file");
    let history_file = AppConfig::internal().sessions_dir.join(filename);
    fs::write(history_file, serde_json::to_string_pretty(history).unwrap())?;
    Ok(())
}

pub fn load_session(filename: &str) -> Result<History> {
    debug!("Loading history from file");
    let history_file = AppConfig::internal().mia_dir.join("sessions").join(filename);
    if history_file.exists() {
        let history = fs::read_to_string(history_file)?;
        return Ok(serde_json::from_str(&history)?)
    } else {
        info!("History file not found");
        anyhow::bail!("History file not found");
    }
}