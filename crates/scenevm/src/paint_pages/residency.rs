//! One private producer's mutable residency table. Source keys remain immutable;
//! only atlas rectangles change after the producer's preceding receipt completes.
use super::*;
use crate::{GeoId, Raster3DSurfacePaintEntry};
use renderkit::{GpuImage, RenderError};
use std::{collections::HashMap, sync::Mutex};

pub(crate) struct ResidentTable {
    pub buffer: wgpu::Buffer,
    pub update: Mutex<TableUpdate>,
}
pub(crate) struct TableUpdate {
    pub revision: u64,
    pub rectangles: [Option<u32>; 2],
}
/// A binding retains allocation lifetime, not a snapshot of resident pixels or
/// rectangles. The producer must wait for all queued consumers before replace().
/// Keep this owner private to one replay operation; ordinary shared live sources
/// use immutable PaintPageBindingPreparation instead.
pub struct PaintResidencyTable {
    context: Arc<GpuContext>,
    binding: Arc<PaintPageBinding>,
    residents: HashMap<u32, [u32; 2]>,
    origins: HashMap<[u32; 2], u32>,
    uploaded_bytes: u64,
}
impl PaintResidencyTable {
    pub fn new(
        context: Arc<GpuContext>,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: Arc<[Raster3DSurfacePaintEntry]>,
        alpha_geo_ids: Vec<GeoId>,
    ) -> Result<Self> {
        // Validate once. All later changes are at most two rectangle writes;
        // neither source keys nor the full CPU table are cloned per fault.
        let mut preparation =
            PaintPageBindingPreparation::new(&context, color, material, entries, alpha_geo_ids)?;
        preparation.advance(usize::MAX)?;
        let mut binding = preparation.finish(&context)?;
        if binding
            .entries
            .iter()
            .any(|entry| entry.uv_size != [0; 2] && entry.atlas_rect != [0; 4])
        {
            return Err(RenderError(
                "residency table must start with unloaded source keys".into(),
            ));
        }
        use wgpu::util::DeviceExt;
        let bytes = bytemuck::cast_slice(binding.entries.as_ref());
        let table = Arc::new(ResidentTable {
            buffer: context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("private paint residency lookup"),
                    contents: bytes,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
                }),
            update: Mutex::new(TableUpdate {
                revision: 0,
                rectangles: [None; 2],
            }),
        });
        let uploaded_bytes = bytes.len() as u64;
        context.record_upload(uploaded_bytes);
        let value = Arc::get_mut(&mut binding).unwrap();
        value.raster_entries = Some(table.buffer.clone());
        value.resident_table = Some(table);
        Ok(Self {
            context,
            binding,
            residents: HashMap::new(),
            origins: HashMap::new(),
            uploaded_bytes,
        })
    }
    pub fn binding(&self) -> Result<Arc<PaintPageBinding>> {
        if self.context.is_lost() {
            return Err(RenderError("paint residency table device was lost".into()));
        }
        Ok(self.binding.clone())
    }
    /// Replace a physical slot after its preceding dependency receipt completes.
    /// Validates the entire change before queuing either rectangle write.
    pub fn replace(&mut self, evicted: Option<u32>, source: u32, origin: [u32; 2]) -> Result<()> {
        let valid_source = self
            .binding
            .entries
            .get(source as usize)
            .is_some_and(|entry| entry.uv_size == [64; 2]);
        if self.context.is_lost()
            || !valid_source
            || evicted.is_some_and(|slot| !self.residents.contains_key(&slot))
            || self.residents.contains_key(&source)
            || self
                .origins
                .get(&origin)
                .is_some_and(|slot| Some(*slot) != evicted)
            || !(0..2).all(|axis| {
                origin[axis] % 64 == 0
                    && origin[axis].checked_add(64).is_some_and(|end| {
                        end <= [
                            self.binding.color.descriptor.width,
                            self.binding.color.descriptor.height,
                        ][axis]
                    })
            })
        {
            return Err(RenderError(
                "invalid paint residency rectangle replacement".into(),
            ));
        }
        let table = self.binding.resident_table.as_ref().unwrap();
        let mut update = table
            .update
            .lock()
            .map_err(|_| RenderError("paint residency update was interrupted".into()))?;
        let revision = update
            .revision
            .checked_add(1)
            .ok_or_else(|| RenderError("paint residency revision exhausted".into()))?;
        let write = |slot: u32, rectangle: [u32; 4]| {
            self.context.queue.write_buffer(
                &table.buffer,
                u64::from(slot) * size_of::<Raster3DSurfacePaintEntry>() as u64
                    + std::mem::offset_of!(Raster3DSurfacePaintEntry, atlas_rect) as u64,
                bytemuck::cast_slice(&rectangle),
            );
            self.context.record_upload(16);
        };
        if let Some(evicted) = evicted {
            write(evicted, [0; 4]);
            self.origins
                .remove(&self.residents.remove(&evicted).unwrap());
            self.uploaded_bytes += 16;
        }
        write(source, [origin[0], origin[1], 64, 64]);
        self.residents.insert(source, origin);
        self.origins.insert(origin, source);
        self.uploaded_bytes += 16;
        update.revision = revision;
        update.rectangles = [evicted, Some(source)];
        Ok(())
    }
    pub fn allocated_bytes(&self) -> u64 {
        self.binding.raster_table_bytes()
    }
    pub fn uploaded_bytes(&self) -> u64 {
        self.uploaded_bytes
    }
}
