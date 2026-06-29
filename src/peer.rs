//! Local-network peer streaming (desktop proof).
//!
//! Wraps cranpose's [`peer`](cranpose_services::peer) transport so two cranamp
//! instances on the same LAN can stream each other's libraries: one **hosts**
//! the tracks it has loaded (each as a [`FileByteSource`]) plus a tiny
//! `manifest`, and another **consumes** by spooling a track to a temp file and
//! playing it. This is the Phase-1 data-channel proof — no pairing, no
//! discovery; the consumer is given the host's `address` + `token` directly.

use cranpose_services::peer::{self, ByteSource, BytesSource, PeerServer, SourceResolver};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// A track the host is willing to serve: its local path + display title.
#[derive(Clone)]
struct SharedTrack {
    path: String,
    title: String,
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

    let server = PeerServer::start("0.0.0.0:0", token.clone(), resolver)
        .map_err(|error| error.to_string())?;
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

/// Replaces the set of tracks the host shares with the given `(path, title)`
/// pairs (only local-file tracks belong here). No-op when the host is off.
pub fn set_shared_tracks(tracks: &[(String, String)]) {
    let Ok(guard) = HOST.lock() else { return };
    let Some(host) = guard.as_ref() else { return };
    let Ok(mut map) = host.shared.lock() else {
        return;
    };
    map.clear();
    for (index, (path, title)) in tracks.iter().enumerate() {
        map.insert(
            index.to_string(),
            SharedTrack {
                path: path.clone(),
                title: title.clone(),
            },
        );
    }
}

/// One entry from a peer's manifest.
pub struct PeerEntry {
    pub handle: String,
    pub title: String,
}

/// Fetches a peer's manifest (the list of tracks it is sharing).
pub fn fetch_manifest(base: &str, token: &str) -> Result<Vec<PeerEntry>, String> {
    let result = peer::fetch_range(base, token, "manifest", 0, None).map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&result.bytes);
    Ok(parse_manifest(&text))
}

/// Spools a peer track to a temp file and returns `(spool_path, title)`. Runs on
/// a background thread (it does network I/O).
pub fn spool_peer_track(
    base: &str,
    token: &str,
    entry: &PeerEntry,
) -> Result<(String, String), String> {
    let dir = peer_cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let spool = dir.join(format!(
        "{}-{}.dat",
        sanitize(base),
        sanitize(&entry.handle)
    ));
    let mut file = std::fs::File::create(&spool).map_err(|e| e.to_string())?;
    peer::fetch_to_writer(base, token, &entry.handle, 0, None, &mut file)
        .map_err(|e| e.to_string())?;
    Ok((spool.to_string_lossy().into_owned(), entry.title.clone()))
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
            format!("{handle}\t{title}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_manifest(text: &str) -> Vec<PeerEntry> {
    text.lines()
        .filter_map(|line| {
            let (handle, title) = line.split_once('\t')?;
            if handle.is_empty() {
                return None;
            }
            Some(PeerEntry {
                handle: handle.to_string(),
                title: title.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips() {
        let mut map = HashMap::new();
        map.insert(
            "0".to_string(),
            SharedTrack {
                path: "/a.mp3".into(),
                title: "First\tSong".into(),
            },
        );
        map.insert(
            "1".to_string(),
            SharedTrack {
                path: "/b.mp3".into(),
                title: "Second".into(),
            },
        );
        let text = manifest_text(&map);
        let entries = parse_manifest(&text);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].handle, "0");
        assert_eq!(entries[0].title, "First Song"); // tab sanitized to space
        assert_eq!(entries[1].handle, "1");
    }

    #[test]
    fn host_starts_serves_and_streams() {
        // Write a real file, share it, fetch its manifest + bytes back.
        let dir = peer_cache_dir().join("selftest");
        std::fs::create_dir_all(&dir).unwrap();
        let track = dir.join("tone.dat");
        let data: Vec<u8> = (0..3000u32).map(|i| i as u8).collect();
        std::fs::write(&track, &data).unwrap();

        let (addr, token) = start_host().expect("start");
        set_shared_tracks(&[(track.to_string_lossy().into_owned(), "Tone".to_string())]);
        // Connect to the loopback port directly (display_addr may pick a LAN IP).
        let port = addr.rsplit(':').next().unwrap();
        let base = format!("127.0.0.1:{port}");

        let manifest = fetch_manifest(&base, &token).expect("manifest");
        assert_eq!(manifest.len(), 1);
        assert_eq!(manifest[0].title, "Tone");

        let (spool, title) = spool_peer_track(&base, &token, &manifest[0]).expect("spool");
        assert_eq!(title, "Tone");
        assert_eq!(std::fs::read(&spool).unwrap(), data);

        stop_host();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
