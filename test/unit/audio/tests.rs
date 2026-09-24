use super::{
    compute_analyzer_bands, equalizer_value_gain_db, has_uri_scheme, media_item, mixed_down, Track,
};
#[test]
fn extensions_include_common_winamp_formats() {
    let extensions = super::supported_audio_extensions();
    assert!(extensions.contains(&"mp3"));
    assert!(extensions.contains(&"flac"));
    assert!(extensions.contains(&"m4a"));
    assert!(extensions.contains(&"mp4"));
    assert!(extensions.contains(&"ogg"));
    assert!(extensions.contains(&"wav"));
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn demo_playlist_uses_external_mp3_files() {
    let tracks = super::demo_playlist_tracks();
    assert_eq!(tracks.len(), 5);
    assert!(tracks.iter().all(|track| {
        track
            .path
            .as_deref()
            .map(|path| path.ends_with(".mp3"))
            .unwrap_or(false)
    }));
    assert_eq!(
        tracks
            .iter()
            .map(|track| track.duration_seconds)
            .collect::<Vec<_>>(),
        vec![
            Some(100.0),
            Some(90.0),
            Some(120.0),
            Some(100.0),
            Some(90.0)
        ]
    );
}
#[test]
fn analyzer_bands_follow_sample_energy() {
    let sample_rate = 44_100;
    let samples = (0..2048)
        .map(|sample| {
            let phase = (sample as f32 * 440.0 * std::f32::consts::TAU) / sample_rate as f32;
            phase.sin() * 0.8
        })
        .collect::<Vec<_>>();
    let bands = compute_analyzer_bands(&samples, sample_rate);
    assert!(bands.iter().any(|band| *band > 0.2));
}
#[test]
fn a_source_that_names_a_scheme_is_a_uri_and_the_rest_is_a_path() {
    assert!(has_uri_scheme("blob:https://example.test/9f2c"));
    assert!(has_uri_scheme("content://media/external/audio/media/42"));
    assert!(has_uri_scheme("file:///music/track.mp3"));
    assert!(!has_uri_scheme("/music/track.mp3"));
    assert!(!has_uri_scheme("C:\\Music\\track.mp3"));
    assert!(!has_uri_scheme("relative/track.mp3"));
}
#[test]
fn a_blob_backed_track_keeps_its_url_and_a_path_becomes_a_file_uri() {
    let blob = Track {
        title: "Web".to_string(),
        path: Some("blob:https://example.test/9f2c".to_string()),
        duration_seconds: None,
    };
    assert_eq!(
        media_item(&blob)
            .expect("a blob-backed track has a source")
            .uri,
        "blob:https://example.test/9f2c"
    );
    let local = Track {
        title: "Local".to_string(),
        path: Some("/music/track.mp3".to_string()),
        duration_seconds: None,
    };
    assert_eq!(
        media_item(&local).expect("a local track has a source").uri,
        if cfg!(target_arch = "wasm32") {
            "/music/track.mp3"
        } else {
            "file:///music/track.mp3"
        }
    );
    let missing = Track {
        title: "Nothing".to_string(),
        path: None,
        duration_seconds: None,
    };
    assert!(media_item(&missing).is_none());
}
#[test]
fn browser_relative_media_stays_relative_to_the_document() {
    let path = "demo-music/cranamp-demo-01-retro-tracker.mp3";
    let track = super::track_from_title_path("Demo", path);
    let uri = media_item(&track).expect("demo track source").uri;
    if cfg!(target_arch = "wasm32") {
        assert_eq!(uri, path);
    } else {
        assert!(uri.starts_with("file://"));
        assert!(uri.ends_with(path));
    }
}
#[test]
fn a_centred_equalizer_slider_is_flat_and_the_ends_are_symmetric() {
    assert_eq!(equalizer_value_gain_db(0.5), 0.0);
    assert_eq!(equalizer_value_gain_db(1.0), 12.0);
    assert_eq!(equalizer_value_gain_db(0.0), -12.0);
    assert_eq!(equalizer_value_gain_db(2.0), 12.0);
    assert_eq!(equalizer_value_gain_db(-1.0), -12.0);
}

#[test]
fn the_oscilloscope_hears_every_channel_at_once() {
    let stereo = [1.0, -1.0, 0.5, 0.5, 0.25, -0.75];
    assert_eq!(mixed_down(&stereo, 2, 576), vec![0.0, 0.5, -0.25]);
    assert_eq!(
        mixed_down(&stereo, 2, 2),
        vec![0.0, 0.5],
        "no more than asked for"
    );
    assert_eq!(
        mixed_down(&[0.5, 0.25], 0, 576),
        vec![0.5, 0.25],
        "no channels reads as one"
    );
}
