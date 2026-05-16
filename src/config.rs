use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub base_dir: Option<PathBuf>,
    pub podcasts: Vec<PodcastConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PodcastConfig {
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<PathBuf>,
    #[serde(default)]
    pub max_episodes: Option<usize>,
    #[serde(default)]
    pub paused: bool,
}

impl PodcastConfig {
    /// Resolve the directory this podcast should be downloaded into.
    ///
    /// Uses the per-podcast `output_dir` if set, otherwise falls back to
    /// `<base_dir>/<sanitized name>`. Errors if neither is available.
    pub fn resolved_output_dir(&self, base_dir: Option<&Path>) -> Result<PathBuf> {
        if let Some(dir) = &self.output_dir {
            return Ok(dir.clone());
        }
        let base = base_dir.ok_or_else(|| {
            anyhow::anyhow!(
                "No output directory for podcast '{}': set `base_dir` at the top of config.toml, \
                 or give this podcast its own `output_dir`.",
                self.name
            )
        })?;
        Ok(base.join(sanitize_dir_name(&self.name)))
    }
}

/// Sanitize a podcast name into a safe directory component.
fn sanitize_dir_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect();
    sanitized.trim().to_string()
}

impl Config {
    /// Get the config directory path (~/.config/podcast-getter)
    pub fn config_dir() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("podcast-getter");
        Ok(config_dir)
    }

    /// Get the config file path
    pub fn config_file() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load config from file
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file()?;

        if !config_path.exists() {
            anyhow::bail!(
                "Config file not found at {}. Please create it first.",
                config_path.display()
            );
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;

        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = config_dir.join("config.toml");

        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        fs::write(&config_path, content)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Create example config
    pub fn create_example() -> Result<()> {
        let config_dir = Self::config_dir()?;
        let example_path = config_dir.join("config.toml.example");

        fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;

        let example_config = Config {
            base_dir: Some(
                dirs::download_dir()
                    .context("Could not determine download directory")?
                    .join("podcasts"),
            ),
            podcasts: vec![PodcastConfig {
                name: "Example Podcast".to_string(),
                url: "https://example.com/feed.xml".to_string(),
                output_dir: None,
                max_episodes: None,
                paused: false,
            }],
        };

        let content = toml::to_string_pretty(&example_config)
            .context("Failed to serialize example config")?;

        fs::write(&example_path, content)
            .context("Failed to write example config")?;

        println!("Created example config at {}", example_path.display());

        Ok(())
    }
}
