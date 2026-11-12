use super::*;
use theframework::platform::{InputEvent, PointerButton};

fn render(
    session: &GameSession,
    screen: &mut GameScreenController,
    fonts: &mut GameFonts,
    views: &mut SceneViews,
    registry: &mut ImageRegistry,
    images: &mut ScreenImageCache,
    renderer: &mut PaintRenderer,
    target: &GpuImage,
) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
    screen
        .prepare_game_views(&session.view(), views, registry)
        .unwrap();
    let list = {
        let mut sources = images.begin_frame(registry).unwrap();
        screen.paint(&session.view(), fonts, &mut sources).unwrap()
    };
    renderer
        .render(
            &list,
            registry,
            &target.view,
            target.descriptor.format,
            (600, 360),
            1.5,
            [0.; 4],
        )
        .unwrap();
    screen.acknowledge_world_frame(&session.view(), Some(views));
}
#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn owned_item_drag_blits_its_source_over_game_views_and_emits_one_transfer_without_cpu_composition()
{
    let mut session = fixture(true);
    session.server.state = rusterix::ServerState::Running;
    let tile = Tile::from_texture(Texture::from_color([0, 255, 255, 255]));
    let tile_id = tile.id;
    session.assets.tiles.insert(tile_id, tile);
    let mut item = rusterix::Item::new();
    item.id = 7;
    item.set_attribute("source", Value::Source(PixelSource::TileId(tile_id)));
    session.project.regions[0].map.entities[0].inventory = vec![Some(item), None];
    let map = &mut session.project.screens.values_mut().next().unwrap().map;
    sector(
        map,
        20,
        Rect::new(16., 176., 48., 48.),
        "[ui]\nrole='button'\ninventory_index=0\ndrag_drop=true\nshow_icon=false",
    );
    sector(
        map,
        21,
        Rect::new(80., 176., 48., 48.),
        "[ui]\nrole='button'\ninventory_index=1\ndrag_drop=true\nshow_icon=false",
    );
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let mut fonts = GameFonts::default();
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    let down = InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: [30., 190.],
    };
    let response = screen.input(&session.view(), &mut fonts, &down).unwrap();
    assert!(response.cancel_controls && response.effects.is_empty());
    assert!(screen.route_world_first(&InputEvent::PointerMoved {
        position: [70., 130.]
    }));
    screen.world_input(
        &session.view(),
        Some(&views),
        &InputEvent::PointerMoved {
            position: [70., 130.],
        },
        false,
    );
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    let uploaded = gpu.transfers().uploaded_bytes;
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    assert_eq!(
        gpu.transfers().uploaded_bytes,
        uploaded,
        "the slot and ghost share retained source artwork"
    );
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert!(screen.elements.game_widgets.values().all(
        |widget| widget.buffer.pixels().is_empty() && widget.upscale_buffer.pixels().is_empty()
    ));
    let pixels = capture(&gpu, target.clone());
    let offset = (195 * 600 + 105) * 4;
    assert_eq!(&pixels[offset..offset + 4], &[0, 255, 255, 255]);
    let file = std::fs::File::create("/private/tmp/eldiron-native-owned-item-drag.png").unwrap();
    let mut encoder = png::Encoder::new(file, 600, 360);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    let up = InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position: [100., 190.],
    };
    let result = screen.world_input(&session.view(), Some(&views), &up, false);
    assert_eq!(
        result.effects,
        vec![eldiron_presentation::ScreenEffect::Action(
            rusterix::EntityAction::MoveItem {
                item_id: 7,
                owner_entity_id: Some(1),
                target_entity_id: Some(1),
                to_inventory_index: Some(1),
                to_equipped_slot: None
            }
        )]
    );
    assert!(
        screen
            .world_input(&session.view(), Some(&views), &up, false)
            .effects
            .is_empty()
    );
    assert_eq!(
        session.project.regions[0].map.entities[0].inventory[0]
            .as_ref()
            .unwrap()
            .id,
        7,
        "presentation emits an action; runtime state is authoritative"
    );
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut views,
        &mut registry,
        &mut images,
        &mut renderer,
        &target,
    );
    assert_ne!(
        &capture(&gpu, target)[offset..offset + 4],
        &[0, 255, 255, 255]
    );
}
