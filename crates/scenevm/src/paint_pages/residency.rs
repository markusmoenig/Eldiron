//! One private producer's mutable residency table. Source keys remain immutable;
//! only atlas rectangles change after the producer's preceding receipt completes.
use super::binding::AlphaInput;
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
    pub missing: bool,
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
/// Initial validation/upload can share a live host's metadata and transfer budget.
/// The complete source backing is immutable and no partial binding is exposed.
pub struct PaintResidencyTablePreparation {
    context: Arc<GpuContext>,
    preparation: PaintPageBindingPreparation,
}
impl PaintResidencyTablePreparation {
    pub fn new_indexed(
        context: Arc<GpuContext>,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: Arc<Vec<Raster3DSurfacePaintEntry>>,
        alpha_geo_ids: Vec<GeoId>,
    ) -> Result<Self> {
        Self::new_entries(
            context,
            color,
            material,
            PaintEntryStorage::Vec(entries),
            AlphaInput::Ids(Arc::new(alpha_geo_ids)),
        )
    }
    pub fn new_indexed_alpha(
        context: Arc<GpuContext>,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: Arc<Vec<Raster3DSurfacePaintEntry>>,
        alpha: Arc<rustc_hash::FxHashSet<GeoId>>,
    ) -> Result<Self> {
        Self::new_entries(
            context,
            color,
            material,
            PaintEntryStorage::Vec(entries),
            AlphaInput::Ready(alpha),
        )
    }
    fn new_entries(
        context: Arc<GpuContext>,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: PaintEntryStorage,
        alpha: AlphaInput,
    ) -> Result<Self> {
        let preparation = PaintPageBindingPreparation::new_entries(
            &context, color, material, entries, alpha, true,
        )?;
        Ok(Self {
            context,
            preparation,
        })
    }
    pub fn advance(&mut self, units: usize) -> Result<bool> {
        self.preparation.advance(units)
    }
    pub fn upload(&mut self, max_bytes: usize) -> Result<usize> {
        self.preparation.upload(&self.context, max_bytes)
    }
    pub fn uploaded_bytes(&self) -> u64 {
        self.preparation.uploaded_bytes() as u64
    }
    pub fn upload_ready(&self) -> bool {
        self.preparation.upload_ready()
    }
    pub fn allocated_bytes(&self) -> u64 {
        self.preparation.raster_table_bytes()
    }
    pub fn finish(mut self) -> Result<PaintResidencyTable> {
        if !self.upload_ready() {
            return Err(RenderError("residency lookup upload is incomplete".into()));
        }
        let mut binding = self.preparation.finish(&self.context)?;
        let table = Arc::new(ResidentTable {
            buffer: binding.raster_entries.as_ref().unwrap().clone(),
            update: Mutex::new(TableUpdate {
                revision: 0,
                missing: binding.source_count > 0,
                rectangles: [None; 2],
            }),
        });
        let uploaded_bytes = binding.raster_table_bytes();
        Arc::get_mut(&mut binding).unwrap().resident_table = Some(table);
        Ok(PaintResidencyTable {
            context: self.context,
            binding,
            residents: HashMap::new(),
            origins: HashMap::new(),
            uploaded_bytes,
        })
    }
}
impl PaintResidencyTable {
    pub fn new(
        context: Arc<GpuContext>,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: Arc<[Raster3DSurfacePaintEntry]>,
        alpha_geo_ids: Vec<GeoId>,
    ) -> Result<Self> {
        let mut preparation = PaintResidencyTablePreparation::new_entries(
            context,
            color,
            material,
            PaintEntryStorage::Slice(entries),
            AlphaInput::Ids(Arc::new(alpha_geo_ids)),
        )?;
        preparation.advance(usize::MAX)?;
        while !preparation.upload_ready() {
            preparation.upload(64 * 1024)?;
        }
        preparation.finish()
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
        update.missing = self.residents.len() < self.binding.source_count;
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
