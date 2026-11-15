//! Immutable source pages for native terrain/floor GPU sampling. Normalization
//! visits authored source texels once per revision, never a destination canvas.
use super::IsometricSourceVersion;
use renderkit::{GpuContext, RenderError, Result, wgpu};
use rustc_hash::FxHashMap;
use rusterix::Tile;
use std::sync::Arc;
use theframework_core::Uuid;

/// GPU material blob: three header words, page descriptors, then RGBA/R/M/E
/// texels. Each word is `[u32;4]`; material channels retain their original f32
/// precision until GPU blend/lighting packing. No CPU display image is retained.
pub struct GpuMaterialSource {
    storage: renderkit::WorkingBufferLease,
    generation: u64,
    bytes: u64,
}
impl GpuMaterialSource {
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.storage.buffer()
    }
    pub fn device_generation(&self) -> u64 {
        self.generation
    }
    pub fn resident_bytes(&self) -> u64 {
        self.bytes
    }
}
struct Entry {
    source: Arc<GpuMaterialSource>,
    used: u64,
}
pub struct IsometricMaterialSources {
    context: Arc<GpuContext>,
    entries: FxHashMap<(IsometricSourceVersion, Uuid, usize), Entry>,
    publications:
        super::publications::Publications<(IsometricSourceVersion, Uuid, usize), GpuMaterialSource>,
    budget: u64,
    limit: usize,
    bytes: u64,
    clock: u64,
    builds: u64,
}
impl IsometricMaterialSources {
    pub fn new(context: Arc<GpuContext>, byte_budget: u64, entry_limit: usize) -> Self {
        Self {
            context,
            entries: FxHashMap::default(),
            publications: Default::default(),
            budget: byte_budget,
            limit: entry_limit,
            bytes: 0,
            clock: 0,
            builds: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn resident_bytes(&self) -> u64 {
        self.bytes
    }
    pub fn builds(&self) -> u64 {
        self.builds
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.publications.clear();
        self.bytes = 0;
    }
    pub fn prepare(
        &mut self,
        tile: &Tile,
        frame: usize,
        version: IsometricSourceVersion,
    ) -> Result<Arc<GpuMaterialSource>> {
        if self.context.is_lost() {
            return Err(RenderError("isometric material GPU device lost".into()));
        }
        let pages = tile.uv_pages.as_ref();
        let area = pages.map(|p| u64::from(p.size[0]) * u64::from(p.size[1]));
        let count = area.unwrap_or(1).max(1);
        let limit = u64::from(self.context.device.limits().max_storage_buffer_binding_size)
            .min(self.context.device.limits().max_buffer_size)
            / 16;
        if count.checked_add(3).is_none_or(|n| n > limit) {
            return Err(RenderError(
                "isometric material page table exceeds device limits".into(),
            ));
        }
        let phase = pages.map_or_else(
            || frame % tile.textures.len().max(1),
            |p| frame % p.animation_frames.max(1) as usize,
        );
        let base = phase
            .checked_mul(area.unwrap_or(1) as usize)
            .ok_or_else(|| RenderError("isometric material phase overflow".into()))?;
        let key = (version, tile.id, base);
        self.clock = self.clock.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.used = self.clock;
            return Ok(entry.source.clone());
        }
        if let Some(source) = self.publications.get(&key) {
            return Ok(source);
        }
        let mut words = 3 + count;
        for index in 0..count as usize {
            if let Some(texture) = tile.textures.get(
                base.checked_add(index)
                    .ok_or_else(|| RenderError("isometric page index overflow".into()))?,
            ) {
                words = words
                    .checked_add(
                        (texture.width as u64)
                            .checked_mul(texture.height as u64)
                            .ok_or_else(|| {
                                RenderError("isometric material dimensions overflow".into())
                            })?,
                    )
                    .filter(|n| *n <= limit)
                    .ok_or_else(|| {
                        RenderError("isometric material source exceeds device limits".into())
                    })?;
            }
        }
        let mut data = vec![[0u32; 4]; 3 + count as usize];
        data.reserve((words - 3 - count) as usize);
        data[0] = [count as u32, 1, 1, 0];
        if let Some(p) = pages.filter(|p| p.size.iter().all(|v| *v > 0)) {
            let periodic = p.repeat_period.iter().all(|v| v.is_finite() && *v > 0.);
            data[0] = [
                count as u32,
                p.size[0],
                p.size[1],
                1 | if periodic { 2 } else { 0 },
            ];
            data[1] = [
                p.origin[0] as u32,
                p.origin[1] as u32,
                p.units_per_page.max(1),
                0,
            ];
            if periodic {
                data[2] = [
                    p.repeat_period[0].to_bits(),
                    p.repeat_period[1].to_bits(),
                    0,
                    0,
                ];
            }
        }
        for page in 0..count as usize {
            let index = base + page;
            let Some(texture) = tile.textures.get(index) else {
                continue;
            };
            let width = u32::try_from(texture.width)
                .map_err(|_| RenderError("isometric source width overflow".into()))?;
            let height = u32::try_from(texture.height)
                .map_err(|_| RenderError("isometric source height overflow".into()))?;
            data[3 + page] = [data.len() as u32, width, height, 0];
            for y in 0..height {
                for x in 0..width {
                    let (rgba, material) =
                        tile.source_surface_texel(index, x, y).ok_or_else(|| {
                            RenderError("invalid isometric material source texel".into())
                        })?;
                    data.push([
                        u32::from_le_bytes(rgba),
                        material[0].to_bits(),
                        material[1].to_bits(),
                        material[3].to_bits(),
                    ]);
                }
            }
        }
        let raw: &[u8] = bytemuck::cast_slice(&data);
        let bytes = raw.len() as u64;
        let storage = self
            .context
            .acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(bytes))?;
        self.context.queue.write_buffer(storage.buffer(), 0, raw);
        self.context.record_upload(bytes);
        let source = Arc::new(GpuMaterialSource {
            storage,
            generation: self.context.generation(),
            bytes,
        });
        self.publications.publish(key, &source);
        self.builds += 1;
        self.bytes += bytes;
        self.entries.insert(
            key,
            Entry {
                source: source.clone(),
                used: self.clock,
            },
        );
        while self.bytes > self.budget || self.entries.len() > self.limit {
            let Some(key) = self
                .entries
                .iter()
                .min_by_key(|(_, e)| e.used)
                .map(|(k, _)| *k)
            else {
                break;
            };
            self.bytes -= self.entries.remove(&key).unwrap().source.resident_bytes();
        }
        Ok(source)
    }
}
