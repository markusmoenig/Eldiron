//! Device-local retained target storage. Callers keep leases until every command
//! using a target has been submitted on this context's queue. Reused contents are
//! unspecified: render targets must be cleared or fully overwritten before use.
use crate::{GpuContext, RenderError, Result};
use std::sync::{Arc, Mutex};

const BYTE_LIMIT: u64 = 512 * 1024 * 1024;
const TEXTURE_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TargetDescriptor {
    pub width: u32,
    pub height: u32,
    /// D2 source arrays share the same lifetime/accounting policy. Multisampled
    /// attachments require one layer; estimated bytes include every layer/mip.
    pub array_layers: u32,
    pub format: wgpu::TextureFormat,
    pub samples: u32,
    pub mip_levels: u32,
    pub usage: wgpu::TextureUsages,
}
impl TargetDescriptor {
    pub fn color(width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        Self {
            width,
            height,
            array_layers: 1,
            format,
            samples: 1,
            mip_levels: 1,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
        }
    }
    pub fn attachment(width: u32, height: u32, format: wgpu::TextureFormat, samples: u32) -> Self {
        Self {
            width,
            height,
            array_layers: 1,
            format,
            samples,
            mip_levels: 1,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        }
    }
    fn estimated_bytes(self) -> Result<u64> {
        let (w, h) = self.format.block_dimensions();
        // Depth24Plus can use a native 32-bit depth plane plus stencil/padding.
        // This conservative storage estimate excludes driver/object overhead.
        let block = match self.format {
            wgpu::TextureFormat::Depth24Plus
            | wgpu::TextureFormat::Depth24PlusStencil8
            | wgpu::TextureFormat::Depth32FloatStencil8 => 8,
            _ => self
                .format
                .block_copy_size(None)
                .ok_or_else(|| RenderError("unsupported target storage format".into()))?,
        };
        let mut total = 0_u64;
        for mip in 0..self.mip_levels {
            let width = (self.width >> mip).max(1);
            let height = (self.height >> mip).max(1);
            let level = u64::from(width.div_ceil(w))
                .checked_mul(u64::from(height.div_ceil(h)))
                .and_then(|n| n.checked_mul(u64::from(block)))
                .and_then(|n| n.checked_mul(u64::from(self.samples)))
                .and_then(|n| n.checked_mul(u64::from(self.array_layers)))
                .ok_or_else(|| RenderError("target storage size overflow".into()))?;
            total = total
                .checked_add(level)
                .ok_or_else(|| RenderError("target mip storage size overflow".into()))?;
        }
        Ok(total)
    }
}

/// The cache bounds retained estimates, not active uncached targets, queue-owned
/// resources awaiting completion, or total driver memory. Large working targets
/// remain valid and uncached rather than failing an otherwise supported frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TargetPoolUsage {
    /// All allocations still owned by cache entries or live leases, including
    /// uncached/evicted active owners. Excludes raw queue/driver-only handles.
    pub live_bytes: u64,
    pub live_allocations: u64,
    pub peak_live_bytes: u64,
    pub cached_bytes: u64,
    pub cached_textures: usize,
    pub byte_limit: u64,
    pub texture_limit: usize,
    pub allocations: u64,
    pub hits: u64,
    pub evictions: u64,
    pub uncached_allocations: u64,
}
struct Target {
    _charge: crate::allocation_usage::AllocationCharge,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    generation: u64,
}
/// Raw texture/view clones alone do not pin a cache entry. Keep this lease (or a
/// pooled GpuImage) alive through submission of all consuming commands. Queue
/// ordering then makes reuse safe without a blocking completion/readback wait.
#[derive(Clone)]
pub struct TargetLease(Arc<Target>);
impl TargetLease {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.0.texture
    }
    pub fn view(&self) -> &wgpu::TextureView {
        &self.0.view
    }
    pub fn device_generation(&self) -> u64 {
        self.0.generation
    }
}
struct Entry {
    descriptor: TargetDescriptor,
    target: Arc<Target>,
    bytes: u64,
    used: u64,
}
struct Cache {
    entries: Vec<Entry>,
    clock: u64,
    usage: TargetPoolUsage,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            clock: 0,
            usage: TargetPoolUsage {
                byte_limit: BYTE_LIMIT,
                texture_limit: TEXTURE_LIMIT,
                ..Default::default()
            },
        }
    }
}
impl Cache {
    fn evict(&mut self) -> bool {
        let candidate = self
            .entries
            .iter()
            .enumerate()
            // Prefer idle storage, but dropping a cached owner never destroys a
            // caller-held lease or wgpu's references in submitted commands.
            .min_by_key(|(_, e)| (Arc::strong_count(&e.target) != 1, e.used))
            .map(|(i, _)| i);
        let Some(index) = candidate else {
            return false;
        };
        let entry = self.entries.swap_remove(index);
        self.usage.cached_bytes -= entry.bytes;
        self.usage.cached_textures -= 1;
        self.usage.evictions += 1;
        // Do not call texture.destroy(): queued work and outstanding leases may
        // still use this allocation after it leaves the retention cache.
        true
    }
}
#[derive(Default)]
pub(crate) struct TargetPool(Mutex<Cache>, Arc<crate::allocation_usage::AllocationUsage>);
impl TargetPool {
    pub fn usage(&self) -> TargetPoolUsage {
        let mut usage = self.0.lock().unwrap().usage;
        (
            usage.live_bytes,
            usage.live_allocations,
            usage.peak_live_bytes,
        ) = self.1.snapshot();
        usage
    }
    pub fn set_limits(&self, bytes: u64, textures: usize) {
        let mut cache = self.0.lock().unwrap();
        cache.usage.byte_limit = bytes;
        cache.usage.texture_limit = textures;
        while cache.usage.cached_bytes > bytes || cache.entries.len() > textures {
            if !cache.evict() {
                break;
            }
        }
    }
    pub fn acquire(
        &self,
        context: &GpuContext,
        descriptor: TargetDescriptor,
    ) -> Result<TargetLease> {
        let limit = context.device.limits().max_texture_dimension_2d;
        if context.is_lost()
            || descriptor.width == 0
            || descriptor.height == 0
            || descriptor.array_layers == 0
            || descriptor.array_layers > context.device.limits().max_texture_array_layers
            || descriptor.width > limit
            || descriptor.height > limit
            || !matches!(descriptor.samples, 1 | 2 | 4 | 8 | 16)
            || descriptor.mip_levels == 0
            || descriptor.mip_levels > 32 - descriptor.width.max(descriptor.height).leading_zeros()
            || (descriptor.samples > 1
                && (descriptor.mip_levels != 1 || descriptor.array_layers != 1))
            || descriptor.usage.is_empty()
        {
            return Err(RenderError("invalid or lost-device render target".into()));
        }
        let bytes = descriptor.estimated_bytes()?;
        let mut cache = self.0.lock().unwrap();
        cache.clock = cache.clock.saturating_add(1);
        let used = cache.clock;
        if let Some(index) = cache
            .entries
            .iter()
            .position(|e| e.descriptor == descriptor && Arc::strong_count(&e.target) == 1)
        {
            cache.entries[index].used = used;
            let target = cache.entries[index].target.clone();
            cache.usage.hits += 1;
            return Ok(TargetLease(target));
        }
        let retain = bytes <= cache.usage.byte_limit && cache.usage.texture_limit > 0;
        if retain {
            while cache.usage.cached_bytes + bytes > cache.usage.byte_limit
                || cache.entries.len() >= cache.usage.texture_limit
            {
                if !cache.evict() {
                    break;
                }
            }
        } else {
            // An oversized working set should not also retain old idle targets.
            while cache.evict() {}
        }
        let texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("renderkit pooled target"),
            size: wgpu::Extent3d {
                width: descriptor.width,
                height: descriptor.height,
                depth_or_array_layers: descriptor.array_layers,
            },
            mip_level_count: descriptor.mip_levels,
            sample_count: descriptor.samples,
            dimension: wgpu::TextureDimension::D2,
            format: descriptor.format,
            usage: descriptor.usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let target = Arc::new(Target {
            _charge: self.1.charge(bytes),
            texture,
            view,
            generation: context.generation(),
        });
        cache.usage.allocations += 1;
        if retain {
            cache.entries.push(Entry {
                descriptor,
                target: target.clone(),
                bytes,
                used,
            });
            cache.usage.cached_bytes += bytes;
            cache.usage.cached_textures += 1;
        } else {
            cache.usage.uncached_allocations += 1;
        }
        Ok(TargetLease(target))
    }
}
