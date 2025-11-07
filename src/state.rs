use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    /// Map of podcast name to last-check timestamp
    pub last_checks: HashMap<String, DateTime<Utc>>,
}

impl State {
    /// Get the state file path
    pub fn state_file() -> Result<std::path::PathBuf> {
        Ok(Config::config_dir()?.join("state.json"))
    }

    /// Load state from file
    pub fn load() -> Result<Self> {
        let state_path = Self::state_file()?;

        if !state_path.exists() {
            return Ok(State::default());
        }

        let content = fs::read_to_string(&state_path)
            .context("Failed to read state file")?;

        let state: State = serde_json::from_str(&content)
            .context("Failed to parse state file")?;

        Ok(state)
    }

    /// Save state to file
    pub fn save(&self) -> Result<()> {
        let state_path = Self::state_file()?;

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize state")?;

        fs::write(&state_path, content)
            .context("Failed to write state file")?;

        Ok(())
    }

    /// Get last check time for a podcast
    pub fn get_last_check(&self, podcast_name: &str) -> Option<DateTime<Utc>> {
        self.last_checks.get(podcast_name).copied()
    }

    /// Update last check time for a podcast
    pub fn set_last_check(&mut self, podcast_name: String, time: DateTime<Utc>) {
        self.last_checks.insert(podcast_name, time);
    }
}
