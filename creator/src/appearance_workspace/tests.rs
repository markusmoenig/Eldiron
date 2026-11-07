use super::*;
fn fixture() -> (TheUI, TheContext) {
    let mut ui = TheUI::new();
    let mut ctx = TheContext::new(1440, 900, 1.);
    ui.style.set_theme(Box::new(TheBlackBlueTheme::new()));
    crate::procedural_material::register_preview_icons(&mut ctx, ui.style.theme().as_ref());
    ui.canvas = main_canvas(TheCanvas::new());
    ui.canvas.set_dim(TheDim::new(0, 0, 1440, 900), &mut ctx);
    (ui, ctx)
}
#[test]
fn viewport_inputs_do_not_reach_the_map_and_pause_holds_the_displayed_pose() {
    let (mut ui, mut ctx) = fixture();
    let mut p = Project::default();
    let mut c = Character::new();
    let id = c.id;
    c.name = "Orc".into();
    p.characters.insert(id, c);
    let mut host = AppearanceWorkspace {
        owner: Some(Owner::Character(id)),
        playing: true,
        phase: 0.,
        playhead: 0.43,
        zoom: 1.,
        ..Default::default()
    };
    let mut server = ServerContext::default();
    AppearanceWorkspace::show(&mut ui, &mut ctx, true);
    assert!(host.handle_event(
        &TheEvent::IndexChanged(TheId::named("Definition Appearance Play"), 0),
        &mut p,
        &mut ui,
        &mut ctx,
        &mut server
    ));
    assert!(!host.playing);
    assert_eq!(host.phase, 0.43);
    assert!(host.handle_event(
        &TheEvent::RenderViewPreciseScrollBy(TheId::named(VIEW), Vec2::new(0, -10)),
        &mut p,
        &mut ui,
        &mut ctx,
        &mut server
    ));
    assert!(host.zoom < 1.);
    assert!(host.handle_event(
        &TheEvent::RenderViewClicked(TheId::named(VIEW), Vec2::new(0, 0)),
        &mut p,
        &mut ui,
        &mut ctx,
        &mut server
    ));
    assert!(host.handle_event(
        &TheEvent::RenderViewDragged(TheId::named(VIEW), Vec2::new(20, 0)),
        &mut p,
        &mut ui,
        &mut ctx,
        &mut server
    ));
    assert!(host.yaw > 0.);
    let before = p.characters[&id].data.clone();
    host.handle_event(
        &TheEvent::IndexChanged(TheId::named("Definition Appearance Dimension"), 0),
        &mut p,
        &mut ui,
        &mut ctx,
        &mut server,
    );
    assert!(host.two_d);
    assert_eq!(p.characters[&id].data, before);
}
#[test]
fn setup_is_not_duplicated_in_the_preview_and_item_states_have_direct_entrances() {
    let (mut ui, mut ctx) = fixture();
    let mut p = Project::default();
    p.sync_ruleset_items().unwrap();
    let mut c = Character::new();
    c.name = "Orc".into();
    c.data = "[attributes]\nrace='Orc'\nclass='Warrior'\nstart_equipped_items=['Linen Shirt','Training Sword']\n".into();
    let id = c.id;
    p.characters.insert(id, c);
    let mut assets = rusterix::Assets::new();
    assets.rules = p.rules_source().unwrap();
    assets.items = p
        .items
        .values()
        .map(|i| (i.name.clone(), (i.source.clone(), i.data.clone())))
        .collect();
    let mut host = AppearanceWorkspace {
        owner: Some(Owner::Character(id)),
        snapshot: Some(Arc::new(assets)),
        ..Default::default()
    };
    host.refresh_form(&p, &mut ui, &mut ctx);
    AppearanceWorkspace::show(&mut ui, &mut ctx, true);
    ui.canvas.set_dim(TheDim::new(0, 0, 1440, 900), &mut ctx);
    let d = ui.get_render_view(VIEW).unwrap().dim();
    assert_eq!(
        d.width, 1440,
        "the preview should use the whole editor width"
    );
    assert!(d.height > 790);
    for name in [
        "Definition Appearance Asset",
        "Definition Appearance Variant",
        "Definition Appearance Model",
        "Definition Appearance Slot torso",
        "Definition Appearance Sharing",
    ] {
        assert!(ui.get_widget(name).is_none(), "unnecessary control: {name}");
    }
    assert!(ui.get_widget("Definition Appearance Setup").is_some());
    let job = host.request(&p, (320, 320), [0, 0, 0, 255]).unwrap();
    assert_eq!(job.entity.attributes.get_str("avatar"), Some("orc"));
    assert!(job.entity.equipped.contains_key("torso"));
    assert!(job.entity.equipped.contains_key("main_hand"));
    assert!(p.characters[&id].preview_rigging.is_empty());
    let torch = p
        .items
        .values()
        .find(|i| data::attribute(&i.data, "ruleset_id").as_deref() == Some("torch"))
        .unwrap()
        .id;
    host.owner = Some(Owner::Item(torch));
    host.refresh_form(&p, &mut ui, &mut ctx);
    assert!(ui.get_widget("Definition Appearance Setup").is_none());
    for name in [
        "Definition Appearance Icon On",
        "Definition Appearance Icon Off",
        "Definition Appearance Model On",
        "Definition Appearance Model Off",
    ] {
        assert!(
            ui.get_widget(name).is_some(),
            "missing state entrance {name}"
        );
    }
    let mut server = ServerContext::default();
    let before = p.procedural_materials.clone();
    assert!(host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("Definition Appearance Icon Off"),
            TheWidgetState::Clicked
        ),
        &mut p,
        &mut ui,
        &mut ctx,
        &mut server
    ));
    assert!(host.two_d && host.ground && !host.active_state);
    assert_eq!(before, p.procedural_materials);
    assert!(
        host.request(&p, (320, 320), [0, 0, 0, 255])
            .unwrap()
            .icon
            .is_some()
    );
    AppearanceWorkspace::show(&mut ui, &mut ctx, true);
    assert!(host.handle_event(
        &TheEvent::StateChanged(
            TheId::named("Definition Appearance Icon On"),
            TheWidgetState::Clicked
        ),
        &mut p,
        &mut ui,
        &mut ctx,
        &mut server
    ));
    assert!(host.active_state);
}
#[test]
#[ignore = "requires a native GPU adapter"]
fn appearance_roundtrip_preview_and_ui_snapshots() {
    use std::path::Path;
    let out = Path::new("/tmp/appearance-workspace-review");
    std::fs::create_dir_all(out).unwrap();
    let (mut ui, mut ctx) = fixture();
    let mut p = Project::default();
    p.sync_ruleset_items().unwrap();
    let mut config = p.config.parse::<toml_edit::DocumentMut>().unwrap();
    config["appearance"]["sprite_pixels_2d"] = toml_edit::value(32);
    config["appearance"]["pixel_size_3d"] = toml_edit::value(4);
    p.config = config.to_string();
    let mut c = Character::new();
    c.name = "Orc warrior".into();
    c.data = "[attributes]\nrace='Orc'\nclass='Warrior'\nstart_equipped_items=['Linen Shirt','Wool Trousers','Leather Shoes','Training Sword','Round Shield']\n".into();
    let id = c.id;
    p.characters.insert(id, c);
    let canonical = p.procedural_appearances[&shared::rulesets::geometry_asset_id("orc")].clone();
    data::make_variant(Owner::Character(id), &mut p).unwrap();
    let saved = serde_json::to_string(&p).unwrap();
    p = serde_json::from_str(&saved).unwrap();
    assert_eq!(
        p.procedural_appearances[&shared::rulesets::geometry_asset_id("orc")],
        canonical
    );
    {
        let mut runtime = RUSTERIX.write().unwrap();
        runtime.assets.config = p.config.clone();
        runtime.assets.rules = p.rules_source().unwrap();
        runtime.assets.items = p
            .items
            .values()
            .map(|i| (i.name.clone(), (i.source.clone(), i.data.clone())))
            .collect();
        shared::rusterix_utils::sync_item_icon_assets(&mut runtime, &p);
        shared::rusterix_utils::sync_procedural_avatar_assets(&mut runtime.assets, &p);
    }
    let mut tree = TheTreeLayout::new(TheId::named("Appearance Review Tree"));
    tree.limiter_mut().set_max_width(300);
    let root = tree.get_root();
    let mut characters = TheTreeNode::new(TheId::named("Characters"));
    characters.set_open(true);
    let mut character = gen_character_tree_node(&p.characters[&id]);
    character.set_open(true);
    characters.add_child(character);
    root.add_child(characters);
    let mut items = TheTreeNode::new(TheId::named("Items"));
    populate_item_tree(&mut items, &p);
    items.set_open(true);
    root.add_child(items);
    let mut browser = TheCanvas::new();
    browser.set_layout(tree);
    ui.canvas.set_right(browser);
    let mut host = AppearanceWorkspace {
        owner: Some(Owner::Character(id)),
        snapshot: Some(Arc::new(RUSTERIX.read().unwrap().assets.clone())),
        clip: 1,
        phase: 0.3,
        playing: false,
        yaw: 45.,
        zoom: 1.,
        ..Default::default()
    };
    host.refresh_form(&p, &mut ui, &mut ctx);
    AppearanceWorkspace::show(&mut ui, &mut ctx, true);
    if let Some(w) = ui
        .get_widget("Definition Appearance Play")
        .and_then(|w| w.as_group_button())
    {
        w.set_index(0);
    }
    ui.set_widget_value(
        "Definition Appearance Phase",
        &mut ctx,
        TheValue::Float(host.phase),
    );
    if let Some(w) = ui
        .get_widget("Definition Appearance Clip")
        .and_then(|w| w.as_drop_down_menu())
    {
        w.set_selected_index(host.clip as i32);
    }
    ui.set_widget_value(
        "Definition Appearance Status",
        &mut ctx,
        TheValue::Text("Outfit from character setup · drag to turn · scroll to zoom".into()),
    );
    let mut vm = scenevm::SceneVM::new(800, 800);
    let mut uploaded = Default::default();
    let mut captures = vec![];
    for (name, two_d) in [("3d", false), ("2d", true)] {
        host.two_d = two_d;
        if let Some(w) = ui
            .get_widget("Definition Appearance Dimension")
            .and_then(|w| w.as_group_button())
        {
            w.set_index(if two_d { 0 } else { 1 });
        }
        ui.canvas.set_dim(TheDim::new(0, 0, 1440, 900), &mut ctx);
        let d = *ui.get_render_view(VIEW).unwrap().dim();
        let job = host
            .request(
                &p,
                (d.width as u32, d.height as u32),
                *ui.style.theme().color(MaterialCanvas),
            )
            .unwrap();
        let (buffer, phase) = preview::render(&mut vm, &mut uploaded, &job).unwrap();
        assert!((phase - 0.3).abs() < 0.0001);
        assert!(
            buffer
                .pixels()
                .chunks_exact(4)
                .map(|p| p[0])
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 5
        );
        std::fs::write(
            out.join(format!("preview-{name}.png")),
            buffer.to_png().unwrap(),
        )
        .unwrap();
        captures.push(buffer.pixels().to_vec());
        *ui.get_render_view(VIEW).unwrap().render_buffer_mut() = buffer;
        ui.get_render_view(VIEW).unwrap().set_needs_redraw(true);
        let mut rgba = vec![0; 1440 * 900 * 4];
        ui.draw(&mut rgba, &mut ctx);
        std::fs::write(
            out.join(format!("workspace-{name}.png")),
            TheRGBABuffer::from(rgba, 1440, 900).to_png().unwrap(),
        )
        .unwrap();
    }
    assert_ne!(captures[0], captures[1]);
    let entity = host.request(&p, (320, 320), [0, 0, 0, 255]).unwrap().entity;
    let assets = host.snapshot.as_ref().unwrap();
    let appearance = rusterix::procedural_avatar::Runtime::resolve(&entity, assets).unwrap();
    let equipped = appearance.sprite(&entity, assets, "idle", 0.).unwrap();
    let mut without_shield = entity.clone();
    without_shield.equipped.shift_remove("shield");
    assert_ne!(
        equipped,
        appearance
            .sprite(&without_shield, assets, "idle", 0.)
            .unwrap(),
        "the configured shield must render through the ruleset off-hand anchor"
    );
    let mut naked = entity.clone();
    naked.equipped.shift_remove("main_hand");
    naked.equipped.shift_remove("shield");
    assert_ne!(
        equipped,
        appearance.sprite(&naked, assets, "idle", 0.).unwrap()
    );
    // Keep time fixed: live attachment changes must render in both dimensions.
    for two_d in [true, false] {
        let dimension = if two_d { "2d" } else { "3d" };
        let mut job = host.request(&p, (384, 384), [24, 22, 20, 255]).unwrap();
        job.two_d = two_d;
        job.clip = "idle".into();
        job.phase = 0.;
        job.seconds = 0.;
        job.playing = false;
        job.entity = entity.clone();
        let worn = preview::render(&mut vm, &mut uploaded, &job).unwrap().0;
        job.entity.equipped.shift_remove("main_hand");
        job.entity.equipped.shift_remove("shield");
        let unarmed = preview::render(&mut vm, &mut uploaded, &job).unwrap().0;
        assert_ne!(
            worn.pixels(),
            unarmed.pixels(),
            "{dimension} weapon attachments must update"
        );
        job.entity.equipped.clear();
        let bare = preview::render(&mut vm, &mut uploaded, &job).unwrap().0;
        assert_ne!(
            unarmed.pixels(),
            bare.pixels(),
            "{dimension} clothing must update"
        );
        job.entity = entity.clone();
        let restored = preview::render(&mut vm, &mut uploaded, &job).unwrap().0;
        assert_eq!(
            worn.pixels(),
            restored.pixels(),
            "{dimension} re-equipping must restore the same appearance"
        );
        for (state, image) in [("equipped", worn), ("unarmed", unarmed), ("bare", bare)] {
            std::fs::write(
                out.join(format!("equipment-{dimension}-{state}.png")),
                image.to_png().unwrap(),
            )
            .unwrap();
        }
    }
    // Review both direct item-state entrances in the same native workspace.
    let character_owner = host.owner;
    let torch = p
        .items
        .values()
        .find(|i| data::attribute(&i.data, "ruleset_id").as_deref() == Some("torch"))
        .unwrap()
        .id;
    host.owner = Some(Owner::Item(torch));
    host.two_d = true;
    host.ground = true;
    for (state, on) in [("on", true), ("off", false)] {
        host.active_state = on;
        host.refresh_form(&p, &mut ui, &mut ctx);
        if let Some(w) = ui
            .get_widget("Definition Appearance Dimension")
            .and_then(|w| w.as_group_button())
        {
            w.set_index(0);
        }
        ui.canvas.set_dim(TheDim::new(0, 0, 1440, 900), &mut ctx);
        let d = *ui.get_render_view(VIEW).unwrap().dim();
        for name in [
            "Definition Appearance Icon On",
            "Definition Appearance Icon Off",
            "Definition Appearance Model On",
            "Definition Appearance Model Off",
        ] {
            let widget = ui.get_widget(name).unwrap();
            assert!(
                widget.dim().x + widget.dim().width <= d.width,
                "state button clipped: {name}"
            );
        }
        let job = host
            .request(
                &p,
                (d.width as u32, d.height as u32),
                *ui.style.theme().color(MaterialCanvas),
            )
            .unwrap();
        let (buffer, _) = preview::render(&mut vm, &mut uploaded, &job).unwrap();
        *ui.get_render_view(VIEW).unwrap().render_buffer_mut() = buffer;
        ui.get_render_view(VIEW).unwrap().set_needs_redraw(true);
        let mut rgba = vec![0; 1440 * 900 * 4];
        ui.draw(&mut rgba, &mut ctx);
        std::fs::write(
            out.join(format!("item-{state}.png")),
            TheRGBABuffer::from(rgba, 1440, 900).to_png().unwrap(),
        )
        .unwrap();
    }
    // Torch effects use the same item state and posed attachment in every live view.
    for (name, two_d, ground) in [
        ("2d-ground", true, true),
        ("2d-held", true, false),
        ("3d-ground", false, true),
        ("3d-held", false, false),
    ] {
        host.two_d = two_d;
        host.ground = ground;
        host.phase = 0.;
        host.clip = 0;
        host.playing = false;
        let mut effect_player = rusterix::procedural_avatar::effects::Player::default();
        let mut images = vec![];
        for (state, on) in [("on", true), ("off", false)] {
            host.active_state = on;
            let job = host.request(&p, (512, 512), [24, 22, 20, 255]).unwrap();
            let image =
                preview::render_with_effects(&mut vm, &mut uploaded, &job, &mut effect_player)
                    .unwrap()
                    .0;
            std::fs::write(
                out.join(format!("torch-{name}-{state}.png")),
                image.to_png().unwrap(),
            )
            .unwrap();
            images.push(image.pixels().to_vec());
        }
        assert_ne!(
            images[0], images[1],
            "{name} must extinguish the flame and light"
        );
    }
    host.owner = character_owner;
    host.ground = false;
    let before = p.clone();
    p.palette.colors.swap(20, 45);
    {
        let mut runtime = RUSTERIX.write().unwrap();
        shared::rusterix_utils::sync_procedural_avatar_assets(&mut runtime.assets, &p);
        host.snapshot = Some(Arc::new(runtime.assets.clone()));
    }
    let after = host.request(&p, (320, 320), [0, 0, 0, 255]).unwrap();
    let asset =
        rusterix::procedural_avatar::Runtime::resolve(&after.entity, &after.assets).unwrap();
    assert_ne!(
        equipped,
        asset
            .sprite(&after.entity, &after.assets, "idle", 0.)
            .unwrap()
    );
    // Check project undo/redo after saving appearance assets and character setup.
    let atom = ProjectUndoAtom::ProjectEdit(
        "Appearance review".into(),
        Box::new(before.clone()),
        Box::new(p.clone()),
    );
    let mut server = ServerContext::default();
    atom.undo(&mut p, &mut ui, &mut ctx, &mut server);
    assert_eq!(p.palette.colors, before.palette.colors);
    atom.redo(&mut p, &mut ui, &mut ctx, &mut server);
    assert_ne!(p.palette.colors, before.palette.colors);
    std::fs::write(out.join("saved-project.json"), saved).unwrap();
}
