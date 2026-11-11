use super::{InputEvent, InputResponse};
use crate::{paint::PaintList, prelude::*, render_wgpu::UiImages, theui::paint::UiPaintError};
use renderkit::{AcquireFrame, GpuContext, RenderError, Result, SurfaceState};
use std::sync::Arc;

/// UI state survives surface/device replacement and belongs to the application host.
pub enum HostRequest {
    Exit,
    Title(String),
}
pub struct UiSession {
    pub ui: TheUI,
    pub context: TheContext,
    pub requests: Vec<HostRequest>,
}
impl UiSession {
    pub fn new(logical_size: (usize, usize), density: f32) -> Self {
        let mut context = TheContext::new(logical_size.0, logical_size.1, density);
        context.ui.command_paint = true;
        let mut ui = TheUI::new();
        ui.init(&mut context);
        Self {
            ui,
            context,
            requests: Vec::new(),
        }
    }
    pub fn resize(&mut self, physical: (u32, u32), density: f32) {
        let density = if density.is_finite() && density > 0. {
            density
        } else {
            1.
        };
        self.context.width = (physical.0 as f32 / density).round() as usize;
        self.context.height = (physical.1 as f32 / density).round() as usize;
        self.context.scale_factor = density;
        self.context.framebuffer_width = physical.0 as usize;
        self.context.framebuffer_height = physical.1 as usize;
        self.context.ui.relayout = true;
        self.context.ui.redraw_all = true;
    }
    pub fn input(&mut self, event: &InputEvent) -> InputResponse {
        self.ui.prepare_paint_layout(&mut self.context);
        self.ui.dispatch_input(event, &mut self.context)
    }
    pub fn paint(&mut self, images: &mut UiImages) -> std::result::Result<PaintList, UiPaintError> {
        self.ui.paint_commands(&mut self.context, images)
    }
}
/// Timing policy is explicit: an editor can sleep while a player requests frames.
#[derive(Clone, Copy, Debug)]
pub struct FrameRequest {
    pub redraw: bool,
    pub wake_after: Option<web_time::Duration>,
}
impl FrameRequest {
    pub fn idle() -> Self {
        Self {
            redraw: false,
            wake_after: None,
        }
    }
    pub fn continuous(fps: f64) -> Self {
        Self {
            redraw: true,
            wake_after: Some(web_time::Duration::from_secs_f64(1. / fps.clamp(1., 240.))),
        }
    }
}
/// The native/browser/foreign-layer hosts all execute the same application contract.
/// There is deliberately no callback receiving destination CPU pixels.
pub trait GpuApplication {
    fn title(&self) -> String {
        "Eldiron".into()
    }
    fn initial_size(&self) -> (u32, u32) {
        (1200, 700)
    }
    fn initialize(&mut self, ui: &mut UiSession, resources: &mut UiImages) -> Result<()>;
    /// Recreate device resources after loss; document and simulation state survive.
    fn gpu_recreated(&mut self, _ui: &mut UiSession, _resources: &mut UiImages) -> Result<()> {
        Ok(())
    }
    fn input(&mut self, event: &InputEvent, ui: &mut UiSession) -> InputResponse {
        ui.input(event)
    }
    fn update(&mut self, ui: &mut UiSession, _elapsed: web_time::Duration) -> FrameRequest {
        FrameRequest {
            redraw: ui.ui.update(&mut ui.context),
            wake_after: None,
        }
    }
    fn frame(&mut self, ui: &mut UiSession, resources: &mut UiImages) -> Result<PaintList> {
        ui.paint(resources).map_err(|e| RenderError(e.to_string()))
    }
    fn close_requested(&mut self, _ui: &mut UiSession) -> bool {
        true
    }
    fn suspended(&mut self, _ui: &mut UiSession) {}
    fn resumed(&mut self, _ui: &mut UiSession) {}
}
/// Common compositor/surface owner, including an Xcode-supplied CAMetalLayer.
/// A foreign host drives `present` and input using its own event/display loop.
pub struct GpuPresenter<'a> {
    pub surface: SurfaceState<'a>,
    pub images: UiImages,
    renderer: crate::render_wgpu::PaintRenderer,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentResult {
    Presented,
    Deferred,
    SurfaceLost,
}
impl<'a> GpuPresenter<'a> {
    pub fn new(surface: SurfaceState<'a>) -> Result<Self> {
        let context = surface.context.clone();
        Ok(Self {
            surface,
            images: UiImages::new(context.clone()),
            renderer: crate::render_wgpu::PaintRenderer::new(context)?,
        })
    }
    pub fn context(&self) -> Arc<GpuContext> {
        self.surface.context.clone()
    }
    pub fn present(
        &mut self,
        paint: &PaintList,
        density: f32,
        clear: [f32; 4],
    ) -> Result<PresentResult> {
        if self.surface.context.is_lost() {
            return Ok(PresentResult::SurfaceLost);
        }
        if let Err(error) = self.surface.context.poll() {
            if self.surface.context.is_lost() {
                return Ok(PresentResult::SurfaceLost);
            }
            return Err(error);
        }
        let frame = match self.surface.acquire()? {
            AcquireFrame::Ready(frame) => frame,
            AcquireFrame::Deferred | AcquireFrame::Reconfigure => {
                return Ok(PresentResult::Deferred);
            }
            AcquireFrame::Lost => return Ok(PresentResult::SurfaceLost),
        };
        let view = frame.texture.create_view(&Default::default());
        self.renderer.render(
            paint,
            &self.images.registry,
            &view,
            self.surface.format(),
            self.surface.physical_size(),
            density,
            clear,
        )?;
        frame.present();
        Ok(PresentResult::Presented)
    }
}
