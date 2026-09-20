use super::*;
use cranpose_services::preferences::{MemoryPreferences, PreferencesError};
#[test]
fn archives_round_trip_and_removal_preserves_player_preferences() {
    let store = MemoryPreferences::default();
    store.set("cranamp.player", "saved player").unwrap();
    let key = save(&store, "../Cat.wsz", &[0, 255, 128, 42]).unwrap();
    assert_eq!(list(&store), vec![("Cat.wsz".into(), key.clone())]);
    assert_eq!(load(&store, &key).unwrap(), [0, 255, 128, 42]);
    save(&store, "Cat.wsz", &[7, 8]).unwrap();
    assert_eq!(list(&store).len(), 1);
    assert_eq!(load(&store, &key).unwrap(), [7, 8]);
    assert!(remove(&store, "cranamp.player").is_err());
    remove(&store, &key).unwrap();
    assert!(list(&store).is_empty());
    assert_eq!(store.get("cranamp.player").as_deref(), Some("saved player"));
    assert!(load(&store, &key).is_err());
}
#[test]
fn corrupt_and_legacy_paths_do_not_decode_as_skins() {
    let store = MemoryPreferences::default();
    store.set("cranamp.skin.v1/broken.wsz", "??").unwrap();
    assert!(load(&store, "cranamp.skin.v1/broken.wsz").is_err());
    assert!(load(&store, "blob:old-session").is_err());
}
struct FullStore(MemoryPreferences);
impl PreferencesStore for FullStore {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key)
    }
    fn keys(&self) -> Vec<String> {
        self.0.keys()
    }
    fn set(&self, _: &str, _: &str) -> Result<(), PreferencesError> {
        Err(PreferencesError::Io("quota exceeded".into()))
    }
    fn remove(&self, key: &str) -> Result<(), PreferencesError> {
        self.0.remove(key)
    }
    fn clear(&self) -> Result<(), PreferencesError> {
        self.0.clear()
    }
}
#[test]
fn failed_overwrite_retains_the_saved_archive() {
    let memory = MemoryPreferences::default();
    let key = save(&memory, "Cat.wsz", &[1, 2, 3]).unwrap();
    let full = FullStore(memory);
    assert!(save(&full, "Cat.wsz", &[4, 5]).is_err());
    assert_eq!(load(&full, &key).unwrap(), [1, 2, 3]);
}
