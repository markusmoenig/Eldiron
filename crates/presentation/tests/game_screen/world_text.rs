use super::*;
use rusterix::{
    Item, PlayerCamera,
    world_text::{TextAnchor, WorldText},
};
use theframework::platform::{InputEvent, LogicalKey, PhysicalKey, PointerButton};

fn setup() -> (GameSession, GameScreenController, GameFonts) {
    let mut session = fixture(true);
    let mut item = Item::new();
    item.id = 22;
    item.position = Vec3::new(2., 0., 2.5);
    item.set_attribute("height", Value::Float(0.1));
    session.project.regions[0].map.items.push(item);
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    for game in screen.elements.game_widgets.values_mut() {
        game.camera = PlayerCamera::D2;
        game.grid_size = 32.;
        game.top_left = Vec2::zero();
    }
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    (session, screen, fonts)
}
fn receive(screen: &mut GameScreenController, session: &GameSession, receiver: u32, text: &str) {
    screen.receive(
        &session.view(),
        &RegionEvents {
            world_text: vec![(
                receiver,
                WorldText {
                    anchor: TextAnchor::Item(22),
                    text: text.into(),
                },
            )],
            ..Default::default()
        },
    );
}
fn prepare(screen: &mut GameScreenController, session: &GameSession, fonts: &mut GameFonts) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
}
fn click(
    screen: &mut GameScreenController,
    session: &GameSession,
    fonts: &mut GameFonts,
    point: [f32; 2],
) {
    let down = screen
        .input(
            &session.view(),
            fonts,
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position: point,
            },
        )
        .unwrap();
    assert!(down.input.consumed && down.cancel_controls);
    assert!(
        down.effects.is_empty(),
        "reading a description cannot click the underlying world"
    );
    let up = screen
        .input(
            &session.view(),
            fonts,
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: false,
                position: [-50., -50.],
            },
        )
        .unwrap();
    assert!(up.input.consumed && up.effects.is_empty());
}
#[test]
fn native_world_text_is_private_tracks_its_item_and_reuses_shaping_until_source_or_fonts_change() {
    let (mut session, mut screen, mut fonts) = setup();
    assert!(screen.world_texts.visible_bounds().next().is_none());
    receive(&mut screen, &session, 999, "A door.");
    prepare(&mut screen, &session, &mut fonts);
    assert!(screen.world_texts.passages().is_empty());
    receive(&mut screen, &session, 1, "A door.");
    prepare(&mut screen, &session, &mut fonts);
    let first = screen.world_texts.visible_bounds().next().unwrap();
    let built = screen.world_texts.layouts_built();
    for _ in 0..4 {
        prepare(&mut screen, &session, &mut fonts);
    }
    assert_eq!(screen.world_texts.passages().len(), 1);
    assert_eq!(screen.world_texts.layouts_built(), built);
    session.project.regions[0].map.items[0].position.x += 1.;
    prepare(&mut screen, &session, &mut fonts);
    let moved = screen.world_texts.visible_bounds().next().unwrap();
    assert!(
        moved.x > first.x + 15.,
        "the placement follows the live item, not its initial snapshot"
    );
    assert_eq!(screen.world_texts.layouts_built(), built);
    fonts.text_mut();
    prepare(&mut screen, &session, &mut fonts);
    assert!(screen.world_texts.layouts_built() > built);
    click(
        &mut screen,
        &session,
        &mut fonts,
        [moved.x + 3., moved.y + 3.],
    );
    prepare(&mut screen, &session, &mut fonts);
    assert!(screen.world_texts.passages().is_empty());
    assert!(screen.world_texts.visible_bounds().next().is_none());
}
#[test]
fn native_world_text_paginates_shaped_unicode_lines_and_captures_dismissal_releases() {
    let (session, mut screen, mut fonts) = setup();
    let text = "A weathered door.\r\nمرحبا e\u{301}\n東京\n".repeat(12);
    receive(&mut screen, &session, 1, &text);
    prepare(&mut screen, &session, &mut fonts);
    assert_eq!(screen.world_texts.passages()[0].message.text, text);
    let pages = screen.world_texts.passages()[0].pages;
    assert!(pages > 1);
    screen.world_texts.advance(Duration::from_secs(60));
    prepare(&mut screen, &session, &mut fonts);
    assert_eq!(
        screen.world_texts.passages().len(),
        1,
        "explicit pages remain until read or dismissed"
    );
    for page in 0..pages {
        assert_eq!(screen.world_texts.passages()[0].page, page);
        let bounds = screen.world_texts.visible_bounds().next().unwrap();
        assert!(
            bounds.x >= 0.
                && bounds.y >= 0.
                && bounds.x + bounds.width <= 400.
                && bounds.y + bounds.height <= 240.
        );
        click(
            &mut screen,
            &session,
            &mut fonts,
            [bounds.x + 3., bounds.y + 3.],
        );
        prepare(&mut screen, &session, &mut fonts);
    }
    assert!(screen.world_texts.passages().is_empty());
    receive(&mut screen, &session, 1, &text);
    prepare(&mut screen, &session, &mut fonts);
    for pressed in [true, false] {
        let response = screen
            .input(
                &session.view(),
                &mut fonts,
                &InputEvent::Key {
                    physical: PhysicalKey::Escape,
                    logical: LogicalKey::Named(theframework::TheKeyCode::Escape),
                    pressed,
                    repeat: false,
                },
            )
            .unwrap();
        assert!(response.input.consumed && response.effects.is_empty());
    }
    assert!(screen.world_texts.passages().is_empty());
}
#[test]
fn native_world_text_expires_with_explicit_time_and_revokes_hidden_resized_or_other_region_hits() {
    let (session, mut screen, mut fonts) = setup();
    receive(&mut screen, &session, 1, "A door.");
    prepare(&mut screen, &session, &mut fonts);
    assert!(screen.world_texts.visible_bounds().next().is_some());
    screen.world_texts.advance(Duration::from_secs(30));
    prepare(&mut screen, &session, &mut fonts);
    assert!(screen.world_texts.passages().is_empty());
    receive(&mut screen, &session, 1, "A door.");
    prepare(&mut screen, &session, &mut fonts);
    let old = screen.world_texts.visible_bounds().next().unwrap();
    let hidden = screen
        .elements
        .game_widgets
        .values()
        .map(|g| g.name.clone())
        .collect::<Vec<_>>();
    screen.elements.widgets_to_hide.extend(hidden);
    prepare(&mut screen, &session, &mut fonts);
    assert!(screen.world_texts.visible_bounds().next().is_none());
    let input = screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed: true,
                position: [old.x + 3., old.y + 3.],
            },
        )
        .unwrap();
    assert!(!input.input.consumed);
    screen.elements.widgets_to_hide.clear();
    prepare(&mut screen, &session, &mut fonts);
    assert!(screen.world_texts.visible_bounds().next().is_some());
    screen.elements.layout.screen_responsive = true;
    assert!(screen.resize((640, 480)));
    assert!(screen.world_texts.visible_bounds().next().is_none());
    prepare(&mut screen, &session, &mut fonts);
    let mut project = session.project.clone();
    project.regions[0].map.id = Uuid::new_v4();
    let mut other = GameSession::new(project);
    other.current_map = "World".into();
    prepare(&mut screen, &other, &mut fonts);
    assert!(screen.world_texts.passages().is_empty());
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn native_world_text_gpu_uses_cached_swash_glyphs_without_hud_bitmaps_or_readback() {
    let (session, mut screen, mut fonts) = setup();
    receive(
        &mut screen,
        &session,
        1,
        "A weathered door.\r\nمرحبا e\u{301}\n東京",
    );
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    let mut counts = None;
    for _ in 0..2 {
        prepare(&mut screen, &session, &mut fonts);
        screen
            .prepare_game_views(&session.view(), &mut views, &mut registry)
            .unwrap();
        let list = {
            let mut frame = images.begin_frame(&mut registry).unwrap();
            screen
                .paint(&session.view(), &mut fonts, &mut frame)
                .unwrap()
        };
        assert!(screen.world_texts.visible_bounds().next().is_some());
        renderer
            .render(
                &list,
                &registry,
                &target.view,
                target.descriptor.format,
                (600, 360),
                1.5,
                [0.; 4],
            )
            .unwrap();
        let now = (
            renderer.glyphs.rasterizations,
            gpu.transfers().uploaded_bytes,
        );
        assert!(now.0 > 0);
        if let Some(counts) = counts {
            assert_eq!(now, counts);
        }
        counts = Some(now);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        for game in screen.elements.game_widgets.values() {
            assert!(game.buffer.pixels().is_empty() && game.upscale_buffer.pixels().is_empty());
        }
    }
    let pixels = capture(&gpu, target);
    if let Some(file) = std::env::var_os("ELDIRON_NATIVE_WORLD_TEXT_CAPTURE") {
        let mut encoder = png::Encoder::new(std::fs::File::create(file).unwrap(), 600, 360);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
}
