use super::*;
use renderkit::ReadbackTicket;
use rusterix::OrthographicBakeController;
use scenevm::prelude::Vec3;

fn fixture(region: Uuid, camera: Camera3D) -> OrthographicBakeController {
    let mut owner = OrthographicBakeController::default();
    owner.request_render(region, 1);
    let mut tile = 0u8;
    while owner.is_rendering() {
        let work = owner
            .prepare_work(region, 256, 256, camera, Some((-2., 2., -2., 2.)))
            .unwrap()
            .unwrap();
        let mut color = Vec::new();
        let mut depth = Vec::new();
        let mut albedo = Vec::new();
        let mut normal = Vec::new();
        let mut material = Vec::new();
        for y in 0..work.tile_size {
            for x in 0..work.tile_size {
                let alpha = if (x + y) % 31 == 0 {
                    0
                } else if x % 3 == 0 {
                    128u32
                } else {
                    255
                };
                color.extend(
                    [
                        (20 + (x % 31) * 3) * alpha / 255,
                        (20 + (y % 31) * 3) * alpha / 255,
                        (30 + u32::from(tile) * 20) * alpha / 255,
                        alpha,
                    ]
                    .map(|v| v as u8),
                );
                depth.push(2. + (x + y * work.tile_size) as f32 / 10000.);
                albedo.extend([80 + (x % 64) as u8, 96, 127, 255]);
                normal.extend([128, 255, 128, 255]);
                material.extend([200, (x % 127) as u8, 255, 0]);
            }
        }
        owner.preview_current_tile(&color).unwrap();
        assert!(owner.finish_sample());
        owner
            .commit_current_tile(color, depth, albedo, normal, material)
            .unwrap();
        tile += 1;
    }
    assert_eq!(tile, 4);
    owner
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
fn colors(context: &GpuContext, output: &OrthographicBakeOutput) -> Vec<u8> {
    let mut capture = ReadbackTicket::capture(context, output.image.clone()).unwrap();
    finish(context);
    capture
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}
fn depths(context: &GpuContext, output: &OrthographicBakeOutput) -> Vec<f32> {
    let bytes = u64::from(output.size.0) * u64::from(output.size.1) * 4;
    let staging = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("explicit bake depth fixture capture"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(&output.depth, 0, &staging, 0, bytes);
    context.queue.submit(Some(encoder.finish()));
    context.record_capture(bytes);
    let (send, receive) = std::sync::mpsc::channel();
    staging
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap()
        });
    finish(context);
    receive.recv().unwrap().unwrap();
    let result = bytemuck::cast_slice::<u8, f32>(&staging.slice(..).get_mapped_range()).to_vec();
    staging.unmap();
    result
}

#[test]
#[ignore = "requires native GPU; explicit color/depth compatibility captures"]
fn tiles_compose_with_reference_pan_resize_alpha_sun_depth_and_retained_sources() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let region = Uuid::new_v4();
    let mut camera = Camera3D::iso();
    camera.ortho_half_h = 1.;
    let mut owner = fixture(region, camera);
    let mut renderer = OrthographicBakeRenderer::new(context.clone()).unwrap();
    for (phase, (size, sun)) in [((96, 64), false), ((135, 83), true), ((64, 129), true)]
        .into_iter()
        .enumerate()
    {
        if phase != 0 {
            camera.pos += camera.right * 0.137 + camera.up * -0.211 - camera.forward * 0.75;
        }
        let lighting = sun.then_some(OrthographicBakeLighting {
            sun_direction: Vec3::new(0.2, -1., 0.1),
            sun_color: Vec3::new(1., 0.8, 0.6),
            sun_intensity: 1.75,
            sun_enabled: true,
        });
        let source = owner.presentation_source(region, &camera).unwrap();
        let output = renderer.render(&source, &camera, size, lighting).unwrap();
        assert!(renderer.source_resident_bytes() <= renderer.source_budget_bytes());
        let uploaded = context.transfers().uploaded_bytes;
        let repeated = renderer.render(&source, &camera, size, lighting).unwrap();
        assert!(Arc::ptr_eq(&output.image, &repeated.image));
        assert!(
            context.transfers().uploaded_bytes - uploaded
                <= std::mem::size_of::<Params>() as u64 * 5,
            "unchanged tiles cannot be uploaded again"
        );
        if phase == 0 {
            assert_eq!(context.transfers().readback_bytes, 0);
            assert_eq!(context.transfers().capture_requests, 0);
        }
        let expected = owner
            .compose_rgba_lit(region, size.0, size.1, &camera, lighting)
            .unwrap();
        let actual = colors(&context, &output);
        let delta = actual
            .iter()
            .zip(&expected)
            .map(|(a, b)| a.abs_diff(*b))
            .max()
            .unwrap();
        eprintln!("Baked source phase {phase}: max color delta={delta}");
        if delta > 1 {
            for (index, (a, b)) in actual
                .chunks_exact(4)
                .zip(expected.chunks_exact(4))
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .take(12)
            {
                eprintln!(
                    "mismatch {},{}: {a:?} != {b:?}",
                    index % size.0 as usize,
                    index / size.0 as usize
                );
            }
        }
        assert!(
            delta <= 1,
            "bake color compatibility phase {phase}: {delta}"
        );
        let expected_depth = owner
            .compose_depth(region, size.0, size.1, &camera)
            .unwrap();
        for (index, (actual, expected)) in depths(&context, &output)
            .into_iter()
            .zip(expected_depth)
            .enumerate()
        {
            assert!(
                actual == expected || (actual - expected).abs() < 0.00001,
                "bake depth phase{phase} index{index}: {actual} != {expected}"
            );
        }
    }
    let mut legacy = owner.take_persisted_update().unwrap().unwrap();
    legacy.version = 3;
    owner.sync_persisted_asset(region, Some(&legacy));
    let output = renderer
        .render(
            &owner.presentation_source(region, &camera).unwrap(),
            &camera,
            (96, 64),
            None,
        )
        .unwrap();
    assert!(!output.depth_enabled);
    assert!(
        depths(&context, &output)
            .into_iter()
            .all(|depth| depth == f32::INFINITY)
    );
    let source = owner.presentation_source(region, &camera).unwrap();
    assert!(renderer.render(&source, &camera, (0, 64), None).is_err());
    let mut wrong_camera = camera;
    wrong_camera.kind = scenevm::CameraKind::OrbitPersp;
    assert!(
        renderer
            .render(&source, &wrong_camera, (96, 64), None)
            .is_err()
    );
    // More than 64 MiB of visible channels must fail before source upload or
    // submission; retained command buffers must not defeat the cache budget.
    let tile = legacy.tiles[0].clone();
    legacy.tiles = (0..43)
        .map(|x| {
            let mut tile = tile.clone();
            tile.x = x;
            tile.y = 0;
            tile
        })
        .collect();
    legacy.projected_bounds = [0., 86., -1., 1.];
    owner.sync_persisted_asset(region, Some(&legacy));
    camera.pos +=
        camera.right * (43. - camera.pos.dot(camera.right)) - camera.up * camera.pos.dot(camera.up);
    camera.ortho_half_h = 40.;
    let uploaded = context.transfers().uploaded_bytes;
    let error = renderer
        .render(
            &owner.presentation_source(region, &camera).unwrap(),
            &camera,
            (96, 64),
            None,
        )
        .err()
        .expect("oversized working set accepted");
    assert!(error.to_string().contains("paging is required"));
    assert_eq!(context.transfers().uploaded_bytes, uploaded);
}
