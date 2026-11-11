use eldiron_presentation::{
    GameFonts, GameScreenController, SceneViews,
    screen::{ContentRevision, ScreenImageCache, ScreenImages},
};
use eldiron_runtime::{GameSession, Project, RegionEvents};
use renderkit::{
    GpuContext, GpuImage, ImageDescriptor, ImageHandle, ImageRegistry, ReadbackTicket, wgpu,
};
use rusterix::{
    Entity, Map, PixelSource, Rect, Texture, Tile, Value, client::screen_model::ScreenLayout,
};
use scenevm::{
    SharedAtlas,
    prelude::{Vec2, Vec3},
};
use std::{sync::Arc, time::Duration};
use theframework::{paint::PaintOp, render_wgpu::PaintRenderer, text::TextSystem};
use theframework_core::Uuid;
fn layout() -> ScreenLayout {
    ScreenLayout {
        screen_responsive: false,
        reference_viewport: Vec2::new(400, 240),
        viewport: Vec2::new(400, 240),
        grid_size: 8.,
    }
}
fn sector(map: &mut Map, id: u32, rect: Rect, data: &str) -> Uuid {
    let layout = layout();
    let (sx, sy) = rusterix::utils::align_screen_to_grid(400., 240., layout.grid_size);
    let x = rect.x / layout.grid_size + sx;
    let y = rect.y / layout.grid_size + sy;
    let w = rect.width / layout.grid_size;
    let h = rect.height / layout.grid_size;
    let points =
        [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|(x, y)| map.add_vertex_at(x, y));
    let lines = (0..4)
        .map(|i| map.create_linedef_manual(points[i], points[(i + 1) % 4]))
        .collect();
    let mut sector = rusterix::Sector::new(id, lines);
    sector.name = format!("Widget{id}");
    sector.properties.set("data", Value::Str(data.into()));
    let key = sector.creator_id;
    map.sectors.push(sector);
    key
}
fn fixture(games: bool) -> GameSession {
    let mut project = Project::new();
    project.screens.clear();
    project.config = "[viewport]\nwidth=400\nheight=240\ngrid_size=8\nscreen_background='#101820'\n[renderer]\nbackend_2d='raster'\n[render]\nambient_strength=1.0".into();
    let id = Uuid::new_v4();
    let screen = project.screens.entry(id).or_default();
    screen.id = id;
    screen.map = Map::default();
    screen.map.name = "Main".into();
    sector(
        &mut screen.map,
        1,
        Rect::new(16., 8., 368., 32.),
        "[ui]\nrole='text'\ntext='Eldiron — العربية 東京'\nfont_size=20.0",
    );
    sector(
        &mut screen.map,
        2,
        Rect::new(16., 176., 180., 48.),
        "[ui]\nrole='button'\ncommand='game.start'\nlabel='Start'\nshow_icon=false\nbackground_color='#204060'",
    );
    sector(
        &mut screen.map,
        3,
        Rect::new(208., 176., 176., 48.),
        "[ui]\nrole='input'\nbind='start.name'\ndefault='Éowyn'",
    );
    if games {
        sector(
            &mut screen.map,
            10,
            Rect::new(16., 48., 168., 112.),
            "[ui]\nrole='game'\ngrid_size=32.0",
        );
        sector(
            &mut screen.map,
            11,
            Rect::new(208., 48., 176., 112.),
            "[ui]\nrole='game'\ngrid_size=32.0\nupscale=2.0",
        );
        sector(
            &mut screen.map,
            12,
            Rect::new(0., 0., 400., 240.),
            "[ui]\nrole='deco'\nlayer=-1\ncolor='#00000080'",
        );
    } else {
        sector(
            &mut screen.map,
            4,
            Rect::new(16., 48., 368., 112.),
            "[ui]\nrole='messages'\nfont_size=16.0\nportrait=false\nbackground_color='#202c38'",
        );
    }
    let mut font = shared::asset::Asset::new();
    font.name = "Roboto".into();
    font.buffer = shared::asset::AssetBuffer::Font(
        include_bytes!("../../theframework/embedded/fonts/SourceCodePro-Bold.ttf").to_vec(),
    );
    project.assets.insert(font.id, font);
    project.regions[0].name = "World".into();
    project.regions[0].map = Map::default();
    project.regions[0].map.name = "World".into();
    let mut player = Entity::default();
    player.id = 1;
    player.position = Vec3::new(1., 0., 1.);
    player.attributes.set("player", Value::Bool(true));
    player.attributes.set("name", Value::Str("Hero".into()));
    project.regions[0].map.entities.push(player);
    let mut session = GameSession::new(project);
    session.current_map = "World".into();
    session.assets.config = session.project.config.clone();
    let tile = Tile::from_texture(Texture::from_color([255, 32, 8, 255]));
    let tid = tile.id;
    session
        .assets
        .set_tiles([(tid, tile)].into_iter().collect());
    let map = &mut session.project.regions[0].map;
    let points = [(0., 0.), (2., 0.), (2., 2.), (0., 2.)].map(|(x, y)| map.add_vertex_at(x, y));
    for i in 0..4 {
        map.create_linedef(points[i], points[(i + 1) % 4]);
    }
    map.sectors[0]
        .properties
        .set("source", Value::Source(PixelSource::TileId(tid)));
    session
}
struct Images(u32);
impl ScreenImages for Images {
    fn rgba(&mut self, _: &str, _: u32, _: u32, _: &[u8]) -> Result<ImageHandle, String> {
        self.0 += 1;
        Ok(ImageHandle {
            owner: 1,
            slot: self.0,
            generation: 0,
        })
    }
}
fn capture(gpu: &GpuContext, target: Arc<GpuImage>) -> Vec<u8> {
    let mut read = ReadbackTicket::capture(gpu, target).unwrap();
    gpu.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(30)),
        })
        .unwrap();
    read.try_finish(gpu).unwrap().unwrap().into_pixels()
}
#[test]
fn native_screen_preparation_reflows_without_consuming_messages_or_replacing_chrome_fonts() {
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut controller =
        GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let mut events = RegionEvents::default();
    events
        .messages
        .push((None, None, 1, "Hello — 世界".into(), "status".into()));
    controller.receive(&session.view(), &events);
    for _ in 0..3 {
        controller
            .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
            .unwrap();
    }
    assert_eq!(controller.elements.messages_widgets[0].messages.len(), 1);
    let mut images = Images(0);
    let list = controller
        .paint(&session.view(), &mut fonts, &mut images)
        .unwrap();
    list.validate().unwrap();
    assert!(
        list.ops
            .iter()
            .any(|op| matches!(op, PaintOp::GlyphRun { .. }))
    );
    assert_eq!(
        images.0, 0,
        "text and chrome must not become raster source images"
    );
    for widget in controller.elements.text_widgets.values() {
        assert!(widget.buffer.pixels().is_empty());
        assert!(widget.font.is_none());
    }
    assert!(
        controller.elements.messages_widgets[0]
            .buffer
            .pixels()
            .is_empty()
    );
    let chosen = fonts.text_mut().layout("font", "Roboto", 18., None);
    assert_eq!(
        &*chosen.runs[0].font.data,
        include_bytes!("../../theframework/embedded/fonts/SourceCodePro-Bold.ttf")
    );
    let mut chrome = TextSystem::default();
    chrome.register_font_named(
        "Roboto",
        include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf").to_vec(),
    );
    let chrome = chrome.layout("font", "Roboto", 18., None);
    assert_ne!(chosen.runs[0].font.identity, chrome.runs[0].font.identity);
    assert_eq!(
        &*chrome.runs[0].font.data,
        include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf")
    );
}
#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn authored_screen_composes_two_independent_game_views_and_vector_layer_clips() {
    let session = fixture(true);
    let mut fonts = GameFonts::default();
    let mut controller =
        GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    for frame in 0..12 {
        controller
            .prepare(&session.view(), &mut fonts, ContentRevision(1), frame)
            .unwrap();
        controller
            .prepare_game_views(&session.view(), &mut views, &mut registry)
            .unwrap();
        let list = {
            let mut frame = images.begin_frame(&mut registry).unwrap();
            controller
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
        assert_eq!(
            list.ops
                .iter()
                .filter(|op| matches!(op, PaintOp::PushPathClip(..)))
                .count(),
            2
        );
        assert_eq!(controller.view_images().len(), 2);
        for widget in controller.elements.game_widgets.values() {
            assert!(widget.buffer.pixels().is_empty() && widget.upscale_buffer.pixels().is_empty());
        }
        for widget in controller.elements.deco_widgets.values() {
            assert!(widget.buffer.pixels().is_empty());
        }
        assert_eq!(gpu.transfers().capture_requests, 0);
        std::thread::sleep(Duration::from_millis(5));
    }
    let outputs = controller
        .view_images()
        .values()
        .copied()
        .collect::<Vec<_>>();
    assert_ne!(outputs[0].image, outputs[1].image);
    for output in &outputs {
        assert!(!views.has_cpu_target_storage(output.view).unwrap());
    }
    let pixels = capture(&gpu, target);
    for (left, right) in [(16usize, 184usize), (208, 384)] {
        let mut red = 0;
        for y in 48 * 3 / 2..160 * 3 / 2 {
            for x in left * 3 / 2..right * 3 / 2 {
                let p = &pixels[(y * 600 + x) * 4..(y * 600 + x + 1) * 4];
                if p[0] > 50 && u16::from(p[0]) > u16::from(p[1]) * 2 {
                    red += 1;
                }
            }
        }
        assert!(
            red > 500,
            "each native scene must contain visible world geometry: {red}"
        );
    }
    let p = &pixels[(20 * 600 + 590) * 4..(20 * 600 + 591) * 4];
    assert_eq!(
        p,
        &[16, 24, 32, 255],
        "base outside both views covers the negative dimmer using vector clips"
    );
    let mut png = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-game-screen.png").unwrap(),
        600,
        360,
    );
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
    controller.elements.widgets_to_hide = vec!["Widget10".into(), "Widget11".into()];
    controller
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 12)
        .unwrap();
    controller
        .prepare_game_views(&session.view(), &mut views, &mut registry)
        .unwrap();
    assert!(controller.view_images().is_empty());
    for output in &outputs {
        assert_eq!(
            views.output(output.view).unwrap().revision,
            output.revision,
            "hidden views do not submit another scene render"
        );
    }
    controller
        .release_game_views(&mut views, &mut registry)
        .unwrap();
    for output in outputs {
        assert!(registry.get(output.image).is_none());
    }
}

#[test]
fn project_font_edits_and_removal_invalidate_every_screen_using_the_font_owner() {
    let mut session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut a = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let mut b = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let mut images = Images(0);
    let mut identity = None;
    for controller in [&mut a, &mut b] {
        controller
            .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
            .unwrap();
        let list = controller
            .paint(&session.view(), &mut fonts, &mut images)
            .unwrap();
        let run = list
            .ops
            .iter()
            .find_map(|op| {
                if let PaintOp::GlyphRun { run, .. } = op {
                    Some(run)
                } else {
                    None
                }
            })
            .unwrap();
        if let Some(id) = identity {
            assert_eq!(id, run.font.identity);
        }
        identity = Some(run.font.identity);
        assert_eq!(
            &*run.font.data,
            include_bytes!("../../theframework/embedded/fonts/SourceCodePro-Bold.ttf")
        );
    }
    let version = fonts.version();
    a.prepare(&session.view(), &mut fonts, ContentRevision(2), 0)
        .unwrap();
    assert_eq!(
        fonts.version(),
        version,
        "an unrelated source revision must not register the same fonts again"
    );
    session.project.assets.clear();
    a.prepare(&session.view(), &mut fonts, ContentRevision(3), 0)
        .unwrap();
    assert_ne!(fonts.version(), version);
    assert!(
        b.paint(&session.view(), &mut fonts, &mut images).is_err(),
        "a retained screen must reflow after its shared font owner changes"
    );
    b.prepare(&session.view(), &mut fonts, ContentRevision(3), 0)
        .unwrap();
    let list = b.paint(&session.view(), &mut fonts, &mut images).unwrap();
    let run = list
        .ops
        .iter()
        .find_map(|op| {
            if let PaintOp::GlyphRun { run, .. } = op {
                Some(run)
            } else {
                None
            }
        })
        .unwrap();
    assert_ne!(run.font.identity, identity.unwrap());
    assert_eq!(
        &*run.font.data,
        include_bytes!("../../theframework/embedded/fonts/Roboto-Bold.ttf")
    );
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn screen_navigation_reuses_project_font_identities_and_swash_gpu_cache() {
    let session = fixture(false);
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(600, 360)).unwrap();
    let mut fonts = GameFonts::default();
    let mut first = None;
    for _ in 0..3 {
        let mut controller =
            GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
        controller
            .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
            .unwrap();
        let list = {
            let mut frame = images.begin_frame(&mut registry).unwrap();
            controller
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
        let counts = (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
        );
        if let Some(first) = first {
            assert_eq!(counts, first);
        }
        assert!(counts.1 > 0);
        first = Some(counts);
    }
    assert_eq!(gpu.transfers().capture_requests, 0);
    assert!(
        images.is_empty(),
        "source-free UI navigation must only use GPU glyph atlases"
    );
}

#[test]
fn native_text_fields_preserve_ime_bindings_focus_and_clipboard_ownership() {
    use theframework::platform::*;
    use theframework::text_edit::{ClipboardResult, EditorAction, EditorClipboard};
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut controller =
        GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    controller.elements.text_input_widgets.insert(
        20,
        rusterix::client::widget::TextInputWidget {
            name: "SecondInput".into(),
            id: 20,
            rect: Rect::new(16., 130., 180., 36.),
            binding: "second".into(),
            text: "Other".into(),
            font: "Roboto".into(),
            font_size: 18.,
            color: [255; 4],
            background_color: [0, 0, 0, 255],
            border_color: [255; 4],
            border_size: 1,
        },
    );
    controller
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let key = |physical| InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed: true,
        repeat: false,
    };
    let click = InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: [210., 200.],
    };
    assert!(
        controller
            .text_input_event(&mut fonts, &click)
            .input
            .consumed
    );
    assert_eq!(controller.focused_input, Some(3));
    assert!(controller.text_ime_area(&mut fonts).is_some());
    controller.text_input_event(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    controller.text_input_event(&mut fonts, &key(PhysicalKey::Letter(b'a')));
    controller.text_input_event(&mut fonts, &InputEvent::Modifiers(Modifiers::default()));
    let old = controller.elements.ui_state.get("start.name").cloned();
    let response = controller.text_input_event(
        &mut fonts,
        &InputEvent::Composition {
            text: "にほん".into(),
            cursor: Some((3, 3)),
        },
    );
    assert!(!response.editor.changed);
    assert_eq!(controller.elements.text_input_widgets[&3].text, "Éowyn");
    assert_eq!(controller.elements.ui_state.get("start.name").cloned(), old);
    controller
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 1)
        .unwrap();
    assert!(controller.text_ime_area(&mut fonts).is_some());
    let mut images = Images(0);
    controller
        .paint(&session.view(), &mut fonts, &mut images)
        .unwrap()
        .validate()
        .unwrap();
    let response = controller.text_input_event(&mut fonts, &InputEvent::Text("日本".into()));
    assert!(response.editor.changed);
    assert_eq!(controller.elements.ui_state["start.name"], "日本");
    controller.text_input_event(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    controller.text_input_event(&mut fonts, &key(PhysicalKey::Letter(b'z')));
    assert_eq!(controller.elements.ui_state["start.name"], "Éowyn");
    let Some(EditorClipboard::Read { request }) = controller
        .text_input_event(&mut fonts, &key(PhysicalKey::Letter(b'v')))
        .editor
        .clipboard
    else {
        panic!("paste request")
    };
    controller.text_input_event(&mut fonts, &InputEvent::Modifiers(Modifiers::default()));
    let response = controller.text_input_event(&mut fonts, &key(PhysicalKey::Tab));
    assert_eq!(
        response.editor.action,
        Some(EditorAction::FocusNext { backwards: false })
    );
    assert_eq!(controller.focused_input, Some(20));
    assert!(
        !controller
            .complete_text_clipboard(
                &mut fonts,
                3,
                request,
                ClipboardResult::Text(Some("late".into()))
            )
            .editor
            .changed
    );
    // Shift held before the newly focused field must be retained for traversal.
    controller.text_input_event(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            shift: true,
            ..Default::default()
        }),
    );
    controller.text_input_event(&mut fonts, &key(PhysicalKey::Tab));
    assert_eq!(controller.focused_input, Some(3));
    controller.text_input_event(&mut fonts, &InputEvent::Modifiers(Modifiers::default()));
    let response = controller.text_input_event(&mut fonts, &InputEvent::Focus(false));
    assert!(response.input.redraw);
    assert_eq!(controller.focused_input, None);
    assert!(controller.text_ime_area(&mut fonts).is_none());
    assert!(
        !controller
            .text_input_event(&mut fonts, &InputEvent::Text("world".into()))
            .input
            .consumed
    );
    controller.focus_input(&mut fonts, Some(20));
    controller
        .elements
        .widgets_to_hide
        .push("SecondInput".into());
    assert!(
        !controller
            .text_input_event(&mut fonts, &InputEvent::Text("hidden".into()))
            .input
            .consumed
    );
    assert_eq!(controller.focused_input, None);
    assert_eq!(controller.elements.text_input_widgets[&20].text, "Other");
}

#[test]
fn native_dropdown_capture_precedes_fields_and_commits_keyboard_selection_once() {
    use rusterix::client::widget::choice::{ChoiceWidget, ChoiceWidgetKind};
    use theframework::platform::*;
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    screen.elements.choice_widgets.insert(
        30,
        ChoiceWidget::from_ui(
            "ClassChoice".into(),
            30,
            Rect::new(208., 130., 176., 32.),
            ChoiceWidgetKind::Dropdown,
            &"bind='class'\noptions=['Warrior','Mage','Rogue']"
                .parse()
                .unwrap(),
        )
        .unwrap(),
    );
    screen.elements.set_binding("class", "warrior");
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let pointer = |x, y, pressed| InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed,
        position: [x, y],
    };
    let key = |physical| InputEvent::Key {
        physical,
        logical: LogicalKey::Other,
        pressed: true,
        repeat: false,
    };
    screen.focus_input(&mut fonts, Some(3));
    assert!(
        screen
            .choice_input_event(&mut fonts, &pointer(220., 140., true))
            .consumed
    );
    assert_eq!(screen.focused_input, None);
    assert_eq!(screen.choice_controller.open_id(), Some(30));
    assert!(
        screen
            .choice_input_event(&mut fonts, &pointer(220., 140., false))
            .consumed
    );
    assert!(
        screen
            .choice_input_event(&mut fonts, &key(PhysicalKey::Down))
            .consumed
    );
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 1)
        .unwrap();
    assert_eq!(screen.elements.choice_widgets[&30].selected, 1);
    assert_eq!(screen.elements.ui_state["class"], "warrior");
    assert!(
        screen
            .choice_input_event(&mut fonts, &InputEvent::Text("ignored".into()))
            .consumed
    );
    assert!(
        screen
            .choice_input_event(&mut fonts, &key(PhysicalKey::Enter))
            .consumed
    );
    assert_eq!(screen.elements.ui_state["class"], "mage");
    assert_eq!(screen.choice_controller.open_id(), None);
    screen.choice_input_event(&mut fonts, &pointer(220., 140., true));
    screen.choice_input_event(&mut fonts, &pointer(220., 140., false));
    // Outside popup and over the Start button: dismiss without click-through.
    assert!(
        screen
            .choice_input_event(&mut fonts, &pointer(30., 200., true))
            .consumed
    );
    assert_eq!(screen.choice_controller.open_id(), None);
    assert!(
        screen
            .choice_input_event(&mut fonts, &pointer(30., 200., false))
            .consumed
    );
    assert!(
        !screen
            .choice_input_event(&mut fonts, &pointer(30., 200., false))
            .consumed
    );
    screen.choice_input_event(&mut fonts, &pointer(220., 140., true));
    screen.choice_input_event(&mut fonts, &key(PhysicalKey::Down));
    assert!(
        screen
            .choice_input_event(&mut fonts, &InputEvent::Focus(false))
            .redraw
    );
    assert_eq!(screen.elements.ui_state["class"], "mage");
    assert_eq!(screen.elements.choice_widgets[&30].selected, 1);
    assert_eq!(screen.choice_controller.open_id(), None);
}

#[test]
fn screen_navigation_keeps_typed_bindings_without_mutating_the_project() {
    use theframework::platform::*;
    let mut session = fixture(false);
    let copy = session.view().screen("Main").unwrap().map.clone();
    let id = Uuid::new_v4();
    let other = session.project.screens.entry(id).or_default();
    other.id = id;
    other.map = copy;
    other.map.name = "Other".into();
    sector(
        &mut other.map,
        8,
        Rect::new(0., 0., 16., 16.),
        "[ui]\nrole='button'\nactive=true\nbind='start.class'\nvalue='Warrior'\ngroup='classes'\nhide=['MageOnly']",
    );
    let before = serde_json::to_value(&session.project).unwrap();
    let mut fonts = GameFonts::default();
    let mut main = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    main.prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    main.focus_input(&mut fonts, Some(3));
    main.text_input_event(
        &mut fonts,
        &InputEvent::Modifiers(Modifiers {
            control: true,
            ..Default::default()
        }),
    );
    main.text_input_event(
        &mut fonts,
        &InputEvent::Key {
            physical: PhysicalKey::Letter(b'a'),
            logical: LogicalKey::Other,
            pressed: true,
            repeat: false,
        },
    );
    main.text_input_event(&mut fonts, &InputEvent::Modifiers(Modifiers::default()));
    main.text_input_event(&mut fonts, &InputEvent::Text("  東京  ".into()));
    main.elements.set_binding("start.class", "Mage");
    let mut other = GameScreenController::from_screen_with_state(
        "Other",
        &session.view(),
        layout(),
        main.elements.ui_state.clone(),
    )
    .unwrap();
    other
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    assert_eq!(other.elements.text_input_widgets[&3].text, "  東京  ");
    assert_eq!(other.elements.ui_state["start.class"], "Mage");
    assert_eq!(other.focused_input, None);
    assert!(!other.elements.activated_widgets.contains(&8));
    assert!(
        !other
            .elements
            .widgets_to_hide
            .iter()
            .any(|name| name == "MageOnly")
    );
    assert_eq!(serde_json::to_value(&session.project).unwrap(), before);
    assert!(
        other
            .elements
            .text_widgets
            .values()
            .all(|w| w.buffer.pixels().is_empty())
    );
}

#[test]
fn region_fallback_uses_shared_fixed_or_responsive_geometry_without_pixels() {
    let session = fixture(false);
    let mut fonts = GameFonts::default();
    let mut responsive = layout();
    responsive.screen_responsive = true;
    let mut screen =
        GameScreenController::region_fallback(&session.view(), responsive, Default::default())
            .unwrap();
    assert_eq!(screen.name, "World");
    assert_eq!(screen.elements.game_widgets.len(), 1);
    assert!(screen.resize((560, 320)));
    let game = &screen.elements.game_widgets[&Uuid::nil()];
    assert_eq!((game.rect.width, game.rect.height), (560., 320.));
    assert!(game.buffer.pixels().is_empty() && game.upscale_buffer.pixels().is_empty());
    assert!(game.text_font.is_none());
    screen
        .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
        .unwrap();
    let mut fixed =
        GameScreenController::region_fallback(&session.view(), layout(), Default::default())
            .unwrap();
    assert!(!fixed.resize((1000, 600)));
    assert_eq!(fixed.elements.game_widgets[&Uuid::nil()].rect.width, 400.);
}

#[test]
#[ignore = "requires native GPU; run with --ignored"]
fn region_fallback_renders_and_resizes_using_only_gpu_game_targets() {
    let session = fixture(true);
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    let mut cache = ScreenImageCache::default();
    let mut fonts = GameFonts::default();
    let mut responsive = layout();
    responsive.screen_responsive = true;
    let mut screen =
        GameScreenController::region_fallback(&session.view(), responsive, Default::default())
            .unwrap();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let mut previous = None;
    for (width, height) in [(400, 240), (320, 180)] {
        screen.resize((width, height));
        screen
            .prepare(&session.view(), &mut fonts, ContentRevision(1), 0)
            .unwrap();
        screen
            .prepare_game_views(&session.view(), &mut views, &mut registry)
            .unwrap();
        let output = screen.view_images()[&Uuid::nil()];
        assert!(!views.has_cpu_target_storage(output.view).unwrap());
        if let Some((view, image)) = previous {
            assert_eq!(output.view, view);
            assert_ne!(output.image, image);
            assert!(registry.get(image).is_none());
        }
        previous = Some((output.view, output.image));
        let list = screen
            .paint(
                &session.view(),
                &mut fonts,
                &mut cache.begin_frame(&mut registry).unwrap(),
            )
            .unwrap();
        let target = GpuImage::new(&gpu, ImageDescriptor::rgba(width, height)).unwrap();
        let readbacks = gpu.transfers().readback_bytes;
        renderer
            .render(
                &list,
                &registry,
                &target.view,
                target.descriptor.format,
                (width, height),
                1.,
                [0.; 4],
            )
            .unwrap();
        assert_eq!(gpu.transfers().readback_bytes, readbacks);
        let pixels = capture(&gpu, target);
        assert!(
            pixels
                .chunks_exact(4)
                .any(|p| p[0] > 100 && p[1] < 50 && p[2] < 50)
        );
        assert!(
            screen
                .elements
                .game_widgets
                .values()
                .all(|w| w.buffer.pixels().is_empty() && w.upscale_buffer.pixels().is_empty())
        );
    }
    let image = screen.view_images()[&Uuid::nil()].image;
    screen
        .release_game_views(&mut views, &mut registry)
        .unwrap();
    assert!(registry.get(image).is_none());
}
