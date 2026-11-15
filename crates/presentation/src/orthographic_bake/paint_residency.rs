//! Private producer residency. One fault loads at most one original page; a
//! dispatch pins the dependencies it discovers until it succeeds or subdivides.
use super::*;
use scenevm::paint_pages::{PaintPageBinding, PaintResidencyTable, PaintResidencyTablePreparation};
use shared::surface_paint::pages::PaintPageIndex;
use std::collections::{HashMap, HashSet};

const PAGE: u32 = 64;
const PAGE_BYTES: u64 = 64 * 64 * 4 * 2;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintPageRequest {
    Pending,
    Resident,
    Loaded,
    WorkingSetFull,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintResidencyUsage {
    pub gpu_plane_bytes: u64,
    pub capacity: usize,
    pub resident: usize,
    pub pinned: usize,
    pub page_uploads: u64,
    pub uploaded_bytes: u64,
    pub gpu_table_bytes: u64,
    pub table_uploaded_bytes: u64,
}
pub struct PaintPageResidency {
    context: Arc<GpuContext>,
    source: Arc<PaintPageIndex>,
    color: Arc<GpuImage>,
    material: Arc<GpuImage>,
    size: [u32; 2],
    slots: Vec<Option<u32>>,
    residents: HashMap<u32, usize>,
    pinned: HashSet<u32>,
    uploads: u64,
    table: PaintResidencyTable,
}
/// Cooperative initial key validation/upload. Original pixels remain shared and
/// untouched until a requesting private frame reports a missing source slot.
pub struct PaintPageResidencyPreparation {
    context: Arc<GpuContext>,
    source: Arc<PaintPageIndex>,
    color: Arc<GpuImage>,
    material: Arc<GpuImage>,
    size: [u32; 2],
    table: PaintResidencyTablePreparation,
}
impl PaintPageResidencyPreparation {
    pub fn new(
        context: Arc<GpuContext>,
        source: Arc<PaintPageIndex>,
        plane_byte_limit: u64,
    ) -> Result<Self> {
        let dimension = context.device.limits().max_texture_dimension_2d / PAGE;
        let max_capacity = (plane_byte_limit / PAGE_BYTES).min(u64::from(dimension).pow(2)) as u32;
        let desired = max_capacity.min(source.source_count().clamp(1, u32::MAX as usize) as u32);
        if context.is_lost()
            || desired == 0
            || source.table_bytes() + 20
                > u64::from(context.device.limits().max_storage_buffer_binding_size)
        {
            return Err(RenderError(
                "paint page residency exceeds device/budget limits".into(),
            ));
        }
        let columns = desired.min(dimension);
        // Padding may use spare budget, but cannot round the allocation above
        // the caller's cap. This also fits all sources when spare rows permit.
        let rows = desired.div_ceil(columns).min(max_capacity / columns).max(1);
        let size = [columns * PAGE, rows * PAGE];
        let descriptor = |format| ImageDescriptor {
            width: size[0],
            height: size[1],
            format,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        };
        let color = GpuImage::new(&context, descriptor(wgpu::TextureFormat::Rgba8Unorm))?;
        let material = GpuImage::new(&context, descriptor(wgpu::TextureFormat::Rgba8Uint))?;
        let table = PaintResidencyTablePreparation::new_indexed(
            context.clone(),
            color.clone(),
            material.clone(),
            source.unloaded_entries_shared(),
            source.alpha_geo_ids().to_vec(),
        )?;
        Ok(Self {
            table,
            context,
            source,
            color,
            material,
            size,
        })
    }
    pub fn advance(&mut self, units: usize) -> Result<bool> {
        self.table.advance(units)
    }
    pub fn upload(&mut self, max_bytes: usize) -> Result<usize> {
        self.table.upload(max_bytes)
    }
    pub fn ready(&self) -> bool {
        self.table.upload_ready()
    }
    pub fn usage(&self) -> PaintResidencyUsage {
        PaintResidencyUsage {
            gpu_plane_bytes: u64::from(self.size[0]) * u64::from(self.size[1]) * 8,
            capacity: (self.size[0] / PAGE * (self.size[1] / PAGE)) as usize,
            resident: 0,
            pinned: 0,
            page_uploads: 0,
            uploaded_bytes: 0,
            gpu_table_bytes: self.table.allocated_bytes(),
            table_uploaded_bytes: self.table.uploaded_bytes(),
        }
    }
    pub fn finish(self) -> Result<PaintPageResidency> {
        let table = self.table.finish()?;
        Ok(PaintPageResidency {
            context: self.context,
            source: self.source,
            color: self.color,
            material: self.material,
            size: self.size,
            table,
            slots: vec![None; (self.size[0] / PAGE * (self.size[1] / PAGE)) as usize],
            residents: HashMap::new(),
            pinned: HashSet::new(),
            uploads: 0,
        })
    }
}
impl PaintPageResidency {
    /// Explicit private/bake callers may drain the same preparation kernel.
    /// Live callers advance PaintPageResidencyPreparation under their frame budget.
    pub fn new(
        context: Arc<GpuContext>,
        source: Arc<PaintPageIndex>,
        plane_byte_limit: u64,
    ) -> Result<Self> {
        let mut preparation =
            PaintPageResidencyPreparation::new(context, source, plane_byte_limit)?;
        preparation.advance(usize::MAX)?;
        while !preparation.ready() {
            preparation.upload(64 * 1024)?;
        }
        preparation.finish()
    }
    pub fn begin_dispatch(&mut self) {
        self.pinned.clear();
    }
    /// Call only after the previous dependency receipt completes, so queued
    /// consumers have finished reading any atlas slot about to be replaced.
    pub fn request(&mut self, source_slot: u32) -> Result<PaintPageRequest> {
        if self.context.is_lost() {
            return Err(RenderError("paint residency device lost".into()));
        }
        if self.source.page(source_slot).is_none() {
            return Err(RenderError(
                "paint dependency names an absent source key".into(),
            ));
        }
        if self.residents.contains_key(&source_slot) {
            self.pinned.insert(source_slot);
            return Ok(PaintPageRequest::Resident);
        }
        let Some(slot) = self.slots.iter().position(Option::is_none).or_else(|| {
            self.slots
                .iter()
                .position(|old| old.is_some_and(|old| !self.pinned.contains(&old)))
        }) else {
            return Ok(PaintPageRequest::WorkingSetFull);
        };
        let Some(page) = self.source.poll_pixels(source_slot).map_err(RenderError)? else {
            return Ok(PaintPageRequest::Pending);
        };
        let columns = self.size[0] / PAGE;
        let atlas_origin = [slot as u32 % columns * PAGE, slot as u32 / columns * PAGE];
        self.table
            .replace(self.slots[slot], source_slot, atlas_origin)?;
        let origin = wgpu::Origin3d {
            x: slot as u32 % columns * PAGE,
            y: slot as u32 / columns * PAGE,
            z: 0,
        };
        for (image, bytes) in [(&self.color, &page.color), (&self.material, &page.material)] {
            self.context.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &image.texture,
                    mip_level: 0,
                    origin,
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(PAGE * 4),
                    rows_per_image: Some(PAGE),
                },
                wgpu::Extent3d {
                    width: PAGE,
                    height: PAGE,
                    depth_or_array_layers: 1,
                },
            );
        }
        self.context.record_upload(PAGE_BYTES);
        if let Some(old) = self.slots[slot].replace(source_slot) {
            self.residents.remove(&old);
        }
        self.residents.insert(source_slot, slot);
        self.pinned.insert(source_slot);
        self.uploads += 1;
        Ok(PaintPageRequest::Loaded)
    }
    pub fn binding(&self) -> Result<Arc<PaintPageBinding>> {
        self.table.binding()
    }

    pub fn usage(&self) -> PaintResidencyUsage {
        PaintResidencyUsage {
            gpu_plane_bytes: self.slots.len() as u64 * PAGE_BYTES,
            capacity: self.slots.len(),
            resident: self.residents.len(),
            pinned: self.pinned.len(),
            page_uploads: self.uploads,
            uploaded_bytes: self.uploads * PAGE_BYTES,
            gpu_table_bytes: self.table.allocated_bytes(),
            table_uploaded_bytes: self.table.uploaded_bytes(),
        }
    }
}

#[cfg(test)]
mod tests;
