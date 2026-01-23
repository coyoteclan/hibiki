use crate::domain::config::KeystrokeConfig;
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tracing::{debug, info};

pub struct SettingsRepository {
    config_path: PathBuf,
}

impl SettingsRepository {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;
        let config_path = config_dir.join("keystroke").join("config.json");
        Ok(Self { config_path })
    }

    pub fn with_path(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    pub fn load(&self) -> Result<KeystrokeConfig> {
        if self.config_path.exists() {
            let content = fs::read_to_string(&self.config_path)
                .with_context(|| format!("Failed to read config: {:?}", self.config_path))?;
            let config: KeystrokeConfig =
                serde_json::from_str(&content).with_context(|| "Failed to parse config file")?;
            info!("Loaded configuration from {:?}", self.config_path);
            Ok(config)
        } else {
            debug!("No config file found, using defaults");
            Ok(KeystrokeConfig::default())
        }
    }

    pub fn save(&self, config: &KeystrokeConfig) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(config).context("Failed to serialize config")?;

        let parent = self.config_path.parent().unwrap_or(&self.config_path);
        let mut temp_file = NamedTempFile::new_in(parent)
            .with_context(|| format!("Failed to create temp file in {:?}", parent))?;

        temp_file
            .write_all(content.as_bytes())
            .context("Failed to write to temp config file")?;

        temp_file
            .persist(&self.config_path)
            .with_context(|| format!("Failed to persist config to {:?}", self.config_path))?;

        info!("Saved configuration to {:?}", self.config_path);
        Ok(())
    }
}
