use keystroke::application::config_service::ConfigService;
use keystroke::domain::config::{KeystrokeConfig, Position};
use keystroke::infrastructure::settings_repository::SettingsRepository;
use std::fs;
use std::time::Duration;

#[tokio::test]
async fn test_config_service_updates() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");
    let repo = SettingsRepository::with_path(config_path.clone());
    let service = ConfigService::new(repo).unwrap();

    let mut rx = service.subscribe();

    // 1. Update Keystroke Config
    let mut cfg = service.get_config();
    cfg.max_keys = 20;
    cfg.position = Position::TopLeft;
    service.update_config(cfg.clone()).unwrap();

    // Verify broadcast
    rx.changed().await.unwrap();
    let current = rx.borrow().clone();
    assert_eq!(current.max_keys, 20);
    assert_eq!(current.position, Position::TopLeft);

    // 2. Update Bubble Config
    let mut cfg = service.get_config();
    cfg.bubble_timeout_ms = 5000;
    cfg.bubble_position = Position::BottomRight;
    service.update_config(cfg.clone()).unwrap();

    // Verify broadcast
    rx.changed().await.unwrap();
    let current = rx.borrow().clone();
    assert_eq!(current.bubble_timeout_ms, 5000);
    assert_eq!(current.bubble_position, Position::BottomRight);
    
    // 3. Verify persistence
    let content = fs::read_to_string(&config_path).unwrap();
    let persisted: KeystrokeConfig = serde_json::from_str(&content).unwrap();
    assert_eq!(persisted.bubble_timeout_ms, 5000);
}
