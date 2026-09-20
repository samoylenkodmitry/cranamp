use super::*;
use cranpose_services::{FolderEntry, FolderReader, FolderWriter, WritableFolderStore};
use std::sync::{Arc, Mutex as StdMutex};
type MockFiles = Arc<StdMutex<Vec<(String, Vec<u8>)>>>;
#[derive(Default)]
struct MockStore {
    files: MockFiles,
    writable: bool,
}
impl MockStore {
    fn new(writable: bool) -> Arc<Self> {
        Arc::new(Self {
            files: Arc::new(StdMutex::new(Vec::new())),
            writable,
        })
    }
}
struct OneChunkReader(Option<Vec<u8>>);
impl FolderReader for OneChunkReader {
    fn read_chunk(&mut self) -> Result<Option<Vec<u8>>, FolderError> {
        Ok(self.0.take())
    }
}
struct CollectingWriter {
    store: MockFiles,
    name: String,
    buffer: Vec<u8>,
}
impl FolderWriter for CollectingWriter {
    fn write_chunk(&mut self, bytes: &[u8]) -> Result<(), FolderError> {
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }
    fn finish(self: Box<Self>) -> Result<(), FolderError> {
        let mut files = self.store.lock().unwrap();
        files.retain(|(name, _)| name != &self.name);
        files.push((self.name, self.buffer));
        Ok(())
    }
}
impl WritableFolderStore for MockStore {
    fn write(&self, name: &str, contents: &[u8]) -> Result<(), FolderError> {
        if !self.writable {
            return Err(FolderError::ReadOnly);
        }
        let mut files = self.files.lock().unwrap();
        files.retain(|(existing, _)| existing != name);
        files.push((name.to_string(), contents.to_vec()));
        Ok(())
    }
    fn read(&self, name: &str) -> Result<Vec<u8>, FolderError> {
        self.files
            .lock()
            .unwrap()
            .iter()
            .find(|(existing, _)| existing == name)
            .map(|(_, bytes)| bytes.clone())
            .ok_or_else(|| FolderError::NotFound(name.to_string()))
    }
    fn list(&self) -> Result<Vec<FolderEntry>, FolderError> {
        Ok(self
            .files
            .lock()
            .unwrap()
            .iter()
            .map(|(name, contents)| FolderEntry {
                name: name.clone(),
                len: contents.len() as u64,
                modified_millis: None,
            })
            .collect())
    }
    fn open_read(&self, name: &str) -> Result<Box<dyn FolderReader>, FolderError> {
        let bytes = self.read(name)?;
        Ok(Box::new(OneChunkReader(Some(bytes))))
    }
    fn open_write(&self, name: &str) -> Result<Box<dyn FolderWriter>, FolderError> {
        if !self.writable {
            return Err(FolderError::ReadOnly);
        }
        Ok(Box::new(CollectingWriter {
            store: Arc::clone(&self.files),
            name: name.to_string(),
            buffer: Vec::new(),
        }))
    }
    fn remove(&self, name: &str) -> Result<(), FolderError> {
        self.files
            .lock()
            .unwrap()
            .retain(|(existing, _)| existing != name);
        Ok(())
    }
    fn is_writable(&self) -> bool {
        self.writable
    }
    fn handle(&self) -> String {
        "/mock".to_string()
    }
}
fn runtime_with(store: Arc<MockStore>) -> SyncRuntime {
    let config = SyncConfig {
        device_id: "self".to_string(),
        device_label: "Me".to_string(),
        enabled: true,
        folder: Some("/mock".to_string()),
    };
    SyncRuntime::new(config, Some(store))
}
fn drive_flush(rt: &mut SyncRuntime, force: bool) {
    if let Some(job) = rt.prepare_flush(force) {
        let result = store_write(&job.store, &job.file_name, &job.contents);
        rt.finish_flush(result);
    }
}
fn drive_poll(rt: &SyncRuntime) -> MergedSync {
    let (store, own) = rt.prepare_poll().expect("store");
    let raw = store_read_documents(&store).unwrap();
    let own_id = own.device_id.clone();
    let mut docs: Vec<SyncDocument> = super::super::parse_documents(&raw)
        .into_iter()
        .filter(|doc| doc.device_id != own_id)
        .collect();
    docs.push(own);
    merge(&docs)
}
#[test]
fn flush_writes_one_file_and_merges_peers() {
    let store = MockStore::new(true);
    let mut peer = SyncDocument::new("peer".into(), "Phone".into(), "android".into());
    peer.updated_at = 5;
    peer.resume = Some(ResumePoint {
        fingerprint: TrackFingerprint::new("song.mp3", 200),
        title: "Song".into(),
        position_s: 30.0,
        updated_at: 5,
    });
    peer.counts = vec![PlayCount {
        fingerprint: TrackFingerprint::new("song.mp3", 200),
        title: "Song".into(),
        count: 2,
        last_played: 5,
    }];
    store
        .write(
            "peer.cransync",
            super::super::serialize_document(&peer).as_bytes(),
        )
        .unwrap();
    let mut rt = runtime_with(store.clone());
    rt.record_play(TrackFingerprint::new("song.mp3", 200), "Song".into());
    drive_flush(&mut rt, true);
    assert_eq!(store.list().unwrap().len(), 2);
    let merged = drive_poll(&rt);
    let entry = merged
        .counts
        .get(&TrackFingerprint::new("song.mp3", 200))
        .expect("count");
    assert_eq!(entry.count, 3);
    assert_eq!(merged.resume.unwrap().fingerprint.basename, "song.mp3");
}
#[test]
fn read_only_store_degrades_to_receive_only() {
    let store = MockStore::new(false);
    let mut rt = runtime_with(store);
    rt.record_resume(TrackFingerprint::new("a.mp3", 10), "A".into(), 3.0);
    drive_flush(&mut rt, true);
    assert_eq!(rt.status, SyncStatus::ReceiveOnly);
}
#[test]
fn debounce_blocks_until_forced() {
    let store = MockStore::new(true);
    let mut rt = runtime_with(store.clone());
    rt.record_play(TrackFingerprint::new("x.mp3", 100), "X".into());
    rt.last_flush = now();
    assert!(rt.prepare_flush(false).is_none());
    drive_flush(&mut rt, true);
    assert_eq!(store.list().unwrap().len(), 1);
}
