//! Window-independent rendering with explicitly injected GPU ownership.
use crate::{GPUState, SceneVM, SharedAtlas, Texture, VM};
use renderkit::{
    AlphaMode, ColorSpace, GpuContext, GpuImage, ImageDescriptor, RenderError, Result,
};
use std::sync::Arc;

/// One retained view target. Keep separate SceneVM/view state for independent
/// cameras/temporal history; share assets through new_with_context_and_atlas.
pub struct SceneRenderTarget {
    texture: Texture,
    context: Arc<GpuContext>,
    image: Arc<GpuImage>,
}
/// Retains allocation lifetime, not a snapshot of a still-active view. Submit
/// sampling/capture before rendering that view again, and retain the image until
/// that submission; retiring its owner cannot recycle a leased allocation.
#[derive(Clone)]
pub struct SceneOutput {
    pub image: Arc<GpuImage>,
    pub revision: u64,
}
impl SceneRenderTarget {
    pub fn new(context: Arc<GpuContext>, width: u32, height: u32) -> Result<Self> {
        if width == 0
            || height == 0
            || width > context.device.limits().max_texture_dimension_2d
            || height > context.device.limits().max_texture_dimension_2d
        {
            return Err(RenderError("invalid scene target size".into()));
        }
        let texture = Texture::pooled_render_target(&context, width, height)?;
        let image = GpuImage::from_target(
            &context,
            texture
                .gpu
                .as_ref()
                .unwrap()
                .target_lease
                .as_ref()
                .unwrap()
                .clone(),
            ImageDescriptor {
                width,
                height,
                format: wgpu::TextureFormat::Rgba8Unorm,
                color_space: ColorSpace::Srgb,
                alpha_mode: AlphaMode::Straight,
            },
        )?;
        Ok(Self {
            texture,
            context,
            image,
        })
    }
    pub fn image(&self) -> Arc<GpuImage> {
        self.image.clone()
    }
    pub fn size(&self) -> (u32, u32) {
        (self.texture.width, self.texture.height)
    }
    pub fn has_cpu_storage(&self) -> bool {
        !self.texture.data.is_empty()
    }
    pub fn has_readback_storage(&self) -> bool {
        self.texture
            .gpu
            .as_ref()
            .is_some_and(|g| g.readback.is_some())
    }
}
impl SceneVM {
    /// Lease original layer channels before window composition. Material data
    /// uses all RGBA bytes even when A is zero, so it must bypass alpha blending.
    /// Capture immediately before a later render can reuse this layer target.
    pub fn layer_output(
        &self,
        context: &GpuContext,
        layer: usize,
        revision: u64,
    ) -> Result<Option<SceneOutput>> {
        if context.is_lost() || self.device_generation() != Some(context.generation()) {
            return Err(RenderError(
                "layer output belongs to a lost or different device".into(),
            ));
        }
        let Some(texture) = self
            .vm_ref_by_index(layer)
            .and_then(|vm| vm.composite_texture())
        else {
            return Ok(None);
        };
        let Some(gpu) = &texture.gpu else {
            return Ok(None);
        };
        let descriptor = ImageDescriptor {
            width: texture.width,
            height: texture.height,
            format: gpu.texture.format(),
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        };
        let image = if let Some(lease) = &gpu.target_lease {
            GpuImage::from_target(context, lease.clone(), descriptor)?
        } else {
            Arc::new(GpuImage::from_texture(
                context,
                gpu.texture.clone(),
                descriptor,
            )?)
        };
        Ok(Some(SceneOutput { image, revision }))
    }
    pub fn shared_atlas(&self) -> SharedAtlas {
        self.atlas.clone()
    }
    /// CPU storage retained by live layer/history targets (not source assets).
    pub fn render_target_cpu_bytes(&self) -> usize {
        self.vm.render_target_cpu_bytes()
            + self
                .overlay_vms
                .iter()
                .map(|v| v.render_target_cpu_bytes())
                .sum::<usize>()
    }

    /// Construct a renderer on the host's device, without creating a window or
    /// consulting the compatibility global GPU singleton.
    pub fn new_with_context(context: Arc<GpuContext>, width: u32, height: u32) -> Self {
        Self::new_with_context_and_atlas(context, width, height, SharedAtlas::new(4096, 4096))
    }
    pub fn new_with_context_and_atlas(
        context: Arc<GpuContext>,
        width: u32,
        height: u32,
        atlas: SharedAtlas,
    ) -> Self {
        let mut surface = Texture::render_target(width.max(1), height.max(1));
        surface.ensure_gpu_in_domain(&context.device, context.generation());
        let gpu = GPUState {
            context_generation: Some(context.generation()),
            _instance: context.instance.clone(),
            _adapter: context.adapter.clone(),
            device: context.device.clone(),
            queue: context.queue.clone(),
            surface,
            #[cfg(not(target_arch = "wasm32"))]
            window_surface: None,
        };
        let mut this = Self {
            size: (width.max(1), height.max(1)),
            gpu: Some(gpu),
            #[cfg(target_arch = "wasm32")]
            needs_gpu_init: false,
            #[cfg(target_arch = "wasm32")]
            init_in_flight: false,
            atlas: atlas.clone(),
            vm: VM::new_with_shared_atlas(atlas),
            overlay_vms: vec![],
            active_vm_index: 0,
            log_layer_activity: false,
            compositing_pipeline: None,
            rgba_overlay_pipeline: None,
            rgba_overlay: None,
            isometric_pass: None,
        };
        this.vm.gpu_domain = Some(context.generation());
        this.vm.target_context = Some(context);
        this.refresh_layer_metadata();
        this
    }
    /// Diagnostic for the remaining explicit reference adapter. Querying this
    /// never initializes a GPU; canonical application startup must leave it cold.
    pub fn compatibility_gpu_initialized() -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::GLOBAL_GPU.get().is_some()
        }
        #[cfg(target_arch = "wasm32")]
        {
            crate::GLOBAL_GPU_WASM.with(|state| state.borrow().is_some())
        }
    }

    pub fn device_generation(&self) -> Option<u64> {
        self.gpu.as_ref().and_then(|gpu| gpu.context_generation)
    }

    /// Rebuild derived GPU state on a replacement host device while retaining
    /// the prepared CPU scene and view policy. No compatibility device lookup.
    pub fn rebind_context(&mut self, context: Arc<GpuContext>) {
        let generation = context.generation();
        if self.device_generation() == Some(generation) {
            return;
        }
        let mut surface = Texture::render_target(self.size.0.max(1), self.size.1.max(1));
        surface.ensure_gpu_in_domain(&context.device, generation);
        self.vm.reset_gpu_domain(generation);
        self.vm.target_context = Some(context.clone());
        for vm in &mut self.overlay_vms {
            vm.reset_gpu_domain(generation);
            vm.target_context = Some(context.clone());
        }
        self.compositing_pipeline = None;
        self.rgba_overlay_pipeline = None;
        if let Some(overlay) = self.rgba_overlay.as_mut() {
            overlay.texture.gpu = None;
            overlay.dirty = true;
        }
        self.isometric_pass = None;
        self.gpu = Some(GPUState {
            context_generation: Some(generation),
            _instance: context.instance.clone(),
            _adapter: context.adapter.clone(),
            device: context.device.clone(),
            queue: context.queue.clone(),
            surface,
            #[cfg(not(target_arch = "wasm32"))]
            window_surface: None,
        });
        #[cfg(target_arch = "wasm32")]
        {
            self.needs_gpu_init = false;
            self.init_in_flight = false;
        }
    }

    pub fn render_gpu(
        &mut self,
        target: &mut SceneRenderTarget,
        revision: u64,
    ) -> Result<SceneOutput> {
        self.render_gpu_base_state(target, revision, false)
    }

    /// Render only the private base attempt. Overlay history and final target
    /// composition wait until the requesting owner has resolved every region.
    pub fn render_base_gpu(&mut self, target: &mut SceneRenderTarget) -> Result<()> {
        self.validate_render_target(target)?;
        let gpu = self.gpu.as_ref().unwrap();
        let (width, height) = target.size();
        self.size = (width, height);
        self.vm
            .draw_into(&gpu.device, &gpu.queue, &mut target.texture, width, height)
            .map_err(|error| RenderError(format!("private scene base: {error:?}")))
    }

    /// Compose an already completed base and draw overlays exactly once. The
    /// caller must have rendered the unchanged base at this exact target size.
    pub fn compose_prepared_base_gpu(
        &mut self,
        target: &mut SceneRenderTarget,
        revision: u64,
    ) -> Result<SceneOutput> {
        self.validate_render_target(target)?;
        let texture = self
            .vm
            .composite_texture()
            .ok_or_else(|| RenderError("prepared scene base is missing".into()))?;
        if (texture.width, texture.height) != target.size() || texture.gpu.is_none() {
            return Err(RenderError("prepared scene base target is stale".into()));
        }
        self.render_gpu_base_state(target, revision, true)
    }

    fn validate_render_target(&self, target: &SceneRenderTarget) -> Result<()> {
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError("scene GPU is not initialized".into()))?;
        if gpu.context_generation != Some(target.context.generation()) {
            return Err(RenderError(
                "scene and target belong to different devices".into(),
            ));
        }
        if self.vm.gpu_domain != gpu.context_generation
            || self
                .overlay_vms
                .iter()
                .any(|v| v.gpu_domain != gpu.context_generation)
        {
            return Err(RenderError(
                "scene contains renderer state from a different GPU domain".into(),
            ));
        }
        if target.context.is_lost() {
            return Err(RenderError("scene GPU device lost".into()));
        }
        Ok(())
    }

    fn render_gpu_base_state(
        &mut self,
        target: &mut SceneRenderTarget,
        revision: u64,
        base_prepared: bool,
    ) -> Result<SceneOutput> {
        self.validate_render_target(target)?;
        let gpu = self.gpu.as_ref().unwrap();
        let (w, h) = target.size();
        self.size = (w, h);
        Self::draw_all_vms(
            &mut self.vm,
            &mut self.overlay_vms,
            &gpu.device,
            &gpu.queue,
            &mut target.texture,
            w,
            h,
            self.log_layer_activity,
            &mut self.compositing_pipeline,
            &mut self.rgba_overlay,
            &mut self.rgba_overlay_pipeline,
            true,
            base_prepared,
        )
        .map_err(|e| RenderError(format!("scene rendering: {e:?}")))?;
        Ok(SceneOutput {
            image: target.image(),
            revision,
        })
    }
}
