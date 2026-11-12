//! Host/application and surface contracts shared by Winit and foreign layers.
//! This module has no widget tree, simulation clock or destination CPU bitmap.
use super::{InputEvent, InputResponse, TextInputState};
use crate::paint::PaintList;
use renderkit::{AcquireFrame, GpuContext, ImageRegistry, Result, SurfaceState};
use std::sync::Arc;

pub enum HostRequest {
    Exit,
    Fail(String),
    Title(String),
    ClipboardRead { request: u64 },
    ClipboardWrite { text: String, request: Option<u64> },
}
/// Host metrics and requested platform state survive surface/device replacement.
/// Logical positions use floating-point precision; density is applied once by
/// the presenter and native platform adapters, including Swift/Xcode hosts.
pub struct HostContext {
    pub physical_size: (u32, u32),
    pub logical_size: [f32; 2],
    pub density: f32,
    pub requests: Vec<HostRequest>,
    pub text_input: Option<TextInputState>,
    pub cursor: crate::thecontext::TheCursorIcon,
    pub cursor_visible: bool,
}
impl HostContext {
    pub fn new(logical_size: (u32, u32)) -> Self {
        Self {
            physical_size: logical_size,
            logical_size: [logical_size.0 as f32, logical_size.1 as f32],
            density: 1.,
            requests: Vec::new(),
            text_input: None,
            cursor: crate::thecontext::TheCursorIcon::Default,
            cursor_visible: true,
        }
    }
    pub fn resize(&mut self, physical_size: (u32, u32), density: f32) {
        self.density = if density.is_finite() && density > 0. {
            density
        } else {
            1.
        };
        self.physical_size = physical_size;
        self.logical_size = [
            physical_size.0 as f32 / self.density,
            physical_size.1 as f32 / self.density,
        ];
        if self.logical_size.iter().any(|size| !size.is_finite()) {
            self.density = 1.;
            self.logical_size = [physical_size.0 as f32, physical_size.1 as f32];
        }
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
        let fps = if fps.is_nan() {
            60.
        } else {
            fps.clamp(1., 240.)
        };
        Self {
            redraw: true,
            wake_after: Some(web_time::Duration::from_secs_f64(1. / fps)),
        }
    }
}
/// Optional native window metadata. Surface/device reconstruction does not
/// replay user window-move/resize callbacks or persist desktop preferences.
#[derive(Clone, Debug, Default)]
pub struct WindowConfiguration {
    pub minimum_size: Option<(u32, u32)>,
    pub position: Option<(i32, i32)>,
    pub icon: Option<(Vec<u8>, u32, u32)>,
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
    fn window_configuration(&self) -> WindowConfiguration {
        WindowConfiguration::default()
    }
    fn document_edited(&self) -> bool {
        false
    }
    fn window_moved(&mut self, _position: (i32, i32), _host: &mut HostContext) {}
    fn window_resized(&mut self, _host: &mut HostContext) {}
    fn initialize(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<()>;
    /// CPU document/runtime/controllers belong to the application and survive.
    fn gpu_recreated(
        &mut self,
        _host: &mut HostContext,
        _images: &mut ImageRegistry,
    ) -> Result<()> {
        Ok(())
    }
    fn resize(&mut self, _host: &mut HostContext) {}
    /// CPU text/source metadata for native IME and accessibility. The target and
    /// source revision address subsequent range edits; this never captures pixels.
    fn native_text_snapshot(
        &mut self,
        _host: &mut HostContext,
    ) -> Option<crate::text_edit::NativeTextSnapshot> {
        None
    }
    fn native_text_query(
        &mut self,
        _host: &mut HostContext,
        _request: &crate::text_edit::NativeTextRequest,
    ) -> Option<crate::text_edit::NativeTextAnswer> {
        None
    }
    fn input(&mut self, _event: &InputEvent, _host: &mut HostContext) -> InputResponse {
        InputResponse::default()
    }
    fn update(&mut self, _host: &mut HostContext, _elapsed: web_time::Duration) -> FrameRequest {
        FrameRequest::idle()
    }
    fn frame(&mut self, host: &mut HostContext, images: &mut ImageRegistry) -> Result<PaintList>;
    /// Called once after successful surface presentation, never for a deferred
    /// acquisition. This acknowledges submission/present, not physical display
    /// completion; offscreen frame preparation is a separate operation.
    fn presented(&mut self, _host: &mut HostContext) {}
    fn close_requested(&mut self, _host: &mut HostContext) -> bool {
        true
    }
    fn suspended(&mut self, _host: &mut HostContext) {}
    fn resumed(&mut self, _host: &mut HostContext) {}
}
/// Common compositor/surface owner, including an Xcode-supplied CAMetalLayer.
/// A foreign host drives `present` and input using its own event/display loop.
pub struct GpuPresenter<'a> {
    pub surface: SurfaceState<'a>,
    pub images: ImageRegistry,
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
            images: ImageRegistry::new(context.clone()),
            renderer: crate::render_wgpu::PaintRenderer::new(context)?,
        })
    }
    pub fn context(&self) -> Arc<GpuContext> {
        self.surface.context.clone()
    }
    fn acquire_frame(&mut self) -> Result<AcquireFrame> {
        if self.surface.context.is_lost() {
            return Ok(AcquireFrame::Lost);
        }
        if let Err(error) = self.surface.context.poll() {
            if self.surface.context.is_lost() {
                return Ok(AcquireFrame::Lost);
            }
            return Err(error);
        }
        self.surface.acquire()
    }
    fn render_acquired(
        &mut self,
        frame: renderkit::wgpu::SurfaceTexture,
        paint: &PaintList,
        density: f32,
        clear: [f32; 4],
    ) -> Result<PresentResult> {
        let view = frame.texture.create_view(&Default::default());
        if let Err(error) = self.renderer.render(
            paint,
            &self.images,
            &view,
            self.surface.format(),
            self.surface.physical_size(),
            density,
            clear,
        ) {
            if self.surface.context.is_lost() {
                return Ok(PresentResult::SurfaceLost);
            }
            return Err(error);
        }
        frame.present();
        Ok(PresentResult::Presented)
    }
    /// Present an already prepared/cached paint list. A deferred acquisition
    /// does not submit paint or acknowledge an application presentation.
    pub fn present(
        &mut self,
        paint: &PaintList,
        density: f32,
        clear: [f32; 4],
    ) -> Result<PresentResult> {
        match self.acquire_frame()? {
            AcquireFrame::Ready(frame) => self.render_acquired(frame, paint, density, clear),
            AcquireFrame::Deferred | AcquireFrame::Reconfigure => Ok(PresentResult::Deferred),
            AcquireFrame::Lost => Ok(PresentResult::SurfaceLost),
        }
    }
    /// Acquire before preparing UI/scenes. An occluded/locked/zero-sized surface
    /// never invokes the callback or allocates a new visual frame. Simulation
    /// updates remain the application's independent host timing responsibility.
    pub fn present_with(
        &mut self,
        density: f32,
        clear: [f32; 4],
        prepare: impl FnOnce(&mut ImageRegistry) -> Result<PaintList>,
    ) -> Result<PresentResult> {
        let frame = match self.acquire_frame()? {
            AcquireFrame::Ready(frame) => frame,
            AcquireFrame::Deferred | AcquireFrame::Reconfigure => {
                return Ok(PresentResult::Deferred);
            }
            AcquireFrame::Lost => return Ok(PresentResult::SurfaceLost),
        };
        let paint = match prepare(&mut self.images) {
            Ok(paint) => paint,
            Err(_) if self.surface.context.is_lost() => return Ok(PresentResult::SurfaceLost),
            Err(error) => return Err(error),
        };
        self.render_acquired(frame, &paint, density, clear)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_host_metrics_and_invalid_density_are_normalized_once() {
        let mut host = HostContext::new((801, 603));
        host.resize((801, 603), 1.5);
        assert_eq!(host.physical_size, (801, 603));
        assert_eq!(host.logical_size, [534., 402.]);
        assert_eq!(host.density, 1.5);
        host.resize((0, 0), f32::NAN);
        assert_eq!(host.logical_size, [0.; 2]);
        assert_eq!(host.density, 1.);
        host.resize((640, 480), -2.);
        assert_eq!(host.logical_size, [640., 480.]);
        host.resize((640, 480), f32::from_bits(1));
        assert_eq!(host.logical_size, [640., 480.]);
        assert_eq!(
            FrameRequest::continuous(f64::NAN).wake_after,
            Some(web_time::Duration::from_secs_f64(1. / 60.))
        );
    }
}
