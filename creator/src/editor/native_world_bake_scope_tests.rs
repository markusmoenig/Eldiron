//! Publication-owner contract. Drive the real CPU controller to a completed
//! asset without preparing a displayed scene; GPU tile production has its own
//! channel/loss oracle and assembled application coverage.
use super::*;

#[test]
#[ignore = "native GPU context injection; no world framebuffer or capture"]
fn frozen_bake_scope_survives_missing_layout_and_rejects_retired_completed_sources() {
    let gpu = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    for phase in 0..6 {
        let mut editor = Editor::new();
        let region = editor.documents.project.regions[0].id;
        let map = editor.documents.project.regions[0].map.id;
        editor.documents.project.regions[0].map.camera = MapCamera::ThreeDIso;
        editor.server_ctx.shared.curr_region = region;
        editor.server_ctx.shared.pc = ProjectContext::Region(region);
        editor.server_ctx.shared.editor_view_mode = EditorViewMode::Iso;
        editor.native_viewport.enabled = true;
        editor.server_ctx.reference.bind_context(gpu.clone());
        let mut ui = TheUI::new();
        let mut ctx = TheContext::new(256, 256, 1.);
        let mut view = TheRenderView::new(TheId::named("PolyView"));
        view.set_dim(TheDim::sized(128, 128), &mut ctx);
        ui.canvas.set_widget(view);
        editor.server_ctx.bake.write().request_render(map, 1);
        editor.update_native_bake(&mut ui, &mut ctx);
        let job = editor
            .native_viewport
            .world
            .bake_jobs
            .active
            .as_ref()
            .unwrap();
        let camera = job.frame.camera;
        assert_eq!(job.viewport, (128, 128));
        assert!(job.gpu.is_none());
        let revision = editor.server_ctx.history.read().revision();
        // Simulate the producer/controller handoff immediately before its next
        // update publication. No shader or requested GPU capture occurs here.
        let mut bake = editor.server_ctx.bake.write();
        let work = bake
            .prepare_work(map, 128, 128, camera, Some((-0.25, 0.25, -0.25, 0.25)))
            .unwrap()
            .unwrap();
        assert!(bake.finish_work_sample(&work).unwrap());
        let pixels = (work.tile_size * work.tile_size) as usize;
        assert!(
            bake.commit_work(
                &work,
                vec![180; pixels * 4],
                vec![0.; pixels],
                vec![100; pixels * 4],
                vec![128; pixels * 4],
                vec![0; pixels * 4]
            )
            .unwrap()
        );
        assert!(!bake.is_rendering());
        drop(bake);
        assert!(
            editor.documents.project.regions[0]
                .map
                .orthographic_bake
                .is_none()
        );
        if phase == 1 {
            // A widget may disappear entirely while its owning document lives.
            ui = TheUI::new();
        } else {
            ui.get_widget("PolyView")
                .unwrap()
                .set_dim(TheDim::sized(0, 0), &mut ctx);
        }
        match phase {
            2 => editor.documents.project.regions[0].map.changed += 1,
            3 => editor.documents.project.regions.clear(),
            4 => editor
                .documents
                .install(editor.documents.project.clone(), None, false),
            5 => editor.server_ctx.shared.editor_view_mode = EditorViewMode::D2,
            _ => (),
        }
        editor.update_native_bake(&mut ui, &mut ctx);
        assert!(editor.native_viewport.world.bake_jobs.active.is_none());
        assert!(
            editor
                .server_ctx
                .bake
                .write()
                .take_persisted_update_for(map)
                .is_none(),
            "a completion must be either published or retired under its owner"
        );
        if phase < 2 {
            assert!(
                editor.documents.project.regions[0]
                    .map
                    .orthographic_bake
                    .is_some(),
                "layout availability cannot discard valid asset publication"
            );
            assert!(editor.server_ctx.history.read().revision() > revision);
            assert!(editor.documents.sessions[editor.documents.active_session].dirty);
        } else if let Some(region) = editor.documents.project.regions.first() {
            assert!(
                region.map.orthographic_bake.is_none(),
                "retired source cannot enter authored data"
            );
            assert_eq!(editor.server_ctx.history.read().revision(), revision);
        }
    }
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert_eq!(gpu.capture_queue_usage().requests, 0);
}

/// Exercise the actual private producer while no world widget is drawable.
/// Linked paint is the only color source; changing its catalog revision alone
/// must retire the next pending capture without replacing the completed asset.
#[test]
#[ignore = "actual GPU original-channel generation and pending capture retirement"]
fn hidden_world_bakes_linked_prefab_paint_and_retires_catalog_edits() {
    run_hidden_linked_paint(false, false, false);
}

#[test]
#[ignore = "actual GPU canonical source above former UV input limit"]
fn hidden_world_bakes_large_linked_paint_without_source_size_adapter() {
    run_hidden_linked_paint(true, false, false);
}

#[test]
#[ignore = "actual GPU replacement independent of oversized saved original tile"]
fn hidden_world_rebakes_over_an_oversized_saved_original_without_decoding_it() {
    run_hidden_linked_paint(false, true, false);
}

#[test]
#[ignore = "actual GPU cooperative stamp preparation above former count guard"]
fn hidden_world_bakes_many_original_stamps_with_cooperative_shared_sprites() {
    run_hidden_linked_paint(false, false, true);
}

fn run_hidden_linked_paint(large: bool, oversized: bool, stamps: bool) {
    use shared::iso_paint::{IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner};
    use std::time::{Duration, Instant};
    let gpu = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut editor = Editor::new();
    let floor = rusterix::GeometryObject::box_from_bounds(
        "Linked painted floor",
        Vec3::new(-0.75, -0.25, -0.75),
        Vec3::new(0.75, 0., 0.75),
    );
    let mut paint = IsoPaintLayer::default();
    for face in &floor.faces {
        let id = rusterix::geometry_face_effective_paint_surface_id(face).as_u128()
            ^ floor.id.as_u128().rotate_left(1);
        let geo = [
            id as u32,
            (id >> 32) as u32,
            (id >> 64) as u32,
            (id >> 96) as u32,
        ];
        for y in -2..=2 {
            for x in -2..=2 {
                let mut chunk = IsoPaintBakedChunk::new(
                    IsoPaintOwner::GeometryObject(floor.id),
                    geo,
                    [x * 64, y * 64],
                );
                chunk.color_rgba = [24, 190, 72, 255].repeat(chunk.color_rgba.len() / 4).into();
                chunk.revision = 1;
                paint.baked_chunks.insert(format!("{id}:{x}:{y}"), chunk);
            }
        }
    }
    if large {
        let prototype = paint.baked_chunks.values().next().unwrap().clone();
        while paint.baked_chunks.len() < 2051 {
            let number = paint.baked_chunks.len();
            let mut chunk = prototype.clone();
            chunk.origin = [number as i32 * 64, 8192];
            // Distinct original planes: a large logical source cannot hide an
            // accidental whole-source allocation behind shared fixture aliases.
            chunk.color_rgba[..4].copy_from_slice(&[number as u8, 190, 72, 255]);
            chunk.material_rgba[1] = number as u8;
            paint.baked_chunks.insert(format!("unused:{number}"), chunk);
        }
    }
    let asset = rusterix::BlockPropAsset::new_authored("Painted linked source", vec![floor]);
    let asset_id = asset.id;
    editor.documents.project.block_props.insert(asset_id, asset);
    editor
        .documents
        .project
        .block_prop_paint
        .insert(asset_id, paint);
    let region = &mut editor.documents.project.regions[0];
    let region_id = region.id;
    region.map = Map::new();
    region.map.camera = MapCamera::ThreeDIso;
    region.iso_paint.visible = false;
    region.editing_position_iso_3d = Some(Vec3::zero());
    region.editing_iso_scale = Some(3.);
    for x in [-0.9, 0.9] {
        let mut instance = rusterix::BlockPropInstance::new(asset_id);
        instance.world_transform[3][0] = x;
        if x > 0. {
            // Rotate the second instance without changing its source-local UVs.
            instance.world_transform[0] = [0., 0., -1., 0.];
            instance.world_transform[2] = [1., 0., 0., 0.];
        }
        region.map.block_prop_instances.push(instance);
    }
    if oversized {
        let asset: rusterix::map::OrthographicBakeAsset = serde_json::from_str(include_str!(
            "../../tests/fixtures/oversized-current-bake.json"
        ))
        .unwrap();
        assert!(bake::supported(&asset));
        region.map.orthographic_bake = Some(asset);
        editor
            .server_ctx
            .bake
            .write()
            .sync_persisted_asset(region.map.id, region.map.orthographic_bake.as_ref());
        assert_eq!(editor.server_ctx.bake.read().decoded_resident_bytes(), 0);
    }
    if stamps {
        use shared::iso_paint::IsoPaintPoint;
        region.iso_paint.visible = true;
        region.iso_paint.active_brush = "grass".into();
        region.iso_paint.active_size = 1.;
        region.iso_paint.active_color = [24, 190, 72, 255];
        region.iso_paint.add_stamp(IsoPaintPoint::new(
            [8, 8],
            Some(Vec3::new(0., 0.1, 0.)),
            None,
        ));
        let chunk = region.iso_paint.chunks.values_mut().next().unwrap();
        let prototype = chunk.stamps[0].clone();
        chunk.stamps = (0..300)
            .map(|order| {
                let mut stamp = prototype.clone();
                stamp.id = Uuid::new_v4();
                stamp.order = order;
                stamp
            })
            .collect();
    }
    let authored_region_paint = region.iso_paint.clone();
    let map_id = region.map.id;
    let map_changed = region.map.changed;
    let authored_paint = editor.documents.project.block_prop_paint.clone();
    editor.server_ctx.shared.curr_region = region_id;
    editor.server_ctx.shared.pc = ProjectContext::Region(region_id);
    editor.server_ctx.shared.editor_view_mode = EditorViewMode::Iso;
    editor.server_ctx.camera.iso_camera.scale = 3.;
    editor.native_viewport.enabled = true;
    editor.server_ctx.reference.bind_context(gpu.clone());
    editor
        .server_ctx
        .reference
        .write()
        .assets
        .set_block_props(editor.documents.project.block_props.clone());
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(256, 256, 1.);
    let mut view = TheRenderView::new(TheId::named("PolyView"));
    view.set_dim(TheDim::sized(128, 128), &mut ctx);
    ui.canvas.set_widget(view);
    let history = editor.server_ctx.history.read().revision();
    editor.server_ctx.bake.write().request_render(map_id, 2);
    editor.update_native_bake(&mut ui, &mut ctx);
    let camera = editor
        .native_viewport
        .world
        .bake_jobs
        .active
        .as_ref()
        .unwrap()
        .frame
        .camera;
    ui.get_widget("PolyView")
        .unwrap()
        .set_dim(TheDim::sized(0, 0), &mut ctx);
    if large || stamps {
        let before_preparation = gpu.transfers().asset_bake_readback_requests;
        for _ in 0..10 {
            editor.update_native_bake(&mut ui, &mut ctx);
            editor.prepare_native_bake(&mut ctx, &images);
        }
        let job = editor
            .native_viewport
            .world
            .bake_jobs
            .active
            .as_ref()
            .unwrap();
        assert!(job.source.ready());
        assert!(job.gpu.is_none());
        assert!(!job.paint.as_ref().unwrap().ready());
        if stamps {
            assert!(
                job.prepared_scene.is_some(),
                "stamp preparation must own its private scene"
            );
        }
        // A catalog edit during original index/stamp preparation must retire
        // that frozen source and private scene like a pending GPU sample.
        let chunk = editor
            .documents
            .project
            .block_prop_paint
            .get_mut(&asset_id)
            .unwrap()
            .baked_chunks
            .values_mut()
            .last()
            .unwrap();
        chunk.revision += 1;
        editor.update_native_bake(&mut ui, &mut ctx);
        assert!(editor.native_bake_paint_retired_for_test(&None));
        assert_eq!(
            gpu.transfers().asset_bake_readback_requests,
            before_preparation
        );
        // Restore the fixture input, then issue a new, independently frozen job.
        editor
            .documents
            .project
            .block_prop_paint
            .get_mut(&asset_id)
            .unwrap()
            .baked_chunks
            .values_mut()
            .last()
            .unwrap()
            .revision -= 1;
        ui.get_widget("PolyView")
            .unwrap()
            .set_dim(TheDim::sized(128, 128), &mut ctx);
        editor.server_ctx.bake.write().request_render(map_id, 2);
        editor.update_native_bake(&mut ui, &mut ctx);
        ui.get_widget("PolyView")
            .unwrap()
            .set_dim(TheDim::sized(0, 0), &mut ctx);
    }
    let before = gpu.transfers();
    let mut preparation_yields = 0;
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        editor.update_native_bake(&mut ui, &mut ctx);
        editor.prepare_native_bake(&mut ctx, &images);
        if let Some(job) = &editor.native_viewport.world.bake_jobs.active {
            assert!(editor.native_viewport.world.bake_jobs.legacy.is_none());
            if job.source.ready() && job.gpu.is_none() {
                preparation_yields += 1;
                assert_eq!(
                    gpu.transfers().asset_bake_readback_requests,
                    before.asset_bake_readback_requests
                );
            }
        }
        gpu.device.poll(wgpu::PollType::Poll).unwrap();
        if !editor.server_ctx.bake.read().is_rendering()
            && editor.native_viewport.world.bake_jobs.active.is_none()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "hidden producer failed: {:?}",
            editor.server_ctx.bake.read().status()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    if large || stamps {
        assert!(
            preparation_yields > 128,
            "large material preparation must yield"
        );
        eprintln!(
            "Canonical paint: large_uv={large}, stamps={stamps}, {preparation_yields} preparation yields, no adapter"
        );
    }
    let old = editor.documents.project.regions[0]
        .map
        .orthographic_bake
        .clone();
    let asset = old
        .as_ref()
        .expect("hidden completion must enter authored history");
    assert_eq!(asset.samples, 2);
    assert_eq!(asset.tile_size, 256);
    if oversized {
        eprintln!(
            "Replaced valid 2048px saved tile with native 256px original production; previous source was never decoded"
        );
    }
    assert!(editor.server_ctx.history.read().revision() > history);
    assert!(editor.documents.sessions[editor.documents.active_session].dirty);
    assert_eq!(
        editor.documents.project.regions[0].map.changed,
        map_changed + 1
    );
    assert_eq!(editor.documents.project.block_prop_paint, authored_paint);
    assert_eq!(
        editor.documents.project.regions[0].iso_paint,
        authored_region_paint
    );
    let bake = editor.server_ctx.bake.read();
    let source = bake.presentation_source(map_id, &camera).unwrap();
    let green: usize = source
        .tiles()
        .map(|(_, tile)| {
            assert!(tile.depth.iter().any(|d| d.is_finite()));
            tile.albedo
                .chunks_exact(4)
                .filter(|p| p[3] > 0 && p[1] > p[0].saturating_add(25) && p[1] > p[2])
                .count()
        })
        .sum();
    assert!(
        green > 100,
        "linked source paint must reach actual saved albedo: {green}"
    );
    eprintln!(
        "Hidden linked-Prefab bake: {} tiles, {green} green albedo pixels",
        asset.tiles.len()
    );
    drop(bake);
    assert!(gpu.transfers().asset_bake_readback_requests > before.asset_bake_readback_requests);
    assert_eq!(
        gpu.transfers().readback_bytes - before.readback_bytes,
        gpu.transfers().asset_bake_readback_bytes - before.asset_bake_readback_bytes
    );
    assert!(
        editor.native_viewport.world.gpu.is_none(),
        "a hidden bake cannot create a displayed world scene"
    );
    // A new request gets sampling dimensions once, then loses its entire widget.
    ui.get_widget("PolyView")
        .unwrap()
        .set_dim(TheDim::sized(128, 128), &mut ctx);
    editor.server_ctx.bake.write().request_render(map_id, 2);
    editor.update_native_bake(&mut ui, &mut ctx);
    ui = TheUI::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        editor.update_native_bake(&mut ui, &mut ctx);
        editor.prepare_native_bake(&mut ctx, &images);
        if editor.native_bake_paint_pending_for_test() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "missing-widget capture never became pending"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    let changed = editor.documents.project.regions[0].map.changed;
    let history = editor.server_ctx.history.read().revision();
    let chunk = editor
        .documents
        .project
        .block_prop_paint
        .get_mut(&asset_id)
        .unwrap()
        .baked_chunks
        .values_mut()
        .next()
        .unwrap();
    chunk.color_rgba[..4].copy_from_slice(&[220, 40, 30, 255]);
    chunk.revision += 1;
    editor.update_native_bake(&mut ui, &mut ctx);
    assert!(editor.native_bake_paint_retired_for_test(&old));
    assert_eq!(editor.documents.project.regions[0].map.changed, changed);
    assert_eq!(editor.server_ctx.history.read().revision(), history);
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    assert_eq!(gpu.capture_queue_usage().requests, 0);
}

#[test]
#[ignore = "native GPU context; virtual stamp raster admission and cancellation"]
fn oversized_stamp_raster_enters_native_owner_without_partial_publication() {
    use shared::iso_paint::IsoPaintPoint;
    let gpu = futures::executor::block_on(renderkit::GpuContext::headless()).unwrap();
    let images = ImageRegistry::new(gpu.clone());
    let mut editor = Editor::new();
    let region = &mut editor.documents.project.regions[0];
    let id = region.id;
    let map = region.map.id;
    region.map.camera = MapCamera::ThreeDIso;
    region.iso_paint.active_brush = "grass".into();
    region.iso_paint.active_size = 1000.;
    region
        .iso_paint
        .add_stamp(IsoPaintPoint::new([8, 8], Some(Vec3::zero()), None));
    let authored = region.iso_paint.clone();
    editor.server_ctx.shared.curr_region = id;
    editor.server_ctx.shared.pc = ProjectContext::Region(id);
    editor.server_ctx.shared.editor_view_mode = EditorViewMode::Iso;
    editor.native_viewport.enabled = true;
    editor.server_ctx.reference.bind_context(gpu.clone());
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(256, 256, 1.);
    let mut view = TheRenderView::new(TheId::named("PolyView"));
    view.set_dim(TheDim::sized(128, 128), &mut ctx);
    ui.canvas.set_widget(view);
    editor.server_ctx.bake.write().request_render(map, 2);
    let request = editor.server_ctx.bake.read().active_request();
    let before = gpu.transfers();
    for _ in 0..128 {
        editor.update_native_bake(&mut ui, &mut ctx);
        editor.prepare_native_bake(&mut ctx, &images);
        if editor.native_viewport.world.bake_jobs.active.as_ref().is_some_and(|job| job.gpu.is_some()) {
            break;
        }
    }
    assert!(editor.native_viewport.world.bake_jobs.active.as_ref().is_some_and(|job| job.gpu.is_some()));
    assert!(!editor.native_viewport.world.bake_jobs.uses_adapter(editor.documents.active_id(), request));
    assert_eq!(editor.server_ctx.bake.read().active_request(), request);
    assert_eq!(gpu.transfers().readback_bytes, before.readback_bytes);
    assert_eq!(editor.documents.project.regions[0].iso_paint, authored);
    assert!(
        editor.documents.project.regions[0]
            .map
            .orthographic_bake
            .is_none()
    );
    assert!(
        editor
            .server_ctx
            .bake
            .write()
            .take_persisted_update_for(map)
            .is_none()
    );
    editor.cancel_native_bake_job();
}
