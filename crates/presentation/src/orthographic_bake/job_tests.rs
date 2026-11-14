use super::*;
use rusterix::editor_view::EditorViewFrame;
use rusterix::{
    Assets, D3Camera, D3IsoCamera, Map, MapCamera, OrthographicBakeController, Rusterix,
    SceneHandler, Texture,
};
use scenevm::{
    Atom, Chunk, GeoId, Poly3D, SceneVM, SharedAtlas,
    prelude::{Vec2, Vec3},
};

fn scene(context: &Arc<GpuContext>, assets: &Assets, chunk: &Chunk) -> SceneHandler {
    let vm = SceneVM::new_with_context_and_atlas(context.clone(), 1, 1, SharedAtlas::new(256, 256));
    let mut scene = SceneHandler::with_vm(vm);
    scene.sync_base_render_settings(&assets.config);
    scene.build_atlas(&assets.tiles, true);
    scene.vm.set_background_lighting(false);
    scene.vm.execute(Atom::AddChunk {
        id: Uuid::new_v4(),
        chunk: chunk.clone(),
    });
    scene
}
fn finish(context: &GpuContext) {
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .unwrap();
}
#[test]
#[ignore = "native GPU; unloaded authored paint must withhold original samples"]
fn missing_paint_dependencies_never_publish_or_capture_an_original_sample() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut assets = Assets::default();
    let mut texture = Texture::new(vec![210, 60, 40, 255], 1, 1);
    texture.data_ext = Some(vec![0x08, 0x0f, 128, 128]);
    let tile = rusterix::Tile::from_texture(texture);
    let tile_id = tile.id;
    assets.set_tiles([(tile_id, tile)].into_iter().collect());
    let geo = [7u32, 0, 0, 1];
    let mut poly = Poly3D::cube(GeoId::Sector(1), tile_id, Vec3::zero(), 1.2);
    poly.paint_surface_id = Some(geo);
    poly.paint_uvs = vec![[0.5; 2]; poly.vertices.len()];
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(poly);
    let mut map = Map::new();
    map.camera = MapCamera::ThreeDIso;
    let mut camera = D3IsoCamera::new();
    camera.scale = 1.;
    camera.distance = 4.;
    camera.height_clearance = 0.;
    camera.azimuth_deg = 0.;
    camera.elevation_deg = 0.;
    let camera_frame = camera.as_scenevm_camera_for_surface(256., 256.);
    let mut reference = Rusterix::with_scene_handler(scene(&context, &assets, &chunk));
    reference.assets = assets.clone();
    reference.client.camera_d3 = Box::new(camera.clone());
    let EditorViewFrame::D3(frame) =
        reference
            .client
            .editor_view_3d(&map, (256, 256), true, None, true, false)
    else {
        unreachable!()
    };
    let mut prepared = scene(&context, &assets, &chunk);
    let empty = scenevm::Raster3DSurfacePaintEntry {
        geo: [0; 4],
        uv_origin: [0; 2],
        uv_size: [0; 2],
        atlas_rect: [0; 4],
    };
    let mut entries = vec![empty; 2];
    let slot = geo
        .into_iter()
        .chain([0, 0])
        .fold(2166136261u32, |h, value| (h ^ value).wrapping_mul(16777619)) as usize
        & 1;
    entries[slot] = scenevm::Raster3DSurfacePaintEntry {
        geo,
        uv_origin: [0; 2],
        uv_size: [64; 2],
        atlas_rect: [0; 4],
    };
    prepared
        .vm
        .set_raster3d_surface_paint(1, 1, vec![0; 4], vec![0; 4], entries, Vec::new());
    let mut job = OrthographicBakeJob::new(context.clone(), prepared, frame.clone()).unwrap();
    let mut bake = OrthographicBakeController::default();
    bake.request_render(map.id, 2);
    let before = context.transfers();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let error = loop {
        match job.advance(&mut bake, map.id, camera_frame, (256, 256)) {
            Ok(BakeAdvance::Pending) => finish(&context),
            Err(error) => break error,
            other => panic!("unresolved paint cannot advance: {other:?}"),
        }
        assert!(std::time::Instant::now() < deadline);
    };
    assert!(
        error
            .to_string()
            .contains(&format!("paint entry {slot} is not resident"))
    );
    let after = context.transfers();
    assert_eq!(
        after.status_readback_bytes - before.status_readback_bytes,
        4
    );
    assert_eq!(after.readback_bytes, before.readback_bytes);
    assert_eq!(after.capture_requests, before.capture_requests);
    assert!(bake.presentation_source(map.id, &camera_frame).is_none());
    assert!(bake.take_persisted_update_for(map.id).is_none());
    assert_eq!(
        job.advance(&mut bake, map.id, camera_frame, (256, 256))
            .unwrap(),
        BakeAdvance::Retired
    );

    // Tight primary bounds are fully covered by the cube. The surrounding
    // painted box is hidden from every primary ray, but catches indirect rays.
    let mut indirect_chunk = chunk.clone();
    for poly in indirect_chunk.polys3d_map.values_mut().flatten() {
        poly.paint_surface_id = Some([8, 0, 0, 1]);
    }
    let mut wall = Poly3D::cube(GeoId::Sector(2), tile_id, Vec3::zero(), 20.);
    wall.paint_surface_id = Some(geo);
    wall.paint_uvs = vec![[0.5; 2]; wall.vertices.len()];
    indirect_chunk.add_3d(wall);
    let mut prepared = scene(&context, &assets, &indirect_chunk);
    let mut entries = vec![empty; 2];
    entries[slot] = scenevm::Raster3DSurfacePaintEntry {
        geo,
        uv_origin: [0; 2],
        uv_size: [64; 2],
        atlas_rect: [0; 4],
    };
    prepared.vm.set_raster3d_surface_paint(
        1,
        1,
        vec![0; 4],
        vec![0; 4],
        entries.clone(),
        Vec::new(),
    );
    let mut tight_camera = camera_frame;
    tight_camera.ortho_half_h = 0.5;
    // With indirect transport disabled, the same geometry/camera must visit no
    // unloaded key. This makes the subsequent miss an indirect-only witness.
    prepared
        .vm
        .execute(Atom::SetRenderMode(scenevm::RenderMode::Compute3D));
    prepared.vm.execute(Atom::SetCamera3D {
        camera: tight_camera,
    });
    prepared.vm.set_layer_compute3d_bake_output(0, 1);
    let mut primary_target =
        scenevm::render_output::SceneRenderTarget::new(context.clone(), 256, 256).unwrap();
    prepared.vm.render_gpu(&mut primary_target, 1).unwrap();
    let mut primary_status = prepared
        .vm
        .vm
        .capture_paint_page_status(&context)
        .unwrap()
        .unwrap();
    finish(&context);
    assert_eq!(
        primary_status.try_finish(&context).unwrap(),
        Some(scenevm::paint_feedback::PaintPageStatus::Ready),
        "painted enclosure must be hidden from primary rays"
    );
    let mut witness = renderkit::ReadbackTicket::capture_asset(
        &context,
        prepared
            .vm
            .layer_output(&context, 0, 1)
            .unwrap()
            .unwrap()
            .image,
    )
    .unwrap();
    finish(&context);
    let witness = witness.try_finish(&context).unwrap().unwrap();
    assert!(
        witness
            .pixels()
            .chunks_exact(4)
            .all(|pixel| pixel[3] == 255),
        "primary receiver must cover the whole dispatch"
    );
    prepared.vm.set_layer_compute3d_bake_output(0, 0);
    prepared.vm.set_layer_progressive_sample_index(0, Some(0));
    prepared.vm.render_gpu(&mut primary_target, 2).unwrap();
    let mut raw_status = prepared
        .vm
        .vm
        .capture_paint_page_status(&context)
        .unwrap()
        .unwrap();
    finish(&context);
    assert_eq!(
        raw_status.try_finish(&context).unwrap(),
        Some(scenevm::paint_feedback::PaintPageStatus::MissingEntry(
            slot as u32
        )),
        "direct progressive dispatch must sample enclosure"
    );
    // The same auxiliary allocation must reset the fault on the next pass.
    let mut outside_camera = tight_camera;
    outside_camera.pos = Vec3::new(30., 0., 0.);
    outside_camera.forward = Vec3::unit_x();
    prepared.vm.execute(Atom::SetCamera3D {
        camera: outside_camera,
    });
    prepared.vm.render_gpu(&mut primary_target, 3).unwrap();
    let mut reset_status = prepared
        .vm
        .vm
        .capture_paint_page_status(&context)
        .unwrap()
        .unwrap();
    finish(&context);
    assert_eq!(
        reset_status.try_finish(&context).unwrap(),
        Some(scenevm::paint_feedback::PaintPageStatus::Ready)
    );
    let mut indirect = OrthographicBakeJob::new(context.clone(), prepared, frame.clone()).unwrap();
    let mut indirect_bake = OrthographicBakeController::default();
    indirect_bake.request_render(map.id, 1);
    indirect_bake
        .prepare_work(map.id, 256, 256, tight_camera, Some((-0.5, 0.5, -0.5, 0.5)))
        .unwrap();
    let before = context.transfers();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match indirect.advance(&mut indirect_bake, map.id, tight_camera, (256, 256)) {
            Ok(BakeAdvance::Pending) => finish(&context),
            Err(error) => {
                assert!(error.to_string().contains("not resident"));
                break;
            }
            other => panic!("off-screen paint cannot be omitted: {other:?}"),
        }
        assert!(std::time::Instant::now() < deadline);
    }
    assert_eq!(context.transfers().readback_bytes, before.readback_bytes);
    assert!(indirect_bake.take_persisted_update_for(map.id).is_none());

    // An authored key that no traced ray visits is different from a page miss.
    // Its status resolves Ready and normal original-channel production proceeds.
    let mut unused_chunk = chunk.clone();
    for poly in unused_chunk.polys3d_map.values_mut().flatten() {
        poly.paint_surface_id = Some([8, 0, 0, 1]);
    }
    let mut prepared = scene(&context, &assets, &unused_chunk);
    prepared
        .vm
        .set_raster3d_surface_paint(1, 1, vec![0; 4], vec![0; 4], entries, Vec::new());
    let mut unused = OrthographicBakeJob::new(context.clone(), prepared, frame).unwrap();
    let mut unused_bake = OrthographicBakeController::default();
    unused_bake.request_render(map.id, 1);
    unused_bake
        .prepare_work(map.id, 256, 256, camera_frame, Some((-0.5, 0.5, -0.5, 0.5)))
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        match unused
            .advance(&mut unused_bake, map.id, camera_frame, (256, 256))
            .unwrap()
        {
            BakeAdvance::Tile { complete: true } => break,
            BakeAdvance::Pending | BakeAdvance::Sample => finish(&context),
            other => panic!("unused source key should not block: {other:?}"),
        }
        assert!(std::time::Instant::now() < deadline);
    }
    assert_eq!(
        unused_bake
            .take_persisted_update_for(map.id)
            .unwrap()
            .unwrap()
            .tiles
            .len(),
        1
    );
}

#[test]
#[ignore = "native GPU; progressive original channel/reference and retirement integration"]
fn cooperative_tile_job_matches_reference_channels_and_retires_pending_requests() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut assets = Assets::default();
    let tile = rusterix::Tile::from_texture(Texture::new(
        vec![
            210, 60, 40, 255, 50, 170, 80, 160, 30, 80, 220, 255, 170, 150, 60, 0,
        ],
        2,
        2,
    ));
    let tile_id = tile.id;
    assets.set_tiles([(tile_id, tile)].into_iter().collect());
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(Poly3D::cube(
        GeoId::GeometryObject(Uuid::new_v4()),
        tile_id,
        Vec3::zero(),
        1.,
    ));
    let mut map = Map::new();
    map.camera = MapCamera::ThreeDIso;
    let authored = serde_json::to_vec(&map).unwrap();
    let mut camera = D3IsoCamera::new();
    camera.scale = 1.;
    camera.distance = 4.;
    camera.height_clearance = 0.;
    camera.azimuth_deg = 90.;
    camera.elevation_deg = 0.;
    let camera_frame = camera.as_scenevm_camera_for_surface(256., 256.);
    let mut reference = Rusterix::with_scene_handler(scene(&context, &assets, &chunk));
    reference.assets = assets.clone();
    reference.client.camera_d3 = Box::new(camera.clone());
    reference
        .client
        .set_server_time(theframework_core::TheTime::new_time(12, 0).unwrap());
    let EditorViewFrame::D3(frame) =
        reference
            .client
            .editor_view_3d(&map, (256, 256), true, None, true, false)
    else {
        unreachable!()
    };
    let mut prepared = scene(&context, &assets, &chunk);
    prepared.runtime_render_state = reference.render_state_for_view(&map, false);
    let mut job = OrthographicBakeJob::new(context.clone(), prepared, frame.clone()).unwrap();
    let mut bake = OrthographicBakeController::default();
    bake.request_render(map.id, 2);
    let suspended = context.transfers();
    assert_eq!(
        job.advance(&mut bake, map.id, camera_frame, (0, 256))
            .unwrap(),
        BakeAdvance::Pending
    );
    assert_eq!(context.transfers(), suspended);
    assert!(!job.has_pending_capture());
    let mut previews = 0;
    let mut complete = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !complete {
        let before = context.transfers();
        let advance = job
            .advance(&mut bake, map.id, camera_frame, (256, 256))
            .unwrap();
        let after = context.transfers();
        assert_eq!(
            after.readback_bytes - after.asset_bake_readback_bytes,
            before.readback_bytes - before.asset_bake_readback_bytes
        );
        assert!(after.asset_bake_readback_requests - before.asset_bake_readback_requests <= 1);
        assert!(!job.has_cpu_viewport_storage());
        match advance {
            BakeAdvance::Sample => {
                previews += 1;
                assert!(bake.presentation_source(map.id, &camera_frame).is_some());
                assert!(bake.take_persisted_update_for(map.id).is_none());
                if previews == 1 {
                    bake.toggle_visibility();
                    assert!(bake.presentation_source(map.id, &camera_frame).is_none());
                    bake.toggle_visibility();
                }
            }
            BakeAdvance::Tile { complete: done } => complete = done,
            BakeAdvance::Pending => {}
            other => panic!("unexpected progressive state {other:?}"),
        }
        if job.has_pending_capture() {
            finish(&context);
        }
        assert!(std::time::Instant::now() < deadline);
    }
    assert_eq!(previews, 2);
    let asset = bake.take_persisted_update_for(map.id).unwrap().unwrap();
    assert_eq!(asset.version, 5);
    assert_eq!(asset.samples, 2);
    assert_eq!(asset.tiles.len(), 1);
    assert_eq!(serde_json::to_vec(&map).unwrap(), authored);
    let counters = context.transfers();
    assert_eq!(counters.readback_bytes, counters.asset_bake_readback_bytes);
    assert_eq!(
        counters.capture_requests,
        counters.asset_bake_readback_requests
    );
    assert_eq!(counters.asset_bake_readback_requests, 6); // 2 beauty + 3 channels + 1 depth
    assert_eq!(
        counters.asset_bake_readback_bytes,
        256 * 256 * (2 * 4 + 3 * 4 + 32)
    );

    // The retained reference's real CPU drawing/capture adapter is the oracle;
    // prewarm its geometry before its existing bounds-based planner runs.
    let mut pixels = vec![0; 256 * 256 * 4];
    reference.draw_d3_with_editor_background(&map, &mut pixels, 256, 256, true);
    reference.request_orthographic_bake(map.id, 2);
    while reference.orthographic_bake.is_rendering() {
        reference.draw_d3_with_editor_background(&map, &mut pixels, 256, 256, true);
    }
    let expected = reference
        .orthographic_bake
        .take_persisted_update_for(map.id)
        .unwrap()
        .unwrap();
    assert_eq!(asset.projected_bounds, expected.projected_bounds);
    assert_eq!(asset.tile_size, expected.tile_size);
    let actual = bake.presentation_source(map.id, &camera_frame).unwrap();
    let expected = reference
        .orthographic_bake
        .presentation_source(map.id, &camera_frame)
        .unwrap();
    for (at, lease) in actual.tiles() {
        let tile = lease.source();
        let expected_lease = expected.tile(at.0, at.1).unwrap();
        let expected = expected_lease.source();
        for (name, a, b) in [
            ("beauty", tile.color, expected.color),
            ("albedo", tile.albedo, expected.albedo),
            ("normal", tile.normal, expected.normal),
            ("material", tile.material, expected.material),
        ] {
            assert_eq!(a.len(), b.len());
            let delta = a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
            eprintln!("Progressive bake {name}: max reference delta={delta}");
            assert!(delta <= 1, "{name}: {delta}");
        }
        eprintln!(
            "Progressive bounds={:?} camera={camera_frame:?} alpha: beauty={} albedo={} normal={} material={} depth={}/{} reference_depth={}",
            asset.projected_bounds,
            tile.color.chunks_exact(4).filter(|p| p[3] != 0).count(),
            tile.albedo.chunks_exact(4).filter(|p| p[3] != 0).count(),
            tile.normal.chunks_exact(4).filter(|p| p[3] != 0).count(),
            tile.material.chunks_exact(4).filter(|p| p[3] != 0).count(),
            tile.depth.iter().filter(|d| d.is_finite()).count(),
            tile.depth.len(),
            expected.depth.iter().filter(|d| d.is_finite()).count()
        );
        if std::env::var_os("ELDIRON_BAKE_DEBUG").is_some() {
            let surface = reference
                .scene_handler
                .vm
                .paint_surface_buffer_gpu(256, 256)
                .unwrap();
            eprintln!(
                "Reference post-bake static valid={} stats={:?}",
                surface.pixels.iter().filter(|p| p.valid).count(),
                reference.scene_handler.vm.active_vm().debug_stats()
            );
        }
        assert!(tile.color.chunks_exact(4).any(|pixel| pixel[3] != 0));
        assert!(
            tile.material
                .chunks_exact(4)
                .any(|pixel| pixel[0] != 0 || pixel[2] != 0),
            "zero emissive must retain roughness/opacity bytes"
        );
        assert!(tile.depth.iter().any(|depth| depth.is_finite()));
        for (a, b) in tile.depth.iter().zip(expected.depth) {
            if b.is_finite() {
                assert!((a - b).abs() < 0.00001);
            } else {
                assert!(a.is_infinite());
            }
        }
    }
    drop(job);
    drop(reference);
    for phase in 0..4 {
        let mut job = OrthographicBakeJob::new(
            context.clone(),
            scene(&context, &assets, &chunk),
            frame.clone(),
        )
        .unwrap();
        let mut bake = OrthographicBakeController::default();
        bake.request_render(map.id, 2);
        job.advance(&mut bake, map.id, camera_frame, (256, 256))
            .unwrap();
        job.advance(&mut bake, map.id, camera_frame, (256, 256))
            .unwrap();
        assert!(job.has_pending_capture());
        match phase {
            0 => {
                bake.request_render(map.id, 2);
                assert_eq!(
                    job.advance(&mut bake, map.id, camera_frame, (256, 256))
                        .unwrap(),
                    BakeAdvance::Retired
                );
            }
            1 => {
                bake.clear();
                assert_eq!(
                    job.advance(&mut bake, map.id, camera_frame, (256, 256))
                        .unwrap(),
                    BakeAdvance::Idle
                );
                assert_eq!(bake.take_persisted_update_for(map.id), Some(None));
            }
            2 => {
                assert_eq!(
                    job.advance(&mut bake, Uuid::new_v4(), camera_frame, (256, 256))
                        .unwrap(),
                    BakeAdvance::Retired
                );
            }
            _ => {
                let mut changed = camera_frame;
                changed.forward = -changed.forward;
                assert!(job.advance(&mut bake, map.id, changed, (256, 256)).is_err());
            }
        }
        assert!(!job.has_pending_capture());
        assert!(!job.has_cpu_viewport_storage());
        assert!(bake.take_persisted_update_for(map.id).is_none());
    }
    // Actual device loss invalidates a pending original-tile capture.
    let mut job =
        OrthographicBakeJob::new(context.clone(), scene(&context, &assets, &chunk), frame).unwrap();
    let mut bake = OrthographicBakeController::default();
    bake.request_render(map.id, 2);
    job.advance(&mut bake, map.id, camera_frame, (256, 256))
        .unwrap();
    job.advance(&mut bake, map.id, camera_frame, (256, 256))
        .unwrap();
    context.device.destroy();
    for _ in 0..100 {
        let _ = context.poll();
        if context.is_lost() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(context.is_lost(), "device loss callback was not delivered");
    assert!(
        job.advance(&mut bake, map.id, camera_frame, (256, 256))
            .is_err()
    );
    assert!(!job.has_pending_capture());
}
