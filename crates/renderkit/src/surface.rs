use crate::{GpuContext, RenderError, Result};
use std::sync::Arc;

/// Host-owned presentation target. Drop it before destroying a foreign native layer.
pub struct SurfaceState<'a> {
    surface: wgpu::Surface<'a>,
    pub context: Arc<GpuContext>,
    config: wgpu::SurfaceConfiguration,
    physical_size: (u32, u32),
    suspended: bool,
}
pub enum AcquireFrame {
    Ready(wgpu::SurfaceTexture),
    Deferred,
    Reconfigure,
    Lost,
}
impl<'a> SurfaceState<'a> {
    /// A Rust window, owned Arc<Window>, or browser canvas can implement this target.
    pub async fn from_target(
        target: impl Into<wgpu::SurfaceTarget<'a>>,
        size: (u32, u32),
    ) -> Result<Self> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(target)
            .map_err(|e| RenderError(format!("create surface: {e}")))?;
        let context = GpuContext::request(instance, Some(&surface)).await?;
        Self::with_context(context, surface, size)
    }
    pub fn with_context(
        context: Arc<GpuContext>,
        surface: wgpu::Surface<'a>,
        size: (u32, u32),
    ) -> Result<Self> {
        let caps = surface.get_capabilities(&context.adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or(caps.formats.first().copied())
            .ok_or_else(|| RenderError("surface has no compatible formats".into()))?;
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Fifo) {
            wgpu::PresentMode::Fifo
        } else {
            *caps
                .present_modes
                .first()
                .ok_or_else(|| RenderError("surface has no present modes".into()))?
        };
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        let mut state = Self {
            surface,
            context,
            config,
            physical_size: size,
            suspended: false,
        };
        state.configure();
        Ok(state)
    }
    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }
    pub fn physical_size(&self) -> (u32, u32) {
        self.physical_size
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.physical_size = (width, height);
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.configure();
    }
    pub fn suspend(&mut self) {
        self.suspended = true;
    }
    pub fn resume(&mut self) {
        self.suspended = false;
        self.configure();
    }
    pub fn configure(&mut self) {
        if !self.suspended && self.physical_size.0 > 0 && self.physical_size.1 > 0 {
            self.surface.configure(&self.context.device, &self.config);
        }
    }
    /// The caller must submit drawing before presenting a Ready frame. No frame may
    /// remain alive when configuring/resizing this surface.
    pub fn acquire(&mut self) -> Result<AcquireFrame> {
        if self.suspended || self.physical_size.0 == 0 || self.physical_size.1 == 0 {
            return Ok(AcquireFrame::Deferred);
        }
        use wgpu::CurrentSurfaceTexture::*;
        Ok(match self.surface.get_current_texture() {
            Success(frame) | Suboptimal(frame) => AcquireFrame::Ready(frame),
            Timeout | Occluded => AcquireFrame::Deferred,
            Outdated => {
                self.configure();
                AcquireFrame::Reconfigure
            }
            Lost => AcquireFrame::Lost,
            Validation => return Err(RenderError("surface acquisition validation failed".into())),
        })
    }
}
#[cfg(any(target_os = "macos", target_os = "ios"))]
impl SurfaceState<'static> {
    /// Create the Rust WGPU surface over a Swift/Xcode-owned CAMetalLayer.
    /// No Winit window or loop is constructed.
    ///
    /// # Safety
    /// `layer` must identify a live CAMetalLayer, used on its permitted host thread.
    /// The caller retains the layer until this state is dropped, must not replace
    /// it while frames are alive, and must not independently present its drawables.
    pub async unsafe fn from_metal_layer(
        layer: *mut std::ffi::c_void,
        size: (u32, u32),
    ) -> Result<Self> {
        if layer.is_null() {
            return Err(RenderError("null CAMetalLayer".into()));
        }
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle_from_env()
        });
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer))
        }
        .map_err(|e| RenderError(format!("CAMetalLayer surface: {e}")))?;
        let context = GpuContext::request(instance, Some(&surface)).await?;
        Self::with_context(context, surface, size)
    }
}
