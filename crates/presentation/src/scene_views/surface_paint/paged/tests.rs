use super::*;
use renderkit::{ImageRegistry, ReadbackTicket};
use scenevm::{
    Atom, Camera3D, CameraKind, Chunk, GeoId, Poly3D, SharedAtlas,
    prelude::{Vec2, Vec3},
};
use shared::iso_paint::{IsoPaintBakedChunk, IsoPaintLayer, IsoPaintOwner};

fn prepare(views: &mut SceneViews, id: ViewId, chunk: &Chunk) {
    let scene = views.scene_mut(id).unwrap();
    scene
        .vm
        .execute(Atom::SetRenderMode(scenevm::RenderMode::Raster3D));
    scene.vm.set_background_lighting(false);
    scene.vm.execute(Atom::SetRaster3DMsaaSamples(4));
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
}
fn render(
    views: &mut SceneViews,
    id: ViewId,
    images: &mut ImageRegistry,
    changing: bool,
) -> (ViewImage, Option<u64>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut first = None;
    for tick in 0.. {
        views.context().device.poll(wgpu::PollType::Poll).unwrap();
        views.begin_surface_paint_upload_frame();
        views.prepare_scene_source_uploads(id).unwrap();
        let revision = if changing { 100 + tick } else { 1 };
        if let Some(output) = views.render(id, revision, images).unwrap() {
            return (output, first);
        }
        if views.private_paint_frame_pending(id).unwrap() {
            first.get_or_insert(revision);
        }
        assert!(
            views.has_pending_surface_paint_frames(),
            "pending source/frame must keep the host scheduled"
        );
        assert!(
            std::time::Instant::now() < deadline,
            "live indexed paint did not complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    unreachable!()
}
fn capture(context: &GpuContext, image: Arc<GpuImage>) -> Vec<u8> {
    let mut ticket = ReadbackTicket::capture(context, image).unwrap();
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
#[ignore = "native GPU; canonical live indexed residency and content-revision publication"]
fn live_indexed_source_completes_with_frozen_content_revision() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(context.clone());
    let atlas = SharedAtlas::new(64, 64);
    let tile = Uuid::new_v4();
    atlas.add_tile(tile, 1, 1, vec![vec![200, 50, 30, 255]], vec![]);
    let geo = [7, 0, 0, 1];
    let mut plane = Poly3D::cube(GeoId::Sector(1), tile, Vec3::zero(), 1.6);
    plane.indices.retain(|(first, _, _)| (4..8).contains(first));
    plane.paint_surface_id = Some(geo);
    plane.paint_uvs = plane
        .uvs
        .iter()
        .map(|uv| [uv[0] * 6., uv[1] * 2.])
        .collect();
    let mut chunk = Chunk::new(Vec2::zero(), 8);
    chunk.add_3d(plane);
    let mut layer = IsoPaintLayer::default();
    for at in 0..3 {
        let mut page = IsoPaintBakedChunk::new(IsoPaintOwner::Sector(1), geo, [at * 64, 0]);
        for pixel in page.color_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[30 + at as u8 * 75, 190, 60, if at == 1 { 160 } else { 255 }]);
        }
        layer.baked_chunks.insert(at.to_string(), page);
    }
    let index = PaintPageIndex::from_layer(&layer, 1024).unwrap();
    let residents: Vec<_> = (0..3)
        .map(|at| (index.slot(geo, [at * 64, 0]).unwrap(), [at as u32 * 64, 0]))
        .collect();
    let source = Arc::new(SurfacePaint {
        key: 17,
        width: 192,
        height: 64,
        pages: residents
            .iter()
            .map(|(slot, origin)| {
                let page = index.page(*slot).unwrap();
                rusterix::surface_paint::SurfacePaintPage {
                    atlas_origin: *origin,
                    color_rgba: page.color.clone(),
                    material_rgba: page.material.clone(),
                }
            })
            .collect(),
        entries: index
            .resident_entries([192, 64], &residents)
            .unwrap()
            .into(),
        alpha_geo_ids: Arc::new(index.alpha_geo_ids().to_vec()),
    });
    let reference_id = ViewId::new();
    let mut reference = SceneViews::new(&images, atlas.clone());
    reference.create(reference_id, (96, 64)).unwrap();
    prepare(&mut reference, reference_id, &chunk);
    reference
        .set_surface_paint_source(reference_id, Some(&source))
        .unwrap();
    let expected = render(&mut reference, reference_id, &mut images, false).0;
    let expected = capture(&context, images.get(expected.image).unwrap().clone());
    assert!(
        expected
            .chunks_exact(4)
            .any(|pixel| pixel != &expected[..4]),
        "reference geometry must contribute pixels"
    );
    let id = ViewId::new();
    let mut views = SceneViews::new(&images, atlas);
    views
        .set_surface_paint_residency_limit(64 * 64 * 8 * 2)
        .unwrap();
    views.create(id, (96, 64)).unwrap();
    prepare(&mut views, id, &chunk);
    views.set_surface_paint_source(id, Some(&source)).unwrap();
    let (output, started) = render(&mut views, id, &mut images, true);
    assert_eq!(
        Some(output.content_revision),
        started,
        "publish the frozen request revision, not the latest tick"
    );
    assert_eq!(views.surface_paint_stats().gpu_plane_bytes, 64 * 64 * 8 * 2);
    assert!(!views.has_pending_surface_paint_frames());
    assert!(!views.has_cpu_target_storage(id).unwrap());
    assert_eq!(
        expected,
        capture(&context, images.get(output.image).unwrap().clone())
    );
    assert!(matches!(
        views.pick_game(output, [0.5; 2]),
        crate::ViewPick::Hit(_)
    ));
    views.remove(id, &mut images).unwrap();
    assert_eq!(views.surface_paint_stats().gpu_plane_bytes, 0);
    assert_eq!(views.surface_paint_stats().sources, 0);
    assert!(!SceneVM::compatibility_gpu_initialized());
}
