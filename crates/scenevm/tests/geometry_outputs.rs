#![cfg(feature = "gpu")]
use renderkit::{GpuContext, ReadbackTicket, wgpu};
use scenevm::{
    Atom, Camera3D, CameraKind, Chunk, GeoId, Poly2D, Poly3D, RenderMode, SceneVM, SharedAtlas,
    prelude::{Vec2, Vec3, Vec4},
    render_output::SceneRenderTarget,
};

fn chunk(tile: uuid::Uuid, three_d: bool, count: u32) -> Chunk {
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    for index in 0..count {
        let offset = if index == 0 {
            0.
        } else {
            100. + index as f32 * 2.
        };
        if three_d {
            chunk.add_3d(Poly3D::cube(
                GeoId::Sector(index),
                tile,
                Vec3::new(offset, 0., 0.),
                1.4,
            ));
        } else {
            chunk.add(Poly2D::poly(
                GeoId::Unknown(index),
                tile,
                vec![
                    [4. + offset, 1.],
                    [12. + offset, 1.],
                    [12. + offset, 7.],
                    [4. + offset, 7.],
                ],
                vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
                vec![(0, 1, 2), (0, 2, 3)],
            ));
        }
    }
    chunk
}

#[test]
#[ignore = "native GPU; raster/compute geometry lease and capacity ownership"]
fn geometry_growth_mode_switches_and_retirement_keep_visible_contents() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    for three_d in [false, true] {
        let atlas = SharedAtlas::new(64, 64);
        let tile = uuid::Uuid::new_v4();
        let mut scene = SceneVM::new_with_context_and_atlas(context.clone(), 17, 9, atlas);
        scene.execute(Atom::AddTile {
            id: tile,
            width: 1,
            height: 1,
            frames: vec![vec![210, 30, 20, 255]],
            material_frames: None,
        });
        scene.set_background_lighting(false);
        scene.execute(Atom::SetBackground(Vec4::new(0., 0., 0., 1.)));
        scene.execute(Atom::SetGP3(Vec4::one()));
        scene.execute(Atom::SetGP8(Vec4::new(0., 0., 1., 1.)));
        scene.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 2.2)));
        scene.execute(Atom::BuildAtlas);
        scene.execute(Atom::SetCamera3D {
            camera: Camera3D::builder()
                .with_kind(CameraKind::OrthoIso)
                .with_pos(Vec3::new(0., 0., 4.))
                .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
                .with_ortho_half_h(1.2),
        });
        let id = uuid::Uuid::new_v4();
        let modes = if three_d {
            [
                RenderMode::Raster3D,
                RenderMode::Compute3D,
                RenderMode::Raster3D,
            ]
        } else {
            [
                RenderMode::Raster2D,
                RenderMode::Compute2D,
                RenderMode::Raster2D,
            ]
        };
        let mut target = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
        for mode in modes {
            scene.execute(Atom::SetRenderMode(mode));
            let mut baseline = None;
            for count in [1, 48, 1] {
                scene.execute(Atom::AddChunk {
                    id,
                    chunk: chunk(tile, three_d, count),
                });
                let output = scene.render_gpu(&mut target, count as u64).unwrap();
                assert!(context.working_buffer_pool_usage().live_bytes > 0);
                let allocations = context.working_buffer_pool_usage().allocations;
                let warm = scene.render_gpu(&mut target, 100 + count as u64).unwrap();
                assert_eq!(
                    context.working_buffer_pool_usage().allocations,
                    allocations,
                    "{mode:?} warm geometry"
                );
                assert_eq!(context.capture_queue_usage().bytes, 0);
                let mut ticket = ReadbackTicket::capture(&context, warm.image).unwrap();
                context
                    .device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(std::time::Duration::from_secs(20)),
                    })
                    .unwrap();
                let pixels = ticket
                    .try_finish(&context)
                    .unwrap()
                    .unwrap()
                    .pixels()
                    .to_vec();
                assert!(
                    pixels
                        .chunks_exact(4)
                        .any(|p| p[0] as u32 > p[1] as u32 + 20),
                    "{mode:?} visible geometry"
                );
                if let Some(baseline) = &baseline {
                    assert_eq!(
                        &pixels, baseline,
                        "{mode:?}: hidden geometry changes must retain visible pixels"
                    );
                } else {
                    baseline = Some(pixels);
                }
                drop(output);
            }
        }
        drop((scene, target));
        context.set_working_buffer_cache_limits(0, 0);
        assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
        assert_eq!(context.working_buffer_pool_usage().live_allocations, 0);
        context.set_working_buffer_cache_limits(128 * 1024 * 1024, 64);
    }
    assert!(!SceneVM::compatibility_gpu_initialized());
}
