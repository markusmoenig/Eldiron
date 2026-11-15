//! Device-local immutable draw data. Identical bytes share storage, including
//! across simultaneous draws. GPU buffers are never overwritten or explicitly
//! destroyed by the cache.
use crate::{GpuContext, RenderError, Result};
use std::{
    collections::{BTreeMap, HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};
use wgpu::util::DeviceExt;

const BYTE_LIMIT: u64 = 128 * 1024 * 1024;
const ENTRY_LIMIT: usize = 16_384;

/// Retained GPU/CPU cache bytes plus all live immutable allocation owners.
/// Caller Arcs may outlive eviction. Raw queued/driver-only handles are excluded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BufferCacheUsage {
    /// All cached or caller-owned immutable allocations. Raw queued/driver-only
    /// buffer handles are excluded; gpu_bytes below is the cached subset.
    pub live_bytes: u64,
    pub live_allocations: u64,
    pub peak_live_bytes: u64,
    pub gpu_bytes: u64,
    pub cpu_bytes: u64,
    pub entries: usize,
    pub byte_limit: u64,
    pub entry_limit: usize,
    pub allocations: u64,
    pub hits: u64,
    pub evictions: u64,
    pub uploaded_bytes: u64,
}

/// Immutable shader/geometry input. Use STORAGE only with read-only bindings;
/// writing through a shader would invalidate the shared-content contract.
/// COPY_DST/MAP_WRITE usages are rejected. Retain this Arc through submission.
pub struct GpuBuffer {
    _allocation: crate::allocation_usage::AllocationCharge,
    buffer: wgpu::Buffer,
    generation: u64,
}
impl GpuBuffer {
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn device_generation(&self) -> u64 {
        self.generation
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    usage: u32,
    bytes: usize,
    hash: u64,
}
struct Entry {
    bytes: Box<[u8]>,
    buffer: Arc<GpuBuffer>,
    used: u64,
}
struct Cache {
    entries: HashMap<Key, Vec<Entry>>,
    clock: u64,
    lru: BTreeMap<u64, Key>,
    usage: BufferCacheUsage,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            clock: 0,
            lru: BTreeMap::new(),
            usage: BufferCacheUsage {
                byte_limit: BYTE_LIMIT,
                entry_limit: ENTRY_LIMIT,
                ..Default::default()
            },
        }
    }
}
impl Cache {
    fn evict(&mut self) -> bool {
        let Some((used, key)) = self.lru.pop_first() else {
            return false;
        };
        let entries = self.entries.get_mut(&key).unwrap();
        let index = entries.iter().position(|entry| entry.used == used).unwrap();
        let entry = entries.swap_remove(index);
        self.usage.cpu_bytes -= entry.bytes.len() as u64;
        self.usage.gpu_bytes -= entry.buffer.buffer.size();
        self.usage.entries -= 1;
        self.usage.evictions += 1;
        if entries.is_empty() {
            self.entries.remove(&key);
        }
        // Ordinary drop preserves in-flight commands and caller-held Arcs.
        true
    }
}
#[derive(Default)]
pub(crate) struct BufferCache(Mutex<Cache>, Arc<crate::allocation_usage::AllocationUsage>);
impl BufferCache {
    pub fn usage(&self) -> BufferCacheUsage {
        let mut usage = self.0.lock().unwrap().usage;
        (
            usage.live_bytes,
            usage.live_allocations,
            usage.peak_live_bytes,
        ) = self.1.snapshot();
        usage
    }
    pub fn set_limits(&self, bytes: u64, entries: usize) {
        let mut cache = self.0.lock().unwrap();
        cache.usage.byte_limit = bytes;
        cache.usage.entry_limit = entries;
        while cache.usage.gpu_bytes + cache.usage.cpu_bytes > bytes || cache.usage.entries > entries
        {
            if !cache.evict() {
                break;
            }
        }
    }
    pub fn upload(
        &self,
        context: &GpuContext,
        bytes: &[u8],
        usage: wgpu::BufferUsages,
    ) -> Result<Arc<GpuBuffer>> {
        let allowed = wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::INDEX
            | wgpu::BufferUsages::UNIFORM
            | wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC;
        let size = (bytes.len() as u64).max(1).div_ceil(4) * 4;
        let limits = context.device.limits();
        if context.is_lost()
            || usage.is_empty()
            || !(usage & !allowed).is_empty()
            || size > limits.max_buffer_size
            || (usage.contains(wgpu::BufferUsages::UNIFORM)
                && size > u64::from(limits.max_uniform_buffer_binding_size))
            || (usage.contains(wgpu::BufferUsages::STORAGE)
                && size > u64::from(limits.max_storage_buffer_binding_size))
        {
            return Err(RenderError(
                "invalid immutable buffer usage, size or device".into(),
            ));
        }
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let key = Key {
            usage: usage.bits(),
            bytes: bytes.len(),
            hash: hasher.finish(),
        };
        let mut cache = self.0.lock().unwrap();
        cache.clock = cache
            .clock
            .checked_add(1)
            .ok_or_else(|| RenderError("immutable buffer cache epoch exhausted".into()))?;
        let used = cache.clock;
        let hit = cache
            .entries
            .get_mut(&key)
            .and_then(|entries| entries.iter_mut().find(|entry| &*entry.bytes == bytes))
            .map(|entry| {
                // Actual bytes too: a hash collision cannot alias content.
                let previous = entry.used;
                entry.used = used;
                (previous, entry.buffer.clone())
            });
        if let Some((previous, buffer)) = hit {
            cache.lru.remove(&previous);
            cache.lru.insert(used, key);
            cache.usage.hits += 1;
            return Ok(buffer);
        }
        let retain = size.saturating_add(bytes.len() as u64) <= cache.usage.byte_limit
            && cache.usage.entry_limit > 0;
        if retain {
            while cache.usage.gpu_bytes + cache.usage.cpu_bytes + size + bytes.len() as u64
                > cache.usage.byte_limit
                || cache.usage.entries >= cache.usage.entry_limit
            {
                if !cache.evict() {
                    break;
                }
            }
        }
        let buffer = if bytes.is_empty() {
            context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("renderkit immutable draw data"),
                size,
                usage,
                mapped_at_creation: false,
            })
        } else {
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("renderkit immutable draw data"),
                    contents: bytes,
                    usage,
                })
        };
        let buffer = Arc::new(GpuBuffer {
            _allocation: self.1.charge(size),
            buffer,
            generation: context.generation(),
        });
        cache.usage.allocations += 1;
        cache.usage.uploaded_bytes += bytes.len() as u64;
        if retain {
            cache.usage.gpu_bytes += buffer.buffer.size();
            cache.usage.cpu_bytes += bytes.len() as u64;
            cache.usage.entries += 1;
            cache.lru.insert(used, key);
            cache.entries.entry(key).or_default().push(Entry {
                bytes: bytes.into(),
                buffer: buffer.clone(),
                used,
            });
        }
        Ok(buffer)
    }
}
