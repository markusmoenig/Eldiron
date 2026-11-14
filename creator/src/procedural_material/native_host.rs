//! Revision-scoped material display on the application-owned GPU. Authored bake
//! jobs and immutable GPU graph thumbnails are independent from view output.
use super::native_preview::{Preview, Request};
use super::*;
use renderkit::{GpuImage, ImageHandle, ImageRegistry};
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

#[derive(Clone, Debug, PartialEq)]
enum Target {
    Root,
    Selection(Option<PreviewSelection>),
}
struct Input<'a> {
    tile: Option<Uuid>,
    bundle: Option<&'a str>,
    target: Target,
    tick: usize,
    channel: usize,
    size: [u32; 2],
    checker: [[u8; 4]; 2],
}
struct Source {
    tile: Option<Uuid>,
    bundle: Option<String>,
    target: Target,
    frames: u32,
    origin: [f32; 2],
    asset: Result<Option<Arc<EditableMaterialAsset>>, String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Key {
    source: u64,
    time: u32,
    channel: u32,
    pixels: u32,
    continuous: bool,
    size: [u32; 2],
    checker: [[u8; 4]; 2],
}
struct Latest {
    key: Key,
    request: Result<Option<Request>, String>,
}
struct Published {
    token: PreviewToken,
    key: Key,
    image: ImageHandle,
    _lease: Arc<GpuImage>,
    density: u32,
}
pub(super) struct Host {
    pub enabled: bool,
    pub pixels: u32,
    pub continuous: bool,
    pub asset_job_pending: bool,
    pub needs_asset_worker: bool,
    worker_generation: Option<u64>,
    thumbnails: super::native_thumbnails::Cache,
    thumbnail_error: Option<String>,
    owner: u64,
    serial: u64,
    source_epoch: u64,
    source: Option<Source>,
    latest: Option<Latest>,
    registry: Option<u64>,
    producer: Option<Preview>,
    checker: Option<([[u8; 4]; 2], Arc<GpuImage>)>,
    staged: Option<Published>,
    displayed: Option<Published>,
    retired: Vec<ImageHandle>,
    waiting: bool,
    error: Option<String>,
    failed: Option<(Key, u32)>,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            enabled: false,
            pixels: pixel_style::DEFAULT_PIXELS_PER_TILE,
            continuous: true,
            asset_job_pending: false,
            needs_asset_worker: false,
            worker_generation: None,
            thumbnails: Default::default(),
            thumbnail_error: None,
            owner: theframework::paint::new_receipt_owner(),
            serial: 0,
            source_epoch: 0,
            source: None,
            latest: None,
            registry: None,
            producer: None,
            checker: None,
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
    fn retire_gpu(&mut self) {
        self.producer = None;
        self.checker = None;
        self.waiting = false;
        self.error = None;
        self.failed = None;
        if let Some(frame) = self.staged.take() {
            self.retired.push(frame.image);
        }
        if let Some(frame) = self.displayed.take() {
            self.retired.push(frame.image);
        }
    }
    pub fn reset_bake_domain(&mut self) {
        self.worker_generation = None;
        self.needs_asset_worker = false;
        self.asset_job_pending = false;
    }
    pub fn clear(&mut self) {
        self.retire_gpu();
        self.thumbnails.clear();
        self.latest = None;
        self.source = None;
        self.source_epoch += 1;
        self.asset_job_pending = false;
    }
    fn drain(&mut self, images: &mut ImageRegistry) {
        if self.registry != Some(images.owner_id()) {
            self.retire_gpu();
            self.retired.clear();
            self.registry = Some(images.owner_id());
        } else {
            for image in self.retired.drain(..) {
                images.remove(image);
            }
        }
    }
    fn fallback(&self) -> Option<PreviewToken> {
        self.staged
            .as_ref()
            .or(self.displayed.as_ref())
            .map(|p| p.token)
    }
    fn queue(&mut self, input: Input<'_>) -> bool {
        let same = self.source.as_ref().is_some_and(|s| {
            s.tile == input.tile && s.bundle.as_deref() == input.bundle && s.target == input.target
        });
        if !same {
            let mut frames = 1;
            let mut origin = [-0.5; 2];
            let asset = (|| {
                let Some(bundle) = input.bundle else {
                    return Ok(None);
                };
                let asset = EditableMaterialAsset::reopen(bundle).map_err(|e| e.to_string())?;
                asset.validate().map_err(|e| e.to_string())?;
                frames = super::preview_metadata::animation_settings(&asset)?.frames;
                origin = asset
                    .instances
                    .iter()
                    .find(|r| r.id == super::composition::SURFACE_DOMAIN)
                    .map_or([-0.5; 2], |r| [r.uv_transform[4], r.uv_transform[5]]);
                let selected = match &input.target {
                    Target::Root => asset,
                    Target::Selection(None) => return Ok(None),
                    Target::Selection(Some(PreviewSelection::Operation(id))) => {
                        operation_preview_asset(&asset, id)?
                    }
                    Target::Selection(Some(PreviewSelection::Source(id))) => {
                        source_preview_asset(id)?
                    }
                    Target::Selection(Some(PreviewSelection::Input(target, reference))) => {
                        input_preview_asset(&asset, target, reference)?
                    }
                    Target::Selection(Some(PreviewSelection::Material(id))) => {
                        let mut selected = asset;
                        selected.selected_material = id.clone();
                        selected.validate().map_err(|e| e.to_string())?;
                        selected
                    }
                };
                Ok(Some(Arc::new(selected)))
            })();
            self.source_epoch += 1;
            self.source = Some(Source {
                tile: input.tile,
                bundle: input.bundle.map(str::to_owned),
                target: input.target.clone(),
                frames,
                origin,
                asset,
            });
        }
        let source = self.source.as_ref().unwrap();
        let time = if matches!(&source.asset, Ok(Some(_))) {
            (input.tick % source.frames.max(1) as usize) as f32 / source.frames.max(1) as f32
        } else {
            0.
        };
        let channel = if matches!(input.target, Target::Root) {
            PREVIEW_MAPS[input.channel.min(4)] as u32
        } else {
            0
        };
        let key = Key {
            source: self.source_epoch,
            time: time.to_bits(),
            channel,
            pixels: self.pixels,
            continuous: self.continuous,
            size: input.size,
            checker: input.checker,
        };
        if self.latest.as_ref().is_some_and(|latest| latest.key == key) {
            return false;
        }
        let request = source.asset.as_ref().map_err(Clone::clone).map(|asset| {
            asset.as_ref().map(|asset| Request {
                asset: asset.clone(),
                time,
                origin: Some(source.origin),
                pixels: self.pixels,
                continuous: self.continuous,
                channel,
                size: input.size,
                checker: input.checker,
            })
        });
        self.latest = Some(Latest { key, request });
        self.failed = None;
        true
    }
    fn output(
        &mut self,
        ui: &mut TheUI,
        ctx: &TheContext,
        images: &mut ImageRegistry,
    ) -> Result<Option<PreviewToken>, String> {
        let Some(latest) = &self.latest else {
            return Ok(self.fallback());
        };
        let same = |p: &Published| p.key == latest.key && p.density == ctx.scale_factor.to_bits();
        if let Some(p) = self.staged.as_ref().filter(|p| same(p)) {
            return Ok(Some(p.token));
        }
        if let Some(p) = self.displayed.as_ref().filter(|p| same(p)) {
            return Ok(Some(p.token));
        }
        if self.failed.as_ref().is_some_and(|(key, density)| {
            key == &latest.key && *density == ctx.scale_factor.to_bits()
        }) {
            return Ok(self.fallback());
        }
        let request = latest.request.as_ref().map_err(Clone::clone)?;
        let mut actual_key = latest.key.clone();
        let (image, paint) = if let Some(request) = request {
            if self
                .producer
                .as_ref()
                .is_none_or(|p| p.generation() != images.context().generation())
            {
                self.producer = Some(Preview::new(images.context().clone())?);
            }
            let Some(frame) = self
                .producer
                .as_mut()
                .unwrap()
                .render(request, ctx.scale_factor)?
            else {
                self.waiting = true;
                return Ok(self.fallback());
            };
            actual_key.time = frame.time.to_bits();
            self.waiting = actual_key.time != latest.key.time;
            let handle = images
                .insert(frame.image.clone())
                .map_err(|e| e.to_string())?;
            let paint = frame.paint(handle);
            ((handle, frame.image), paint)
        } else {
            self.producer = None;
            if self
                .checker
                .as_ref()
                .is_none_or(|(colors, _)| colors != &latest.key.checker)
            {
                let [a, b] = latest.key.checker;
                let source = renderkit::ImageAsset::rgba(2, 2, [a, b, b, a].concat())
                    .map_err(|e| e.to_string())?;
                let handle = images.upload(&source).map_err(|e| e.to_string())?;
                let image = images.get(handle).unwrap();
                images.remove(handle);
                self.checker = Some((latest.key.checker, image));
            }
            let image = self.checker.as_ref().unwrap().1.clone();
            let handle = images.insert(image.clone()).map_err(|e| e.to_string())?;
            let mut paint = theframework::paint::PaintList::default();
            paint.ops.push(theframework::paint::PaintOp::Image {
                image: handle,
                rect: theframework::paint::Rect::new(
                    0.,
                    0.,
                    latest.key.size[0] as f32,
                    latest.key.size[1] as f32,
                ),
                uv: theframework::paint::Rect::new(
                    0.,
                    0.,
                    latest.key.size[0] as f32 / 24.,
                    latest.key.size[1] as f32 / 24.,
                ),
                sampling: theframework::paint::Sampling::RepeatNearest,
                tint: [1.; 4],
            });
            ((handle, image), paint)
        };
        let (handle, lease) = image;
        self.serial = self
            .serial
            .checked_add(1)
            .expect("preview receipt space exhausted");
        let token = PreviewToken {
            owner: self.owner,
            serial: self.serial,
        };
        let mut paint = paint;
        paint
            .ops
            .push(theframework::paint::PaintOp::Receipt(token.receipt()));
        let Some(view) = ui.get_render_view(PREVIEW) else {
            images.remove(handle);
            return Ok(None);
        };
        if let Err(error) = view.set_paint_list(Some(Arc::new(paint))) {
            images.remove(handle);
            return Err(error.to_string());
        }
        if let Some(old) = self.staged.replace(Published {
            token,
            key: actual_key,
            image: handle,
            _lease: lease,
            density: ctx.scale_factor.to_bits(),
        }) {
            self.retired.push(old.image);
        }
        self.error = None;
        Ok(Some(token))
    }
}
impl ProceduralMaterialEditor {
    pub(crate) fn enable_gpu_preview(&mut self) {
        self.native.enabled = true;
        self.worker = None;
        self.native.worker_generation = None;
        self.native.asset_job_pending = true;
    }
    pub(super) fn queue_native_preview(&mut self, ui: &mut TheUI, tick: usize) -> bool {
        if !self.active {
            return false;
        }
        let size = ui
            .get_render_view(PREVIEW)
            .map(|view| {
                [
                    view.dim().width.max(0) as u32,
                    view.dim().height.max(0) as u32,
                ]
            })
            .unwrap_or([0; 2]);
        let checker = [
            *ui.style.theme().color(MaterialCheckerDark),
            *ui.style.theme().color(MaterialCheckerLight),
        ];
        let target = if self.preview_tile {
            Target::Selection(self.selection_target())
        } else {
            Target::Root
        };
        self.native.queue(Input {
            tile: self.tile,
            bundle: self.bundle.as_deref(),
            target,
            tick,
            channel: self.channel,
            size,
            checker,
        })
    }
    pub(crate) fn reset_model_gpu(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        { self.model_sources = Default::default(); }
    }
    pub(crate) fn invalidate_gpu_preview(&mut self, ui: &mut TheUI) {
        self.native.retire_gpu();
        self.native.thumbnails.invalidate();
        if let Some(widget) = ui
            .get_widget(EDITOR)
            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
        {
            widget.set_native_thumbnails(Default::default());
        }
        if let Some(view) = ui.get_render_view(PREVIEW) {
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
        tick: usize,
    ) -> Option<PreviewToken> {
        self.native.waiting = false;
        let changed = self.native.registry != Some(images.owner_id());
        self.native.drain(images);
        if !self.native.enabled {
            return None;
        }
        if changed {
            if let Some(view) = ui.get_render_view(PREVIEW) {
                let _ = view.set_paint_list(None);
            }
        }
        let graph_visible = self.active
            && ui
                .get_stack_layout("Sidebar Page Stack")
                .is_none_or(|stack| stack.index() == SIDEBAR_PAGE)
            && ui
                .get_stack_layout(crate::geometry_editor::STACK)
                .is_none_or(|stack| stack.index() == 0);
        if ((graph_visible && self.tile.is_some()) || self.native.needs_asset_worker)
            && self.native.worker_generation != Some(images.context().generation())
        {
            self.worker = Some(gpu::Worker::new_asset_bakes(images.context().clone()));
            self.native.worker_generation = Some(images.context().generation());
            self.native.asset_job_pending = true;
            self.requested.clear();
        }
        let keys = ui
            .get_widget(EDITOR)
            .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
            .filter(|_| graph_visible)
            .map(|w| w.visible_thumbnail_keys())
            .unwrap_or_default();
        let mut thumbnails_waiting = false;
        match self
            .native
            .thumbnails
            .prepare(images, self.bundle.as_deref(), &keys)
        {
            Ok(output) => {
                self.native.thumbnail_error = None;
                thumbnails_waiting = output.waiting;
                if let Some(widget) = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                {
                    widget.set_native_thumbnails(output.handles);
                }
                for error in output.errors {
                    Self::status(ui, ctx, error);
                }
            }
            Err(error) => {
                if let Some(widget) = ui
                    .get_widget(EDITOR)
                    .and_then(|w| w.as_any().downcast_mut::<MaterialEditorWidget>())
                {
                    widget.set_native_thumbnails(Default::default());
                }
                if self.native.thumbnail_error.as_ref() != Some(&error) {
                    Self::status(ui, ctx, error.clone());
                    self.native.thumbnail_error = Some(error);
                }
            }
        }
        let visible = graph_visible
            && ui
                .get_stack_layout(PREVIEW_STACK)
                .is_none_or(|stack| stack.index() == 1);
        if !visible
            || ui
                .get_render_view(PREVIEW)
                .is_none_or(|view| !view.dim().is_valid())
        {
            self.native.retire_gpu();
            self.native.latest = None;
            self.native.drain(images);
            self.native.waiting = thumbnails_waiting;
            if let Some(view) = ui.get_render_view(PREVIEW) {
                let _ = view.set_paint_list(None);
            }
            return None;
        }
        self.queue_native_preview(ui, tick);
        let result = match self.native.output(ui, ctx, images) {
            Ok(token) => token,
            Err(error) => {
                self.native.waiting = false;
                if self.native.error.as_ref() != Some(&error) {
                    Self::status(ui, ctx, error.clone());
                    self.native.error = Some(error);
                }
                self.native.failed = self
                    .native
                    .latest
                    .as_ref()
                    .map(|latest| (latest.key.clone(), ctx.scale_factor.to_bits()));
                self.native.fallback()
            }
        };
        self.native.waiting |= thumbnails_waiting;
        result
    }
    pub(crate) fn presented_gpu_preview(&mut self, token: PreviewToken) -> bool {
        if !self.native.staged.as_ref().is_some_and(|frame| {
            frame.token == token && frame.key.source == self.native.source_epoch
        }) {
            return false;
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
    use renderkit::{GpuContext, ImageDescriptor};
    use theframework::render_wgpu::{PaintRenderer, UiImageCache};
    fn complete(
        host: &mut ProceduralMaterialEditor,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        images: &mut ImageRegistry,
    ) -> PreviewToken {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            if let Some(token) = host.prepare_gpu_preview(ui, ctx, images, 0) {
                if !host.gpu_preview_waiting() {
                    return token;
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "native material output did not complete"
            );
            images
                .context()
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(30)),
                })
                .unwrap();
        }
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_material_panel_uses_shared_device_and_owns_painted_source_receipts() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let (mut ui, mut ctx) = super::super::tests::workspace_ui(1100, 800);
        ctx.ui.command_paint = true;
        let mut project = Project::default();
        project.config = "[ruleset]\nsource='project'\n[materials]\npixels_per_tile=16\n".into();
        let id = Uuid::new_v4();
        let asset = starter(id).unwrap();
        let operation = asset.root.project.materials[0].rows[0].tiles[0].id.clone();
        project
            .procedural_materials
            .insert(id, asset.save().unwrap());
        let mut tile = rusterix::Tile::empty();
        tile.id = id;
        project.tiles.insert(id, tile);
        let mut host = ProceduralMaterialEditor::default();
        host.enable_gpu_preview();
        host.open(id, &mut ui, &mut ctx, &project);
        ui.get_stack_layout(PREVIEW_STACK).unwrap().set_index(1);
        ui.prepare_paint_layout(&mut ctx);
        let mut server = CreatorContext::default();
        host.update(&mut ui, &mut ctx, &mut project, &mut server);
        ui.prepare_paint_layout(&mut ctx);
        let authored = serde_json::to_value(&project).unwrap();
        assert!(
            host.worker.is_none(),
            "CPU update must not discover/create a legacy GPU"
        );
        let mut images = ImageRegistry::new(gpu.clone());
        let token = complete(&mut host, &mut ui, &mut ctx, &mut images);
        assert_eq!(host.native.worker_generation, Some(gpu.generation()));
        assert!(
            host.worker.is_some(),
            "explicit asset jobs get this same injected device"
        );
        assert!(host.native.asset_job_pending);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        let mut cache = UiImageCache::default();
        let list = ui
            .paint_commands(&mut ctx, &mut cache.frame(&mut images))
            .unwrap();
        cache.finish_frame(&mut images, &list).unwrap();
        assert!(list.receipts().any(|r| r == token.receipt()));
        let thumbnails = host.native.thumbnails.handles();
        assert!(
            !thumbnails.is_empty(),
            "the actual material graph must publish GPU thumbnails"
        );
        let painted = list
            .image_handles()
            .collect::<std::collections::HashSet<_>>();
        assert!(
            thumbnails.values().all(|handle| painted.contains(handle)),
            "the root graph must consume GPU thumbnail handles"
        );
        assert!(ui.canvas.buffer.pixels().is_empty());
        let output = GpuImage::new(&gpu, ImageDescriptor::rgba(1100, 800)).unwrap();
        PaintRenderer::new(gpu.clone())
            .unwrap()
            .render(
                &list,
                &images,
                &output.view,
                output.descriptor.format,
                (1100, 800),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert!(host.presented_gpu_preview(token));
        // Explicit review capture; normal display above has no color readback.
        let mut capture = renderkit::ReadbackTicket::capture(&gpu, output).unwrap();
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        let pixels = capture.try_finish(&gpu).unwrap().unwrap();
        std::fs::write(
            "/private/tmp/eldiron-native-material-panel-gpu.png",
            TheRGBABuffer::from(pixels.pixels().to_vec(), 1100, 800)
                .to_png()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&project).unwrap(), authored);
        // The real update owner installs explicit bake results. Display-only
        // selections below must not enqueue another CPU bake/preview job.
        let recipe = project.procedural_materials[&id].clone();
        let expected_key = pixel_style::bake_key(&recipe, 16);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while project.procedural_material_cache_keys.get(&id) != Some(&expected_key) {
            assert!(
                std::time::Instant::now() < deadline,
                "the injected asset bake did not publish"
            );
            host.update(&mut ui, &mut ctx, &mut project, &mut server);
            gpu.poll().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(project.procedural_materials[&id], recipe);
        assert_eq!(project.tiles[&id].textures[0].width, 16);
        assert!(
            host.frame.as_ref().unwrap().thumbnails.is_empty(),
            "authored bake jobs must not return CPU graph display images"
        );
        let authored = serde_json::to_value(&project).unwrap();
        let jobs = host.worker.as_ref().unwrap().submitted_jobs();
        assert!(jobs > 0);
        let statuses = gpu.transfers().status_readback_bytes;
        host.handle_event(
            &TheEvent::IndexChanged(TheId::named("Material Preview Channel"), 2),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        host.update(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(host.worker.as_ref().unwrap().submitted_jobs(), jobs);
        let channel = complete(&mut host, &mut ui, &mut ctx, &mut images);
        assert_ne!(channel, token);
        assert_eq!(
            gpu.transfers().status_readback_bytes,
            statuses,
            "channel changes reuse material maps"
        );
        assert_eq!(host.native.latest.as_ref().unwrap().key.channel, 3);
        assert_eq!(host.native.displayed.as_ref().unwrap().token, token);
        host.preview_tile = true;
        host.selected_operation = Some("missing-operation".into());
        assert_eq!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut images, 0),
            Some(channel)
        );
        assert!(host.native.error.is_some());
        assert!(!host.presented_gpu_preview(channel));
        let before = gpu.transfers();
        assert_eq!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut images, 0),
            Some(channel)
        );
        assert_eq!(gpu.transfers(), before);
        host.selected_operation = Some(operation);
        host.preview_request = true;
        host.update(&mut ui, &mut ctx, &mut project, &mut server);
        assert_eq!(
            host.worker.as_ref().unwrap().submitted_jobs(),
            jobs,
            "selection-only updates must not send CPU display jobs"
        );
        let selected = complete(&mut host, &mut ui, &mut ctx, &mut images);
        assert!(host.presented_gpu_preview(selected));
        assert!(host.native.error.is_none());
        assert_eq!(
            serde_json::to_value(&project).unwrap(),
            authored,
            "view preparation changes no project data"
        );
        let old = host.native.displayed.as_ref().unwrap().image;
        let mut replacement =
            ImageRegistry::new(pollster::block_on(GpuContext::headless()).unwrap());
        let rebuilt = complete(&mut host, &mut ui, &mut ctx, &mut replacement);
        assert!(!host.presented_gpu_preview(selected));
        assert!(host.presented_gpu_preview(rebuilt));
        assert!(
            images.get(old).is_some(),
            "never retire a foreign registry's handles"
        );
        assert_eq!(replacement.context().transfers().readback_bytes, 0);
        ui.get_stack_layout(PREVIEW_STACK).unwrap().set_index(0);
        assert!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement, 0)
                .is_none()
        );
        assert!(!host.presented_gpu_preview(rebuilt));
        ui.get_stack_layout(PREVIEW_STACK).unwrap().set_index(1);
        let reopened = complete(&mut host, &mut ui, &mut ctx, &mut replacement);
        assert!(host.presented_gpu_preview(reopened));
        let current = host.native.displayed.as_ref().unwrap().image;
        ui.get_stack_layout(crate::geometry_editor::STACK)
            .unwrap()
            .set_index(1);
        host.worker = None;
        host.native.worker_generation = None;
        host.update(&mut ui, &mut ctx, &mut project, &mut server);
        assert!(
            !host.native.needs_asset_worker,
            "a fully baked hidden material requests no worker"
        );
        assert!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement, 0)
                .is_none()
        );
        assert!(
            host.worker.is_none(),
            "opening the Meshes sibling must not initialize a material worker"
        );
        assert!(
            host.native.thumbnails.handles().is_empty(),
            "the sibling Meshes page must retire hidden material card outputs"
        );
        assert!(replacement.get(current).is_none());
        ui.get_stack_layout(crate::geometry_editor::STACK)
            .unwrap()
            .set_index(0);
        let restored = complete(&mut host, &mut ui, &mut ctx, &mut replacement);
        assert!(host.presented_gpu_preview(restored));
        let current = host.native.displayed.as_ref().unwrap().image;
        host.reset(&mut ui, &mut ctx);
        assert!(
            host.prepare_gpu_preview(&mut ui, &mut ctx, &mut replacement, 0)
                .is_none()
        );
        assert!(replacement.get(current).is_none());
        assert!(host.native.enabled);
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_material_selected_targets_keep_root_domain_and_reference_animation() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let id = Uuid::new_v4();
        let source = super::super::STARTER.replace("  tiling raw", "  tiling raw\n  animation 3 2");
        let mut asset =
            EditableMaterialAsset::parse("selected-targets", &source, "surface").unwrap();
        let mut domain = denrim_materials::integration::InstanceRecord::new(
            super::super::composition::SURFACE_DOMAIN,
        );
        domain.uv_transform[4] = -0.123;
        domain.uv_transform[5] = 1.8;
        asset.instances.push(domain);
        let bundle = asset.save().unwrap();
        let worker = gpu::Worker::new(gpu.device.clone(), gpu.queue.clone());
        let source = source_catalog()
            .into_iter()
            .find(|s| s.metadata.name == "Perlin Noise")
            .unwrap()
            .id;
        let targets = [
            PreviewSelection::Operation("pigment".into()),
            PreviewSelection::Source(source),
            PreviewSelection::Input(
                MaterialInputTarget::Coordinates {
                    row: "color".into(),
                },
                MaterialReferenceView {
                    row: "color".into(),
                    components: "x".into(),
                    mapping: Some([0.2, 0.8]),
                },
            ),
            PreviewSelection::Material("surface".into()),
        ];
        let checker = [[17, 39, 62, 255], [104, 89, 72, 255]];
        let mut host = Host::default();
        host.pixels = 16;
        let mut producer = Preview::new(gpu.clone()).unwrap();
        for continuous in [false, true] {
            host.continuous = continuous;
            let size = if continuous { [191, 103] } else { [113, 177] };
            for target in &targets {
                worker.send(gpu::Job {
                    pixels_per_tile: 16,
                    tile: id,
                    revision: 7,
                    bundle: bundle.clone(),
                    continuous,
                    selection: Some(target.clone()),
                    preview_aspect: size[0] as f32 / size[1] as f32,
                });
                let reference = worker
                    .results
                    .recv_timeout(std::time::Duration::from_secs(30))
                    .unwrap()
                    .frame
                    .unwrap();
                assert_eq!(reference.animation_selections.len(), 3);
                for tick in 0..3 {
                    host.queue(Input {
                        tile: Some(id),
                        bundle: Some(&bundle),
                        target: Target::Selection(Some(target.clone())),
                        tick,
                        channel: 4,
                        size,
                        checker,
                    });
                    let request = host
                        .latest
                        .as_ref()
                        .unwrap()
                        .request
                        .as_ref()
                        .unwrap()
                        .as_ref()
                        .unwrap();
                    assert_eq!(request.origin, Some([-0.123, 1.8]));
                    assert_eq!(request.time, tick as f32 / 3.);
                    assert_eq!(request.channel, 0, "selected targets always show color");
                    let before = gpu.transfers();
                    let frame = if let Some(frame) = producer.render(request, 1.).unwrap() {
                        frame
                    } else {
                        gpu.device
                            .poll(wgpu::PollType::Wait {
                                submission_index: None,
                                timeout: Some(std::time::Duration::from_secs(30)),
                            })
                            .unwrap();
                        producer.render(request, 1.).unwrap().unwrap()
                    };
                    assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
                    let mut capture =
                        renderkit::ReadbackTicket::capture(&gpu, frame.image.clone()).unwrap();
                    gpu.device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: Some(std::time::Duration::from_secs(30)),
                        })
                        .unwrap();
                    let actual = capture.try_finish(&gpu).unwrap().unwrap();
                    let mut expected =
                        TheRGBABuffer::new(TheDim::sized(size[0] as i32, size[1] as i32));
                    super::super::draw_preview_with_colors(
                        &mut expected,
                        Some((384, &reference.animation_selections[tick], continuous)),
                        checker,
                    );
                    assert_eq!(
                        actual.pixels(),
                        expected.pixels(),
                        "{target:?} tick={tick} continuous={continuous}"
                    );
                }
            }
        }
        assert_eq!(
            asset.save().unwrap(),
            bundle,
            "selection preparation leaves authored data unchanged"
        );
    }
    #[test]
    #[ignore = "requires native GPU"]
    fn native_material_asset_worker_keeps_bake_pixels_and_omits_display_maps() {
        let gpu = pollster::block_on(GpuContext::headless()).unwrap();
        let id = Uuid::new_v4();
        let asset = starter(id).unwrap();
        let bundle = asset.save().unwrap();
        let operation = asset.root.project.materials[0].rows[0].tiles[0].id.clone();
        let worker = gpu::Worker::new_asset_bakes(gpu.clone());
        worker.send(gpu::Job {
            pixels_per_tile: 16,
            tile: id,
            revision: 7,
            bundle: bundle.clone(),
            continuous: true,
            selection: Some(PreviewSelection::Operation(operation.clone())),
            preview_aspect: 3.,
        });
        let native = worker
            .results
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap()
            .frame
            .unwrap();
        assert_eq!(native.size, 16);
        assert_eq!(native.images[0].len(), 16 * 16 * 4);
        assert!(
            native.selection.is_none(),
            "transient selection previews stay GPU resident"
        );
        assert!(
            native.thumbnails.is_empty(),
            "native graph thumbnails never return CPU pixels"
        );
        let reference = gpu::Worker::new(gpu.device.clone(), gpu.queue.clone());
        reference.send(gpu::Job {
            pixels_per_tile: 16,
            tile: id,
            revision: 7,
            bundle,
            continuous: true,
            selection: Some(PreviewSelection::Operation(operation)),
            preview_aspect: 3.,
        });
        let reference = reference
            .results
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap()
            .frame
            .unwrap();
        assert_eq!(reference.size, 384);
        assert!(reference.selection.is_some());
        assert_eq!(
            native.tile, reference.tile,
            "serialized authored cache keeps the exact existing bake output"
        );
        assert!(
            !reference.thumbnails.is_empty(),
            "independent CPU reference thumbnails remain available"
        );
        // Keep the existing authored animation/page policy while replacing its
        // blocking readback implementation with the shared cooperative engine.
        let legacy = gpu::Worker::reference_asset_bakes(gpu.device.clone(), gpu.queue.clone());
        for (mode, frames) in [("raw", 2), ("repeat", 2), ("seamless", 1)] {
            let mut asset = asset.clone();
            asset.root.project.materials[0].tiling.mode =
                denrim_materials::periodic::Mode::parse(mode).unwrap();
            asset.root.project.materials[0].tiling.period = [2., 1.];
            asset.root.project.materials[0].animation.frames = frames;
            super::super::composition::surface_pages::apply(
                &mut asset,
                Some(&rusterix::TileUvPages {
                    origin: [-1, -1],
                    size: [2, 1],
                    axes: [[1., 0., 0.], [0., 0., 1.]],
                    units_per_page: 1,
                    animation_frames: 1,
                    repeat_period: [0.; 2],
                }),
            );
            let bundle = asset.save().unwrap();
            for worker in [&worker, &legacy] {
                assert!(worker.send(gpu::Job {
                    pixels_per_tile: 16,
                    tile: id,
                    revision: 8,
                    bundle: bundle.clone(),
                    continuous: true,
                    selection: None,
                    preview_aspect: 1.
                }));
            }
            let native = worker
                .results
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap()
                .frame
                .unwrap();
            let reference = legacy
                .results
                .recv_timeout(std::time::Duration::from_secs(30))
                .unwrap()
                .frame
                .unwrap();
            assert_eq!(native.tile, reference.tile, "{mode} base tile");
            assert_eq!(native.images, reference.images, "{mode} source maps");
            assert_eq!(
                native.animation_images, reference.animation_images,
                "{mode} animation"
            );
            let native = native.surface_pages.unwrap();
            let reference = reference.surface_pages.unwrap();
            assert_eq!(native.layout, reference.layout, "{mode} page domains");
            assert_eq!(
                native.textures, reference.textures,
                "{mode} serialized page pixels/materials"
            );
        }
        let transfers = gpu.transfers();
        assert!(transfers.asset_bake_readback_requests > 0);
        assert_eq!(
            transfers.readback_bytes, transfers.asset_bake_readback_bytes,
            "all color reads are labeled explicit CPU asset production"
        );
    }

}
