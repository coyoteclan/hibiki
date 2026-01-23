use crate::domain::config::KeystrokeConfig;
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
    pub fn new(repository: SettingsRepository) -> Result<Self> {
        let config = repository.load()?;
        let (tx, rx) = watch::channel(config);
        Ok(Self {
            tx: Arc::new(tx),
            rx,
            repository: Arc::new(repository),
        })
    }

    pub fn get_config(&self) -> KeystrokeConfig {
        self.rx.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<KeystrokeConfig> {
        self.rx.clone()
    }

    pub fn update_config(&self, config: KeystrokeConfig) -> Result<()> {
        self.repository.save(&config)?;
        self.tx.send_replace(config);
        Ok(())
    }
}
