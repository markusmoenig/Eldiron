use super::item_pointer::render;
use super::*;
use eldiron_presentation::{HudPart, ScreenCapture, ScreenEffect};
use rusterix::client::{command::ClientCommandBinding, rules_ui, screen_model};
use theframework::platform::{InputEvent, PointerButton};

fn center(rect: Rect) -> [f32; 2] {
    [rect.x + rect.width / 2., rect.y + rect.height / 2.]
}
fn button(position: [f32; 2], pressed: bool) -> InputEvent {
    InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed,
        position,
    }
}
#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn native_catalogue_composes_nine_slice_icons_cooldowns_wrapped_details_and_retained_command_drag()
{
    let mut session = fixture(true);
    session.server.state = rusterix::ServerState::Running;
    session.assets.rules = r#"
        [classes.Adventurer.action_bar]
        main = ['rules.basic_attack', 'rules.minor_heal']
        [actions.basic_attack]
        name = 'Arc 東京'
        kind = 'attack'
        icon = 'arc'
        description = 'A carefully aimed strike. This explanation wraps across multiple lines using retained text layout, with facts and effects below the summary.'
        [actions.minor_heal]
        name = 'Minor Heal'
        kind = 'spell'
        icon = 'heal'
        requires = { spell = 'minor_heal' }
        cooldown = 4.0
        [spells.minor_heal]
        name = 'Minor Heal'
    "#.into();
    session
        .assets
        .textures
        .insert("arc".into(), Texture::from_color([0, 255, 255, 255]));
    session
        .assets
        .textures
        .insert("heal".into(), Texture::from_color([255, 64, 16, 255]));
    let actor = &mut session.project.regions[0].map.entities[0];
    actor.set_attribute("class", Value::Str("Adventurer".into()));
    actor.set_attribute("spells", Value::StrArray(vec!["minor_heal".into()]));
    actor.set_attribute("cooldown_left_spell_minor_heal", Value::Float(2.));
    actor.set_attribute("cooldown_total_spell_minor_heal", Value::Float(4.));
    // Distinct corners/edges/center detect incorrect nine-slice UVs.
    let mut pixels = vec![0; 6 * 6 * 4];
    for y in 0..6 {
        for x in 0..6 {
            let pixel = if x < 2 && y >= 4 {
                [192, 48, 160, 255]
            } else if x < 2 || y < 2 || x >= 4 || y >= 4 {
                [64, 96, 128, 255]
            } else {
                [24, 36, 48, 255]
            };
            pixels[(y * 6 + x) * 4..(y * 6 + x) * 4 + 4].copy_from_slice(&pixel);
        }
    }
    let config = screen_model::CatalogPanelConfig {
        columns: 2,
        rows: Some(1),
        cell_size: 32.,
        spacing: 4.,
        padding: 6.,
        title_height: 28.,
        tab_height: 22.,
        detail_width: 136.,
        detail_gap: 8.,
        icon_inset: 5.,
        font_size: 12.,
        title_font_size: 16.,
        small_font_size: 10.,
        title: "Actions 東京".into(),
        show_assign: true,
        authored_rect: Some(Rect::new(8., 8., 384., 166.)),
        frame_texture: Some(Texture::new(pixels, 6, 6)),
        frame_slice: 2,
        border_size: 0,
        slot_texture: Some(Texture::from_color([48, 64, 80, 255])),
        ..Default::default()
    };
    sector(
        &mut session.project.screens.values_mut().next().unwrap().map,
        20,
        Rect::new(140., 184., 40., 40.),
        "[ui]\nrole='button'\ncommand_slot='main.1'\nshow_icon=false",
    );
    let actor = &session.project.regions[0].map.entities[0];
    let groups = rules_ui::action_catalog(&session.assets, Some(actor));
    let geometry =
        screen_model::catalog_panel_geometry(&groups, &config, "all", 0, None, Vec2::new(400, 240));
    let attack = geometry
        .entries
        .iter()
        .find(|entry| entry.command == "rules.basic_attack")
        .unwrap();
    let heal = geometry
        .entries
        .iter()
        .find(|entry| entry.command == "rules.minor_heal")
        .unwrap();
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    screen.elements.custom_actions_panel_config = Some(config);
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("actions".into()))
    ));
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
    assert!(counts.1 > 0);
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
    assert!(
        screen
            .elements
            .game_widgets
            .values()
            .all(|w| w.buffer.pixels().is_empty() && w.upscale_buffer.pixels().is_empty())
    );
    assert_eq!(gpu.transfers().readback_bytes, 0);
    let pixels = capture(&gpu, target.clone());
    let pixel = |x: f32, y: f32| {
        let i = ((y * 1.5) as usize * 600 + (x * 1.5) as usize) * 4;
        &pixels[i..i + 4]
    };
    // Avoid the intentionally selected border, cooldown number and glyphs.
    assert_eq!(
        pixel(9., 173.),
        &[192, 48, 160, 255],
        "unscaled bottom-left corner"
    );
    assert_eq!(
        pixel(200., 168.),
        &[24, 36, 48, 255],
        "stretched frame center"
    );
    let attack_point = center(attack.icon_rect);
    // Selected command icons use the shared filter, rather than changing source pixels.
    let selected = pixel(attack_point[0], attack_point[1]);
    assert!(
        selected[1] > 240 && selected[2] > 210 && selected[0] < 100,
        "{selected:?}"
    );
    let upper = pixel(heal.icon_rect.x + 3., heal.rect.y + 9.);
    let lower = pixel(heal.icon_rect.x + 3., heal.rect.y + heal.rect.height - 9.);
    assert!(
        upper[0] < lower[0],
        "cooldown shades only the upper half: {upper:?} / {lower:?}"
    );
    assert!(
        screen
            .input(&session.view(), &mut fonts, &button(attack_point, true))
            .unwrap()
            .effects
            .is_empty()
    );
    assert!(
        matches!(screen.control_capture(), Some(ScreenCapture::Hud(id)) if matches!(id.part, HudPart::CatalogEntry(_)))
    );
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::PointerMoved {
                position: [190., 202.],
            },
        )
        .unwrap();
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
    // Selection/press/drag need no new artwork or glyphs after warming.
    assert_eq!(
        (
            gpu.transfers().uploaded_bytes,
            renderer.glyphs.rasterizations,
            screen.source_builds()
        ),
        counts
    );
    let before_capture = gpu.transfers().readback_bytes;
    let pixels = capture(&gpu, target.clone());
    let i = (303 * 600 + 285) * 4;
    assert_eq!(
        &pixels[i..i + 4],
        &[0, 255, 255, 255],
        "normal source reused for drag ghost"
    );
    assert!(gpu.transfers().readback_bytes > before_capture);
    let mut encoder = png::Encoder::new(
        std::fs::File::create("/private/tmp/eldiron-native-catalogue-hud.png").unwrap(),
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
    assert_eq!(
        screen
            .input(&session.view(), &mut fonts, &button([160., 204.], false))
            .unwrap()
            .effects,
        vec![ScreenEffect::Action(
            rusterix::EntityAction::SetCommandSlot {
                slot: "main.1".into(),
                command: Some("rules.basic_attack".into())
            }
        )]
    );
    assert!(
        screen
            .input(&session.view(), &mut fonts, &button([160., 204.], false))
            .unwrap()
            .effects
            .is_empty()
    );
}
