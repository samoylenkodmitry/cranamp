use super::*;
use cranpose_services::preferences::MemoryPreferences;

fn track(name: &str) -> Track {
    Track {
        title: name.into(),
        path: Some(format!("content://music/{name}.flac")),
        duration_seconds: Some(120.),
    }
}
#[test]
fn saved_lists_preserve_order_and_original_uris_across_restart() {
    let store = MemoryPreferences::default();
    let mut data = Collection::default();
    let original = vec![track("First"), track("Second"), track("Third")];
    let id = data.create(" Evening ", original.clone()).unwrap();
    data.move_track(id, 2, 0).unwrap();
    save(&store, &data).unwrap();
    let loaded = load(&store).unwrap();
    assert_eq!(loaded.get(id).unwrap().name, "Evening");
    assert_eq!(
        loaded.get(id).unwrap().tracks,
        [
            original[2].clone(),
            original[0].clone(),
            original[1].clone()
        ]
    );
    assert_eq!(loaded.available, original);
    assert_eq!(store.keys(), [KEY]);
}
#[test]
fn edits_and_deletion_leave_other_lists_and_available_music_unchanged() {
    let mut data = Collection::default();
    let original = vec![track("One"), track("Two")];
    let id = data.create("Morning", original.clone()).unwrap();
    let copy = data.duplicate(id).unwrap();
    assert_eq!(
        data.add(id, &[track("One"), track("Three"), track("Three")])
            .unwrap(),
        1
    );
    data.rename(id, "Evening").unwrap();
    data.get_mut(id).unwrap().tracks.remove(0);
    assert_eq!(data.get(copy).unwrap().tracks, original);
    data.remove(id);
    assert_eq!(data.lists.len(), 1);
    assert_eq!(data.available.len(), 3);
    assert_eq!(data.get(copy).unwrap().name, "Morning copy");
}
#[test]
fn invalid_names_and_stale_track_indices_do_not_change_saved_lists() {
    let mut data = Collection::default();
    let id = data.create("Road", vec![track("One")]).unwrap();
    let before = data.clone();
    assert!(data.create(" road ", vec![]).is_err());
    assert!(data.rename(id, "   ").is_err());
    assert!(data.rename(id, &"a".repeat(81)).is_err());
    assert!(data.move_track(id, 0, 8).is_err());
    assert_eq!(data, before);
}
#[test]
fn corrupt_or_unknown_data_is_kept_instead_of_loading_an_empty_collection() {
    let store = MemoryPreferences::default();
    for text in [
        "broken",
        r#"{"version":2,"next_id":1,"lists":[],"available":[]}"#,
    ] {
        store.set(KEY, text).unwrap();
        assert!(load(&store).is_err());
        assert_eq!(store.get(KEY).as_deref(), Some(text));
    }
}
