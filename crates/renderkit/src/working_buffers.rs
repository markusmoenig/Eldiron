//! Device-local reusable writable GPU storage. Retained entries are bounded;
//! caller leases and queued commands may outlive cache eviction.
use crate::{GpuContext, RenderError, Result};
use std::sync::{Arc, Mutex};

const BYTE_LIMIT: u64 = 128 * 1024 * 1024;
const ENTRY_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkingBufferDescriptor {
    pub bytes: u64,
    pub usage: wgpu::BufferUsages,
}
impl WorkingBufferDescriptor {
    pub fn storage(bytes: u64) -> Self {
        Self {
            bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        }
    }
}

/// GPU allocation sizes retained by the cache. Active uncached leases, queued
/// resources and driver/allocator overhead are not a total-memory guarantee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkingBufferPoolUsage {
    /// All allocations still owned by cache entries or live leases, including
    /// uncached/evicted active owners. Excludes raw queue/driver-only handles.
    pub live_bytes: u64,
    pub live_allocations: u64,
    pub peak_live_bytes: u64,
    pub cached_bytes: u64,
    pub cached_buffers: usize,
    pub byte_limit: u64,
    pub buffer_limit: usize,
    pub allocations: u64,
    pub hits: u64,
    pub evictions: u64,
    pub uncached_allocations: u64,
}
struct Buffer {
    _charge: crate::allocation_usage::AllocationCharge,
    buffer: wgpu::Buffer,
    generation: u64,
}
/// Contents are unspecified on acquisition: overwrite every element before
/// reading. Retain this lease until every use has been submitted on the owning
/// queue. Raw buffer clones do not prevent reuse by the cache.
#[derive(Clone)]
pub struct WorkingBufferLease(Arc<Buffer>);
impl WorkingBufferLease {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.0.buffer
    }
    pub fn device_generation(&self) -> u64 {
        self.0.generation
    }
}
struct Entry {
    descriptor: WorkingBufferDescriptor,
    buffer: Arc<Buffer>,
    used: u64,
}
struct Cache {
    entries: Vec<Entry>,
    clock: u64,
    usage: WorkingBufferPoolUsage,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            clock: 0,
            usage: WorkingBufferPoolUsage {
                byte_limit: BYTE_LIMIT,
                buffer_limit: ENTRY_LIMIT,
                ..Default::default()
            },
        }
    }
}
impl Cache {
    fn evict(&mut self) -> bool {
        let Some(index) = self
            .entries
            .iter()
            .enumerate()
            .min_by_key(|(_, e)| (Arc::strong_count(&e.buffer) != 1, e.used))
            .map(|(i, _)| i)
        else {
            return false;
        };
        let entry = self.entries.swap_remove(index);
        self.usage.cached_bytes -= entry.descriptor.bytes;
        self.usage.cached_buffers -= 1;
        self.usage.evictions += 1;
        // No explicit destroy: live leases and command ownership stay valid.
        true
    }
}
#[derive(Default)]
pub(crate) struct WorkingBuffers(Mutex<Cache>, Arc<crate::allocation_usage::AllocationUsage>);
impl WorkingBuffers {
    pub fn usage(&self) -> WorkingBufferPoolUsage {
        let mut usage = self.0.lock().unwrap().usage;
        (
            usage.live_bytes,
            usage.live_allocations,
            usage.peak_live_bytes,
        ) = self.1.snapshot();
        usage
    }
    pub fn set_limits(&self, bytes: u64, buffers: usize) {
        let mut cache = self.0.lock().unwrap();
        cache.usage.byte_limit = bytes;
        cache.usage.buffer_limit = buffers;
        while cache.usage.cached_bytes > bytes || cache.entries.len() > buffers {
            if !cache.evict() {
                break;
            }
        }
    }
    pub fn acquire(
        &self,
        context: &GpuContext,
        descriptor: WorkingBufferDescriptor,
    ) -> Result<WorkingBufferLease> {
        let allowed = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::UNIFORM
            | wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::INDEX
            | wgpu::BufferUsages::INDIRECT
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST;
        let limits = context.device.limits();
        if context.is_lost()
            || descriptor.bytes == 0
            || descriptor.bytes % 4 != 0
            || descriptor.bytes > limits.max_buffer_size
            || descriptor.usage.is_empty()
            || !(descriptor.usage & !allowed).is_empty()
            || (descriptor.usage.contains(wgpu::BufferUsages::STORAGE)
                && descriptor.bytes > u64::from(limits.max_storage_buffer_binding_size))
            || (descriptor.usage.contains(wgpu::BufferUsages::UNIFORM)
                && descriptor.bytes > u64::from(limits.max_uniform_buffer_binding_size))
        {
            return Err(RenderError("invalid or lost-device working buffer".into()));
        }
        let mut cache = self.0.lock().unwrap();
        cache.clock = cache.clock.saturating_add(1);
        let used = cache.clock;
        if let Some(index) = cache.entries.iter().position(|entry| {
            entry.descriptor == descriptor && Arc::strong_count(&entry.buffer) == 1
        }) {
            cache.entries[index].used = used;
            let buffer = cache.entries[index].buffer.clone();
            cache.usage.hits += 1;
            return Ok(WorkingBufferLease(buffer));
        }
        let retain = descriptor.bytes <= cache.usage.byte_limit && cache.usage.buffer_limit > 0;
        if retain {
            while cache
                .usage
                .cached_bytes
                .checked_add(descriptor.bytes)
                .is_none_or(|bytes| bytes > cache.usage.byte_limit)
                || cache.entries.len() >= cache.usage.buffer_limit
            {
                if !cache.evict() {
                    break;
                }
            }
        } else {
            while cache.evict() {}
        }
        let buffer = Arc::new(Buffer {
            _charge: self.1.charge(descriptor.bytes),
            buffer: context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("renderkit pooled working buffer"),
                size: descriptor.bytes,
                usage: descriptor.usage,
                mapped_at_creation: false,
            }),
            generation: context.generation(),
        });
        cache.usage.allocations += 1;
        if retain {
            cache.entries.push(Entry {
                descriptor,
                buffer: buffer.clone(),
                used,
            });
            cache.usage.cached_bytes += descriptor.bytes;
            cache.usage.cached_buffers += 1;
        } else {
            cache.usage.uncached_allocations += 1;
        }
        Ok(WorkingBufferLease(buffer))
    }
}
