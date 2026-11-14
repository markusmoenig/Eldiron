//! Cooperative original-tile production on an injected device. The application
//! owns source/document stamps and an independent frozen scene for this job.
//! Each advance submits at most one render/capture or polls one outstanding map.
use super::*;
use renderkit::ReadbackTicket;
use rusterix::editor_view::{EditorView3D, EditorViewFrame};
use rusterix::{OrthographicBakeController, OrthographicBakeWork, SceneHandler};
use scenevm::{
    paint_feedback::{PaintPageReadback, PaintPageStatus},
    paint_pages::LayerHistoryCheckpoint,
    render_output::SceneRenderTarget,
    surface_output::{SurfaceDepthCapture, SurfaceSelection},
};

#[cfg(test)]
mod paint_replay_tests;
mod regions;
use regions::Regions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BakeAdvance {
    Idle,
    Pending,
    Sample,
    Tile { complete: bool },
    Retired,
}
#[derive(Clone, Copy)]
enum Channel {
    Beauty,
    Albedo,
    Material,
    Normal,
    Depth,
}
enum Pending {
    Source {
        work: OrthographicBakeWork,
        channel: Channel,
        index: u32,
    },
    Paint {
        work: OrthographicBakeWork,
        channel: Channel,
        image: Arc<GpuImage>,
        status: PaintPageReadback,
        history: Option<LayerHistoryCheckpoint>,
    },
    Color {
        work: OrthographicBakeWork,
        channel: Channel,
        ticket: ReadbackTicket,
    },
    Depth {
        work: OrthographicBakeWork,
        ticket: SurfaceDepthCapture,
    },
}
#[derive(Default)]
struct Tile {
    rgba: Vec<u8>,
    albedo: Vec<u8>,
    material: Vec<u8>,
    normal: Vec<u8>,
}
/// An independent scene is moved into the job once. It cannot borrow or mutate
/// a displayed view's VM/camera/temporal history, or initialize another device.
/// Dropping the job retires its async map, targets and frozen scene together.
pub struct OrthographicBakeJob {
    context: Arc<GpuContext>,
    scene: SceneHandler,
    frame: EditorView3D,
    target: Option<SceneRenderTarget>,
    pending: Option<Pending>,
    paint_pages: Option<PaintPageResidency>,
    replay: Option<(OrthographicBakeWork, Channel)>,
    regions: Option<Regions>,
    subdivisions: u64,
    final_work: Option<(OrthographicBakeWork, Channel)>,
    tile: Tile,
    revision: u64,
    request: Option<(Uuid, Uuid)>,
    prepared_geometry: bool,
    retired: bool,
}
impl OrthographicBakeJob {
    pub fn new(
        context: Arc<GpuContext>,
        mut scene: SceneHandler,
        frame: EditorView3D,
    ) -> Result<Self> {
        if context.is_lost() || scene.vm.device_generation() != Some(context.generation()) {
            return Err(RenderError(
                "bake scene belongs to a lost or different device".into(),
            ));
        }
        scene.vm.set_background_lighting(false);
        scene.vm.set_active_vm(0);
        scene.vm.execute(scenevm::Atom::ClearDynamics);
        scene.vm.clear_layer_frozen_rgba(0);
        scene.vm.set_layer_ping_pong_enabled(0, false);
        scene.vm.set_layer_compute3d_bake_output(0, 0);
        scene.vm.set_layer_progressive_sample_index(0, None);
        Ok(Self {
            context,
            scene,
            frame,
            target: None,
            pending: None,
            paint_pages: None,
            replay: None,
            regions: None,
            subdivisions: 0,
            final_work: None,
            tile: Tile::default(),
            revision: 0,
            request: None,
            prepared_geometry: false,
            retired: false,
        })
    }
    /// Configure before advancing the private job. The frozen source survives
    /// resident eviction; a fault never advances the controller's sample clock.
    pub fn set_paint_pages(
        &mut self,
        source: Arc<shared::surface_paint::pages::PaintPageIndex>,
        plane_byte_limit: u64,
    ) -> Result<()> {
        if self.prepared_geometry || self.request.is_some() || self.retired {
            return Err(RenderError(
                "paint sources must be bound before bake preparation".into(),
            ));
        }
        let pages = PaintPageResidency::new(self.context.clone(), source, plane_byte_limit)?;
        self.scene.vm.set_active_vm(0);
        self.scene
            .vm
            .set_paint_page_binding(&self.context, pages.binding()?)?;
        self.paint_pages = Some(pages);
        Ok(())
    }
    pub fn paint_subdivisions(&self) -> u64 {
        self.subdivisions
    }
    pub fn paint_residency_usage(&self) -> Option<PaintResidencyUsage> {
        self.paint_pages.as_ref().map(PaintPageResidency::usage)
    }
    pub fn cancel(&mut self) {
        self.retired = true;
        self.reset_tile();
    }
    fn reset_tile(&mut self) {
        self.pending = None;
        self.replay = None;
        self.regions = None;
        self.scene.vm.set_layer_paint_dispatch(0, None);
        self.final_work = None;
        self.tile = Tile::default();
        self.scene.vm.set_layer_ping_pong_enabled(0, false);
        self.scene.vm.set_layer_progressive_sample_index(0, None);
        self.scene.vm.set_layer_compute3d_bake_output(0, 0);
    }
    pub fn has_pending_capture(&self) -> bool {
        matches!(
            self.pending,
            Some(Pending::Paint { .. } | Pending::Color { .. } | Pending::Depth { .. })
        )
    }
    pub fn has_pending_source(&self) -> bool {
        matches!(self.pending, Some(Pending::Source { .. }))
    }
    pub fn device_generation(&self) -> u64 {
        self.context.generation()
    }
    pub fn has_cpu_viewport_storage(&self) -> bool {
        self.scene.vm.render_target_cpu_bytes() != 0
            || self
                .target
                .as_ref()
                .is_some_and(|target| target.has_cpu_storage() || target.has_readback_storage())
            || self.scene.vm.has_surface_readback_storage()
    }
    /// The application validates its source stamp before every advance. The
    /// controller independently rejects results from retired requests/samples.
    pub fn advance(
        &mut self,
        bake: &mut OrthographicBakeController,
        map: Uuid,
        camera: Camera3D,
        viewport: (u32, u32),
    ) -> Result<BakeAdvance> {
        let result = self.advance_inner(bake, map, camera, viewport);
        if result.is_err() {
            self.cancel();
        }
        result
    }
    fn advance_inner(
        &mut self,
        bake: &mut OrthographicBakeController,
        map: Uuid,
        camera: Camera3D,
        viewport: (u32, u32),
    ) -> Result<BakeAdvance> {
        if self.retired {
            return Ok(BakeAdvance::Retired);
        }
        let Some(request) = bake.active_request() else {
            self.reset_tile();
            return Ok(BakeAdvance::Idle);
        };
        if request.0 != map || self.request.is_some_and(|old| old != request) {
            self.cancel();
            return Ok(BakeAdvance::Retired);
        }
        self.request = Some(request);
        if self.context.is_lost() {
            self.cancel();
            return Err(RenderError("bake device lost".into()));
        }
        if viewport.0 == 0 || viewport.1 == 0 {
            return Ok(BakeAdvance::Pending);
        }
        if !self.prepared_geometry {
            self.render(camera, (1, 1), None, 0)?;
            self.prepared_geometry = true;
            return Ok(BakeAdvance::Pending);
        }
        let bounds = bake
            .needs_scene_bounds(map)
            .then(|| {
                self.scene.vm.layer_scene_projected_bounds_3d(
                    0,
                    camera.right.normalized(),
                    camera.up.normalized(),
                )
            })
            .flatten();
        let next_work = bake
            .prepare_work(map, viewport.0, viewport.1, camera, bounds)
            .map_err(RenderError)?;
        if let Some(mut pending) = self.pending.take() {
            let work = match &pending {
                Pending::Paint { work, .. }
                | Pending::Source { work, .. }
                | Pending::Color { work, .. }
                | Pending::Depth { work, .. } => *work,
            };
            if !bake.work_is_current(&work) {
                self.cancel();
                return Ok(BakeAdvance::Retired);
            }
            match &mut pending {
                Pending::Source { channel, index, .. } => {
                    self.resolve_source(work, *channel, *index)?
                }
                Pending::Paint {
                    channel,
                    image,
                    status,
                    history,
                    ..
                } => match status.try_finish(&self.context)? {
                    None => self.pending = Some(pending),
                    Some(PaintPageStatus::Ready) => {
                        self.finish_region(work, *channel, image.clone(), history.take())?;
                    }
                    Some(PaintPageStatus::MissingEntry(index)) => {
                        if let Some(checkpoint) = history.take() {
                            if !self.scene.vm.restore_layer_history(0, checkpoint) {
                                return Err(RenderError(
                                    "paint replay lost its valid sample history".into(),
                                ));
                            }
                        }
                        self.resolve_source(work, *channel, index)?;
                    }
                },
                Pending::Color {
                    channel, ticket, ..
                } => {
                    let Some(image) = ticket.try_finish(&self.context)? else {
                        self.pending = Some(pending);
                        return Ok(BakeAdvance::Pending);
                    };
                    let bytes = image.pixels().to_vec();
                    match channel {
                        Channel::Beauty => {
                            bake.preview_work(&work, &bytes).map_err(RenderError)?;
                            if !bake.finish_work_sample(&work).map_err(RenderError)? {
                                return Ok(BakeAdvance::Sample);
                            }
                            self.tile.rgba = bytes;
                            self.scene.vm.set_layer_ping_pong_enabled(0, false);
                            self.final_work = Some((work, Channel::Albedo));
                            return Ok(BakeAdvance::Sample);
                        }
                        Channel::Albedo => {
                            self.tile.albedo = bytes;
                            self.final_work = Some((work, Channel::Material));
                        }
                        Channel::Material => {
                            self.tile.material = bytes;
                            self.final_work = Some((work, Channel::Normal));
                        }
                        Channel::Normal => {
                            self.tile.normal = bytes;
                            self.final_work = Some((work, Channel::Depth));
                        }
                        Channel::Depth => unreachable!(),
                    }
                }
                Pending::Depth { ticket, .. } => {
                    let Some(mut depth) = ticket.try_finish(&self.context)? else {
                        self.pending = Some(pending);
                        return Ok(BakeAdvance::Pending);
                    };
                    #[cfg(test)]
                    if std::env::var_os("ELDIRON_BAKE_DEBUG").is_some() {
                        eprintln!(
                            "Job raw captured static depth finite={}",
                            depth.depth.iter().filter(|depth| depth.is_finite()).count()
                        );
                    }
                    for (depth, normal) in
                        depth.depth.iter_mut().zip(self.tile.normal.chunks_exact(4))
                    {
                        if normal[3] == 0 {
                            *depth = f32::INFINITY;
                        }
                    }
                    let tile = std::mem::take(&mut self.tile);
                    let complete = bake
                        .commit_work(
                            &work,
                            tile.rgba,
                            depth.depth,
                            tile.albedo,
                            tile.normal,
                            tile.material,
                        )
                        .map_err(RenderError)?;
                    self.reset_tile();
                    return Ok(BakeAdvance::Tile { complete });
                }
            }
            return Ok(BakeAdvance::Pending);
        }
        let replay = self.replay.take();
        let replaying = replay.is_some();
        let (work, channel) = if let Some((work, channel)) = replay {
            if !bake.work_is_current(&work) {
                self.cancel();
                return Ok(BakeAdvance::Retired);
            }
            (work, channel)
        } else if let Some((work, channel)) = self.final_work.take() {
            if !bake.work_is_current(&work) {
                self.cancel();
                return Ok(BakeAdvance::Retired);
            }
            (work, channel)
        } else {
            let Some(work) = next_work else {
                self.reset_tile();
                return Ok(BakeAdvance::Idle);
            };
            (work, Channel::Beauty)
        };
        if matches!(channel, Channel::Depth) {
            self.scene.vm.set_active_vm(0);
            let surface = self.scene.vm.render_surface_gpu(
                &self.context,
                (work.tile_size, work.tile_size),
                SurfaceSelection::Static,
                self.revision,
            )?;
            self.pending = Some(Pending::Depth {
                work,
                ticket: surface.capture_asset_depth(&self.context)?,
            });
            return Ok(BakeAdvance::Pending);
        }
        let channel_index = match channel {
            Channel::Beauty => 0,
            Channel::Albedo => 1,
            Channel::Material => 2,
            Channel::Normal => 3,
            Channel::Depth => unreachable!(),
        };
        let beauty = matches!(channel, Channel::Beauty);
        if !replaying && let Some(pages) = self.paint_pages.as_mut() {
            pages.begin_dispatch();
            self.regions = Some(Regions::new(work.tile_size));
            self.scene.vm.set_layer_paint_dispatch(0, None);
        }
        if let Some(regions) = &self.regions {
            self.scene
                .vm
                .set_layer_paint_dispatch(0, Some(regions.current()));
        }
        if beauty && work.sample == 0 && !replaying {
            self.scene.vm.set_layer_ping_pong_enabled(0, false);
            self.scene.vm.set_layer_ping_pong_enabled(0, true);
        }
        let history = if self.regions.is_none() && beauty && work.sample > 0 {
            Some(self.scene.vm.layer_history_checkpoint(0).ok_or_else(|| {
                RenderError("paint replay requires previous sample history".into())
            })?)
        } else {
            None
        };
        let image = self.render(
            work.camera,
            (work.tile_size, work.tile_size),
            Some(if beauty { work.sample } else { 0 }),
            channel_index,
        )?;
        let history = if self.regions.is_some() {
            self.scene.vm.take_layer_attempt_history(0)
        } else {
            history
        };
        if let Some(status) = self.scene.vm.vm.capture_paint_page_status(&self.context)? {
            self.pending = Some(Pending::Paint {
                work,
                channel,
                image,
                status,
                history,
            });
        } else {
            self.finish_region(work, channel, image, history)?;
        }
        Ok(BakeAdvance::Pending)
    }
    fn finish_region(
        &mut self,
        work: OrthographicBakeWork,
        channel: Channel,
        image: Arc<GpuImage>,
        history: Option<LayerHistoryCheckpoint>,
    ) -> Result<()> {
        if self.regions.as_mut().is_some_and(Regions::next) {
            if let Some(history) = history {
                if !self.scene.vm.restore_layer_history(0, history) {
                    return Err(RenderError(
                        "resolved paint region lost its previous sample history".into(),
                    ));
                }
            } else if matches!(channel, Channel::Beauty) {
                return Err(RenderError(
                    "partial beauty has no previous sample lease".into(),
                ));
            }
            self.paint_pages.as_mut().unwrap().begin_dispatch();
            self.replay = Some((work, channel));
        } else {
            self.regions = None;
            self.scene.vm.set_layer_paint_dispatch(0, None);
            self.pending = Some(Pending::Color {
                work,
                channel,
                ticket: ReadbackTicket::capture_asset(&self.context, image)?,
            });
        }
        Ok(())
    }
    fn resolve_source(
        &mut self,
        work: OrthographicBakeWork,
        channel: Channel,
        index: u32,
    ) -> Result<()> {
        let pages = self.paint_pages.as_mut().ok_or_else(|| {
            RenderError(format!(
                "Bake source entry {index} is not resident; sample withheld"
            ))
        })?;
        match pages.request(index)? {
            PaintPageRequest::Pending => {
                self.pending = Some(Pending::Source {
                    work,
                    channel,
                    index,
                })
            }
            PaintPageRequest::Loaded => {
                self.scene
                    .vm
                    .set_paint_page_binding(&self.context, pages.binding()?)?;
                self.replay = Some((work, channel));
            }
            PaintPageRequest::WorkingSetFull => {
                let regions = self.regions.as_mut().ok_or_else(|| {
                    RenderError("paint pressure has no private region owner".into())
                })?;
                if !regions.split() {
                    return Err(RenderError("single-pixel paint/stamp dependency working set exceeds resident capacity; sample withheld".into()));
                }
                self.subdivisions += 1;
                pages.begin_dispatch();
                self.replay = Some((work, channel));
            }
            PaintPageRequest::Resident => {
                return Err(RenderError(
                    "dependency receipt names an already resident source".into(),
                ));
            }
        }
        Ok(())
    }
    fn render(
        &mut self,
        camera: Camera3D,
        size: (u32, u32),
        sample: Option<u32>,
        channel: u32,
    ) -> Result<Arc<GpuImage>> {
        self.scene.vm.set_layer_compute3d_bake_output(0, channel);
        self.scene.vm.set_layer_progressive_sample_index(0, sample);
        let mut frame = self.frame.clone();
        frame.size = size;
        frame.camera = camera;
        frame.static_geometry = true;
        EditorViewFrame::D3(frame).prepare(&mut self.scene);
        // Authored transport excludes transient editor symbols/actors.
        for layer in 1..self.scene.vm.vm_layer_count() {
            self.scene.vm.set_layer_enabled(layer, false);
        }
        self.scene.vm.set_active_vm(0);
        if self
            .target
            .as_ref()
            .is_none_or(|target| target.size() != size)
        {
            self.target = Some(SceneRenderTarget::new(
                self.context.clone(),
                size.0,
                size.1,
            )?);
        }
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| RenderError("bake revision exhausted".into()))?;
        self.scene
            .vm
            .render_gpu(self.target.as_mut().unwrap(), self.revision)?;
        Ok(self
            .scene
            .vm
            .layer_output(&self.context, 0, self.revision)?
            .ok_or_else(|| RenderError("bake layer has no rendered channels".into()))?
            .image)
    }
}
