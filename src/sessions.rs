use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use itertools::Itertools;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::api::History;
use crate::config::AppConfig;

#[derive(Serialize, Deserialize)]
pub struct Session {
    pub name: String,
    pub created: DateTime<Local>,
    pub owner: String,
    pub filename: String,
    pub history: History,
}

/// Get list of all the session files (filenames not full paths)
fn get_session_list() -> Vec<String> {
    let dir = AppConfig::internal().sessions_dir.clone();
    if dir.exists()
        && let Ok(entries) = fs::read_dir(&dir)
    {
        let mut items: Vec<String> = Vec::new();
        for entry in entries {
            if let Ok(item) = entry {
                items.push(item.file_name().to_string_lossy().to_string());
            }
        }
        items
    } else {
        fs::create_dir_all(&dir).unwrap();
        info!("Created sessions directory");
        Vec::new()
    }
}

/// Create a new session and return it
pub fn create_new_session(kind: &str, owner: &str) -> Result<Session> {
    let time = chrono::Local::now();
    let time_string = time.format("%Y%m%d_%H%M%S").to_string();

    let session_dir = AppConfig::internal().sessions_dir.clone();
    if !session_dir.exists() {
        fs::create_dir_all(&session_dir).context("Failed to create sessions directory")?;
        info!("Created sessions directory");
    }
    let filename = format!("{kind}_{owner}_{time_string}.json");
    let filepath = session_dir.join(&filename);

    let new_session = Session {
        name: String::new(),
        created: time,
        owner: owner.to_string(),
        filename: filename,
        history: History::new(),
    };

    fs::write(
        filepath,
        serde_json::to_string_pretty(&new_session).context("Failed to serialize session")?,
    )
    .context("Failed to write session to file")?;
    debug!("Created new session: {}", new_session.filename);

    Ok(new_session)
}

/// Get the last session of the given owner and return it, otherwise return None
pub fn get_last_session(kind: &str, owner: &str) -> Result<Option<Session>> {
    let sessions = get_session_list();
    let wanted_session = sessions
        .iter()
        .filter(|&s| s.starts_with(&format!("{kind}_{owner}_")))
        .sorted()
        .last()
        .map(|s| s.to_string());

    if wanted_session.is_none() {
        return Ok(None)
    }

    let sessions_dir = AppConfig::internal().sessions_dir.clone();
    let filename = sessions_dir.join(&wanted_session.unwrap());

    let last_session: Session =
        serde_json::from_str(&fs::read_to_string(filename).expect("Failed to read session file"))
            .expect("Failed to deserialize session file");
    debug!("Loaded last session: {}", last_session.filename);

    Ok(Some(last_session))
}

/// Save an existing session
pub fn save_session(session: &Session) -> Result<()> {
    let filename = &session.filename;
    let filepath = AppConfig::internal().sessions_dir.join(&filename);

    fs::write(
        filepath,
        serde_json::to_string_pretty(&session).context("Failed to serialize session")?,
    )
    .context("Failed to write session file")?;
    debug!("Saved session: {}", filename);

    Ok(())
}
