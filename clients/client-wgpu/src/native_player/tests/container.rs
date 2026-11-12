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
    app.player.runtime.stop();
}
