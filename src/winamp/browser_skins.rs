use base64::{engine::general_purpose::STANDARD, Engine};
use cranpose_services::preferences::PreferencesStore;
const PREFIX: &str = "cranamp.skin.v1/";
pub(super) fn list(store: &dyn PreferencesStore) -> Vec<(String, String)> {
    store
        .keys()
        .into_iter()
        .filter_map(|key| {
            key.strip_prefix(PREFIX)
                .map(|name| (name.to_string(), key.clone()))
        })
        .collect()
}
pub(super) fn save(
    store: &dyn PreferencesStore,
    name: &str,
    bytes: &[u8],
) -> Result<String, String> {
    let key = format!("{PREFIX}{}", super::sanitize_skin_file_name(name));
    store
        .set(&key, &STANDARD.encode(bytes))
        .map_err(|e| e.to_string())?;
    Ok(key)
}
pub(super) fn load(store: &dyn PreferencesStore, key: &str) -> Result<Vec<u8>, String> {
    if !key.starts_with(PREFIX) {
        return Err("This skin is not in the browser library. Add it again.".into());
    }
    let encoded = store.get(key).ok_or("Saved skin is no longer available")?;
    STANDARD
        .decode(encoded)
        .map_err(|_| "Saved skin data is damaged".into())
}
pub(super) fn remove(store: &dyn PreferencesStore, key: &str) -> Result<(), String> {
    if !key.starts_with(PREFIX) {
        return Err("This is not a browser-library skin".into());
    }
    store.remove(key).map_err(|e| e.to_string())
}
#[cfg(test)]
#[path = "../../test/unit/winamp/browser_skins/tests.rs"]
mod tests;
