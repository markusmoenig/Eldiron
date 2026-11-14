//! Creator-owned geometry publication and completed-frame picking receipts.
use super::*;
use crate::geometry_editor::native_preview::{Frame as MeshFrame, NativePreview};
use renderkit::{ImageHandle, ImageRegistry};
use std::sync::Arc;
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

#[derive(Clone)]
pub(super) struct ContourRequest {
    pub epoch: u64,
    pub revision: u64,
    pub paint: Arc<theframework::paint::PaintList>,
    pub layout: Option<presentation::ContourFrame>,
}
enum Frame {
    Mesh(MeshFrame),
    Contour(ContourRequest),
    Baked(baked_preview::Frame),
}
impl Frame {
    fn epoch(&self) -> u64 {
        match self {
            Self::Mesh(f) => f.epoch,
            Self::Contour(f) => f.epoch,
            Self::Baked(f) => f.epoch,
        }
    }
    fn revision(&self) -> u64 {
        match self {
            Self::Mesh(f) => f.revision,
            Self::Contour(f) => f.revision,
            Self::Baked(f) => f.revision,
        }
    }
    fn mesh(&self) -> Option<&MeshFrame> {
        match self {
            Self::Mesh(f) => Some(f),
            _ => None,
        }
    }
}
struct Published {
    token: PreviewToken,
    frame: Frame,
    image: Option<ImageHandle>,
    camera: usize,
    operation: Option<String>,
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
    pub contour: Option<ContourRequest>,
    pub baked: Option<baked_preview::Request>,
    pub waiting: bool,
    pub error: Option<String>,
    pub thumbnails: super::native_thumbnails::Cache,
    pub thumbnail_palette: ThePalette,
    owner: u64,
    serial: u64,
    registry: Option<u64>,
    producer: Option<NativePreview>,
    baked_producer: Option<baked_preview::Preview>,
    staged: Option<Published>,
    displayed: Option<Published>,
    retired: Vec<ImageHandle>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            enabled: false,
            latest: None,
            contour: None,
            baked: None,
            waiting: false,
            error: None,
            thumbnails: Default::default(),
            thumbnail_palette: Default::default(),
            owner: theframework::paint::new_receipt_owner(),
            serial: 0,
            registry: None,
            producer: None,
            baked_producer: None,
            staged: None,
            displayed: None,
            retired: Vec::new(),
        }
    }
}
impl Host {
    pub fn invalidate(&mut self) {
        self.latest = None;
        self.contour = None;
        self.baked = None;
        self.waiting = false;
        self.error = None;
        if let Some(frame) = self.staged.take() {
            self.retired.extend(frame.image);
        }
        if let Some(frame) = self.displayed.take() {
            self.retired.extend(frame.image);
        }
        self.producer = None;
        self.baked_producer = None;
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
        let contour = self.native.contour.take();
        let baked = self.native.baked.take();
        self.native.invalidate();
        self.native.latest = latest;
        self.native.contour = contour;
        self.native.baked = baked;
        self.contour_frame = None;
        self.scene.projection = None;
        self.preview_key.clear();
        if let Some(view) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
            let _ = view.set_paint_list(None);
        }
    }
    /// Select before the first CPU update. Converted preview requests must not
    /// discover a GPU through the reference facade or start the reference framebuffer worker.
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
        let visible = self.native.enabled
            && self.active
            && ui
                .get_stack_layout("Sidebar Page Stack")
                .is_none_or(|s| s.index() == crate::procedural_material::SIDEBAR_PAGE)
            && ui
                .get_stack_layout(super::STACK)
                .is_none_or(|stack| stack.index() == 1);
        let keys = ui
            .get_widget(EDITOR)
            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
            .filter(|_| visible)
            .map(|w| w.visible_thumbnail_keys())
            .unwrap_or_default();
        let output = match self.native.thumbnails.prepare(
            images,
            self.key.as_deref(),
            self.evaluation.as_ref(),
            &self.native.thumbnail_palette,
            ui.style.theme().as_ref(),
            &mut ctx.text,
            &keys,
        ) {
            Ok(output) => output,
            Err(error) => {
                if let Some(widget) = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                {
                    widget.set_native_thumbnails(Default::default());
                }
                return Err(error);
            }
        };
        if let Some(widget) = ui
            .get_widget(EDITOR)
            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
        {
            widget.set_native_thumbnails(output.handles);
        }
        for error in output.errors {
            Self::status(ctx, error);
        }
        let result = self.prepare_gpu_view(ui, ctx, images);
        self.native.waiting |= output.waiting;
        result
    }
    fn prepare_gpu_view(
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
        let contour = if registry_changed {
            self.native.contour.take()
        } else {
            None
        };
        let baked = if registry_changed {
            self.native.baked.take()
        } else {
            None
        };
        self.native.drain(images);
        if registry_changed {
            self.native.latest = latest;
            self.native.contour = contour;
            self.native.baked = baked;
            self.contour_frame = None;
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
            self.contour_frame = None;
            self.scene.projection = None;
            if let Some(view) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) {
                view.set_paint_list(None).map_err(|e| e.to_string())?;
            }
            return Ok(None);
        }
        if ui
            .get_widget(PREVIEW)
            .is_none_or(|view| view.dim().width < 16 || view.dim().height < 16)
        {
            self.invalidate_gpu_preview(ui);
            self.native.drain(images);
            return Ok(None);
        }
        let use_baked = self.camera == 4 && self.native.baked.is_some();
        let (epoch, revision) = if use_baked {
            let request = self.native.baked.as_ref().unwrap();
            (request.epoch, request.revision)
        } else if self.camera == 3 {
            let Some(request) = self.native.contour.as_ref() else {
                return Ok(None);
            };
            (request.epoch, request.revision)
        } else {
            let Some(request) = self.native.latest.as_ref() else {
                return Ok(None);
            };
            (request.epoch, request.job.revision)
        };
        if epoch != self.preview_epoch {
            return Ok(None);
        }
        let same = |p: &Published| {
            p.frame.epoch() == epoch
                && p.frame.revision() == revision
                && p.camera == self.camera
                && p.operation == self.selected_operation
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
        let (frame, image, paint, handles) = if use_baked {
            self.native.producer = None;
            if self
                .native
                .baked_producer
                .as_ref()
                .is_none_or(|p| p.generation() != images.context().generation())
            {
                self.native.baked_producer =
                    Some(baked_preview::Preview::new(images.context().clone()));
            }
            let request = self.native.baked.as_ref().unwrap();
            let Some(frame) = self
                .native
                .baked_producer
                .as_mut()
                .unwrap()
                .render(request, ctx.scale_factor)?
            else {
                self.native.waiting = true;
                return Ok(self.native_preview_fallback_token());
            };
            let image = images
                .insert(frame.image.clone())
                .map_err(|e| e.to_string())?;
            let paint = Arc::new(frame.paint(image));
            (Frame::Baked(frame), Some(image), paint, Vec::new())
        } else if self.camera == 3 {
            self.native.baked_producer = None;
            self.native.producer = None;
            let request = self.native.contour.as_ref().unwrap().clone();
            let paint = request.paint.clone();
            (Frame::Contour(request), None, paint, Vec::new())
        } else {
            self.native.baked_producer = None;
            if self
                .native
                .producer
                .as_ref()
                .is_none_or(|p| p.generation() != images.context().generation())
            {
                self.native.producer = Some(NativePreview::new(images.context().clone()));
            }
            let request = self.native.latest.as_ref().unwrap();
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
            // A completed compatible animation frame owns its own pose and
            // revision. Schedule catch-up if a newer request is still waiting.
            self.native.waiting = frame.revision != request.job.revision;
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
            (Frame::Mesh(frame), Some(image), Arc::new(paint), handles)
        };
        self.native.error = None;
        self.native.serial = self
            .native
            .serial
            .checked_add(1)
            .expect("preview receipt space exhausted");
        let token = PreviewToken {
            owner: self.native.owner,
            serial: self.native.serial,
        };
        let mut paint = (*paint).clone();
        paint
            .ops
            .push(theframework::paint::PaintOp::Receipt(token.receipt()));
        let paint = Arc::new(paint);
        let Some(view) = ui.get_widget(PREVIEW).and_then(|w| w.as_render_view()) else {
            if let Some(image) = image {
                images.remove(image);
            }
            return Ok(None);
        };
        if let Err(error) = view.set_paint_list(Some(paint)) {
            if let Some(image) = image {
                images.remove(image);
            }
            return Err(error.to_string());
        }
        if let Some(old) = self.native.staged.replace(Published {
            token,
            frame,
            image,
            camera: self.camera,
            operation: self.selected_operation.clone(),
            density: ctx.scale_factor.to_bits(),
            source_revision: self.source_revision,
            source_key: self.key.clone().unwrap_or_default(),
            selection: self.inspector.clone(),
            tool: self.scene.tool,
            handles,
        }) {
            self.native.retired.extend(old.image);
        }
        Ok(Some(token))
    }
    /// Snapshot only immutable authored source pixels. Viewport scaling and
    /// alpha/checker composition are performed by the injected GPU producer.
    pub(super) fn queue_baked_preview(
        &mut self,
        source: &rusterix::Texture,
        dim: TheDim,
        checker: [[u8; 4]; 2],
    ) -> Result<bool, String> {
        if !baked_preview::Preview::source_valid(source) {
            return Err("Invalid baked preview source or source exceeds 16 MiB".into());
        }
        let size = [dim.width as u32, dim.height as u32];
        let same_source = |request: &baked_preview::Request| {
            request.source.width() as usize == source.width
                && request.source.height() as usize == source.height
                && request.source.pixels() == source.data
        };
        if self.native.baked.as_ref().is_some_and(|r| {
            r.epoch == self.preview_epoch
                && r.size == size
                && r.checker == checker
                && same_source(r)
        }) {
            return Ok(false);
        }
        let source = if let Some(old) = self.native.baked.as_ref().filter(|r| same_source(r)) {
            old.source.clone()
        } else {
            Arc::new(
                renderkit::ImageAsset::rgba(
                    source.width as u32,
                    source.height as u32,
                    source.data.clone(),
                )
                .map_err(|e| e.to_string())?,
            )
        };
        self.preview_epoch += 1;
        self.revision += 1;
        self.native.baked = Some(baked_preview::Request {
            epoch: self.preview_epoch,
            revision: self.revision,
            source,
            size,
            checker,
        });
        self.native.latest = None;
        self.native.contour = None;
        self.preview_key.clear();
        Ok(true)
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
        if frame.frame.epoch() != self.preview_epoch
            || frame.source_revision != self.source_revision
            || frame.selection != self.inspector
            || frame.tool != self.scene.tool
            || frame.camera != self.camera
            || frame.operation != self.selected_operation
        {
            return false;
        }
        let frame = self.native.staged.take().unwrap();
        self.displayed_revision = frame.frame.revision();
        match &frame.frame {
            Frame::Mesh(mesh) => {
                self.scene.projection = Some(mesh.projection.clone());
                self.scene.posed = mesh.posed.clone();
                self.contour_frame = None;
            }
            Frame::Contour(contour) => {
                self.contour_frame = contour.layout.clone();
                self.scene.projection = None;
                self.scene.posed.clear();
            }
            Frame::Baked(_) => {
                self.contour_frame = None;
                self.scene.projection = None;
                self.scene.posed.clear();
            }
        }
        self.scene.previewing_animation = self.playback.selected > 0;
        if let Some(old) = self.native.displayed.replace(frame) {
            self.native.retired.extend(old.image);
        }
        true
    }
    pub(super) fn native_pick_ready(&self, project: &Project) -> bool {
        if !self.native.enabled {
            return true;
        }
        let Some(p) = self.native.displayed.as_ref() else {
            return false;
        };
        if p.frame.epoch() != self.preview_epoch
            || p.source_revision != self.source_revision
            || p.selection != self.inspector
            || p.tool != self.scene.tool
            || p.camera != self.camera
            || p.operation != self.selected_operation
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
        if !self.native.enabled || self.camera == 3 {
            return self.scene.hit(doc, e, &self.inspector, p);
        }
        let frame = self.native.displayed.as_ref()?;
        frame
            .handles
            .iter()
            .filter_map(|(handle, world)| {
                let q = frame.frame.mesh()?.projection.project(*world);
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
    fn native_geometry_baked_host_selects_authored_idle_frames_and_rebuilds_sources() {
        let creator_history = CreatorHistory::default();
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let (mut ui, mut ctx) = super::super::viewport_tests::fixture(1100, 800);
        ctx.ui.command_paint = true;
        let mut project = Project::default();
        project.config = "[ruleset]\nsource='project'\n".into();
        let recipe = Uuid::new_v4();
        project
            .procedural_appearances
            .insert(recipe, scene::new_document().save().unwrap());
        let output = Uuid::new_v4();
        let mut tile =
            rusterix::Tile::from_texture(rusterix::Texture::new(vec![0, 0, 0, 255], 1, 1));
        tile.id = output;
        tile.isometric = Some(rusterix::isometric::SpriteLayout {
            directions: 1,
            frames_per_direction: 3,
            walk_frames: 2,
            frames: vec![
                rusterix::Texture::new(vec![255, 0, 0, 255], 1, 1),
                rusterix::Texture::new(vec![0, 255, 0, 255], 1, 1),
                rusterix::Texture::new(vec![0, 0, 255, 128], 1, 1),
            ],
            ..Default::default()
        });
        project.tiles.insert(output, tile);
        project.isometric_bakes.insert(
            output,
            shared::isometric::Bake {
                recipe,
                ground: false,
                yaw: 0.,
                directions: 1,
                motion: None,
                frames: 3,
                signature: 0,
            },
        );
        let original = serde_json::to_value(&project).unwrap();
        let mut editor = GeometryEditor::default();
        editor.enable_gpu_preview();
        editor.handle_event(
            &TheEvent::Custom(
                TheId::named("Open Procedural Geometry"),
                TheValue::Id(recipe),
            ),
            &mut project,
            &mut ui,
            &mut ctx,
         &creator_history);
        editor.camera = 4;
        editor.iso_output = Some(output);
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        assert!(editor.worker.is_none());
        assert!(editor.native.latest.is_none());
        assert_eq!(
            editor.native.baked.as_ref().unwrap().source.pixels(),
            [0, 0, 255, 128]
        );
        let mut images = ImageRegistry::new(gpu.clone());
        let token = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap()
            .unwrap();
        assert!(editor.native.producer.is_none());
        assert!(!editor.native_pick_ready(&project));
        assert!(editor.presented_gpu_preview(token));
        assert!(editor.scene.projection.is_none());
        assert!(editor.native_pick_ready(&project));
        assert_eq!(gpu.transfers().uploaded_bytes, 4);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let old_image = editor.native.displayed.as_ref().unwrap().image.unwrap();
        let transfers = gpu.transfers();
        assert!(!editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default()));
        assert_eq!(
            editor
                .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
                .unwrap(),
            Some(token)
        );
        assert_eq!(gpu.transfers(), transfers);
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
        project
            .tiles
            .get_mut(&output)
            .unwrap()
            .isometric
            .as_mut()
            .unwrap()
            .frames[2]
            .data = vec![241, 19, 33, 255];
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        let changed = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap()
            .unwrap();
        assert_ne!(changed, token);
        assert!(images.get(old_image).is_some());
        assert!(!editor.native_pick_ready(&project));
        assert!(editor.presented_gpu_preview(changed));
        assert_eq!(gpu.transfers().uploaded_bytes, 8);
        let mut replacement =
            ImageRegistry::new(pollster::block_on(GpuContext::headless()).unwrap());
        let rebuilt = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
            .unwrap()
            .unwrap();
        assert!(!editor.native_pick_ready(&project));
        assert!(!editor.presented_gpu_preview(changed));
        assert!(editor.presented_gpu_preview(rebuilt));
        assert_eq!(replacement.context().transfers().uploaded_bytes, 4);
        assert_eq!(replacement.context().transfers().readback_bytes, 0);
        editor.reset(&mut ui, &mut ctx);
        assert!(
            editor
                .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
                .unwrap()
                .is_none()
        );
        assert!(!editor.presented_gpu_preview(rebuilt));
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_geometry_contour_paint_shares_reference_layout_and_acknowledged_input() {
        let creator_history = CreatorHistory::default();
        use theframework::paint::PaintOp;
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let (mut ui, mut ctx) = super::super::viewport_tests::fixture(1100, 800);
        ctx.ui.command_paint = true;
        let mut project = Project::default();
        project.config = "[ruleset]\nsource='project'\n".into();
        let doc=Document::load(r#"{"version":1,"id":"contour","label":"Profile","rows":[{"id":"profile","label":"Profile","tiles":[{"id":"outline","op":{"type":"contour","points":[[-0.1,0],[0.1,0],[0,0.8]]}}]},{"id":"model","label":"Model","tiles":[{"id":"box","op":{"type":"box","size":[1,1,1]}}]}],"output":"model"}"#).unwrap();
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
         &creator_history);
        editor.camera = 3;
        editor.selected_operation = Some("outline".into());
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        assert!(editor.worker.is_none());
        assert!(editor.native.latest.is_none());
        assert!(
            editor.contour_frame.is_none(),
            "CPU update cannot promote unpresented picking geometry"
        );
        let dim = *ui.get_widget(PREVIEW).unwrap().dim();
        let request = editor.native.contour.as_ref().unwrap();
        let reference = presentation::contour_image(
            &[[-0.1, 0.], [0.1, 0.], [0., 0.8]],
            dim.width,
            dim.height,
            ui.style.theme().as_ref(),
            None,
        )
        .1;
        assert_eq!(request.layout, reference);
        let paint = request.paint.clone();
        assert!(
            paint
                .ops
                .iter()
                .any(|op| matches!(op, PaintOp::GlyphRun { .. }))
        );
        assert!(
            !paint
                .ops
                .iter()
                .any(|op| matches!(op, PaintOp::Image { .. }))
        );
        let mut images = ImageRegistry::new(gpu.clone());
        let token = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap()
            .unwrap();
        assert_eq!(
            gpu.transfers().uploaded_bytes,
            0,
            "profiles upload no source/viewport bitmaps"
        );
        // The real canvas traversal, not the controller's active flag, decides
        // which receipts can be acknowledged after presenting the root list.
        let mut cache = theframework::render_wgpu::UiImageCache::default();
        let list = ui
            .paint_commands(&mut ctx, &mut cache.frame(&mut images))
            .unwrap();
        assert!(list.receipts().any(|receipt| receipt == token.receipt()));
        let cards = editor.native.thumbnails.handles();
        assert!(
            !cards.is_empty(),
            "the real geometry graph publishes GPU inspection cards"
        );
        let painted = list
            .image_handles()
            .collect::<std::collections::HashSet<_>>();
        assert!(
            cards.values().all(|handle| painted.contains(handle)),
            "the root must consume the GPU card handles"
        );
        let mut tokens = crate::editor::native_views::NativeViewTokens::previews(
            Some(token), None, None,
        );
        tokens.retain_painted(&list);
        assert_eq!(tokens.geometry, Some(token));
        ui.get_stack_layout(super::super::layout::MAIN_STACK)
            .unwrap()
            .set_index(0);
        let hidden = ui
            .paint_commands(&mut ctx, &mut cache.frame(&mut images))
            .unwrap();
        tokens.retain_painted(&hidden);
        assert!(
            tokens.geometry.is_none(),
            "a hidden ancestor must discard the unpainted receipt"
        );
        ui.get_stack_layout(super::super::layout::MAIN_STACK)
            .unwrap()
            .set_index(1);
        ui.prepare_paint_layout(&mut ctx);
        assert!(
            editor.native.producer.is_none(),
            "contours allocate neither mesh targets nor material runtimes"
        );
        assert!(editor.native.staged.as_ref().unwrap().image.is_none());
        editor.handle_event(
            &TheEvent::RenderViewClicked(TheId::named(PREVIEW), Vec2::new(100, 100)),
            &mut project,
            &mut ui,
            &mut ctx,
         &creator_history);
        assert!(editor.drag.is_none());
        assert!(editor.presented_gpu_preview(token));
        assert!(editor.native_pick_ready(&project));
        let frame = editor.contour_frame.clone().unwrap();
        let p = frame.project([-0.1, 0.]);
        let p = Vec2::new(p[0].round() as i32, p[1].round() as i32);
        assert_eq!(frame.hit(p), Some(0));
        // Source-independent resize must retain the old hit transform until ack.
        ui.canvas.set_dim(TheDim::new(0, 0, 1400, 900), &mut ctx);
        ctx.width = 1400;
        ctx.height = 900;
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        let resized = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap()
            .unwrap();
        assert_eq!(editor.contour_frame.as_ref(), Some(&frame));
        assert!(!editor.native_pick_ready(&project));
        assert!(!editor.presented_gpu_preview(token));
        assert!(editor.presented_gpu_preview(resized));
        assert_ne!(editor.contour_frame.as_ref(), Some(&frame));
        assert!(editor.native_pick_ready(&project));
        let new_frame = editor.contour_frame.clone().unwrap();
        let p = new_frame.project([-0.1, 0.]);
        let p = Vec2::new(p[0].round() as i32, p[1].round() as i32);
        editor.handle_event(
            &TheEvent::RenderViewClicked(TheId::named(PREVIEW), p),
            &mut project,
            &mut ui,
            &mut ctx,
         &creator_history);
        assert_eq!(editor.drag.as_ref().and_then(|d| d.1), Some(0));
        // GPU receipt invalidation must not refit the CPU drag's frozen plane.
        editor.invalidate_gpu_preview(&mut ui);
        assert!(editor.contour_frame.is_none());
        assert_eq!(editor.contour_drag_frame.as_ref(), Some(&new_frame));
        editor.handle_event(
            &TheEvent::RenderViewDragged(TheId::named(PREVIEW), p + Vec2::new(8, -6)),
            &mut project,
            &mut ui,
            &mut ctx,
         &creator_history);
        assert_ne!(serde_json::to_value(&project).unwrap(), original);
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        assert_eq!(
            editor
                .native
                .contour
                .as_ref()
                .unwrap()
                .layout
                .as_ref()
                .unwrap()
                .unproject(p),
            new_frame.unproject(p)
        );
        let dragged = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap()
            .unwrap();
        assert!(editor.presented_gpu_preview(dragged));
        editor.finish_drag(&mut project, &mut ctx, true, &creator_history);
        assert!(editor.contour_drag_frame.is_none());
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        let restored = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut images)
            .unwrap()
            .unwrap();
        assert!(editor.presented_gpu_preview(restored));

        let mut painter = PaintRenderer::new(gpu.clone()).unwrap();
        let output = GpuImage::new(
            &gpu,
            ImageDescriptor::rgba(dim.width as u32, dim.height as u32),
        )
        .unwrap();
        painter
            .render(
                &paint,
                &images,
                &output.view,
                output.descriptor.format,
                (dim.width as u32, dim.height as u32),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert!(painter.glyphs.rasterizations > 0);
        let rasterizations = painter.glyphs.rasterizations;
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let transfers = gpu.transfers();
        painter
            .render(
                &paint,
                &images,
                &output.view,
                output.descriptor.format,
                (dim.width as u32, dim.height as u32),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(painter.glyphs.rasterizations, rasterizations);
        assert_eq!(gpu.transfers(), transfers);
        let mut capture = renderkit::ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let captured = capture.try_finish(&gpu).unwrap().unwrap();
        let mut encoder = png::Encoder::new(
            std::fs::File::create("/private/tmp/eldiron-native-contour-preview.png").unwrap(),
            dim.width as u32,
            dim.height as u32,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(captured.pixels())
            .unwrap();
        let mut replacement =
            ImageRegistry::new(pollster::block_on(GpuContext::headless()).unwrap());
        let rebuilt = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
            .unwrap()
            .unwrap();
        assert!(editor.contour_frame.is_none());
        assert!(!editor.native_pick_ready(&project));
        assert!(!editor.presented_gpu_preview(resized));
        assert!(editor.presented_gpu_preview(rebuilt));
        assert_eq!(editor.contour_frame.as_ref(), Some(&new_frame));
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
        editor.reset(&mut ui, &mut ctx);
        assert!(
            editor
                .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
                .unwrap()
                .is_none()
        );
        assert!(!editor.presented_gpu_preview(rebuilt));
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_geometry_host_keeps_picks_on_presented_frames_and_rebuilds_only_gpu_ownership() {
        let creator_history = CreatorHistory::default();
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
         &creator_history);
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        ui.prepare_paint_layout(&mut ctx);
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
        assert!(
            editor.worker.is_none(),
            "the converted path must never obtain a device via the reference facade or start a color-readback worker"
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
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
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
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
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
        editor.update(&project, &mut ui, &mut ctx, &CreatorContext::default());
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
        let old_image = editor.native.displayed.as_ref().unwrap().image.unwrap();
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
        let target = GpuImage::new(
            &replacement_gpu,
            current.frame.mesh().unwrap().image.descriptor,
        )
        .unwrap();
        let mut compositor = PaintRenderer::new(replacement_gpu.clone()).unwrap();
        compositor
            .render(
                &current.frame.mesh().unwrap().paint(current.image.unwrap()),
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
        let dim = *ui.get_widget(PREVIEW).unwrap().dim();
        let visible_image = editor.native.displayed.as_ref().unwrap().image.unwrap();
        ui.get_widget(PREVIEW)
            .unwrap()
            .set_dim(TheDim::zero(), &mut ctx);
        assert!(
            editor
                .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
                .unwrap()
                .is_none()
        );
        assert!(!editor.presented_gpu_preview(rebuilt));
        assert!(!editor.native_pick_ready(&project));
        assert!(replacement.get(visible_image).is_none());
        ui.get_widget(PREVIEW).unwrap().set_dim(dim, &mut ctx);
        let visible = editor
            .prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement)
            .unwrap()
            .unwrap();
        assert!(editor.presented_gpu_preview(visible));
        assert!(editor.native_pick_ready(&project));
        let mut changed = Document::load(&project.procedural_appearances[&id]).unwrap();
        changed.label = "External edit".into();
        project
            .procedural_appearances
            .insert(id, changed.save().unwrap());
        assert!(
            !editor.native_pick_ready(&project),
            "external source changes reject old handle input before the next CPU update"
        );
        let last = editor.native.displayed.as_ref().unwrap().image.unwrap();
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
