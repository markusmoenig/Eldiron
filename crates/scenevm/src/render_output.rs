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
}
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
        let mut texture = Texture::render_target(width, height);
        texture.ensure_gpu_with(&context.device);
        Ok(Self { texture, context })
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
        surface.ensure_gpu_with(&context.device);
        let gpu = GPUState {
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
        this.refresh_layer_metadata();
        this
    }
    pub fn render_gpu(
        &mut self,
        target: &mut SceneRenderTarget,
        revision: u64,
    ) -> Result<SceneOutput> {
        let gpu = self
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError("scene GPU is not initialized".into()))?;
        if gpu.device != target.context.device {
            return Err(RenderError(
                "scene and target belong to different devices".into(),
            ));
        }
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
        )
        .map_err(|e| RenderError(format!("scene rendering: {e:?}")))?;
        let texture = target
            .texture
            .gpu
            .as_ref()
            .ok_or_else(|| RenderError("scene target missing GPU texture".into()))?
            .texture
            .clone();
        let descriptor = ImageDescriptor {
            width: w,
            height: h,
            format: wgpu::TextureFormat::Rgba8Unorm,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Straight,
        };
        Ok(SceneOutput {
            image: Arc::new(GpuImage::from_texture(
                &target.context,
                texture,
                descriptor,
            )?),
            revision,
        })
    }
}
