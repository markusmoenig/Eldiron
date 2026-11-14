//! Shared source-buffer residency and bounded upload submissions. Renderers
//! cache weak references; idle buffers can be evicted across view owners.
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

const BYTE_LIMIT: u64 = 64 * 1024 * 1024;
const SUBMISSION_LIMIT: u64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePoolUsage {
    pub bytes: u64,
    pub upload_bytes: u64,
    pub submissions: u64,
    pub byte_limit: u64,
    pub submission_limit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upload_bytes_and_submission_slots_are_shared_and_release_on_retirement() {
        let pool = SourcePool::default();
        let full = pool.submission(BYTE_LIMIT).unwrap();
        let before = pool.usage();
        assert!(pool.submission(1).is_none());
        assert_eq!(pool.usage(), before);
        drop(full);
        let leases: Vec<_> = (0..SUBMISSION_LIMIT)
            .map(|_| pool.submission(0).unwrap())
            .collect();
        assert!(pool.submission(0).is_none());
        assert_eq!(pool.usage().submissions, SUBMISSION_LIMIT);
        drop(leases);
        assert_eq!(pool.usage().submissions, 0);
        assert_eq!(pool.usage().bytes, 0);
    }
}
#[derive(Default)]
struct Usage {
    bytes: AtomicU64,
    uploads: AtomicU64,
    submissions: AtomicU64,
}
/// Retain pin() through the submission that consumes this allocation. A plain
/// cloned wgpu buffer handle alone does not pin the pool reservation.
pub struct SourceBuffer {
    buffer: wgpu::Buffer,
    bytes: u64,
    allocation: Arc<Allocation>,
}
struct Allocation {
    usage: Arc<Usage>,
    bytes: u64,
    pins: AtomicU64,
}
impl Drop for Allocation {
    fn drop(&mut self) {
        self.usage.bytes.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}
/// Send-safe lifetime/accounting metadata. GPU handles stay on their owning
/// thread; queued command buffers retain the actual GPU resources themselves.
pub struct SourceBufferPin(Arc<Allocation>);
impl Drop for SourceBufferPin {
    fn drop(&mut self) {
        self.0.pins.fetch_sub(1, Ordering::Release);
    }
}
impl SourceBuffer {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
    pub fn pin(&self) -> SourceBufferPin {
        self.allocation.pins.fetch_add(1, Ordering::Relaxed);
        SourceBufferPin(self.allocation.clone())
    }
}
impl Drop for SourceBuffer {
    fn drop(&mut self) {
        if self.allocation.pins.load(Ordering::Acquire) == 0 {
            self.buffer.destroy();
        }
        // When the whole pool retires during a submission, ordinary handle
        // drop lets wgpu's submitted command ownership retire the resource.
        // Its pin keeps the allocation charged until completion as well.
    }
}
/// Upload staging and one outstanding submission slot. Move this lease into
/// on_submitted_work_done together with pins for the source allocations used.
pub struct SourceSubmission {
    usage: Arc<Usage>,
    bytes: u64,
}
impl Drop for SourceSubmission {
    fn drop(&mut self) {
        self.usage.bytes.fetch_sub(self.bytes, Ordering::Relaxed);
        self.usage.uploads.fetch_sub(self.bytes, Ordering::Relaxed);
        self.usage.submissions.fetch_sub(1, Ordering::Relaxed);
    }
}
#[derive(Default)]
struct State {
    clock: u64,
    buffers: HashMap<u64, (Arc<SourceBuffer>, u64)>,
}
#[derive(Default)]
pub(crate) struct SourcePool {
    state: Mutex<State>,
    usage: Arc<Usage>,
}
impl SourcePool {
    pub(crate) fn usage(&self) -> SourcePoolUsage {
        SourcePoolUsage {
            bytes: self.usage.bytes.load(Ordering::Relaxed),
            upload_bytes: self.usage.uploads.load(Ordering::Relaxed),
            submissions: self.usage.submissions.load(Ordering::Relaxed),
            byte_limit: BYTE_LIMIT,
            submission_limit: SUBMISSION_LIMIT,
        }
    }
    fn room(&self, state: &mut State, bytes: u64) -> bool {
        if bytes > BYTE_LIMIT {
            return false;
        }
        while self
            .usage
            .bytes
            .load(Ordering::Relaxed)
            .saturating_add(bytes)
            > BYTE_LIMIT
        {
            let victim = state
                .buffers
                .iter()
                .filter(|(_, (b, _))| {
                    Arc::strong_count(b) == 1 && b.allocation.pins.load(Ordering::Acquire) == 0
                })
                .min_by_key(|(_, (_, age))| *age)
                .map(|(id, _)| *id);
            let Some(victim) = victim else {
                return false;
            };
            state.buffers.remove(&victim);
        }
        true
    }
    pub(crate) fn submission(&self, bytes: u64) -> Option<SourceSubmission> {
        let mut state = self.state.lock().unwrap();
        if self.usage.submissions.load(Ordering::Relaxed) >= SUBMISSION_LIMIT
            || !self.room(&mut state, bytes)
        {
            return None;
        }
        self.usage.bytes.fetch_add(bytes, Ordering::Relaxed);
        self.usage.uploads.fetch_add(bytes, Ordering::Relaxed);
        self.usage.submissions.fetch_add(1, Ordering::Relaxed);
        Some(SourceSubmission {
            usage: self.usage.clone(),
            bytes,
        })
    }
    pub(crate) fn buffer(
        &self,
        device: &wgpu::Device,
        bytes: u64,
        data: Option<&[u8]>,
    ) -> Option<Arc<SourceBuffer>> {
        let mut state = self.state.lock().unwrap();
        if bytes == 0 || !self.room(&mut state, bytes) {
            return None;
        }
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pooled original source channels"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: data.is_some(),
        });
        if let Some(data) = data {
            buffer
                .slice(..)
                .get_mapped_range_mut()
                .slice(..data.len())
                .copy_from_slice(data);
            buffer.unmap();
        }
        self.usage.bytes.fetch_add(bytes, Ordering::Relaxed);
        let buffer = Arc::new(SourceBuffer {
            buffer,
            bytes,
            allocation: Arc::new(Allocation {
                usage: self.usage.clone(),
                bytes,
                pins: AtomicU64::new(0),
            }),
        });
        state.clock = state
            .clock
            .checked_add(1)
            .expect("source pool epoch exhausted");
        let id = state.clock;
        state.buffers.insert(id, (buffer.clone(), id));
        Some(buffer)
    }
    pub(crate) fn touch(&self, buffer: &Arc<SourceBuffer>) {
        let mut state = self.state.lock().unwrap();
        state.clock = state
            .clock
            .checked_add(1)
            .expect("source pool epoch exhausted");
        let age = state.clock;
        if let Some((_, last)) = state
            .buffers
            .values_mut()
            .find(|(entry, _)| Arc::ptr_eq(entry, buffer))
        {
            *last = age;
        }
    }
}
