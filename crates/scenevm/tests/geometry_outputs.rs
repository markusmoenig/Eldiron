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

#[test]
#[ignore = "native GPU; derived SDF table allocation and dummy ownership"]
fn sdf_table_growth_and_empty_sentinels_retire_with_the_scene() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let mut scene =
        SceneVM::new_with_context_and_atlas(context.clone(), 17, 9, SharedAtlas::new(64, 64));
    let tile = uuid::Uuid::new_v4();
    scene.execute(Atom::AddTile {
        id: tile,
        width: 1,
        height: 1,
        frames: vec![vec![210, 30, 20, 255]],
        material_frames: None,
    });
    scene.execute(Atom::BuildAtlas);
    let rect = scene.atlas_sdf_uv4(&tile, 0).unwrap();
    scene.execute(Atom::SetRenderMode(RenderMode::Sdf));
    scene.execute(Atom::SetGP3(Vec4::new(0.5, 0.5, 0.25, 0.15)));
    scene.execute(Atom::SetBackground(Vec4::new(0., 0., 0., 1.)));
    let mut target = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    let mut baseline = None;
    for count in [0, 1, 48, 1, 0] {
        scene.execute(Atom::SetSdfData(vec![rect; count]));
        let output = scene.render_gpu(&mut target, count as u64).unwrap();
        assert_eq!(
            scene
                .vm
                .gpu
                .as_ref()
                .unwrap()
                .sdf_data_ssbo
                .as_ref()
                .unwrap()
                .size(),
            count.max(1) as u64 * 16
        );
        let allocations = context.working_buffer_pool_usage().allocations;
        scene.render_gpu(&mut target, 100 + count as u64).unwrap();
        assert_eq!(context.working_buffer_pool_usage().allocations, allocations);
        let mut ticket = ReadbackTicket::capture(&context, output.image).unwrap();
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
        if count == 0 {
            assert!(pixels.chunks_exact(4).all(|p| p == [0, 0, 0, 255]));
        } else {
            assert!(
                pixels
                    .chunks_exact(4)
                    .any(|p| p[0] as u32 > p[1] as u32 + 20)
            );
            if let Some(baseline) = &baseline {
                assert_eq!(&pixels, baseline);
            } else {
                baseline = Some(pixels);
            }
        }
    }
    drop((scene, target));
    context.set_working_buffer_cache_limits(0, 0);
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
    assert_eq!(context.working_buffer_pool_usage().live_allocations, 0);
}

#[test]
#[ignore = "native GPU; private Raster3D attachment group ownership"]
fn raster_attachment_groups_survive_eviction_msaa_changes_and_resize() {
    let context = pollster::block_on(GpuContext::headless()).unwrap();
    let make = |color: Vec<u8>| {
        let mut scene =
            SceneVM::new_with_context_and_atlas(context.clone(), 17, 9, SharedAtlas::new(64, 64));
        let tile = uuid::Uuid::new_v4();
        scene.execute(Atom::AddTile {
            id: tile,
            width: 1,
            height: 1,
            frames: vec![color],
            material_frames: None,
        });
        scene.execute(Atom::BuildAtlas);
        scene.execute(Atom::SetRenderMode(RenderMode::Raster3D));
        scene.execute(Atom::SetRaster3DMsaaSamples(1));
        scene.execute(Atom::SetGP3(Vec4::one()));
        scene.execute(Atom::SetGP8(Vec4::new(0., 0., 1., 1.)));
        scene.execute(Atom::SetGP9(Vec4::new(0., 0., 1., 2.2)));
        scene.set_background_lighting(false);
        scene.execute(Atom::SetCamera3D {
            camera: Camera3D::builder()
                .with_kind(CameraKind::OrthoIso)
                .with_pos(Vec3::new(0., 0., 4.))
                .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
                .with_ortho_half_h(1.2),
        });
        scene.execute(Atom::AddChunk {
            id: uuid::Uuid::new_v4(),
            chunk: chunk(tile, true, 1),
        });
        scene
    };
    let capture = |image| {
        let mut ticket = ReadbackTicket::capture(&context, image).unwrap();
        context
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(20)),
            })
            .unwrap();
        ticket
            .try_finish(&context)
            .unwrap()
            .unwrap()
            .pixels()
            .to_vec()
    };
    let mut a = make(vec![210, 30, 20, 255]);
    let mut b = make(vec![20, 30, 210, 255]);
    let mut ta = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    let mut tb = SceneRenderTarget::new(context.clone(), 17, 9).unwrap();
    let red = a.render_gpu(&mut ta, 1).unwrap();
    let blue = b.render_gpu(&mut tb, 1).unwrap();
    assert_eq!(context.transfers().readback_bytes, 0);
    let expected_red = capture(red.image.clone());
    let expected_blue = capture(blue.image.clone());
    assert!(
        expected_red
            .chunks_exact(4)
            .any(|p| p[0] as u32 > p[2] as u32 + 30)
    );
    assert!(
        expected_blue
            .chunks_exact(4)
            .any(|p| p[2] as u32 > p[0] as u32 + 30)
    );
    let ga = a.vm.gpu.as_ref().unwrap();
    let gb = b.vm.gpu.as_ref().unwrap();
    assert_ne!(ga.raster3d_scene_tex, gb.raster3d_scene_tex);
    assert_ne!(ga.raster3d_depth_tex, gb.raster3d_depth_tex);
    assert_ne!(ga.raster3d_shadow_tex, gb.raster3d_shadow_tex);
    context.set_target_cache_limits(0, 0);
    for samples in [1, 4, 1] {
        a.execute(Atom::SetRaster3DMsaaSamples(samples));
        let frame = a.render_gpu(&mut ta, samples as u64).unwrap();
        let pixels = capture(frame.image);
        let allocations = context.target_pool_usage().allocations;
        let warm = a.render_gpu(&mut ta, 10 + samples as u64).unwrap();
        assert_eq!(context.target_pool_usage().allocations, allocations);
        assert_eq!(capture(warm.image), pixels);
        assert_eq!(capture(blue.image.clone()), expected_blue);
    }
    assert_eq!(capture(red.image.clone()), expected_red);
    let mut larger = SceneRenderTarget::new(context.clone(), 23, 11).unwrap();
    let enlarged = a.render_gpu(&mut larger, 20).unwrap();
    assert_ne!(enlarged.image.texture, red.image.texture);
    assert_eq!(capture(red.image.clone()), expected_red);
    drop((a, b, ta, tb, larger, enlarged));
    context.set_working_buffer_cache_limits(0, 0);
    assert_eq!(context.working_buffer_pool_usage().live_bytes, 0);
    assert_eq!(context.target_pool_usage().live_allocations, 2);
    assert_eq!(context.target_pool_usage().live_bytes, 2 * 784);
    drop((red, blue));
    assert_eq!(context.target_pool_usage().live_bytes, 0);
}
