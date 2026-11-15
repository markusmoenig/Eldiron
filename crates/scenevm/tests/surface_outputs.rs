#![cfg(feature = "gpu")]
use renderkit::{GpuContext, TargetDescriptor, TargetLease, wgpu};
use scenevm::{
    Atom, Camera3D, CameraKind, Chunk, GeoId, Poly3D, RenderMode, SceneVM, SharedAtlas,
    prelude::{Vec2, Vec3},
    render_output::SceneRenderTarget,
    surface_output::{SurfaceOutput, SurfaceSelection},
};

fn depth(context: &GpuContext, source: &SurfaceOutput) -> Vec<f32> {
    let mut capture = source.capture_asset_depth(context).unwrap();
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(20)),
        })
        .unwrap();
    capture.try_finish(context).unwrap().unwrap().depth
}
fn clear(context: &GpuContext, target: &TargetLease, color: wgpu::Color) {
    let mut encoder = context.device.create_command_encoder(&Default::default());
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.view(),
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
    }
    context.queue.submit([encoder.finish()]);
}

#[test]
#[ignore = "native GPU; pooled surface query output/resize/capture retirement"]
fn surface_query_outputs_pin_targets_after_resize_and_scene_retirement() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    context.set_target_cache_limits(1024 * 1024, 32);
    let atlas = SharedAtlas::new(64, 64);
    let tile = uuid::Uuid::new_v4();
    atlas.add_tile(tile, 1, 1, vec![vec![200, 50, 30, 255]], vec![]);
    let size = (17, 9);
    let mut scene = SceneVM::new_with_context_and_atlas(context.clone(), size.0, size.1, atlas);
    scene.execute(Atom::SetRenderMode(RenderMode::Raster3D));
    scene.set_background_lighting(false);
    scene.execute(Atom::SetCamera3D {
        camera: Camera3D::builder()
            .with_kind(CameraKind::OrthoIso)
            .with_pos(Vec3::new(0., 0., 4.))
            .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
            .with_ortho_half_h(1.2),
    });
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(Poly3D::cube(GeoId::Sector(1), tile, Vec3::zero(), 1.4));
    scene.execute(Atom::AddChunk {
        id: uuid::Uuid::new_v4(),
        chunk,
    });
    let mut target = SceneRenderTarget::new(context.clone(), size.0, size.1).unwrap();
    scene.render_gpu(&mut target, 1).unwrap();
    let before = context.target_pool_usage();
    let buffers = context.working_buffer_pool_usage();
    let first = scene
        .render_surface_gpu(&context, size, SurfaceSelection::Static, 1)
        .unwrap();
    assert_eq!(
        context.target_pool_usage().allocations,
        before.allocations + 3
    );
    assert_eq!(
        context.working_buffer_pool_usage().allocations,
        buffers.allocations + 1
    );
    let warm = context.target_pool_usage();
    let small = scene
        .render_surface_gpu(&context, size, SurfaceSelection::Static, 2)
        .unwrap();
    assert_eq!(
        first.world(),
        small.world(),
        "same-size queries reuse the producer target"
    );
    assert_eq!(context.target_pool_usage().allocations, warm.allocations);
    assert_eq!(
        context.working_buffer_pool_usage().allocations,
        buffers.allocations + 1
    );
    assert!(!scene.has_surface_readback_storage());
    assert_eq!(context.transfers().readback_bytes, 0);
    drop(first);
    let expected = depth(&context, &small);
    assert!(expected.iter().any(|value| value.is_finite()));
    let readback_before = context.transfers().readback_bytes;
    // Authoring capture charges padded staging before allocation. Refusal leaves
    // no retained storage or transfer, and retiring another job admits retry.
    let padded = (size.0 * 16).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let staging_bytes = u64::from(padded) * u64::from(size.1) * 2;
    let occupied = context
        .reserve_capture(context.capture_queue_usage().byte_limit - staging_bytes + 1)
        .unwrap();
    assert!(scene.paint_surface_buffer_gpu(size.0, size.1).is_none());
    assert_eq!(context.transfers().readback_bytes, readback_before);
    assert!(!scene.has_surface_readback_storage());
    drop(occupied);
    for request in 1..=2 {
        let captured = scene.paint_surface_buffer_gpu(size.0, size.1).unwrap();
        assert!(captured.pixels.iter().any(|pixel| pixel.valid));
        for (pixel, expected) in captured.pixels.iter().zip(&expected) {
            if pixel.valid {
                assert!((pixel.depth - expected).abs() < 0.00001);
                assert_eq!(pixel.geo_id, GeoId::Sector(1));
            } else {
                assert!(expected.is_infinite());
            }
        }
        assert_eq!(context.capture_queue_usage().bytes, 0);
        assert_eq!(context.capture_queue_usage().requests, 0);
        assert!(!scene.has_surface_readback_storage());
        assert_eq!(
            context.transfers().readback_bytes,
            readback_before + request * u64::from(size.0 * size.1) * 32
        );
    }
    assert!(scene.paint_surface_buffer_gpu(u32::MAX, size.1).is_none());
    let larger = scene
        .render_surface_gpu(&context, (23, 11), SurfaceSelection::Static, 3)
        .unwrap();
    assert_ne!(larger.world(), small.world());
    let held = small.clone();
    drop((small, larger, scene, target));
    let descriptor = TargetDescriptor {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        ..TargetDescriptor::color(size.0, size.1, wgpu::TextureFormat::Rgba32Float)
    };
    let other_world = context.acquire_target(descriptor).unwrap();
    let other_metadata = context.acquire_target(descriptor).unwrap();
    for other in [&other_world, &other_metadata] {
        assert_ne!(other.view(), held.world());
        assert_ne!(other.view(), held.metadata());
    }
    clear(&context, &other_world, wgpu::Color::RED);
    clear(&context, &other_metadata, wgpu::Color::BLUE);
    assert_eq!(
        depth(&context, &held),
        expected,
        "retired scene output stays intact"
    );
    drop((other_world, other_metadata));
    context.set_target_cache_limits(0, 0);
    context.set_working_buffer_cache_limits(0, 0);
    assert_eq!(context.target_pool_usage().live_allocations, 2);
    assert_eq!(
        context.target_pool_usage().live_bytes,
        u64::from(size.0 * size.1) * 32
    );
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
    drop(held);
    assert_eq!(context.target_pool_usage().live_bytes, 0);
    assert_eq!(context.target_pool_usage().live_allocations, 0);
    assert!(!SceneVM::compatibility_gpu_initialized());
}
