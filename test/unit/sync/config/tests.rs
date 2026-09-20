use super::*;
#[test]
fn config_round_trips() {
    let config = SyncConfig {
        device_id: "abc123".to_string(),
        device_label: "Living Room PC".to_string(),
        enabled: true,
        folder: Some("/mnt/tailnet/cranamp-sync".to_string()),
    };
    let parsed = parse_config(&serialize_config(&config)).expect("parse");
    assert_eq!(parsed, config);
}
#[test]
fn empty_folder_round_trips_as_none() {
    let config = SyncConfig {
        device_id: "abc123".to_string(),
        device_label: "Phone".to_string(),
        enabled: false,
        folder: None,
    };
    let parsed = parse_config(&serialize_config(&config)).expect("parse");
    assert_eq!(parsed.folder, None);
    assert!(!parsed.is_active());
}
#[test]
fn is_active_requires_enabled_and_folder() {
    let mut config = SyncConfig::fresh();
    assert!(!config.is_active());
    config.enabled = true;
    assert!(!config.is_active());
    config.folder = Some("/x".to_string());
    assert!(config.is_active());
    config.folder = Some(String::new());
    assert!(!config.is_active());
}
#[test]
fn new_device_ids_are_unique_and_hex() {
    let a = new_device_id();
    let b = new_device_id();
    assert_eq!(a.len(), 32);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, b);
}
#[test]
fn parse_rejects_missing_magic() {
    assert!(parse_config("device_id=abc\n").is_none());
}
