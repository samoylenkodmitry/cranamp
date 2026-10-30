//! Named playlists contain references to original audio, never imported audio copies.
use crate::audio::Track;
use cranpose_services::preferences::PreferencesStore;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const KEY: &str = "cranamp.playlists.v1";
const MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Playlist {
    pub id: u64,
    pub name: String,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(crate) struct Collection {
    version: u32,
    next_id: u64,
    pub lists: Vec<Playlist>,
    // Retain track references when playback switches to a saved list.
    pub available: Vec<Track>,
}
impl Default for Collection {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            lists: Vec::new(),
            available: Vec::new(),
        }
    }
}
pub(crate) fn track_key(track: &Track) -> &str {
    track.path.as_deref().unwrap_or(&track.title)
}
impl Collection {
    pub fn get(&self, id: u64) -> Option<&Playlist> {
        self.lists.iter().find(|list| list.id == id)
    }
    pub fn get_mut(&mut self, id: u64) -> Result<&mut Playlist, String> {
        self.lists
            .iter_mut()
            .find(|list| list.id == id)
            .ok_or_else(|| "This playlist is no longer available.".into())
    }
    fn name(&self, name: &str, except: Option<u64>) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a playlist name.".into());
        }
        if name.chars().count() > 80 {
            return Err("Use a name of 80 characters or fewer.".into());
        }
        if self
            .lists
            .iter()
            .any(|list| Some(list.id) != except && list.name.to_lowercase() == name.to_lowercase())
        {
            return Err("A playlist already has this name. Choose another name.".into());
        }
        Ok(name.into())
    }
    pub fn remember_tracks(&mut self, tracks: &[Track]) -> bool {
        let mut known: HashSet<String> = self
            .available
            .iter()
            .map(|track| track_key(track).into())
            .collect();
        let before = self.available.len();
        self.available.extend(
            tracks
                .iter()
                .filter(|track| known.insert(track_key(track).into()))
                .cloned(),
        );
        self.available.len() != before
    }
    pub fn create(&mut self, name: &str, tracks: Vec<Track>) -> Result<u64, String> {
        let name = self.name(name, None)?;
        let id = self.next_id;
        self.next_id = id.checked_add(1).ok_or("Playlist limit reached.")?;
        self.remember_tracks(&tracks);
        self.lists.push(Playlist { id, name, tracks });
        Ok(id)
    }
    pub fn rename(&mut self, id: u64, name: &str) -> Result<(), String> {
        let name = self.name(name, Some(id))?;
        self.get_mut(id)?.name = name;
        Ok(())
    }
    pub fn duplicate(&mut self, id: u64) -> Result<u64, String> {
        let original = self
            .get(id)
            .ok_or("This playlist is no longer available.")?;
        let base: String = original.name.chars().take(65).collect();
        let tracks = original.tracks.clone();
        let mut name = format!("{base} copy");
        for suffix in 2.. {
            if self.name(&name, None).is_ok() {
                break;
            }
            name = format!("{base} copy {suffix}");
        }
        self.create(&name, tracks)
    }
    pub fn add(&mut self, id: u64, tracks: &[Track]) -> Result<usize, String> {
        let list = self.get_mut(id)?;
        let mut known: HashSet<String> = list
            .tracks
            .iter()
            .map(|track| track_key(track).into())
            .collect();
        let before = list.tracks.len();
        list.tracks.extend(
            tracks
                .iter()
                .filter(|track| known.insert(track_key(track).into()))
                .cloned(),
        );
        let added = list.tracks.len() - before;
        self.remember_tracks(tracks);
        Ok(added)
    }
    pub fn remove(&mut self, id: u64) {
        self.lists.retain(|list| list.id != id);
    }
    pub fn move_track(&mut self, id: u64, from: usize, to: usize) -> Result<(), String> {
        let list = self.get_mut(id)?;
        if from >= list.tracks.len() || to >= list.tracks.len() {
            return Err("Choose the track again.".into());
        }
        let track = list.tracks.remove(from);
        list.tracks.insert(to, track);
        Ok(())
    }
}
pub(crate) fn load(store: &dyn PreferencesStore) -> Result<Collection, String> {
    let Some(text) = store.get(KEY) else {
        return Ok(Collection::default());
    };
    if text.len() > MAX_BYTES {
        return Err("Saved playlists exceed the storage limit.".into());
    }
    let collection: Collection = serde_json::from_str(&text)
        .map_err(|_| "Saved playlists could not be read. Your saved data has been kept.")?;
    let mut ids = HashSet::new();
    if collection.version != 1
        || collection
            .lists
            .iter()
            .any(|list| list.id == 0 || list.id >= collection.next_id || !ids.insert(list.id))
    {
        return Err(
            "Saved playlists use an unsupported format. Your saved data has been kept.".into(),
        );
    }
    Ok(collection)
}
pub(crate) fn save(store: &dyn PreferencesStore, collection: &Collection) -> Result<(), String> {
    let text = serde_json::to_string(collection).map_err(|error| error.to_string())?;
    if text.len() > MAX_BYTES {
        return Err("Playlists exceed the storage limit.".into());
    }
    store.set(KEY, &text).map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../../test/unit/playlists.rs"]
mod tests;
