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
}
