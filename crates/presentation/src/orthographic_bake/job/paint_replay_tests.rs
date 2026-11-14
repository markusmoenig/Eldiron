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
    replay_fixture(false, false);
}
#[test]
#[ignore = "native GPU; primary-hidden indirect paint must resolve before replay publication"]
fn demand_paint_replay_resolves_offscreen_indirect_dependencies() {
    replay_fixture(true, false);
}

#[test]
#[ignore = "native GPU; over-capacity dispatches withhold original samples until subdivision"]
fn paint_working_set_pressure_never_publishes_incomplete_originals() {
    replay_fixture(false, true);
}

fn replay_fixture(indirect: bool, pressure: bool) {
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
    poly.paint_surface_id = Some(if indirect { [8, 0, 0, 1] } else { geo });
    poly.paint_uvs = if pressure {
        poly.uvs.iter().map(|uv| [uv[0] * 6., uv[1] * 2.]).collect()
    } else {
        vec![[0.5; 2]; poly.vertices.len()]
    };
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(poly);
    if indirect {
        let mut wall = Poly3D::cube(GeoId::Sector(2), tile_id, Vec3::zero(), 20.);
        wall.paint_surface_id = Some(geo);
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
            pixel.copy_from_slice(&[30, 190, 60, 255]);
        }
        for pixel in page.material_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[254, 1, 255, 255]);
        }
        paint.baked_chunks.insert(at.to_string(), page);
    }
    let source = Arc::new(PaintPageIndex::from_layer(&paint, 1024).unwrap());
    let slot = source.slot(geo, [0; 2]).unwrap();
    if pressure {
        let mut job =
            OrthographicBakeJob::new(context.clone(), scene(&context, &assets, &chunk), frame)
                .unwrap();
        job.set_paint_pages(source, 64 * 64 * 8 * 2).unwrap();
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
                    assert!(error.to_string().contains("subdivision required"));
                    break;
                }
                other => panic!("over-capacity paint cannot publish: {other:?}"),
            }
            assert!(std::time::Instant::now() < deadline);
        }
        assert_eq!(job.paint_residency_usage().unwrap().page_uploads, 2);
        assert_eq!(job.paint_residency_usage().unwrap().pinned, 2);
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

    let mut prepared = scene(&context, &assets, &chunk);
    let original = source.page(slot).unwrap();
    prepared.vm.set_raster3d_surface_paint(
        64,
        64,
        original.color.to_vec(),
        original.material.to_vec(),
        source.resident_entries([64; 2], &[(slot, [0; 2])]).unwrap(),
        source.alpha_geo_ids().to_vec(),
    );
    let full = OrthographicBakeJob::new(context.clone(), prepared, frame.clone()).unwrap();
    let mut paged =
        OrthographicBakeJob::new(context.clone(), scene(&context, &assets, &chunk), frame).unwrap();
    paged.set_paint_pages(source.clone(), 64 * 64 * 8).unwrap();

    // Retiring a request while its dependency receipt is still pending must
    // cancel private work without ever starting original-channel capture.
    let mut cancelled = OrthographicBakeJob::new(
        context.clone(),
        scene(&context, &assets, &chunk),
        paged.frame.clone(),
    )
    .unwrap();
    cancelled
        .set_paint_pages(source.clone(), 64 * 64 * 8)
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
            if let Some((work, _)) = job.replay {
                faults.push(work.sample);
                assert_eq!(
                    context.transfers().readback_bytes,
                    before.readback_bytes,
                    "fault cannot capture an original channel"
                );
                assert_eq!(
                    work.sample, samples,
                    "fault cannot advance the sample clock"
                );
                assert!(bake.take_persisted_update_for(map.id).is_none());
            }
            match result {
                BakeAdvance::Sample => {
                    samples += 1;
                    if reset && samples == 1 {
                        // Drop residency after one real sample to force a fault
                        // with nonempty accumulation. CPU originals stay fixed.
                        let empty =
                            PaintPageResidency::new(context.clone(), source.clone(), 64 * 64 * 8)
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
        assert_eq!(
            context.transfers().asset_bake_readback_requests - start.asset_bake_readback_requests,
            7
        );
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
