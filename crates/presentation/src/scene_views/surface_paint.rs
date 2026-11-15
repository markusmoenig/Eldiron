//! Immutable paint pages and lookup data share this SceneViews device. Validation,
//! changed-page transfers and lookup writes stay private until the source is complete.
use super::*;
use renderkit::{AlphaMode, ColorSpace, GpuImage, ImageDescriptor, wgpu};
use rusterix::surface_paint::{PAGE_SIZE, SurfacePaint, SurfacePaintValidation};
use scenevm::paint_pages::{PaintPageBinding, PaintPageBindingPreparation};
use shared::surface_paint::pages::{PaintPageIndex, SurfacePaintIndexBuilder};
use std::collections::VecDeque;
mod paged;
mod reuse;

const FRAME_BYTES: usize = 512 * 1024;
const FRAME_STEPS: usize = 4096;
const PAGE_BYTES: usize = (PAGE_SIZE * PAGE_SIZE * 8) as usize;

/// Producer allocations and transfer counts. Auxiliary scene buffers, queued
/// consumers and driver/cache peaks are accounted separately.
#[derive(Clone, Copy, Debug, Default)]
pub struct SurfacePaintStats {
    pub sources: usize,
    pub logical_cpu_page_bytes: u64,
    pub gpu_plane_bytes: u64,
    pub gpu_table_bytes: u64,
    pub uploaded_pages: u64,
    pub reused_pages: u64,
    pub gpu_copied_bytes: u64,
    pub reused_bindings: u64,
    pub pending_sources: usize,
    pub frame_uploaded_bytes: usize,
    pub pending_views: usize,
}
struct Upload {
    color: Arc<GpuImage>,
    material: Arc<GpuImage>,
    cursor: usize,
    previous: Option<Arc<SurfacePaint>>,
}
enum State {
    Validate(SurfacePaintValidation),
    Index(SurfacePaintIndexBuilder),
    Indexed(Arc<PaintPageIndex>),
    Upload(Upload),
    Binding(PaintPageBindingPreparation),
    Failed(String),
    Ready(Arc<PaintPageBinding>),
}
struct BoundSource {
    source: Arc<SurfacePaint>,
    state: State,
    previous: Option<reuse::Previous>,
}
#[derive(Default)]
struct SourceStep {
    queued: usize,
    copied: u64,
    reused: bool,
}
pub(super) struct Sources {
    bindings: FxHashMap<usize, BoundSource>,
    pending: VecDeque<usize>,
    uploaded_pages: u64,
    reused_pages: u64,
    gpu_copied_bytes: u64,
    reused_bindings: u64,
    frame_bytes: usize,
    frame_steps: usize,
    frame_started: Option<web_time::Instant>,
    plane_limit: u64,
    view_turn: usize,
}
impl Default for Sources {
    fn default() -> Self {
        Self {
            bindings: Default::default(),
            pending: VecDeque::new(),
            uploaded_pages: 0,
            reused_pages: 0,
            gpu_copied_bytes: 0,
            reused_bindings: 0,
            frame_bytes: 0,
            frame_steps: 0,
            frame_started: None,
            plane_limit: 64 * 1024 * 1024,
            view_turn: 0,
        }
    }
}
impl Sources {
    pub(super) fn available_frame_bytes(&self) -> usize {
        if self
            .frame_started
            .is_some_and(|started| started.elapsed() >= web_time::Duration::from_millis(2))
        {
            return 0;
        }
        FRAME_BYTES.saturating_sub(self.frame_bytes)
    }
    pub(super) fn record_residency_uploads(
        &mut self,
        before: crate::orthographic_bake::PaintResidencyUsage,
        after: crate::orthographic_bake::PaintResidencyUsage,
    ) {
        self.frame_bytes += ((after.uploaded_bytes - before.uploaded_bytes)
            + (after.table_uploaded_bytes - before.table_uploaded_bytes))
            as usize;
        self.uploaded_pages += after.page_uploads - before.page_uploads;
    }
    fn request(&mut self, context: &GpuContext, source: &Arc<SurfacePaint>) -> Result<()> {
        if context.is_lost() {
            return Err(RenderError("Surface paint device was lost".into()));
        }
        let identity = Arc::as_ptr(source) as usize;
        if let Some(old) = self.bindings.get(&identity) {
            if let State::Failed(error) = &old.state {
                return Err(RenderError(error.clone()));
            }
            self.reused_bindings += 1;
            return Ok(());
        }
        let limits = context.device.limits();
        if (std::mem::size_of_val(source.entries.as_ref()) as u64 + 20)
            > u64::from(limits.max_storage_buffer_binding_size)
        {
            return Err(RenderError(
                "Surface paint source exceeds this device's table limits".into(),
            ));
        }
        let paged = source.width > limits.max_texture_dimension_2d
            || source.height > limits.max_texture_dimension_2d
            || u64::from(source.width)
                .saturating_mul(u64::from(source.height))
                .saturating_mul(8)
                > self.plane_limit;
        let (state, previous) = if paged {
            (
                State::Index(
                    SurfacePaintIndexBuilder::new(
                        source.clone(),
                        u64::from(limits.max_storage_buffer_binding_size).saturating_sub(20),
                    )
                    .map_err(RenderError)?,
                ),
                None,
            )
        } else {
            (
                State::Validate(SurfacePaintValidation::new(source.clone()).map_err(RenderError)?),
                self.bindings
                    .values()
                    .find_map(|old| reuse::Previous::candidate(old, source)),
            )
        };
        self.bindings.insert(
            identity,
            BoundSource {
                source: source.clone(),
                state,
                previous,
            },
        );
        self.pending.push_back(identity);
        Ok(())
    }
    fn advance(&mut self, context: &GpuContext) -> Result<()> {
        if context.is_lost() {
            return Err(RenderError("Surface paint device was lost".into()));
        }
        if let Some(error) = self.bindings.values().find_map(|entry| match &entry.state {
            State::Failed(error) => Some(error.clone()),
            _ => None,
        }) {
            return Err(RenderError(error));
        }
        while !self.pending.is_empty()
            && self.frame_bytes + PAGE_BYTES <= FRAME_BYTES
            && self.frame_steps < FRAME_STEPS
        {
            let started = self
                .frame_started
                .get_or_insert_with(web_time::Instant::now);
            if started.elapsed() >= web_time::Duration::from_millis(2) {
                break;
            }
            let identity = self.pending.pop_front().unwrap();
            let Some(entry) = self.bindings.get_mut(&identity) else {
                continue;
            };
            let units = (FRAME_STEPS - self.frame_steps).min(64);
            let uploading = matches!(entry.state, State::Upload(_));
            let mut step = SourceStep::default();
            let result = Self::advance_source(
                entry,
                context,
                units,
                FRAME_BYTES - self.frame_bytes,
                &mut step,
            );
            self.frame_steps += if uploading { 1 } else { units };
            self.frame_bytes += step.queued;
            self.gpu_copied_bytes += step.copied;
            self.reused_pages += u64::from(step.reused);
            if uploading && step.queued > 0 {
                self.uploaded_pages += 1;
            }
            match result {
                Ok(_) => (),
                Err(error) => {
                    entry.state = State::Failed(error.to_string());
                    return Err(error);
                }
            }
            if !matches!(entry.state, State::Ready(_) | State::Indexed(_)) {
                self.pending.push_back(identity);
            }
        }
        Ok(())
    }
    fn advance_source(
        entry: &mut BoundSource,
        context: &GpuContext,
        units: usize,
        available_bytes: usize,
        step: &mut SourceStep,
    ) -> Result<()> {
        match &mut entry.state {
            State::Index(builder) => {
                if builder.advance(units).map_err(RenderError)? {
                    let State::Index(builder) = std::mem::replace(
                        &mut entry.state,
                        State::Failed("consumed paint index".into()),
                    ) else {
                        unreachable!()
                    };
                    entry.state = State::Indexed(Arc::new(builder.finish().map_err(RenderError)?));
                }
            }
            State::Indexed(_) => (),
            State::Validate(validation) => {
                if validation.advance(units).map_err(RenderError)? {
                    let descriptor = |format| ImageDescriptor {
                        width: entry.source.width,
                        height: entry.source.height,
                        format,
                        color_space: ColorSpace::Srgb,
                        alpha_mode: AlphaMode::Straight,
                    };
                    let color =
                        GpuImage::render_target(context, descriptor(wgpu::TextureFormat::Rgba8Unorm))?;
                    let material =
                        GpuImage::render_target(context, descriptor(wgpu::TextureFormat::Rgba8Uint))?;
                    let previous = match entry.previous.take() {
                        Some(previous)
                            if previous
                                .binding
                                .copy_planes_into(context, &color, &material)? =>
                        {
                            step.copied =
                                u64::from(entry.source.width) * u64::from(entry.source.height) * 8;
                            Some(previous.source)
                        }
                        _ => None,
                    };
                    entry.state = State::Upload(Upload {
                        color,
                        material,
                        cursor: 0,
                        previous,
                    });
                }
            }
            State::Upload(upload) => {
                let page = &entry.source.pages[upload.cursor];
                let reuse = upload
                    .previous
                    .as_ref()
                    .and_then(|previous| previous.pages.get(upload.cursor))
                    .is_some_and(|old| reuse::same_page(old, page));
                let origin = wgpu::Origin3d {
                    x: page.atlas_origin[0],
                    y: page.atlas_origin[1],
                    z: 0,
                };
                if !reuse {
                    for (image, pixels) in [
                        (&upload.color, &page.color_rgba),
                        (&upload.material, &page.material_rgba),
                    ] {
                        context.queue.write_texture(
                            wgpu::TexelCopyTextureInfo {
                                texture: &image.texture,
                                mip_level: 0,
                                origin,
                                aspect: wgpu::TextureAspect::All,
                            },
                            pixels,
                            wgpu::TexelCopyBufferLayout {
                                offset: 0,
                                bytes_per_row: Some(PAGE_SIZE * 4),
                                rows_per_image: Some(PAGE_SIZE),
                            },
                            wgpu::Extent3d {
                                width: PAGE_SIZE,
                                height: PAGE_SIZE,
                                depth_or_array_layers: 1,
                            },
                        );
                    }
                    context.record_upload(PAGE_BYTES as u64);
                    step.queued += PAGE_BYTES;
                } else {
                    step.reused = true;
                }
                upload.cursor += 1;
                if upload.cursor == entry.source.pages.len() {
                    entry.state = State::Binding(PaintPageBindingPreparation::new_shared_alpha(
                        context,
                        upload.color.clone(),
                        upload.material.clone(),
                        entry.source.entries.clone(),
                        entry.source.alpha_geo_ids.clone(),
                    )?);
                }
            }
            State::Binding(preparation) => {
                if preparation.advance(units)? {
                    let bytes = preparation.upload(context, available_bytes)?;
                    step.queued += bytes;
                    if preparation.upload_ready() {
                        entry.state = State::Ready(preparation.finish(context)?);
                    }
                    return Ok(());
                }
            }
            State::Failed(error) => return Err(RenderError(error.clone())),
            State::Ready(_) => (),
        }
        Ok(())
    }
    fn ready(&self, source: &Arc<SurfacePaint>) -> Option<Arc<PaintPageBinding>> {
        match &self.bindings.get(&(Arc::as_ptr(source) as usize))?.state {
            State::Ready(binding) => Some(binding.clone()),
            State::Validate(_)
            | State::Upload(_)
            | State::Binding(_)
            | State::Failed(_)
            | State::Index(_)
            | State::Indexed(_) => None,
        }
    }
    fn index(&self, source: &Arc<SurfacePaint>) -> Option<Arc<PaintPageIndex>> {
        if let State::Indexed(index) = &self.bindings.get(&(Arc::as_ptr(source) as usize))?.state {
            Some(index.clone())
        } else {
            None
        }
    }
    fn stats(&self) -> SurfacePaintStats {
        SurfacePaintStats {
            sources: self.bindings.len(),
            logical_cpu_page_bytes: self
                .bindings
                .values()
                .map(|entry| {
                    entry
                        .source
                        .pages
                        .iter()
                        .map(|page| (page.color_rgba.len() + page.material_rgba.len()) as u64)
                        .sum::<u64>()
                })
                .sum(),
            gpu_plane_bytes: self
                .bindings
                .values()
                .map(|entry| {
                    if matches!(
                        entry.state,
                        State::Validate(_) | State::Failed(_) | State::Index(_) | State::Indexed(_)
                    ) {
                        0
                    } else {
                        u64::from(entry.source.width) * u64::from(entry.source.height) * 8
                    }
                })
                .sum(),
            gpu_table_bytes: self
                .bindings
                .values()
                .map(|entry| match &entry.state {
                    State::Binding(preparation) => preparation.raster_table_bytes(),
                    State::Ready(binding) => binding.raster_table_bytes(),
                    _ => 0,
                })
                .sum(),
            uploaded_pages: self.uploaded_pages,
            reused_pages: self.reused_pages,
            gpu_copied_bytes: self.gpu_copied_bytes,
            reused_bindings: self.reused_bindings,
            pending_sources: self.pending.len(),
            frame_uploaded_bytes: self.frame_bytes,
            pending_views: 0,
        }
    }
}
impl SceneViews {
    pub fn surface_paint_stats(&self) -> SurfacePaintStats {
        let mut stats = self.surface_paint.stats();
        for view in self.views.values() {
            if let Some(usage) = view.scene.residency_usage() {
                stats.gpu_plane_bytes += usage.gpu_plane_bytes;
                stats.gpu_table_bytes += usage.gpu_table_bytes;
            }
            stats.pending_views +=
                usize::from(view.scene.pending() || view.scene.preparation.is_some());
        }
        stats
    }
    /// One budget covers every view in an application frame, including shared
    /// sources. The canonical callers invoke this once before their view loops.
    pub fn begin_surface_paint_upload_frame(&mut self) {
        self.source_upload_budget.reset();
        self.surface_paint.view_turn = self.surface_paint.view_turn.wrapping_add(1);
        self.surface_paint.frame_bytes = 0;
        self.surface_paint.frame_steps = 0;
        self.surface_paint.frame_started = None;
    }
    pub fn has_pending_surface_paint_uploads(&self) -> bool {
        !self.surface_paint.pending.is_empty()
    }
    /// A source installed while another view rendered still needs its own
    /// output/receipt before an otherwise-idle host may stop requesting frames.
    pub fn has_pending_surface_paint_frames(&self) -> bool {
        self.has_pending_surface_paint_uploads()
            || self.views.values().any(|view| {
                view.size.0 > 0
                    && view.size.1 > 0
                    && (view.scene.pending()
                        || view.scene.preparation.is_some()
                        || (!view.surface_paint_ready
                            && view
                                .surface_paint_source
                                .as_ref()
                                .is_some_and(|source| self.surface_paint.index(source).is_some())))
            })
            || self.views.values().any(|view| {
                (view.surface_paint_source.is_some() || !view.scene_sources_ready)
                    && view.surface_paint_ready
                    && view.stamp_paint_ready
                    && view.dirty
                    && view.size.0 > 0
                    && view.size.1 > 0
            })
    }
    pub fn surface_paint_ready(&self, id: ViewId) -> Result<bool> {
        self.views
            .get(&id)
            .map(|view| {
                view.surface_paint_ready
                    && !view.scene.pending()
                    && view.scene.preparation.is_none()
            })
            .ok_or_else(|| RenderError("unknown scene view".into()))
    }
    pub fn advance_surface_paint_uploads(&mut self) -> Result<()> {
        self.surface_paint.advance(&self.context)?;
        self.advance_paged_surface_paint()?;
        let mut installed = false;
        for view in self
            .views
            .values_mut()
            .filter(|view| !view.surface_paint_ready)
        {
            if let Some(binding) = view
                .surface_paint_source
                .as_ref()
                .and_then(|source| self.surface_paint.ready(source))
            {
                view.scene.retire_paint();
                view.scene.vm.set_active_vm(0);
                view.scene
                    .vm
                    .set_paint_page_binding(&self.context, binding)?;
                view.installed_surface_paint_source = view.surface_paint_source.clone();
                installed = true;
                view.surface_paint_ready = true;
                view.dirty = true;
            }
        }
        if installed {
            self.retire_unused_surface_paint();
        }
        Ok(())
    }
    pub(super) fn retire_unused_surface_paint(&mut self) {
        let live: std::collections::HashSet<_> = self
            .views
            .values()
            .flat_map(|view| {
                [
                    view.surface_paint_source.as_ref(),
                    view.installed_surface_paint_source.as_ref(),
                ]
            })
            .flatten()
            .map(|source| Arc::as_ptr(source) as usize)
            .collect();
        self.surface_paint
            .bindings
            .retain(|identity, _| live.contains(identity));
        self.surface_paint
            .pending
            .retain(|identity| live.contains(identity));
    }
    /// Retire an unfinished GPU request when its CPU producer has been replaced.
    /// The previous complete VM binding/output may remain until the replacement.
    pub fn defer_surface_paint_source(&mut self, id: ViewId) -> Result<()> {
        let view = self
            .views
            .get_mut(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        view.scene.vm.cancel_source_uploads();
        view.scene.retire_paint();
        view.surface_paint_source = None;
        view.surface_paint_ready = false;
        view.pending_content_revision = None;
        view.dirty = true;
        self.retire_unused_surface_paint();
        Ok(())
    }
    /// Hide/detach an owner without retaining partially uploaded GPU images.
    /// Complete displayed bindings/output remain valid until their normal retire.
    pub fn cancel_surface_paint_uploads(&mut self) {
        for view in self.views.values_mut() {
            view.scene.vm.cancel_source_uploads();
            view.scene.retire_paint();
            view.scene_sources_ready = true;
        }
        for view in self
            .views
            .values_mut()
            .filter(|view| !view.surface_paint_ready)
        {
            view.surface_paint_source = None;
        }
        self.retire_unused_surface_paint();
    }
    /// Request immutable CPU pages. Pending uploads remain private; render()
    /// returns no new output until this source is fully installed.
    pub fn set_surface_paint_source(
        &mut self,
        id: ViewId,
        source: Option<&Arc<SurfacePaint>>,
    ) -> Result<()> {
        let view = self
            .views
            .get(&id)
            .ok_or_else(|| RenderError("unknown scene view".into()))?;
        let same = match (&view.surface_paint_source, source) {
            (None, None) => view.surface_paint_ready,
            (Some(old), Some(next)) => Arc::ptr_eq(old, next),
            _ => false,
        };
        if same {
            return Ok(());
        }
        if let Some(source) = source {
            self.surface_paint.request(&self.context, source)?;
        }
        let view = self.views.get_mut(&id).unwrap();
        view.scene.retire_paint();
        view.scene.vm.cancel_source_uploads();
        view.game_scope = None;
        view.scene_sources_ready = true;
        view.pending_content_revision = None;
        view.surface_paint_source = source.cloned();
        view.surface_paint_ready = source.is_none();
        view.scene.vm.set_active_vm(0);
        if source.is_none() {
            view.installed_surface_paint_source = None;
            view.scene
                .vm
                .execute(scenevm::Atom::ClearRaster3DPaintOverlay);
        }
        view.dirty = true;
        self.retire_unused_surface_paint();
        Ok(())
    }
}
