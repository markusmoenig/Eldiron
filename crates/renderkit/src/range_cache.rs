//! Bounded exact-content ownership for immutable GPU ranges. Physical pages are
//! charged once, even when many cached inputs share one allocation.
use crate::{BufferCacheUsage, GpuBufferRange, GpuContext, RenderError, Result, wgpu};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    usage: u32,
    size: usize,
    hash: u64,
}
struct Entry {
    data: Box<[u8]>,
    range: Arc<GpuBufferRange>,
    used: u64,
}
struct Cache {
    entries: HashMap<Key, Vec<Entry>>,
    pages: HashMap<usize, (usize, u64)>,
    lru: BTreeMap<u64, Key>,
    clock: u64,
    storage: crate::range_storage::RangeStorage,
    usage: BufferCacheUsage,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            pages: Default::default(),
            lru: Default::default(),
            clock: 0,
            storage: Default::default(),
            usage: BufferCacheUsage {
                byte_limit: 64 * 1024 * 1024,
                entry_limit: 16_384,
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
        if entries.is_empty() {
            self.entries.remove(&key);
        }
        self.usage.cpu_bytes -= entry.data.len() as u64;
        self.usage.entries -= 1;
        self.usage.evictions += 1;
        let id = entry.range.allocation_identity();
        let page = self.pages.get_mut(&id).unwrap();
        page.0 -= 1;
        if page.0 == 0 {
            self.usage.gpu_bytes -= page.1;
            self.pages.remove(&id);
        }
        true
    }
    fn additional(&self, range: &GpuBufferRange) -> u64 {
        if self.pages.contains_key(&range.allocation_identity()) {
            0
        } else {
            range.buffer().size()
        }
    }
}
#[derive(Default)]
pub(crate) struct RangeCache(Mutex<Cache>, Arc<crate::allocation_usage::AllocationUsage>);
impl RangeCache {
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
    ) -> Result<Arc<GpuBufferRange>> {
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
                "invalid immutable range usage, size or device".into(),
            ));
        }
        let key = Key {
            usage: usage.bits(),
            size: bytes.len(),
            hash: crate::content_fingerprint::content_fingerprint(bytes),
        };
        let mut cache = self.0.lock().unwrap();
        cache.clock = cache
            .clock
            .checked_add(1)
            .ok_or_else(|| RenderError("immutable range epoch exhausted".into()))?;
        let used = cache.clock;
        let hit = cache
            .entries
            .get_mut(&key)
            .and_then(|entries| entries.iter_mut().find(|e| &*e.data == bytes))
            .map(|entry| {
                let previous = entry.used;
                entry.used = used;
                (previous, entry.range.clone())
            });
        if let Some((previous, range)) = hit {
            cache.lru.remove(&previous);
            cache.lru.insert(used, key);
            cache.usage.hits += 1;
            return Ok(range);
        }
        let (range, created) = cache.storage.upload(context, bytes, usage, &self.1)?;
        cache.usage.allocations += u64::from(created);
        cache.usage.uploaded_bytes += size;
        if range.buffer().size() + bytes.len() as u64 <= cache.usage.byte_limit
            && cache.usage.entry_limit > 0
        {
            while cache.usage.gpu_bytes
                + cache.usage.cpu_bytes
                + cache.additional(&range)
                + bytes.len() as u64
                > cache.usage.byte_limit
                || cache.usage.entries >= cache.usage.entry_limit
            {
                if !cache.evict() {
                    break;
                }
            }
            let id = range.allocation_identity();
            if let Some(page) = cache.pages.get_mut(&id) {
                page.0 += 1;
            } else {
                cache.pages.insert(id, (1, range.buffer().size()));
                cache.usage.gpu_bytes += range.buffer().size();
            }
            cache.usage.cpu_bytes += bytes.len() as u64;
            cache.usage.entries += 1;
            cache.lru.insert(used, key);
            cache.entries.entry(key).or_default().push(Entry {
                data: bytes.into(),
                range: range.clone(),
                used,
            });
        }
        Ok(range)
    }
}
