use super::*;

fn track(title: &str, file: &str) -> Track {
    Track {
        title: title.into(),
        path: Some(file.into()),
        duration_seconds: Some(90.0),
    }
}

fn state() -> WinampState {
    WinampState {
        playlist: Rc::new(vec![
            track("A", "/music/a.mp3"),
            track("B", "/music/b.mp3"),
            track("A", "/music/a.mp3"),
            track("C", "/music/c.mp3"),
        ]),
        current_index: Some(2),
        selected_indices: vec![0, 2],
        selection_anchor: Some(2),
        shuffle_order: vec![3, 2, 1, 0],
        elapsed_seconds: 32.0,
        position: 0.4,
        playback: PlaybackState::Playing,
        ..Default::default()
    }
}

#[test]
fn moving_duplicates_preserves_the_current_occurrence_and_audio_position() {
    let mut s = state();
    assert!(move_track(&mut s, 2, 0));
    assert_eq!(s.current_index, Some(0));
    assert_eq!(s.selected_indices, vec![0, 1]);
    assert_eq!(s.selection_anchor, Some(0));
    assert_eq!(s.shuffle_order, vec![3, 0, 2, 1]);
    assert_eq!(s.elapsed_seconds, 32.0);
    assert_eq!(s.position, 0.4);
    assert_eq!(s.playback, PlaybackState::Playing);
}

#[test]
fn moving_a_different_track_remaps_the_current_track_in_both_directions() {
    let mut s = state();
    move_track(&mut s, 0, 3);
    assert_eq!(s.current_index, Some(1));
    move_track(&mut s, 3, 0);
    assert_eq!(s.current_index, Some(2));
    assert_eq!(s.playlist, state().playlist);
    assert!(!move_track(&mut s, 4, 0));
    assert!(!move_track(&mut s, 0, 4));
}

#[test]
fn play_next_handles_tracks_before_and_after_current_in_shuffle_mode() {
    for from in [0, 1, 3] {
        let mut s = state();
        let queued = s.playlist[from].clone();
        s.shuffle = true;
        play_next(&mut s, from);
        let current = s.current_index.unwrap();
        assert_eq!(s.playlist[current + 1], queued);
        let shuffle_current = s.shuffle_order.iter().position(|i| *i == current).unwrap();
        assert_eq!(s.shuffle_order[shuffle_current + 1], current + 1);
        let mut indices = s.shuffle_order.clone();
        indices.sort_unstable();
        assert_eq!(indices, vec![0, 1, 2, 3]);
        assert_eq!(s.elapsed_seconds, 32.0);
    }
}

#[test]
fn play_next_ignores_current_and_can_queue_without_a_current_track() {
    let mut s = state();
    let before = s.clone();
    play_next(&mut s, 2);
    assert_eq!(s, before);
    s.current_index = None;
    play_next(&mut s, 3);
    assert_eq!(s.playlist[0].title, "C");
    assert_eq!(s.current_index, None);
}

#[test]
fn search_matches_all_words_without_changing_playlist_order() {
    let tracks = Rc::new(vec![
        track("Björk - Jóga", "/music/Homogenic/01.flac"),
        track("Björk - Army of Me", "/music/Post/01.flac"),
    ]);
    let mut cache = FilterCache::default();
    assert_eq!(&*cache.project(&tracks, "BJÖRK HOMOGENIC"), &[0]);
    assert_eq!(&*cache.project(&tracks, "army post"), &[1]);
    assert!(cache.project(&tracks, "missing").is_empty());
    assert_eq!(&*cache.project(&tracks, "  "), &[0, 1]);
    assert_eq!(tracks[0].title, "Björk - Jóga");
}

#[test]
fn playback_ticks_reuse_search_results_and_appended_tracks_refresh_them() {
    let tracks = state().playlist;
    let mut cache = FilterCache::default();
    let first = cache.project(&tracks, "music");
    assert!(Rc::ptr_eq(&first, &cache.project(&tracks, "music")));
    let mut next = tracks.clone();
    Rc::make_mut(&mut next).push(track("D", "/music/d.mp3"));
    let refreshed = cache.project(&next, "music");
    assert_eq!(refreshed.len(), 5);
    assert!(!Rc::ptr_eq(&first, &refreshed));
}

#[test]
fn pending_actions_allow_appends_but_reject_reorder_or_replacement() {
    let original = state().playlist;
    let mut appended = original.as_ref().clone();
    appended.push(track("D", "/music/d.mp3"));
    assert!(unchanged_prefix(&original, &appended));
    appended.swap(0, 1);
    assert!(!unchanged_prefix(&original, &appended));
    assert!(!unchanged_prefix(&original, &original[..2]));
}

#[test]
fn sorting_preserves_the_playing_duplicate_and_the_exact_selection() {
    let mut s = state();
    s.selected_indices = vec![2];
    sort_playlist_tracks_by_field(&mut s, PlaylistSortField::Title);
    assert_eq!(s.current_index, Some(1));
    assert_eq!(s.selected_indices, vec![1]);
    assert_eq!(s.selection_anchor, Some(1));
    assert_eq!(s.elapsed_seconds, 32.0);
}

#[test]
fn summary_does_not_present_partial_durations_as_the_playlist_total() {
    let mut tracks = state().playlist.as_ref().clone();
    assert_eq!(library_summary(&tracks), "4 tracks · 6:00");
    tracks[0].duration_seconds = None;
    assert_eq!(library_summary(&tracks), "4 tracks");
    assert_eq!(library_summary(&[]), "0 tracks");
}

fn network_folder(name: &str) -> NetworkFolder {
    NetworkFolder {
        name: name.into(),
        address: format!("http://drive/{name}/"),
    }
}

#[test]
fn browsing_keeps_the_way_back_and_asks_for_each_folder_again() {
    let mut browse = Browse::default();
    browse.start(network_folder("Taildrive"));
    browse.listing = Listing::Ready(vec![network_folder("server")]);
    let started = browse.generation;

    browse.enter(network_folder("server"));
    browse.enter(network_folder("music"));
    assert_eq!(browse.path(), "Taildrive / server / music");
    assert_eq!(browse.listing, Listing::Loading);
    assert_eq!(browse.generation, started.wrapping_add(2));

    browse.up();
    assert_eq!(browse.path(), "Taildrive / server");
    assert_eq!(browse.trail.last(), Some(&network_folder("server")));

    browse.up();
    let at_root = browse.generation;
    browse.up();
    assert_eq!(browse.path(), "Taildrive");
    assert_eq!(
        browse.generation, at_root,
        "the first folder has nothing above it"
    );
}

#[test]
fn only_subfolders_are_listed_by_name_without_regard_to_case() {
    let folder = |address: &str| {
        cranpose_services::ContentEntry::Folder(
            cranpose_services::WebDavFolder::new(address, cranpose_services::default_http_client())
                .expect("a WebDAV address")
                .handle(),
        )
    };
    let entries = vec![
        folder("http://drive/share/beta/"),
        cranpose_services::ContentEntry::File(cranpose_services::file_content("/music/a.mp3")),
        folder("http://drive/share/Alpha/"),
        folder("http://drive/share/gamma%20ray/"),
    ];
    let listed = subfolders(entries);
    assert_eq!(
        listed
            .iter()
            .map(|folder| (folder.name.as_str(), folder.address.as_str()))
            .collect::<Vec<_>>(),
        [
            ("Alpha", "http://drive/share/Alpha/"),
            ("beta", "http://drive/share/beta/"),
            ("gamma ray", "http://drive/share/gamma%20ray/"),
        ]
    );
}
