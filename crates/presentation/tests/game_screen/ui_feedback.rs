use super::item_pointer::render;
use super::*;
use eldiron_presentation::ScreenEffect;
use rusterix::client::command::ClientCommandBinding;
use theframework::platform::{InputEvent, PointerButton};
fn pixel(pixels: &[u8], point: [f32; 2]) -> [u8; 4] {
    let i = ((point[1] * 1.5) as usize * 600 + (point[0] * 1.5) as usize) * 4;
    pixels[i..i + 4].try_into().unwrap()
}
#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn ui_only_drop_cursors_and_target_highlight_compose_above_two_views_with_one_retained_drag_source()
{
    let mut session = fixture(true);
    session.server.state = rusterix::ServerState::Running;
    let artwork = Tile::from_texture(Texture::from_color([0, 255, 255, 255]));
    let artwork_id = artwork.id;
    session.assets.tiles.insert(artwork_id, artwork);
    let mut cursor = |color: [u8; 4]| {
        let tile = Tile::from_texture(Texture::new(color.repeat(16), 4, 4));
        let id = tile.id;
        session.assets.tiles.insert(id, tile);
        id
    };
    let default = cursor([255, 0, 0, 255]);
    let hover = cursor([0, 255, 0, 255]);
    let pressed = cursor([0, 0, 255, 255]);
    let mut config = session.project.config.parse::<toml::Table>().unwrap();
    config
        .get_mut("viewport")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .insert("cursor_id".into(), default.to_string().into());
    session.project.config = toml::to_string(&config).unwrap();
    session.assets.config = session.project.config.clone();
    let mut item = rusterix::Item::new();
    item.id = 7;
    item.set_attribute("source", Value::Source(PixelSource::TileId(artwork_id)));
    session.project.regions[0].map.entities[0].inventory = vec![Some(item), None];
    let map = &mut session.project.screens.values_mut().next().unwrap().map;
    // This fixture keeps the two real views and substitutes its own authored
    // item/intent controls for the starter menu's Start/name fields.
    map.sectors.retain(|sector| !matches!(sector.id, 2 | 3));
    sector(
        map,
        20,
        Rect::new(20., 176., 40., 40.),
        "[ui]\nrole='button'\ninventory_index=0\ndrag_drop=true\nshow_icon=true",
    );
    sector(
        map,
        21,
        Rect::new(80., 176., 40., 40.),
        "[ui]\nrole='button'\ninventory_index=1\ndrag_drop=true\nshow_icon=false",
    );
    sector(
        map,
        22,
        Rect::new(320., 176., 64., 40.),
        "[ui]\nrole='button'\ncommand='intent.drop'\nlabel='Drop'\nshow_icon=false",
    );
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let intent = screen.elements.button_widgets.get_mut(&22).unwrap();
    intent.item_cursor_id = Some(hover);
    intent.item_clicked_cursor_id = Some(pressed);
    screen
        .activate_binding(&session.view(), ClientCommandBinding::Intent("drop".into()))
        .unwrap();
    let time = session.project.time;
    let mut fonts = GameFonts::default();
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    macro_rules! frame {
        () => {
            render(
                &session,
                &mut screen,
                &mut fonts,
                &mut views,
                &mut registry,
                &mut images,
                &mut renderer,
                &target,
            )
        };
    }
    frame!();
    // Authored/UI-only hosts need no world-query callback for owned-item cursors.
    let moved = screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [40., 196.],
            },
        )
        .unwrap();
    assert!(moved.input.redraw && moved.effects.is_empty());
    frame!();
    assert!(screen.uses_custom_cursor());
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let pixels = capture(&gpu, target.clone());
    assert_eq!(
        pixel(&pixels, [40., 196.]),
        [0, 255, 0, 255],
        "owned hover cursor"
    );
    let button = |position, pressed| InputEvent::PointerButton {
        button: PointerButton::Primary,
        position,
        pressed,
    };
    assert!(
        screen
            .input(&session.view(), &mut fonts, &button([40., 196.], true))
            .unwrap()
            .effects
            .is_empty()
    );
    frame!();
    let pixels = capture(&gpu, target.clone());
    assert_eq!(
        pixel(&pixels, [40., 196.]),
        [0, 0, 255, 255],
        "original pressed cursor"
    );
    let builds = screen.source_builds();
    let explicit_readback = gpu.transfers().readback_bytes;
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [100., 196.],
            },
        )
        .unwrap();
    assert_eq!(
        screen.drag_target_feedback(&session.view()),
        Some(Rect::new(80., 176., 40., 40.))
    );
    frame!();
    assert_eq!(
        screen.source_builds(),
        builds,
        "slot and ghost use the same immutable source lease"
    );
    let counts = (
        gpu.transfers().uploaded_bytes,
        renderer.glyphs.rasterizations,
        screen.source_builds(),
    );
    frame!();
    assert_eq!(
        (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
            screen.source_builds()
        ),
        counts
    );
    assert_eq!(gpu.transfers().readback_bytes, explicit_readback);
    let pixels = capture(&gpu, target.clone());
    assert_eq!(
        pixel(&pixels, [81., 180.]),
        [255, 236, 132, 255],
        "shared target border above authored chrome"
    );
    assert_eq!(
        pixel(&pixels, [92., 190.]),
        [0, 255, 255, 255],
        "opaque ghost paints above target tint"
    );
    assert_eq!(
        pixel(&pixels, [100., 196.]),
        [255, 0, 0, 255],
        "default cursor paints above the drag ghost"
    );
    let mut encoder = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-ui-feedback.png").unwrap(),
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
    assert!(screen.elements.game_widgets.values().all(
        |widget| widget.buffer.pixels().is_empty() && widget.upscale_buffer.pixels().is_empty()
    ));
    assert_eq!(
        screen
            .input(&session.view(), &mut fonts, &button([100., 196.], false))
            .unwrap()
            .effects,
        vec![ScreenEffect::Action(rusterix::EntityAction::MoveItem {
            item_id: 7,
            owner_entity_id: Some(1),
            target_entity_id: Some(1),
            to_inventory_index: Some(1),
            to_equipped_slot: None
        })]
    );
    assert!(screen.drag_target_feedback(&session.view()).is_none());
    assert!(
        screen
            .input(&session.view(), &mut fonts, &button([100., 196.], false))
            .unwrap()
            .effects
            .is_empty()
    );
    assert!(session.project.time == time);
}
