//! Replicas consume serializable CPU packets and separately provisioned content.
use eldiron_runtime::{
    ClientFrame, ClientWorld, GameSession, PlayerConnectionId, Project, RegionEvents,
    ReplicationError, SessionOptions,
    replication::{ApplyResult, MAX_QUEUED_FRAMES, RegionDelivery, WorldUpdate},
};
use rusterix::{
    Entity, EntityAction, Item, Value,
    server::message::{AudioCommand, RegionMessage},
};
use theframework_core::Uuid;

fn item(id: u32, name: &str) -> Item {
    let mut item = Item::default();
    item.id = id;
    item.attributes.set("name", Value::Str(name.into()));
    item
}

fn world() -> GameSession {
    let mut project = Project::new();
    project.regions[0].name = "Replicas".into();
    project.regions[0].map.name = "Replicas".into();
    project.config = "[game]\nstart_region='Replicas'\nauto_create_player=false".into();
    let mut world = GameSession::new(project);
    world.server.print_log_messages = false;
    world
        .start(SessionOptions {
            create_local_player: false,
            ..Default::default()
        })
        .unwrap();
    world
}
fn join(world: &mut GameSession, name: &str) -> PlayerConnectionId {
    let connection = world.open_player_connection().unwrap();
    let mut actor = Entity::default();
    actor.set_attribute("player", Value::Bool(true));
    actor.set_attribute("class_name", Value::Str("Replica".into()));
    actor.set_attribute("name", Value::Str(name.into()));
    let region = world.project.regions[0].map.id;
    let context = world.server.instances[0].lock().unwrap().context().unwrap();
    context
        .lock()
        .unwrap()
        .entity_player_classes
        .insert("Replica".into());
    world
        .server
        .create_connected_player(connection, &region, actor)
        .unwrap();
    for _ in 0..3 {
        assert!(world.tick_once().replication_error.is_none());
    }
    assert!(world.server.player_control(connection).is_some());
    connection
}
fn baseline(world: &mut GameSession, connection: PlayerConnectionId) -> ClientFrame {
    world.subscribe_replication(connection).unwrap();
    world.take_client_frames(connection).unwrap().pop().unwrap()
}
fn receiver(world: &GameSession, frame: &ClientFrame) -> ClientWorld {
    let mut client = ClientWorld::new(frame.world, frame.connection);
    let WorldUpdate::Baseline(regions) = &frame.update else {
        panic!("baseline expected");
    };
    for region in regions {
        let source = &world
            .project
            .regions
            .iter()
            .find(|r| r.map.id == region.content.map)
            .unwrap()
            .map;
        client.install_content(region.content, source).unwrap();
    }
    client
}

#[test]
fn late_join_private_state_deltas_and_events_use_independent_owned_replicas() {
    let mut host = world();
    let alice = join(&mut host, "Alice");
    let bob = join(&mut host, "Bob");
    let a = host.server.player_control(alice).unwrap();
    let b = host.server.player_control(bob).unwrap();
    let region = host.project.regions[0].map.id;
    let context = host.server.instances[a.region_id as usize]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    {
        let mut ctx = context.lock().unwrap();
        for (actor, secret, id) in [
            (a.entity_id, "ALICE_PRIVATE", 1001),
            (b.entity_id, "BOB_PRIVATE", 2001),
        ] {
            let entity = ctx.map.entities.iter_mut().find(|e| e.id == actor).unwrap();
            entity
                .attributes
                .set("quest_note", Value::Str(secret.into()));
            entity
                .attributes
                .set("_server_secret", Value::Str("HOST_ONLY".into()));
            let mut bag = item(id, secret);
            bag.container = Some(vec![item(id + 1, secret)]);
            entity.inventory = vec![Some(bag), None];
            entity
                .equipped
                .insert("hand".into(), item(id + 2, "Visible sword"));
            entity
                .equipped
                .get_mut("hand")
                .unwrap()
                .attributes
                .set("appearance_hold_joint", Value::Str("hand.R".into()));
            entity.wallet.balances.insert("gold".into(), id as i64);
            entity.clear_dirty(); // late join must not depend on global dirty flags
        }
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::WorldText(
                a.region_id,
                a.entity_id,
                rusterix::world_text::WorldText {
                    anchor: rusterix::world_text::TextAnchor::Entity(b.entity_id),
                    text: "Alice inspection".into(),
                },
            ))
            .unwrap();
    }
    assert!(host.poll().replication_error.is_none());
    let a_frame = baseline(&mut host, alice);
    let b_frame = baseline(&mut host, bob);
    let json = serde_json::to_string(&a_frame).unwrap();
    assert!(json.contains("ALICE_PRIVATE"));
    assert!(!json.contains("BOB_PRIVATE"));
    assert!(!json.contains("HOST_ONLY"));
    let decoded: ClientFrame = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, a_frame);
    let mut client = receiver(&host, &decoded);
    assert_eq!(client.apply(&decoded), Ok(ApplyResult::Applied));
    let actor = client
        .map(region)
        .unwrap()
        .entities
        .iter()
        .find(|e| e.id == a.entity_id)
        .unwrap();
    assert_eq!(actor.inventory.len(), 2);
    assert_eq!(
        actor.inventory[0]
            .as_ref()
            .unwrap()
            .container
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(actor.wallet.balances["gold"], 1001);
    assert_eq!(actor.equipped["hand"].id, 1003);
    let other = client
        .map(region)
        .unwrap()
        .entities
        .iter()
        .find(|e| e.id == b.entity_id)
        .unwrap();
    assert!(other.inventory.is_empty() && other.wallet.balances.is_empty());
    assert_eq!(other.equipped["hand"].id, 2003);
    assert_eq!(
        other.equipped["hand"]
            .attributes
            .get_str("appearance_hold_joint"),
        Some("hand.R")
    );
    assert!(!std::ptr::eq(
        client.map(region).unwrap(),
        &host.project.regions[0].map
    ));
    assert_eq!(
        host.take_connection_events(alice, region)
            .unwrap()
            .world_text
            .len(),
        1
    );
    assert!(
        host.take_connection_events(bob, region)
            .unwrap()
            .world_text
            .is_empty()
    );

    host.server.messages.insert(
        a.region_id,
        vec![
            (
                None,
                None,
                a.entity_id,
                "Alice message".into(),
                "status".into(),
            ),
            (
                None,
                None,
                b.entity_id,
                "Bob message".into(),
                "status".into(),
            ),
        ],
    );
    host.server.says.insert(
        a.region_id,
        vec![(
            Some(a.entity_id),
            None,
            "Public speech".into(),
            "say".into(),
        )],
    );
    host.server
        .audio_commands
        .insert(a.region_id, vec![AudioCommand::ClearAll]);
    let mut choice =
        rusterix::MultipleChoice::new(a.region_id, a.entity_id, b.entity_id, i64::MAX, 2.);
    choice.add(rusterix::Choice::Cancel(
        a.entity_id,
        b.entity_id,
        i64::MAX,
        2.,
    ));
    host.server
        .multiple_choice
        .insert(a.region_id, vec![choice]);
    {
        let mut ctx = context.lock().unwrap();
        let actor = ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == a.entity_id)
            .unwrap();
        actor.attributes.remove("quest_note");
        actor.inventory[0] = None;
        actor.equipped.clear();
        actor.wallet.balances.clear();
        actor.clear_dirty();
    }
    assert!(host.poll().replication_error.is_none());
    let a_events = host.take_connection_events(alice, region).unwrap();
    let b_events = host.take_connection_events(bob, region).unwrap();
    assert_eq!(a_events.messages.len(), 1);
    assert_eq!(a_events.messages[0].3, "Alice message");
    assert_eq!(b_events.messages[0].3, "Bob message");
    assert_eq!(a_events.says, b_events.says);
    assert_eq!(a_events.audio, b_events.audio);
    assert!(a_events.choices.is_empty());
    assert_eq!(b_events.choices.len(), 1);
    assert!(
        host.take_events(&region).messages.is_empty(),
        "uncontrolled local observer gets no private events"
    );
    let deltas = host.take_client_frames(alice).unwrap();
    assert_eq!(deltas.len(), 1);
    assert!(matches!(deltas[0].update, WorldUpdate::Delta(_)));
    client
        .apply(
            &serde_json::from_str::<ClientFrame>(&serde_json::to_string(&deltas[0]).unwrap())
                .unwrap(),
        )
        .unwrap();
    let actor = client
        .map(region)
        .unwrap()
        .entities
        .iter()
        .find(|e| e.id == a.entity_id)
        .unwrap();
    assert!(actor.inventory[0].is_none());
    assert!(actor.equipped.is_empty() && actor.wallet.balances.is_empty());
    assert!(actor.attributes.get("quest_note").is_none());
    assert_eq!(client.take_events(region).messages.len(), 1);
    assert_eq!(client.apply(&deltas[0]), Ok(ApplyResult::IgnoredStale));
    assert!(client.take_events(region).messages.is_empty());

    // The receiver can present its owned state after the host is gone.
    let content = host.project.clone();
    let assets = rusterix::Assets::default();
    assert_ne!(a_frame.connection, b_frame.connection);
    drop(host);
    assert_eq!(
        client
            .view(&content, &assets, None)
            .unwrap()
            .player_entity()
            .unwrap()
            .id,
        a.entity_id
    );
}

#[test]
fn shared_container_and_trade_visibility_are_granted_and_revoked_per_actor() {
    let mut host = world();
    let alice = join(&mut host, "Alice");
    let bob = join(&mut host, "Bob");
    let a = host.server.player_control(alice).unwrap();
    let b = host.server.player_control(bob).unwrap();
    let region = host.project.regions[0].map.id;
    let context = host.server.instances[a.region_id as usize]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    {
        let mut ctx = context.lock().unwrap();
        let position = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == a.entity_id)
            .unwrap()
            .position;
        for entity in &mut ctx.map.entities {
            entity.position = position;
        }
        let mut chest = item(3001, "Shared chest");
        chest.position = position;
        chest.container = Some(vec![item(3002, "CHEST_SECRET")]);
        ctx.map.items.push(chest);
        let bob = ctx
            .map
            .entities
            .iter_mut()
            .find(|e| e.id == b.entity_id)
            .unwrap();
        let mut bag = item(4001, "Bob bag");
        bag.container = Some(vec![item(4002, "BOB_BAG_SECRET")]);
        let mut offer = item(4003, "Offered apple");
        offer.attributes.set("worth", Value::Int(7));
        bob.inventory = vec![Some(bag), Some(offer), Some(item(4004, "UNLISTED_SECRET"))];
    }
    host.poll();
    assert!(
        !serde_json::to_string(&baseline(&mut host, alice))
            .unwrap()
            .contains("CHEST_SECRET")
    );
    for (connection, actor) in [(alice, a.entity_id), (bob, b.entity_id)] {
        host.connected_player_action(
            connection,
            EntityAction::OpenContainer {
                item_id: 3001,
                owner_entity_id: None,
            },
        )
        .unwrap();
        assert!(host.poll().replication_error.is_none());
        assert_eq!(
            host.server.player_container_access(connection),
            Some((3001, None)),
            "actor {actor}"
        );
        assert_eq!(
            host.take_connection_events(connection, region)
                .unwrap()
                .containers,
            vec![(3001, None)]
        );
        let other = if connection == alice { bob } else { alice };
        assert!(
            host.take_connection_events(other, region)
                .unwrap()
                .containers
                .is_empty()
        );
        let frame = baseline(&mut host, connection);
        assert!(
            serde_json::to_string(&frame)
                .unwrap()
                .contains("CHEST_SECRET")
        );
    }
    // Claiming another live player's ownership is not authorization.
    host.connected_player_action(
        alice,
        EntityAction::OpenContainer {
            item_id: 4001,
            owner_entity_id: Some(b.entity_id),
        },
    )
    .unwrap();
    host.poll();
    assert!(
        host.take_connection_events(alice, region)
            .unwrap()
            .containers
            .is_empty()
    );
    assert!(
        !serde_json::to_string(&baseline(&mut host, alice))
            .unwrap()
            .contains("BOB_BAG_SECRET")
    );
    {
        let mut ctx = context.lock().unwrap();
        ctx.active_choice_sessions
            .push(rusterix::server::regionctx::ChoiceSession {
                from: b.entity_id,
                to: a.entity_id,
                expires_at_tick: i64::MAX,
                max_distance: 2.,
            });
        let mut choices =
            rusterix::MultipleChoice::new(a.region_id, b.entity_id, a.entity_id, i64::MAX, 2.);
        choices.add(rusterix::Choice::ItemToSell(
            4003,
            b.entity_id,
            a.entity_id,
            i64::MAX,
            2.,
        ));
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::MultipleChoice(choices))
            .unwrap();
    }
    assert!(host.poll().replication_error.is_none());
    let view = host.view_for_connection(alice).unwrap();
    let seller = view
        .map
        .unwrap()
        .entities
        .iter()
        .find(|e| e.id == b.entity_id)
        .unwrap();
    assert_eq!(seller.inventory.len(), 1);
    assert_eq!(
        seller.inventory[0]
            .as_ref()
            .unwrap()
            .attributes
            .get_int("worth"),
        Some(7)
    );
    assert!(
        !serde_json::to_string(&baseline(&mut host, alice))
            .unwrap()
            .contains("UNLISTED_SECRET")
    );
    {
        let mut ctx = context.lock().unwrap();
        ctx.map
            .entities
            .iter_mut()
            .find(|e| e.id == a.entity_id)
            .unwrap()
            .position
            .x += 100.;
    }
    assert!(host.poll().replication_error.is_none());
    let packet = baseline(&mut host, alice);
    let json = serde_json::to_string(&packet).unwrap();
    assert!(!json.contains("CHEST_SECRET") && !json.contains("Offered apple"));
    assert!(
        serde_json::to_string(&baseline(&mut host, bob))
            .unwrap()
            .contains("CHEST_SECRET")
    );
}

#[test]
fn ordering_content_readiness_resync_and_backpressure_are_explicit() {
    let mut host = world();
    let connection = join(&mut host, "Receiver");
    let frame = baseline(&mut host, connection);
    let region = frame.current_region.unwrap();
    let mut client = ClientWorld::new(frame.world, frame.connection);
    assert!(matches!(
        client.apply(&frame),
        Err(ReplicationError::MissingContent(_))
    ));
    assert_eq!(client.sequence(), 0);
    client = receiver(&host, &frame);
    client.apply(&frame).unwrap();
    let view_id = client
        .view(&host.project, &host.assets, None)
        .unwrap()
        .session_id;
    host.poll();
    let delta = host.take_client_frames(connection).unwrap().pop().unwrap();
    for mutate in [0, 1, 2, 3, 4] {
        let mut invalid = delta.clone();
        match mutate {
            0 => invalid.connection = Uuid::new_v4(),
            1 => invalid.world = Uuid::new_v4(),
            2 => invalid.version += 1,
            3 => invalid.sequence += 1,
            _ => invalid.generation += 1,
        }
        assert!(client.apply(&invalid).is_err());
        assert_eq!(client.sequence(), frame.sequence);
        assert_eq!(
            client
                .view(&host.project, &host.assets, None)
                .unwrap()
                .session_id,
            view_id
        );
    }
    let mut malformed = delta.clone();
    malformed.events = vec![RegionDelivery {
        region,
        events: RegionEvents {
            messages: vec![(None, None, 99999, "Wrong recipient".into(), "status".into())],
            ..Default::default()
        },
    }];
    assert!(client.apply(&malformed).is_err());
    client.apply(&delta).unwrap();
    assert_eq!(client.apply(&frame), Ok(ApplyResult::IgnoredStale));
    assert_eq!(
        client
            .view(&host.project, &host.assets, None)
            .unwrap()
            .session_id,
        view_id
    );
    host.request_baseline(connection).unwrap();
    let replacement = host.take_client_frames(connection).unwrap().pop().unwrap();
    assert!(replacement.generation > frame.generation);
    client.apply(&replacement).unwrap();
    assert_ne!(
        client
            .view(&host.project, &host.assets, None)
            .unwrap()
            .session_id,
        view_id
    );
    assert_eq!(client.apply(&delta), Ok(ApplyResult::IgnoredStale));
    for _ in 0..=MAX_QUEUED_FRAMES {
        assert!(host.poll().replication_error.is_none());
    }
    assert_eq!(
        host.take_client_frames(connection),
        Err(ReplicationError::Backpressure)
    );
    host.subscribe_replication(connection).unwrap();
    let latest = host.take_client_frames(connection).unwrap().pop().unwrap();
    assert!(matches!(latest.update, WorldUpdate::Baseline(_)));
    client.apply(&latest).unwrap();
    assert_eq!(client.sequence(), latest.sequence);
    assert!(host.close_player_connection(connection));
    assert!(host.client_world(connection).is_none());
    assert_eq!(
        host.take_client_frames(connection),
        Err(ReplicationError::UnknownConnection)
    );
}

fn typed_fixture() -> (ClientWorld, ClientFrame, rusterix::Map) {
    use eldiron_runtime::replication::{EntitySnapshot, RegionContentRef, RegionSnapshot};
    let map = rusterix::Map::default();
    let reference = RegionContentRef {
        map: map.id,
        map_revision: map.changed,
        runtime_revision: 0,
    };
    let frame = ClientFrame {
        version: eldiron_runtime::replication::REPLICATION_VERSION,
        world: Uuid::new_v4(),
        connection: Uuid::new_v4(),
        generation: 1,
        sequence: 1,
        tick: 0,
        movement_tick: 0,
        motion: None,
        controlled_entity: Some(1),
        current_region: Some(map.id),
        state: rusterix::ServerState::Running,
        update: WorldUpdate::Baseline(vec![RegionSnapshot {
            content: reference,
            entities: vec![EntitySnapshot {
                id: 1,
                creator_id: Uuid::new_v4(),
                position: [0.; 3],
                orientation: [1., 0.],
                tilt: 0.,
                motion_revision: 0,
                attributes: Default::default(),
                equipped: Default::default(),
                inventory: Some(vec![]),
                wallet: Some(Default::default()),
            }],
            items: vec![],
            props: vec![],
                resource_views: vec![],
            time: Default::default(),
            render: Default::default(),
        }]),
        events: vec![],
    };
    let mut client = ClientWorld::new(frame.world, frame.connection);
    client.install_content(reference, &map).unwrap();
    (client, frame, map)
}

#[test]
fn content_replacement_removals_and_invalid_object_trees_apply_atomically() {
    use eldiron_runtime::replication::{ItemSnapshot, RegionDelta};
    let (mut client, mut frame, mut source) = typed_fixture();
    let WorldUpdate::Baseline(regions) = &mut frame.update else {
        unreachable!()
    };
    let reference = regions[0].content;
    regions[0].items.push(ItemSnapshot {
        id: 2,
        creator_id: Uuid::new_v4(),
        position: [0.; 3],
        item_type: "Chest".into(),
        max_capacity: 4,
        is_container: true,
        attributes: Default::default(),
        contents: Some(vec![]),
    });
    client.apply(&frame).unwrap();
    let mut delta = frame.clone();
    delta.sequence += 1;
    delta.update = WorldUpdate::Delta(vec![RegionDelta {
        poses: vec![],
        content: reference,
        entities: vec![],
        removed_entities: vec![1],
        items: vec![],
        removed_items: vec![2],
        props: None,
        resource_views: None,
        time: Default::default(),
        render: Default::default(),
    }]);
    let mut invalid = delta.clone();
    if let WorldUpdate::Delta(regions) = &mut invalid.update {
        regions[0].removed_entities.push(1);
    }
    assert!(client.apply(&invalid).is_err());
    assert_eq!(client.map(source.id).unwrap().entities.len(), 1);
    client.apply(&delta).unwrap();
    assert!(client.map(source.id).unwrap().entities.is_empty());
    assert!(client.map(source.id).unwrap().items.is_empty());

    let mut replacement = frame.clone();
    replacement.generation += 1;
    source.changed += 1;
    source.name = "Updated geometry".into();
    if let WorldUpdate::Baseline(regions) = &mut replacement.update {
        regions[0].content.map_revision = source.changed;
        regions[0].content.runtime_revision += 1;
        let root = regions[0].items[0].clone();
        regions[0].items[0].contents = Some(vec![root]); // duplicate nested ID
    }
    assert!(client.apply(&replacement).is_err());
    assert_eq!(client.generation(), 1);
    if let WorldUpdate::Baseline(regions) = &mut replacement.update {
        regions[0].items[0].contents = Some(vec![]);
    }
    let WorldUpdate::Baseline(regions) = &replacement.update else {
        unreachable!()
    };
    assert_eq!(
        client.apply(&replacement),
        Err(ReplicationError::MissingContent(regions[0].content))
    );
    client.install_content(regions[0].content, &source).unwrap();
    client.apply(&replacement).unwrap();
    assert_eq!(client.current_map().unwrap().name, "Updated geometry");
    assert_eq!(client.apply(&frame), Ok(ApplyResult::IgnoredStale));
}

#[test]
fn unread_events_are_bounded_and_overflow_is_reported_without_stalling_world_updates() {
    use eldiron_runtime::replication::{MAX_PENDING_EVENTS, RegionDelta};
    let (mut client, frame, map) = typed_fixture();
    client.apply(&frame).unwrap();
    let WorldUpdate::Baseline(regions) = &frame.update else {
        unreachable!()
    };
    let mut delta = frame.clone();
    delta.update = WorldUpdate::Delta(vec![RegionDelta {
        poses: vec![],
        content: regions[0].content,
        entities: vec![],
        removed_entities: vec![],
        items: vec![],
        removed_items: vec![],
        props: None,
        resource_views: None,
        time: Default::default(),
        render: Default::default(),
    }]);
    delta.events = vec![RegionDelivery {
        region: map.id,
        events: RegionEvents {
            messages: vec![(None, None, 1, "Transient".into(), "status".into()); 4096],
            ..Default::default()
        },
    }];
    for _ in 0..=MAX_PENDING_EVENTS / 4096 {
        delta.sequence += 1;
        assert_eq!(client.apply(&delta), Ok(ApplyResult::Applied));
    }
    assert_eq!(client.sequence(), delta.sequence);
    assert!(client.take_event_overflow());
    assert!(!client.take_event_overflow());
    assert_eq!(
        client.take_events(map.id).messages.len(),
        MAX_PENDING_EVENTS
    );
    delta.sequence += 1;
    client.apply(&delta).unwrap();
    assert_eq!(client.take_events(map.id).messages.len(), 4096);
}

#[test]
fn separately_installed_prop_materials_survive_sanitized_snapshots_and_resync() {
    let (_, mut frame, mut source) = typed_fixture();
    let mut prop = rusterix::BlockPropInstance::new(Uuid::new_v4());
    prop.parameter_overrides.set("color", Value::Int(12));
    prop.runtime_state
        .set("secret", Value::Str("Not runtime content".into()));
    source.block_prop_instances.push(prop.clone());
    prop.parameter_overrides = Default::default();
    prop.runtime_state = Default::default();
    prop.runtime_state.set("open", Value::Bool(true));
    let WorldUpdate::Baseline(regions) = &mut frame.update else {
        unreachable!()
    };
    regions[0].props.push(prop);
    let reference = regions[0].content;
    let mut client = ClientWorld::new(frame.world, frame.connection);
    client.install_content(reference, &source).unwrap();
    client.apply(&frame).unwrap();
    let applied = &client.current_map().unwrap().block_prop_instances[0];
    assert_eq!(applied.parameter_overrides.get_int("color"), Some(12));
    assert_eq!(applied.runtime_state.get_bool("open"), Some(true));
    assert!(applied.runtime_state.get("secret").is_none());
    frame.generation += 1;
    client.apply(&frame).unwrap();
    assert_eq!(
        client.current_map().unwrap().block_prop_instances[0]
            .parameter_overrides
            .get_int("color"),
        Some(12)
    );
}

#[test]
fn poses_preserve_private_state_and_reject_unknown_conflicting_or_invalid_updates_atomically() {
    use eldiron_runtime::replication::{EntityPose, RegionDelta};
    let (mut client, mut frame, map) = typed_fixture();
    let WorldUpdate::Baseline(regions) = &mut frame.update else {
        panic!()
    };
    let reference = regions[0].content;
    regions[0].entities[0]
        .attributes
        .insert("private".into(), Value::Str("keep".into()));
    regions[0].entities[0]
        .wallet
        .as_mut()
        .unwrap()
        .insert("gold".into(), 7);
    let original = regions[0].entities[0].clone();
    client.apply(&frame).unwrap();
    let mut delta = frame.clone();
    delta.sequence += 1;
    let pose = EntityPose {
        id: 1,
        position: [2., 0., 3.],
        orientation: [0., 1.],
        tilt: 0.2,
        motion_revision: 3,
    };
    delta.update = WorldUpdate::Delta(vec![RegionDelta {
        content: reference,
        poses: vec![pose],
        entities: vec![],
        removed_entities: vec![],
        items: vec![],
        removed_items: vec![],
        props: None,
        resource_views: None,
        time: Default::default(),
        render: Default::default(),
    }]);
    for mode in 0..5 {
        let mut invalid = delta.clone();
        let WorldUpdate::Delta(regions) = &mut invalid.update else {
            panic!()
        };
        match mode {
            0 => regions[0].poses[0].id = 99,
            1 => regions[0].poses[0].position[0] = f32::NAN,
            2 => {
                let duplicate = regions[0].poses[0].clone();
                regions[0].poses.push(duplicate);
            }
            3 => regions[0].entities.push(original.clone()),
            _ => regions[0].removed_entities.push(1),
        }
        assert!(client.apply(&invalid).is_err());
        assert_eq!(client.sequence(), 1);
        assert_eq!(client.map(map.id).unwrap().entities[0].position.x, 0.);
    }
    client.apply(&delta).unwrap();
    let actor = &client.map(map.id).unwrap().entities[0];
    assert_eq!(actor.position.x, 2.);
    assert_eq!(actor.wallet.balances["gold"], 7);
    assert_eq!(actor.attributes.get_str("private"), Some("keep"));
    assert_eq!(actor.attributes.get_uint("__motion_revision"), Some(3));
    assert_eq!(client.apply(&delta), Ok(ApplyResult::IgnoredStale));
}

#[test]
fn shared_public_projection_sends_poses_and_revokes_private_party_access_with_replacements() {
    let mut host = world();
    let alice = join(&mut host, "Alice");
    let bob = join(&mut host, "Bob");
    let a = host.server.player_control(alice).unwrap().entity_id;
    let b = host.server.player_control(bob).unwrap().entity_id;
    let context = host.server.instances[0].lock().unwrap().context().unwrap();
    {
        let mut ctx = context.lock().unwrap();
        let actor = ctx.map.entities.iter_mut().find(|e| e.id == b).unwrap();
        actor
            .attributes
            .set("private", Value::Str("BOB_PRIVATE".into()));
        actor.inventory = vec![Some(item(1000, "PRIVATE_BAG"))];
        actor.wallet.balances.insert("gold".into(), 50);
    }
    host.poll();
    let first = baseline(&mut host, alice);
    let mut client = receiver(&host, &first);
    client.apply(&first).unwrap();
    let mut generation = first.generation;
    for phase in 0..4 {
        {
            let mut ctx = context.lock().unwrap();
            let actor = ctx.map.entities.iter_mut().find(|e| e.id == b).unwrap();
            actor.position.x += 1.;
            match phase {
                1 => {
                    actor.attributes.set("party_leader_id", Value::UInt(a));
                }
                2 => {
                    actor.attributes.remove("party_leader_id");
                }
                3 => {
                    actor.attributes.set("name", Value::Str("Changed".into()));
                }
                _ => {}
            }
        }
        host.poll();
        let frame = host.take_client_frames(alice).unwrap().pop().unwrap();
        assert_eq!(frame.generation, generation);
        generation = frame.generation;
        let WorldUpdate::Delta(regions) = &frame.update else {
            panic!()
        };
        if phase == 0 {
            assert!(regions[0].poses.iter().any(|p| p.id == b));
            assert!(!regions[0].entities.iter().any(|e| e.id == b));
        } else {
            assert!(regions[0].entities.iter().any(|e| e.id == b));
            assert!(!regions[0].poses.iter().any(|e| e.id == b));
        }
        let json = serde_json::to_string(&frame).unwrap();
        assert_eq!(json.contains("BOB_PRIVATE"), phase == 1);
        assert_eq!(json.contains("PRIVATE_BAG"), phase == 1);
        client.apply(&frame).unwrap();
        let actor = client
            .current_map()
            .unwrap()
            .entities
            .iter()
            .find(|e| e.id == b)
            .unwrap();
        assert_eq!(actor.inventory.is_empty(), phase != 1);
        assert_eq!(actor.wallet.balances.contains_key("gold"), phase == 1);
    }
    host.stop();
}

#[test]
fn pose_indices_survive_selective_removals_enters_and_replacement_baselines() {
    use eldiron_runtime::replication::{EntityPose, RegionDelta};
    let (mut client, mut frame, map) = typed_fixture();
    let WorldUpdate::Baseline(regions) = &mut frame.update else {
        panic!()
    };
    let original = regions[0].entities[0].clone();
    let mut other = original.clone();
    other.id = 3;
    regions[0].entities.push(other);
    let reference = regions[0].content;
    client.apply(&frame).unwrap();
    let mut delta = frame.clone();
    delta.sequence = 2;
    delta.update = WorldUpdate::Delta(vec![RegionDelta {
        content: reference,
        poses: vec![EntityPose {
            id: 3,
            position: [7., 0., 0.],
            orientation: [1., 0.],
            tilt: 0.,
            motion_revision: 0,
        }],
        entities: vec![],
        removed_entities: vec![1],
        items: vec![],
        removed_items: vec![],
        props: None,
        resource_views: None,
        time: Default::default(),
        render: Default::default(),
    }]);
    client.apply(&delta).unwrap();
    assert_eq!(client.map(map.id).unwrap().entities[0].id, 3);
    assert_eq!(client.map(map.id).unwrap().entities[0].position.x, 7.);
    delta.sequence = 3;
    let WorldUpdate::Delta(regions) = &mut delta.update else {
        panic!()
    };
    regions[0].removed_entities.clear();
    regions[0].entities.push(original.clone());
    regions[0].poses[0].position[0] = 8.;
    client.apply(&delta).unwrap();
    assert_eq!(client.map(map.id).unwrap().entities.len(), 2);
    frame.generation = 2;
    let WorldUpdate::Baseline(regions) = &mut frame.update else {
        panic!()
    };
    regions[0].entities = vec![original];
    client.apply(&frame).unwrap();
    delta.generation = 2;
    delta.sequence = 2;
    let WorldUpdate::Delta(regions) = &mut delta.update else {
        panic!()
    };
    regions[0].entities.clear();
    assert!(client.apply(&delta).is_err());
    assert_eq!(client.sequence(), 1);
    assert_eq!(client.map(map.id).unwrap().entities.len(), 1);
}
