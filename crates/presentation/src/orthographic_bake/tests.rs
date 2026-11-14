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
fn ready(
    renderer: &mut OrthographicBakeRenderer,
    source: &OrthographicBakeSource<'_>,
    camera: &Camera3D,
    size: (u32, u32),
    light: Option<OrthographicBakeLighting>,
) -> OrthographicBakeOutput {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let output = renderer.render(source, camera, size, light).unwrap();
        let usage = renderer.context.source_pool_usage();
        assert!(usage.bytes <= usage.byte_limit && usage.submissions <= usage.submission_limit);
        if let Some(output) = output
            && !renderer.waiting()
        {
            return output;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "source pages did not complete"
        );
        finish(&renderer.context);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
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
        let output = ready(&mut renderer, &source, &camera, size, lighting);
        assert!(renderer.source_resident_bytes() <= renderer.source_budget_bytes());
        let uploaded = context.transfers().uploaded_bytes;
        let repeated = ready(&mut renderer, &source, &camera, size, lighting);
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
    let output = ready(
        &mut renderer,
        &owner.presentation_source(region, &camera).unwrap(),
        &camera,
        (96, 64),
        None,
    );
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
    // Visible sets larger than the source residency budget page to a private
    // target without retaining all original-channel GPU buffers.
    let alternate = legacy.tiles[1].clone();
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
    // 32 full original tiles exceed the CPU decode cache, but fit the GPU
    // working set. Resolve them one at a time rather than pinning the entire
    // visible set in the decoder before upload.
    let remaining = legacy.tiles.split_off(32);
    owner.sync_persisted_asset(region, Some(&legacy));
    assert_eq!(owner.decoded_resident_bytes(), 0);
    let before = context.transfers();
    let source = owner.presentation_source(region, &camera).unwrap();
    let paged = ready(&mut renderer, &source, &camera, (96, 64), None);
    assert_eq!(context.transfers().readback_bytes, before.readback_bytes);
    assert!(renderer.source_resident_bytes() <= PAGE_BYTES);
    assert!(source.decoded_resident_bytes() <= 32 * 1024 * 1024);
    let expected = owner.compose_rgba(region, 96, 64, &camera).unwrap();
    let actual = colors(&context, &paged);
    assert!(actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1));
    eprintln!(
        "Paged original channels: GPU={} CPU={} bytes",
        renderer.source_resident_bytes(),
        source.decoded_resident_bytes()
    );
    legacy.tiles.extend(remaining);
    legacy.version = 5;
    owner.sync_persisted_asset(region, Some(&legacy));
    let before = context.transfers();
    let source = owner.presentation_source(region, &camera).unwrap();
    assert!(
        renderer
            .render(&source, &camera, (96, 64), None)
            .unwrap()
            .is_none(),
        "a new projection cannot publish its first partial page"
    );
    let large = ready(&mut renderer, &source, &camera, (96, 64), None);
    assert_eq!(context.transfers().readback_bytes, before.readback_bytes);
    let expected = owner.compose_rgba(region, 96, 64, &camera).unwrap();
    assert!(
        colors(&context, &large)
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 1)
    );
    finish(&context);
    assert_eq!(context.source_pool_usage().submissions, 0);
    assert_eq!(context.source_pool_usage().upload_bytes, 0);
    assert!(context.source_pool_usage().bytes <= SOURCE_BUDGET);
    eprintln!(
        "43 visible source tiles completed with shared source bytes={}",
        context.source_pool_usage().bytes
    );
    let expected_depth = owner.compose_depth(region, 96, 64, &camera).unwrap();
    for (actual, expected) in depths(&context, &large).into_iter().zip(expected_depth) {
        assert!(actual == expected || (actual - expected).abs() < 1e-5);
    }
    // Two view owners share the pool and independently complete their pages.
    let mut second = OrthographicBakeRenderer::new(context.clone()).unwrap();
    camera.pos += camera.right * 0.037;
    let mut other = camera;
    other.pos += other.up * 0.025;
    let before = context.transfers().readback_bytes;
    let mut a = None;
    let mut b = None;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if a.is_none() {
            a = renderer.render(&source, &camera, (96, 64), None).unwrap();
        }
        if b.is_none() {
            b = second.render(&source, &other, (96, 64), None).unwrap();
        }
        let usage = context.source_pool_usage();
        assert!(usage.bytes <= usage.byte_limit && usage.submissions <= usage.submission_limit);
        if a.is_some() && b.is_some() {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "two source owners did not finish"
        );
        finish(&context);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(context.transfers().readback_bytes, before);
    let a = a.unwrap();
    let b = b.unwrap();
    assert!(!Arc::ptr_eq(a.image(), b.image()));
    for (output, camera) in [(&a, &camera), (&b, &other)] {
        let expected = owner.compose_rgba(region, 96, 64, camera).unwrap();
        assert!(
            colors(&context, output)
                .iter()
                .zip(expected)
                .all(|(a, b)| a.abs_diff(b) <= 1)
        );
    }
    // Revoke a pending camera request, then replace its source under a new
    // owner using identical tile coordinates. No old partial image may publish.
    other.pos += other.right * 0.5;
    assert!(
        second
            .render(&source, &other, (96, 64), None)
            .unwrap()
            .is_none()
    );
    let old_revision = source.revision();
    let next_region = Uuid::new_v4();
    legacy.tiles = (0..43)
        .map(|x| {
            let mut t = alternate.clone();
            t.x = x;
            t.y = 0;
            t
        })
        .collect();
    owner.sync_persisted_asset(next_region, Some(&legacy));
    let source = owner.presentation_source(next_region, &other).unwrap();
    assert_ne!(source.revision(), old_revision);
    assert!(
        second
            .render(&source, &other, (96, 64), None)
            .unwrap()
            .is_none()
    );
    let replaced = ready(&mut second, &source, &other, (96, 64), None);
    assert_eq!(replaced.source_revision(), source.revision());
    let expected = owner.compose_rgba(next_region, 96, 64, &other).unwrap();
    assert!(
        colors(&context, &replaced)
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 1)
    );
    other.pos += other.right * 0.1;
    assert!(
        second
            .render(&source, &other, (96, 64), None)
            .unwrap()
            .is_none()
    );
    context.device.destroy();
    for _ in 0..100 {
        let _ = context.poll();
        if context.is_lost() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(context.is_lost(), "device loss callback was not delivered");
    assert!(second.render(&source, &other, (96, 64), None).is_err());
    drop(second);
    drop(renderer);
    let _ = context.device.poll(wgpu::PollType::Poll);
    assert_eq!(context.source_pool_usage().submissions, 0);
}

#[test]
#[ignore = "actual GPU progressive source coalescing across page boundaries"]
fn progressive_source_updates_do_not_starve_pages_or_escape_their_request() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let map = Uuid::new_v4();
    let mut camera = Camera3D::iso();
    camera.ortho_half_h = 10.;
    let mut owner = OrthographicBakeController::default();
    owner.request_render(map, 1);
    let mut first_revision = None;
    for _ in 0..8 {
        let work = owner
            .prepare_work(map, 2048, 2048, camera, Some((-8., 8., -8., 8.)))
            .unwrap()
            .unwrap();
        let n = (work.tile_size * work.tile_size) as usize;
        assert!(owner.finish_sample());
        owner
            .commit_current_tile(
                [40, 80, 120, 255].repeat(n),
                vec![2.; n],
                [40, 80, 120, 255].repeat(n),
                [128, 255, 128, 255].repeat(n),
                [200, 0, 255, 0].repeat(n),
            )
            .unwrap();
    }
    let mut renderer = OrthographicBakeRenderer::new(context.clone()).unwrap();
    let request = owner.active_request().unwrap();
    let before = context.transfers().readback_bytes;
    let mut published = false;
    for step in 0..32 {
        let work = owner
            .prepare_work(map, 2048, 2048, camera, Some((-8., 8., -8., 8.)))
            .unwrap()
            .unwrap();
        let n = (work.tile_size * work.tile_size) as usize;
        owner
            .preview_current_tile(&[40 + step, 80, 120, 255].repeat(n))
            .unwrap();
        let source = owner.presentation_source(map, &camera).unwrap();
        first_revision.get_or_insert(source.revision());
        if let Some(output) = renderer.render(&source, &camera, (96, 64), None).unwrap() {
            assert_eq!(output.request(), Some(request));
            published = true;
            break;
        }
        finish(&context);
    }
    assert!(
        published,
        "continuous sample revisions must not starve a frozen request's pages"
    );
    assert_eq!(context.transfers().readback_bytes, before);
    // A later producer request with the same map ID is a distinct scope.
    owner.cancel_work();
    owner.request_render(map, 1);
    for _ in 0..8 {
        let work = owner
            .prepare_work(map, 2048, 2048, camera, Some((-8., 8., -8., 8.)))
            .unwrap()
            .unwrap();
        let n = (work.tile_size * work.tile_size) as usize;
        assert!(owner.finish_sample());
        owner
            .commit_current_tile(
                [150, 40, 20, 255].repeat(n),
                vec![4.; n],
                [150, 40, 20, 255].repeat(n),
                [128, 255, 128, 255].repeat(n),
                [200, 0, 255, 0].repeat(n),
            )
            .unwrap();
    }
    let source = owner.presentation_source(map, &camera).unwrap();
    assert_ne!(source.request(), Some(request));
    assert!(
        renderer
            .render(&source, &camera, (96, 64), None)
            .unwrap()
            .is_none()
    );
    let output = ready(&mut renderer, &source, &camera, (96, 64), None);
    assert_eq!(output.request(), source.request());
    let expected = owner.compose_rgba(map, 96, 64, &camera).unwrap();
    assert!(
        colors(&context, &output)
            .iter()
            .zip(expected)
            .all(|(a, b)| a.abs_diff(b) <= 1)
    );
}

#[test]
#[ignore = "actual GPU per-tile identity, changed-only upload and request retirement"]
fn progressive_small_sources_retain_unchanged_gpu_tiles() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let map = Uuid::new_v4();
    let mut camera = Camera3D::iso();
    camera.ortho_half_h = 1.;
    let mut owner = OrthographicBakeController::default();
    let mut renderer = OrthographicBakeRenderer::new(context.clone()).unwrap();
    let mut old_request = None;
    for request_phase in 0..2 {
        owner.cancel_work();
        owner.request_render(map, 2);
        let request = owner.active_request().unwrap();
        assert_ne!(old_request, Some(request));
        let mut committed = Vec::new();
        for _ in 0..2 {
            let work = owner
                .prepare_work(map, 256, 256, camera, Some((-2., 2., -2., 2.)))
                .unwrap()
                .unwrap();
            let n = (work.tile_size * work.tile_size) as usize;
            assert!(!owner.finish_sample());
            assert!(owner.finish_sample());
            owner
                .commit_current_tile(
                    [40 + request_phase * 50, 80, 120, 255].repeat(n),
                    vec![2.; n],
                    [40, 80, 120, 255].repeat(n),
                    [128, 255, 128, 255].repeat(n),
                    [200, 0, 255, 0].repeat(n),
                )
                .unwrap();
        }
        let work = owner
            .prepare_work(map, 256, 256, camera, Some((-2., 2., -2., 2.)))
            .unwrap()
            .unwrap();
        let n = (work.tile_size * work.tile_size) as usize;
        owner
            .preview_current_tile(&[60, 90, 130, 255].repeat(n))
            .unwrap();
        let source = owner.presentation_source(map, &camera).unwrap();
        assert_eq!(source.positions().count(), 3);
        for at in source.positions() {
            committed.push((at, source.tile_revision(at.0, at.1).unwrap()));
        }
        let before = context.transfers();
        let output = ready(&mut renderer, &source, &camera, (96, 64), None);
        assert_eq!(output.request(), Some(request));
        let uploaded = context.transfers().uploaded_bytes - before.uploaded_bytes;
        let channels = n as u64 * 24 * 3;
        assert!(
            (channels..channels + 4096).contains(&uploaded),
            "new request needs all three original tiles plus small dispatch metadata: {uploaded}"
        );
        finish(&context);
        for sample in 0..4 {
            let before = context.transfers();
            let rgba = match sample {
                1 => [0; 4],
                2 => [20, 30, 40, 128],
                _ => [70 + sample, 100, 140, 255],
            };
            owner.preview_current_tile(&rgba.repeat(n)).unwrap();
            let source = owner.presentation_source(map, &camera).unwrap();
            let unchanged = committed
                .iter()
                .filter(|(at, id)| source.tile_revision(at.0, at.1) == Some(*id))
                .count();
            assert_eq!(
                unchanged, 2,
                "only the current preview tile changes identity"
            );
            if sample == 0 {
                let held: Vec<_> = (0..context.source_pool_usage().submission_limit)
                    .map(|_| context.try_source_submission(0).unwrap())
                    .collect();
                let previous = renderer
                    .render(&source, &camera, (96, 64), None)
                    .unwrap()
                    .unwrap();
                assert!(renderer.waiting());
                assert_eq!(previous.source_revision(), output.source_revision());
                assert_eq!(previous.request(), Some(request));
                let mut moved = camera;
                moved.pos += camera.right * 0.1;
                assert!(
                    renderer
                        .render(&source, &moved, (96, 64), None)
                        .unwrap()
                        .is_none(),
                    "a pending new projection cannot inherit the prior image"
                );
                drop(held);
            }
            let output = ready(&mut renderer, &source, &camera, (96, 64), None);
            let uploaded = context.transfers().uploaded_bytes - before.uploaded_bytes;
            let channels = n as u64 * 24;
            assert!(
                (channels..channels + 4096).contains(&uploaded),
                "only one changed tile plus small dispatch metadata may upload: {uploaded}"
            );
            assert_eq!(context.transfers().readback_bytes, before.readback_bytes);
            let expected = owner.compose_rgba(map, 96, 64, &camera).unwrap();
            assert!(
                colors(&context, &output)
                    .iter()
                    .zip(expected)
                    .all(|(a, b)| a.abs_diff(b) <= 1)
            );
            let expected = owner.compose_depth(map, 96, 64, &camera).unwrap();
            assert!(
                depths(&context, &output)
                    .into_iter()
                    .zip(expected)
                    .all(|(a, b)| a == b || (a - b).abs() < 1e-5)
            );
            let usage = context.source_pool_usage();
            assert!(usage.bytes <= usage.byte_limit);
        }
        old_request = Some(request);
    }
}

fn solid_gpu(context: &GpuContext, size: (u32, u32), bytes: [u8; 4]) -> Arc<GpuImage> {
    let image = GpuImage::new(
        context,
        ImageDescriptor {
            width: size.0,
            height: size.1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            color_space: ColorSpace::Srgb,
            alpha_mode: AlphaMode::Premultiplied,
        },
    )
    .unwrap();
    let c = bytes.map(|v| f64::from(v) / 255.);
    let mut encoder = context.device.create_command_encoder(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("GPU-only overlay fixture"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &image.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: c[0],
                        g: c[1],
                        b: c[2],
                        a: c[3],
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
    context.queue.submit(Some(encoder.finish()));
    image
}

#[test]
#[ignore = "requires native GPU; real dynamic surface/depth composition and stale-output guards"]
fn dynamic_surfaces_occlude_actors_preserve_guides_and_clear_without_readback_storage() {
    use crate::{SceneViews, ViewId};
    use renderkit::ImageRegistry;
    use scenevm::{
        Atom, Chunk, DynamicObject, GeoId, Poly3D, RenderMode, SharedAtlas,
        dynamic::DynamicMeshVertex, prelude::Vec2, surface_output::SurfaceSelection,
    };
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let size = (64, 64);
    let mut images = ImageRegistry::new(context.clone());
    let mut scenes = SceneViews::new(&images, SharedAtlas::new(256, 256));
    let view = ViewId::new();
    scenes.create(view, size).unwrap();
    scenes
        .configure_editor(view, &rusterix::Assets::default())
        .unwrap();
    let camera = Camera3D::builder()
        .with_kind(scenevm::CameraKind::OrthoIso)
        .with_pos(Vec3::new(0., 0., 4.))
        .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
        .with_ortho_half_h(1.);
    {
        let scene = scenes.scene_mut(view).unwrap();
        scene.vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
        scene.vm.execute(Atom::SetCamera3D { camera });
        let mut chunk = Chunk::new(Vec2::zero(), 8);
        // Static geometry is closer than the actor, but Dynamic surface policy
        // must still report the actor's own depth rather than the static cube.
        chunk.add_3d(Poly3D::cube(
            GeoId::Sector(1),
            scene.white,
            Vec3::new(0., 0., 2.),
            1.,
        ));
        scene.vm.execute(Atom::AddChunk {
            id: Uuid::new_v4(),
            chunk,
        });
        let vertices =
            [(-0.8, -0.8), (0.8, -0.8), (0.8, 0.8), (-0.8, 0.8)].map(|(x, y)| DynamicMeshVertex {
                position: Vec3::new(x, y, 0.),
                uv: Vec2::zero(),
                normal: Vec3::unit_z(),
            });
        scene.vm.execute(Atom::AddDynamic {
            object: DynamicObject::mesh(
                GeoId::Item(7),
                scene.white,
                vertices.to_vec(),
                vec![0, 1, 2, 0, 2, 3],
            ),
        });
    }
    let displayed = scenes.render(view, 1, &mut images).unwrap().unwrap();
    let surface = scenes
        .render_surface_output(displayed, SurfaceSelection::Dynamic)
        .unwrap()
        .unwrap();
    assert_eq!(surface.revision(), displayed.revision);
    assert!(
        !scenes
            .scene(view)
            .unwrap()
            .vm
            .has_surface_readback_storage()
    );
    assert!(!scenes.has_cpu_target_storage(view).unwrap());
    assert_eq!(scenes.scene(view).unwrap().vm.render_target_cpu_bytes(), 0);
    let bake_image = solid_gpu(&context, size, [20, 40, 60, 128]);
    let overlay_image = solid_gpu(&context, size, [100, 20, 10, 128]);
    let depth = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GPU depth fixture"),
        size: u64::from(size.0 * size.1 * 4),
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let module = context.device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl("@group(0) @binding(0) var<storage,read_write> depth:array<f32>; @compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>) {if id.x<4096u {depth[id.x]=select(3.,5.,id.x%64u>=32u);}}".into()),
    });
    let pipeline = context
        .device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let bind = context
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: depth.as_entire_binding(),
            }],
        });
    let mut encoder = context.device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        pass.dispatch_workgroups(64, 1, 1);
    }
    context.queue.submit(Some(encoder.finish()));
    let mut bake = OrthographicBakeOutput {
        image: bake_image,
        depth,
        source_revision: Uuid::new_v4(),
        request: None,
        size,
        depth_enabled: true,
    };
    let mut compositor = OrthographicBakeOverlay::new(context.clone()).unwrap();
    let composed = compositor
        .render(&bake, &overlay_image, &surface, [40, 50, 60, 255])
        .unwrap();
    assert_eq!(context.transfers().readback_bytes, 0);
    assert_eq!(context.transfers().capture_requests, 0);
    let capture = |image: Arc<GpuImage>| {
        let mut capture = ReadbackTicket::capture(&context, image).unwrap();
        finish(&context);
        capture
            .try_finish(&context)
            .unwrap()
            .unwrap()
            .pixels()
            .to_vec()
    };
    let pixels = capture(composed);
    let at = |pixels: &[u8], x, y| {
        pixels[(y * size.0 as usize + x) * 4..(y * size.0 as usize + x) * 4 + 4].to_vec()
    };
    assert_eq!(
        at(&pixels, 16, 32),
        [40, 65, 90, 255],
        "baked depth hides the actor"
    );
    assert_eq!(
        at(&pixels, 48, 32),
        [120, 52, 55, 255],
        "actor in front of baked depth"
    );
    assert_eq!(
        at(&pixels, 1, 1),
        [120, 52, 55, 255],
        "guides without dynamic geometry remain visible"
    );
    bake.depth_enabled = false;
    let no_depth = capture(
        compositor
            .render(&bake, &overlay_image, &surface, [40, 50, 60, 255])
            .unwrap(),
    );
    assert_eq!(
        at(&no_depth, 16, 32),
        [120, 52, 55, 255],
        "legacy bake has no depth occlusion"
    );
    bake.depth_enabled = true;
    // Asset requests copy only original channels and own their mapping storage.
    // Hold depth across same-target reuse and resize to verify snapshot lifetime.
    let before_asset = context.transfers();
    let mut asset_color = ReadbackTicket::capture_asset(&context, bake.image.clone()).unwrap();
    let mut asset_depth = surface.capture_asset_depth(&context).unwrap();
    let usage = context.capture_queue_usage();
    assert_eq!(usage.requests, 2, "color and depth share one staging queue");
    let full = context
        .reserve_capture(usage.byte_limit - usage.bytes)
        .unwrap();
    let rejected = context.transfers();
    assert!(surface.capture_asset_depth(&context).is_err());
    assert_eq!(context.transfers(), rejected);
    drop(full);
    let after_asset = context.transfers();
    let bytes = u64::from(size.0 * size.1) * (4 + 32);
    assert_eq!(
        after_asset.readback_bytes - before_asset.readback_bytes,
        bytes
    );
    assert_eq!(
        after_asset.asset_bake_readback_bytes - before_asset.asset_bake_readback_bytes,
        bytes
    );
    assert_eq!(
        after_asset.capture_requests - before_asset.capture_requests,
        2
    );
    assert_eq!(
        after_asset.asset_bake_readback_requests - before_asset.asset_bake_readback_requests,
        2
    );
    assert!(
        !scenes
            .scene(view)
            .unwrap()
            .vm
            .has_surface_readback_storage()
    );
    finish(&context);
    let color = asset_color.try_finish(&context).unwrap().unwrap();
    assert!(
        color
            .pixels()
            .chunks_exact(4)
            .all(|pixel| pixel == [20, 40, 60, 128])
    );
    assert!(asset_color.try_finish(&context).is_err());
    // The explicit compatibility capture still decodes the same surface IDs,
    // camera-linear depth and UVs after the GPU producer was factored out.
    let reference = scenes
        .scene_mut(view)
        .unwrap()
        .vm
        .paint_surface_buffer_gpu_dynamic_only(size.0, size.1)
        .unwrap();
    let center = &reference.pixels[(32 * size.0 + 32) as usize];
    assert!(center.valid);
    assert_eq!(center.geo_id, GeoId::Item(7));
    assert!((center.depth - 4.).abs() < 0.00001);
    assert_eq!(center.uv, [0., 0.]);
    assert!(
        scenes
            .scene(view)
            .unwrap()
            .vm
            .has_surface_readback_storage()
    );
    scenes
        .scene_mut(view)
        .unwrap()
        .vm
        .execute(Atom::ClearDynamics);
    assert!(
        scenes
            .render_surface_output(displayed, SurfaceSelection::Dynamic)
            .unwrap()
            .is_none()
    );
    let newer = scenes.render(view, 2, &mut images).unwrap().unwrap();
    assert_ne!(newer.revision, displayed.revision);
    let empty = scenes
        .render_surface_output(newer, SurfaceSelection::Dynamic)
        .unwrap()
        .unwrap();
    let cleared = capture(
        compositor
            .render(&bake, &overlay_image, &empty, [40, 50, 60, 255])
            .unwrap(),
    );
    assert_eq!(
        at(&cleared, 16, 32),
        [120, 52, 55, 255],
        "empty query must clear the old dynamic footprint"
    );
    assert!(
        scenes
            .scene(view)
            .unwrap()
            .vm
            .has_surface_readback_storage(),
        "only the explicit compatibility capture requested this optional storage"
    );
    let static_surface = scenes
        .render_surface_output(newer, SurfaceSelection::Static)
        .unwrap()
        .unwrap();
    assert!(
        compositor
            .render(&bake, &overlay_image, &static_surface, [40, 50, 60, 255])
            .is_err()
    );
    scenes.resize(view, (83, 49), &mut images).unwrap();
    assert!(
        scenes
            .render_surface_output(newer, SurfaceSelection::Dynamic)
            .unwrap()
            .is_none()
    );
    let resized = scenes.render(view, 3, &mut images).unwrap().unwrap();
    let resized_surface = scenes
        .render_surface_output(resized, SurfaceSelection::Dynamic)
        .unwrap()
        .unwrap();
    assert_eq!(resized_surface.size(), (83, 49));
    assert!(
        compositor
            .render(&bake, &overlay_image, &resized_surface, [40, 50, 60, 255])
            .is_err()
    );
    assert!(
        !scenes
            .scene(view)
            .unwrap()
            .vm
            .has_surface_readback_storage()
    );
    finish(&context);
    let captured = asset_depth.try_finish(&context).unwrap().unwrap();
    assert_eq!(captured.size, size);
    assert_eq!(captured.revision, displayed.revision);
    assert_eq!(captured.selection, SurfaceSelection::Dynamic);
    assert!((captured.depth[(32 * size.0 + 32) as usize] - 4.).abs() < 0.00001);
    assert!(captured.depth[0].is_infinite());
    assert!(asset_depth.try_finish(&context).is_err());
    let mut empty = resized_surface.capture_asset_depth(&context).unwrap();
    finish(&context);
    assert!(
        empty
            .try_finish(&context)
            .unwrap()
            .unwrap()
            .depth
            .iter()
            .all(|depth| depth.is_infinite())
    );
    // A retired ticket can be dropped while its async map is pending. A new
    // request still succeeds, and a replacement device cannot consume its data.
    drop(resized_surface.capture_asset_depth(&context).unwrap());
    let other = futures::executor::block_on(GpuContext::headless()).unwrap();
    assert!(resized_surface.capture_asset_depth(&other).is_err());
    let mut retired = resized_surface.capture_asset_depth(&context).unwrap();
    assert!(retired.try_finish(&other).is_err());
    drop(retired);
    let mut next = resized_surface.capture_asset_depth(&context).unwrap();
    finish(&context);
    assert!(next.try_finish(&context).unwrap().is_some());
    assert!(
        !scenes
            .scene(view)
            .unwrap()
            .vm
            .has_surface_readback_storage()
    );
    // Unaligned rows must preserve real geometry, not merely empty pixels.
    let padded_surface = scenes
        .render_surface_output(resized, SurfaceSelection::Static)
        .unwrap()
        .unwrap();
    let mut padded_capture = padded_surface.capture_asset_depth(&context).unwrap();
    finish(&context);
    let actual = padded_capture.try_finish(&context).unwrap().unwrap();
    assert_eq!(actual.size, (83, 49));
    assert!(actual.depth.iter().any(|depth| depth.is_finite()));
    let scene = scenes.scene_mut(view).unwrap();
    scene.vm.set_active_vm(0);
    let expected = scene.vm.paint_surface_buffer_gpu(83, 49).unwrap();
    for (depth, pixel) in actual.depth.iter().zip(expected.pixels.iter()) {
        if pixel.valid {
            assert!((depth - pixel.depth).abs() < 0.00001);
        } else {
            assert!(depth.is_infinite());
        }
    }
    // Owned pending tickets can be retired even before loss notification has
    // reached the host. Destroying their exclusive buffers is idempotent.
    let mut ready_depth = padded_surface.capture_asset_depth(&context).unwrap();
    finish(&context);
    let lost_color = ReadbackTicket::capture_asset(&context, bake.image.clone()).unwrap();
    let lost_depth = padded_surface.capture_asset_depth(&context).unwrap();
    context.device.destroy();
    drop((lost_color, lost_depth));
    for _ in 0..100 {
        let _ = context.poll();
        if context.is_lost() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(
        context.is_lost(),
        "wait for the actual device-loss notification"
    );
    assert!(ready_depth.try_finish(&context).is_err());
    drop(ready_depth);
    for _ in 0..100 {
        let _ = context.poll();
        if context.capture_queue_usage().requests == 0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(context.capture_queue_usage().bytes, 0);
    assert_eq!(context.capture_queue_usage().requests, 0);
}
