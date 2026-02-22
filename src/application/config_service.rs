use crate::domain::config::{KeystrokeConfig, Validate};
use crate::infrastructure::settings_repository::SettingsRepository;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Clone)]
pub struct ConfigService {
    tx: Arc<watch::Sender<KeystrokeConfig>>,
    rx: watch::Receiver<KeystrokeConfig>,
    repository: Arc<SettingsRepository>,
}

impl ConfigService {
    /// Create a new `ConfigService`.
    ///
    /// # Errors
    ///
    /// Returns an error if the initial configuration cannot be loaded.
    pub fn new(repository: SettingsRepository) -> Result<Self> {
        let config = repository.load()?;
        let (tx, rx) = watch::channel(config);
        Ok(Self {
            tx: Arc::new(tx),
            rx,
            repository: Arc::new(repository),
        })
    }

    #[must_use]
    pub fn get_config(&self) -> KeystrokeConfig {
        self.rx.borrow().clone()
    }

    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<KeystrokeConfig> {
        self.rx.clone()
    }

    /// Update the configuration and notify subscribers.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be saved to the repository.
    pub fn update_config(&self, mut config: KeystrokeConfig) -> Result<()> {
        config.validate();
        self.repository.save(&config)?;
        self.tx.send_replace(config);
        Ok(())
    }
}
