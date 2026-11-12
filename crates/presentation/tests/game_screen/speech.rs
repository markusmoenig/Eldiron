use super::*;
use rusterix::{Item, PlayerCamera, ServerState, speech::SpeechTarget};
use theframework::platform::{InputEvent, PointerButton};
fn setup() -> (GameSession, GameScreenController, GameFonts) {
    let mut session = fixture(true);
    session.server.state = ServerState::Running;
    session.project.time = theframework_core::TheTime::new_time(12, 0).unwrap();
    session
        .project
        .config
        .push_str("\n[say]\nduration=1.0\nwarning='#cc224480'\nbackground_color='#10203099'\n");
    let mut item = Item::new();
    item.id = 1;
    item.position = Vec3::new(2., 0., 2.5);
    session.project.regions[0].map.items.push(item);
    session.project.regions[0].map.entities[0].position = Vec3::new(2., 0., 2.5);
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    for game in screen.elements.game_widgets.values_mut() {
        game.camera = PlayerCamera::D2;
        game.top_left = Vec2::zero();
        game.grid_size = 32.;
    }
    let fonts = GameFonts::default();
    (session, screen, fonts)
}
fn prepare(session: &GameSession, screen: &mut GameScreenController, fonts: &mut GameFonts) {
    screen
        .prepare(&session.view(), fonts, ContentRevision(1), 0)
        .unwrap();
}
fn receive(session: &GameSession, screen: &mut GameScreenController, text: &str) {
    screen.receive(
        &session.view(),
        &RegionEvents {
            says: vec![(Some(1), None, text.into(), "warning".into())],
            ..Default::default()
        },
    );
}
#[test]
fn native_speech_renders_once_per_view_reuses_shaping_and_tracks_live_sender_without_capturing_input()
 {
    let (mut session, mut screen, mut fonts) = setup();
    prepare(&session, &mut screen, &mut fonts);
    assert!(screen.speech.messages().is_empty());
    receive(&session, &mut screen, "Hi");
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(screen.speech.messages().len(), 1);
    let before = screen.speech.visible_bounds().collect::<Vec<_>>();
    assert_eq!(before.len(), 2);
    let built = screen.speech.layouts_built();
    for _ in 0..5 {
        prepare(&session, &mut screen, &mut fonts);
    }
    assert_eq!(screen.speech.layouts_built(), built);
    assert_eq!(screen.speech.messages().len(), 1);
    session.project.regions[0].map.entities[0].position.x += 0.5;
    prepare(&session, &mut screen, &mut fonts);
    for (id, old) in before {
        let moved = screen
            .speech
            .visible_bounds()
            .find(|(v, _)| *v == id)
            .unwrap()
            .1;
        assert!(moved.x > old.x + 10.);
        let input = screen
            .input(
                &session.view(),
                &mut fonts,
                &InputEvent::PointerButton {
                    button: PointerButton::Primary,
                    pressed: true,
                    position: [moved.x + 2., moved.y + 2.],
                },
            )
            .unwrap();
        assert!(
            !input.input.consumed,
            "ordinary speech does not invent an interactive/modal target"
        );
    }
    assert_eq!(screen.speech.layouts_built(), built);
    fonts.text_mut();
    prepare(&session, &mut screen, &mut fonts);
    assert!(screen.speech.layouts_built() > built);
}
#[test]
fn native_speech_local_durations_pause_expiry_hidden_views_and_region_stop_lifecycle() {
    let (mut session, mut screen, mut fonts) = setup();
    let id = *screen.elements.game_widgets.keys().min().unwrap();
    screen
        .elements
        .game_widgets
        .get_mut(&id)
        .unwrap()
        .toml_str
        .push_str("\n[say]\nduration=0.25\n");
    receive(&session, &mut screen, "Hi");
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(screen.speech.visible_bounds().count(), 2);
    session.server.state = ServerState::Paused;
    for _ in 0..20 {
        prepare(&session, &mut screen, &mut fonts);
    }
    assert_eq!(screen.speech.visible_bounds().count(), 2);
    session.project.time.seconds = 15;
    prepare(&session, &mut screen, &mut fonts);
    let visible = screen.speech.visible_bounds().collect::<Vec<_>>();
    assert_eq!(visible.len(), 1);
    assert_ne!(visible[0].0, id);
    let other = visible[0].0;
    screen
        .elements
        .widgets_to_hide
        .push(screen.elements.game_widgets[&other].name.clone());
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(screen.speech.visible_bounds().count(), 0);
    screen.elements.widgets_to_hide.clear();
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(screen.speech.visible_bounds().count(), 1);
    session.project.time.minutes += 1;
    prepare(&session, &mut screen, &mut fonts);
    assert!(screen.speech.messages().is_empty());
    receive(&session, &mut screen, "again");
    prepare(&session, &mut screen, &mut fonts);
    session.project.regions[0].map.id = Uuid::new_v4();
    prepare(&session, &mut screen, &mut fonts);
    assert!(screen.speech.messages().is_empty());
    receive(&session, &mut screen, "again");
    prepare(&session, &mut screen, &mut fonts);
    session.server.state = ServerState::Off;
    prepare(&session, &mut screen, &mut fonts);
    assert!(screen.speech.messages().is_empty());
}
#[test]
fn native_speech_preserves_typed_sender_ids_exact_unicode_and_view_local_styles() {
    let (session, mut screen, mut fonts) = setup();
    let text = "Hi e\u{301}\r\nمرحبا\n東京";
    screen.receive(
        &session.view(),
        &RegionEvents {
            says: vec![
                (Some(1), None, text.into(), "warning".into()),
                (None, Some(1), "Item".into(), "".into()),
            ],
            ..Default::default()
        },
    );
    prepare(&session, &mut screen, &mut fonts);
    assert_eq!(screen.speech.messages().len(), 2);
    assert_eq!(screen.speech.messages()[0].target, SpeechTarget::Entity(1));
    assert_eq!(screen.speech.messages()[1].target, SpeechTarget::Item(1));
    assert_eq!(screen.speech.messages()[0].text.as_ref(), text);
    assert_eq!(screen.speech.visible_bounds().count(), 4);
    for (_, bounds) in screen.speech.visible_bounds() {
        assert!(
            bounds.x >= 0.
                && bounds.y >= 0.
                && bounds.x + bounds.width <= 400.
                && bounds.y + bounds.height <= 240.
        );
    }
    screen.elements.layout.screen_responsive = true;
    screen.resize((640, 480));
    assert_eq!(screen.speech.visible_bounds().count(), 0);
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn native_speech_gpu_composes_each_view_styles_with_cached_swash_and_no_bubble_images() {
    let (session, mut screen, mut fonts) = setup();
    let ids = screen
        .elements
        .game_widgets
        .keys()
        .copied()
        .collect::<Vec<_>>();
    screen
        .elements
        .game_widgets
        .get_mut(&ids[0])
        .unwrap()
        .toml_str
        .push_str("\n[say]\nwarning='#22dd88ff'\nbackground_enabled=false\n");
    receive(&session, &mut screen, "Hi e\u{301}\r\nمرحبا\n東京");
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    let mut counts = None;
    for _ in 0..2 {
        prepare(&session, &mut screen, &mut fonts);
        screen
            .prepare_game_views(&session.view(), &mut views, &mut registry)
            .unwrap();
        let list = {
            let mut frame = images.begin_frame(&mut registry).unwrap();
            screen
                .paint(&session.view(), &mut fonts, &mut frame)
                .unwrap()
        };
        assert_eq!(screen.speech.visible_bounds().count(), 2);
        let mut ordered = ids.clone();
        ordered.sort();
        for (index, id) in ordered.iter().enumerate() {
            let image = screen.view_images()[id].image;
            let start = list
                .ops
                .iter()
                .position(|op| matches!(op,PaintOp::Image {image: handle,..} if *handle == image))
                .unwrap();
            let end = ordered
                .get(index + 1)
                .map(|next| {
                    let image = screen.view_images()[next].image;
                    list.ops
                        .iter()
                        .position(
                            |op| matches!(op,PaintOp::Image {image: handle,..} if *handle == image),
                        )
                        .unwrap()
                })
                .unwrap_or(list.ops.len());
            let expected = if *id == ids[0] {
                [34., 221., 136., 255.]
            } else {
                [204., 34., 68., 128.]
            }
            .map(|v| v / 255.);
            assert!(
                list.ops[start + 1..end]
                    .iter()
                    .any(|op| matches!(op,PaintOp::GlyphRun {color,..} if *color == expected)),
                "speech must be composited with its own game view before a later overlapping view"
            );
        }
        for color in [[34., 221., 136., 255.], [204., 34., 68., 128.]] {
            let expected = color.map(|v| v / 255.);
            assert!(
                list.ops
                    .iter()
                    .any(|op| matches!(op,PaintOp::GlyphRun {color,..} if *color == expected))
            );
        }
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
        if let Some(before) = counts {
            assert_eq!(now, before);
        }
        counts = Some(now);
        assert!(now.0 > 0);
        assert_eq!(gpu.transfers().readback_bytes, 0);
        for game in screen.elements.game_widgets.values() {
            assert!(game.buffer.pixels().is_empty() && game.upscale_buffer.pixels().is_empty());
        }
    }
    let pixels = capture(&gpu, target.clone());
    if let Some(file) = std::env::var_os("ELDIRON_NATIVE_SPEECH_CAPTURE") {
        let mut encoder = png::Encoder::new(std::fs::File::create(file).unwrap(), 600, 360);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    // Actual overlap, not just a list-order assertion: a front view's opaque
    // image must cover speech owned by the view behind it.
    let mut ordered = ids.clone();
    ordered.sort();
    for (index, id) in ordered.iter().enumerate() {
        let game = screen.elements.game_widgets.get_mut(id).unwrap();
        game.rect = Rect::new(16., 48., 168., 112.);
        let mut source = game.toml_str.parse::<toml::Table>().unwrap();
        source.insert(
            "say".into(),
            toml::Value::Table(
                [
                    (
                        "warning".into(),
                        toml::Value::String(
                            if index == 0 { "#22dd88ff" } else { "#cc224480" }.into(),
                        ),
                    ),
                    ("background_enabled".into(), toml::Value::Boolean(false)),
                ]
                .into_iter()
                .collect(),
            ),
        );
        game.toml_str = toml::to_string(&source).unwrap();
    }
    let front = screen.elements.game_widgets[&ordered[1]].name.clone();
    let green = |pixels: &[u8]| {
        pixels
            .chunks_exact(4)
            .filter(|p| p[1] as u16 > p[0] as u16 + 24 && p[1] as u16 > p[2] as u16 + 24)
            .count()
    };
    screen.elements.widgets_to_hide.push(front.clone());
    for covered in [false, true] {
        if covered {
            screen
                .elements
                .widgets_to_hide
                .retain(|name| name != &front);
        }
        prepare(&session, &mut screen, &mut fonts);
        screen
            .prepare_game_views(&session.view(), &mut views, &mut registry)
            .unwrap();
        let list = {
            let mut frame = images.begin_frame(&mut registry).unwrap();
            screen
                .paint(&session.view(), &mut fonts, &mut frame)
                .unwrap()
        };
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
        let pixels = capture(&gpu, target.clone());
        if covered {
            assert_eq!(
                green(&pixels),
                0,
                "front game image must occlude the rear view's speech"
            );
        } else {
            assert!(
                green(&pixels) > 10,
                "rear speech is visible before covering its view"
            );
        }
    }
}
