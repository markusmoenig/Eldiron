//! Creator-owned geometry publication and completed-frame picking receipts.
use super::*;
use crate::geometry_editor::native_preview::{Frame, NativePreview};
use renderkit::{ImageHandle, ImageRegistry};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PreviewToken {
    owner: u64,
    serial: u64,
}
struct Published {
    token: PreviewToken,
    frame: Frame,
    image: ImageHandle,
    density: u32,
    source_revision: u64,
    source_key: String,
    selection: rigging::Selection,
    tool: usize,
    handles: Vec<(scene::Handle, [f32; 3])>,
}
pub(super) struct Host {
    pub enabled: bool,
    pub latest: Option<preview::Request>,
    pub waiting: bool,
    pub error: Option<String>,
    owner: u64,
    serial: u64,
    registry: Option<u64>,
    producer: Option<NativePreview>,
    staged: Option<Published>,
    displayed: Option<Published>,
    retired: Vec<ImageHandle>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            enabled: false,
            latest: None,
            waiting: false,
            error: None,
            owner: NEXT_OWNER.fetch_add(1, Ordering::Relaxed),
            serial: 0,
            registry: None,
            producer: None,
            staged: None,
            displayed: None,
            retired: Vec::new(),
        }
    }
}
impl Host {
    pub fn invalidate(&mut self) {
        self.latest = None;
        self.waiting = false;
        self.error = None;
        if let Some(frame) = self.staged.take() {
            self.retired.push(frame.image);
        }
        if let Some(frame) = self.displayed.take() {
            self.retired.push(frame.image);
        }
        self.producer = None;
    }
    fn drain(&mut self, images: &mut ImageRegistry) {
        if self.registry != Some(images.owner_id()) {
            self.invalidate();
            self.retired.clear();
            self.registry = Some(images.owner_id());
        } else {
            for image in self.retired.drain(..) {
                images.remove(image);
            }
        }
    }
}
impl GeometryEditor {
    pub(crate) fn invalidate_gpu_preview(&mut self, ui: &mut TheUI) {
        let latest = self.native.latest.take();
        self.native.invalidate();
        self.native.latest = latest;
        self.scene.projection = None;
        self.preview_key.clear();
        if self.camera != 3 && self.camera != 4 {
            if let Some(view) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                let _ = view.set_paint_list(None);
            }
        }
    }
    /// Select before the first CPU update. Converted preview requests must not
    /// discover a GPU through RUSTERIX or start the reference framebuffer worker.
    pub(crate) fn enable_gpu_preview(&mut self) {
        if !self.native.enabled {
            self.native.enabled = true;
            self.worker = None;
            self.preview_key.clear();
        }
    }
    pub(crate) fn prepare_gpu_preview(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut ImageRegistry,
    ) -> Result<Option<PreviewToken>, String> {
        self.native.waiting = false;
        let registry_changed = self.native.registry != Some(images.owner_id());
        // Keep the latest CPU request across GPU-domain replacement; all old
        // images/receipts belong to the previous registry and cannot be reused.
        let latest = if registry_changed {
            self.native.latest.take()
        } else {
            None
        };
        self.native.drain(images);
        if registry_changed {
            self.native.latest = latest;
            self.scene.projection = None;
            if let Some(view) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                view.set_paint_list(None).map_err(|e| e.to_string())?;
            }
        }
        if !self.native.enabled {
            return Ok(None);
        }
        if !self.wants_realtime() || self.selected.is_none() {
            self.native.invalidate();
            self.native.drain(images);
            self.scene.projection = None;
            if let Some(view) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                view.set_paint_list(None).map_err(|e| e.to_string())?;
            }
            return Ok(None);
        }
        // Contour and baked-isometric previews are separately tracked adapters
        // until their source/overlay contribution is converted.
        if self.camera == 3 || self.camera == 4 {
            self.native.invalidate();
            self.native.drain(images);
            return Ok(None);
        }
        let Some(request) = self.native.latest.as_ref() else {
            return Ok(None);
        };
        if request.epoch != self.preview_epoch {
            return Ok(None);
        }
        let same = |p: &Published| {
            p.frame.epoch == request.epoch
                && p.frame.revision == request.job.revision
                && p.density == ctx.scale_factor.to_bits()
                && p.selection == self.inspector
                && p.tool == self.scene.tool
                && p.source_revision == self.source_revision
                && Some(&p.source_key) == self.key.as_ref()
        };
        if let Some(staged) = self.native.staged.as_ref().filter(|p| same(p)) {
            return Ok(Some(staged.token));
        }
        if let Some(current) = self.native.displayed.as_ref().filter(|p| same(p)) {
            return Ok(Some(current.token));
        }
        if self
            .native
            .producer
            .as_ref()
            .is_none_or(|p| p.generation() != images.context().generation())
        {
            self.native.producer = Some(NativePreview::new(images.context().clone()));
        }
        self.native.waiting = false;
        let Some(frame) = self
            .native
            .producer
            .as_mut()
            .unwrap()
            .render(request, ctx.scale_factor)?
        else {
            self.native.waiting = true;
            return Ok(self.native_preview_fallback_token());
        };
        self.native.error = None;
        let mut scene = self.scene.clone();
        scene.projection = Some(frame.projection.clone());
        scene.posed = frame.posed.clone();
        scene.previewing_animation = self.playback.selected > 0;
        let (Some(doc), Some(evaluation)) = (&self.document, &self.evaluation) else {
            return Ok(None);
        };
        let handles = scene
            .handles(doc, evaluation, &self.inspector)
            .into_iter()
            .map(|(h, p, _)| (h, p))
            .collect();
        let mut list = frame.overlay.clone();
        list.ops.extend(
            scene
                .paint_native(doc, evaluation, &self.inspector, ui.style.theme().as_ref())
                .ops,
        );
        let image = images
            .insert(frame.image.clone())
            .map_err(|e| e.to_string())?;
        let mut paint = frame.paint(image);
        // Replace the frame's base guides with the assembled guides/handles.
        paint.ops.truncate(1);
        paint.ops.extend(list.ops);
        let Some(view) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) else {
            images.remove(image);
            return Ok(None);
        };
        if let Err(error) = view.set_paint_list(Some(Arc::new(paint))) {
            images.remove(image);
            return Err(error.to_string());
        }
        self.native.serial += 1;
        let token = PreviewToken {
            owner: self.native.owner,
            serial: self.native.serial,
        };
        if let Some(old) = self.native.staged.replace(Published {
            token,
            frame,
            image,
            density: ctx.scale_factor.to_bits(),
            source_revision: self.source_revision,
            source_key: self.key.clone().unwrap_or_default(),
            selection: self.inspector.clone(),
            tool: self.scene.tool,
            handles,
        }) {
            self.native.retired.push(old.image);
        }
        Ok(Some(token))
    }
    pub(crate) fn gpu_preview_waiting(&self) -> bool {
        self.native.waiting
    }
    pub(crate) fn native_preview_fallback_token(&self) -> Option<PreviewToken> {
        self.native
            .staged
            .as_ref()
            .or(self.native.displayed.as_ref())
            .map(|p| p.token)
    }
    pub(crate) fn report_gpu_preview_error(&mut self, message: String, ctx: &mut TheContext) {
        self.native.waiting = false;
        if self.native.error.as_ref() != Some(&message) {
            ctx.ui
                .send(TheEvent::SetStatusText(TheId::empty(), message.clone()));
            self.native.error = Some(message);
        }
    }
    /// Called only for the token included in a successful surface presentation.
    pub(crate) fn presented_gpu_preview(&mut self, token: PreviewToken) -> bool {
        let Some(frame) = self.native.staged.as_ref().filter(|p| p.token == token) else {
            return false;
        };
        if frame.frame.epoch != self.preview_epoch
            || frame.source_revision != self.source_revision
            || frame.selection != self.inspector
            || frame.tool != self.scene.tool
        {
            return false;
        }
        let frame = self.native.staged.take().unwrap();
        self.displayed_revision = frame.frame.revision;
        self.scene.projection = Some(frame.frame.projection.clone());
        self.scene.posed = frame.frame.posed.clone();
        self.scene.previewing_animation = self.playback.selected > 0;
        if let Some(old) = self.native.displayed.replace(frame) {
            self.native.retired.push(old.image);
        }
        true
    }
    pub(super) fn native_pick_ready(&self, project: &Project) -> bool {
        if !self.native.enabled || self.camera == 3 || self.camera == 4 {
            return true;
        }
        let Some(p) = self.native.displayed.as_ref() else {
            return false;
        };
        if p.frame.epoch != self.preview_epoch
            || p.source_revision != self.source_revision
            || p.selection != self.inspector
            || p.tool != self.scene.tool
        {
            return false;
        }
        self.selected
            .and_then(|id| {
                project
                    .procedural_appearances
                    .get(&id)
                    .map(|source| self.source_key(project, id, source))
            })
            .as_ref()
            == Some(&p.source_key)
    }
    pub(super) fn preview_handle_hit(
        &self,
        doc: &Document,
        e: &Evaluation,
        p: Vec2<i32>,
    ) -> Option<(scene::Handle, [f32; 3])> {
        if !self.native.enabled || self.camera == 3 || self.camera == 4 {
            return self.scene.hit(doc, e, &self.inspector, p);
        }
        let frame = self.native.displayed.as_ref()?;
        frame
            .handles
            .iter()
            .filter_map(|(handle, world)| {
                let q = frame.frame.projection.project(*world);
                let distance = (q[0] - p.x as f32).hypot(q[1] - p.y as f32);
                (distance < 12.).then_some((handle.clone(), *world, distance))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(handle, world, _)| (handle, world))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderkit::{GpuContext, GpuImage, ImageDescriptor};
    use theframework::render_wgpu::{PaintRenderer, UiImages};
    #[test]
    #[ignore = "requires native GPU"]
    fn native_geometry_host_keeps_picks_on_presented_frames_and_rebuilds_only_gpu_ownership() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let (mut ui, mut ctx) = super::super::viewport_tests::fixture(1100, 800);
        ctx.ui.command_paint = true;
        let mut project = Project::default();
        project.config = "[ruleset]\nsource='project'\n".into();
        let mut doc = scene::new_document();
        let joint = scene::add_joint(&mut doc, "root", [0.6, 1.1, 0.2]).unwrap();
        let id = Uuid::new_v4();
        project
            .procedural_appearances
            .insert(id, doc.save().unwrap());
        let original = serde_json::to_value(&project).unwrap();
        let mut editor = GeometryEditor::default();
        editor.enable_gpu_preview();
        editor.handle_event(
            &TheEvent::Custom(TheId::named("Open Procedural Geometry"), TheValue::Id(id)),
            &mut project,
            &mut ui,
            &mut ctx,
        );
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &ServerContext::default());
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &ServerContext::default());
        assert!(
            editor.worker.is_none(),
            "the converted path must never obtain a device via RUSTERIX or start a color-readback worker"
        );
        assert!(editor.native.latest.is_some());
        let mut images = UiImages::new(gpu.clone());
        let token = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images.registry)
            .unwrap()
            .unwrap();
        assert!(!editor.native_pick_ready(&project));
        assert!(editor.presented_gpu_preview(token));
        assert!(editor.native_pick_ready(&project));
        let (handle, world) = editor
            .native
            .displayed
            .as_ref()
            .unwrap()
            .handles
            .iter()
            .find(|(handle, _)| matches!(handle,scene::Handle::Joint(id) if id==&joint))
            .unwrap()
            .clone();
        let point = editor.scene.projection.as_ref().unwrap().project(world);
        let hit = editor
            .preview_handle_hit(
                editor.document.as_ref().unwrap(),
                editor.evaluation.as_ref().unwrap(),
                Vec2::new(point[0].round() as i32, point[1].round() as i32),
            )
            .unwrap();
        assert!(
            matches!((&handle,&hit.0),(scene::Handle::Joint(a),scene::Handle::Joint(b)) if a==b)
        );
        let old_projection = editor.scene.projection.as_ref().unwrap().project(world);
        editor.yaw += 40.;
        editor.preview_key.clear();
        editor.update(&project, &mut ui, &mut ctx, &ServerContext::default());
        let pending = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images.registry)
            .unwrap()
            .unwrap();
        assert_ne!(pending, token);
        assert_eq!(
            editor.scene.projection.as_ref().unwrap().project(world),
            old_projection
        );
        assert!(editor.native_pick_ready(&project));
        // A still newer unacknowledged frame cannot revive a superseded token.
        editor.yaw += 15.;
        editor.preview_key.clear();
        editor.update(&project, &mut ui, &mut ctx, &ServerContext::default());
        let newest = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images.registry)
            .unwrap()
            .unwrap();
        assert!(!editor.presented_gpu_preview(pending));
        assert!(editor.presented_gpu_preview(newest));
        assert_ne!(
            editor.scene.projection.as_ref().unwrap().project(world),
            old_projection
        );
        let list = ui.paint_commands(&mut ctx, &mut images).unwrap();
        images.finish_frame(&list).unwrap();
        assert!(ui.canvas.buffer.pixels().is_empty());
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1100, 800)).unwrap();
        let mut painter = PaintRenderer::new(gpu.clone()).unwrap();
        painter
            .render(
                &list,
                &images.registry,
                &output.view,
                output.descriptor.format,
                (1100, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(gpu.transfers().readback_bytes, 0);
        // A failed newer request keeps the actual staged paint and its receipt.
        editor.yaw += 9.;
        editor.preview_key.clear();
        editor.update(&project, &mut ui, &mut ctx, &ServerContext::default());
        let retained = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images.registry)
            .unwrap()
            .unwrap();
        let valid = {
            let job = &editor.native.latest.as_ref().unwrap().job;
            (job.revision, job.yaw)
        };
        let bad = editor.native.latest.as_mut().unwrap();
        bad.job.revision += 1;
        bad.job.yaw = f32::NAN;
        let error = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images.registry)
            .unwrap_err();
        editor.report_gpu_preview_error(error.clone(), &mut ctx);
        assert_eq!(editor.native.error.as_ref(), Some(&error));
        assert!(!editor.gpu_preview_waiting());
        assert_eq!(editor.native_preview_fallback_token(), Some(retained));
        assert!(editor.presented_gpu_preview(retained));
        assert!(editor.native_pick_ready(&project));
        let restored = &mut editor.native.latest.as_mut().unwrap().job;
        restored.revision = valid.0;
        restored.yaw = valid.1;
        let old_image = editor.native.displayed.as_ref().unwrap().image;
        let replacement_gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let mut replacement = ImageRegistry::new(replacement_gpu.clone());
        let rebuilt = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
            .unwrap()
            .unwrap();
        assert!(!editor.native_pick_ready(&project));
        assert!(!editor.presented_gpu_preview(newest));
        assert!(editor.presented_gpu_preview(rebuilt));
        let current = editor.native.displayed.as_ref().unwrap();
        let target = GpuImage::new(&replacement_gpu, current.frame.image.descriptor).unwrap();
        let mut compositor = PaintRenderer::new(replacement_gpu.clone()).unwrap();
        compositor
            .render(
                &current.frame.paint(current.image),
                &replacement,
                &target.view,
                target.descriptor.format,
                (target.descriptor.width, target.descriptor.height),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(replacement_gpu.transfers().readback_bytes, 0);

        assert!(
            images.registry.get(old_image).is_some(),
            "never remove foreign registry handles during reconstruction"
        );
        assert!(replacement.get(old_image).is_none());
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
        let mut changed = Document::load(&project.procedural_appearances[&id]).unwrap();
        changed.label = "External edit".into();
        project
            .procedural_appearances
            .insert(id, changed.save().unwrap());
        assert!(
            !editor.native_pick_ready(&project),
            "external source changes reject old handle input before the next CPU update"
        );
        let last = editor.native.displayed.as_ref().unwrap().image;
        editor.reset(&mut ui, &mut ctx);
        assert!(
            editor
                .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
                .unwrap()
                .is_none()
        );
        assert!(replacement.get(last).is_none());
        assert!(!editor.presented_gpu_preview(rebuilt));
    }
}
