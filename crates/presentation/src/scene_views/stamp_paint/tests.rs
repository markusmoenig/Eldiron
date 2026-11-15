use super::*;
use crate::scene_views::GamePaintScope;
use shared::iso_paint::{IsoPaintLayer, IsoPaintPoint};
use shared::iso_paint_render::stamp_billboards::StampSpriteRequest;

#[test]
#[ignore = "native GPU; common view placement ownership and scope cancellation"]
fn stamp_placement_owner_keeps_pose_and_retires_changed_scope() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(context);
    let mut views = SceneViews::new(&images, SharedAtlas::new(64, 64));
    let id = ViewId::new();
    views.create(id, (32, 32)).unwrap();
    let session = Uuid::new_v4();
    let map = Uuid::new_v4();
    let original_source = Arc::new(rusterix::stamp_paint::StampPaint { stamps: Vec::new() });
    let make_scope = || GamePaintScope {
        description: (session, map, rusterix::PlayerCamera::D3Iso, [0; 17]),
        stamps: Some(original_source.clone()),
        map_revision: 0,
        assets_revision: 0,
    };
    let mut layer = IsoPaintLayer::default();
    layer.active_brush = "grass".into();
    layer.active_size = 1.;
    layer.add_stamp(IsoPaintPoint::new([8, 8], Some(vek::Vec3::zero()), None));
    let chunk = layer.chunks.values_mut().next().unwrap();
    chunk.stamps = vec![chunk.stamps[0].clone(); 8193].into();
    let mut prep = StampSpriteRequest::capture(&layer).prepare(None);
    while !prep.advance(1).unwrap() {}
    let pixels = Arc::new(prep.finish().unwrap());
    views
        .scene_mut(id)
        .unwrap()
        .vm
        .execute(scenevm::Atom::SetRenderMode(scenevm::RenderMode::Raster3D));
    views.set_game_paint_scope(id, make_scope()).unwrap();
    views.set_stamp_paint(id, Some((&layer, &pixels))).unwrap();
    assert!(!views.stamp_paint_ready(id).unwrap());
    assert!(views.stamp_placement_pending(id).unwrap());
    assert!(views.has_pending_surface_paint_frames());
    let mut output = None;
    let mut saw_cpu_work = false;
    for revision in 100..200 {
        views.begin_surface_paint_upload_frame();
        assert!(views.keep_game_paint_frame(id, &make_scope()).unwrap());
        views.prepare_scene_source_uploads(id).unwrap();
        saw_cpu_work |= views.scene_source_preparation_calls() > 0;
        output = views.render(id, revision, &mut images).unwrap();
        if output.is_some() {
            break;
        }
        assert!(views.scene_preparation_pending(id).unwrap());
    }
    assert!(saw_cpu_work);
    let output = output.expect("complete placement/source packet must publish");
    assert_eq!(output.content_revision, 100);
    assert!(views.stamp_paint_ready(id).unwrap());
    assert!(!views.has_cpu_target_storage(id).unwrap());
    layer.chunks.values_mut().next().unwrap().stamps[0].world = Some([2., 0., 0.]);
    views.set_stamp_paint(id, Some((&layer, &pixels))).unwrap();
    assert!(views.stamp_placement_pending(id).unwrap());
    let mut changed_scope = make_scope();
    changed_scope.description.3[0] = 1;
    assert!(!views.keep_game_paint_frame(id, &changed_scope).unwrap());
    assert!(!views.stamp_placement_pending(id).unwrap());
    assert!(!views.scene_preparation_pending(id).unwrap());
    assert!(!views.stamp_paint_ready(id).unwrap());
    assert_eq!(views.output(id).unwrap().revision, output.revision);
    views.set_game_paint_scope(id, changed_scope).unwrap();
    views.set_stamp_paint(id, Some((&layer, &pixels))).unwrap();
    assert!(views.stamp_placement_pending(id).unwrap());
    views.cancel_scene_source_uploads(id).unwrap();
    assert!(!views.stamp_placement_pending(id).unwrap());
    views.set_stamp_paint(id, None).unwrap();
    assert!(views.stamp_paint_ready(id).unwrap());
    assert!(views.views[&id].stamp_placements.is_none());
}
