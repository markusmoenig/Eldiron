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
    let image_count = images.len();
    let source_builds = screen.source_builds();
    let rasters = renderer.glyphs.rasterizations;
    let mut next = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    next.retain_session_presentation_from(&mut screen, &mut fonts);
    screen
        .release_game_views(&mut views, &mut registry)
        .unwrap();
    screen = next;
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
        screen.source_builds(),
        source_builds,
        "source recipes remain owned by the session across navigation"
    );
    assert_eq!(
        images.len(),
        image_count,
        "retained HUD artwork resolves to the same GPU source handles"
    );
    assert_eq!(renderer.glyphs.rasterizations, rasters);
    let counts = (
        gpu.transfers().uploaded_bytes,
        renderer.glyphs.rasterizations,
        screen.source_builds(),
    );
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

#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn native_inventory_and_equipment_share_retained_sources_glyphs_and_typed_transfers() {
    use rusterix::client::{command::ClientCommandBinding, screen_model};
    use rusterix::{
        Avatar, AvatarAnimation, AvatarAnimationFrame, AvatarDirection, AvatarPerspective,
        AvatarPerspectiveCount,
    };
    let mut session = fixture(true);
    session.server.state = rusterix::ServerState::Running;
    session.assets.rules =
        "[equipment]\nweapon_slots=['main_hand','off_hand']\narmor_slots=['head','body']".into();
    let tile = Tile::from_texture(Texture::from_color([0, 255, 255, 255]));
    let tile_id = tile.id;
    session.assets.tiles.insert(tile_id, tile);
    let mut avatar_pixels = vec![0; 8 * 8 * 4];
    for y in 1..7 {
        for x in 2..6 {
            avatar_pixels[(y * 8 + x) * 4..(y * 8 + x) * 4 + 4]
                .copy_from_slice(&[24, 128, 240, 255]);
        }
    }
    session.assets.avatars.insert(
        "Human".into(),
        Avatar {
            id: Default::default(),
            name: "Human".into(),
            resolution: 8,
            perspective_count: AvatarPerspectiveCount::Four,
            animations: vec![AvatarAnimation {
                id: Default::default(),
                name: "Idle".into(),
                speed: 1.,
                perspectives: vec![AvatarPerspective {
                    direction: AvatarDirection::Front,
                    frames: vec![AvatarAnimationFrame::new(Texture::new(avatar_pixels, 8, 8))],
                    weapon_main_anchor: None,
                    weapon_off_anchor: None,
                }],
            }],
        },
    );
    session.assets.default_avatar = Some("Human".into());
    let mut item = rusterix::Item::new();
    item.id = 7;
    item.set_stack_quantity(17);
    item.set_attribute("source", Value::Source(PixelSource::TileId(tile_id)));
    let actor = &mut session.project.regions[0].map.entities[0];
    actor.inventory = vec![Some(item.clone()), None];
    let mut equipped = item;
    equipped.id = 8;
    actor.equipped.insert("main_hand".into(), equipped);
    sector(
        &mut session.project.screens.values_mut().next().unwrap().map,
        21,
        Rect::new(320., 180., 48., 48.),
        "[ui]\nrole='button'\ninventory_index=1\ndrag_drop=true\nshow_icon=false",
    );
    let mut screen = GameScreenController::from_screen("Main", &session.view(), layout()).unwrap();
    let inventory = screen_model::InventoryPanelConfig {
        columns: 4,
        rows: 2,
        cell_size: 28.,
        spacing: 4.,
        padding: 4.,
        title_height: 24.,
        tab_height: 20.,
        footer_height: 24.,
        font_size: 10.,
        title_font_size: 14.,
        title: "Inventory 東京".into(),
        authored_rect: Some(Rect::new(8., 8., 280., 164.)),
        ..Default::default()
    };
    let equipment = screen_model::EquipmentPanelConfig {
        slot_size: 24.,
        padding: 4.,
        title_height: 24.,
        label_width: 60.,
        avatar_width: 56.,
        avatar_height: 70.,
        font_size: 10.,
        title_font_size: 14.,
        title: "Equipment 東京".into(),
        authored_rect: Some(Rect::new(8., 8., 280., 164.)),
        ..Default::default()
    };
    let geometry = screen_model::inventory_panel_geometry(
        &session.view().map.unwrap().entities[0],
        &inventory,
        "all",
        "newest",
        0,
        None,
        layout().viewport,
    );
    let slot = geometry.slot(0).unwrap().rect;
    let point = [slot.x + slot.width / 2., slot.y + slot.height / 2.];
    screen.elements.custom_inventory_panel_config = Some(inventory);
    screen.elements.custom_equipment_panel_config = Some(equipment.clone());
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("inventory".into()))
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
    let down = InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: true,
        position: point,
    };
    assert!(
        screen
            .input(&session.view(), &mut fonts, &down)
            .unwrap()
            .input
            .consumed
    );
    screen.world_input(
        &session.view(),
        Some(&views),
        &InputEvent::PointerMoved {
            position: [340., 150.],
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
        counts
    );
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert!(screen.elements.game_widgets.values().all(
        |widget| widget.buffer.pixels().is_empty() && widget.upscale_buffer.pixels().is_empty()
    ));
    let pixels = capture(&gpu, target.clone());
    let pixel = |point: [f32; 2]| {
        let (x, y) = ((point[0] * 1.5) as usize, (point[1] * 1.5) as usize);
        &pixels[(y * 600 + x) * 4..(y * 600 + x) * 4 + 4]
    };
    write_panel_evidence("/private/tmp/eldiron-native-inventory-hud.png", &pixels);
    assert_eq!(
        pixel([slot.x + 8., slot.y + 8.]),
        &[0, 255, 255, 255],
        "native inventory source outside its quantity badge"
    );
    assert_eq!(
        pixel([340., 150.]),
        &[0, 255, 255, 255],
        "retained GPU inventory drag"
    );

    let up = InputEvent::PointerButton {
        button: PointerButton::Primary,
        pressed: false,
        position: [344., 204.],
    };
    assert_eq!(
        screen
            .world_input(&session.view(), Some(&views), &up, false)
            .effects,
        vec![ScreenEffect::Action(rusterix::EntityAction::MoveItem {
            item_id: 7,
            owner_entity_id: Some(1),
            target_entity_id: Some(1),
            to_inventory_index: Some(1),
            to_equipped_slot: None
        })]
    );
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("equipment".into()))
    ));
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
        gpu.transfers().readback_bytes,
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
            screen.source_builds(),
            gpu.transfers().readback_bytes
        ),
        counts
    );
    let pixels = capture(&gpu, target.clone());
    let geometry = screen_model::equipment_panel_geometry(
        &session.view().map.unwrap().entities[0],
        &equipment,
        &session.assets,
        None,
        layout().viewport,
    );
    let slot = geometry
        .slots
        .iter()
        .find(|slot| slot.slot == "main_hand")
        .unwrap()
        .rect;
    let pixel = |x: f32, y: f32| {
        let (x, y) = ((x * 1.5) as usize, (y * 1.5) as usize);
        &pixels[(y * 600 + x) * 4..(y * 600 + x) * 4 + 4]
    };
    assert_eq!(pixel(slot.x + 8., slot.y + 8.), &[0, 255, 255, 255]);
    let avatar = geometry.avatar_rect;
    assert_eq!(
        pixel(avatar.x + avatar.width / 2., avatar.y + avatar.height / 2.),
        &[24, 128, 240, 255],
        "retained equipment avatar source"
    );
    let available = avatar.with_border(4.);
    let fitted_width = (available.width / 4.).min(available.height / 6.) * 4.;
    assert_eq!(
        pixel(
            avatar.x + avatar.width / 2. + fitted_width * 0.4,
            avatar.y + avatar.height / 2.
        ),
        &[24, 128, 240, 255],
        "cropped avatar end coordinates are not extents"
    );
    write_panel_evidence("/private/tmp/eldiron-native-equipment-hud.png", &pixels);
}
fn write_panel_evidence(path: &str, pixels: &[u8]) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 600, 360);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
}
