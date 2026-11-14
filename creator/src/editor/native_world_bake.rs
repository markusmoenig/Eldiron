//! Application-owned persisted bake presentation. Progressive work keeps its
//! explicit adapter until its asset capture/scheduling path is adopted.
use super::*;
use eldiron_presentation::orthographic_bake::{OrthographicBakeOverlay, OrthographicBakeRenderer};
use renderkit::{GpuImage, ImageHandle, ImageRegistry};
use rusterix::orthographic_bake::{OrthographicBakeLighting, OrthographicBakeSource};

pub(super) fn supported(asset: &rusterix::map::OrthographicBakeAsset) -> bool {
    u64::from(asset.tile_size)
        .checked_mul(u64::from(asset.tile_size))
        .and_then(|count| count.checked_mul(24))
        .and_then(|bytes| bytes.checked_mul(asset.tiles.len() as u64))
        .is_some_and(|bytes| bytes <= 64 * 1024 * 1024)
}

#[derive(Default)]
pub(super) struct Gpu {
    renderer: Option<OrthographicBakeRenderer>,
    overlay: Option<OrthographicBakeOverlay>,
    image: Option<(Arc<GpuImage>, ImageHandle)>,
}
impl Gpu {
    pub(super) fn clear(&mut self, images: &mut ImageRegistry) {
        if let Some((_, image)) = self.image.take() {
            images.remove(image);
        }
        self.renderer = None;
        self.overlay = None;
    }
    pub(super) fn compose(
        &mut self,
        source: &OrthographicBakeSource<'_>,
        camera: &scenevm::Camera3D,
        output: ViewImage,
        scenes: &mut SceneViews,
        client: &rusterix::Client,
        images: &mut ImageRegistry,
    ) -> Result<ImageHandle, String> {
        if self.renderer.is_none() {
            self.renderer = Some(
                OrthographicBakeRenderer::new(images.context().clone())
                    .map_err(|e| e.to_string())?,
            );
            self.overlay = Some(
                OrthographicBakeOverlay::new(images.context().clone())
                    .map_err(|e| e.to_string())?,
            );
        }
        let settings = &scenes
            .scene(output.view)
            .map_err(|e| e.to_string())?
            .settings;
        let lighting = OrthographicBakeLighting::from_settings(settings);
        let background = rusterix::orthographic_bake::background_color(
            settings,
            true,
            client
                .editor_canvas_colors
                .filter(|_| client.use_editor_canvas)
                .map(|colors| colors[0]),
        );
        let size = (output.descriptor.width, output.descriptor.height);
        let bake = self
            .renderer
            .as_mut()
            .unwrap()
            .render(source, camera, size, Some(lighting))
            .map_err(|e| e.to_string())?;
        let surface = scenes
            .render_surface_output(output, scenevm::surface_output::SurfaceSelection::Dynamic)
            .map_err(|e| e.to_string())?
            .ok_or("bake overlay source is stale")?;
        let overlay = images
            .get(output.image)
            .ok_or("missing bake dynamic image")?;
        let composed = self
            .overlay
            .as_mut()
            .unwrap()
            .render(&bake, &overlay, &surface, background)
            .map_err(|e| e.to_string())?;
        if let Some((old, handle)) = &self.image
            && Arc::ptr_eq(old, &composed)
        {
            return Ok(*handle);
        }
        if let Some((_, image)) = self.image.take() {
            images.remove(image);
        }
        let image = images.insert(composed.clone()).map_err(|e| e.to_string())?;
        self.image = Some((composed, image));
        Ok(image)
    }
}

#[cfg(test)]
pub(crate) struct Fixture {
    project: Project,
    assets: rusterix::Assets,
    camera: crate::editcamera::EditCamera,
    bake: rusterix::OrthographicBakeController,
    context: ProjectContext,
    mode: EditorViewMode,
    history: crate::undo::UndoManager,
    clipboard: Map,
    animation: usize,
    authored: Vec<u8>,
    floor: Uuid,
}

#[cfg(test)]
impl Editor {
    pub(crate) fn begin_native_bake_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> Fixture {
        let mut fixture = Fixture {
            project: self.documents.project.clone(),
            assets: self.server_ctx.reference.read().assets.clone(),
            camera: std::mem::replace(
                &mut self.server_ctx.camera,
                crate::editcamera::EditCamera::new(),
            ),
            bake: std::mem::take(&mut *self.server_ctx.bake.write()),
            context: self.server_ctx.pc,
            mode: self.server_ctx.editor_view_mode,
            history: self.server_ctx.history.read().clone(),
            clipboard: self.server_ctx.clipboard.clone(),
            animation: self.server_ctx.reference.read().client.animation_frame,
            authored: Vec::new(),
            floor: Uuid::nil(),
        };
        let tile = rusterix::Tile::from_texture(rusterix::Texture::new(
            [210, 50, 70, 112].repeat(16),
            4,
            4,
        ));
        let tile_id = tile.id;
        self.documents.project.tiles.insert(tile_id, tile.clone());
        let mut tiles = fixture.assets.tiles.clone();
        tiles.insert(tile_id, tile);
        self.server_ctx.reference.write().assets.set_tiles(tiles);
        let region = self
            .documents
            .project
            .get_region_mut(&self.server_ctx.curr_region)
            .unwrap();
        let id = region.id;
        region.map = Map::new();
        region.iso_paint = Default::default();
        region.map.camera = MapCamera::ThreeDIso;
        let sector = crate::project_templates::rectangle(&mut region.map, -2., -2., 4., 4.);
        region.map.find_sector_mut(sector).unwrap().properties.set(
            "floor_source",
            Value::Source(rusterix::PixelSource::TileId(tile_id)),
        );
        region.map.clear_selection();
        region.map.clear_temp();
        let floor = rusterix::map::geometry_object::GeometryObject::box_from_bounds(
            "Baked picking floor",
            Vec3::new(-2., -0.25, -2.),
            Vec3::new(2., 0., 2.),
        );
        fixture.floor = floor.id;
        region.map.geometry_objects.push(floor);
        region.editing_position_3d = Vec3::zero();
        region.editing_position_iso_3d = Some(Vec3::zero());
        region.editing_iso_scale = Some(3.);
        self.server_ctx.editor_view_mode = EditorViewMode::Iso;
        self.server_ctx.camera.iso_camera.scale = 3.;
        crate::undo::project_helper::set_project_context(
            ctx,
            ui,
            &self.documents.project,
            &mut self.server_ctx,
            ProjectContext::Region(id),
        );
        self.server_ctx.editor_view_mode = EditorViewMode::Iso;
        let camera = self
            .native_world_camera()
            .unwrap()
            .as_scenevm_camera_for_surface(256., 256.);
        let region = self.documents.project.get_region_mut(&id).unwrap();
        for (index, x) in [-0.8f32, 0.8].into_iter().enumerate() {
            let mut item = rusterix::Item::new();
            item.id = 71 + index as u32;
            item.position = camera.right * x + Vec3::new(0., 0.5, 0.);
            item.set_attribute("visible", Value::Bool(true));
            item.set_attribute(
                "source",
                Value::Source(rusterix::PixelSource::TileId(tile_id)),
            );
            region.map.items.push(item);
        }
        let mut source = rusterix::OrthographicBakeController::default();
        source.request_render(region.map.id, 1);
        let work = source
            .prepare_work(region.map.id, 256, 256, camera, Some((-3., 3., -3., 3.)))
            .unwrap()
            .unwrap();
        let mut rgba = Vec::new();
        let mut depth = Vec::new();
        let mut albedo = Vec::new();
        for y in 0..work.tile_size {
            for x in 0..work.tile_size {
                let alpha = if (x + y) % 3 == 0 { 128u32 } else { 255 };
                rgba.extend(
                    [
                        ((x % 64) + 16) * alpha / 255,
                        ((y % 64) + 24) * alpha / 255,
                        48 * alpha / 255,
                        alpha,
                    ]
                    .map(|v| v as u8),
                );
                depth.push(if x < work.tile_size / 2 { 0. } else { 1000. });
                albedo.extend([80, 120, 160, 255]);
            }
        }
        assert!(source.finish_sample());
        assert!(
            source
                .commit_current_tile(
                    rgba,
                    depth,
                    albedo,
                    [128, 255, 128, 255].repeat(256 * 256),
                    [200, 32, 255, 0].repeat(256 * 256)
                )
                .unwrap()
        );
        region.map.orthographic_bake = source.take_persisted_update().unwrap();
        region.map.changed += 1;
        fixture.authored = serde_json::to_vec(&region.map).unwrap();
        std::fs::write(
            "/private/tmp/eldiron-native-bake-project.json",
            serde_json::to_vec(&self.documents.project).unwrap(),
        )
        .unwrap();
        self.server_ctx.hover = (None, None, None);
        self.server_ctx.geo_hit = None;
        self.server_ctx.hover_cursor = None;
        self.server_ctx.hover_cursor_3d = None;
        self.invalidate_native_world_viewport(ui);
        fixture
    }
    pub(crate) fn set_native_bake_phase_for_test(
        &mut self,
        fixture: &mut Fixture,
        phase: usize,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        use crate::actions::Action;
        if matches!(phase, 1 | 2) {
            crate::actions::orthographic_bake::ToggleOrthographicBakeVisibility::new().apply(
                scope::map_mut(&mut self.documents.project, &self.server_ctx).unwrap(),
                ui,
                ctx,
                &mut self.server_ctx,
            );
        }
        if phase == 3 {
            let map = scope::map_mut(&mut self.documents.project, &self.server_ctx).unwrap();
            map.orthographic_bake.as_mut().unwrap().version = 3;
            map.changed += 1;
            fixture.authored = serde_json::to_vec(map).unwrap();
        }
        if phase == 4 {
            self.documents
                .project
                .get_region_mut(&self.server_ctx.curr_region)
                .unwrap()
                .editing_position_3d += Vec3::new(0.25, 0., -0.125);
        }
        self.server_ctx.reference.write().client.animation_frame = 0;
    }
    pub(crate) fn native_bake_image_for_test(
        &self,
        fixture: &Fixture,
    ) -> Option<(ImageHandle, (u32, u32))> {
        let world = &self.native_viewport.world;
        let frame = world.presented.as_ref().filter(|frame| frame.ready)?;
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        assert_eq!(serde_json::to_vec(map).unwrap(), fixture.authored);
        let gpu = world.gpu.as_ref().unwrap();
        assert!(
            !gpu.scenes
                .scene(gpu.view)
                .unwrap()
                .vm
                .has_surface_readback_storage()
        );
        let camera = self
            .native_world_camera()
            .unwrap()
            .as_scenevm_camera_for_surface(
                frame.binding.size.0 as f32,
                frame.binding.size.1 as f32,
            );
        let baked = self
            .server_ctx
            .bake
            .read()
            .presentation_source(map.id, &camera)
            .is_some();
        assert_eq!(gpu.bake.image.is_some(), baked);
        assert_eq!(frame.baked, baked);
        Some((
            gpu.bake
                .image
                .as_ref()
                .map(|(_, image)| *image)
                .unwrap_or(frame.output.image),
            (frame.binding.size.0 as u32, frame.binding.size.1 as u32),
        ))
    }
    pub(crate) fn check_native_bake_pick_for_test(&mut self, fixture: &Fixture) {
        let frame = self.native_viewport.world.presented.clone().unwrap();
        assert!(
            frame.source.chunks().values().any(|chunk| chunk
                .polys3d_map
                .keys()
                .any(|geo| *geo == scenevm::GeoId::GeometryObject(fixture.floor))),
            "bake fixture must retain actual solid geometry, not only a guide plane"
        );
        let size = frame.binding.size;
        let camera = self
            .native_world_camera()
            .unwrap()
            .as_scenevm_camera_for_surface(size.0 as f32, size.1 as f32);
        let at = crate::tools::geometry::project_geometry_point_to_screen(
            &camera,
            Vec3::zero(),
            size.0 as f32,
            size.1 as f32,
        )
        .unwrap();
        let gpu = self.native_viewport.world.gpu.as_mut().unwrap();
        let picked = gpu
            .scenes
            .pick_editor_surface_filtered(
                frame.output,
                [at.x / size.0 as f32, at.y / size.1 as f32],
                |geo| geo == scenevm::GeoId::GeometryObject(fixture.floor),
            )
            .unwrap()
            .base
            .unwrap();
        assert_eq!(picked.0, scenevm::GeoId::GeometryObject(fixture.floor));
        assert!(
            picked.1.y.abs() < 0.01,
            "baked display keeps the acknowledged static floor query"
        );
    }
    pub(crate) fn native_bake_reference_for_test(
        &self,
        gpu: &Arc<renderkit::GpuContext>,
        size: (u32, u32),
    ) -> Vec<u8> {
        let bridge = self.server_ctx.reference.read();
        let world = &self.native_viewport.world;
        let native = world.gpu.as_ref().unwrap();
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        let camera = self.native_world_camera().unwrap();
        let mut reference =
            rusterix::Rusterix::with_scene_handler(rusterix::SceneHandler::with_vm(
                scenevm::SceneVM::new_with_context(gpu.clone(), size.0, size.1),
            ));
        reference.assets = bridge.assets.clone();
        reference.scene_handler.settings =
            native.scenes.scene(native.view).unwrap().settings.clone();
        reference
            .scene_handler
            .build_atlas(&reference.assets.tiles, true);
        for chunk in world.presented.as_ref().unwrap().source.chunks().values() {
            reference.scene_handler.build_index.index_chunk(chunk);
            reference.scene_handler.vm.execute(scenevm::Atom::AddChunk {
                id: Uuid::new_v4(),
                chunk: chunk.as_ref().clone(),
            });
        }
        reference.scene_handler.runtime_render_state = bridge.render_state_for_view(map, true);
        reference.client.editor_canvas_colors = bridge.client.editor_canvas_colors;
        reference.client.use_editor_canvas = bridge.client.use_editor_canvas;
        reference.client.server_time = bridge.client.server_time;
        reference.client.animation_frame = 0;
        reference.scene_handler.set_entity_item_selection_visible(
            self.server_ctx
                .tools
                .read()
                .current_game_tool_is("tool.entity"),
        );
        reference.scene_handler.billboards =
            native.scenes.scene(native.view).unwrap().billboards.clone();
        reference.client.camera_d3 = self.native_world_camera().unwrap();
        // The compatibility facade flushes its projection-specific atlas before
        // publishing dynamics. A cold facade otherwise clears these actors in
        // its first draw, producing an invalid zero-dynamic reference surface.
        reference.build_dynamics_3d(map, 0);
        self.server_ctx
            .tools
            .write()
            .prepare_geometry_overlay_3d_for_map(
                &self.documents.project,
                &self.server_ctx,
                &reference.assets,
                camera.as_ref(),
                &mut reference.scene_handler,
                Some(map),
                None,
            );
        reference.client.camera_d3 = camera;
        let camera = reference
            .client
            .camera_d3
            .as_scenevm_camera_for_surface(size.0 as f32, size.1 as f32);
        let mut bake = rusterix::OrthographicBakeController::default();
        bake.sync_persisted_asset(map.id, map.orthographic_bake.as_ref());
        if self
            .server_ctx
            .bake
            .read()
            .presentation_source(map.id, &camera)
            .is_none()
        {
            bake.toggle_visibility();
        }
        let mut pixels = vec![0; size.0 as usize * size.1 as usize * 4];
        reference.draw_d3_with_bake_controller(
            map,
            &mut pixels,
            size.0 as usize,
            size.1 as usize,
            true,
            &mut bake,
        );
        if std::env::var_os("ELDIRON_BAKE_DEBUG").is_some() {
            reference.scene_handler.vm.set_active_vm(0);
            let surface = reference
                .scene_handler
                .vm
                .paint_surface_buffer_gpu_dynamic_only(size.0, size.1)
                .unwrap();
            let depth = bake.compose_depth(map.id, size.0, size.1, &camera);
            eprintln!(
                "Bake reference dynamic valid={} size={size:?} camera={camera:?}",
                surface.pixels.iter().filter(|p| p.valid).count()
            );
            for (x, y) in [(277, 186), (303, 201), (315, 208), (450, 200)] {
                let index = y * size.0 as usize + x;
                eprintln!(
                    "Bake reference surface {x},{y}: {:?}, baked={:?}",
                    surface.pixels[index],
                    depth.as_ref().map(|depth| depth[index])
                );
            }
        }
        pixels
    }
    pub(crate) fn request_native_progressive_bake_for_test(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) -> (
        serde_json::Value,
        Option<rusterix::map::OrthographicBakeAsset>,
        u64,
    ) {
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        let mut authored = serde_json::to_value(map).unwrap();
        authored
            .as_object_mut()
            .unwrap()
            .remove("orthographic_bake");
        authored.as_object_mut().unwrap().remove("changed");
        let old = map.orthographic_bake.clone();
        let revision = self.server_ctx.history.read().revision();
        let params = {
            let actions = self.server_ctx.actions.read();
            let mut params = actions
                .get_action_by_command_id("bake.render")
                .unwrap()
                .params();
            params.set_i32_value("actionBakeSamples", 2);
            crate::actions::nodeui_to_toml(&params)
        };
        self.sidebar
            .execute_action_command(
                &crate::actionscript::EditorActionRequest {
                    command_id: "bake.render".into(),
                    parameters_toml: params,
                },
                ui,
                ctx,
                &mut self.documents.project,
                &mut self.server_ctx,
            )
            .unwrap();
        (authored, old, revision)
    }
    pub(crate) fn native_progressive_bake_complete_for_test(
        &self,
        expected: &serde_json::Value,
        old: &Option<rusterix::map::OrthographicBakeAsset>,
        revision: u64,
    ) -> bool {
        let map = scope::map(&self.documents.project, &self.server_ctx).unwrap();
        let mut authored = serde_json::to_value(map).unwrap();
        authored
            .as_object_mut()
            .unwrap()
            .remove("orthographic_bake");
        authored.as_object_mut().unwrap().remove("changed");
        assert_eq!(
            &authored, expected,
            "bake cannot modify authored geometry/selection"
        );
        let bake = self.server_ctx.bake.read();
        assert!(
            !self
                .native_viewport
                .world
                .bake_jobs
                .uses_adapter(self.documents.active_id(), bake.active_request()),
            "controlled progressive fixture unexpectedly selected its adapter"
        );
        if bake.is_rendering() {
            assert_eq!(
                &map.orthographic_bake, old,
                "preview cannot persist partial tiles"
            );
            return false;
        }
        if &map.orthographic_bake == old {
            return false;
        }
        if !self
            .native_viewport
            .world
            .presented
            .as_ref()
            .is_some_and(|frame| frame.ready && frame.baked && frame.stamp == SourceStamp::new(map))
        {
            return false;
        }
        let asset = map.orthographic_bake.as_ref().unwrap();
        assert_eq!(asset.version, 5);
        assert_eq!(asset.samples, 2);
        assert!(!asset.tiles.is_empty());
        let source = bake
            .presentation_source(
                map.id,
                &self
                    .native_world_camera()
                    .unwrap()
                    .as_scenevm_camera_for_surface(256., 256.),
            )
            .unwrap();
        assert!(
            source
                .tiles()
                .any(|(_, tile)| tile.depth.iter().any(|depth| depth.is_finite()))
        );
        assert!(source.tiles().any(|(_, tile)| {
            tile.material
                .chunks_exact(4)
                .any(|p| p[0] != 0 || p[2] != 0)
        }));
        assert!(self.server_ctx.history.read().revision() != revision);
        assert!(self.server_ctx.history.read().has_unsaved());
        assert!(self.document_info().unwrap().dirty);
        true
    }
    pub(crate) fn native_generated_bake_image_for_test(&self) -> (ImageHandle, (u32, u32)) {
        let world = &self.native_viewport.world;
        let frame = world.presented.as_ref().unwrap();
        (
            world.gpu.as_ref().unwrap().bake.image.as_ref().unwrap().1,
            (frame.binding.size.0 as u32, frame.binding.size.1 as u32),
        )
    }
    pub(crate) fn check_native_generated_bake_history_for_test(
        &mut self,
        old: &Option<rusterix::map::OrthographicBakeAsset>,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        let generated = scope::map(&self.documents.project, &self.server_ctx)
            .unwrap()
            .orthographic_bake
            .clone();
        self.server_ctx.history.write().undo(
            &mut self.server_ctx,
            &mut self.documents.project,
            ui,
            ctx,
        );
        assert_eq!(
            &scope::map(&self.documents.project, &self.server_ctx)
                .unwrap()
                .orthographic_bake,
            old
        );
        self.server_ctx.history.write().redo(
            &mut self.server_ctx,
            &mut self.documents.project,
            ui,
            ctx,
        );
        assert_eq!(
            scope::map(&self.documents.project, &self.server_ctx)
                .unwrap()
                .orthographic_bake,
            generated
        );
        let document = self.document_info().unwrap().document;
        let save = self.prepare_document_save(document, ctx).unwrap();
        std::fs::write(
            "/private/tmp/eldiron-native-generated-bake.eldiron",
            self.document_save_bytes(save.ticket).unwrap(),
        )
        .unwrap();
        assert!(
            self.complete_document_save(save.ticket, true, None, ui, ctx)
                .unwrap()
                .clean
        );
    }
    pub(crate) fn finish_native_bake_for_test(
        &mut self,
        fixture: Fixture,
        ui: &mut TheUI,
        ctx: &mut TheContext,
    ) {
        self.documents.project = fixture.project;
        self.server_ctx.reference.write().assets = fixture.assets;
        self.server_ctx.reference.write().client.animation_frame = fixture.animation;
        self.server_ctx.camera = fixture.camera;
        *self.server_ctx.bake.write() = fixture.bake;
        *self.server_ctx.history.write() = fixture.history;
        self.server_ctx.clipboard = fixture.clipboard;
        self.server_ctx.editor_view_mode = fixture.mode;
        self.server_ctx.hover = (None, None, None);
        self.server_ctx.geo_hit = None;
        crate::undo::project_helper::set_project_context(
            ctx,
            ui,
            &self.documents.project,
            &mut self.server_ctx,
            fixture.context,
        );
        self.server_ctx.editor_view_mode = fixture.mode;
        self.server_ctx.history.read().set_undo_state_to_ui(ctx);
        self.invalidate_native_world_viewport(ui);
    }
}
