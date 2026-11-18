use eldiron_runtime::{
    ClientFrame, MotionClient, MotionSettings, MovementCommand, MovementIntent, Project,
    RegionEvents,
    prediction::MAX_PREDICTION_FRAMES,
    replication::{
        ApplyResult, EntitySnapshot, REPLICATION_VERSION, RegionContentRef, RegionDelivery,
        RegionDelta, RegionSnapshot, WorldUpdate,
    },
};
use rusterix::{
    Assets, Entity, Map, PlayerCamera, ServerState, Value,
    collision_world::CollisionWorld,
    map::mini::MapMini,
    movement::{
        CollisionMode, ControllerState, MotionParameters, MotionSnapshot, MovementWorld,
        motion_revision, step_controller,
    },
};
use std::{collections::VecDeque, time::Duration};
use theframework_core::{
    Uuid,
    prelude::{Vec2, Vec3},
};

struct Model {
    world: Uuid,
    connection: Uuid,
    map: Map,
    reference: RegionContentRef,
    mini: MapMini,
    collision: CollisionWorld,
    assets: Assets,
    actor: Entity,
    remote: Entity,
    controller: ControllerState,
    parameters: MotionParameters,
    generation: u64,
    packet: u64,
    tick: u64,
    received: u64,
    processed: u64,
    queue: VecDeque<MovementCommand>,
}
fn snapshot(actor: &Entity, private: bool) -> EntitySnapshot {
    EntitySnapshot {
        id: actor.id,
        creator_id: actor.creator_id,
        position: [actor.position.x, actor.position.y, actor.position.z],
        orientation: [actor.orientation.x, actor.orientation.y],
        tilt: actor.tilt,
        motion_revision: motion_revision(actor),
        attributes: actor
            .attributes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        equipped: Default::default(),
        inventory: private.then(Vec::new),
        wallet: private.then(Default::default),
    }
}
impl Model {
    fn new(camera: PlayerCamera) -> Self {
        let map = Map::default();
        let reference = RegionContentRef {
            map: map.id,
            map_revision: map.changed,
            runtime_revision: 0,
        };
        let mut actor = Entity::default();
        actor.id = 1;
        actor.position = Vec3::new(0.5, 0., 0.5);
        actor.set_attribute("player", Value::Bool(true));
        actor.set_attribute("name", Value::Str("Owned".into()));
        actor.set_attribute("player_camera", Value::PlayerCamera(camera));
        let mut remote = Entity::default();
        remote.id = 2;
        remote.position = Vec3::new(10., 0., 0.5);
        remote.set_attribute("name", Value::Str("Remote".into()));
        Self {
            world: Uuid::new_v4(),
            connection: Uuid::new_v4(),
            map,
            reference,
            mini: Default::default(),
            collision: CollisionWorld::new(10),
            assets: Default::default(),
            actor,
            remote,
            controller: Default::default(),
            parameters: MotionParameters {
                camera,
                units_per_second: 4.,
                turn_degrees_per_second: 120.,
                entity_block_mode: 1,
                collision_mode: CollisionMode::Tile,
                turn_period: 0.,
            },
            generation: 1,
            packet: 0,
            tick: 0,
            received: 0,
            processed: 0,
            queue: Default::default(),
        }
    }
    fn frame(&mut self, baseline: bool, events: RegionEvents) -> ClientFrame {
        self.packet += 1;
        let entities = vec![snapshot(&self.actor, true), snapshot(&self.remote, false)];
        let update = if baseline {
            WorldUpdate::Baseline(vec![RegionSnapshot {
                content: self.reference,
                entities,
                items: vec![],
                props: self.map.block_prop_instances.clone(),
                resource_views: vec![],
                time: Default::default(),
                render: Default::default(),
            }])
        } else {
            WorldUpdate::Delta(vec![RegionDelta {
                poses: vec![],
                content: self.reference,
                entities,
                removed_entities: vec![],
                items: vec![],
                removed_items: vec![],
                props: Some(self.map.block_prop_instances.clone()),
                resource_views: None,
                time: Default::default(),
                render: Default::default(),
            }])
        };
        ClientFrame {
            version: REPLICATION_VERSION,
            world: self.world,
            connection: self.connection,
            generation: self.generation,
            sequence: self.packet,
            tick: self.tick / 12,
            movement_tick: self.tick,
            controlled_entity: Some(1),
            current_region: Some(self.map.id),
            state: ServerState::Running,
            motion: Some(MotionSnapshot {
                generation: self.generation,
                processed_sequence: self.processed,
                revision: motion_revision(&self.actor),
                controller: self.controller.clone(),
                parameters: self.parameters,
                predictable: true,
            }),
            update,
            events: if events.count() == 0 {
                vec![]
            } else {
                vec![RegionDelivery {
                    region: self.map.id,
                    events,
                }]
            },
        }
    }
    fn client(&mut self, settings: MotionSettings) -> MotionClient {
        let mut client = MotionClient::new(self.world, self.connection, settings);
        client.install_content(self.reference, &self.map).unwrap();
        client.install_collision(self.reference, self.mini.clone(), self.collision.clone());
        client
            .receive(&self.frame(true, Default::default()), &self.assets)
            .unwrap();
        client
    }
    fn receive(&mut self, command: MovementCommand) {
        if command.generation == self.generation && command.sequence == self.received + 1 {
            self.received = command.sequence;
            self.queue.push_back(command);
        }
    }
    fn step(&mut self) {
        self.tick += 1;
        if let Some(command) = self.queue.pop_front() {
            step_controller(
                &MovementWorld {
                    diagnostics: None,
                    map: &self.map,
                    mapmini: &self.mini,
                    collision_world: &self.collision,
                    assets: &self.assets,
                    mode: self.parameters.collision_mode,
                },
                &mut self.actor,
                &mut self.controller,
                self.parameters,
                command,
            );
            self.processed = command.sequence;
        }
        self.remote.position.z += 0.01;
    }
    fn rebase(&mut self) -> ClientFrame {
        self.generation += 1;
        self.packet = 0;
        self.received = 0;
        self.processed = 0;
        self.queue.clear();
        self.controller = Default::default();
        self.frame(true, Default::default())
    }
}
fn x(client: &MotionClient, project: &Project, assets: &Assets) -> f32 {
    client
        .view(project, assets, None)
        .unwrap()
        .player_entity()
        .unwrap()
        .position
        .x
}

#[test]
fn movement_and_mouse_look_are_visible_before_the_first_server_roundtrip() {
    let project = Project::new();
    let mut server = Model::new(PlayerCamera::D3FirstPMouse);
    let mut client = server.client(MotionSettings::default());
    client.set_intent(MovementIntent::Forward);
    client.look(90.);
    client.advance(Duration::from_millis(5), &server.assets);
    let view = client.view(&project, &server.assets, None).unwrap();
    let actor = view.player_entity().unwrap();
    assert!((actor.position.z - 0.52).abs() < 1e-5);
    assert!(actor.orientation.x.abs() < 1e-5);
    assert_eq!(
        client.authority().current_map().unwrap().entities[0]
            .position
            .z,
        0.5
    );
    assert!(
        client.pending_inputs().is_empty(),
        "partial rendering is not an extra simulation frame"
    );
    client.advance(Duration::from_millis(15), &server.assets);
    let input = client.pending_inputs();
    assert_eq!(input.len(), 1);
    assert_eq!(input[0].yaw, 90.);
    assert_eq!(input[0].sequence, 1);
}

#[test]
fn latency_retry_loss_and_collision_corrections_converge_without_replaying_output() {
    for one_way in [5, 10] {
        // 10 ms harness ticks: 100 and 200 ms RTT.
        let project = Project::new();
        let mut server = Model::new(PlayerCamera::D3FirstPMouse);
        let mut client = server.client(MotionSettings::default());
        server.mini.blocked_tiles.insert(Vec2::new(3, 0));
        let mut outbound = VecDeque::new();
        let mut inbound = VecDeque::new();
        let mut dropped = false;
        let mut delivered_events = 0;
        for tick in 0..300 {
            if tick == 0 {
                client.set_intent(MovementIntent::Forward);
            }
            if tick == 200 {
                client.set_intent(MovementIntent::Stop);
            }
            if tick == 100 {
                // A closed door/collision update arrives after initial misprediction.
                client.install_collision(
                    server.reference,
                    server.mini.clone(),
                    server.collision.clone(),
                );
            }
            client.advance(Duration::from_millis(10), &server.assets);
            for command in client.pending_inputs() {
                if command.sequence == 5 && !dropped {
                    dropped = true;
                    continue;
                }
                outbound.push_back((tick + one_way, command));
            }
            while outbound.front().is_some_and(|(when, _)| *when <= tick) {
                server.receive(outbound.pop_front().unwrap().1);
            }
            if tick % 2 == 0 {
                server.step();
                let events = if tick == 50 {
                    RegionEvents {
                        messages: vec![(None, None, 1, "One delivery".into(), "status".into())],
                        ..Default::default()
                    }
                } else {
                    Default::default()
                };
                let frame = server.frame(false, events);
                inbound.push_back((
                    tick + one_way,
                    serde_json::from_str::<ClientFrame>(&serde_json::to_string(&frame).unwrap())
                        .unwrap(),
                ));
            }
            while inbound.front().is_some_and(|(when, _)| *when <= tick) {
                let frame = inbound.pop_front().unwrap().1;
                assert_eq!(
                    client.receive(&frame, &server.assets),
                    Ok(ApplyResult::Applied)
                );
                assert_eq!(
                    client.receive(&frame, &server.assets),
                    Ok(ApplyResult::IgnoredStale)
                );
            }
            delivered_events += client.take_events(server.map.id).messages.len();
            assert!(!client.needs_baseline());
            assert!(
                server.actor.position.x < 3.,
                "authority never crosses the closed door"
            );
        }
        assert_eq!(delivered_events, 1);
        assert!((x(&client, &project, &server.assets) - server.actor.position.x).abs() < 0.002);
        assert!(client.diagnostics().replayed_steps > 0);
        assert!(client.diagnostics().corrections > 0);
        let metrics = client.diagnostics();
        assert!(metrics.max_correction_distance.is_finite());
        assert!(metrics.max_correction_distance > 0.);
        assert_eq!(metrics.pending_frames, client.pending_inputs().len());
        assert!(client.pending_inputs().len() < 16);
    }
}

#[test]
fn remote_interpolation_uses_server_time_shortest_angles_and_snaps_teleports() {
    let project = Project::new();
    let mut server = Model::new(PlayerCamera::D3FirstPMouse);
    server.remote.position.x = 5.;
    server.remote.orientation =
        Vec2::new((-179f32).to_radians().cos(), (-179f32).to_radians().sin());
    let mut client = server.client(MotionSettings::default());
    server.tick = 5;
    server.remote.position.x = 6.;
    server.remote.orientation = Vec2::new(179f32.to_radians().cos(), 179f32.to_radians().sin());
    client
        .receive(&server.frame(false, Default::default()), &server.assets)
        .unwrap();
    client.advance(Duration::from_millis(50), &server.assets);
    let view = client.view(&project, &server.assets, None).unwrap();
    let remote = &view.map.unwrap().entities[1];
    assert!((remote.position.x - 5.5).abs() < 1e-4);
    assert!(
        remote.orientation.x < -0.99,
        "interpolate through 180 degrees, not zero"
    );
    assert_eq!(
        client.authority().current_map().unwrap().entities[1]
            .position
            .x,
        6.
    );
    server.tick += 1;
    server.remote.position.x = 40.;
    rusterix::movement::mark_discontinuity(&mut server.remote);
    client
        .receive(&server.frame(false, Default::default()), &server.assets)
        .unwrap();
    assert_eq!(
        client
            .view(&project, &server.assets, None)
            .unwrap()
            .map
            .unwrap()
            .entities[1]
            .position
            .x,
        40.
    );
    let mut removal = server.frame(false, Default::default());
    if let WorldUpdate::Delta(regions) = &mut removal.update {
        regions[0].entities.retain(|e| e.id != 2);
        regions[0].removed_entities.push(2);
    }
    client.receive(&removal, &server.assets).unwrap();
    assert!(
        !client
            .view(&project, &server.assets, None)
            .unwrap()
            .map
            .unwrap()
            .entities
            .iter()
            .any(|e| e.id == 2)
    );
}

#[test]
fn corrections_smooth_only_presentation_and_gaps_overflow_and_suspend_require_rebase() {
    let project = Project::new();
    let mut server = Model::new(PlayerCamera::D3FirstPMouse);
    let mut client = server.client(MotionSettings::embedded());
    let scope = client
        .view(&project, &server.assets, None)
        .unwrap()
        .session_id;
    client.set_intent(MovementIntent::Forward);
    client.advance(Duration::from_millis(20), &server.assets);
    server.receive(client.pending_inputs()[0]);
    server.step();
    server.actor.position.x -= 0.03;
    client.set_intent(MovementIntent::Stop);
    client
        .receive(&server.frame(false, Default::default()), &server.assets)
        .unwrap();
    assert!((x(&client, &project, &server.assets) - 0.58).abs() < 1e-5);
    assert!(
        (client.authority().current_map().unwrap().entities[0]
            .position
            .x
            - 0.55)
            .abs()
            < 1e-5
    );
    client.advance(Duration::from_millis(400), &server.assets);
    assert!((x(&client, &project, &server.assets) - 0.55).abs() < 0.001);
    assert_eq!(
        scope,
        client
            .view(&project, &server.assets, None)
            .unwrap()
            .session_id
    );
    let mut gap = server.frame(false, Default::default());
    gap.sequence += 1;
    assert!(client.receive(&gap, &server.assets).is_err());
    assert!(client.needs_baseline());
    assert_ne!(
        scope,
        client
            .view(&project, &server.assets, None)
            .unwrap()
            .session_id
    );
    client.receive(&server.rebase(), &server.assets).unwrap();
    assert!(!client.needs_baseline());
    client.advance(
        rusterix::simulation::MOVEMENT_STEP * (MAX_PREDICTION_FRAMES as u32 + 1),
        &server.assets,
    );
    assert!(client.needs_baseline());
    assert!(client.pending_inputs().is_empty());
    client.receive(&server.rebase(), &server.assets).unwrap();
    client.suspend();
    server.step();
    assert_eq!(server.tick, 2);
    assert!(client.needs_baseline());
    client.advance(Duration::from_secs(30), &server.assets);
    assert!(client.pending_inputs().is_empty());
    client.resume();
    client.receive(&server.rebase(), &server.assets).unwrap();
    assert!(client.ready());
    let snaps = client.diagnostics().pose_snaps;
    server.actor.position.x += 10.;
    server.step();
    client
        .receive(&server.frame(false, Default::default()), &server.assets)
        .unwrap();
    assert_eq!(client.diagnostics().pose_snaps, snaps + 1);
    assert!(client.diagnostics().max_correction_distance >= 9.9);
}

#[test]
fn grid_taps_are_predicted_once_and_replay_does_not_repeat_turn_callbacks() {
    let project = Project::new();
    let mut server = Model::new(PlayerCamera::D3FirstPGrid);
    server.actor.position = Vec3::new(2.5, 0., 2.5);
    server.actor.orientation = Vec2::new(0., -1.);
    let mut client = server.client(MotionSettings::embedded());
    client.set_intent(MovementIntent::Forward);
    client.set_intent(MovementIntent::Stop);
    client.advance(Duration::from_secs(1), &server.assets);
    let mut turns = 0;
    for command in client.pending_inputs() {
        let before = server.controller.grid.is_none();
        server.receive(command);
        server.step();
        if before && command.intent == MovementIntent::Forward {
            turns += 1;
        }
    }
    assert_eq!(turns, 1);
    assert!(
        (client
            .view(&project, &server.assets, None)
            .unwrap()
            .player_entity()
            .unwrap()
            .position
            .z
            - 1.5)
            .abs()
            < 1e-4
    );
    assert!((server.actor.position.z - 1.5).abs() < 1e-4);
    client
        .receive(&server.frame(false, Default::default()), &server.assets)
        .unwrap();
    assert!(client.pending_inputs().is_empty());
}

#[test]
fn prediction_is_render_rate_independent_in_every_camera_mode() {
    for camera in [
        PlayerCamera::D2,
        PlayerCamera::D2Iso,
        PlayerCamera::D3Iso,
        PlayerCamera::D2Grid,
        PlayerCamera::D2IsoGrid,
        PlayerCamera::D3FirstP,
        PlayerCamera::D3FirstPMouse,
        PlayerCamera::D3FirstPGrid,
    ] {
        let mut result: Option<Vec3<f32>> = None;
        for fps in [10, 30, 60, 144] {
            let mut server = Model::new(camera);
            let mut client = server.client(MotionSettings::embedded());
            client.set_intent(MovementIntent::Forward);
            let mut previous = Duration::ZERO;
            for frame in 1..=fps {
                let now = Duration::from_nanos(1_000_000_000 * frame / fps);
                client.advance(now - previous, &server.assets);
                previous = now;
            }
            let inputs = client.pending_inputs();
            assert_eq!(inputs.len(), 50, "{camera:?} at {fps} FPS");
            for command in inputs {
                server.receive(command);
                server.step();
            }
            let before = client
                .view(&Project::new(), &server.assets, None)
                .unwrap()
                .player_entity()
                .unwrap()
                .position;
            assert!((before - server.actor.position).magnitude() < 1e-4);
            if let Some(expected) = result {
                assert!((before - expected).magnitude() < 1e-4);
            }
            result = Some(before);
        }
    }
}

#[test]
fn authoritative_door_state_rebuilds_prediction_collision_without_a_gpu() {
    use rusterix::chunkbuilder::{ChunkBuilder, d3chunkbuilder::D3ChunkBuilder};
    use rusterix::{BlockPropAsset, BlockPropComponent, BlockPropInstance, GeometryObject};
    fn rebuild(server: &mut Model) {
        let mut builder = D3ChunkBuilder::new();
        for x in -1..=1 {
            for z in -1..=1 {
                let key = Vec2::new(x, z);
                server.collision.update_chunk(
                    key,
                    builder.build_collision(&server.map, &server.assets, key, 10),
                );
            }
        }
    }
    let project = Project::new();
    let mut server = Model::new(PlayerCamera::D3FirstPMouse);
    server.parameters.collision_mode = CollisionMode::Mesh;
    server.map.geometry_objects.push(GeometryObject::box_(
        "Floor",
        Vec3::new(4., -0.25, 0.5),
        Vec3::new(12., 0.5, 12.),
    ));
    let leaf = GeometryObject::box_("Leaf", Vec3::new(2.5, 1., 0.5), Vec3::new(0.2, 2., 3.));
    let mut asset = BlockPropAsset::new_authored("Door", vec![leaf]);
    let mut door = BlockPropComponent::new("Door");
    door.properties.set("part_id", Value::Id(asset.parts[0].id));
    door.properties.set("motion", Value::Str("Slide".into()));
    door.properties.set("slide_axis", Value::Vec3([0., 0., 1.]));
    door.properties.set("slide_distance", Value::Float(4.));
    let component = door.id;
    asset.components.push(door);
    server
        .map
        .block_prop_instances
        .push(BlockPropInstance::new(asset.id));
    server.assets.block_props.insert(asset.id, asset);
    rebuild(&mut server);
    let mut client = server.client(MotionSettings::embedded());
    client.set_intent(MovementIntent::Forward);
    for _ in 0..50 {
        client.advance(Duration::from_millis(20), &server.assets);
        for command in client.pending_inputs() {
            server.receive(command);
        }
        server.step();
        client
            .receive(&server.frame(false, Default::default()), &server.assets)
            .unwrap();
    }
    assert!(
        server.actor.position.x < 2.1,
        "closed door blocks authority"
    );
    assert!((x(&client, &project, &server.assets) - server.actor.position.x).abs() < 0.001);
    rusterix::set_block_prop_door_open(&mut server.map.block_prop_instances[0], component, true);
    rebuild(&mut server);
    client
        .receive(&server.frame(false, Default::default()), &server.assets)
        .unwrap();
    let replayed = client.diagnostics().corrections;
    for _ in 0..50 {
        client.advance(Duration::from_millis(20), &server.assets);
        for command in client.pending_inputs() {
            server.receive(command);
        }
        server.step();
        client
            .receive(&server.frame(false, Default::default()), &server.assets)
            .unwrap();
    }
    assert!(server.actor.position.x > 4., "opened door permits walking");
    assert!((x(&client, &project, &server.assets) - server.actor.position.x).abs() < 0.001);
    assert_eq!(
        client.diagnostics().corrections,
        replayed,
        "client rebuilt the changed collision, rather than relying on correction every frame"
    );
}

#[test]
fn region_and_content_rebases_revoke_history_and_wait_for_matching_collision() {
    let project = Project::new();
    let mut server = Model::new(PlayerCamera::D3FirstPMouse);
    let mut client = server.client(MotionSettings::default());
    client.set_intent(MovementIntent::Forward);
    client.advance(Duration::from_millis(100), &server.assets);
    let old = client.pending_inputs()[0];
    let stale_frame = server.frame(false, Default::default());
    let old_scope = client
        .view(&project, &server.assets, None)
        .unwrap()
        .session_id;
    server.map = Map::default();
    server.reference.map = server.map.id;
    server.reference.map_revision = server.map.changed;
    server.actor.position.x = 30.5;
    let transfer = server.rebase();
    assert_eq!(
        client.receive(&transfer, &server.assets),
        Err(eldiron_runtime::replication::ReplicationError::MissingContent(server.reference))
    );
    assert!(client.needs_baseline());
    client
        .install_content(server.reference, &server.map)
        .unwrap();
    client.receive(&transfer, &server.assets).unwrap();
    assert_eq!(x(&client, &project, &server.assets), 30.5);
    assert!(client.pending_inputs().is_empty());
    assert_ne!(
        client
            .view(&project, &server.assets, None)
            .unwrap()
            .session_id,
        old_scope
    );
    assert!(!client.ready());
    client.advance(Duration::from_millis(100), &server.assets);
    assert!(
        client.pending_inputs().is_empty(),
        "no speculation without destination collision"
    );
    client.install_collision(
        server.reference,
        server.mini.clone(),
        server.collision.clone(),
    );
    assert!(client.ready());
    assert_eq!(
        client.receive(&stale_frame, &server.assets),
        Ok(ApplyResult::IgnoredStale)
    );
    server.receive(old);
    assert!(
        server.queue.is_empty(),
        "old-region input cannot cross a generation"
    );
    client.set_intent(MovementIntent::Forward);
    client.advance(Duration::from_millis(20), &server.assets);
    assert_eq!(client.pending_inputs()[0].sequence, 1);
    assert_eq!(client.pending_inputs()[0].generation, transfer.generation);

    server.map.changed += 1;
    server.reference.map_revision = server.map.changed;
    server.reference.runtime_revision += 1;
    let content = server.rebase();
    client
        .install_content(server.reference, &server.map)
        .unwrap();
    client.receive(&content, &server.assets).unwrap();
    assert!(!client.ready(), "old collision revision cannot be reused");
    assert!(client.pending_inputs().is_empty());
    client.install_collision(
        server.reference,
        server.mini.clone(),
        server.collision.clone(),
    );
    assert!(client.ready());
}

#[test]
fn real_session_acks_simulated_frames_expires_backlogs_and_preserves_embedded_control() {
    use eldiron_runtime::{FrameInputError, GameSession, MovementSubmission, SessionOptions};
    let mut project = Project::new();
    project.regions[0].name = "Frames".into();
    project.regions[0].map.name = "Frames".into();
    project.config = "[game]\nstart_region='Frames'\nauto_create_player=false\ncollision_mode='tile'\ngame_tick_ms=250".into();
    let mut host = GameSession::new(project);
    host.server.print_log_messages = false;
    host.start(SessionOptions {
        create_local_player: false,
        ..Default::default()
    })
    .unwrap();
    let id = host.server.local_connection();
    let region = host.project.regions[0].map.id;
    let context = host.server.instances[0].lock().unwrap().context().unwrap();
    context
        .lock()
        .unwrap()
        .entity_player_classes
        .insert("Mover".into());
    let mut actor = Entity::default();
    actor.set_attribute("class_name", Value::Str("Mover".into()));
    actor.set_attribute("player", Value::Bool(true));
    actor.set_attribute(
        "player_camera",
        Value::PlayerCamera(PlayerCamera::D3FirstPMouse),
    );
    actor.position = Vec3::new(0.5, 0., 0.5);
    host.server
        .create_connected_player(id, &region, actor)
        .unwrap();
    for _ in 0..3 {
        host.tick_once();
    }
    context.lock().unwrap().mapmini = Default::default();
    host.clock.reset_elapsed();
    let mut client = host
        .create_motion_client(id, MotionSettings::embedded())
        .unwrap();
    let tick = host.movement_tick_sequence();
    let other = host.open_player_connection().unwrap();
    assert!(
        host.advance_motion_client(other, &mut client, Duration::from_secs(1))
            .is_err()
    );
    assert_eq!(
        host.movement_tick_sequence(),
        tick,
        "a mismatched client cannot advance the host or submit another player's input"
    );
    client.set_intent(MovementIntent::Forward);
    client.advance(Duration::from_millis(20), &host.assets);
    let command = client.pending_inputs()[0];
    let origin = host.view().player_entity().unwrap().position;
    assert_eq!(
        host.submit_input_frame(id, command),
        Ok(MovementSubmission::Accepted)
    );
    assert_eq!(
        host.submit_input_frame(id, command),
        Ok(MovementSubmission::IgnoredStale)
    );
    host.poll();
    assert_eq!(
        host.client_world(id)
            .unwrap()
            .motion()
            .unwrap()
            .processed_sequence,
        0,
        "receipt is not simulation"
    );
    assert_eq!(host.view().player_entity().unwrap().position, origin);
    let frames = host.take_client_frames(id).unwrap();
    for frame in &frames {
        client.receive(frame, &host.assets).unwrap();
    }
    host.advance(Duration::from_millis(20));
    assert_eq!(
        host.client_world(id)
            .unwrap()
            .motion()
            .unwrap()
            .processed_sequence,
        1
    );
    assert!((host.view().player_entity().unwrap().position.x - origin.x - 0.08).abs() < 1e-4);
    for frame in host.take_client_frames(id).unwrap() {
        client.receive(&frame, &host.assets).unwrap();
    }
    assert!(client.pending_inputs().is_empty());
    let stopped = host.view().player_entity().unwrap().position;
    host.advance(Duration::from_millis(400));
    assert_eq!(
        host.view().player_entity().unwrap().position,
        stopped,
        "missing input is not repeated movement"
    );
    host.advance(Duration::from_millis(200));
    assert!(
        host.client_world(id).unwrap().generation() > command.generation,
        "stalled input expires to a new baseline"
    );
    assert_eq!(
        host.submit_input_frame(id, command),
        Err(FrameInputError::NeedsBaseline)
    );

    let baseline = host.client_world(id).unwrap().generation();
    let mut command = MovementCommand {
        generation: baseline,
        sequence: 2,
        intent: MovementIntent::Forward,
        yaw: 0.,
    };
    assert_eq!(
        host.submit_input_frame(id, command),
        Err(FrameInputError::MissingSequence { expected: 1 })
    );
    command.sequence = 1;
    let mut malformed = command;
    malformed.yaw = f32::NAN;
    assert_eq!(
        host.submit_input_frame(id, malformed),
        Err(FrameInputError::InvalidInput)
    );
    for sequence in 1..=32 {
        command.sequence = sequence;
        host.submit_input_frame(id, command).unwrap();
    }
    command.sequence = 33;
    assert_eq!(
        host.submit_input_frame(id, command),
        Err(FrameInputError::Backpressure)
    );
    let before = host.view().player_entity().unwrap().position;
    host.advance(Duration::from_secs(1));
    assert!(
        host.view().player_entity().unwrap().position.x - before.x <= 2.001,
        "old queued frames expire, they cannot purchase time"
    );
    assert!(host.client_world(id).unwrap().generation() > baseline);

    host.server.pause();
    host.advance(Duration::from_secs(1));
    command.generation = host.client_world(id).unwrap().generation();
    command.sequence = 1;
    assert_eq!(
        host.submit_input_frame(id, command),
        Err(FrameInputError::Paused)
    );
    host.server.continue_instances();
    host.advance(Duration::ZERO);
    host.disable_frame_input(id).unwrap();
    let before = host.view().player_entity().unwrap().position;
    host.action(rusterix::EntityAction::Forward);
    host.advance(Duration::from_millis(20));
    assert!(
        host.view().player_entity().unwrap().position.x > before.x,
        "ordinary embedded controls still work"
    );

    let mut client = host
        .create_motion_client(id, MotionSettings::embedded())
        .unwrap();
    let before = host.view().player_entity().unwrap().position.x;
    client.set_intent(MovementIntent::Forward);
    host.advance_motion_client(id, &mut client, Duration::from_millis(5))
        .unwrap();
    assert!((x(&client, &host.project, &host.assets) - before - 0.02).abs() < 1e-4);
    assert_eq!(host.view().player_entity().unwrap().position.x, before);
    host.advance_motion_client(id, &mut client, Duration::from_millis(15))
        .unwrap();
    assert!(client.pending_inputs().is_empty());
    assert!((x(&client, &host.project, &host.assets) - before - 0.08).abs() < 1e-4);

    // A pending teleport is collected before sampling the next old-scope input.
    host.server
        .player_teleport_pos(id, Vec2::new(20.5, 0.5))
        .unwrap();
    host.advance_motion_client(id, &mut client, Duration::from_millis(20))
        .unwrap();
    assert!((x(&client, &host.project, &host.assets) - 20.5).abs() < 1e-4);
    assert_eq!(host.view().player_entity().unwrap().position.x, 20.5);
    let generation = client.authority().generation();
    host.server.pause();
    host.advance_motion_client(id, &mut client, Duration::from_secs(1))
        .unwrap();
    assert!(!client.ready());
    host.server.continue_instances();
    host.advance_motion_client(id, &mut client, Duration::ZERO)
        .unwrap();
    assert!(client.ready());
    assert!(client.authority().generation() > generation);
    client.suspend();
    let tick = host.movement_tick_sequence();
    host.advance_motion_client(id, &mut client, Duration::from_millis(100))
        .unwrap();
    assert_eq!(host.movement_tick_sequence(), tick + 5);
    assert_eq!(host.server.state, ServerState::Running);
    client.resume();
    host.advance_motion_client(id, &mut client, Duration::ZERO)
        .unwrap();
    assert!(client.ready());
}
