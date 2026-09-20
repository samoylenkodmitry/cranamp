use super::{hex_decode, hex_encode};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncConfig {
    pub device_id: String,
    pub device_label: String,
    pub enabled: bool,
    pub folder: Option<String>,
}
impl SyncConfig {
    pub fn fresh() -> Self {
        Self {
            device_id: new_device_id(),
            device_label: default_device_label(),
            enabled: false,
            folder: None,
        }
    }
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
#[path = "../../test/unit/sync/config/tests.rs"]
mod tests;
