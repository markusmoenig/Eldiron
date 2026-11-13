//! Application-owned appearance GPU output and acknowledged playback receipts.
use super::*;
use renderkit::{GpuContext, GpuImage, ImageHandle, ImageRegistry};
use scenevm::{SceneVM, render_output::SceneRenderTarget};
const TARGET_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreviewToken {
    owner: u64,
    serial: u64,
}
impl PreviewToken {
    pub(crate) fn receipt(self) -> theframework::paint::FrameReceipt {
        theframework::paint::FrameReceipt {
            owner: self.owner,
            serial: self.serial,
        }
    }
}

struct Frame {
    image: Arc<GpuImage>,
    phase: f32,
}
#[derive(Default)]
struct CpuState {
    resources: Option<Arc<rusterix::Assets>>,
    effects: rusterix::procedural_avatar::effects::Player,
}
impl CpuState {
    fn source(&mut self, assets: &Arc<rusterix::Assets>) {
        if self
            .resources
            .as_ref()
            .is_none_or(|r| !Arc::ptr_eq(r, assets))
        {
            self.resources = Some(assets.clone());
            self.effects = Default::default();
        }
    }
}
struct Producer {
    context: Arc<GpuContext>,
    vm: SceneVM,
    uploaded: FxHashSet<Uuid>,
    resources: Option<Arc<rusterix::Assets>>,
    targets: Vec<SceneRenderTarget>,
}
impl Producer {
    fn new(context: Arc<GpuContext>) -> Self {
        Self {
            vm: SceneVM::new_with_context(context.clone(), 1, 1),
            context,
            uploaded: Default::default(),
            resources: None,
            targets: Vec::new(),
        }
    }
    fn render(
        &mut self,
        job: &preview::Request,
        density: f32,
        cpu: &mut CpuState,
    ) -> Result<Option<Frame>, String> {
        if self.context.is_lost() {
            return Err("Appearance preview device was lost".into());
        }
        if job.width == 0
            || job.height == 0
            || !density.is_finite()
            || density <= 0.
            || !job.zoom.is_finite()
            || job.zoom <= 0.
            || !job.yaw.is_finite()
            || !job.phase.is_finite()
            || !job.seconds.is_finite()
        {
            return Err("Invalid appearance preview dimensions/camera/time".into());
        }
        let physical = [job.width, job.height].map(|v| (v as f64 * density as f64).round());
        if physical
            .iter()
            .any(|v| *v < 1. || *v > self.context.device.limits().max_texture_dimension_2d as f64)
        {
            return Err("Appearance preview exceeds GPU texture limits".into());
        }
        let [w, h] = physical.map(|v| v as u32);
        let bytes = w as u64 * h as u64 * 4;
        if bytes * 2 > TARGET_BYTES {
            return Err("Appearance preview exceeds its output/history budget".into());
        }
        let leased = |target: &SceneRenderTarget| Arc::strong_count(&target.image()) > 2;
        self.targets.retain(|t| leased(t) || t.size() == (w, h));
        let index = if let Some(index) = self
            .targets
            .iter()
            .position(|t| !leased(t) && t.size() == (w, h))
        {
            index
        } else {
            let resident: u64 = self
                .targets
                .iter()
                .map(|t| t.size().0 as u64 * t.size().1 as u64 * 4)
                .sum();
            if self.targets.len() >= 3 || resident + bytes * 2 > TARGET_BYTES {
                return Ok(None);
            }
            self.targets.push(
                SceneRenderTarget::new(self.context.clone(), w, h).map_err(|e| e.to_string())?,
            );
            self.targets.len() - 1
        };
        if self
            .resources
            .as_ref()
            .is_none_or(|r| !Arc::ptr_eq(r, &job.assets))
        {
            self.vm.execute(scenevm::Atom::ClearTiles);
            self.uploaded.clear();
            self.resources = Some(job.assets.clone());
        }
        cpu.source(&job.assets);
        let mut physical_job = job.clone();
        physical_job.width = w;
        physical_job.height = h;
        let phase = preview::prepare_with_effects(
            &mut self.vm,
            &mut self.uploaded,
            &physical_job,
            &mut cpu.effects,
        )?;
        let output = self
            .vm
            .render_gpu(&mut self.targets[index], job.revision)
            .map_err(|e| e.to_string())?;
        Ok(Some(Frame {
            image: output.image,
            phase,
        }))
    }
}
struct Request {
    serial: u64,
    job: preview::Request,
}
struct Published {
    token: PreviewToken,
    image: ImageHandle,
    frame: Frame,
    revision: u64,
    request: u64,
    density: u32,
    owner: Owner,
}
pub(super) struct Host {
    pub enabled: bool,
    owner: u64,
    serial: u64,
    request_serial: u64,
    latest: Option<Request>,
    producer: Option<Producer>,
    cpu: CpuState,
    registry: Option<u64>,
    staged: Option<Published>,
    displayed: Option<Published>,
    retired: Vec<ImageHandle>,
    pub waiting: bool,
    error: Option<String>,
    failed: Option<(u64, u32)>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            enabled: false,
            owner: theframework::paint::new_receipt_owner(),
            serial: 0,
            request_serial: 0,
            latest: None,
            producer: None,
            cpu: Default::default(),
            registry: None,
            staged: None,
            displayed: None,
            retired: Vec::new(),
            waiting: false,
            error: None,
            failed: None,
        }
    }
}
impl Host {
    pub fn background_changed(&self, color: [u8; 4]) -> bool {
        self.enabled
            && self
                .latest
                .as_ref()
                .is_some_and(|r| r.job.background != color)
    }
    pub fn queue(&mut self, job: preview::Request) {
        self.request_serial += 1;
        self.failed = None;
        self.latest = Some(Request {
            serial: self.request_serial,
            job,
        });
    }
    pub fn invalidate(&mut self) {
        self.latest = None;
        self.cpu = Default::default();
        self.retire_gpu();
    }
    fn retire_gpu(&mut self) {
        self.producer = None;
        self.waiting = false;
        self.error = None;
        self.failed = None;
        if let Some(p) = self.staged.take() {
            self.retired.push(p.image);
        }
        if let Some(p) = self.displayed.take() {
            self.retired.push(p.image);
        }
    }
    fn fallback(&self) -> Option<PreviewToken> {
        self.staged
            .as_ref()
            .or(self.displayed.as_ref())
            .map(|p| p.token)
    }
    fn drain(&mut self, images: &mut ImageRegistry) {
        if self.registry != Some(images.owner_id()) {
            self.retire_gpu();
            self.registry = Some(images.owner_id());
            self.retired.clear();
        } else {
            for image in self.retired.drain(..) {
                images.remove(image);
            }
        }
    }
}
impl AppearanceWorkspace {
    pub(crate) fn enable_gpu_preview(&mut self) {
        self.native.enabled = true;
        self.worker = None;
        self.pending = false;
        self.dirty = true;
    }
    pub(crate) fn invalidate_gpu_preview(&mut self, ui: &mut TheUI) {
        self.native.retire_gpu();
        if let Some(view) = ui.get_render_view(VIEW) {
            let _ = view.set_paint_list(None);
        }
    }
    pub(crate) fn gpu_preview_waiting(&self) -> bool {
        self.native.waiting
    }
    pub(crate) fn prepare_gpu_preview(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut ImageRegistry,
    ) -> Option<PreviewToken> {
        self.native.waiting = false;
        let registry_changed = self.native.registry != Some(images.owner_id());
        self.native.drain(images);
        if registry_changed {
            if let Some(view) = ui.get_render_view(VIEW) {
                let _ = view.set_paint_list(None);
            }
        }
        if !self.native.enabled {
            return None;
        }
        let drawable = ui
            .get_render_view(VIEW)
            .is_some_and(|view| view.dim().is_valid());
        if !self.visible(ui) || self.owner.is_none() || !drawable {
            self.native.latest = None;
            self.native.retire_gpu();
            self.native.drain(images);
            self.dirty = true;
            if let Some(view) = ui.get_render_view(VIEW) {
                let _ = view.set_paint_list(None);
            }
            return None;
        }
        match self.prepare_gpu_output(ui, ctx, images) {
            Ok(token) => token,
            Err(error) => {
                if self.native.error.as_ref() != Some(&error) {
                    ui.set_widget_value(
                        "Definition Appearance Status",
                        ctx,
                        TheValue::Text(error.clone()),
                    );
                    self.native.error = Some(error);
                }
                self.native.failed = self
                    .native
                    .latest
                    .as_ref()
                    .map(|r| (r.serial, ctx.scale_factor.to_bits()));
                self.native.waiting = false;
                self.playing = false;
                self.effects_active = false;
                self.dirty = false;
                if let Some(button) = ui
                    .get_widget("Definition Appearance Play")
                    .and_then(|w| w.as_group_button())
                {
                    button.set_index(0);
                }
                self.native.fallback()
            }
        }
    }
    fn prepare_gpu_output(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut ImageRegistry,
    ) -> Result<Option<PreviewToken>, String> {
        let Some(request) = self.native.latest.as_ref() else {
            return Ok(None);
        };
        let Some(owner) = self.owner else {
            return Ok(None);
        };
        if request.job.revision != self.revision {
            return Ok(self.native.fallback());
        }
        let same = |p: &Published| {
            p.request == request.serial
                && p.revision == self.revision
                && p.owner == owner
                && p.density == ctx.scale_factor.to_bits()
        };
        if let Some(p) = self.native.staged.as_ref().filter(|p| same(p)) {
            return Ok(Some(p.token));
        }
        if let Some(p) = self.native.displayed.as_ref().filter(|p| same(p)) {
            return Ok(Some(p.token));
        }
        if self.native.failed == Some((request.serial, ctx.scale_factor.to_bits())) {
            return Ok(self.native.fallback());
        }
        if self
            .native
            .producer
            .as_ref()
            .is_none_or(|p| p.context.generation() != images.context().generation())
        {
            self.native.producer = Some(Producer::new(images.context().clone()));
        }
        let Some(frame) = self.native.producer.as_mut().unwrap().render(
            &request.job,
            ctx.scale_factor,
            &mut self.native.cpu,
        )?
        else {
            self.native.waiting = true;
            return Ok(self.native.fallback());
        };
        let image = images
            .insert(frame.image.clone())
            .map_err(|e| e.to_string())?;
        let mut list = theframework::paint::PaintList::default();
        list.image(
            image,
            theframework::paint::Rect::new(
                0.,
                0.,
                request.job.width as f32,
                request.job.height as f32,
            ),
            theframework::paint::Sampling::Nearest,
        );
        self.native.serial = self
            .native
            .serial
            .checked_add(1)
            .expect("preview receipt space exhausted");
        let token = PreviewToken {
            owner: self.native.owner,
            serial: self.native.serial,
        };
        list.ops
            .push(theframework::paint::PaintOp::Receipt(token.receipt()));
        let Some(view) = ui.get_render_view(VIEW) else {
            images.remove(image);
            return Ok(None);
        };
        if let Err(error) = view.set_paint_list(Some(Arc::new(list))) {
            images.remove(image);
            return Err(error.to_string());
        }
        if let Some(old) = self.native.staged.replace(Published {
            token,
            image,
            frame,
            revision: self.revision,
            request: request.serial,
            density: ctx.scale_factor.to_bits(),
            owner,
        }) {
            self.native.retired.push(old.image);
        }
        self.native.error = None;
        Ok(Some(token))
    }
    pub(crate) fn presented_gpu_preview(
        &mut self,
        token: PreviewToken,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> bool {
        let Some(frame) = self
            .native
            .staged
            .as_ref()
            .filter(|p| p.token == token && Some(p.owner) == self.owner)
        else {
            return false;
        };
        let phase = frame.frame.phase;
        self.playhead = phase;
        if !ctx
            .ui
            .focus
            .as_ref()
            .is_some_and(|id| id.name == "Definition Appearance Phase")
        {
            ui.set_widget_value("Definition Appearance Phase", ctx, TheValue::Float(phase));
        }
        if self.native.error.is_none() {
            ui.set_widget_value(
                "Definition Appearance Status",
                ctx,
                TheValue::Text(
                    if matches!(self.owner, Some(Owner::Character(_))) {
                        "Outfit from character setup · drag to turn · scroll to zoom"
                    } else {
                        "Item preview · drag to turn · scroll to zoom"
                    }
                    .into(),
                ),
            );
        }
        let frame = self.native.staged.take().unwrap();
        if let Some(old) = self.native.displayed.replace(frame) {
            self.native.retired.push(old.image);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderkit::{ImageDescriptor, ReadbackTicket};
    fn job() -> preview::Request {
        let mut assets = rusterix::Assets::new();
        assets.config = "[appearance]\nsprite_pixels_2d=32\npixel_size_3d=4\n".into();
        assets.palette = ThePalette::default();
        assets.procedural_avatars.insert(
            "human".into(),
            Arc::new(
                rusterix::procedural_avatar::Appearance::new(
                    denrim_materials::geometry::HUMAN_EXAMPLE,
                    &[],
                    11,
                )
                .unwrap(),
            ),
        );
        let mut entity = rusterix::Entity::default();
        entity.set_attribute("avatar", rusterix::Value::Str("human".into()));
        let mut item = rusterix::Item::default();
        item.set_attribute("appearance_geometry", rusterix::Value::Str("human".into()));
        preview::Request {
            revision: 1,
            assets: Arc::new(assets),
            entity,
            item: Some(item),
            icon: Some(rusterix::Texture::new(
                vec![
                    240, 40, 20, 255, 20, 200, 80, 128, 9, 40, 230, 0, 9, 40, 230, 255,
                ],
                2,
                2,
            )),
            ground: false,
            two_d: false,
            clip: "idle".into(),
            seconds: 0.,
            phase: 0.3,
            playing: false,
            yaw: 45.,
            zoom: 1.,
            width: 128,
            height: 96,
            background: [24, 22, 20, 255],
        }
    }
    fn status_glyphs(
        ui: &mut TheUI,
        ctx: &mut TheContext,
        gpu: &Arc<GpuContext>,
    ) -> Vec<(u32, [f32; 2])> {
        let mut list = theframework::paint::PaintList::default();
        let mut images = theframework::render_wgpu::UiImages::new(gpu.clone());
        let mut theme = TheBlackBlueTheme::new();
        let mut painter = theframework::theui::paint::WidgetPainter {
            list: &mut list,
            text: &mut ctx.text,
            images: &mut images,
            ui: &ctx.ui,
            theme: &mut theme,
        };
        ui.get_widget("Definition Appearance Status")
            .unwrap()
            .paint(&mut painter)
            .unwrap();
        list.ops
            .iter()
            .flat_map(|op| match op {
                theframework::paint::PaintOp::GlyphRun { run, .. } => run
                    .glyphs
                    .iter()
                    .map(|g| (g.id, g.position))
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect()
    }
    fn capture(gpu: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
        let mut ticket = ReadbackTicket::capture(gpu, image).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_appearance_outputs_match_reference_preserve_leases_and_have_no_color_staging() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut producer = Producer::new(gpu.clone());
        let mut cpu = CpuState::default();
        let mut reference = SceneVM::new_with_context(gpu.clone(), 1, 1);
        let mut uploaded = Default::default();
        let mut job = job();
        for (two_d, ground) in [(false, false), (true, false), (false, true), (true, true)] {
            job.two_d = two_d;
            job.ground = ground;
            let before = gpu.transfers();
            let frame = producer.render(&job, 1., &mut cpu).unwrap().unwrap();
            assert!((frame.phase - 0.3).abs() < 0.0001);
            assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
            assert_eq!(producer.vm.render_target_cpu_bytes(), 0);
            assert!(
                producer
                    .targets
                    .iter()
                    .all(|t| !t.has_cpu_storage() && !t.has_readback_storage())
            );
            let actual = capture(&gpu, frame.image.clone());
            let expected = preview::render(&mut reference, &mut uploaded, &job)
                .unwrap()
                .0;
            assert_eq!(actual, expected.pixels(), "2d={two_d} ground={ground}");
            assert!(
                actual
                    .chunks_exact(4)
                    .map(|p| p[0])
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    > 3,
                "actual model or icon must be rendered"
            );
        }
        let first = producer.render(&job, 1., &mut cpu).unwrap().unwrap();
        let original = capture(&gpu, first.image.clone());
        let second = producer.render(&job, 1., &mut cpu).unwrap().unwrap();
        let third = producer.render(&job, 1., &mut cpu).unwrap().unwrap();
        assert!(producer.render(&job, 1., &mut cpu).unwrap().is_none());
        assert_eq!(capture(&gpu, first.image.clone()), original);
        drop(second);
        drop(third);
        let scaled = producer.render(&job, 1.5, &mut cpu).unwrap().unwrap();
        assert_eq!(
            (
                scaled.image.descriptor.width,
                scaled.image.descriptor.height
            ),
            (192, 144)
        );
        assert!(producer.render(&job, f32::NAN, &mut cpu).is_err());
        job.zoom = 0.;
        assert!(producer.render(&job, 1., &mut cpu).is_err());
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_appearance_host_queues_without_worker_and_promotes_only_painted_playback() {
    let reference = CreatorReference::default();
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(640, 480, 1.);
        ctx.ui.command_paint = true;
        ui.canvas = main_canvas(TheCanvas::new());
        AppearanceWorkspace::show(&mut ui, &mut ctx, true);
        ui.canvas.set_dim(TheDim::new(0, 0, 640, 480), &mut ctx);
        ui.prepare_paint_layout(&mut ctx);
        let mut project = Project::default();
        let mut character = Character::new();
        character.name = "Human".into();
        character.data = "[attributes]\navatar='human'\n".into();
        let id = character.id;
        project.characters.insert(id, character);
        let owner = Owner::Character(id);
        let source = job();
        let key = rusterix::procedural_avatar::fingerprint(&(owner, owner.data(&project), 0u64));
        let mut host = AppearanceWorkspace {
            owner: Some(owner),
            key: Some(key),
            snapshot: Some(source.assets.clone()),
            phase: 0.3,
            zoom: 1.,
            yaw: 45.,
            ..Default::default()
        };
        host.enable_gpu_preview();
        let original = serde_json::to_value(&project).unwrap();
        assert!(host.update(&project, &mut ui, &mut ctx, 0, &reference));
        assert!(host.worker.is_none());
        assert!(!host.pending);
        let mut images = ImageRegistry::new(gpu.clone());
        let token = host
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap();
        assert_eq!(host.playhead, 0.);
        assert!(host.presented_gpu_preview(token, &mut ui, &mut ctx));
        assert!((host.playhead - 0.3).abs() < 0.0001);
        host.phase = 0.6;
        host.invalidate();
        host.update(&project, &mut ui, &mut ctx, 0, &reference);
        let pending = host
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap();
        assert!((host.playhead - 0.3).abs() < 0.0001);
        host.phase = 0.8;
        host.invalidate();
        host.update(&project, &mut ui, &mut ctx, 0, &reference);
        let newest = host
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap();
        assert!(!host.presented_gpu_preview(pending, &mut ui, &mut ctx));
        // Reject a failed source without replacing the successfully staged paint.
        host.native.latest.as_mut().unwrap().job.zoom = f32::NAN;
        host.native.latest.as_mut().unwrap().serial += 1;
        assert_eq!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut images),
            Some(newest)
        );
        assert!(host.native.error.is_some());
        let error_glyphs = status_glyphs(&mut ui, &mut ctx, &gpu);
        assert!(!error_glyphs.is_empty());
        assert!(host.presented_gpu_preview(newest, &mut ui, &mut ctx));
        assert!((host.playhead - 0.8).abs() < 0.0001);
        assert_eq!(
            status_glyphs(&mut ui, &mut ctx, &gpu),
            error_glyphs,
            "fallback acknowledgement must retain the newer failure text"
        );
        let transfers = gpu.transfers();
        assert_eq!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut images),
            Some(newest)
        );
        assert_eq!(
            gpu.transfers(),
            transfers,
            "failed requests are not retried each frame"
        );
        host.native.latest.as_mut().unwrap().job.zoom = 1.;
        let old_image = host.native.displayed.as_ref().unwrap().image;
        let source_cpu = host.native.cpu.resources.clone().unwrap();
        host.invalidate_gpu_preview(&mut ui);
        assert!(Arc::ptr_eq(
            host.native.cpu.resources.as_ref().unwrap(),
            &source_cpu
        ));
        let mut replacement =
            ImageRegistry::new(pollster::block_on(GpuContext::headless()).unwrap());
        let rebuilt = host
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
            .unwrap();
        assert!(!host.presented_gpu_preview(newest, &mut ui, &mut ctx));
        assert!(host.presented_gpu_preview(rebuilt, &mut ui, &mut ctx));
        assert!(Arc::ptr_eq(
            host.native.cpu.resources.as_ref().unwrap(),
            &source_cpu
        ));
        assert!(
            images.get(old_image).is_some(),
            "recovery cannot retire a foreign registry's handles"
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
        let frame = host.native.displayed.as_ref().unwrap();
        let target = GpuImage::new(replacement.context(), ImageDescriptor::rgba(640, 480)).unwrap();
        let mut paint = theframework::paint::PaintList::default();
        paint.image(
            frame.image,
            theframework::paint::Rect::new(0., 0., 640., 480.),
            theframework::paint::Sampling::Nearest,
        );
        theframework::render_wgpu::PaintRenderer::new(replacement.context().clone())
            .unwrap()
            .render(
                &paint,
                &replacement,
                &target.view,
                target.descriptor.format,
                (640, 480),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(replacement.context().transfers().readback_bytes, 0);
        let image = frame.image;
        AppearanceWorkspace::show(&mut ui, &mut ctx, false);
        assert!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
                .is_none()
        );
        assert!(replacement.get(image).is_none());
        AppearanceWorkspace::show(&mut ui, &mut ctx, true);
        assert!(
            host.update(&project, &mut ui, &mut ctx, 0, &reference),
            "a paused view must queue again after reopening"
        );
        let reopened = host
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
            .unwrap();
        assert!(host.presented_gpu_preview(reopened, &mut ui, &mut ctx));
        let image = host.native.displayed.as_ref().unwrap().image;
        let dim = *ui.get_render_view(VIEW).unwrap().dim();
        ui.get_render_view(VIEW)
            .unwrap()
            .set_dim(TheDim::zero(), &mut ctx);
        assert!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
                .is_none(),
            "a zero-sized view cannot acknowledge unpainted output"
        );
        assert!(replacement.get(image).is_none());
        assert!(Arc::ptr_eq(
            host.native.cpu.resources.as_ref().unwrap(),
            &source_cpu
        ));
        ui.get_render_view(VIEW).unwrap().set_dim(dim, &mut ctx);
        assert!(host.update(&project, &mut ui, &mut ctx, 0, &reference));
        let visible = host
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
            .unwrap();
        assert!(host.presented_gpu_preview(visible, &mut ui, &mut ctx));
        let image = host.native.displayed.as_ref().unwrap().image;
        host.reset(&mut ui, &mut ctx);
        assert!(host.native.enabled);
        assert!(host.native.cpu.resources.is_none());
        assert!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
                .is_none()
        );
        assert!(replacement.get(image).is_none());
        assert!(!host.presented_gpu_preview(rebuilt, &mut ui, &mut ctx));
    }
}

/// Explicit test capture of an independently injected native renderer for the
/// shipped outfit review. Production display never requests this CPU image.
#[cfg(test)]
pub(super) fn assert_reference_pixels(job: &preview::Request, reference: &TheRGBABuffer) {
    let gpu = pollster::block_on(GpuContext::headless()).unwrap();
    let mut producer = Producer::new(gpu.clone());
    let mut cpu = CpuState::default();
    let frame = producer.render(job, 1., &mut cpu).unwrap().unwrap();
    assert_eq!(producer.vm.render_target_cpu_bytes(), 0);
    assert!(
        producer
            .targets
            .iter()
            .all(|t| !t.has_cpu_storage() && !t.has_readback_storage())
    );
    let mut capture = renderkit::ReadbackTicket::capture(&gpu, frame.image).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    let captured = capture.try_finish(&gpu).unwrap().unwrap();
    assert_eq!(
        captured.pixels(),
        reference.pixels(),
        "shipped appearance 2d={} ground={}",
        job.two_d,
        job.ground
    );
}
