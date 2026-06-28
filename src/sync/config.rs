//! Persistent sync configuration: this device's stable identity plus where (and
//! whether) it syncs. Stored next to `player.conf` as `sync.conf`, in the same
//! line-based `key=value` + hex format, so it works identically on every target.
//!
//! The `folder` handle is intentionally opaque: a filesystem path on desktop, an
//! Android SAF tree URI on Android. The platform store factory interprets it.

use super::{hex_decode, hex_encode};

/// Stable per-install identity and sync preferences for this device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncConfig {
    /// Random id generated once per install; names this device's document file.
    pub device_id: String,
    /// Human-friendly name shown in other devices' Settings (user editable).
    pub device_label: String,
    /// Whether cross-device sync is turned on.
    pub enabled: bool,
    /// Opaque handle to the writable sync folder (path / SAF tree URI). `None`
    /// until the user picks one.
    pub folder: Option<String>,
}

impl SyncConfig {
    /// A fresh config for a never-before-synced install: a new random id, a
    /// sensible default label, sync off and unconfigured.
    pub fn fresh() -> Self {
        Self {
            device_id: new_device_id(),
            device_label: default_device_label(),
            enabled: false,
            folder: None,
        }
    }

    /// Sync is only live when explicitly enabled *and* a folder is set.
    pub fn is_active(&self) -> bool {
        self.enabled && self.folder.as_deref().is_some_and(|f| !f.is_empty())
    }
}

const CONFIG_MAGIC: &str = "cranamp-sync-config";

pub fn serialize_config(config: &SyncConfig) -> String {
    let lines = [
        format!("{CONFIG_MAGIC}=1"),
        format!("device_id={}", hex_encode(&config.device_id)),
        format!("device_label={}", hex_encode(&config.device_label)),
        format!("enabled={}", if config.enabled { 1 } else { 0 }),
        format!(
            "folder={}",
            config.folder.as_deref().map(hex_encode).unwrap_or_default()
        ),
    ];
    lines.join("\n") + "\n"
}

/// Parses a `sync.conf`. Returns `None` if the file is missing its magic or has
/// no device id, so a corrupt file falls back to a fresh config rather than a
/// bogus identity.
pub fn parse_config(input: &str) -> Option<SyncConfig> {
    let mut magic_ok = false;
    let mut device_id = String::new();
    let mut device_label = String::new();
    let mut enabled = false;
    let mut folder = None;

    for line in input.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            CONFIG_MAGIC => magic_ok = true,
            "device_id" => device_id = hex_decode(value).unwrap_or_default(),
            "device_label" => device_label = hex_decode(value).unwrap_or_default(),
            "enabled" => enabled = matches!(value, "1" | "true" | "on"),
            "folder" => {
                folder = if value.is_empty() {
                    None
                } else {
                    hex_decode(value).filter(|f| !f.is_empty())
                }
            }
            _ => {}
        }
    }

    if !magic_ok || device_id.is_empty() {
        return None;
    }
    if device_label.is_empty() {
        device_label = default_device_label();
    }
    Some(SyncConfig {
        device_id,
        device_label,
        enabled,
        folder,
    })
}

/// 128 bits of randomness, hex-encoded. Falls back to a time-seeded id if the
/// OS RNG is unavailable (never expected, but sync must not panic over it).
pub fn new_device_id() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::fill(&mut bytes).is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        bytes.copy_from_slice(&nanos.to_le_bytes()[..16]);
    }
    hex_encode_bytes(&bytes)
}

/// Platform tag written into each device document (`desktop`/`android`/`ios`).
pub fn current_platform() -> &'static str {
    #[cfg(target_os = "android")]
    {
        "android"
    }
    #[cfg(target_os = "ios")]
    {
        "ios"
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        "desktop"
    }
}

/// A friendly default name for this device: the host name where we can read it,
/// otherwise the platform tag.
pub fn default_device_label() -> String {
    #[cfg(all(unix, not(target_os = "android"), not(target_os = "ios")))]
    {
        if let Ok(host) = std::fs::read_to_string("/etc/hostname") {
            let host = host.trim();
            if !host.is_empty() {
                return host.to_string();
            }
        }
    }
    match current_platform() {
        "android" => "Android device".to_string(),
        "ios" => "iPhone / iPad".to_string(),
        _ => "Desktop".to_string(),
    }
}

fn hex_encode_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
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
}
