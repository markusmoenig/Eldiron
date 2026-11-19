use super::*;
use eldiron_presentation::ScreenEffect;
use rusterix::client::{
    command::ClientCommandBinding,
    screen_model::{JournalModel, JournalTab, journal_geometry},
};
use theframework::platform::{InputEvent, PointerButton};

#[test]
#[ignore = "requires native GPU; run with --include-ignored"]
fn journal_gpu_renders_search_requirements_target_picker_and_knowledge_with_retained_text() {
    let mut session = fixture(false);
    session.server.state = rusterix::ServerState::Running;
    session.assets.rules = shared::rulesets::latest_official_ruleset().into();
    let templates =
        shared::rulesets::ruleset_item_templates_from_source(&session.assets.rules).unwrap();
    let rules = session.assets.value_rules().unwrap();
    let map = &mut session.project.regions[0].map;
    let actor = &mut map.entities[0];
    actor.inventory = vec![None; 10];
    actor.set_attribute("skill_boneworking", Value::Int(10));
    actor.set_attribute("STR", Value::Int(12));
    rusterix::learning::acquire(
        actor,
        &rules,
        &["bear_inscription".into()],
        rusterix::learning::Origin::Character,
    )
    .unwrap();
    for (id, name) in [
        (100, "bone_cuirass"),
        (101, "bear_inlay"),
        (102, "tree_resin"),
        (103, "wooden_bowl"),
        (104, "stone_knife"),
        (105, "bone_workbench"),
    ] {
        let mut item = rusterix::Item::new();
        item.id = id;
        rusterix::server::data::apply_item_data(
            &mut item,
            &templates.iter().find(|t| t.id == name).unwrap().data,
        );
        if name == "wooden_bowl" {
            item.set_attribute("liquid_item", Value::Str("fresh_water".into()));
            item.set_attribute("liquid_quantity", Value::Int(1));
        }
        if name == "bone_workbench" {
            item.position = actor.position;
            map.items.push(item);
        } else {
            actor.add_item(item).unwrap();
        }
    }
    let size = Vec2::new(960, 540);
    let mut screen = GameScreenController::from_screen(
        "Main",
        &session.view(),
        ScreenLayout {
            screen_responsive: false,
            reference_viewport: size,
            viewport: size,
            grid_size: 8.,
        },
    )
    .unwrap();
    assert!(screen.handle_hud_effect(
        &session.view(),
        &ScreenEffect::Command(ClientCommandBinding::Ui("journal".into()))
    ));
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut registry = ImageRegistry::new(gpu.clone());
    let mut images = ScreenImageCache::default();
    let mut fonts = GameFonts::default();
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 540)).unwrap();
    let mut views = SceneViews::new(&registry, SharedAtlas::new(64, 64));
    fn render(
        session: &GameSession,
        screen: &mut GameScreenController,
        fonts: &mut GameFonts,
        images: &mut ScreenImageCache,
        registry: &mut ImageRegistry,
        renderer: &mut PaintRenderer,
        target: &GpuImage,
        views: &mut SceneViews,
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
                (960, 540),
                1.,
                [0.; 4],
            )
            .unwrap();
        screen.acknowledge_world_frame(&session.view(), Some(views));
    }
    fn click(
        session: &GameSession,
        screen: &mut GameScreenController,
        fonts: &mut GameFonts,
        bounds: Rect,
    ) {
        let position = [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.];
        for pressed in [true, false] {
            let response = screen
                .input(
                    &session.view(),
                    fonts,
                    &InputEvent::PointerButton {
                        button: PointerButton::Primary,
                        pressed,
                        position,
                    },
                )
                .unwrap();
            assert!(response.effects.is_empty());
        }
    }
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut images,
        &mut registry,
        &mut renderer,
        &target,
        &mut views,
    );
    let mut model = JournalModel::default();
    let map = session.view().map.unwrap();
    model
        .bind(&session.assets, &map.entities[0], map, false)
        .unwrap();
    let geometry = journal_geometry(
        size,
        None,
        model
            .entries(&session.assets, JournalTab::Recipes, "", "all")
            .unwrap()
            .len(),
        0,
    );
    click(&session, &mut screen, &mut fonts, geometry.search);
    screen
        .input(
            &session.view(),
            &mut fonts,
            &InputEvent::Text("Inscribe".into()),
        )
        .unwrap();
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut images,
        &mut registry,
        &mut renderer,
        &target,
        &mut views,
    );
    click(&session, &mut screen, &mut fonts, geometry.target);
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut images,
        &mut registry,
        &mut renderer,
        &target,
        &mut views,
    );
    click(
        &session,
        &mut screen,
        &mut fonts,
        Rect::new(
            geometry.target.x,
            geometry.target.y - 28.,
            geometry.target.width,
            28.,
        ),
    );
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut images,
        &mut registry,
        &mut renderer,
        &target,
        &mut views,
    );
    let uploads = gpu.transfers().uploaded_bytes;
    let glyphs = renderer.glyphs.rasterizations;
    let position = [
        geometry.craft.x + geometry.craft.width / 2.,
        geometry.craft.y + geometry.craft.height / 2.,
    ];
    let mut effects = Vec::new();
    for pressed in [true, false] {
        effects.extend(
            screen
                .input(
                    &session.view(),
                    &mut fonts,
                    &InputEvent::PointerButton {
                        button: PointerButton::Primary,
                        pressed,
                        position,
                    },
                )
                .unwrap()
                .effects,
        );
    }
    assert_eq!(
        effects,
        vec![ScreenEffect::Action(rusterix::EntityAction::ItemClicked(
            100,
            0.,
            Some("action:inscribe_bear_cuirass".into()),
            Some(1)
        ))]
    );
    render(
        &session,
        &mut screen,
        &mut fonts,
        &mut images,
        &mut registry,
        &mut renderer,
        &target,
        &mut views,
    );
    assert_eq!(gpu.transfers().uploaded_bytes, uploads);
    assert_eq!(renderer.glyphs.rasterizations, glyphs);
    assert_eq!(
        gpu.transfers().readback_bytes,
        0,
        "normal journal painting never reads pixels back"
    );
    let pixels = capture(&gpu, target.clone());
    assert!(
        pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 160 && p[1] > 160 && p[2] > 140)
            .count()
            > 300,
        "journal text must render"
    );
    let path = std::env::var("ELDIRON_JOURNAL_PREVIEW")
        .unwrap_or("/private/tmp/eldiron-crafting-journal.png".into());
    let mut png = png::Encoder::new(std::fs::File::create(path).unwrap(), 960, 540);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header()
        .unwrap()
        .write_image_data(&pixels)
        .unwrap();
}
