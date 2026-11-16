use super::*;
use eldiron_presentation::{HudPart, ScreenCapture};
use rusterix::{
    Item,
    client::{rules_ui, screen_model},
};
fn pump_until(
    app: &mut PlayerApplication,
    host: &mut HostContext,
    phase: &str,
    mut predicate: impl FnMut(&PlayerApplication) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        app.update(host, Duration::from_millis(20));
        assert!(app.failure().is_none(), "{:?}", app.failure());
        if predicate(app) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "runtime/HUD {phase} timed out: {}",
            app.player.runtime.server.log
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn pointer(app: &mut PlayerApplication, host: &mut HostContext, point: [f32; 2], pressed: bool) {
    let point = app
        .viewport()
        .host_rect(Rect::new(point[0], point[1], 1., 1.));
    let point = [point.x, point.y];
    assert!(
        app.route(
            &InputEvent::PointerButton {
                button: PointerButton::Primary,
                pressed,
                position: point
            },
            host
        )
        .unwrap()
        .consumed
    );
}
#[test]
#[ignore = "requires native GPU and real starter runtime"]
fn native_container_routes_one_real_server_transfer_and_keeps_its_owner_through_gpu_recovery() {
    let observed = Arc::new(Mutex::new(Observation::default()));
    let mut app = PlayerApplication::new(
        starter_with_menu(),
        false,
        Box::new(Services(observed.clone())),
    )
    .unwrap();
    app.player.runtime.server.print_log_messages = false;
    app.command(ClientCommandBinding::Game("start".into()))
        .unwrap();
    // Keep the actual session running while presenting a menu with no scene VM.
    app.navigate("Setup").unwrap();
    let mut host = HostContext::new((960, 540));
    host.resize((960, 540), 1.);
    pump_until(&mut app, &mut host, "registration", |app| {
        app.player.runtime.server.has_local_player()
            && app
                .player
                .runtime
                .view()
                .map
                .is_some_and(|map| map.entities.iter().any(|e| e.is_player()))
    });
    let map_id = app.player.runtime.view().map.unwrap().id;
    let region_id = app.player.runtime.server.region_id_map[&map_id];
    let instance = app
        .player
        .runtime
        .server
        .instances
        .iter()
        .find(|instance| instance.lock().unwrap().id == region_id)
        .unwrap()
        .clone();
    let context = instance.lock().unwrap().context().unwrap();
    // Fixture setup changes the authoritative region, never the client's copy.
    let (actor_id, bag_id, child_id, template) = {
        let mut ctx = context.lock().unwrap();
        let bag_id = ctx.world.next_id();
        let child_id = ctx.world.next_id();
        let mut child = Item::new();
        child.id = child_id;
        child.set_attribute("name", Value::Str("Native transfer item".into()));
        // A project-defined custom equippable item uses the normal server slot
        // validation; no authoritative permission or capacity check is skipped.
        child.set_attribute("slot", Value::Str("main_hand".into()));
        let mut bag = Item::new();
        bag.id = bag_id;
        bag.max_capacity = 4;
        bag.set_attribute("name", Value::Str("Native bag".into()));
        bag.container = Some(vec![child]);
        let template = rules_ui::container_template_for_item(&app.player.runtime.assets, &bag);
        let actor = ctx
            .map
            .entities
            .iter_mut()
            .find(|actor| actor.is_player())
            .unwrap();
        for slot in 0..actor.inventory.len() {
            actor.remove_item_from_slot(slot);
        }
        actor.inventory.resize(3, None);
        actor.set_attribute("inventory_slots", Value::Int(3));
        assert_eq!(actor.add_item(bag).unwrap(), 0);
        actor.mark_all_dirty();
        (actor.id, bag_id, child_id, template)
    };
    pump_until(&mut app, &mut host, "inventory publication", |app| {
        app.player
            .runtime
            .view()
            .map
            .unwrap()
            .entities
            .iter()
            .find(|entity| entity.id == actor_id)
            .is_some_and(|entity| {
                entity
                    .inventory
                    .iter()
                    .flatten()
                    .any(|item| item.id == bag_id)
            })
    });
    app.apply(vec![ScreenEffect::ToggleContainer {
        region: map_id,
        item: bag_id,
        owner: actor_id,
        anchor: Rect::new(200., 100., 32., 32.),
    }])
    .unwrap();
    assert!(
        observed.lock().unwrap().effects.is_empty(),
        "the common player owns container effects"
    );
    let geometry = screen_model::ContainerGeometry::new(
        Vec2::new(244, 100),
        &template,
        4,
        Vec2::new(960, 540),
    );
    let slot = geometry.slot(0).unwrap();
    let point = [slot.x + slot.width / 2., slot.y + slot.height / 2.];
    let gpu = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images = ImageRegistry::new(gpu.clone());
    let mut renderer = PaintRenderer::new(gpu.clone()).unwrap();
    let target = GpuImage::new(&gpu, ImageDescriptor::rgba(960, 540)).unwrap();
    app.initialize(&mut host, &mut images).unwrap();
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    let session_id = app.player.runtime.view().session_id;
    let source_builds = app.screen.source_builds();
    app.navigate("Setup").unwrap();
    render(&mut app, &mut host, &mut images, &mut renderer, &target);
    app.presented(&mut host);
    assert_eq!(
        app.screen.source_builds(),
        source_builds,
        "navigation retains the project's HUD source cache and missing-source memoization"
    );
    pointer(&mut app, &mut host, point, true);
    let original = match app.screen.control_capture().unwrap() {
        ScreenCapture::Hud(id) => id,
        other => panic!("HUD source owner: {other:?}"),
    };
    assert_eq!(original.part, HudPart::Slot(0));
    let gpu2 = futures::executor::block_on(GpuContext::headless()).unwrap();
    let mut images2 = ImageRegistry::new(gpu2.clone());
    let mut renderer2 = PaintRenderer::new(gpu2.clone()).unwrap();
    let target2 = GpuImage::new(&gpu2, ImageDescriptor::rgba(960, 540)).unwrap();
    app.gpu_recreated(&mut host, &mut images2).unwrap();
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    pointer(&mut app, &mut host, point, false);
    assert_eq!(app.player.runtime.view().session_id, session_id);
    {
        let ctx = context.lock().unwrap();
        let actor = ctx.map.entities.iter().find(|e| e.id == actor_id).unwrap();
        assert_eq!(
            actor.inventory[0]
                .as_ref()
                .unwrap()
                .container
                .as_ref()
                .unwrap()
                .len(),
            1,
            "late pre-recovery release did not pick up a child"
        );
    }
    pointer(&mut app, &mut host, point, true);
    assert_eq!(
        app.screen.control_capture(),
        Some(ScreenCapture::Hud(original)),
        "GPU replacement retains the CPU panel identity"
    );
    pointer(&mut app, &mut host, point, false);
    // A duplicate release cannot become a second child/world action.
    app.route(
        &InputEvent::PointerButton {
            button: PointerButton::Primary,
            pressed: false,
            position: point,
        },
        &mut host,
    )
    .unwrap();
    pump_until(&mut app, &mut host, "child pickup", |_| {
        let ctx = context.lock().unwrap();
        let actor = ctx.map.entities.iter().find(|e| e.id == actor_id).unwrap();
        actor
            .inventory
            .iter()
            .flatten()
            .any(|item| item.id == child_id)
    });
    {
        let ctx = context.lock().unwrap();
        let actor = ctx.map.entities.iter().find(|e| e.id == actor_id).unwrap();
        assert!(
            actor.inventory[0]
                .as_ref()
                .unwrap()
                .container
                .as_ref()
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            actor
                .inventory
                .iter()
                .flatten()
                .filter(|item| item.id == child_id)
                .count(),
            1
        );
    }
    // Server output is consumed once by the HUD, then stripped from service fan-out.
    app.player
        .runtime
        .server
        .open_container_requests
        .entry(region_id)
        .or_default()
        .push((bag_id, Some(actor_id)));
    app.update(&mut host, Duration::ZERO);
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    pointer(&mut app, &mut host, point, true);
    let reopened = match app.screen.control_capture().unwrap() {
        ScreenCapture::Hud(id) => id,
        other => panic!("reopened HUD: {other:?}"),
    };
    assert_ne!(original.panel, reopened.panel);
    pointer(&mut app, &mut host, point, false);
    app.update(&mut host, Duration::ZERO);
    pointer(&mut app, &mut host, point, true);
    assert_eq!(
        app.screen.control_capture(),
        Some(ScreenCapture::Hud(reopened)),
        "polling cannot reopen an already delivered container request"
    );
    pointer(&mut app, &mut host, point, false);
    assert_eq!(observed.lock().unwrap().containers_forwarded, 0);
    assert_eq!(gpu.transfers().readback_bytes, 0);
    assert_eq!(gpu2.transfers().readback_bytes, 0);
    // UI commands are consumed by the same retained presentation owner. The
    // actual server and its delta publication confirm this is not a wire-only
    // inventory fixture.
    pump_until(&mut app, &mut host, "child publication", |app| {
        app.player
            .runtime
            .view()
            .map
            .unwrap()
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .is_some_and(|actor| {
                actor
                    .inventory
                    .iter()
                    .flatten()
                    .any(|item| item.id == child_id)
            })
    });
    let inventory_config = screen_model::InventoryPanelConfig {
        columns: 3,
        rows: 1,
        cell_size: 32.,
        spacing: 4.,
        padding: 8.,
        title_height: 26.,
        tab_height: 24.,
        footer_height: 28.,
        authored_rect: Some(ScreenRect::new(24., 24., 400., 180.)),
        ..Default::default()
    };
    app.screen.elements.custom_inventory_panel_config = Some(inventory_config.clone());
    app.command(ClientCommandBinding::Ui("inventory".into()))
        .unwrap();
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    let geometry = {
        let world = app.player.runtime.view();
        let actor = world
            .map
            .unwrap()
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap();
        screen_model::inventory_panel_geometry(
            actor,
            &inventory_config,
            "all",
            "newest",
            0,
            None,
            Vec2::new(960, 540),
        )
    };
    let child_slot = geometry
        .slots()
        .find(|slot| slot.item_id == Some(child_id))
        .unwrap()
        .rect;
    let destination = geometry
        .slots()
        .find(|slot| slot.inventory_index == Some(2))
        .unwrap_or_else(|| {
            panic!(
                "expected inventory destination 2, projected slots: {:?}; actor: {:?}",
                geometry.slots().collect::<Vec<_>>(),
                app.player
                    .runtime
                    .view()
                    .map
                    .unwrap()
                    .entities
                    .iter()
                    .find(|actor| actor.id == actor_id)
                    .unwrap()
                    .inventory
                    .iter()
                    .enumerate()
                    .map(|(index, item)| (
                        index,
                        item.as_ref().map(|item| (item.id, item.item_type.clone()))
                    ))
                    .collect::<Vec<_>>()
            )
        })
        .rect;
    let child_point = [
        child_slot.x + child_slot.width / 2.,
        child_slot.y + child_slot.height / 2.,
    ];
    let target_point = [
        destination.x + destination.width / 2.,
        destination.y + destination.height / 2.,
    ];
    pointer(&mut app, &mut host, child_point, true);
    assert!(
        matches!(app.screen.control_capture(), Some(ScreenCapture::Hud(id))
        if matches!(id.panel, eldiron_presentation::HudPanelId::Inventory { entity, .. } if entity == actor_id))
    );
    let host_target = app
        .viewport()
        .host_rect(Rect::new(target_point[0], target_point[1], 1., 1.));
    app.route(
        &InputEvent::PointerMoved {
            position: [host_target.x, host_target.y],
        },
        &mut host,
    )
    .unwrap();
    let submitted = app.player.input_stats().submitted_actions;
    assert_eq!(
        app.screen.drag_target_feedback(&app.player.runtime.view()),
        Some(destination)
    );
    assert_eq!(
        app.player.input_stats().submitted_actions,
        submitted,
        "highlight query never submits a transfer"
    );
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    pointer(&mut app, &mut host, target_point, false);
    pump_until(&mut app, &mut host, "native inventory transfer", |_| {
        let ctx = context.lock().unwrap();
        let actor = ctx
            .map
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap();
        actor
            .inventory
            .get(2)
            .and_then(Option::as_ref)
            .is_some_and(|item| item.id == child_id)
            && actor
                .inventory
                .iter()
                .flatten()
                .filter(|item| item.id == child_id)
                .count()
                == 1
    });
    pump_until(
        &mut app,
        &mut host,
        "native inventory transfer publication",
        |app| {
            app.player
                .runtime
                .view()
                .map
                .unwrap()
                .entities
                .iter()
                .find(|actor| actor.id == actor_id)
                .is_some_and(|actor| {
                    actor
                        .inventory
                        .get(2)
                        .and_then(Option::as_ref)
                        .is_some_and(|item| item.id == child_id)
                })
        },
    );
    let mut source_widget = rusterix::client::widget::Widget::new();
    source_widget.id = 80;
    source_widget.name = "Owned item exchange".into();
    source_widget.rect = ScreenRect::new(600., 300., 40., 40.);
    source_widget.inventory_index = Some(2);
    source_widget.drag_drop = true;
    app.screen.elements.button_widgets.insert(80, source_widget);
    let equipment_config = screen_model::EquipmentPanelConfig {
        slot_size: 32.,
        padding: 8.,
        title_height: 26.,
        avatar_width: 60.,
        avatar_height: 120.,
        left_slots: vec!["MAIN_HAND".into()],
        authored_rect: Some(ScreenRect::new(24., 24., 400., 220.)),
        ..Default::default()
    };
    app.screen.elements.custom_equipment_panel_config = Some(equipment_config.clone());
    app.command(ClientCommandBinding::Ui("equipment".into()))
        .unwrap();
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    let geometry = {
        let world = app.player.runtime.view();
        let actor = world
            .map
            .unwrap()
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap();
        screen_model::equipment_panel_geometry(
            actor,
            &equipment_config,
            world.assets,
            None,
            Vec2::new(960, 540),
        )
    };
    let equipment_slot = geometry
        .slots
        .iter()
        .find(|slot| slot.slot == "main_hand")
        .unwrap()
        .rect;
    let equipment_point = [
        equipment_slot.x + equipment_slot.width / 2.,
        equipment_slot.y + equipment_slot.height / 2.,
    ];
    pointer(&mut app, &mut host, [620., 320.], true);
    pointer(&mut app, &mut host, equipment_point, false);
    pump_until(&mut app, &mut host, "native equipment transfer", |_| {
        context
            .lock()
            .unwrap()
            .map
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap()
            .get_equipped_item("main_hand")
            .is_some_and(|item| item.id == child_id)
    });
    pump_until(&mut app, &mut host, "native equipment publication", |app| {
        app.player
            .runtime
            .view()
            .map
            .unwrap()
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap()
            .get_equipped_item("main_hand")
            .is_some_and(|item| item.id == child_id)
    });
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    pointer(&mut app, &mut host, equipment_point, true);
    assert!(
        matches!(app.screen.control_capture(), Some(ScreenCapture::Hud(id))
        if matches!(id.panel, eldiron_presentation::HudPanelId::Equipment { entity, .. } if entity == actor_id))
    );
    pointer(&mut app, &mut host, [620., 320.], false);
    pump_until(&mut app, &mut host, "native equipment return", |_| {
        let ctx = context.lock().unwrap();
        let actor = ctx
            .map
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap();
        actor
            .inventory
            .get(2)
            .and_then(Option::as_ref)
            .is_some_and(|item| item.id == child_id)
            && actor
                .inventory
                .iter()
                .flatten()
                .chain(actor.equipped.values())
                .filter(|item| item.id == child_id)
                .count()
                == 1
    });
    assert!(
        observed.lock().unwrap().effects.is_empty(),
        "native inventory commands never reach application service fallback"
    );
    assert_eq!(gpu2.transfers().readback_bytes, 0);
    // The native catalogue sends its addressed action through the same live
    // session; verify both authoritative mutation and normal delta publication.
    let config = screen_model::CatalogPanelConfig {
        columns: 2,
        rows: Some(1),
        cell_size: 40.,
        padding: 8.,
        title_height: 26.,
        tab_height: 24.,
        show_details: false,
        authored_rect: Some(ScreenRect::new(24., 24., 400., 220.)),
        ..Default::default()
    };
    let geometry = {
        let world = app.player.runtime.view();
        let actor = world
            .map
            .unwrap()
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .unwrap();
        let groups = rules_ui::action_catalog(world.assets, Some(actor));
        screen_model::catalog_panel_geometry(&groups, &config, "all", 0, None, Vec2::new(960, 540))
    };
    let entry = geometry
        .entries
        .first()
        .expect("starter player has rules catalogue actions");
    let command = entry.command.clone();
    let source = [
        entry.rect.x + entry.rect.width / 2.,
        entry.rect.y + entry.rect.height / 2.,
    ];
    let mut target_widget = rusterix::client::widget::Widget::new();
    target_widget.id = 81;
    target_widget.name = "Command assignment".into();
    target_widget.rect = ScreenRect::new(700., 360., 40., 40.);
    target_widget.command_slot = Some("main.9".into());
    app.screen.elements.button_widgets.insert(81, target_widget);
    app.screen.elements.custom_actions_panel_config = Some(config);
    app.command(ClientCommandBinding::Ui("actions".into()))
        .unwrap();
    render(&mut app, &mut host, &mut images2, &mut renderer2, &target2);
    app.presented(&mut host);
    pointer(&mut app, &mut host, source, true);
    assert!(
        matches!(app.screen.control_capture(), Some(ScreenCapture::Hud(id))
        if matches!(id.part, HudPart::CatalogEntry(_)))
    );
    pointer(&mut app, &mut host, [720., 380.], false);
    pump_until(&mut app, &mut host, "native command assignment", |_| {
        context
            .lock()
            .unwrap()
            .map
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| actor.attributes.get_str("command_slot_main_9"))
            == Some(command.as_str())
    });
    pump_until(&mut app, &mut host, "native command publication", |app| {
        app.player
            .runtime
            .view()
            .map
            .unwrap()
            .entities
            .iter()
            .find(|actor| actor.id == actor_id)
            .and_then(|actor| actor.attributes.get_str("command_slot_main_9"))
            == Some(command.as_str())
    });
    assert!(
        observed.lock().unwrap().effects.is_empty(),
        "catalogue commands stay in the common session"
    );
    assert_eq!(gpu2.transfers().readback_bytes, 0);
    app.player.runtime.stop();
}
