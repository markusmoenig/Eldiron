use eldiron_presentation::{SceneViews, ViewId};
use renderkit::{GpuContext, GpuImage, ImageDescriptor, ImageRegistry, ReadbackTicket, wgpu};
use scenevm::{
    Atom, GeoId, Poly2D, RenderMode, SharedAtlas,
    prelude::{Mat3, Vec2, Vec4},
};
use std::{sync::Arc, time::Duration};
use theframework::{
    paint::{PaintList, Rect, Sampling},
    render_wgpu::PaintRenderer,
};
use theframework_core::Uuid;
fn finish(ctx: &GpuContext, mut ticket: ReadbackTicket) -> Vec<u8> {
    ctx.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    ticket.try_finish(ctx).unwrap().unwrap().pixels().to_vec()
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn views_compose_independently_and_keep_handles_until_target_replacement() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let atlas = SharedAtlas::new(64, 64);
    let tile = Uuid::new_v4();
    atlas.add_tile(tile, 1, 1, vec![vec![255, 255, 255, 255]], vec![]);
    let mut views = SceneViews::new(&images, atlas.clone());
    let a = ViewId::new();
    let b = ViewId::new();
    views.create(a, (64, 48)).unwrap();
    views.create(b, (64, 48)).unwrap();
    for (id, background, offset) in [
        (a, Vec4::new(1., 0., 0., 1.), 0.),
        (b, Vec4::new(0., 1., 0., 1.), 20.),
    ] {
        let scene = views.scene_mut(id).unwrap();
        scene.vm.execute(Atom::SetRenderMode(RenderMode::Raster2D));
        scene.vm.execute(Atom::SetBackground(background));
        scene.vm.execute(Atom::SetGP3(Vec4::new(1., 1., 1., 1.)));
        scene.vm.execute(Atom::SetTransform2D(
            Mat3::identity().translated_2d(Vec2::new(offset, 0.)),
        ));
        scene.vm.execute(Atom::AddPoly {
            poly: Poly2D::quad(GeoId::Unknown(1), tile, [16., 16.], 8., 0, true),
        });
    }
    assert!(Arc::ptr_eq(
        &atlas.inner_arc(),
        &views.scene(a).unwrap().vm.shared_atlas().inner_arc()
    ));
    assert!(Arc::ptr_eq(
        &views.scene(a).unwrap().vm.shared_atlas().inner_arc(),
        &views.scene(b).unwrap().vm.shared_atlas().inner_arc()
    ));
    assert!(
        views
            .scene_mut(a)
            .unwrap()
            .vm
            .set_layer_ping_pong_enabled(0, true)
    );
    let aa = views.render(a, 4, &mut images).unwrap().unwrap();
    let bb = views.render(b, 4, &mut images).unwrap().unwrap();
    assert_ne!(aa.image, bb.image);
    assert_eq!(views.render(a, 4, &mut images).unwrap(), Some(aa));
    assert_eq!(views.render(b, 4, &mut images).unwrap(), Some(bb));
    assert!(!views.has_cpu_target_storage(a).unwrap() && !views.has_cpu_target_storage(b).unwrap());
    assert_eq!(gpu.transfers().capture_requests, 0);
    let old = images.get(aa.image).unwrap();
    let repeat = views.render(a, 5, &mut images).unwrap().unwrap();
    assert_eq!(aa.image, repeat.image);
    assert_eq!(repeat.revision, 2);
    assert!(Arc::ptr_eq(&old, &images.get(aa.image).unwrap()));
    let mut list = PaintList::default();
    list.image(aa.image, Rect::new(0., 0., 64., 48.), Sampling::Nearest);
    list.image(bb.image, Rect::new(64., 0., 64., 48.), Sampling::Nearest);
    let output = GpuImage::new(&gpu, ImageDescriptor::rgba(128, 48)).unwrap();
    let mut paint = PaintRenderer::new(gpu.clone()).unwrap();
    paint
        .render(
            &list,
            &images,
            &output.view,
            output.descriptor.format,
            (128, 48),
            1.,
            [0.; 4],
        )
        .unwrap();
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let pixels = finish(&gpu, ReadbackTicket::capture(&gpu, output).unwrap());
    let pixel = |x: usize, y: usize| &pixels[(y * 128 + x) * 4..(y * 128 + x + 1) * 4];
    assert_eq!(pixel(2, 2), [255, 0, 0, 255]);
    assert_eq!(pixel(66, 2), [0, 255, 0, 255]);
    assert_eq!(pixel(16, 16), [255, 255, 255, 255]);
    assert_eq!(pixel(100, 16), [255, 255, 255, 255]);
    assert_eq!(
        pixel(80, 16),
        [0, 255, 0, 255],
        "second view camera transform must not overwrite the first"
    );
    let capture = views.capture(a).unwrap();
    assert_eq!((capture.revision, capture.content_revision), (2, 5));
    views.resize(a, (0, 0), &mut images).unwrap();
    assert!(views.render(a, 6, &mut images).unwrap().is_none());
    assert!(views.output(a).is_none());
    views.resize(a, (64, 48), &mut images).unwrap();
    let resumed = views.render(a, 6, &mut images).unwrap().unwrap();
    assert_eq!(resumed.image, aa.image);
    views.resize(a, (31, 23), &mut images).unwrap();
    assert!(images.get(aa.image).is_none());
    let resized = views.render(a, 7, &mut images).unwrap().unwrap();
    assert_ne!(resized.image, aa.image);
    assert_eq!(resized.descriptor.width, 31);
    assert_eq!(views.output(b), Some(bb));
    let old_capture = finish(&gpu, capture.ticket);
    assert_eq!(&old_capture[..4], [255, 0, 0, 255]);
    let mut foreign = ImageRegistry::new(gpu.clone());
    assert!(views.render(b, 5, &mut foreign).is_err());
    assert!(views.remove(a, &mut images).unwrap());
    assert!(images.get(resized.image).is_none());
    assert_eq!(views.output(b), Some(bb));
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn cpu_atlas_sources_reupload_after_device_replacement() {
    let a = futures::executor::block_on(GpuContext::headless()).unwrap();
    let b = futures::executor::block_on(GpuContext::headless()).unwrap();
    let atlas = SharedAtlas::new(16, 16);
    let tile = Uuid::new_v4();
    atlas.add_tile(tile, 1, 1, vec![vec![10, 20, 30, 255]], vec![]);
    atlas.upload_to_gpu_in_domain(&a.device, &a.queue, a.generation());
    assert!(
        atlas
            .with_views(|image, _| image.context_generation == Some(a.generation()))
            .unwrap()
    );
    atlas.upload_to_gpu_in_domain(&b.device, &b.queue, b.generation());
    assert!(
        atlas
            .with_views(|image, _| image.context_generation == Some(b.generation()))
            .unwrap()
    );
    let mut registry = ImageRegistry::new(b.clone());
    let mut views = SceneViews::new(&registry, atlas);
    let id = ViewId::new();
    views.create(id, (8, 8)).unwrap();
    views
        .scene_mut(id)
        .unwrap()
        .vm
        .execute(Atom::SetRenderMode(RenderMode::Raster2D));
    views.render(id, 1, &mut registry).unwrap();
    let mut wrong = scenevm::render_output::SceneRenderTarget::new(a.clone(), 8, 8).unwrap();
    assert!(
        views
            .scene_mut(id)
            .unwrap()
            .vm
            .render_gpu(&mut wrong, 2)
            .is_err(),
        "separate instances must be rejected by domain, even if WGPU device IDs compare equal"
    );
    assert_eq!(
        a.transfers().readback_bytes + b.transfers().readback_bytes,
        0
    );
}
