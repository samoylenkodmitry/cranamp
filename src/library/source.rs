//! Bounded reads through Cranpose's platform media and HTTP services.
use std::io::{self, Read, Seek, SeekFrom};
use symphonia::core::io::MediaSource;

pub(super) const READ_BUDGET: usize = 12 * 1024 * 1024;
const CHUNK: u64 = 64 * 1024;

pub(super) struct BoundedSource {
    source: Box<dyn MediaSource>,
    remaining: usize,
}
impl BoundedSource {
    pub(super) fn new(source: Box<dyn MediaSource>) -> Self {
        Self {
            source,
            remaining: READ_BUDGET,
        }
    }
}
impl Read for BoundedSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            return Err(io::Error::other("Metadata read limit reached"));
        }
        let count = out.len().min(self.remaining);
        let read = self.source.read(&mut out[..count])?;
        self.remaining -= read;
        Ok(read)
    }
}
impl Seek for BoundedSource {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.source.seek(from)
    }
}
impl MediaSource for BoundedSource {
    fn is_seekable(&self) -> bool {
        self.source.is_seekable()
    }
    fn byte_len(&self) -> Option<u64> {
        self.source.byte_len()
    }
}

struct FileSource {
    handle: cranpose_services::MediaSourceHandle,
    seekable: bool,
}
impl Read for FileSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.handle.read(out)
    }
}
impl Seek for FileSource {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.handle.seek(from)
    }
}
impl MediaSource for FileSource {
    fn is_seekable(&self) -> bool {
        self.seekable
    }
    fn byte_len(&self) -> Option<u64> {
        self.handle.byte_len()
    }
}

// A small block cache serves the demuxer's seeks. No audio bytes are written to disk.
struct HttpSource {
    uri: String,
    client: cranpose_services::HttpClientRef,
    position: u64,
    length: Option<u64>,
    seekable: bool,
    blocks: std::collections::BTreeMap<u64, Vec<u8>>,
    fetched: usize,
}
impl HttpSource {
    fn block(&mut self, start: u64) -> io::Result<()> {
        if self.blocks.contains_key(&start) {
            return Ok(());
        }
        if self.fetched >= READ_BUDGET {
            return Err(io::Error::other("Metadata network limit reached"));
        }
        let request = cranpose_services::HttpRequest::get(&self.uri)
            .header("Range", format!("bytes={start}-{}", start + CHUNK - 1))
            .header("Accept-Encoding", "identity");
        let control = cranpose_services::HttpControl::new();
        let result = futures_lite::future::block_on(futures_lite::future::race(
            async {
                let response = self
                    .client
                    .send(&request, control.clone())
                    .await
                    .map_err(io::Error::other)?;
                if response.status == 416 && self.length.is_some_and(|len| start >= len) {
                    return Ok(Vec::new());
                }
                let response = response.error_for_status().map_err(io::Error::other)?;
                if response.status == 206 {
                    let range = response
                        .header("content-range")
                        .and_then(|v| v.strip_prefix("bytes "))
                        .and_then(|v| v.split_once('/'));
                    let (span, total) =
                        range.ok_or_else(|| io::Error::other("Missing content range"))?;
                    let first = span
                        .split_once('-')
                        .and_then(|(a, _)| a.parse::<u64>().ok());
                    if first != Some(start) {
                        return Err(io::Error::other("Incorrect content range"));
                    }
                    self.length = total.parse().ok();
                    self.seekable = true;
                } else if start == 0 {
                    self.length = response.content_length;
                    self.seekable = false;
                } else {
                    return Err(io::Error::other("Server does not support range reads"));
                }
                let mut bytes = Vec::new();
                while bytes.len() < CHUNK as usize {
                    let Some(chunk) = response.read_chunk().await.map_err(io::Error::other)? else {
                        break;
                    };
                    let take = chunk.len().min(CHUNK as usize - bytes.len());
                    bytes.extend_from_slice(&chunk[..take]);
                }
                Ok(bytes)
            },
            async {
                cranpose_core::delay(std::time::Duration::from_secs(8)).await;
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Metadata request timed out",
                ))
            },
        ));
        control.cancel();
        let bytes = result?;
        self.fetched += bytes.len();
        self.blocks.insert(start, bytes);
        Ok(())
    }
}
impl Read for HttpSource {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() || self.length.is_some_and(|len| self.position >= len) {
            return Ok(0);
        }
        let start = self.position / CHUNK * CHUNK;
        self.block(start)?;
        let bytes = &self.blocks[&start];
        let offset = (self.position - start) as usize;
        if offset >= bytes.len() {
            return Ok(0);
        }
        let count = out.len().min(bytes.len() - offset);
        out[..count].copy_from_slice(&bytes[offset..offset + count]);
        self.position += count as u64;
        Ok(count)
    }
}
impl Seek for HttpSource {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::Current(d) => self.position.checked_add_signed(d),
            SeekFrom::End(d) => self.length.and_then(|len| len.checked_add_signed(d)),
        }
        .ok_or_else(|| io::Error::other("Invalid metadata seek"))?;
        self.position = position;
        Ok(position)
    }
}
impl MediaSource for HttpSource {
    fn is_seekable(&self) -> bool {
        self.seekable
    }
    fn byte_len(&self) -> Option<u64> {
        self.length
    }
}

pub(super) fn open(uri: &str) -> io::Result<BoundedSource> {
    let source: Box<dyn MediaSource> = if uri.starts_with("https://") || uri.starts_with("http://")
    {
        let mut source = HttpSource {
            uri: uri.into(),
            client: cranpose_services::default_http_client(),
            position: 0,
            length: None,
            seekable: false,
            blocks: Default::default(),
            fetched: 0,
        };
        source.block(0)?;
        Box::new(source)
    } else {
        let mut handle = cranpose_services::open_media_source(uri)?;
        let seekable = handle.stream_position().is_ok();
        Box::new(FileSource { handle, seekable })
    };
    Ok(BoundedSource::new(source))
}
