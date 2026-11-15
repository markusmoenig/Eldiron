//! Validated paint image/table ownership shared by independent scene views.
use crate::{GeoId, Raster3DSurfacePaintEntry};
use renderkit::{GpuContext, GpuImage, RenderError, Result, wgpu};
use std::sync::Arc;

pub struct PaintPageBinding {
    pub(crate) color: Arc<GpuImage>,
    pub(crate) material: Arc<GpuImage>,
    pub(crate) entries: Arc<[Raster3DSurfacePaintEntry]>,
    pub(crate) alpha_geo_ids: Vec<GeoId>,
    pub(crate) has_missing: bool,
    pub(crate) raster_entries: Option<wgpu::Buffer>,
}
/// Private table validation for cooperative live producers. The images/table
/// remain immutable for this preparation; finish rechecks its device domain.
pub struct PaintPageBindingPreparation {
    binding: Option<PaintPageBinding>,
    cursor: usize,
    failure: Option<String>,
    uploaded_bytes: usize,
}
impl PaintPageBindingPreparation {
    pub fn new(
        context: &GpuContext,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: Arc<[Raster3DSurfacePaintEntry]>,
        alpha_geo_ids: Vec<GeoId>,
    ) -> Result<Self> {
        if context.is_lost()
            || color.device_generation() != context.generation()
            || material.device_generation() != context.generation()
            || color.descriptor.format != wgpu::TextureFormat::Rgba8Unorm
            || material.descriptor.format != wgpu::TextureFormat::Rgba8Uint
            || color.descriptor.width != material.descriptor.width
            || color.descriptor.height != material.descriptor.height
            || ![&color, &material].iter().all(|image| {
                image
                    .texture
                    .usage()
                    .contains(wgpu::TextureUsages::TEXTURE_BINDING)
                    && image.texture.sample_count() == 1
                    && image.texture.depth_or_array_layers() == 1
                    && image.texture.dimension() == wgpu::TextureDimension::D2
            })
            || entries.len() < 2
            || !entries.len().is_power_of_two()
            || (size_of_val(entries.as_ref()) as u64 + 20)
                > u64::from(context.device.limits().max_storage_buffer_binding_size)
            || size_of_val(entries.as_ref()) as u64 > context.device.limits().max_buffer_size
        {
            return Err(RenderError(
                "invalid paint page binding/device/table".into(),
            ));
        }
        Ok(Self {
            binding: Some(PaintPageBinding {
                color,
                material,
                entries,
                alpha_geo_ids,
                has_missing: false,
                raster_entries: None,
            }),
            cursor: 0,
            failure: None,
            uploaded_bytes: 0,
        })
    }
    /// Check at most `units` table slots, including unused slots.
    pub fn advance(&mut self, units: usize) -> Result<bool> {
        if let Some(error) = &self.failure {
            return Err(RenderError(error.clone()));
        }
        let binding = self
            .binding
            .as_mut()
            .ok_or_else(|| RenderError("paint binding preparation was consumed".into()))?;
        let end = self.cursor.saturating_add(units).min(binding.entries.len());
        for entry in &binding.entries[self.cursor..end] {
            if entry.uv_size == [0; 2] {
                continue;
            }
            let error = if entry.uv_size != [64; 2] {
                Some("invalid paint page UV dimensions")
            } else if entry.atlas_rect[2..] == [0, 0] {
                binding.has_missing = true;
                None
            } else if entry.atlas_rect[2..] != [64, 64]
                || !entry.atlas_rect[0]
                    .checked_add(64)
                    .is_some_and(|end| end <= binding.color.descriptor.width)
                || !entry.atlas_rect[1]
                    .checked_add(64)
                    .is_some_and(|end| end <= binding.color.descriptor.height)
            {
                Some("paint page rectangle exceeds its texture")
            } else {
                None
            };
            if let Some(error) = error {
                self.failure = Some(error.into());
                return Err(RenderError(error.into()));
            }
        }
        self.cursor = end;
        Ok(self.cursor == binding.entries.len())
    }
    /// Live producers preupload one shared raster lookup buffer. Explicit
    /// reference/compute producers may finish without it and keep their policy.
    pub fn upload(&mut self, context: &GpuContext, max_bytes: usize) -> Result<usize> {
        if let Some(error) = &self.failure {
            return Err(RenderError(error.clone()));
        }
        let binding = self
            .binding
            .as_mut()
            .ok_or_else(|| RenderError("paint binding preparation was consumed".into()))?;
        if context.is_lost()
            || binding.color.device_generation() != context.generation()
            || binding.material.device_generation() != context.generation()
            || self.cursor != binding.entries.len()
        {
            return Err(RenderError(
                "paint binding validation is incomplete or its device was replaced".into(),
            ));
        }
        let bytes: &[u8] = bytemuck::cast_slice(binding.entries.as_ref());
        let count = (bytes.len() - self.uploaded_bytes)
            .min(max_bytes)
            .min(64 * 1024)
            & !3;
        if count == 0 {
            return Ok(0);
        }
        let buffer = binding.raster_entries.get_or_insert_with(|| {
            context.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shared-raster-paint-lookup"),
                size: bytes.len() as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        });
        context.queue.write_buffer(
            buffer,
            self.uploaded_bytes as u64,
            &bytes[self.uploaded_bytes..self.uploaded_bytes + count],
        );
        context.record_upload(count as u64);
        self.uploaded_bytes += count;
        Ok(count)
    }
    pub fn upload_ready(&self) -> bool {
        self.binding.as_ref().is_some_and(|binding| {
            binding.raster_entries.is_some()
                && self.uploaded_bytes == size_of_val(binding.entries.as_ref())
        })
    }
    pub fn raster_table_bytes(&self) -> u64 {
        self.binding
            .as_ref()
            .map_or(0, |binding| binding.raster_table_bytes())
    }
    pub fn finish(&mut self, context: &GpuContext) -> Result<Arc<PaintPageBinding>> {
        if let Some(error) = &self.failure {
            return Err(RenderError(error.clone()));
        }
        let binding = self
            .binding
            .as_ref()
            .ok_or_else(|| RenderError("paint binding preparation was consumed".into()))?;
        if context.is_lost()
            || binding.color.device_generation() != context.generation()
            || binding.material.device_generation() != context.generation()
            || self.cursor != binding.entries.len()
            || (binding.raster_entries.is_some()
                && self.uploaded_bytes != size_of_val(binding.entries.as_ref()))
        {
            return Err(RenderError(
                "paint binding validation is incomplete or its device was replaced".into(),
            ));
        }
        Ok(Arc::new(self.binding.take().unwrap()))
    }
}
impl PaintPageBinding {
    /// Copy unchanged source planes to a private replacement on this device.
    /// This queues GPU work and performs no CPU image transfer or readback.
    pub fn copy_planes_into(
        &self,
        context: &GpuContext,
        color: &GpuImage,
        material: &GpuImage,
    ) -> Result<bool> {
        if context.is_lost()
            || [&*self.color, &*self.material, color, material]
                .iter()
                .any(|image| image.device_generation() != context.generation())
        {
            return Err(RenderError(
                "paint plane copy belongs to a lost or different device".into(),
            ));
        }
        let pairs = [(&*self.color, color), (&*self.material, material)];
        if pairs.iter().any(|(from, to)| {
            from.texture == to.texture
                || from.descriptor.width != to.descriptor.width
                || from.descriptor.height != to.descriptor.height
                || from.descriptor.format != to.descriptor.format
                || !from.texture.usage().contains(wgpu::TextureUsages::COPY_SRC)
                || !to.texture.usage().contains(wgpu::TextureUsages::COPY_DST)
        }) {
            return Ok(false);
        }
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("paint replacement plane copy"),
            });
        for (from, to) in pairs {
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &from.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &to.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: from.descriptor.width,
                    height: from.descriptor.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        context.queue.submit(Some(encoder.finish()));
        Ok(true)
    }
    pub fn raster_table_bytes(&self) -> u64 {
        if self.raster_entries.is_some() {
            size_of_val(self.entries.as_ref()) as u64
        } else {
            0
        }
    }
    /// Submit consumers before the producer rewrites any resident texture slot.
    /// Retaining this binding preserves allocation lifetime, not a pixel copy.
    pub fn new(
        context: &GpuContext,
        color: Arc<GpuImage>,
        material: Arc<GpuImage>,
        entries: Arc<[Raster3DSurfacePaintEntry]>,
        alpha_geo_ids: Vec<GeoId>,
    ) -> Result<Arc<Self>> {
        let mut preparation =
            PaintPageBindingPreparation::new(context, color, material, entries, alpha_geo_ids)?;
        preparation.advance(usize::MAX)?;
        preparation.finish(context)
    }
}
