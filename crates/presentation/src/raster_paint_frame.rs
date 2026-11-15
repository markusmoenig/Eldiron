//! A frozen private raster frame: scene, camera, target, residency and receipt
//! share one lifetime. No mutable scene borrow or output escapes while pending.
use crate::orthographic_bake::{PaintPageRequest, PaintPageResidency, PaintResidencyUsage};
use renderkit::{GpuContext, RenderError, Result};
use rusterix::SceneHandler;
use scenevm::{
    RenderMode,
    paint_feedback::{PaintPageReadback, PaintPageStatus},
    paint_pages::PaintDispatchRegion,
    render_output::{SceneOutput, SceneRenderTarget},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RasterPaintAdvance {
    Pending,
    Complete,
}
enum Phase {
    Draw,
    Receipt(PaintPageReadback),
    Source(u32),
    Compose,
    Complete(SceneOutput),
    Failed(String),
}
/// Move a fully prepared scene into this owner; dropping cancels the private
/// request. Keep ordinary previous outputs separately until Complete. The host
/// polls its device and schedules advances; this owner never waits for the GPU.
pub struct RasterPaintFrame {
    context: Arc<GpuContext>,
    scene: SceneHandler,
    target: SceneRenderTarget,
    pages: PaintPageResidency,
    revision: u64,
    current: PaintDispatchRegion,
    waiting: Vec<PaintDispatchRegion>,
    phase: Phase,
    subdivisions: u64,
}
impl RasterPaintFrame {
    pub fn new(
        context: Arc<GpuContext>,
        mut scene: SceneHandler,
        target: SceneRenderTarget,
        mut pages: PaintPageResidency,
        revision: u64,
    ) -> Result<Self> {
        Self::prepare(&context, &mut scene, &target, &mut pages)?;
        Ok(Self::from_prepared(context, scene, target, pages, revision))
    }
    pub(crate) fn prepare(
        context: &Arc<GpuContext>,
        scene: &mut SceneHandler,
        target: &SceneRenderTarget,
        pages: &mut PaintPageResidency,
    ) -> Result<()> {
        if context.is_lost()
            || scene.vm.device_generation() != Some(context.generation())
            || scene.vm.vm.render_mode != RenderMode::Raster3D
            || !scene.vm.vm.is_enabled()
            || scene.vm.layer_has_frozen_rgba(0)
            || target.image().device_generation() != context.generation()
        {
            return Err(RenderError(
                "private raster frame has a stale device or inactive base".into(),
            ));
        }
        scene.vm.set_active_vm(0);
        scene
            .vm
            .set_paint_page_binding(&context, pages.binding()?)?;
        pages.begin_dispatch();
        Ok(())
    }
    pub(crate) fn from_prepared(
        context: Arc<GpuContext>,
        mut scene: SceneHandler,
        target: SceneRenderTarget,
        pages: PaintPageResidency,
        revision: u64,
    ) -> Self {
        scene.vm.set_layer_paint_dispatch(0, None);
        let size = target.size();
        let current = PaintDispatchRegion::full([size.0, size.1]);
        scene.vm.set_layer_paint_dispatch(0, Some(current));
        Self {
            context,
            scene,
            target,
            pages,
            revision,
            current,
            waiting: Vec::new(),
            phase: Phase::Draw,
            subdivisions: 0,
        }
    }
    /// At most one page upload and one base attempt or composition per call. A
    /// metadata receipt must complete before any resident slot is replaced.
    pub fn advance(&mut self) -> Result<RasterPaintAdvance> {
        if let Phase::Failed(error) = &self.phase {
            return Err(RenderError(error.clone()));
        }
        let result = self.advance_inner();
        if let Err(error) = &result {
            self.phase = Phase::Failed(error.to_string());
        }
        result
    }
    fn advance_inner(&mut self) -> Result<RasterPaintAdvance> {
        if self.context.is_lost() {
            return Err(RenderError("private raster frame device was lost".into()));
        }
        // Transitions with no GPU/source work are bounded. A pending receipt or
        // source yields immediately; one page load may feed one ordered replay.
        for _ in 0..4 {
            let phase = std::mem::replace(&mut self.phase, Phase::Draw);
            match phase {
                Phase::Draw => {
                    self.scene
                        .vm
                        .set_layer_paint_dispatch(0, Some(self.current));
                    self.scene.vm.render_base_gpu(&mut self.target)?;
                    if let Some(receipt) =
                        self.scene.vm.vm.capture_paint_page_status(&self.context)?
                    {
                        self.phase = Phase::Receipt(receipt);
                    } else {
                        self.finish_region();
                    }
                    return Ok(RasterPaintAdvance::Pending);
                }
                Phase::Receipt(mut receipt) => match receipt.try_finish(&self.context)? {
                    None => {
                        self.phase = Phase::Receipt(receipt);
                        return Ok(RasterPaintAdvance::Pending);
                    }
                    Some(PaintPageStatus::Ready) => self.finish_region(),
                    Some(PaintPageStatus::MissingEntry(index)) => self.phase = Phase::Source(index),
                },
                Phase::Source(index) => match self.pages.request(index)? {
                    PaintPageRequest::Pending => {
                        self.phase = Phase::Source(index);
                        return Ok(RasterPaintAdvance::Pending);
                    }
                    PaintPageRequest::Loaded => {
                        self.phase = Phase::Draw;
                    }
                    PaintPageRequest::WorkingSetFull => {
                        let [first, second] = self.current.split().ok_or_else(|| RenderError(
                                "single-pixel raster paint working set exceeds capacity; frame withheld".into()))?;
                        self.current = first;
                        self.waiting.push(second);
                        self.pages.begin_dispatch();
                        self.subdivisions += 1;
                        self.phase = Phase::Draw;
                    }
                    PaintPageRequest::Resident => {
                        return Err(RenderError(
                            "raster dependency receipt names an already resident source".into(),
                        ));
                    }
                },
                Phase::Compose => {
                    let output = self
                        .scene
                        .vm
                        .compose_prepared_base_gpu(&mut self.target, self.revision)?;
                    self.scene.vm.set_layer_paint_dispatch(0, None);
                    self.phase = Phase::Complete(output);
                    return Ok(RasterPaintAdvance::Complete);
                }
                Phase::Complete(output) => {
                    self.phase = Phase::Complete(output);
                    return Ok(RasterPaintAdvance::Complete);
                }
                Phase::Failed(error) => return Err(RenderError(error)),
            }
        }
        Ok(RasterPaintAdvance::Pending)
    }
    fn finish_region(&mut self) {
        if let Some(next) = self.waiting.pop() {
            self.current = next;
            self.pages.begin_dispatch();
            self.phase = Phase::Draw;
        } else {
            self.phase = Phase::Compose;
        }
    }
    pub fn output(&self) -> Option<&SceneOutput> {
        if !self.context.is_lost()
            && let Phase::Complete(output) = &self.phase
        {
            Some(output)
        } else {
            None
        }
    }
    /// Input/picking may borrow the exact completed camera/scene only after its
    /// output is ready. Pending/private raw regions are never an input receipt.
    pub fn completed_scene(&self) -> Option<&SceneHandler> {
        self.output().map(|_| &self.scene)
    }
    pub(crate) fn scene(&self) -> &SceneHandler {
        &self.scene
    }
    pub(crate) fn has_cpu_target_storage(&self) -> bool {
        self.scene.vm.render_target_cpu_bytes() > 0
            || self.target.has_cpu_storage()
            || self.target.has_readback_storage()
    }
    pub(crate) fn into_parts(
        mut self,
    ) -> (
        SceneHandler,
        SceneRenderTarget,
        PaintPageResidency,
        Option<SceneOutput>,
    ) {
        self.scene.vm.set_layer_paint_dispatch(0, None);
        let output = if let Phase::Complete(output) = self.phase {
            Some(output)
        } else {
            None
        };
        (self.scene, self.target, self.pages, output)
    }
    pub fn residency_usage(&self) -> PaintResidencyUsage {
        self.pages.usage()
    }
    pub fn subdivisions(&self) -> u64 {
        self.subdivisions
    }
    /// Reuse scene, target and resident pages for the next explicitly prepared
    /// frame. Submit sampling of this completed output before reusing its target.
    pub fn into_completed(
        self,
    ) -> Result<(
        SceneHandler,
        SceneRenderTarget,
        PaintPageResidency,
        SceneOutput,
    )> {
        if self.context.is_lost() {
            return Err(RenderError("private raster frame device was lost".into()));
        }
        let Phase::Complete(output) = self.phase else {
            return Err(RenderError("raster frame is not complete".into()));
        };
        Ok((self.scene, self.target, self.pages, output))
    }
}

#[cfg(test)]
mod tests;
