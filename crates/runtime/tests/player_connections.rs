//! Several player connections share one world, with independent control/views.
use eldiron_runtime::{GameSession, PlayerConnectionId, PlayerStartOptions, SessionOptions};
use rusterix::{EntityAction, party::PlayerView};

fn world() -> GameSession {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    let mut destination = project.regions[0].clone();
    destination.id = theframework_core::Uuid::new_v4();
    destination.map.id = theframework_core::Uuid::new_v4();
    destination.name = "Connection destination".into();
    destination.map.name.clone_from(&destination.name);
    project.regions.push(destination);
    let mut world = GameSession::new(project);
    world.server.print_log_messages = false;
    world
        .start(SessionOptions {
            create_local_player: false,
            ..SessionOptions::terminal()
        })
        .unwrap();
    world
}

fn join(world: &mut GameSession, name: &str) -> PlayerConnectionId {
    let connection = world.open_player_connection().unwrap();
    let region = world.current_map.clone();
    let options = PlayerStartOptions {
        name: Some(name),
        ..Default::default()
    };
    assert!(
        world
            .start_connected_player(connection, &region, options)
            .unwrap()
            .is_some()
    );
    assert!(
        world
            .start_connected_player(connection, &region, options)
            .unwrap()
            .is_none()
    );
    connection
}

fn settle(world: &mut GameSession) {
    for _ in 0..3 {
        world.tick_once();
    }
}

#[test]
fn players_share_one_world_but_control_and_observe_their_own_character() {
    let mut world = world();
    let alice = join(&mut world, "Alice");
    let bob = join(&mut world, "Bob");
    assert!(
        world
            .connected_player_action(alice, EntityAction::Forward)
            .is_err(),
        "submission is not a registration acknowledgement"
    );
    assert!(
        world
            .view_for_connection(alice)
            .unwrap()
            .player_entity()
            .is_none()
    );
    settle(&mut world);
    let alice_control = world.server.player_control(alice).unwrap();
    let bob_control = world.server.player_control(bob).unwrap();
    assert_eq!(alice_control.region_id, bob_control.region_id);
    assert_ne!(alice_control.entity_id, bob_control.entity_id);
    assert!(
        !world.server.has_local_player(),
        "remote registration must not capture the implicit local connection"
    );
    assert!(world.view().player_entity().is_none());

    let alice_view = world.view_for_connection(alice).unwrap();
    let bob_view = world.view_for_connection(bob).unwrap();
    assert_eq!(
        alice_view
            .player_entity()
            .unwrap()
            .attributes
            .get_str("name"),
        Some("Alice")
    );
    assert_eq!(
        bob_view.player_entity().unwrap().attributes.get_str("name"),
        Some("Bob")
    );
    assert!(
        !std::ptr::eq(alice_view.map.unwrap(), bob_view.map.unwrap()),
        "each recipient owns an independent projection of the shared simulation"
    );
    assert_ne!(alice_view.session_id, bob_view.session_id);
    assert_eq!(
        bob_view
            .party_entity(bob_view.map.unwrap(), None)
            .unwrap()
            .id,
        bob_control.entity_id
    );
    assert!(
        !bob_view
            .player
            .members(bob_view.map.unwrap())
            .iter()
            .any(|e| e.id == alice_control.entity_id)
    );
    let initial_alice = alice_view.player_entity().unwrap().orientation;
    let initial_bob = bob_view.player_entity().unwrap().orientation;
    let mut alice_input = eldiron_runtime::PlayerInput::for_connection(alice);
    alice_input
        .submit_action(
            &mut world.server,
            true,
            EntityAction::SetPlayerCamera(rusterix::PlayerCamera::D3FirstPMouse),
        )
        .unwrap();
    alice_input
        .submit_action(&mut world.server, true, EntityAction::LookYaw(45.))
        .unwrap();
    world.server.local_player_action(EntityAction::LookYaw(60.));
    settle(&mut world);
    assert_ne!(
        world
            .view_for_connection(alice)
            .unwrap()
            .player_entity()
            .unwrap()
            .orientation,
        initial_alice
    );
    assert_eq!(
        world
            .view_for_connection(bob)
            .unwrap()
            .player_entity()
            .unwrap()
            .orientation,
        initial_bob
    );

    // The compatibility/local path also addresses a single character once one
    // is registered, even with several other active connections in the world.
    world
        .start_player(PlayerStartOptions {
            name: Some("Local"),
            ..Default::default()
        })
        .unwrap();
    settle(&mut world);
    let initial_alice = world
        .view_for_connection(alice)
        .unwrap()
        .player_entity()
        .unwrap()
        .orientation;
    let initial_local = world.view().player_entity().unwrap().orientation;
    world.action(EntityAction::SetPlayerCamera(
        rusterix::PlayerCamera::D3FirstPMouse,
    ));
    world.action(EntityAction::LookYaw(30.));
    settle(&mut world);
    assert_ne!(
        world.view().player_entity().unwrap().orientation,
        initial_local
    );
    assert_eq!(
        world
            .view_for_connection(alice)
            .unwrap()
            .player_entity()
            .unwrap()
            .orientation,
        initial_alice
    );

    let local_map = world.current_map.clone();
    world
        .server
        .player_teleport(alice, "entrance".into(), "Connection destination".into())
        .unwrap();
    settle(&mut world);
    let destination = world.server.region_name_id_map["Connection destination"];
    assert_eq!(
        world.server.player_control(alice).unwrap().region_id,
        destination
    );
    assert_eq!(
        world.server.player_control(bob).unwrap().region_id,
        bob_control.region_id
    );
    assert_eq!(
        world.view_for_connection(alice).unwrap().map.unwrap().name,
        "Connection destination"
    );
    assert_eq!(
        world.current_map, local_map,
        "another connection's transfer must not move the local view"
    );
    assert_eq!(
        world.view_for_connection(alice).unwrap().player,
        PlayerView::Controlled(alice_control.entity_id)
    );
}

#[test]
fn closing_pending_or_active_connections_revokes_control_and_restart_revokes_handles() {
    let mut world = world();
    let pending = join(&mut world, "Pending");
    let mut stale_input = eldiron_runtime::PlayerInput::for_connection(pending);
    stale_input
        .submit_action(&mut world.server, true, EntityAction::Forward)
        .unwrap();
    assert!(world.close_player_connection(pending));
    let active = join(&mut world, "Active");
    settle(&mut world);
    stale_input.flush(&mut world.server);
    assert!(
        stale_input
            .submit_action(&mut world.server, true, EntityAction::Forward)
            .is_err()
    );
    assert!(
        world.server.connection_state(pending).is_none(),
        "late spawn acknowledgement must not resurrect a closed connection"
    );
    assert!(world.view_for_connection(pending).is_err());
    let control = world.server.player_control(active).unwrap();
    world
        .connected_player_action(active, EntityAction::Forward)
        .unwrap();
    assert!(world.close_player_connection(active));
    assert!(!world.close_player_connection(active));
    assert!(
        world
            .connected_player_action(active, EntityAction::Forward)
            .is_err()
    );
    settle(&mut world);
    let context = world.server.instances[control.region_id as usize]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    assert_eq!(
        context
            .lock()
            .unwrap()
            .map
            .entities
            .iter()
            .find(|e| e.id == control.entity_id)
            .unwrap()
            .action,
        EntityAction::Off
    );
    let replacement = join(&mut world, "Replacement");
    settle(&mut world);
    assert_ne!(replacement, active);
    let local = world.server.local_connection();
    world.stop();
    assert_ne!(local, world.server.local_connection());
    assert!(
        world
            .server
            .player_action(replacement, EntityAction::Forward)
            .is_err()
    );
    assert!(world.view_for_connection(replacement).is_err());
}

#[test]
fn text_observers_keep_room_inventory_and_private_messages_bound_to_their_actor() {
    use rusterix::{Entity, Item, Map, Sector, Value};
    use shared::{
        text_game as text,
        text_session::{TextSession, TextSessionOutput},
    };
    let mut map = Map::default();
    for (id, offset, title) in [(1, 0., "West room"), (2, 10., "East room")] {
        let points = [
            (offset, 0.),
            (offset + 4., 0.),
            (offset + 4., 4.),
            (offset, 4.),
        ]
        .map(|(x, y)| map.add_vertex_at(x, y));
        let lines = (0..4)
            .map(|i| map.create_linedef_manual(points[i], points[(i + 1) % 4]))
            .collect();
        let mut sector = Sector::new(id, lines);
        sector.name = title.into();
        sector.properties.set(
            "data",
            Value::Str(format!("description='{title} description'")),
        );
        map.sectors.push(sector);
        let mut player = Entity::default();
        player.id = id;
        player.position.x = offset + 1.;
        player.position.z = 1.;
        player.set_attribute("player", Value::Bool(true));
        player.set_attribute("name", Value::Str(format!("Player {id}")));
        let mut item = Item::default();
        item.attributes
            .set("name", Value::Str(format!("Private item {id}")));
        player.inventory.push(Some(item));
        map.entities.push(player);
    }
    let first = PlayerView::Controlled(1);
    let second = PlayerView::Controlled(2);
    assert_eq!(
        text::build_text_room_for(&map, "", first).unwrap().title,
        "West room"
    );
    assert_eq!(
        text::build_text_room_for(&map, "", second).unwrap().title,
        "East room"
    );
    let inventory = text::render_player_inventory_for(&map, second).unwrap();
    assert!(inventory.contains("Private item 2"));
    assert!(!inventory.contains("Private item 1"));
    assert!(text::render_player_inventory_for(&map, PlayerView::Controlled(999)).is_none());
    map.sectors[0]
        .properties
        .set("iso_interior", Value::Bool(true));
    let mut cutaway = Item::default();
    cutaway
        .attributes
        .set("iso_cutaway_area", Value::Str("West room".into()));
    assert!(rusterix::isometric::item_cutaway_for(&map, &cutaway, first));
    assert!(!rusterix::isometric::item_cutaway_for(
        &map, &cutaway, second
    ));
    let mut session = TextSession::new();
    session.bind_player(second);
    let messages = vec![
        (None, None, 1, "For first player".into(), "status".into()),
        (None, None, 2, "For second player".into(), "status".into()),
    ];
    let output = session.collect(&map, "", messages, vec![], None, None, false);
    assert!(output.iter().any(|entry| matches!(entry, TextSessionOutput::Message { text, .. } if text == "For second player")));
    assert!(!output.iter().any(|entry| matches!(entry, TextSessionOutput::Message { text, .. } if text == "For first player")));
    // Death can stash the physical body before its private text is collected.
    map.entities.retain(|entity| entity.id != 2);
    let output = session.collect(
        &map,
        "",
        vec![(None, None, 2, "Your body is absent".into(), "status".into())],
        vec![],
        None,
        None,
        false,
    );
    assert!(output.iter().any(|entry| matches!(entry, TextSessionOutput::Message { text, .. } if text == "Your body is absent")));
}
