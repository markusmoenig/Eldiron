//! Ordered gameplay delivery while required HTTP content downloads separately.
//! No frame, offer, action receipt or private event is discarded/coalesced.
use crate::content_task::ContentTask;
use crate::*;
use std::collections::VecDeque;
use std::time::Duration;
const MAX_WAIT_PACKETS: usize = 256;
const MAX_WAIT_BYTES: usize = 4 * 1024 * 1024;
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);
pub enum ContentDelivery {
    Content(ContentHint, PhysicsContent),
    Packet(ServerMessage),
}
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct ContentStreamDiagnostics {
    pub queued_packets: usize,
    pub queued_bytes: usize,
    pub peak_packets: usize,
    pub peak_bytes: usize,
    pub downloading: bool,
    pub required_downloads: u64,
}
#[derive(Default)]
pub struct ContentStream {
    queue: VecDeque<(ServerMessage, usize)>,
    bytes: usize,
    hint_index: usize,
    delivered_bytes: usize,
    waiting_since: Option<instant::Instant>,
    cache: ContentLoader,
    download: Option<(ContentHint, ContentTask)>,
    diagnostics: ContentStreamDiagnostics,
    failure: Option<String>,
}
impl ContentStream {
    pub fn push(&mut self, endpoint: &str, packet: ServerMessage) -> Result<(), String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.push_packet(endpoint, packet);
        self.checked(result)
    }
    fn push_packet(&mut self, endpoint: &str, packet: ServerMessage) -> Result<(), String> {
        match packet {
            ServerMessage::Pong { .. } => return Ok(()),
            ServerMessage::Preload { content } => {
                self.cache.prefetch(endpoint, &content);
                return Ok(());
            }
            _ => {}
        }
        let size = codec::encode_server(&packet)
            .map_err(|e| e.to_string())?
            .len();
        if self.queue.len() >= MAX_WAIT_PACKETS || self.bytes.saturating_add(size) > MAX_WAIT_BYTES
        {
            return Err("Content/gameplay consumer fell behind; reconnect to resync".into());
        }
        self.bytes += size;
        self.queue.push_back((packet, size));
        self.diagnostics.peak_packets = self.diagnostics.peak_packets.max(self.queue.len());
        self.diagnostics.peak_bytes = self.diagnostics.peak_bytes.max(self.bytes);
        Ok(())
    }
    pub fn poll(&mut self, endpoint: &str) -> Result<Option<ContentDelivery>, String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.poll_next(endpoint);
        self.checked(result)
    }
    fn checked<T>(&mut self, result: Result<T, String>) -> Result<T, String> {
        if let Err(error) = &result {
            self.failure = Some(error.clone());
            self.download = None;
            self.cache = ContentLoader::default();
        }
        result
    }
    fn poll_next(&mut self, endpoint: &str) -> Result<Option<ContentDelivery>, String> {
        self.cache.poll_ready();
        if self
            .waiting_since
            .is_some_and(|t| t.elapsed() >= WAIT_TIMEOUT)
        {
            return Err("Required content timed out; reconnect to resync".into());
        }
        if let Some((hint, task)) = &mut self.download {
            let Some(result) = task.poll() else {
                return Ok(None);
            };
            let package = result?;
            let hint = hint.clone();
            self.download = None;
            self.hint_index += 1;
            // A newer optional preload may already be cached. The exact older
            // frame still receives its verified package before advancing FIFO.
            self.cache.insert(hint.clone(), package.clone())?;
            return self.content(hint, package);
        }
        let Some((packet, _)) = self.queue.front() else {
            return Ok(None);
        };
        if let ServerMessage::Frame { content, .. } = packet {
            if let Some(hint) = content.get(self.hint_index) {
                let hint = hint.clone();
                if let Some(package) = self.cache.get(&hint) {
                    self.hint_index += 1;
                    return self.content(hint, package);
                }
                self.waiting_since.get_or_insert_with(instant::Instant::now);
                self.download = Some((hint.clone(), ContentTask::start(endpoint, &hint)?));
                self.diagnostics.required_downloads =
                    self.diagnostics.required_downloads.saturating_add(1);
                return Ok(None);
            }
        }
        let (packet, size) = self.queue.pop_front().unwrap();
        self.bytes -= size;
        self.hint_index = 0;
        self.delivered_bytes = 0;
        self.waiting_since = None;
        Ok(Some(ContentDelivery::Packet(packet)))
    }
    fn content(
        &mut self,
        hint: ContentHint,
        package: PhysicsContent,
    ) -> Result<Option<ContentDelivery>, String> {
        let bytes = codec::encode_content(&package)
            .map_err(|e| e.to_string())?
            .len();
        self.delivered_bytes = self.delivered_bytes.saturating_add(bytes);
        if self.delivered_bytes > 16 * 1024 * 1024 {
            return Err(
                "Required frame content exceeds staging budget; reconnect to resync".into(),
            );
        }
        Ok(Some(ContentDelivery::Content(hint, package)))
    }
    pub fn diagnostics(&self) -> ContentStreamDiagnostics {
        let mut d = self.diagnostics.clone();
        d.queued_packets = self.queue.len();
        d.queued_bytes = self.bytes;
        d.downloading = self.download.is_some();
        d
    }
    pub fn cached_regions(&self) -> usize {
        self.cache.len()
    }
    pub fn cached_bytes(&self) -> usize {
        self.cache.encoded_bytes()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "content_stream_tests.rs"]
mod tests;
