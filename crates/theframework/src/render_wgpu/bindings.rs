//! Device-local immutable binding reuse. Entries pin input allocations; dynamic
//! offsets select sized immutable ranges within those same pinned allocations;
//! unused and retired atlas bindings leave at the end of the root frame.
use crate::paint::Sampling;
use renderkit::{GpuBufferRange, GpuImage, wgpu};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

const GROUP_LIMIT: usize = 2048;
const BYTE_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct BindingCacheUsage {
    pub groups: usize,
    /// Retained input estimates plus a fixed entry allowance, counting each
    /// image once and buffer references per binding. Driver memory is unmeasured.
    pub estimated_bytes: u64,
    pub group_limit: usize,
    pub byte_limit: u64,
    pub created: u64,
    pub hits: u64,
    pub evictions: u64,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Key {
    params: (usize, u64),
    field: Option<(usize, u64)>,
    image: usize,
    sampling: Sampling,
}
impl Key {
    pub(super) fn new(
        params: &Arc<GpuBufferRange>,
        field: Option<&Arc<GpuBufferRange>>,
        image: &Arc<GpuImage>,
        sampling: Sampling,
    ) -> Self {
        Self {
            params: (params.allocation_identity(), params.size()),
            field: field.map(|b| (b.allocation_identity(), b.size())),
            image: Arc::as_ptr(image) as usize,
            sampling,
        }
    }
}
struct Entry {
    bind: wgpu::BindGroup,
    _params: Arc<GpuBufferRange>,
    _field: Option<Arc<GpuBufferRange>>,
    image: Arc<GpuImage>,
    bytes: u64,
    frame: u64,
    used: u64,
}
#[derive(Default)]
struct Cache {
    entries: HashMap<Key, Entry>,
    images: HashMap<usize, (usize, u64)>,
    bytes: u64,
    frame: u64,
    clock: u64,
    created: u64,
    hits: u64,
    evictions: u64,
}
impl Cache {
    fn remove(&mut self, key: &Key) {
        if let Some(entry) = self.entries.remove(key) {
            self.bytes -= entry.bytes;
            let image = self.images.get_mut(&key.image).unwrap();
            image.0 -= 1;
            if image.0 == 0 {
                self.bytes -= image.1;
                self.images.remove(&key.image);
            }
            self.evictions += 1;
        }
    }
}
#[derive(Default)]
pub(super) struct Bindings(Mutex<Cache>);
impl Bindings {
    pub(super) fn begin_frame(&self) {
        let mut cache = self.0.lock().unwrap();
        cache.frame = cache.frame.checked_add(1).expect("binding frame exhausted");
    }
    pub(super) fn get(&self, key: &Key) -> Option<wgpu::BindGroup> {
        let mut cache = self.0.lock().unwrap();
        cache.clock = cache.clock.checked_add(1).expect("binding clock exhausted");
        let (frame, used) = (cache.frame, cache.clock);
        let entry = cache.entries.get_mut(key)?;
        entry.frame = frame;
        entry.used = used;
        let bind = entry.bind.clone();
        cache.hits += 1;
        Some(bind)
    }
    pub(super) fn created(
        &self,
        key: Option<Key>,
        bind: &wgpu::BindGroup,
        params: &Arc<GpuBufferRange>,
        field: Option<&Arc<GpuBufferRange>>,
        image: &Arc<GpuImage>,
    ) {
        let mut cache = self.0.lock().unwrap();
        cache.created += 1;
        let Some(key) = key else {
            return;
        };
        // Private draws are serialized today; replacement also keeps accounting
        // correct if immutable binding preparation becomes parallel later.
        if cache.entries.contains_key(&key) {
            cache.remove(&key);
        }
        let bytes = params.buffer().size() + field.map_or(0, |b| b.buffer().size()) + 512;
        let image_bytes = image_bytes(image);
        if bytes + image_bytes > BYTE_LIMIT {
            return;
        }
        while cache.entries.len() >= GROUP_LIMIT
            || cache.bytes
                + bytes
                + if cache.images.contains_key(&key.image) {
                    0
                } else {
                    image_bytes
                }
                > BYTE_LIMIT
        {
            let Some(oldest) = cache
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(key, _)| *key)
            else {
                break;
            };
            cache.remove(&oldest);
        }
        if let Some((count, _)) = cache.images.get_mut(&key.image) {
            *count += 1;
        } else {
            cache.images.insert(key.image, (1, image_bytes));
            cache.bytes += image_bytes;
        }
        cache.bytes += bytes;
        let entry = Entry {
            bind: bind.clone(),
            _params: params.clone(),
            _field: field.cloned(),
            image: image.clone(),
            bytes,
            frame: cache.frame,
            used: cache.clock,
        };
        cache.entries.insert(key, entry);
    }
    pub(super) fn finish_frame(&self, valid: impl Fn(&Arc<GpuImage>) -> bool) {
        let mut cache = self.0.lock().unwrap();
        let retired = cache
            .entries
            .iter()
            .filter(|(_, entry)| entry.frame != cache.frame || !valid(&entry.image))
            .map(|(key, _)| *key)
            .collect::<Vec<_>>();
        for key in retired {
            cache.remove(&key);
        }
    }
    pub(super) fn usage(&self) -> BindingCacheUsage {
        let cache = self.0.lock().unwrap();
        BindingCacheUsage {
            groups: cache.entries.len(),
            estimated_bytes: cache.bytes,
            group_limit: GROUP_LIMIT,
            byte_limit: BYTE_LIMIT,
            created: cache.created,
            hits: cache.hits,
            evictions: cache.evictions,
        }
    }
}
fn image_bytes(image: &GpuImage) -> u64 {
    let (block_w, block_h) = image.descriptor.format.block_dimensions();
    let block_bytes = u64::from(image.descriptor.format.block_copy_size(None).unwrap_or(16));
    let size = image.texture.size();
    (0..image.texture.mip_level_count())
        .map(|level| {
            u64::from((size.width >> level).max(1).div_ceil(block_w))
                * u64::from((size.height >> level).max(1).div_ceil(block_h))
                * u64::from(size.depth_or_array_layers)
                * block_bytes
        })
        .sum()
}
