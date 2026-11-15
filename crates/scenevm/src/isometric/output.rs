//! Injected deferred isometric presentation. Canvas buffers are GPU destinations;
//! CPU upload/composition and blocking download exist only in the legacy adapter.
use super::{Caster, Particle, PointLight, ShadowGeometry, Uniforms, gpu::Pass};
use crate::render_output::SceneOutput;
use renderkit::{
    AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, RenderError, Result,
};
use std::sync::Arc;

/// GPU-produced albedo and packed geometry, in the deferred shader's format.
/// Albedo is one packed RGBA8 word per pixel. Geometry is one `[u32; 2]` per
/// pixel, with the same encoding as the reference adapter. Producers must write
/// every visible pixel; empty geometry uses `[0, f32::INFINITY.to_bits()]`.
/// No CPU canvas or staging/readback allocation is owned here. Retain this owner
/// until all producer/consumer commands submit. Acquisition clears all planes on
/// the GPU, preserving fresh-zero storage before the producer initializes them.
pub struct IsometricCanvas {
    albedo: renderkit::WorkingBufferLease,
    surfaces: renderkit::WorkingBufferLease,
    owners: renderkit::WorkingBufferLease,
    terrain: renderkit::WorkingBufferLease,
    size: (u32, u32),
    generation: u64,
}
impl IsometricCanvas {
    pub fn new(context: &GpuContext, width: u32, height: u32) -> Result<Self> {
        let limits = context.device.limits();
        let pixels = u64::from(width) * u64::from(height);
        if context.is_lost()
            || width == 0
            || height == 0
            || width > limits.max_texture_dimension_2d
            || height > limits.max_texture_dimension_2d
            || pixels * 8 > u64::from(limits.max_storage_buffer_binding_size)
            || pixels * 8 > limits.max_buffer_size
        {
            return Err(RenderError(
                "invalid isometric canvas size or GPU domain".into(),
            ));
        }
        let make = |stride| {
            context.acquire_working_buffer(renderkit::WorkingBufferDescriptor::storage(
                pixels * stride,
            ))
        };
        let canvas = Self {
            albedo: make(4)?,
            surfaces: make(8)?,
            owners: make(4)?,
            terrain: make(8)?,
            size: (width, height),
            generation: context.generation(),
        };
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("isometric canvas initialization"),
            });
        for buffer in [
            canvas.albedo(),
            canvas.surfaces(),
            canvas.owners(),
            canvas.terrain(),
        ] {
            encoder.clear_buffer(buffer, 0, None);
        }
        context.queue.submit([encoder.finish()]);
        Ok(canvas)
    }
    /// GPU-only instance ownership for supporting-floor protection.
    pub fn owners(&self) -> &wgpu::Buffer {
        self.owners.buffer()
    }
    /// GPU ground depth/height used for terrain testing and sprite occlusion.
    pub fn terrain(&self) -> &wgpu::Buffer {
        self.terrain.buffer()
    }
    pub fn size(&self) -> (u32, u32) {
        self.size
    }
    pub fn device_generation(&self) -> u64 {
        self.generation
    }
    pub fn albedo(&self) -> &wgpu::Buffer {
        self.albedo.buffer()
    }
    pub fn surfaces(&self) -> &wgpu::Buffer {
        self.surfaces.buffer()
    }
}

/// One view's retained deferred pipelines/output, on the application's device.
/// Independent instances keep independent output leases and shadow state.
pub struct IsometricRenderer {
    context: Arc<GpuContext>,
    pass: Pass,
    image: Option<Arc<GpuImage>>,
}
impl IsometricRenderer {
    pub fn new(context: Arc<GpuContext>) -> Result<Self> {
        if context.is_lost() {
            return Err(RenderError("isometric GPU device lost".into()));
        }
        Ok(Self {
            pass: Pass::new(&context.device),
            context,
            image: None,
        })
    }
    pub fn render(
        &mut self,
        canvas: &IsometricCanvas,
        uniforms: &Uniforms,
        lights: &[PointLight],
        casters: &[Caster],
        columns: &[[f32; 2]],
        particles: &[Particle],
        geometry: Option<&ShadowGeometry>,
        revision: u64,
    ) -> Result<SceneOutput> {
        if self.context.is_lost() || canvas.generation != self.context.generation() {
            return Err(RenderError(
                "isometric canvas belongs to a lost or different GPU domain".into(),
            ));
        }
        let (width, height) = canvas.size;
        if uniforms.counts != [width, height, lights.len() as u32, casters.len() as u32]
            || lights.len() > u32::MAX as usize
            || casters.len() > u32::MAX as usize
        {
            return Err(RenderError(
                "isometric dimensions or light counts do not match the GPU canvas".into(),
            ));
        }
        self.pass
            .draw_gpu(
                &self.context,
                uniforms,
                (canvas.albedo(), canvas.surfaces()),
                lights,
                casters,
                columns,
                particles,
                geometry,
            )
            .map_err(|e| RenderError(format!("isometric GPU lighting: {e:?}")))?;
        if self
            .image
            .as_ref()
            .is_none_or(|image| (image.descriptor.width, image.descriptor.height) != canvas.size)
        {
            self.image = Some(GpuImage::from_target(
                &self.context,
                self.pass
                    .output
                    .gpu
                    .as_ref()
                    .unwrap()
                    .target_lease
                    .as_ref()
                    .ok_or_else(|| RenderError("isometric output has no allocation lease".into()))?
                    .clone(),
                ImageDescriptor {
                    width,
                    height,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    color_space: ColorSpace::Srgb,
                    alpha_mode: AlphaMode::Straight,
                },
            )?);
        }
        Ok(SceneOutput {
            image: self.image.as_ref().unwrap().clone(),
            revision,
        })
    }
    /// Diagnostic contract for acceptance tests; captures are separate operations.
    pub fn has_cpu_storage(&self) -> bool {
        !self.pass.output.data.is_empty()
    }
    pub fn has_readback_storage(&self) -> bool {
        self.pass
            .output
            .gpu
            .as_ref()
            .is_some_and(|gpu| gpu.readback.is_some())
    }
}
