use super::*;
use renderkit::{ReadbackTicket, wgpu};
use scenevm::{
    Atom, Camera3D, CameraKind, Chunk, GeoId, Poly3D, SceneVM, SharedAtlas,
    prelude::{Vec2, Vec3},
};
use shared::{
    iso_paint::{IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner},
    surface_paint::pages::PaintPageIndex,
};
use theframework_core::Uuid;

fn finish(context: &GpuContext, mut ticket: ReadbackTicket) -> Vec<u8> {
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(20)),
        })
        .unwrap();
    ticket
        .try_finish(context)
        .unwrap()
        .unwrap()
        .pixels()
        .to_vec()
}

#[test]
#[ignore = "native GPU; private raster pressure/replay must match resident original paint"]
fn private_raster_pressure_matches_resident_frame() {
    pressure_fixture(RenderMode::Raster3D);
}
#[test]
#[ignore = "native GPU; private compute regions must restore their sample lease through pressure"]
fn private_compute_pressure_matches_resident_frame() {
    pressure_fixture(RenderMode::Compute3D);
}
fn pressure_fixture(mode: RenderMode) {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let size = (96, 64);
    let atlas = SharedAtlas::new(64, 64);
    let tile = Uuid::new_v4();
    atlas.add_tile(tile, 1, 1, vec![vec![200, 50, 30, 255]], vec![]);
    let geo = [7, 0, 0, 1];
    let mut cube = Poly3D::cube(GeoId::Sector(1), tile, Vec3::zero(), 1.6);
    // One visible face isolates spatial pressure. Overlapping cube faces can
    // shade several paint keys in one pixel before fixed-function depth reject;
    // that is a separate single-pixel capacity/occlusion policy case.
    cube.indices.retain(|(first, _, _)| (4..8).contains(first));
    cube.paint_surface_id = Some(geo);
    cube.paint_uvs = cube.uvs.iter().map(|uv| [uv[0] * 6., uv[1] * 2.]).collect();
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(cube);
    let scene = || {
        let mut scene = SceneHandler::with_vm(SceneVM::new_with_context_and_atlas(
            context.clone(),
            size.0,
            size.1,
            atlas.clone(),
        ));
        scene.vm.execute(Atom::SetRenderMode(mode));
        if mode == RenderMode::Compute3D {
            scene.vm.set_layer_ping_pong_enabled(0, true);
            scene.vm.set_layer_progressive_sample_index(0, Some(0));
        }
        scene.vm.execute(Atom::SetRaster3DMsaaSamples(4));
        scene.vm.set_background_lighting(false);
        scene.vm.execute(Atom::SetCamera3D {
            camera: Camera3D::builder()
                .with_kind(CameraKind::OrthoIso)
                .with_pos(Vec3::new(0., 0., 4.))
                .with_basis(Vec3::new(0., 0., -1.), Vec3::unit_y())
                .with_ortho_half_h(1.),
        });
        scene.vm.execute(Atom::AddChunk {
            id: Uuid::new_v4(),
            chunk: chunk.clone(),
        });
        scene
    };
    let mut paint = IsoPaintLayer::default();
    for at in 0..3 {
        let mut page = IsoPaintBakedChunk::new(IsoPaintOwner::Sector(1), geo, [at * 64, 0]);
        for pixel in page.color_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[30 + at as u8 * 75, 190, 60, 255]);
        }
        paint.baked_chunks.insert(at.to_string(), page);
    }
    let source = Arc::new(PaintPageIndex::from_layer(&paint, 1024).unwrap());
    let page_bytes = 64 * 64 * 8;
    let mut resident =
        PaintPageResidency::new(context.clone(), source.clone(), page_bytes * 3).unwrap();
    for at in 0..3 {
        resident
            .request(source.slot(geo, [at * 64, 0]).unwrap())
            .unwrap();
    }
    let mut reference = scene();
    reference
        .vm
        .set_paint_page_binding(&context, resident.binding().unwrap())
        .unwrap();
    let mut target = SceneRenderTarget::new(context.clone(), size.0, size.1).unwrap();
    let output = reference.vm.render_gpu(&mut target, 1).unwrap();
    let expected = finish(
        &context,
        ReadbackTicket::capture(&context, output.image).unwrap(),
    );
    assert!(
        expected
            .chunks_exact(4)
            .any(|pixel| pixel != &expected[..4]),
        "reference geometry must contribute pixels"
    );
    let mut preparation = crate::orthographic_bake::PaintPageResidencyPreparation::new(
        context.clone(),
        source,
        page_bytes * 2,
    )
    .unwrap();
    assert!(!preparation.ready());
    while !preparation.advance(1).unwrap() {}
    while !preparation.ready() {
        assert!(preparation.upload(64).unwrap() <= 64);
    }
    let pages = preparation.finish().unwrap();
    let mut frame = RasterPaintFrame::new(
        context.clone(),
        scene(),
        SceneRenderTarget::new(context.clone(), size.0, size.1).unwrap(),
        pages,
        17,
    )
    .unwrap();
    assert!(frame.output().is_none() && frame.completed_scene().is_none());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !matches!(frame.phase, Phase::Source(_)) {
        context.device.poll(wgpu::PollType::Poll).unwrap();
        assert_eq!(
            frame.advance_with_source_budget(0).unwrap(),
            RasterPaintAdvance::Pending
        );
        assert_eq!(frame.residency_usage().page_uploads, 0);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(
        frame.advance_with_source_budget(0).unwrap(),
        RasterPaintAdvance::Pending
    );
    assert_eq!(frame.residency_usage().page_uploads, 0);
    loop {
        context.device.poll(wgpu::PollType::Poll).unwrap();
        if frame.advance().unwrap() == RasterPaintAdvance::Complete {
            break;
        }
        assert!(frame.output().is_none() && frame.completed_scene().is_none());
        assert!(
            std::time::Instant::now() < deadline,
            "private raster frame did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(
        frame.subdivisions() > 0,
        "three visible pages must exceed two slots"
    );
    assert_eq!(frame.residency_usage().gpu_plane_bytes, page_bytes * 2);
    assert_eq!(frame.output().unwrap().revision, 17);
    let result = ReadbackTicket::capture(&context, frame.output().unwrap().image.clone()).unwrap();
    assert_eq!(
        expected,
        finish(&context, result),
        "paged raster original paint must match complete resident rendering"
    );
    let (scene, target, _, _) = frame.into_completed().unwrap();
    assert_eq!(scene.vm.render_target_cpu_bytes(), 0);
    assert!(!target.has_cpu_storage() && !target.has_readback_storage());
    assert!(!SceneVM::compatibility_gpu_initialized());
}
