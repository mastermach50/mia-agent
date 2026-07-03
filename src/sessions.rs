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

const SESSION_VERSION: &str = "2";

#[derive(Serialize, Deserialize, Clone)]
pub struct Session {
    pub version: String,
    pub id: String,
    pub title: String,
    pub owner: String,
    pub platform: String,
    pub channel: String,
    pub created: DateTime<Local>,
    pub modified: DateTime<Local>,
    pub history: History,
}

#[derive(Tabled, Deserialize, Clone, Default)]
pub struct PartialSession {
    #[tabled(skip)]
    pub version: String,
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(rename = "Title")]
    pub title: String,
    #[tabled(rename = "Owner")]
    pub owner: String,
    #[tabled(skip)]
    pub platform: String,
    #[tabled(rename = "Channel")]
    pub channel: String,
    #[tabled(skip)]
    pub created: DateTime<Local>,
    #[tabled(rename = "Modified")]
    pub modified: DateTime<Local>,
    // History not included
}

impl Session {
    pub fn new(owner: &str, platform: &str, channel: &str) -> Self {
        let session = Session {
            version: SESSION_VERSION.to_string(),
            id: Uuid::now_v7().to_string(),
            title: String::new(),
            owner: owner.to_string(),
            platform: platform.to_string(),
            channel: channel.to_string(),
            created: Local::now(),
            modified: Local::now(),
            history: History::new(),
        };

        // For internal testing purposes only, length of session id can only be influenced by
        // platform and channel name, which can only be set by developers
        if session.get_extended_session_id().len() > 256 {
            panic!("Session ID is too long");
        }

        session
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

    pub fn load_last_session(owner: &str, platform: &str, channel: &str) -> Result<Self> {
        let last_session = list_sessions(false)?
            .into_iter()
            .filter(|s| s.owner == owner && s.platform == platform &&s.channel == channel)
            .max_by_key(|s| s.id.clone());

        if let Some(session) = last_session {
            Session::load(&session.id)
        } else {
            debug!("No existing session found for ({owner}, {channel})");
            anyhow::bail!("No existing session found for ({owner}, {channel})");
        }
    }

    pub fn get_extended_session_id(&self) -> String {
        // Lengths
        // mia_agent_:_ = 12
        // {platform}   = 104
        // {channel}    = 104
        // {id}         = 36
        // Total        = 256
        format!("mia-agent_{}:{}_{}", self.platform, self.channel, self.id)
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
