//! Bounded immutable source neighborhoods for authored terrain paint. Source
//! chunks remain world texels; no destination-sized image is composed here.
use super::IsometricSourceVersion;
use renderkit::{GpuContext, RenderError, Result, wgpu};
use rustc_hash::FxHashMap;
use rusterix::{
    isometric::paint::{CHUNK_SIZE, PIXELS_PER_UNIT, TerrainPaint},
    material_library::{MaterialDefinition, SEMANTIC_MATERIAL_MARKER},
};
use std::sync::Arc;
use theframework_core::Uuid;
pub struct GpuTerrainPaintSource {
    storage: renderkit::WorkingBufferLease,
    generation: u64,
    bytes: u64,
}
impl GpuTerrainPaintSource {
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
    source: Option<Arc<GpuTerrainPaintSource>>,
    used: u64,
}
pub struct IsometricPaintSources {
    context: Arc<GpuContext>,
    entries: FxHashMap<(IsometricSourceVersion, Uuid, u64, [i32; 2]), Entry>,
    budget: u64,
    limit: usize,
    bytes: u64,
    clock: u64,
    builds: u64,
}
impl IsometricPaintSources {
    pub fn new(context: Arc<GpuContext>, byte_budget: u64, entry_limit: usize) -> Self {
        Self {
            context,
            entries: FxHashMap::default(),
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
        self.bytes = 0;
    }
    /// A clipped world cell and one neighboring chunk on every side cover the
    /// reference nearest-rounded paint lookup, including negative boundaries.
    pub fn prepare(
        &mut self,
        paint: &TerrainPaint,
        map: Uuid,
        cell: [i32; 2],
        version: IsometricSourceVersion,
    ) -> Result<Option<Arc<GpuTerrainPaintSource>>> {
        if self.context.is_lost() {
            return Err(RenderError("terrain paint source GPU device lost".into()));
        }
        let center = cell.map(|v| {
            (((v as f32 + 0.5) * PIXELS_PER_UNIT).round() as i32).div_euclid(CHUNK_SIZE)
                * CHUNK_SIZE
        });
        let key = (version, map, paint.ground_key, center);
        self.clock = self.clock.wrapping_add(1);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.used = self.clock;
            return Ok(entry.source.clone());
        }
        let mut chunks = Vec::new();
        let chunk_bytes = (CHUNK_SIZE * CHUNK_SIZE * 4) as usize;
        for y in -1..=1 {
            for x in -1..=1 {
                let Some(px) = center[0].checked_add(x * CHUNK_SIZE) else {
                    continue;
                };
                let Some(py) = center[1].checked_add(y * CHUNK_SIZE) else {
                    continue;
                };
                if let Some(chunk) = paint.chunks.get(&[px, py]) {
                    if chunk.color.len() != chunk_bytes || chunk.material.len() != chunk_bytes {
                        return Err(RenderError("invalid immutable terrain paint chunk".into()));
                    }
                    chunks.push(([px, py], chunk));
                }
            }
        }
        let source = if chunks.is_empty() {
            None
        } else {
            let mut data =
                Vec::<u32>::with_capacity(4 + 256 * 4 + chunks.len() * (4 + chunk_bytes / 2));
            data.extend([
                chunks.len() as u32,
                CHUNK_SIZE as u32,
                PIXELS_PER_UNIT.to_bits(),
                u32::from(SEMANTIC_MATERIAL_MARKER),
            ]);
            for id in 0..=255 {
                data.extend(
                    MaterialDefinition::from_id(id)
                        .rmoe_values()
                        .map(f32::to_bits),
                );
            }
            let descriptors = data.len();
            data.resize(descriptors + chunks.len() * 4, 0);
            for (index, (origin, chunk)) in chunks.into_iter().enumerate() {
                let at = descriptors + index * 4;
                data[at] = origin[0] as u32;
                data[at + 1] = origin[1] as u32;
                data[at + 2] = data.len() as u32;
                for (color, material) in chunk
                    .color
                    .chunks_exact(4)
                    .zip(chunk.material.chunks_exact(4))
                {
                    data.push(u32::from_le_bytes(color.try_into().unwrap()));
                    data.push(u32::from_le_bytes(material.try_into().unwrap()));
                }
            }
            let bytes: &[u8] = bytemuck::cast_slice(&data);
            if bytes.len() as u64
                > u64::from(self.context.device.limits().max_storage_buffer_binding_size)
                    .min(self.context.device.limits().max_buffer_size)
            {
                return Err(RenderError(
                    "terrain paint source exceeds device storage limits".into(),
                ));
            }
            let storage = self.context.acquire_working_buffer(
                renderkit::WorkingBufferDescriptor::storage(bytes.len() as u64),
            )?;
            self.context.queue.write_buffer(storage.buffer(), 0, bytes);
            self.context.record_upload(bytes.len() as u64);
            self.builds += 1;
            Some(Arc::new(GpuTerrainPaintSource {
                storage,
                generation: self.context.generation(),
                bytes: bytes.len() as u64,
            }))
        };
        self.bytes += source.as_ref().map_or(0, |s| s.bytes);
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
            self.bytes -= self
                .entries
                .remove(&key)
                .unwrap()
                .source
                .map_or(0, |s| s.bytes);
        }
        Ok(source)
    }
}
