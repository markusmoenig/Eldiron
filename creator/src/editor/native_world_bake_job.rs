//! Application-owned frozen bake requests. CPU preparation/publication belongs
//! to update; original-channel GPU work is scheduled independently of the view.
use super::*;
use eldiron_presentation::orthographic_bake::{BakeAdvance, OrthographicBakeJob};
use renderkit::ImageRegistry;
use rusterix::editor_view::EditorView3D;
use rusterix::server::message::RuntimeRenderState;
#[path = "native_world_bake_paint.rs"]
mod paint;

#[derive(Clone, PartialEq)]
struct Key {
    document: Uuid,
    region: Uuid,
    context: ProjectContext,
    source: SourceStamp,
    assets: AssetKey,
    config: String,
    axes: [[u32; 3]; 3],
    paint: paint::Stamp,
    request: Uuid,
}
struct Job {
    key: Key,
    source: crate::scene_source::Source,
    gpu: Option<OrthographicBakeJob>,
    frame: EditorView3D,
    render: RuntimeRenderState,
    viewport: (u32, u32),
    paint: Option<paint::Snapshot>,
}
#[derive(Default)]
pub(super) struct Jobs {
    generation: Option<u64>,
    active: Option<Job>,
    legacy: Option<Key>,
}
impl Jobs {
    #[cfg(any(target_arch = "wasm32", feature = "staticlib"))]
    pub(super) fn diagnostics(
        &self,
        controller: &rusterix::OrthographicBakeController,
    ) -> serde_json::Value {
        serde_json::json!({
            "rendering": controller.is_rendering(),
            "status": format!("{:?}", controller.status()),
            "job": self.active.as_ref().map(|job| serde_json::json!({
                "document": job.key.document, "map": job.key.source.map, "request": job.key.request,
                "preparing": !job.source.ready(), "gpu": job.gpu.is_some(),
                "pending": job.gpu.as_ref().is_some_and(|gpu| gpu.has_pending_capture()),
            })),
            "adapter": self.legacy.is_some() && controller.is_rendering(),
        })
    }
    pub(super) fn uses_adapter(
        &self,
        document: Option<Uuid>,
        request: Option<(Uuid, Uuid)>,
    ) -> bool {
        self.legacy.as_ref().is_some_and(|key| {
            document == Some(key.document) && request == Some((key.source.map, key.request))
        })
    }
}
impl Editor {
    fn native_bake_key(
        &self,
        ui: &mut TheUI,
        request: (Uuid, Uuid),
        sampling_viewport: Option<(u32, u32)>,
    ) -> Option<(Key, scenevm::Camera3D, (u32, u32))> {
        if !self.native_viewport.enabled
            || self.server_ctx.pc.is_prefab()
            || self.server_ctx.get_map_context() != MapContext::Region
            || self.server_ctx.editor_view_mode != EditorViewMode::Iso
        {
            return None;
        }
        let document = self.documents.active_id()?;
        let region = self
            .documents
            .project
            .get_region(&self.server_ctx.curr_region)?;
        if region.map.id != request.0 {
            return None;
        }
        let viewport = ui
            .get_render_view("PolyView")
            .and_then(|view| {
                let dim = view.dim();
                (dim.width > 0 && dim.height > 0).then_some((dim.width as u32, dim.height as u32))
            })
            .or(sampling_viewport)?;
        let camera = self
            .native_world_camera()?
            .as_scenevm_camera_for_surface(viewport.0 as f32, viewport.1 as f32);
        let bridge = self.server_ctx.reference.read();
        let mut assets = self.native_world_asset_key(&bridge.assets);
        // Actor animation cache publications cannot change static transport.
        assets.avatars = 0;
        let paint = paint::Stamp::new(region, &self.documents.project);
        let bits = |v: Vec3<f32>| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()];
        Some((
            Key {
                document,
                region: region.id,
                context: self.server_ctx.pc,
                source: SourceStamp::new(&region.map),
                assets,
                config: self.native_world_geometry_config(),
                axes: [bits(camera.forward), bits(camera.right), bits(camera.up)],
                paint,
                request: request.1,
            },
            camera,
            viewport,
        ))
    }
    pub(crate) fn cancel_native_bake_job(&mut self) {
        self.native_viewport.world.bake_jobs.active = None;
        self.native_viewport.world.bake_jobs.legacy = None;
        if self.server_ctx.bake.initialized() {
            self.server_ctx.bake.write().cancel_work();
        }
        self.server_ctx.background_progress = None;
    }
    pub(crate) fn retire_native_bake_device(&mut self, generation: u64) {
        let old = self.native_viewport.world.bake_jobs.generation;
        if old.is_some_and(|old| old != generation) {
            self.cancel_native_bake_job();
            // Also discard a completed-but-unpublished old-generation result.
            if self.server_ctx.bake.initialized() {
                *self.server_ctx.bake.write() = rusterix::OrthographicBakeController::default();
            }
        }
        self.native_viewport.world.bake_jobs.generation = Some(generation);
    }
    pub(crate) fn native_bake_waiting(&self) -> bool {
        self.native_viewport.enabled
            && self.server_ctx.bake.initialized()
            && self.server_ctx.bake.read().is_rendering()
            && !self.native_viewport.world.bake_jobs.uses_adapter(
                self.documents.active_id(),
                self.server_ctx.bake.read().active_request(),
            )
    }
    /// Runs before viewport selection, including the retained reference adapter.
    pub(crate) fn update_native_bake(&mut self, ui: &mut TheUI, ctx: &mut TheContext) {
        if !self.native_viewport.enabled || !self.server_ctx.bake.initialized() {
            return;
        }
        let mut jobs = std::mem::take(&mut self.native_viewport.world.bake_jobs);
        let request = self.server_ctx.bake.read().active_request();
        let identity = request.or_else(|| {
            jobs.active
                .as_ref()
                .map(|job| (job.key.source.map, job.key.request))
        });
        // Once requested, asset sampling belongs to the job. A hidden/rebuilt
        // world widget cannot revoke its source validation or publication scope.
        let sampling_viewport = jobs.active.as_ref().map(|job| job.viewport);
        let candidate =
            identity.and_then(|request| self.native_bake_key(ui, request, sampling_viewport));
        let Some((key, camera, viewport)) = candidate else {
            // Reconciliation has already retired invalid map/camera/Play work.
            // A zero-size layout can resume the same request without GPU work.
            if request.is_none() {
                if jobs.active.is_some() {
                    // This is an invalid document/map/mode, not missing layout:
                    // a completed native result must never remain unowned.
                    *self.server_ctx.bake.write() = rusterix::OrthographicBakeController::default();
                    self.server_ctx.background_progress = None;
                }
                jobs.active = None;
                jobs.legacy = None;
            }
            self.native_viewport.world.bake_jobs = jobs;
            return;
        };
        if let Some(old) = jobs.legacy.as_ref().filter(|old| *old != &key) {
            if request.is_some_and(|(_, request)| request == old.request) {
                self.server_ctx.bake.write().cancel_work();
                self.server_ctx.background_progress = None;
                jobs.legacy = None;
                self.native_viewport.world.bake_jobs = jobs;
                ctx.ui.redraw_all = true;
                return;
            }
            jobs.legacy = None;
        }
        if jobs.active.as_ref().is_some_and(|old| old.key != key) {
            let old = jobs.active.take().unwrap();
            if request.is_none_or(|(_, request)| request == old.key.request) {
                // Source changed without a new Render command. Discard pending
                // publication as well as samples, then restore authored data.
                *self.server_ctx.bake.write() = rusterix::OrthographicBakeController::default();
                let map = &self.documents.project.get_region(&key.region).unwrap().map;
                self.server_ctx
                    .bake
                    .write()
                    .sync_persisted_asset(map.id, map.orthographic_bake.as_ref());
                self.server_ctx.background_progress = None;
                ctx.ui.redraw_all = true;
                self.native_viewport.world.bake_jobs = jobs;
                return;
            }
        }
        if jobs.active.is_some() && request.is_none() {
            if let Some(asset) = self
                .server_ctx
                .bake
                .write()
                .take_persisted_update_for(key.source.map)
            {
                let map = &mut self
                    .documents
                    .project
                    .get_region_mut(&key.region)
                    .unwrap()
                    .map;
                let before = map.clone();
                map.orthographic_bake = asset;
                map.changed = map.changed.wrapping_add(1);
                let after = map.clone();
                self.server_ctx.history.write().add_undo(
                    ProjectUndoAtom::MapEdit(
                        ProjectContext::Region(key.region),
                        Box::new(before),
                        Box::new(after),
                    ),
                    ctx,
                );
                if !self.documents.sessions.is_empty() {
                    self.documents.sessions[self.documents.active_session].dirty = true;
                }
                ctx.ui.redraw_all = true;
            }
            jobs.active = None;
            self.server_ctx.background_progress = None;
        }
        if let Some((map, _)) = request {
            if jobs.active.is_none() && jobs.legacy.is_none() {
                let bridge = self.server_ctx.reference.read();
                let region = self.documents.project.get_region(&key.region).unwrap();
                if !key.paint.supported()
                    || region
                        .map
                        .orthographic_bake
                        .as_ref()
                        .is_some_and(|asset| !bake::supported(asset))
                {
                    // Above-budget source paging remains an explicit adapter;
                    // no original paint or bake channels are silently omitted.
                    jobs.legacy = Some(key.clone());
                } else {
                    let mut projection = region.map.clone();
                    self.project_native_world_geometry(&mut projection);
                    let source = crate::scene_source::Source::for_editor(
                        crate::scene_source::SourceKey {
                            document: key.document,
                            map,
                            source: u64::from(key.source.changed),
                            assets: key.assets.source,
                        },
                        projection,
                        &bridge.assets,
                    );
                    let frame = EditorView3D {
                        size: viewport,
                        grid_size: region.map.grid_size,
                        offset: region.map.offset,
                        hour: bridge.client.server_time.to_f32(),
                        animation_frame: bridge.client.animation_frame,
                        camera,
                        overlay_camera: camera,
                        canvas_colors: bridge
                            .client
                            .editor_canvas_colors
                            .filter(|_| bridge.client.use_editor_canvas),
                        neutral_background: true,
                        static_geometry: true,
                        transparent_background: false,
                    };
                    jobs.active = Some(Job {
                        key: key.clone(),
                        source,
                        gpu: None,
                        frame,
                        render: bridge.render_state_for_view(&region.map, false),
                        viewport,
                        paint: Some(paint::Snapshot::capture(
                            region,
                            &self.documents.project,
                            &bridge.assets,
                        )),
                    });
                }
            }
            if let Some(job) = jobs.active.as_mut() {
                job.source.advance();
            }
        }
        self.server_ctx.background_progress = self.server_ctx.bake.read().progress_text();
        self.native_viewport.world.bake_jobs = jobs;
    }
    /// Called for every drawable root, even when another dock hides the world.
    pub(crate) fn prepare_native_bake(&mut self, ctx: &mut TheContext, images: &ImageRegistry) {
        self.retire_native_bake_device(images.context().generation());
        let mut jobs = std::mem::take(&mut self.native_viewport.world.bake_jobs);
        if let Some(job) = jobs.active.as_mut() {
            if job.source.ready() && self.server_ctx.bake.read().is_rendering() {
                let result = (|| -> Result<BakeAdvance, String> {
                    if job.gpu.is_none() {
                        let vm = scenevm::SceneVM::new_with_context_and_atlas(
                            images.context().clone(),
                            1,
                            1,
                            scenevm::SharedAtlas::new(1024, 1024),
                        );
                        let mut scene = rusterix::SceneHandler::with_vm(vm);
                        eldiron_presentation::configure_scene_assets(
                            &mut scene,
                            &self.server_ctx.reference.read().assets,
                            true,
                        )
                        .map_err(|error| error.to_string())?;
                        scene.runtime_render_state = job.render.clone();
                        for chunk in job.source.snapshot().chunks().values() {
                            scene.build_index.index_chunk(chunk);
                            scene.vm.execute(scenevm::Atom::AddChunk {
                                id: Uuid::new_v4(),
                                chunk: chunk.as_ref().clone(),
                            });
                        }
                        // Authoring paint uses the same original-world source
                        // policy as the displayed view. Runtime surface_paint
                        // caches are derived game inputs, not authored layers.
                        job.paint.take().unwrap().upload(
                            &mut scene,
                            job.frame.camera,
                            job.viewport,
                        );
                        job.gpu = Some(
                            OrthographicBakeJob::new(
                                images.context().clone(),
                                scene,
                                job.frame.clone(),
                            )
                            .map_err(|error| error.to_string())?,
                        );
                    }
                    job.gpu
                        .as_mut()
                        .unwrap()
                        .advance(
                            &mut self.server_ctx.bake.write(),
                            job.key.source.map,
                            job.frame.camera,
                            job.viewport,
                        )
                        .map_err(|error| error.to_string())
                })();
                match result {
                    Ok(BakeAdvance::Retired) => {
                        jobs.active = None;
                    }
                    Ok(_) => (),
                    Err(error) => {
                        self.server_ctx.bake.write().fail(error.clone());
                        self.server_ctx.background_progress = None;
                        jobs.active = None;
                        ctx.ui.send(TheEvent::SetStatusText(TheId::empty(), error));
                        ctx.ui.redraw_all = true;
                    }
                }
            }
        }
        self.native_viewport.world.bake_jobs = jobs;
    }
}

#[cfg(test)]
#[path = "native_world_bake_scope_tests.rs"]
mod scope_tests;
