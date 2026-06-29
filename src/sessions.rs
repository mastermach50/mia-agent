use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::BufReader;
use tabled::Tabled;
use uuid::Uuid;

use crate::api::History;
use crate::config::AppConfig;

const SESSION_VERSION: &str = "1";

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub id: String,
    pub version: String,
    pub title: String,
    pub created: DateTime<Local>,
    pub modified: DateTime<Local>,
    pub owner: String,
    pub channel: String,
    pub history: History,
}

#[derive(Tabled, Deserialize, Clone, Default)]
pub struct PartialSession {
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(skip)]
    pub version: String,
    #[tabled(rename = "Title")]
    pub title: String,
    #[tabled(skip)]
    pub created: DateTime<Local>,
    #[tabled(rename = "Modified")]
    pub modified: DateTime<Local>,
    #[tabled(rename = "Owner")]
    pub owner: String,
    #[tabled(rename = "Channel")]
    pub channel: String,
    // History not included
}

impl Session {
    pub fn new(owner: &str, channel: &str) -> Self {
        Session {
            id: Uuid::now_v7().to_string(),
            version: SESSION_VERSION.to_string(),
            title: String::new(),
            created: Local::now(),
            modified: Local::now(),
            owner: owner.to_string(),
            channel: channel.to_string(),
            history: History::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let filename = self.id.clone() + ".json";
        let session_dir = AppConfig::internal().sessions_dir.clone();
        let filepath = session_dir.join(&filename);

        fs::write(
            &filepath,
            serde_json::to_string_pretty(self).context("Failed to serialize session")?,
        )
        .context("Failed to save session file")
    }

    pub fn load(id: &str) -> Result<Self> {
        let session_file = AppConfig::internal()
            .sessions_dir
            .join(id.to_owned() + ".json");
        let session: Session = serde_json::from_str(
            &fs::read_to_string(session_file).context("Failed to read session file")?,
        )
        .context("Failed to deserialize session")?;

        Ok(session)
    }

    pub fn load_last_session(owner: &str, channel: &str) -> Result<Self> {
        let last_session = list_sessions(false)?
            .into_iter()
            .filter(|s| s.owner == owner && s.channel == channel)
            .max_by_key(|s| s.id.clone());

        if let Some(session) = last_session {
            Session::load(&session.id)
        } else {
            debug!("No existing session found for ({owner}, {channel})");
            anyhow::bail!("No existing session found for ({owner}, {channel})");
        }
    }
}

pub fn list_sessions(keep_invalid: bool) -> Result<Vec<PartialSession>> {
    let sessions_dir = AppConfig::internal().sessions_dir.clone();
    let mut valid_sessions = Vec::new();

    for file in sessions_dir.read_dir()? {
        if let Ok(file) = file {
            if let Ok(session) = serde_json::from_reader::<BufReader<File>, PartialSession>(
                File::open_buffered(file.path())?,
            ) && session.version == SESSION_VERSION
            {
                valid_sessions.push(session);
            } else if keep_invalid {
                let mut invalid_session = PartialSession::default();
                invalid_session.id =
                    format!("INVALID SESSION: {}", file.file_name().to_string_lossy());
                valid_sessions.push(invalid_session);
            }
        }
    }

    Ok(valid_sessions)
}
