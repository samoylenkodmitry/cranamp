//! Cross-device sync for Cranamp.
//!
//! Each device writes exactly one small document into a shared, writable "sync
//! folder" (a folder the user picks once per device, e.g. a `cranamp-sync/`
//! directory on the same Tailnet/WebDAV that already holds their music). The
//! file is named `<device-id>.cransync` and is overwritten in place, so the
//! folder never accumulates per-session junk — it holds at most one file per
//! device. Reading merges every peer document.
//!
//! The music library itself is never touched: on Android it is a read-only SAF
//! tree, on iOS a read-only security-scoped URL. Sync state lives only in the
//! dedicated writable folder, and every write degrades gracefully — a failed or
//! read-only folder simply drops sync to receive-only and the player keeps
//! running. Storage is cranpose's cross-platform `WritableFolderStore`.
//!
//! ## Cross-device identity
//!
//! The same file is a filesystem path on desktop but a `content://` URI on
//! Android, so raw paths never match across devices. Instead every track is
//! fingerprinted by `(basename, duration)` — the decoded file name plus rounded
//! duration in seconds — which is stable for the same file across platforms.
//! Resume points and play counts key off this fingerprint and therefore merge
//! cleanly even between an Android phone and a Linux desktop.

#![allow(dead_code)]

// Storage is provided by cranpose's cross-platform `WritableFolderStore`
// (desktop std::fs / Android SAF). The runtime consumes it directly.
#[cfg(not(target_arch = "wasm32"))]
mod config;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod runtime;

#[cfg(not(target_arch = "wasm32"))]
pub use runtime::SyncStatus;

use std::collections::HashMap;

/// Seconds since the Unix epoch. 0 means "unknown / never".
pub type UnixSeconds = u64;

/// File extension for a per-device sync document. Distinct from any media
/// extension so a sync folder shared with other files stays unambiguous.
pub const SYNC_FILE_EXT: &str = "cransync";

/// Cross-platform identity for a single track.
///
/// Built from the file basename and rounded duration, both normalized, so the
/// same physical file fingerprints identically whether it was reached as a
/// local path or an Android `content://` URI. The display title is carried
/// alongside (see [`ResumePoint`]/[`PlayCount`]) but is intentionally *not* part
/// of the key, since one device may read an ID3 title while another falls back
/// to the file name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TrackFingerprint {
    /// Lower-cased, percent-decoded file name with no directory component.
    pub basename: String,
    /// Duration rounded to whole seconds; 0 when unknown.
    pub duration_s: u32,
}

impl TrackFingerprint {
    /// Builds a fingerprint from a raw path/URI and an optional duration.
    pub fn from_path(path: &str, duration_s: Option<f32>) -> Self {
        Self {
            basename: extract_basename(path),
            duration_s: round_duration(duration_s),
        }
    }

    /// Builds a fingerprint from an already-extracted basename.
    pub fn new(basename: &str, duration_s: u32) -> Self {
        Self {
            basename: normalize(basename),
            duration_s,
        }
    }

    /// A fingerprint carries no useful identity if it has no basename.
    pub fn is_empty(&self) -> bool {
        self.basename.is_empty()
    }

    /// Whether two fingerprints refer to the same track, tolerating an unknown
    /// (0) duration on either side. Used to match a synced resume point against
    /// a locally loaded playlist where durations may not be known yet.
    pub fn matches(&self, other: &TrackFingerprint) -> bool {
        if self.basename != other.basename || self.basename.is_empty() {
            return false;
        }
        self.duration_s == 0 || other.duration_s == 0 || self.duration_s == other.duration_s
    }
}

/// "Where I left off" on one device.
#[derive(Clone, Debug, PartialEq)]
pub struct ResumePoint {
    pub fingerprint: TrackFingerprint,
    /// Display title (original case), for surfacing in the UI.
    pub title: String,
    /// Elapsed playback position in seconds.
    pub position_s: f32,
    /// When this resume point was recorded.
    pub updated_at: UnixSeconds,
}

/// Aggregate play count for one track on one device. Merged counts sum these
/// across devices, so the per-device value is only ever this device's own plays.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayCount {
    pub fingerprint: TrackFingerprint,
    pub title: String,
    pub count: u32,
    pub last_played: UnixSeconds,
}

/// One playlist entry, as written by a device. `path` is that device's own
/// path/URI and may not resolve elsewhere; the receiving device falls back to
/// fingerprint matching (see [`SyncTrack::fingerprint`]).
#[derive(Clone, Debug, PartialEq)]
pub struct SyncTrack {
    pub title: String,
    pub path: String,
    pub duration_s: Option<f32>,
}

impl SyncTrack {
    pub fn fingerprint(&self) -> TrackFingerprint {
        TrackFingerprint::from_path(&self.path, self.duration_s)
    }
}

/// The complete document a single device publishes into the sync folder.
#[derive(Clone, Debug, PartialEq)]
pub struct SyncDocument {
    pub device_id: String,
    pub device_label: String,
    pub platform: String,
    /// When the document as a whole was last written.
    pub updated_at: UnixSeconds,
    pub resume: Option<ResumePoint>,
    pub playlist: Vec<SyncTrack>,
    /// When the playlist last changed (drives last-writer-wins playlist merge).
    pub playlist_updated_at: UnixSeconds,
    pub counts: Vec<PlayCount>,
}

impl SyncDocument {
    pub fn new(device_id: String, device_label: String, platform: String) -> Self {
        Self {
            device_id,
            device_label,
            platform,
            updated_at: 0,
            resume: None,
            playlist: Vec::new(),
            playlist_updated_at: 0,
            counts: Vec::new(),
        }
    }

    /// File name this document is stored under (`<device-id>.cransync`).
    pub fn file_name(&self) -> String {
        format!("{}.{SYNC_FILE_EXT}", sanitize_id(&self.device_id))
    }
}

/// A peer device as surfaced in the Settings "Sync" device list.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceSummary {
    pub device_id: String,
    pub label: String,
    pub platform: String,
    pub last_seen: UnixSeconds,
}

/// The merged view across every device document in the folder.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MergedSync {
    /// Newest resume point across all devices.
    pub resume: Option<ResumePoint>,
    /// The most recently updated non-empty playlist, with its source device id.
    pub playlist: Option<MergedPlaylist>,
    /// Summed play counts keyed by fingerprint.
    pub counts: HashMap<TrackFingerprint, MergedCount>,
    /// One entry per device document seen.
    pub devices: Vec<DeviceSummary>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergedPlaylist {
    pub source_device_id: String,
    pub tracks: Vec<SyncTrack>,
    pub updated_at: UnixSeconds,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergedCount {
    pub title: String,
    pub count: u32,
    pub last_played: UnixSeconds,
}

/// Conflict-free merge of every device document.
///
/// * resume — the single newest point wins (last write across devices).
/// * playlist — last-writer-wins: the most recently changed non-empty playlist.
/// * counts — summed per fingerprint (each device counts only its own plays),
///   keeping the latest `last_played` and a representative title.
///
/// All three rules are commutative and need no coordination, so two devices can
/// write independently and any reader converges on the same view.
pub fn merge(docs: &[SyncDocument]) -> MergedSync {
    let mut merged = MergedSync::default();

    for doc in docs {
        merged.devices.push(DeviceSummary {
            device_id: doc.device_id.clone(),
            label: doc.device_label.clone(),
            platform: doc.platform.clone(),
            last_seen: doc.updated_at,
        });

        if let Some(resume) = &doc.resume {
            let newer = merged
                .resume
                .as_ref()
                .map(|current| resume.updated_at > current.updated_at)
                .unwrap_or(true);
            if newer {
                merged.resume = Some(resume.clone());
            }
        }

        if !doc.playlist.is_empty() {
            let newer = merged
                .playlist
                .as_ref()
                .map(|current| doc.playlist_updated_at > current.updated_at)
                .unwrap_or(true);
            if newer {
                merged.playlist = Some(MergedPlaylist {
                    source_device_id: doc.device_id.clone(),
                    tracks: doc.playlist.clone(),
                    updated_at: doc.playlist_updated_at,
                });
            }
        }

        for play in &doc.counts {
            let entry = merged
                .counts
                .entry(play.fingerprint.clone())
                .or_insert_with(|| MergedCount {
                    title: play.title.clone(),
                    count: 0,
                    last_played: 0,
                });
            entry.count = entry.count.saturating_add(play.count);
            if play.last_played >= entry.last_played {
                entry.last_played = play.last_played;
                if !play.title.is_empty() {
                    entry.title = play.title.clone();
                }
            }
        }
    }

    merged
        .devices
        .sort_by_key(|device| std::cmp::Reverse(device.last_seen));
    merged
}

/// Parses every `(file_name, contents)` pair into documents, skipping any that
/// fail to parse.
pub fn parse_documents(raw: &[(String, String)]) -> Vec<SyncDocument> {
    raw.iter()
        .filter_map(|(_, contents)| parse_document(contents))
        .collect()
}

// ---------------------------------------------------------------------------
// Serialization (line-based key=value, hex-encoded strings — no serde, matching
// the existing player.conf format so it works identically on every target).
// ---------------------------------------------------------------------------

const DOC_MAGIC: &str = "cranamp-sync";
const DOC_VERSION: u32 = 1;

pub fn serialize_document(doc: &SyncDocument) -> String {
    let mut lines = vec![
        format!("{DOC_MAGIC}={DOC_VERSION}"),
        format!("device_id={}", hex_encode(&doc.device_id)),
        format!("device_label={}", hex_encode(&doc.device_label)),
        format!("platform={}", hex_encode(&doc.platform)),
        format!("updated_at={}", doc.updated_at),
        format!("playlist_updated_at={}", doc.playlist_updated_at),
    ];

    if let Some(resume) = &doc.resume {
        lines.push(format!(
            "resume={}\t{}\t{}\t{:.3}\t{}",
            hex_encode(&resume.fingerprint.basename),
            resume.fingerprint.duration_s,
            hex_encode(&resume.title),
            resume.position_s.max(0.0),
            resume.updated_at,
        ));
    }

    for track in &doc.playlist {
        let duration = track
            .duration_s
            .filter(|duration| *duration > 0.0)
            .map(|duration| format!("{duration:.3}"))
            .unwrap_or_default();
        lines.push(format!(
            "track={}\t{}\t{}",
            hex_encode(&track.title),
            hex_encode(&track.path),
            duration,
        ));
    }

    for play in &doc.counts {
        lines.push(format!(
            "count={}\t{}\t{}\t{}\t{}",
            hex_encode(&play.fingerprint.basename),
            play.fingerprint.duration_s,
            hex_encode(&play.title),
            play.count,
            play.last_played,
        ));
    }

    lines.join("\n") + "\n"
}

pub fn parse_document(input: &str) -> Option<SyncDocument> {
    let mut magic_ok = false;
    let mut doc = SyncDocument::new(String::new(), String::new(), String::new());

    for line in input.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            DOC_MAGIC => magic_ok = true,
            "device_id" => doc.device_id = hex_decode(value).unwrap_or_default(),
            "device_label" => doc.device_label = hex_decode(value).unwrap_or_default(),
            "platform" => doc.platform = hex_decode(value).unwrap_or_default(),
            "updated_at" => doc.updated_at = value.parse().unwrap_or(0),
            "playlist_updated_at" => doc.playlist_updated_at = value.parse().unwrap_or(0),
            "resume" => doc.resume = parse_resume(value),
            "track" => {
                if let Some(track) = parse_track(value) {
                    doc.playlist.push(track);
                }
            }
            "count" => {
                if let Some(play) = parse_count(value) {
                    doc.counts.push(play);
                }
            }
            _ => {}
        }
    }

    if !magic_ok || doc.device_id.is_empty() {
        return None;
    }
    Some(doc)
}

fn parse_resume(value: &str) -> Option<ResumePoint> {
    let mut parts = value.split('\t');
    let basename = hex_decode(parts.next()?)?;
    let duration_s = parts.next()?.parse::<u32>().ok()?;
    let title = hex_decode(parts.next()?)?;
    let position_s = parts.next()?.parse::<f32>().ok()?.max(0.0);
    let updated_at = parts.next()?.parse::<u64>().ok()?;
    Some(ResumePoint {
        fingerprint: TrackFingerprint::new(&basename, duration_s),
        title,
        position_s,
        updated_at,
    })
}

fn parse_track(value: &str) -> Option<SyncTrack> {
    let (title, rest) = value.split_once('\t')?;
    let (path, duration) = rest.split_once('\t').unwrap_or((rest, ""));
    Some(SyncTrack {
        title: hex_decode(title)?,
        path: hex_decode(path)?,
        duration_s: duration.parse::<f32>().ok().filter(|d| *d > 0.0),
    })
}

fn parse_count(value: &str) -> Option<PlayCount> {
    let mut parts = value.split('\t');
    let basename = hex_decode(parts.next()?)?;
    let duration_s = parts.next()?.parse::<u32>().ok()?;
    let title = hex_decode(parts.next()?)?;
    let count = parts.next()?.parse::<u32>().ok()?;
    let last_played = parts.next()?.parse::<u64>().ok()?;
    Some(PlayCount {
        fingerprint: TrackFingerprint::new(&basename, duration_s),
        title,
        count,
        last_played,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn round_duration(duration_s: Option<f32>) -> u32 {
    duration_s
        .filter(|d| d.is_finite() && *d > 0.0)
        .map(|d| d.round() as u32)
        .unwrap_or(0)
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Extracts a normalized file basename from a filesystem path or a `content://`
/// URI. URIs encode `/` as `%2F` and may carry a query string, so we
/// percent-decode, strip any query, then take the trailing path segment.
fn extract_basename(path: &str) -> String {
    let decoded = percent_decode(path);
    let no_query = decoded.split(['?', '#']).next().unwrap_or(&decoded);
    let trimmed = no_query.trim_end_matches(['/', '\\']);
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    normalize(last)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push((high << 4) | low);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Keeps a device id safe as a file-name stem: alphanumerics, dash, underscore.
fn sanitize_id(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "device".to_string()
    } else {
        cleaned
    }
}

fn hex_encode(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(input.len() * 2);
    for byte in input.as_bytes() {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        output.push((hex_value(pair[0])? << 4) | hex_value(pair[1])?);
    }
    String::from_utf8(output).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(basename: &str, duration: u32) -> TrackFingerprint {
        TrackFingerprint::new(basename, duration)
    }

    #[test]
    fn fingerprint_matches_path_and_content_uri_for_same_file() {
        let desktop =
            TrackFingerprint::from_path("/home/s/Music/Artist/Cool Song.mp3", Some(241.4));
        let android = TrackFingerprint::from_path(
            "content://com.roundsync/tree/abc/document/music%2FArtist%2FCool%20Song.mp3",
            Some(241.0),
        );
        assert_eq!(desktop.basename, "cool song.mp3");
        assert_eq!(android.basename, "cool song.mp3");
        assert!(desktop.matches(&android));
        // Exact equality also holds once durations round to the same integer.
        assert_eq!(desktop, android);
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
        assert!(parse_document("cranamp-sync=1\n").is_none()); // no device id
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
        // Devices sorted newest-seen first.
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
}
