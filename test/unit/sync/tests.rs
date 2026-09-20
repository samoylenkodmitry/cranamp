use super::*;
fn fp(basename: &str, duration: u32) -> TrackFingerprint {
    TrackFingerprint::new(basename, duration)
}
#[test]
fn fingerprint_matches_path_and_content_uri_for_same_file() {
    let desktop = TrackFingerprint::from_path("/home/s/Music/Artist/Cool Song.mp3", Some(241.4));
    let android = TrackFingerprint::from_path(
        "content://com.roundsync/tree/abc/document/music%2FArtist%2FCool%20Song.mp3",
        Some(241.0),
    );
    assert_eq!(desktop.basename, "cool song.mp3");
    assert_eq!(android.basename, "cool song.mp3");
    assert!(desktop.matches(&android));
    assert_eq!(desktop, android);
}
#[test]
fn fingerprint_keeps_a_malformed_uri_undecoded_instead_of_substituting() {
    let a = TrackFingerprint::from_path("content://x/doc/track%FFone.mp3", None);
    let b = TrackFingerprint::from_path("content://x/doc/track%FFtwo.mp3", None);
    assert_eq!(a.basename, "track%ffone.mp3");
    assert_eq!(b.basename, "track%fftwo.mp3");
    assert_ne!(a.basename, b.basename);
}
#[test]
fn fingerprint_tolerates_unknown_duration() {
    let known = fp("song.mp3", 200);
    let unknown = fp("song.mp3", 0);
    assert!(known.matches(&unknown));
    assert!(unknown.matches(&known));
    assert!(!known.matches(&fp("other.mp3", 0)));
}
#[test]
fn document_round_trips_through_serialization() {
    let mut doc = SyncDocument::new(
        "dev-1".to_string(),
        "Living Room PC".to_string(),
        "desktop".to_string(),
    );
    doc.updated_at = 1_719_500_000;
    doc.playlist_updated_at = 1_719_499_000;
    doc.resume = Some(ResumePoint {
        fingerprint: fp("cool song.mp3", 241),
        title: "Cool Song".to_string(),
        position_s: 87.5,
        updated_at: 1_719_500_000,
    });
    doc.playlist = vec![
        SyncTrack {
            title: "Cool Song".to_string(),
            path: "/home/s/Music/Cool Song.mp3".to_string(),
            duration_s: Some(241.0),
        },
        SyncTrack {
            title: "Tab\tInside".to_string(),
            path: "content://x/doc/a%2Fb.flac".to_string(),
            duration_s: None,
        },
    ];
    doc.counts = vec![PlayCount {
        fingerprint: fp("cool song.mp3", 241),
        title: "Cool Song".to_string(),
        count: 12,
        last_played: 1_719_400_000,
    }];
    let text = serialize_document(&doc);
    let parsed = parse_document(&text).expect("parse");
    assert_eq!(parsed, doc);
}
#[test]
fn parse_rejects_foreign_or_empty_documents() {
    assert!(parse_document("just some text\n").is_none());
    assert!(parse_document("cranamp-sync=1\n").is_none());
}
#[test]
fn merge_picks_newest_resume() {
    let mut a = SyncDocument::new("a".into(), "A".into(), "desktop".into());
    a.updated_at = 100;
    a.resume = Some(ResumePoint {
        fingerprint: fp("a.mp3", 100),
        title: "A".into(),
        position_s: 10.0,
        updated_at: 100,
    });
    let mut b = SyncDocument::new("b".into(), "B".into(), "android".into());
    b.updated_at = 200;
    b.resume = Some(ResumePoint {
        fingerprint: fp("b.mp3", 200),
        title: "B".into(),
        position_s: 20.0,
        updated_at: 200,
    });
    let merged = merge(&[a, b]);
    let resume = merged.resume.expect("resume");
    assert_eq!(resume.fingerprint.basename, "b.mp3");
    assert_eq!(merged.devices.len(), 2);
    assert_eq!(merged.devices[0].device_id, "b");
}
#[test]
fn merge_sums_play_counts_across_devices() {
    let mut a = SyncDocument::new("a".into(), "A".into(), "desktop".into());
    a.counts = vec![PlayCount {
        fingerprint: fp("song.mp3", 100),
        title: "Song".into(),
        count: 3,
        last_played: 50,
    }];
    let mut b = SyncDocument::new("b".into(), "B".into(), "android".into());
    b.counts = vec![PlayCount {
        fingerprint: fp("song.mp3", 100),
        title: "Song".into(),
        count: 4,
        last_played: 80,
    }];
    let merged = merge(&[a, b]);
    let entry = merged.counts.get(&fp("song.mp3", 100)).expect("count");
    assert_eq!(entry.count, 7);
    assert_eq!(entry.last_played, 80);
}
#[test]
fn merge_playlist_is_last_writer_wins() {
    let mut a = SyncDocument::new("a".into(), "A".into(), "desktop".into());
    a.playlist = vec![SyncTrack {
        title: "Old".into(),
        path: "/a.mp3".into(),
        duration_s: Some(10.0),
    }];
    a.playlist_updated_at = 100;
    let mut b = SyncDocument::new("b".into(), "B".into(), "android".into());
    b.playlist = vec![SyncTrack {
        title: "New".into(),
        path: "/b.mp3".into(),
        duration_s: Some(20.0),
    }];
    b.playlist_updated_at = 300;
    let merged = merge(&[a, b]);
    let playlist = merged.playlist.expect("playlist");
    assert_eq!(playlist.source_device_id, "b");
    assert_eq!(playlist.tracks[0].title, "New");
}
#[test]
fn file_name_is_one_per_device_and_sanitized() {
    let doc = SyncDocument::new("dev/../x 1".into(), "L".into(), "desktop".into());
    assert_eq!(doc.file_name(), "dev____x_1.cransync");
}
