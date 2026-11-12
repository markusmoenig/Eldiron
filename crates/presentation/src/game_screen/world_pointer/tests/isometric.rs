use super::*;
use renderkit::{GpuContext, ImageRegistry};
use scenevm::SharedAtlas;
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn native_isometric_views_publish_only_acknowledged_targets_and_keep_the_displayed_pose() {
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut views = SceneViews::new(&images, SharedAtlas::new(64, 64));
    let (mut session, mut screen, mut fonts, key) = setup();
    let map = &mut session.project.regions[0].map;
    map.camera = rusterix::MapCamera::TwoDIso;
    let tile = rusterix::Tile::from_texture(rusterix::Texture::new(vec![80, 150, 70, 255], 1, 1));
    let tile_id = tile.id;
    session
        .assets
        .set_tiles([(tile.id, tile)].into_iter().collect());
    let ids = [(0., 0.), (4., 0.), (4., 4.), (0., 4.)].map(|(x, z)| map.add_vertex_at(x, z));
    for i in 0..4 {
        map.create_linedef(ids[i], ids[(i + 1) % 4]);
    }
    map.sectors[0].properties.set(
        "source",
        Value::Source(rusterix::PixelSource::TileId(tile_id)),
    );
    let widget = screen.elements.game_widgets.get_mut(&key).unwrap();
    widget.camera = PlayerCamera::D2Iso;
    widget.clamp_camera_to_map = false;
    widget.upscale = 2.;
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    screen
        .prepare_game_views(&session.view(), &mut views, &mut images)
        .unwrap();
    let widget = &screen.elements.game_widgets[&key];
    let at = widget.ground_to_screen(Vec2::new(1.5, 1.5));
    let point = [at.x, at.y];
    assert!(click(&session, &mut screen, point).is_empty());
    screen.acknowledge_world_frame(&session.view(), Some(&views));
    assert!(matches!(
        &click(&session, &mut screen, point)[..],
        [ScreenEffect::Action(EntityAction::EntityClicked(
            1,
            _,
            None
        ))]
    ));
    let map = &mut session.project.regions[0].map;
    map.entities[0].position.x = 2.5;
    map.entities.push(actor(2, 1.5, 1.5));
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 1)
        .unwrap();
    screen
        .prepare_game_views(&session.view(), &mut views, &mut images)
        .unwrap();
    // The GPU view has rendered a new pose, but the host has not acknowledged it.
    assert!(matches!(
        &click(&session, &mut screen, point)[..],
        [ScreenEffect::Action(EntityAction::EntityClicked(
            1,
            _,
            None
        ))]
    ));
    screen.acknowledge_world_frame(&session.view(), Some(&views));
    let at = screen.elements.game_widgets[&key].ground_to_screen(Vec2::new(1.5, 1.5));
    assert!(matches!(
        &click(&session, &mut screen, [at.x, at.y])[..],
        [ScreenEffect::Action(EntityAction::EntityClicked(
            2,
            _,
            None
        ))]
    ));
    screen.elements.game_widgets.get_mut(&key).unwrap().camera = PlayerCamera::D2;
    assert!(click(&session, &mut screen, [at.x, at.y]).is_empty());
    for image in screen.view_images.values() {
        assert!(!views.has_cpu_target_storage(image.view).unwrap());
    }
    assert_eq!(gpu.transfers().capture_requests, 0);
    assert_eq!(gpu.transfers().readback_bytes, 0);
}
