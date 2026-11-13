//! Runs inside the existing assembled Creator checkpoint, not a separate suite.
use super::*;
use renderkit::{GpuContext, ImageRegistry, ReadbackTicket};
use theframework::render_wgpu::PaintRenderer;

fn capture(gpu: &GpuContext, mut ticket: ReadbackTicket) -> Vec<u8> {
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(gpu).unwrap().unwrap().pixels().to_vec()
}
pub(crate) fn check(gpu: &Arc<GpuContext>, renderer: &mut PaintRenderer) {
    // Suppress unrelated dock previews while exercising the real Sidebar route.
    // Editor state and concurrent authoring content are restored on all exits.
    use crate::{dockmanager::DockManagerState, editor::DOCKMANAGER, sidebar::Sidebar};
    struct Plain;
    impl Dock for Plain {
        fn new() -> Self {
            Self
        }
        fn setup(&mut self, _: &mut TheContext) -> TheCanvas {
            TheCanvas::new()
        }
    }
    const KEY: &str = "Scene Minimap Checkpoint";
    struct Restore(DockManagerState, usize, String);
    impl Drop for Restore {
        fn drop(&mut self) {
            let mut manager = DOCKMANAGER.write().unwrap();
            manager.docks.shift_remove(KEY);
            manager.state = self.0;
            manager.index = self.1;
            manager.dock = self.2.clone();
        }
    }
    let _restore = {
        let mut manager = DOCKMANAGER.write().unwrap();
        assert!(!manager.docks.contains_key(KEY));
        let saved = Restore(manager.state, manager.index, manager.dock.clone());
        manager.docks.insert(KEY.into(), Box::new(Plain));
        manager.index = manager.docks.get_index_of(KEY).unwrap();
        manager.state = DockManagerState::Minimized;
        manager.dock = KEY.into();
        saved
    };
    let mut assets = rusterix::Assets::default();
    let mut tile = rusterix::Tile::from_texture(rusterix::Texture::new(
        [100, 180, 80, 255].repeat(16 * 32),
        16,
        32,
    ));
    tile.isometric = Some(rusterix::isometric::SpriteLayout {
        anchor: [8, 30],
        pixels_per_unit: 8.,
        directions: 1,
        frames_per_direction: 1,
        ..Default::default()
    });
    let tile_id = tile.id;
    let flat = rusterix::Tile::from_texture(rusterix::Texture::new(
        [200, 70, 40, 255].repeat(4 * 4),
        4,
        4,
    ));
    let flat_id = flat.id;
    assets.set_tiles([(tile_id, tile), (flat_id, flat)].into_iter().collect());
    let mut project = Project::default();
    project.regions.clear();
    let mut author = Region::new();
    author.map = Map::new();
    let map = &mut author.map;
    let vertices =
        [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]].map(|p| map.add_vertex_at(p[0], p[1]));
    for i in 0..4 {
        map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
    }
    let sector = map.close_polygon_manual().unwrap();
    map.find_sector_mut(sector).unwrap().properties.set(
        "source",
        rusterix::Value::Source(rusterix::PixelSource::TileId(tile_id)),
    );
    author.editing_position_3d = Vec3::new(0.25, 0., 0.5);
    author.editing_look_at_3d = Vec3::new(-0.5, 0., 0.75);
    let id = author.id;
    project.regions.push(author);
    let mut server = ServerContext::new();
    server.pc = ProjectContext::Region(id);
    server.curr_region = id;
    server.editor_view_mode = EditorViewMode::FirstP;
    let mut images = ImageRegistry::new(gpu.clone());
    let mut source_scene =
        rusterix::SceneHandler::with_vm(scenevm::SceneVM::new_with_context(gpu.clone(), 1, 1));
    let mut chunk = scenevm::Chunk::new(Vec2::zero(), 8);
    chunk.add(scenevm::Poly2D::quad(
        scenevm::GeoId::Unknown(1),
        flat_id,
        [0., 0.],
        1.25,
        0,
        true,
    ));
    source_scene.build_index.index_chunk(&chunk);
    let source = source_scene.build_index.snapshot();
    for mode in 0..3 {
        project.regions[0].map.camera = if mode == 2 {
            MapCamera::TwoDIso
        } else {
            MapCamera::TwoD
        };
        assets.config = if mode == 1 {
            "[renderer]\nbackend_2d='compute'".into()
        } else {
            String::new()
        };
        let mut reference_scene =
            eldiron_presentation::SceneViews::new(&images, scenevm::SharedAtlas::new(1024, 1024));
        let reference_id = ViewId::new();
        reference_scene.create(reference_id, (0, 0)).unwrap();
        reference_scene
            .configure_editor(reference_id, &assets)
            .unwrap();
        reference_scene
            .apply_editor_source(reference_id, source.clone())
            .unwrap();
        let mut sidebar = Sidebar::new();
        let mut ctx = TheContext::new(143, 83, 1.);
        ctx.ui.command_paint = true;
        let mut ui = TheUI::new();
        let mut view = TheRenderView::new(TheId::named("MiniMap"));
        let view_id = view.id().clone();
        view.set_dim(TheDim::new(13, 29, 143, 83), &mut ctx);
        view.render_buffer_mut().resize(143, 83);
        ui.canvas.set_widget(view);
        assert!(sidebar.handle_event(
            &TheEvent::Custom(TheId::named("Soft Update Minimap"), TheValue::Empty),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server
        ));
        let mut owner = std::mem::take(&mut sidebar.minimap_scene);
        let before = gpu.transfers();
        let prepare = |owner: &mut SceneOverview,
                       project: &Project,
                       ui: &mut TheUI,
                       ctx: &TheContext,
                       images: &mut ImageRegistry,
                       ready| {
            owner
                .prepare(
                    project,
                    &server,
                    ui,
                    ctx,
                    &assets,
                    mode + 1,
                    source.clone(),
                    ready,
                    0,
                    images,
                )
                .unwrap()
        };
        if mode != 2 {
            assert!(prepare(&mut owner, &project, &mut ui, &ctx, &mut images, false).is_none());
            assert!(
                owner
                    .navigate(&view_id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                    .is_err()
            );
        }
        let token = prepare(&mut owner, &project, &mut ui, &ctx, &mut images, true).unwrap();
        assert!(
            !ui.get_render_view("MiniMap")
                .unwrap()
                .has_cpu_display_storage()
        );
        assert!(
            !owner
                .gpu
                .as_ref()
                .unwrap()
                .scenes
                .has_cpu_target_storage(owner.gpu.as_ref().unwrap().view)
                .unwrap()
        );
        assert_eq!(before.readback_bytes, gpu.transfers().readback_bytes);
        assert!(
            owner
                .navigate(
                    &view_id,
                    Vec2::new(70, 45),
                    &project,
                    &server,
                    &mut ui,
                    &ctx
                )
                .is_err()
        );
        owner.presented(Some(token.clone()));
        let position = owner
            .navigate(
                &view_id,
                Vec2::new(70, 45),
                &project,
                &server,
                &mut ui,
                &ctx,
            )
            .unwrap()
            .unwrap();
        assert!(position.x.is_finite() && position.y.is_finite());
        let frame = owner.current.as_ref().unwrap().clone();
        let stable = gpu.transfers();
        let same = prepare(&mut owner, &project, &mut ui, &ctx, &mut images, true).unwrap();
        assert_eq!(token.receipt(), same.receipt());
        assert_eq!(stable, gpu.transfers());
        let mut reference = TheRGBABuffer::new(TheDim::sized(143, 83));
        if mode == 2 {
            draw_isometric_minimap(&project.regions[0].map, &assets, &mut reference, frame.bbox);
        } else {
            let scene = reference_scene.scene_mut(reference_id).unwrap();
            // Independent construction of the original minimap transform.
            let scale = Vec2::new(143. / frame.bbox.z, 83. / frame.bbox.w);
            let center = Vec2::new(
                frame.bbox.x + frame.bbox.z / 2.,
                frame.bbox.y + frame.bbox.w / 2.,
            );
            let transform = Mat3::<f32>::translation_2d(Vec2::new(71.5, 41.5) - center * scale)
                * Mat3::new(scale.x, 0., 0., 0., scale.y, 0., 0., 0., 1.);
            scene.settings.backend_2d = if mode == 1 {
                rusterix::render_settings::RendererBackend::Compute
            } else {
                rusterix::render_settings::RendererBackend::Raster
            };
            scene.vm.set_active_vm(0);
            for layer in 1..scene.vm.vm_layer_count() {
                scene.vm.set_layer_enabled(layer, false);
            }
            scene.vm.execute(scenevm::Atom::SetRenderMode(
                scene.settings.scenevm_mode_2d(),
            ));
            scene.settings.apply_hour(12.);
            scene.settings.apply_2d(&mut scene.vm);
            scene.vm.execute(scenevm::Atom::SetGP0(Vec4::zero()));
            scene.vm.execute(scenevm::Atom::SetTransform2D(transform));
            scene.vm.execute(scenevm::Atom::SetAnimationCounter(0));
            scene.vm.execute(scenevm::Atom::SetBackground(Vec4::zero()));
            scene.vm.render_frame(reference.pixels_mut(), 143, 83);
        }
        let pixels = capture(
            gpu,
            owner
                .gpu
                .as_ref()
                .unwrap()
                .scenes
                .capture(owner.gpu.as_ref().unwrap().view)
                .unwrap()
                .ticket,
        );
        assert!(
            pixels.chunks_exact(4).any(|p| p[0] > 50 && p[0] != p[1]),
            "minimap mode {mode} must render populated source geometry"
        );
        let delta = pixels
            .iter()
            .zip(reference.pixels())
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        assert!(delta <= 1, "minimap mode {mode}: source max delta {delta}");
        eprintln!("Native minimap mode={mode}: source max delta={delta}");
        let saved = *MINIMAPBOX.read().unwrap();
        *MINIMAPBOX.write().unwrap() = frame.bbox;
        draw_camera_marker(
            &project.regions[0].map,
            Some(&project.regions[0]),
            &mut reference,
            &server,
        );
        *MINIMAPBOX.write().unwrap() = saved;
        // The composited oracle includes marker placement through fractional DPI.
        let mut paint = PaintList::default();
        paint.ops.push(PaintOp::DisplayCompositing(
            theframework::paint::DisplayCompositing::LegacyOpaque,
        ));
        paint.ops.push(PaintOp::SpriteImage {
            image: frame.image,
            origin: [0.; 2],
            scale: [1.; 2],
            tint: [1.; 4],
        });
        paint.ops.extend(frame.markers.ops.iter().cloned());
        for density in [1., 1.25, 1.5, 1.75, 2.] {
            let size = ((143. * density) as u32, (83. * density) as u32);
            let target =
                renderkit::GpuImage::new(gpu, renderkit::ImageDescriptor::rgba(size.0, size.1))
                    .unwrap();
            renderer
                .render(
                    &paint,
                    &images,
                    &target.view,
                    target.descriptor.format,
                    size,
                    density,
                    [0.; 4],
                )
                .unwrap();
            let actual = capture(gpu, ReadbackTicket::capture(gpu, target).unwrap());
            let mut delta = 0;
            for y in 0..size.1 {
                for x in 0..size.0 {
                    let a = ((y * size.0 + x) * 4) as usize;
                    let b = ((((y as f32 + 0.5) / density) as usize) * 143
                        + ((x as f32 + 0.5) / density) as usize)
                        * 4;
                    for c in 0..3 {
                        delta = delta.max(actual[a + c].abs_diff(reference.pixels()[b + c]));
                    }
                }
            }
            assert!(
                delta <= 1,
                "minimap mode {mode} DPI {density}: composed delta {delta}"
            );
        }
        project.regions[0].editing_position_3d.x += 0.25;
        prepare(&mut owner, &project, &mut ui, &ctx, &mut images, true).unwrap();
        assert!(
            owner
                .navigate(&view_id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                .is_ok()
        );
        assert_eq!(
            owner.current.as_ref().unwrap().output,
            frame.output,
            "marker movement must reuse the scene image"
        );
        let changed = project.regions[0].map.changed;
        project.regions[0].map.vertices[0].x += 0.125;
        assert_eq!(changed, project.regions[0].map.changed);
        assert!(
            owner
                .navigate(&view_id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                .is_err()
        );
        let replacement = prepare(&mut owner, &project, &mut ui, &ctx, &mut images, true).unwrap();
        owner.presented(Some(token.clone()));
        assert!(
            owner
                .navigate(&view_id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                .is_err()
        );
        owner.presented(Some(replacement));
        ctx.scale_factor = 1.5;
        assert!(
            owner
                .navigate(&view_id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                .is_err()
        );
        let resized = prepare(&mut owner, &project, &mut ui, &ctx, &mut images, true).unwrap();
        owner.presented(Some(resized));
        let mut document = ServerContext::new();
        document.pc = server.pc;
        document.curr_region = id;
        document.editor_view_mode = server.editor_view_mode;
        assert!(
            owner
                .navigate(&view_id, Vec2::zero(), &project, &document, &mut ui, &ctx)
                .is_err()
        );
        owner.device_reset(&mut ui);
        assert!(
            owner
                .navigate(&view_id, Vec2::zero(), &project, &server, &mut ui, &ctx)
                .is_err()
        );
        assert!(prepare(&mut owner, &project, &mut ui, &ctx, &mut images, true).is_some());
        let acknowledged = prepare(&mut owner, &project, &mut ui, &ctx, &mut images, true).unwrap();
        owner.presented(Some(acknowledged));
        let expected = owner
            .navigate(
                &view_id,
                Vec2::new(70, 45),
                &project,
                &server,
                &mut ui,
                &ctx,
            )
            .unwrap()
            .unwrap();
        sidebar.minimap_scene = owner;
        let _ = sidebar.handle_event(
            &TheEvent::RenderViewClicked(view_id.clone(), Vec2::new(70, 45)),
            &mut ui,
            &mut ctx,
            &mut project,
            &mut server,
        );
        let position = project.regions[0].editing_position_3d;
        assert!((Vec2::new(position.x, position.z) - expected).magnitude() < 0.00001);
        let registered = sidebar.minimap_scene.current.as_ref().unwrap().image;
        sidebar.minimap_scene.deactivate(&mut ui);
        sidebar.reset_for_project_switch();
        let mut owner = std::mem::take(&mut sidebar.minimap_scene);
        assert!(
            owner.gpu.is_some(),
            "reload retains registrations until registry access"
        );
        owner
            .prepare(
                &project,
                &server,
                &mut ui,
                &ctx,
                &assets,
                mode + 1,
                source.clone(),
                true,
                0,
                &mut images,
            )
            .unwrap();
        assert!(owner.gpu.is_none());
        assert!(
            images.get(registered).is_none(),
            "reload retires the old GPU alias"
        );
        reference_scene.clear(&mut images).unwrap();
    }
    check_screens(gpu, renderer);
}

fn check_screens(gpu: &Arc<GpuContext>, renderer: &mut PaintRenderer) {
    fn quad(map: &mut Map, min: [f32; 2], max: [f32; 2], tile: Uuid, clamp: bool, occlusion: f32) {
        let v = [[min[0], min[1]], [max[0], min[1]], max, [min[0], max[1]]]
            .map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        let s = map.find_sector_mut(id).unwrap();
        s.properties.set(
            "source",
            rusterix::Value::Source(rusterix::PixelSource::TileId(tile)),
        );
        s.properties
            .set("tile_mode", rusterix::Value::Int(if clamp { 0 } else { 1 }));
        s.properties
            .set("occlusion", rusterix::Value::Float(occlusion));
    }
    let mut tile = rusterix::Tile::default();
    tile.id = Uuid::new_v4();
    for frame in 0..4 {
        let (w, h) = (11 + 3 * frame, 7 + 2 * frame);
        let mut pixels = Vec::new();
        for y in 0..h {
            for x in 0..w {
                pixels.extend([
                    (x * 17 + frame * 23) as u8,
                    (y * 31 + frame * 47) as u8,
                    ((x + y) * 13 + frame * 17) as u8,
                    [64, 128, 255][(x + y) % 3],
                ]);
            }
        }
        tile.textures.push(rusterix::Texture::new(pixels, w, h));
    }
    let top = rusterix::Tile::from_texture(rusterix::Texture::new(
        [200, 70, 40, 180].repeat(3 * 5),
        3,
        5,
    ));
    let mut map = Map::new();
    quad(&mut map, [-2., -1.], [3., 2.], tile.id, false, 0.6);
    quad(&mut map, [-0.75, -0.25], [2., 1.25], top.id, true, 0.8);
    let mut project = Project::default();
    let mut screen = shared::screen::Screen::new();
    screen.map = map;
    let id = screen.id;
    let mut other_screen = screen.clone();
    other_screen.id = Uuid::new_v4();
    let other_id = other_screen.id;
    project.screens.insert(other_id, other_screen);
    project.screens.insert(id, screen);
    let mut server = ServerContext::new();
    server.pc = ProjectContext::Screen(id);
    server.editor_view_mode = EditorViewMode::D2;
    let mut assets = rusterix::Assets::default();
    for mode in 0..3 {
        tile.uv_pages = if mode == 0 {
            None
        } else {
            Some(rusterix::TileUvPages {
                animation_frames: 1,
                repeat_period: if mode == 2 { [0.75, 1.25] } else { [0.; 2] },
                origin: [0, 0],
                size: [2, 2],
                axes: [[0.; 3]; 2],
                units_per_page: 1,
            })
        };
        assets.set_tiles(
            [(tile.id, tile.clone()), (top.id, top.clone())]
                .into_iter()
                .collect(),
        );
        let saved = serde_json::to_value(&project).unwrap();
        for size in [Vec2::new(143, 83), Vec2::new(84, 144)] {
            let mut images = ImageRegistry::new(gpu.clone());
            let mut ctx = TheContext::new(size.x as usize, size.y as usize, 1.);
            ctx.ui.command_paint = true;
            let mut ui = TheUI::new();
            let mut view = TheRenderView::new(TheId::named("MiniMap"));
            let view_id = view.id().clone();
            view.set_dim(TheDim::new(13, 29, size.x, size.y), &mut ctx);
            view.render_buffer_mut().resize(size.x, size.y);
            ui.canvas.set_widget(view);
            let mut sidebar = crate::sidebar::Sidebar::new();
            assert!(sidebar.handle_event(
                &TheEvent::Custom(TheId::named("Soft Update Minimap"), TheValue::Empty),
                &mut ui,
                &mut ctx,
                &mut project,
                &mut server
            ));
            let mut owner = std::mem::take(&mut sidebar.minimap_scene);
            let source =
                Arc::new(rusterix::scene_build_index::SceneBuildIndex::default()).snapshot();
            let prepare = |owner: &mut SceneOverview,
                           project: &Project,
                           ui: &mut TheUI,
                           ctx: &TheContext,
                           images: &mut ImageRegistry| {
                owner
                    .prepare(
                        project,
                        &server,
                        ui,
                        ctx,
                        &assets,
                        100 + mode,
                        source.clone(),
                        false,
                        0,
                        images,
                    )
                    .unwrap()
                    .unwrap()
            };
            let before = gpu.transfers();
            let token = prepare(&mut owner, &project, &mut ui, &ctx, &mut images);
            assert_eq!(before.readback_bytes, gpu.transfers().readback_bytes);
            assert!(
                !owner.waiting(),
                "screen overviews do not wait for the unrelated region chunk worker"
            );
            assert!(
                !ui.get_render_view("MiniMap")
                    .unwrap()
                    .has_cpu_display_storage()
            );
            assert!(
                owner
                    .navigate(
                        &view_id,
                        Vec2::new(70, 42),
                        &project,
                        &server,
                        &mut ui,
                        &ctx
                    )
                    .is_err()
            );
            owner.presented(Some(token.clone()));
            let expected = owner
                .navigate(
                    &view_id,
                    Vec2::new(70, 42),
                    &project,
                    &server,
                    &mut ui,
                    &ctx,
                )
                .unwrap()
                .unwrap();
            let frame = owner.current.as_ref().unwrap().clone();
            let mut other = ServerContext::new();
            other.document_instance = server.document_instance;
            other.pc = ProjectContext::Screen(other_id);
            other.editor_view_mode = EditorViewMode::D2;
            assert!(
                owner
                    .navigate(&view_id, Vec2::new(70, 42), &project, &other, &mut ui, &ctx)
                    .is_err(),
                "a second screen cannot reuse a presented map ID from another screen context"
            );
            let stable = gpu.transfers();
            let repeated = prepare(&mut owner, &project, &mut ui, &ctx, &mut images);
            assert_eq!(repeated.receipt(), token.receipt());
            assert_eq!(stable, gpu.transfers());
            let original = &project.screens[&id].map;
            let original_offset = original.offset;
            let mut map = original.clone();
            map.selected_linedefs.clear();
            map.selected_sectors.clear();
            let bounds = frame.bbox;
            let scale = Vec2::new(size.x as f32 / bounds.z, size.y as f32 / bounds.w);
            map.grid_size = scale.x.min(scale.y);
            map.camera = MapCamera::TwoD;
            map.offset = Vec2::new(
                -(bounds.x + bounds.z * 0.5) * scale.x,
                (bounds.y + bounds.w * 0.5) * scale.y,
            );
            let transform = Mat3::<f32>::translation_2d(Vec2::new(
                map.offset.x + size.x as f32 / 2.,
                -map.offset.y + size.y as f32 / 2.,
            )) * Mat3::new(scale.x, 0., 0., 0., scale.y, 0., 0., 0., 1.);
            let mut scene = rusterix::D2PreviewBuilder::new().build(
                &map,
                &assets,
                Vec2::new(size.x as f32, size.y as f32),
                &Default::default(),
            );
            let mut reference = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
            rusterix::Rasterizer::setup(Some(transform), Mat4::identity(), Mat4::identity())
                .background([42, 42, 42, 255])
                .ambient(Vec4::one())
                .render_mode(rusterix::RenderMode::render_2d())
                .rasterize(
                    &mut scene,
                    reference.pixels_mut(),
                    size.x as usize,
                    size.y as usize,
                    40,
                    &assets,
                );
            let actual = capture(
                gpu,
                ReadbackTicket::capture(gpu, images.get(frame.output.image).unwrap()).unwrap(),
            );
            let delta = actual
                .iter()
                .zip(reference.pixels())
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            if delta > 1 {
                std::fs::write("/private/tmp/eldiron-overview2d-gpu.rgba", &actual).unwrap();
                std::fs::write(
                    "/private/tmp/eldiron-overview2d-reference.rgba",
                    reference.pixels(),
                )
                .unwrap();
                let differences=actual.chunks_exact(4).zip(reference.pixels().chunks_exact(4)).enumerate().filter(|(_, (a,b))|a.iter().zip(b.iter()).any(|(a,b)|a.abs_diff(*b)>1)).take(8)
                    .map(|(i,(a,b))|serde_json::json!({"at":[i%size.x as usize,i/size.x as usize],"actual":a,"expected":b})).collect::<Vec<_>>();
                eprintln!(
                    "Screen overview mismatch: {}",
                    serde_json::json!({"mode":mode,"size":[size.x,size.y],"delta":delta,"differences":differences})
                );
            }
            assert!(
                delta <= 1,
                "screen overview mode={mode} size={size:?}: source delta={delta}"
            );
            eprintln!("Native screen overview mode={mode} size={size:?}: max delta={delta}");
            if mode == 1 && size.x == 143 {
                // The generic minimap intentionally has no occlusion. Exercise
                // the producer's explicit first-match policy independently.
                let regions = scene.mapmini.occlusion_regions().to_vec();
                assert!(!regions.is_empty());
                let mut native =
                    eldiron_presentation::overview2d::Overview2DRenderer::new(gpu.clone()).unwrap();
                let image = native
                    .render(
                        &scene,
                        &assets,
                        (size.x as u32, size.y as u32),
                        transform,
                        server.document_instance,
                        assets.tiles_revision,
                        &regions,
                    )
                    .unwrap();
                let mut expected = TheRGBABuffer::new(TheDim::sized(size.x, size.y));
                let mut raster = rusterix::Rasterizer::setup(
                    Some(transform),
                    Mat4::identity(),
                    Mat4::identity(),
                )
                .background([42, 42, 42, 255])
                .ambient(Vec4::one())
                .render_mode(rusterix::RenderMode::render_2d());
                raster.mapmini = scene.mapmini.clone();
                raster.rasterize(
                    &mut scene,
                    expected.pixels_mut(),
                    size.x as usize,
                    size.y as usize,
                    40,
                    &assets,
                );
                assert_ne!(
                    expected.pixels(),
                    reference.pixels(),
                    "explicit occlusion changes actual image colors"
                );
                let actual = capture(gpu, ReadbackTicket::capture(gpu, image).unwrap());
                let delta = actual
                    .iter()
                    .zip(expected.pixels())
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap();
                assert!(
                    delta <= 1,
                    "explicit ordered overview occlusion delta={delta}"
                );
                assert!(native.source_resident_bytes() <= 64 * 1024 * 1024);
                eprintln!("Native screen explicit occlusion: max delta={delta}");
            }

            let old = *MINIMAPBOX.read().unwrap();
            *MINIMAPBOX.write().unwrap() = bounds;
            draw_camera_marker(original, None, &mut reference, &server);
            *MINIMAPBOX.write().unwrap() = old;
            let mut paint = PaintList::default();
            paint.ops.push(PaintOp::DisplayCompositing(
                theframework::paint::DisplayCompositing::LegacyOpaque,
            ));
            paint.ops.push(PaintOp::SpriteImage {
                image: frame.image,
                origin: [0.; 2],
                scale: [1.; 2],
                tint: [1.; 4],
            });
            paint.ops.extend(frame.markers.ops.iter().cloned());
            for density in [1., 1.25, 1.5, 1.75, 2.] {
                let extent = (
                    (size.x as f32 * density).round() as u32,
                    (size.y as f32 * density).round() as u32,
                );
                let image = renderkit::GpuImage::new(
                    gpu,
                    renderkit::ImageDescriptor::rgba(extent.0, extent.1),
                )
                .unwrap();
                renderer
                    .render(
                        &paint,
                        &images,
                        &image.view,
                        image.descriptor.format,
                        extent,
                        density,
                        [0.; 4],
                    )
                    .unwrap();
                let pixels = capture(gpu, ReadbackTicket::capture(gpu, image).unwrap());
                let mut delta = 0;
                for y in 0..extent.1 {
                    for x in 0..extent.0 {
                        let sx = ((x as f32 + 0.5) / density).floor() as i32;
                        let sy = ((y as f32 + 0.5) / density).floor() as i32;
                        let expected = reference.get_pixel(sx, sy).unwrap_or([0, 0, 0, 255]);
                        let at = ((y * extent.0 + x) * 4) as usize;
                        for c in 0..4 {
                            delta = delta.max(pixels[at + c].abs_diff(expected[c]));
                        }
                    }
                }
                assert!(
                    delta <= 1,
                    "screen overview mode={mode} DPI={density} delta={delta}"
                );
            }
            sidebar.minimap_scene = owner;
            sidebar.handle_event(
                &TheEvent::RenderViewClicked(view_id.clone(), Vec2::new(70, 42)),
                &mut ui,
                &mut ctx,
                &mut project,
                &mut server,
            );
            assert!(
                (editor_view_center(&project.screens[&id].map) - expected).magnitude() < 0.0001,
                "actual screen navigation uses presented screen bounds"
            );
            // Restore this camera-only mutation before the saved-content assertion.
            project.screens.get_mut(&id).unwrap().map.offset = original_offset;
            let mut owner = std::mem::take(&mut sidebar.minimap_scene);
            owner.deactivate(&mut ui);
            owner
                .prepare(
                    &project,
                    &server,
                    &mut ui,
                    &ctx,
                    &assets,
                    100 + mode,
                    source.clone(),
                    false,
                    0,
                    &mut images,
                )
                .unwrap();
            assert!(images.get(frame.output.image).is_none());
            let current = server.pc;
            server.pc = ProjectContext::Screen(Uuid::new_v4());
            sidebar.handle_event(
                &TheEvent::Custom(TheId::named("Soft Update Minimap"), TheValue::Empty),
                &mut ui,
                &mut ctx,
                &mut project,
                &mut server,
            );
            assert!(
                !ui.get_render_view("MiniMap")
                    .unwrap()
                    .has_cpu_display_storage(),
                "deleting an authored screen cannot opt its minimap back into a bitmap adapter"
            );
            server.pc = current;
        }
        assert_eq!(saved, serde_json::to_value(&project).unwrap());
    }
}
