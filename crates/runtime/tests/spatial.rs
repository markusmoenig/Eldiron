use eldiron_runtime::{
    ClientFrame, GameSession, Project, SessionOptions,
    replication::{SpatialPolicy, SpatialSubscriptions, WorldUpdate},
};
use rusterix::{
    BlockPropAsset, BlockPropInstance, Entity, GeometryObject, Item, Value,
    server::message::{AudioCommand, RegionMessage},
};
use theframework_core::{Uuid, prelude::Vec3};

#[test]
fn spatial_enter_leave_margins_pins_privacy_events_collision_and_region_transfer() {
    let mut project = Project::new();
    project.regions[0].name = "Spatial".into();
    project.regions[0].map.name = "Spatial".into();
    let map = &mut project.regions[0].map;
    let vertices = [[0., 0.], [2., 0.], [2., 2.], [0., 2.]].map(|p| map.add_vertex_at(p[0], p[1]));
    for i in 0..4 {
        map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
    }
    let sector = map.close_polygon_manual().unwrap();
    map.find_sector_mut(sector).unwrap().name = "entrance".into();
    let mut other = project.regions[0].clone();
    other.id = Uuid::new_v4();
    other.map.id = Uuid::new_v4();
    other.name = "Elsewhere".into();
    other.map.name = other.name.clone();
    project.regions.push(other);
    project.config = "[game]\nstart_region='Spatial'\nauto_create_player=false".into();
    let mut game = GameSession::new(project);
    game.start(SessionOptions {
        create_local_player: false,
        ..Default::default()
    })
    .unwrap();
    let region = game.project.regions[0].map.id;
    let destination = game.project.regions[1].map.id;
    let conn = game.open_player_connection().unwrap();
    let ctx = game.server.instances[0].lock().unwrap().context().unwrap();
    let (actor, near, far, foreign, wide, large_prop);
    {
        let mut ctx = ctx.lock().unwrap();
        let mut create = |position: f32, player: bool| {
            let mut e = Entity::default();
            e.id = ctx.world.next_id();
            e.position = Vec3::new(position, 0., 0.5);
            e.set_attribute("player", Value::Bool(player));
            e.set_attribute("class_name", Value::Str("Actor".into()));
            let id = e.id;
            ctx.entity_classes.insert(id, "Actor".into());
            ctx.map.entities.push(e);
            id
        };
        actor = create(0.5, true);
        near = create(6., false);
        far = create(100., false);
        foreign = create(200., true);
        ctx.map
            .entities
            .iter_mut()
            .find(|e| e.id == foreign)
            .unwrap()
            .set_attribute("private_note", Value::Str("OTHER_CHARACTER_SECRET".into()));
        let mut item = Item::default();
        wide = ctx.world.next_id();
        item.id = wide;
        item.position = Vec3::new(100., 0., 0.5);
        item.attributes.set("source_size", Value::Float(202.));
        ctx.map.items.push(item);
        let asset = BlockPropAsset::new_authored(
            "Large object",
            vec![GeometryObject::box_from_bounds(
                "",
                Vec3::new(-101., 0., 0.),
                Vec3::new(101., 1., 1.),
            )],
        );
        let mut prop = BlockPropInstance::new(asset.id);
        prop.world_transform[3][0] = 100.;
        large_prop = prop.id;
        ctx.assets.block_props.insert(asset.id, asset);
        ctx.map.block_prop_instances.push(prop);
        let props = ctx.map.block_prop_instances.clone();
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::BlockPropInstancesUpdate(0, props))
            .unwrap();
    }
    game.server
        .attach_saved_character(conn, region, actor)
        .unwrap();
    game.poll();
    let mut config = SpatialSubscriptions {
        policy: SpatialPolicy {
            radius: 8.,
            margin: 4.,
        },
        ..Default::default()
    };
    config.prefetch.insert(destination);
    game.set_spatial_subscriptions(conn, Some(config.clone()))
        .unwrap();
    game.subscribe_replication(conn).unwrap();
    let baseline = game.take_client_frames(conn).unwrap().pop().unwrap();
    let WorldUpdate::Baseline(regions) = &baseline.update else {
        panic!()
    };
    assert_eq!(regions.len(), 1);
    assert_eq!(
        regions[0]
            .entities
            .iter()
            .map(|e| e.id)
            .collect::<std::collections::HashSet<_>>(),
        std::collections::HashSet::from([actor, near])
    );
    assert_eq!(regions[0].items.iter().filter(|i| i.id == wide).count(), 1);
    assert_eq!(
        regions[0]
            .props
            .iter()
            .filter(|p| p.id == large_prop)
            .count(),
        1,
        "large props use their extent, not just their origin"
    );
    assert_eq!(game.prefetched_content(conn)[0].map, destination);
    assert!(
        game.client_world(conn).unwrap().map(destination).is_none(),
        "a content preload grants no live subscription"
    );
    let advance = |game: &mut GameSession| -> ClientFrame {
        assert!(game.poll().replication_error.is_none());
        game.take_client_frames(conn).unwrap().pop().unwrap()
    };
    // Previously visible objects linger through the exit margin, then leave.
    ctx.lock()
        .unwrap()
        .map
        .entities
        .iter_mut()
        .find(|e| e.id == near)
        .unwrap()
        .position
        .x = 11.;
    let frame = advance(&mut game);
    assert_eq!(frame.generation, baseline.generation);
    assert!(
        game.client_world(conn)
            .unwrap()
            .map(region)
            .unwrap()
            .entities
            .iter()
            .any(|e| e.id == near)
    );
    ctx.lock()
        .unwrap()
        .map
        .entities
        .iter_mut()
        .find(|e| e.id == near)
        .unwrap()
        .position
        .x = 14.;
    let frame = advance(&mut game);
    assert!(matches!(&frame.update,WorldUpdate::Delta(rs) if rs[0].removed_entities==vec![near]));
    assert!(
        ctx.lock()
            .unwrap()
            .map
            .entities
            .iter()
            .any(|e| e.id == near)
    );
    assert!(
        game.server.capture_durable_world().unwrap().regions[0]
            .entities
            .iter()
            .any(|e| e.id == near),
        "leaving interest cannot delete saved state"
    );
    // Distant state and speech are omitted; private addressed output survives.
    {
        let mut ctx = ctx.lock().unwrap();
        ctx.map
            .entities
            .iter_mut()
            .find(|e| e.id == far)
            .unwrap()
            .set_attribute("name", Value::Str("DISTANT_CHANGE".into()));
        let sender = ctx.from_sender.get().unwrap();
        sender
            .send(RegionMessage::Say(
                0,
                Some(far),
                None,
                "DISTANT_SPEECH".into(),
                "say".into(),
            ))
            .unwrap();
        sender
            .send(RegionMessage::Message(
                0,
                Some(far),
                None,
                actor,
                "PRIVATE_REPLY".into(),
                "system".into(),
            ))
            .unwrap();
    }
    let frame = advance(&mut game);
    let json = serde_json::to_string(&frame).unwrap();
    assert!(!json.contains("DISTANT_CHANGE"));
    assert!(!json.contains("DISTANT_SPEECH"));
    assert!(json.contains("PRIVATE_REPLY"));
    // Enter is a full object upsert, with neither duplicate IDs nor a movement
    // generation reset when crossing multiple network cells.
    ctx.lock()
        .unwrap()
        .map
        .entities
        .iter_mut()
        .find(|e| e.id == actor)
        .unwrap()
        .position
        .x = 98.;
    let frame = advance(&mut game);
    assert_eq!(frame.generation, baseline.generation);
    let map = game.client_world(conn).unwrap().map(region).unwrap();
    assert_eq!(map.entities.iter().filter(|e| e.id == far).count(), 1);
    assert!(!map.entities.iter().any(|e| e.id == near));
    // Trusted pins can observe distant public bodies, never their inventory or
    // arbitrary private attributes. Whole-region observers obey that policy too.
    config.entities.insert(foreign);
    game.set_spatial_subscriptions(conn, Some(config.clone()))
        .unwrap();
    let frame = game.take_client_frames(conn).unwrap().pop().unwrap();
    assert!(
        !serde_json::to_string(&frame)
            .unwrap()
            .contains("OTHER_CHARACTER_SECRET")
    );
    assert!(
        game.client_world(conn)
            .unwrap()
            .map(region)
            .unwrap()
            .entities
            .iter()
            .any(|e| e.id == foreign)
    );
    config.regions.insert(destination);
    game.set_spatial_subscriptions(conn, Some(config.clone()))
        .unwrap();
    assert!(game.client_world(conn).unwrap().map(destination).is_some());
    config.regions.clear();
    config.entities.clear();
    game.set_spatial_subscriptions(conn, Some(config)).unwrap();
    game.take_client_frames(conn).unwrap();
    // Collision changes outside spatial interest still replace the immutable
    // region package/reference before prediction can use it.
    let before = game
        .client_world(conn)
        .unwrap()
        .content_reference(region)
        .unwrap();
    ctx.lock()
        .unwrap()
        .collision_world
        .update_chunk([1000, 1000].into(), Default::default());
    let frame = advance(&mut game);
    let after = game
        .client_world(conn)
        .unwrap()
        .content_reference(region)
        .unwrap();
    assert_eq!(
        before, after,
        "physics changes use a separate package stamp, without invalidating dialogue/input context"
    );
    assert!(matches!(frame.update, WorldUpdate::Delta(_)));
    // The transfer drops old live maps. Region-wide audio from the region left
    // behind cannot bleed into the new subscriber.
    game.server
        .player_teleport(conn, "entrance".into(), "Elsewhere".into())
        .unwrap();
    for _ in 0..4 {
        game.poll();
    }
    assert_eq!(
        game.client_world(conn).unwrap().current_region(),
        Some(destination)
    );
    assert!(game.client_world(conn).unwrap().map(region).is_none());
    ctx.lock()
        .unwrap()
        .from_sender
        .get()
        .unwrap()
        .send(RegionMessage::AudioCmd(0, AudioCommand::ClearAll))
        .unwrap();
    let frame = advance(&mut game);
    assert!(frame.events.iter().all(|d| d.events.audio.is_empty()));
    game.set_spatial_subscriptions(conn, None).unwrap();
    assert_eq!(
        game.client_world(conn).unwrap().maps().count(),
        2,
        "embedded/full-world observers remain supported"
    );
}
