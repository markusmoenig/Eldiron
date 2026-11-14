use super::*;
use crate::orthographic_bake::job_tests::{finish, scene};
use rusterix::{Assets, D3Camera, D3IsoCamera, Map, MapCamera, Rusterix, Texture};
use scenevm::{
    Chunk, GeoId, Poly3D,
    prelude::{Vec2, Vec3},
};
use shared::{
    iso_paint::{IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner},
    surface_paint::pages::PaintPageIndex,
};

#[test]
#[ignore = "native GPU; faults after a valid sample must replay without history contamination"]
fn demand_paint_replay_preserves_original_channels_and_previous_valid_samples() {
    replay_fixture(Scenario::Direct);
}
#[test]
#[ignore = "native GPU; primary-hidden indirect paint must resolve before replay publication"]
fn demand_paint_replay_resolves_offscreen_indirect_dependencies() {
    replay_fixture(Scenario::Indirect);
}

#[test]
#[ignore = "native GPU; bounded subregions preserve complete original samples under pressure"]
fn subdivision_preserves_originals_when_three_pages_exceed_two_slots() {
    replay_fixture(Scenario::SpatialPressure);
}

#[test]
#[ignore = "native GPU; original source larger than GPU residency stays complete without an atlas clone"]
fn large_original_paint_source_completes_through_bounded_subdivision() {
    replay_fixture(Scenario::LargeSource);
}

#[test]
#[ignore = "native GPU; one ray exceeding residency must fail without an incomplete sample"]
fn single_pixel_ray_pressure_withholds_all_original_publication() {
    replay_fixture(Scenario::RayPressure);
}

#[test]
#[ignore = "native GPU; missing primary paint cannot pin a provisional fallback page"]
fn unresolved_primary_paint_never_consumes_speculative_fallback_residency() {
    replay_fixture(Scenario::Fallback);
}

#[test]
#[ignore = "native GPU; device loss after a resolved private subregion cannot publish"]
fn device_loss_with_partial_paint_regions_retires_without_original_capture() {
    replay_fixture(Scenario::PartialLoss);
}

enum Scenario {
    Direct,
    Indirect,
    SpatialPressure,
    LargeSource,
    RayPressure,
    Fallback,
    PartialLoss,
}
fn replay_fixture(scenario: Scenario) {
    let ray_pressure = matches!(scenario, Scenario::RayPressure);
    let fallback = matches!(scenario, Scenario::Fallback);
    let indirect = matches!(scenario, Scenario::Indirect | Scenario::RayPressure);
    let pressure = matches!(
        scenario,
        Scenario::SpatialPressure | Scenario::LargeSource | Scenario::PartialLoss
    );
    let partial_loss = matches!(scenario, Scenario::PartialLoss);
    let large = matches!(scenario, Scenario::LargeSource);
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut assets = Assets::default();
    let mut texture = Texture::new(vec![210, 60, 40, 255], 1, 1);
    texture.data_ext = Some(vec![0x08, 0x0f, 128, 128]);
    let tile = rusterix::Tile::from_texture(texture);
    let tile_id = tile.id;
    assets.set_tiles([(tile_id, tile)].into_iter().collect());
    let geo = [7, 0, 0, 1];
    let mut poly = Poly3D::cube(
        GeoId::Sector(1),
        tile_id,
        Vec3::zero(),
        if indirect { 1.2 } else { 1. },
    );
    poly.paint_surface_id = Some(if indirect && !ray_pressure {
        [8, 0, 0, 1]
    } else {
        geo
    });
    poly.paint_uvs = if pressure {
        poly.uvs.iter().map(|uv| [uv[0] * 6., uv[1] * 2.]).collect()
    } else {
        vec![[0.5; 2]; poly.vertices.len()]
    };
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(poly);
    if indirect {
        let mut wall = Poly3D::cube(GeoId::Sector(2), tile_id, Vec3::zero(), 20.);
        wall.paint_surface_id = Some(if ray_pressure { [9, 0, 0, 1] } else { geo });
        wall.paint_uvs = vec![[0.5; 2]; wall.vertices.len()];
        chunk.add_3d(wall);
    }
    let mut map = Map::new();
    map.camera = MapCamera::ThreeDIso;
    let mut camera = D3IsoCamera::new();
    camera.scale = 1.;
    camera.distance = 4.;
    camera.height_clearance = 0.;
    camera.azimuth_deg = 0.;
    camera.elevation_deg = 0.;
    let mut camera_frame = camera.as_scenevm_camera_for_surface(256., 256.);
    if indirect {
        camera_frame.ortho_half_h = 0.5;
    }
    let mut reference = Rusterix::with_scene_handler(scene(&context, &assets, &chunk));
    reference.assets = assets.clone();
    reference.client.camera_d3 = Box::new(camera);
    let EditorViewFrame::D3(frame) =
        reference
            .client
            .editor_view_3d(&map, (256, 256), true, None, true, false)
    else {
        unreachable!()
    };
    let mut paint = IsoPaintLayer::default();
    for at in 0..if pressure { 3 } else { 1 } {
        let mut page = IsoPaintBakedChunk::new(IsoPaintOwner::Sector(1), geo, [at * 64, 0]);
        for pixel in page.color_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[30 + at as u8 * 45, 190, 60, 255]);
        }
        for pixel in page.material_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[254, 1 + at as u8, 255, 255]);
        }
        paint.baked_chunks.insert(at.to_string(), page);
    }
    if fallback {
        let geo = scenevm::core::legacy_raster3d_paint_surface_id(
            GeoId::Sector(1),
            0,
            [0., 0., 1.],
            [-0.5; 3],
        );
        for y in [-64, 0] {
            let mut page = IsoPaintBakedChunk::new(IsoPaintOwner::Sector(1), geo, [0, y]);
            for pixel in page.color_rgba.chunks_exact_mut(4) {
                pixel.copy_from_slice(&[220, 30, 20, 255]);
            }
            paint.baked_chunks.insert(format!("fallback{y}"), page);
        }
    }
    if ray_pressure {
        paint.baked_chunks.insert(
            "enclosure".into(),
            IsoPaintBakedChunk::new(IsoPaintOwner::Sector(2), [9, 0, 0, 1], [0; 2]),
        );
    }
    if large {
        for at in 3..2049 {
            let mut page =
                IsoPaintBakedChunk::new(IsoPaintOwner::Sector(99), [99, at, 0, 1], [0; 2]);
            page.color_rgba[..4].copy_from_slice(&[at as u8, (at >> 8) as u8, 128, 255]);
            paint.baked_chunks.insert(at.to_string(), page);
        }
    }
    let source = Arc::new(PaintPageIndex::from_layer(&paint, 2 * 1024 * 1024).unwrap());
    if large {
        assert!(source.logical_source_bytes() > 64 * 1024 * 1024);
    }
    if ray_pressure {
        let mut job =
            OrthographicBakeJob::new(context.clone(), scene(&context, &assets, &chunk), frame)
                .unwrap();
        job.set_paint_pages(source, 64 * 64 * 8).unwrap();
        let mut bake = OrthographicBakeController::default();
        bake.request_render(map.id, 2);
        bake.prepare_work(map.id, 256, 256, camera_frame, Some((-0.5, 0.5, -0.5, 0.5)))
            .unwrap();
        let before = context.transfers();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            match job.advance(&mut bake, map.id, camera_frame, (256, 256)) {
                Ok(BakeAdvance::Pending) => finish(&context),
                Err(error) => {
                    assert!(error.to_string().contains("single-pixel"));
                    break;
                }
                other => panic!("one over-capacity ray cannot publish: {other:?}"),
            }
            assert!(std::time::Instant::now() < deadline);
        }
        assert!(job.paint_subdivisions() > 0);
        assert_eq!(context.transfers().readback_bytes, before.readback_bytes);
        assert!(bake.presentation_source(map.id, &camera_frame).is_none());
        assert!(bake.take_persisted_update_for(map.id).is_none());
        assert_eq!(
            job.advance(&mut bake, map.id, camera_frame, (256, 256))
                .unwrap(),
            BakeAdvance::Retired
        );
        return;
    }
    if fallback {
        // Prove these keys are real fallback candidates when primary paint is
        // resolved empty. The success case below uses opaque primary paint, so
        // querying them while it is missing would be speculative.
        let mut probe = scene(&context, &assets, &chunk);
        let primary = source.slot(geo, [0; 2]).unwrap();
        probe.vm.set_raster3d_surface_paint(
            64,
            64,
            vec![0; 64 * 64 * 4],
            vec![0; 64 * 64 * 4],
            source
                .resident_entries([64; 2], &[(primary, [0; 2])])
                .unwrap(),
            Vec::new(),
        );
        probe
            .vm
            .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Compute3D));
        probe.vm.execute(scenevm::Atom::SetCamera3D {
            camera: camera_frame,
        });
        probe.vm.set_layer_compute3d_bake_output(0, 1);
        let mut target =
            scenevm::render_output::SceneRenderTarget::new(context.clone(), 256, 256).unwrap();
        probe.vm.render_gpu(&mut target, 1).unwrap();
        let mut status = probe
            .vm
            .vm
            .capture_paint_page_status(&context)
            .unwrap()
            .unwrap();
        finish(&context);
        let Some(scenevm::paint_feedback::PaintPageStatus::MissingEntry(slot)) =
            status.try_finish(&context).unwrap()
        else {
            panic!("resolved-empty primary must query authored fallback");
        };
        assert_ne!(source.page(slot).unwrap().geo, geo);
    }
    let count = if pressure { 3 } else { 1 };
    let width = count * 64;
    let mut color = vec![0; width as usize * 64 * 4];
    let mut material = color.clone();
    let mut residents = Vec::new();
    for at in 0..count {
        let slot = source.slot(geo, [at as i32 * 64, 0]).unwrap();
        let original = source.page(slot).unwrap();
        for row in 0..64 {
            let start = (row * width as usize + at as usize * 64) * 4;
            color[start..start + 256].copy_from_slice(&original.color[row * 256..row * 256 + 256]);
            material[start..start + 256]
                .copy_from_slice(&original.material[row * 256..row * 256 + 256]);
        }
        residents.push((slot, [at * 64, 0]));
    }
    let mut prepared = scene(&context, &assets, &chunk);
    if let Some(path) = std::env::var_os("ELDIRON_PAINT_REFERENCE_BODY") {
        prepared.vm.vm.source3d = std::fs::read_to_string(path).unwrap();
    }
    prepared.vm.set_raster3d_surface_paint(
        width,
        64,
        color,
        material,
        source.resident_entries([width, 64], &residents).unwrap(),
        source.alpha_geo_ids().to_vec(),
    );
    let plane_budget = 64 * 64 * 8 * if pressure { 2 } else { 1 };
    let full = OrthographicBakeJob::new(context.clone(), prepared, frame.clone()).unwrap();
    let mut paged =
        OrthographicBakeJob::new(context.clone(), scene(&context, &assets, &chunk), frame).unwrap();
    paged.set_paint_pages(source.clone(), plane_budget).unwrap();

    // Retiring a request while its dependency receipt is still pending must
    // cancel private work without ever starting original-channel capture.
    let mut cancelled = OrthographicBakeJob::new(
        context.clone(),
        scene(&context, &assets, &chunk),
        paged.frame.clone(),
    )
    .unwrap();
    cancelled
        .set_paint_pages(source.clone(), plane_budget)
        .unwrap();
    let mut retired = OrthographicBakeController::default();
    retired.request_render(map.id, 3);
    retired
        .prepare_work(map.id, 256, 256, camera_frame, Some((-0.5, 0.5, -0.5, 0.5)))
        .unwrap();
    let before_cancel = context.transfers();
    cancelled
        .advance(&mut retired, map.id, camera_frame, (256, 256))
        .unwrap();
    cancelled
        .advance(&mut retired, map.id, camera_frame, (256, 256))
        .unwrap();
    assert!(matches!(&cancelled.pending, Some(Pending::Paint { .. })));
    if pressure {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !cancelled
            .regions
            .as_ref()
            .is_some_and(|regions| regions.resolved() > 0)
        {
            assert_eq!(
                cancelled
                    .advance(&mut retired, map.id, camera_frame, (256, 256))
                    .unwrap(),
                BakeAdvance::Pending
            );
            finish(&context);
            assert!(std::time::Instant::now() < deadline);
        }
        assert!(cancelled.paint_subdivisions() > 0);
        assert!(retired.presentation_source(map.id, &camera_frame).is_none());
        assert!(retired.take_persisted_update_for(map.id).is_none());
        // Leave another region's receipt pending while retiring this request.
        cancelled
            .advance(&mut retired, map.id, camera_frame, (256, 256))
            .unwrap();
        assert!(matches!(&cancelled.pending, Some(Pending::Paint { .. })));
    }
    if partial_loss {
        context.device.destroy();
        for _ in 0..100 {
            let _ = context.poll();
            if context.is_lost() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(context.is_lost());
        assert!(
            cancelled
                .advance(&mut retired, map.id, camera_frame, (256, 256))
                .is_err()
        );
        assert_eq!(
            cancelled
                .advance(&mut retired, map.id, camera_frame, (256, 256))
                .unwrap(),
            BakeAdvance::Retired
        );
        assert_eq!(
            context.transfers().readback_bytes,
            before_cancel.readback_bytes
        );
        assert!(retired.presentation_source(map.id, &camera_frame).is_none());
        assert!(retired.take_persisted_update_for(map.id).is_none());
        return;
    }

    retired.request_render(map.id, 3);
    assert_eq!(
        cancelled
            .advance(&mut retired, map.id, camera_frame, (256, 256))
            .unwrap(),
        BakeAdvance::Retired
    );
    finish(&context);
    assert_eq!(
        context.transfers().readback_bytes,
        before_cancel.readback_bytes
    );
    assert!(retired.take_persisted_update_for(map.id).is_none());
    drop(cancelled);

    let run = |mut job: OrthographicBakeJob, reset: bool| {
        let mut bake = OrthographicBakeController::default();
        bake.request_render(map.id, 3);
        bake.prepare_work(map.id, 256, 256, camera_frame, Some((-0.5, 0.5, -0.5, 0.5)))
            .unwrap();
        let start = context.transfers();
        let mut samples = 0;
        let mut faults = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let before = context.transfers();
            let result = job
                .advance(&mut bake, map.id, camera_frame, (256, 256))
                .unwrap();
            if let Some((work, channel)) = job.replay {
                if matches!(channel, Channel::Beauty) {
                    faults.push(work.sample);
                }
                assert_eq!(
                    context.transfers().readback_bytes,
                    before.readback_bytes,
                    "fault cannot capture an original channel"
                );
                if matches!(channel, Channel::Beauty) {
                    assert_eq!(
                        work.sample, samples,
                        "region replay cannot advance the sample clock"
                    );
                }
                assert!(bake.take_persisted_update_for(map.id).is_none());
            }
            match result {
                BakeAdvance::Sample => {
                    samples += 1;
                    if reset && samples == 1 {
                        // Drop residency after one real sample to force a fault
                        // with nonempty accumulation. CPU originals stay fixed.
                        let empty =
                            PaintPageResidency::new(context.clone(), source.clone(), plane_budget)
                                .unwrap();
                        job.scene
                            .vm
                            .set_paint_page_binding(&context, empty.binding().unwrap())
                            .unwrap();
                        job.paint_pages = Some(empty);
                    }
                }
                BakeAdvance::Tile { complete: true } => break,
                BakeAdvance::Pending => {}
                other => panic!("unexpected replay advance {other:?}"),
            }
            finish(&context);
            assert!(std::time::Instant::now() < deadline);
        }
        assert_eq!(samples, 3);
        if reset && pressure {
            assert!(
                job.paint_subdivisions() > 0,
                "working set must actually subdivide"
            );
            assert!(job.paint_residency_usage().unwrap().gpu_plane_bytes <= plane_budget);
        }
        assert_eq!(
            context.transfers().asset_bake_readback_requests - start.asset_bake_readback_requests,
            7
        );
        if reset && fallback {
            assert_eq!(job.paint_subdivisions(), 0);
            assert_eq!(job.paint_residency_usage().unwrap().page_uploads, 1);
        }
        (bake, faults)
    };
    let (expected, no_faults) = run(full, false);
    assert!(no_faults.is_empty());
    let (actual, faults) = run(paged, true);
    assert!(
        faults.contains(&0) && faults.contains(&1),
        "must replay both first and later samples: {faults:?}"
    );
    let expected = expected.presentation_source(map.id, &camera_frame).unwrap();
    let actual = actual.presentation_source(map.id, &camera_frame).unwrap();
    let mut green = 0;
    for (at, lease) in actual.tiles() {
        let tile = lease.source();
        let reference = expected.tile(at.0, at.1).unwrap();
        let reference = reference.source();
        for (channel, actual, expected) in [
            ("beauty", tile.color, reference.color),
            ("albedo", tile.albedo, reference.albedo),
            ("material", tile.material, reference.material),
            ("normal", tile.normal, reference.normal),
        ] {
            let delta = actual
                .iter()
                .zip(expected)
                .map(|(a, b)| a.abs_diff(*b))
                .max()
                .unwrap();
            eprintln!("Demand paint replay {channel}: max delta={delta}");
            assert!(delta <= 1, "{channel} replay changed original pixels");
        }
        green += tile
            .albedo
            .chunks_exact(4)
            .filter(|pixel| pixel[1] > pixel[0].saturating_add(50))
            .count();
        for (a, b) in tile.depth.iter().zip(reference.depth) {
            assert_eq!(a.is_finite(), b.is_finite());
            if a.is_finite() {
                assert!((a - b).abs() < 1e-5);
            }
        }
    }
    assert!(
        indirect || green > 100,
        "paint must visibly affect actual original albedo"
    );
    assert_eq!(
        context.transfers().readback_bytes,
        context.transfers().asset_bake_readback_bytes
    );
}
