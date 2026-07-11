//! Local-network peer streaming (desktop).
//!
//! Built on cranpose 0.1.25's [`peer`](cranpose_services::peer) transport so two
//! cranamp instances on the same LAN can play each other's libraries: one
//! **hosts** the tracks it has loaded (each a file-backed [`ByteSource`]) plus a
//! `manifest`; another **connects**, loads the host's library as ordinary
//! playlist entries, and each track is spooled to a temp file and decoded only
//! when it is actually played.
//!
//! A peer track is carried in the playlist as a `peer|base|token|handle` path
//! (see [`make_peer_ref`] / [`parse_peer_ref`]); [`start_track`](crate::winamp)
//! resolves it on play. No pairing/discovery yet — the address+token are entered
//! manually in the Settings "Network" section.

use cranpose_services::peer::{self, ByteSource, BytesSource, PeerServer, SourceResolver};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// A track the host is willing to serve.
#[derive(Clone)]
struct SharedTrack {
    path: String,
    title: String,
    duration: Option<f32>,
}

/// A file-backed [`ByteSource`] (local desktop files). The file is opened once
/// per request and reused across range chunks.
struct FileByteSource {
    file: Mutex<std::fs::File>,
    len: Option<u64>,
}

impl FileByteSource {
    fn open(path: &str) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let len = file.metadata().ok().map(|meta| meta.len());
        Ok(Self {
            file: Mutex::new(file),
            len,
        })
    }
}

impl ByteSource for FileByteSource {
    fn len(&self) -> Option<u64> {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| std::io::Error::other("file source poisoned"))?;
        file.seek(SeekFrom::Start(offset))?;
        file.read(buf)
    }
}

struct Host {
    server: PeerServer,
    token: String,
    shared: Arc<Mutex<HashMap<String, SharedTrack>>>,
}

static HOST: Mutex<Option<Host>> = Mutex::new(None);

/// Starts the peer host (idempotent). Returns the `ip:port` to share and the
/// access token.
pub fn start_host() -> Result<(String, String), String> {
    let mut guard = HOST.lock().map_err(|_| "peer host poisoned".to_string())?;
    if let Some(host) = guard.as_ref() {
        return Ok((display_addr(host.server.local_addr()), host.token.clone()));
    }

    let shared: Arc<Mutex<HashMap<String, SharedTrack>>> = Arc::new(Mutex::new(HashMap::new()));
    let token = new_token();

    let resolver_shared = shared.clone();
    let resolver: SourceResolver = Arc::new(move |handle: &str| -> Option<Arc<dyn ByteSource>> {
        let map = resolver_shared.lock().ok()?;
        if handle == "manifest" {
            return Some(Arc::new(BytesSource::new(manifest_text(&map).into_bytes())));
        }
        let track = map.get(handle)?;
        FileByteSource::open(&track.path)
            .ok()
            .map(|source| Arc::new(source) as Arc<dyn ByteSource>)
    });

    let server =
        PeerServer::start("0.0.0.0:0", token.clone(), resolver).map_err(|error| error.to_string())?;
    let addr = display_addr(server.local_addr());
    *guard = Some(Host {
        server,
        token: token.clone(),
        shared,
    });
    Ok((addr, token))
}

/// Stops the host (Drop closes the server).
pub fn stop_host() {
    if let Ok(mut guard) = HOST.lock() {
        *guard = None;
    }
}

/// The host's `ip:port` + token, if running.
pub fn host_info() -> Option<(String, String)> {
    let guard = HOST.lock().ok()?;
    let host = guard.as_ref()?;
    Some((display_addr(host.server.local_addr()), host.token.clone()))
}

/// Replaces the set of tracks the host shares with `(path, title, duration)`
/// tuples (local-file tracks only). No-op when the host is off.
pub fn set_shared_tracks(tracks: &[(String, String, Option<f32>)]) {
    let Ok(guard) = HOST.lock() else { return };
    let Some(host) = guard.as_ref() else { return };
    let Ok(mut map) = host.shared.lock() else {
        return;
    };
    map.clear();
    for (index, (path, title, duration)) in tracks.iter().enumerate() {
        map.insert(
            index.to_string(),
            SharedTrack {
                path: path.clone(),
                title: title.clone(),
                duration: *duration,
            },
        );
    }
}

/// One entry from a peer's manifest.
pub struct PeerEntry {
    pub handle: String,
    pub title: String,
    pub duration: Option<f32>,
}

/// Fetches a peer's manifest (the list of tracks it is sharing).
pub fn fetch_manifest(base: &str, token: &str) -> Result<Vec<PeerEntry>, String> {
    let result = peer::fetch_range(base, token, "manifest", 0, None).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&result.bytes);
    Ok(parse_manifest(&text))
}

/// Spools one shared handle from a peer to a temp file, returning its path. Runs
/// on a background thread (network I/O).
pub fn spool_peer_handle(base: &str, token: &str, handle: &str) -> Result<String, String> {
    let dir = peer_cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let spool = dir.join(format!("{}-{}.dat", sanitize(base), sanitize(handle)));
    let mut file = std::fs::File::create(&spool).map_err(|e| e.to_string())?;
    peer::fetch_to_writer(base, token, handle, 0, None, &mut file).map_err(|e| e.to_string())?;
    Ok(spool.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Peer track references — carried as a playlist `Track::path`.
// ---------------------------------------------------------------------------

/// A reference to a track served by a peer, parsed from a playlist path.
pub struct PeerRef {
    pub base: String,
    pub token: String,
    pub handle: String,
}

/// Encodes a peer track reference into an opaque `Track::path` string.
pub fn make_peer_ref(base: &str, token: &str, handle: &str) -> String {
    // base is `ip:port` and token is hex, neither contains '|'; the handle is
    // percent-encoded so it cannot either.
    format!("peer|{base}|{token}|{}", percent_encode(handle))
}

/// Parses a `peer|…` path back into a [`PeerRef`], or `None` if it is a normal
/// local/`content://` path.
pub fn parse_peer_ref(path: &str) -> Option<PeerRef> {
    let rest = path.strip_prefix("peer|")?;
    let mut parts = rest.splitn(3, '|');
    let base = parts.next()?.to_string();
    let token = parts.next()?.to_string();
    let handle = percent_decode(parts.next()?);
    Some(PeerRef {
        base,
        token,
        handle,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn manifest_text(map: &HashMap<String, SharedTrack>) -> String {
    let mut entries: Vec<(&String, &SharedTrack)> = map.iter().collect();
    entries.sort_by_key(|(handle, _)| handle.parse::<usize>().unwrap_or(usize::MAX));
    entries
        .iter()
        .map(|(handle, track)| {
            let title = track.title.replace(['\t', '\n', '\r'], " ");
            let duration = track
                .duration
                .filter(|d| *d > 0.0)
                .map(|d| format!("{d:.3}"))
                .unwrap_or_default();
            format!("{handle}\t{title}\t{duration}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_manifest(text: &str) -> Vec<PeerEntry> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let handle = parts.next()?.to_string();
            if handle.is_empty() {
                return None;
            }
            let title = parts.next().unwrap_or("").to_string();
            let duration = parts
                .next()
                .and_then(|d| d.parse::<f32>().ok())
                .filter(|d| *d > 0.0);
            Some(PeerEntry {
                handle,
                title,
                duration,
            })
        })
        .collect()
}

fn peer_cache_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("cranamp-peer-cache")
}

fn new_token() -> String {
    let mut bytes = [0u8; 12];
    let _ = getrandom::fill(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The LAN address a peer should connect to: the routable local IP + the bound
/// port (the server binds `0.0.0.0`, which is not itself connectable).
fn display_addr(server_addr: SocketAddr) -> String {
    let ip = lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    format!("{ip}:{}", server_addr.port())
}

fn lan_ip() -> Option<String> {
    // Connecting a UDP socket picks the source IP the OS would route from,
    // without sending anything — the usual zero-dependency "what's my LAN IP".
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(byte: u8) -> Option<u8> {
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

    #[test]
    fn manifest_round_trips_with_duration() {
        let mut map = HashMap::new();
        map.insert(
            "0".to_string(),
            SharedTrack {
                path: "/a.mp3".into(),
                title: "First\tSong".into(),
                duration: Some(241.5),
            },
        );
        map.insert(
            "1".to_string(),
            SharedTrack {
                path: "/b.mp3".into(),
                title: "Second".into(),
                duration: None,
            },
        );
        let entries = parse_manifest(&manifest_text(&map));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].handle, "0");
        assert_eq!(entries[0].title, "First Song"); // tab sanitized
        assert_eq!(entries[0].duration, Some(241.5));
        assert_eq!(entries[1].duration, None);
    }

    #[test]
    fn peer_ref_round_trips() {
        let path = make_peer_ref("192.168.1.5:9000", "abc123", "a|b/c.mp3");
        let parsed = parse_peer_ref(&path).expect("parse");
        assert_eq!(parsed.base, "192.168.1.5:9000");
        assert_eq!(parsed.token, "abc123");
        assert_eq!(parsed.handle, "a|b/c.mp3");
        assert!(parse_peer_ref("/local/file.mp3").is_none());
    }

    #[test]
    fn host_serves_manifest_and_streams_a_handle() {
        let dir = peer_cache_dir().join("selftest");
        std::fs::create_dir_all(&dir).unwrap();
        let track = dir.join("tone.dat");
        let data: Vec<u8> = (0..3000u32).map(|i| i as u8).collect();
        std::fs::write(&track, &data).unwrap();

        let (addr, token) = start_host().expect("start");
        set_shared_tracks(&[(
            track.to_string_lossy().into_owned(),
            "Tone".to_string(),
            Some(3.0),
        )]);
        let port = addr.rsplit(':').next().unwrap();
        let base = format!("127.0.0.1:{port}");

        let manifest = fetch_manifest(&base, &token).expect("manifest");
        assert_eq!(manifest.len(), 1);
        assert_eq!(manifest[0].title, "Tone");
        assert_eq!(manifest[0].duration, Some(3.0));

        let spool = spool_peer_handle(&base, &token, &manifest[0].handle).expect("spool");
        assert_eq!(std::fs::read(&spool).unwrap(), data);

        stop_host();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
