use super::*;
use std::io::{Read, Seek, SeekFrom};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test/fixtures/library")
        .join(name)
}
fn track(name: &str, path: &str) -> Track {
    Track {
        title: name.into(),
        path: Some(path.into()),
        duration_seconds: None,
    }
}
fn reset() {
    RUNTIME.with(|slot| *slot.borrow_mut() = Some(Rc::new(RefCell::new(Runtime::new(None)))));
}
fn index_track(track: &Track, artwork: Option<&str>, album: Option<&str>) {
    let uri = track.path.clone().unwrap_or_default();
    runtime().borrow_mut().index.insert_track(
        uri.clone(),
        Record {
            folder: parent(&uri),
            artwork: artwork.map(str::to_owned),
            album: album.map(str::to_owned),
            ..Default::default()
        },
    );
}
fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(640, 480, image::Rgba([12, 34, 56, 255]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}
fn temp(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "cranamp-index-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn embedded_tags_are_read_across_supported_containers() {
    for name in ["tagged.flac", "tagged.mp3", "tagged.m4a", "tagged.opus"] {
        let (record, _) =
            tags::read(Box::new(std::fs::File::open(fixture(name)).unwrap()), name).unwrap();
        assert_eq!(record.title.as_deref(), Some("Tagged title"), "{name}");
        assert_eq!(record.artist.as_deref(), Some("Tagged artist"), "{name}");
        assert_eq!(record.album.as_deref(), Some("Tagged album"), "{name}");
        assert_eq!(
            record.album_artist.as_deref(),
            Some("Album artist"),
            "{name}"
        );
        assert_eq!(record.track_number, Some(3), "{name}");
        assert_eq!(record.disc_number, Some(2), "{name}");
    }
}
#[test]
fn partial_tags_override_only_fields_they_supply_and_disc_folders_form_one_album() {
    reset();
    let tracks = vec![
        track(
            "02 - Filename Artist - Filename title",
            "/Artist/Folder album/Disc 1/02.flac",
        ),
        track(
            "01 - Filename Artist - Other",
            "/Artist/Folder album/Disc 2/01.flac",
        ),
        track("01 - Someone - Song", "/Other/Folder album/01.flac"),
    ];
    runtime().borrow_mut().index.insert_track(
        tracks[0].path.clone().unwrap(),
        Record {
            title: Some("Embedded title".into()),
            artist: Some("Embedded artist".into()),
            folder: parent(tracks[0].path.as_deref().unwrap()),
            ..Default::default()
        },
    );
    assert_eq!(title(&tracks[0]), "Embedded title");
    assert_eq!(artist(&tracks[0]), "Embedded artist");
    assert_eq!(record(&tracks[0]).album.as_deref(), Some("Folder album"));
    assert_eq!(title(&tracks[1]), "Other");
    let albums = albums(&tracks, &[0, 1, 2]);
    assert_eq!(albums.len(), 2);
    assert!(
        albums.iter().any(|album| album.tracks == [0, 1]),
        "Untagged disc folders must sort before track numbers"
    );
    assert_eq!(record(&tracks[0]).disc_number, Some(1));
    assert_eq!(record(&tracks[1]).disc_number, Some(2));
    assert_eq!(
        parent("file:///Artist/Folder%20album/01.flac"),
        "/Artist/Folder album"
    );
    assert_eq!(
        parent(
            "content://provider/tree/primary%3AMusic/document/primary%3AMusic%2FAlbum%2F01.flac"
        ),
        "content://provider/tree/primary:Music/document/primary:Music/Album"
    );
}
#[test]
fn embedded_artwork_wins_and_late_folder_covers_apply_without_mutating_tracks() {
    reset();
    let track = track("01 - Song", "/Artist/Album/song.flac");
    let original = track.clone();
    assert!(record(&track).artwork.is_none());
    runtime().borrow_mut().index.covers.insert(
        "/Artist/Album".into(),
        Cover {
            thumbnail: "folder".into(),
            ..Default::default()
        },
    );
    assert_eq!(record(&track).artwork.as_deref(), Some("folder"));
    runtime().borrow_mut().index.insert_track(
        track.path.clone().unwrap(),
        Record {
            artwork: Some("embedded".into()),
            album: Some("Tagged album".into()),
            folder: "/Artist/Album".into(),
            ..Default::default()
        },
    );
    assert_eq!(record(&track).artwork.as_deref(), Some("embedded"));
    assert_eq!(record(&track).album.as_deref(), Some("Tagged album"));
    assert_eq!(track, original);
}
#[test]
fn tracks_inherit_late_album_artwork_across_discs_and_keep_their_own_picture() {
    reset();
    let tracks = [
        track("Song", "/Artist/Album/Disc 1/01.flac"),
        track("Other song", "/Artist/Album/Disc 2/02.flac"),
        track("Separate release", "/Other artist/Album/01.flac"),
        track("Different album tag", "/Artist/Album/Disc 1/03.flac"),
    ];
    index_track(&tracks[0], None, None);
    index_track(&tracks[3], None, Some("Another album"));
    assert!(record(&tracks[0]).artwork.is_none());

    index_track(&tracks[1], Some("album-picture"), Some("ALBUM"));
    assert_eq!(record(&tracks[0]).artwork.as_deref(), Some("album-picture"));
    assert!(record(&tracks[2]).artwork.is_none());
    assert!(record(&tracks[3]).artwork.is_none());
    let albums = albums(&tracks, &[0, 1, 2, 3]);
    let album = albums.iter().find(|album| album.tracks == [0, 1]).unwrap();
    assert_eq!(
        record(&tracks[album.cover_track]).artwork,
        record(&tracks[0]).artwork
    );
    assert!(
        runtime().borrow().index.tracks[tracks[0].path.as_ref().unwrap()]
            .artwork
            .is_none(),
        "Inherited images must not replace source metadata"
    );

    runtime().borrow_mut().index.covers.insert(
        "/Artist/Album".into(),
        Cover {
            thumbnail: "folder-picture".into(),
            ..Default::default()
        },
    );
    assert_eq!(
        record(&tracks[0]).artwork.as_deref(),
        Some("folder-picture")
    );
    assert_eq!(record(&tracks[1]).artwork.as_deref(), Some("album-picture"));
    index_track(&tracks[0], Some("own-picture"), None);
    assert_eq!(record(&tracks[0]).artwork.as_deref(), Some("own-picture"));
}
#[test]
fn album_artwork_is_stable_after_restart_and_follows_metadata_changes() {
    reset();
    let root = temp("album-artwork");
    let first = track("First", "/Music/Album/01.flac");
    let second = track("Second", "/Music/Album/02.flac");
    let missing = track("Missing picture", "/Music/Album/03.flac");
    index_track(&second, Some("second-picture"), None);
    index_track(&first, Some("first-picture"), None);
    assert_eq!(record(&missing).artwork.as_deref(), Some("first-picture"));
    let bytes = {
        let rt = runtime();
        let mut data = rt.borrow_mut();
        data.index.version = 1;
        serde_json::to_vec(&data.index).unwrap()
    };
    assert!(!serde_json::from_slice::<serde_json::Value>(&bytes)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("album_artwork"));
    atomic_write(&root.join("index.json"), &bytes).unwrap();
    reset();
    runtime().borrow_mut().index = load_index(Some(&root));
    assert_eq!(record(&missing).artwork.as_deref(), Some("first-picture"));

    index_track(&first, Some("changed-picture"), Some("Different album"));
    assert_eq!(record(&missing).artwork.as_deref(), Some("second-picture"));
    index_track(&second, None, None);
    assert!(record(&missing).artwork.is_none());
    index_track(&second, Some("replacement-picture"), None);
    assert_eq!(
        record(&missing).artwork.as_deref(),
        Some("replacement-picture")
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn folder_art_selection_is_deterministic_and_thumbnail_cache_survives_restart() {
    let root = temp("cache");
    let mut names = [
        "back.JPG",
        "random.jpeg",
        "Folder.png",
        "COVER.JPG",
        "front.jpg",
    ];
    names.sort_by_key(|name| image_rank(name));
    assert_eq!(names[0], "COVER.JPG");
    assert!(image_rank("song.mp3").is_none());
    assert!(thumbnail(b"broken picture").is_err());
    let bytes = thumbnail(&png()).unwrap();
    let image = image::load_from_memory(&bytes).unwrap();
    assert_eq!((image.width(), image.height()), (320, 240));
    let key = store_thumbnail(Some(&root), &bytes).unwrap();
    assert_eq!(store_thumbnail(Some(&root), &bytes).unwrap(), key);
    assert!(thumbnail_exists(Some(&root), &key));
    assert!(!thumbnail_exists(Some(&root), "../../secret"));
    let mut index = Index {
        version: 1,
        ..Default::default()
    };
    index.tracks.insert(
        "original://song".into(),
        Record {
            title: Some("Saved title".into()),
            artwork: Some(key),
            ..Default::default()
        },
    );
    atomic_write(
        &root.join("index.json"),
        &serde_json::to_vec(&index).unwrap(),
    )
    .unwrap();
    assert_eq!(
        load_index(Some(&root)).tracks["original://song"]
            .title
            .as_deref(),
        Some("Saved title")
    );
    assert_eq!(
        std::fs::read_dir(&root).unwrap().count(),
        2,
        "Only index and one deduplicated thumbnail are saved"
    );
    std::fs::write(root.join("index.json"), b"interrupted/corrupt index").unwrap();
    assert!(load_index(Some(&root)).tracks.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn embedded_flac_picture_is_decoded_and_reduced_to_a_thumbnail() {
    let mut data = std::fs::read(fixture("tagged.flac")).unwrap();
    let mut at = 4;
    loop {
        let last = data[at] & 0x80 != 0;
        let len = ((data[at + 1] as usize) << 16)
            | ((data[at + 2] as usize) << 8)
            | data[at + 3] as usize;
        if last {
            data[at] &= 0x7f;
            at += len + 4;
            break;
        }
        at += len + 4;
    }
    let image = png();
    let mut picture = Vec::new();
    for value in [3u32, 9] {
        picture.extend_from_slice(&value.to_be_bytes());
    }
    picture.extend_from_slice(b"image/png");
    for value in [0u32, 640, 480, 32, 0, image.len() as u32] {
        picture.extend_from_slice(&value.to_be_bytes());
    }
    picture.extend(image);
    let size = picture.len() as u32;
    let mut block = vec![0x86, (size >> 16) as u8, (size >> 8) as u8, size as u8];
    block.extend(picture);
    data.splice(at..at, block);
    let (_, image) = tags::read(Box::new(std::io::Cursor::new(data)), "with-art.flac").unwrap();
    let image = image::load_from_memory(&image.unwrap()).unwrap().to_rgba8();
    assert_eq!((image.width(), image.height()), (320, 240));
    assert_eq!(&image.as_raw()[..4], &[12, 34, 56, 255]);
}
#[test]
fn seeking_to_tail_metadata_does_not_read_or_copy_the_audio_payload() {
    let root = temp("seek");
    let original = std::fs::read(fixture("tagged.m4a")).unwrap();
    let moov = original
        .windows(4)
        .position(|part| part == b"moov")
        .unwrap()
        - 4;
    let path = root.join("large.m4a");
    let mut file = std::fs::File::create(&path).unwrap();
    use std::io::Write;
    file.write_all(&original[..moov]).unwrap();
    let gap = 32 * 1024 * 1024u32;
    file.write_all(&gap.to_be_bytes()).unwrap();
    file.write_all(b"free").unwrap();
    file.seek(SeekFrom::Current((gap - 8) as i64)).unwrap();
    file.write_all(&original[moov..]).unwrap();
    drop(file);
    let before = std::fs::metadata(&path).unwrap();
    let source = source::open(path.to_str().unwrap()).unwrap();
    let (record, _) = tags::read(Box::new(source), "large.m4a").unwrap();
    assert_eq!(record.album.as_deref(), Some("Tagged album"));
    assert_eq!(std::fs::metadata(&path).unwrap().len(), before.len());
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    let mut bounded = source::BoundedSource::new(Box::new(std::io::Cursor::new(vec![
        0;
        source::READ_BUDGET
            + 1
    ])));
    assert!(bounded.read_to_end(&mut Vec::new()).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn network_metadata_uses_small_validated_ranges_and_rejects_wrong_offsets() {
    use std::io::Write;
    use std::{
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
    };
    let original = std::fs::read(fixture("tagged.m4a")).unwrap();
    let moov = original
        .windows(4)
        .position(|part| part == b"moov")
        .unwrap()
        - 4;
    let gap = 16 * 1024 * 1024u32;
    let mut bytes = original[..moov].to_vec();
    bytes.extend_from_slice(&gap.to_be_bytes());
    bytes.extend_from_slice(b"free");
    bytes.resize(moov + gap as usize, 0);
    bytes.extend_from_slice(&original[moov..]);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/album.m4a", listener.local_addr().unwrap());
    let stop = Arc::new(AtomicBool::new(false));
    let sent = Arc::new(AtomicUsize::new(0));
    let worker_stop = stop.clone();
    let worker_sent = sent.clone();
    let worker = std::thread::spawn(move || {
        while !worker_stop.load(Ordering::Relaxed) {
            let Ok((mut stream, _)) = listener.accept() else {
                std::thread::sleep(std::time::Duration::from_millis(2));
                continue;
            };
            // Accepted sockets inherit nonblocking mode on macOS.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut request = Vec::new();
            let mut one = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") && request.len() < 8192 {
                if stream.read(&mut one).ok() != Some(1) {
                    break;
                }
                request.push(one[0]);
            }
            let request = String::from_utf8_lossy(&request).to_lowercase();
            let Some((start, end)) = request
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(key, _)| *key == "range")
                        .and_then(|(_, value)| value.trim().strip_prefix("bytes="))
                })
                .and_then(|range| range.split_once('-'))
                .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?)))
            else {
                continue;
            };
            let end = end.min(bytes.len() - 1);
            let reported = if request.starts_with("get /wrong") {
                start + 1
            } else {
                start
            };
            let header = format!("HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {reported}-{end}/{}\r\nConnection: close\r\n\r\n", end - start + 1, bytes.len());
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&bytes[start..=end]);
            worker_sent.fetch_add(end - start + 1, Ordering::Relaxed);
        }
    });
    let result = tags::read(Box::new(source::open(&url).unwrap()), "album.m4a");
    let wrong = source::open(&url.replace("album.m4a", "wrong.m4a"));
    stop.store(true, Ordering::Relaxed);
    worker.join().unwrap();
    assert_eq!(result.unwrap().0.album.as_deref(), Some("Tagged album"));
    assert!(
        wrong.is_err(),
        "Incorrect range offsets must never be treated as audio bytes"
    );
    assert!(
        sent.load(Ordering::Relaxed) < 1024 * 1024,
        "Reading tags downloaded too much of the audio"
    );
}
