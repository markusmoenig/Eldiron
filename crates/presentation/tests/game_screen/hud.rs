use super::item_pointer::render;
use super::*;
use eldiron_presentation::ScreenEffect;
use theframework::platform::{InputEvent, PointerButton};
#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn native_container_composes_template_tiles_swash_title_quantities_and_child_drag_without_a_cpu_panel()
 {
    let mut session = fixture(true);
    session.server.state = rusterix::ServerState::Running;
    let tile = Tile::from_texture(Texture::from_color([0, 255, 255, 255]));
    let tile_id = tile.id;
    session.assets.tiles.insert(tile_id, tile);
    let mut frame = Tile::from_texture(Texture::from_color([32, 64, 48, 255]));
    frame.alias = "Panel; Frame".into();
    session.assets.tiles.insert(frame.id, frame);
    session.assets.rules = "[ui.container_templates.default]\ncolumns=3\nslot_size=32\ngap=4\npadding=8\ntitle=true\n[ui.container_templates.default.tiles]\ncenter='panel'".into();
    let mut child = rusterix::Item::new();
    child.id = 8;
    child.set_stack_quantity(17);
    child.set_attribute("source", Value::Source(PixelSource::TileId(tile_id)));
    let mut bag = rusterix::Item::new();
    bag.id = 7;
    bag.max_capacity = 12;
    bag.set_attribute("name", Value::Str("Bag 東京".into()));
    bag.container = Some(vec![child]);
    session.project.regions[0].map.entities[0].inventory = vec![Some(bag), None];
    sector(
        &mut session.project.screens.values_mut().next().unwrap().map,
        20,
        Rect::new(120., 176., 48., 48.),
        "[ui]\nrole='button'\ninventory_index=1\ndrag_drop=true\nshow_icon=false",
    );
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::ToggleContainer {
            region: session.view().map.unwrap().id,
            item: 7,
            owner: 1,
            anchor: theframework::paint::Rect::new(190., 40., 20., 20.),
        },
    );
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
    let counts = (
        gpu.transfers().uploaded_bytes,
        renderer.glyphs.rasterizations,
        screen.source_builds(),
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
    assert_eq!(
        (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
            screen.source_builds()
        ),
        counts
    );
    assert!(counts.1 > 0);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert!(screen.elements.game_widgets.values().all(
        |widget| widget.buffer.pixels().is_empty() && widget.upscale_buffer.pixels().is_empty()
    ));
    let down = InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: [246., 90.],
    };
    assert!(
        screen
            .input(&session.view(), &mut fonts, &down)
            .unwrap()
            .cancel_controls
    );
    screen.world_input(
        &session.view(),
        Some(&views),
        &InputEvent::PointerMoved {
            position: [180., 130.],
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
    assert_eq!(
        (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
            screen.source_builds()
        ),
        counts,
        "the HUD slot and dragged child share one prepared source"
    );
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let pixels = capture(&gpu, target.clone());
    let pixel = |x: usize, y: usize| &pixels[(y * 600 + x) * 4..(y * 600 + x) * 4 + 4];
    assert_eq!(
        pixel(369, 135),
        &[0, 255, 255, 255],
        "child icon in the clipped HUD grid"
    );
    assert_eq!(
        pixel(270, 195),
        &[0, 255, 255, 255],
        "child drag image over the left view"
    );
    assert_eq!(
        pixel(336, 150),
        &[32, 64, 48, 255],
        "authored frame alias is a source image, not a raster panel"
    );
    let mut encoder = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-container-hud.png").unwrap(),
        600,
        360,
    );
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
        position: [144., 200.],
    };
    assert_eq!(
        screen
            .world_input(&session.view(), Some(&views), &up, false)
            .effects,
        vec![ScreenEffect::Action(
            rusterix::EntityAction::MoveContainerItem {
                item_id: 8,
                container_item_id: 7,
                container_owner_entity_id: Some(1),
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
            .container
            .as_ref()
            .unwrap()
            .len(),
        1,
        "presentation only emits the command"
    );
}
