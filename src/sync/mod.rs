#![allow(dead_code)]
#[cfg(not(target_arch = "wasm32"))]
mod config;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod runtime;
#[cfg(not(target_arch = "wasm32"))]
pub use runtime::SyncStatus;
use std::collections::HashMap;
pub type UnixSeconds = u64;
pub const SYNC_FILE_EXT: &str = "cransync";
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TrackFingerprint {
    pub basename: String,
    pub duration_s: u32,
}
impl TrackFingerprint {
    pub fn from_path(path: &str, duration_s: Option<f32>) -> Self {
        Self {
            basename: extract_basename(path),
            duration_s: round_duration(duration_s),
        }
    }
    pub fn new(basename: &str, duration_s: u32) -> Self {
        Self {
            basename: normalize(basename),
            duration_s,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.basename.is_empty()
    }
    pub fn matches(&self, other: &TrackFingerprint) -> bool {
        if self.basename != other.basename || self.basename.is_empty() {
            return false;
        }
        self.duration_s == 0 || other.duration_s == 0 || self.duration_s == other.duration_s
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ResumePoint {
    pub fingerprint: TrackFingerprint,
    pub title: String,
    pub position_s: f32,
    pub updated_at: UnixSeconds,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PlayCount {
    pub fingerprint: TrackFingerprint,
    pub title: String,
    pub count: u32,
    pub last_played: UnixSeconds,
}
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
#[derive(Clone, Debug, PartialEq)]
pub struct SyncDocument {
    pub device_id: String,
    pub device_label: String,
    pub platform: String,
    pub updated_at: UnixSeconds,
    pub resume: Option<ResumePoint>,
    pub playlist: Vec<SyncTrack>,
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
    pub fn file_name(&self) -> String {
        format!("{}.{SYNC_FILE_EXT}", sanitize_id(&self.device_id))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceSummary {
    pub device_id: String,
    pub label: String,
    pub platform: String,
    pub last_seen: UnixSeconds,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MergedSync {
    pub resume: Option<ResumePoint>,
    pub playlist: Option<MergedPlaylist>,
    pub counts: HashMap<TrackFingerprint, MergedCount>,
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
pub fn parse_documents(raw: &[(String, String)]) -> Vec<SyncDocument> {
    raw.iter()
        .filter_map(|(_, contents)| parse_document(contents))
        .collect()
}
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
fn round_duration(duration_s: Option<f32>) -> u32 {
    duration_s
        .filter(|d| d.is_finite() && *d > 0.0)
        .map(|d| d.round() as u32)
        .unwrap_or(0)
}
fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}
fn extract_basename(path: &str) -> String {
    let decoded =
        cranpose_services::content::percent_decode(path).unwrap_or_else(|| path.to_string());
    let no_query = decoded.split(['?', '#']).next().unwrap_or(&decoded);
    let trimmed = no_query.trim_end_matches(['/', '\\']);
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    normalize(last)
}
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
    for pair in bytes.as_chunks::<2>().0 {
        output.push(u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?);
    }
    String::from_utf8(output).ok()
}
#[cfg(test)]
#[path = "../../test/unit/sync/tests.rs"]
mod tests;
