use super::*;
use scenevm::{Atom, OrganicBillboardInstance, OrganicBillboardSprite, RenderMode};

#[test]
#[ignore = "native GPU; live pose ownership during multi-frame source staging"]
fn live_source_staging_preserves_pose_revision_and_scope() {
    let context = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(context.clone());
    let mut views = SceneViews::new(&images, SharedAtlas::new(64, 64));
    let id = ViewId::new();
    views.create(id, (32, 32)).unwrap();
    let session = Uuid::new_v4();
    let map = Uuid::new_v4();
    let make_scope = || GamePaintScope {
        description: (session, map, rusterix::PlayerCamera::D3Iso, [0; 17]),
        stamps: None,
        map_revision: 0,
        assets_revision: 0,
    };
    let scene = views.scene_mut(id).unwrap();
    scene.vm.execute(Atom::SetRenderMode(RenderMode::Raster3D));
    let mut instances = vec![
        OrganicBillboardInstance {
            center: [100., 0., 0.],
            width: 1.,
            height: 1.,
            sprite_index: 0,
            flags: 0,
        };
        10_000
    ];
    instances[0].center = [0.; 3];
    scene.vm.execute(Atom::SetOrganicBillboards {
        sprites: vec![OrganicBillboardSprite {
            width: 256,
            height: 256,
            rgba: vec![180, 90, 45, 255].repeat(256 * 256).into(),
        }],
        instances,
    });
    views.set_game_paint_scope(id, make_scope()).unwrap();
    let mut first = true;
    let mut output = None;
    for revision in 100..200 {
        views.begin_surface_paint_upload_frame();
        // The host advances simulation while this prepared pose stays owned.
        // A repeated preparation must not restart its partially packed packet.
        if !first {
            assert!(views.keep_game_paint_frame(id, &make_scope()).unwrap());
        }
        output = views.render(id, revision, &mut images).unwrap();
        assert!(views.scene_source_packed_cpu_bytes() <= 512 * 1024);
        assert!(views.scene_source_uploaded_bytes() <= 512 * 1024);
        if output.is_some() {
            break;
        }
        assert!(views.scene_preparation_pending(id).unwrap());
        assert!(views.has_pending_surface_paint_frames());
        first = false;
    }
    assert!(!first, "large source must yield before publication");
    assert_eq!(output.unwrap().content_revision, 100);
    assert!(!views.has_cpu_target_storage(id).unwrap());
    assert!(!views.keep_game_paint_frame(id, &make_scope()).unwrap());
    // New immutable stamp backing invalidates scope even with identical recipes.
    let changed = GamePaintScope {
        stamps: Some(Arc::new(rusterix::stamp_paint::StampPaint {
            stamps: vec![],
        })),
        ..make_scope()
    };
    assert!(!views.keep_game_paint_frame(id, &changed).unwrap());
    assert!(views.views[&id].pending_content_revision.is_none());
    // Explicit scene mutation revokes the prepared scope for the next caller.
    views.scene_mut(id).unwrap();
    assert!(views.views[&id].game_scope.is_none());
}
