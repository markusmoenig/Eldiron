use eldiron_network::*;
use eldiron_runtime::{
    Project,
    persistence::{FileWorldStore, WorldStore},
};
use eldiron_server::{
    auth,
    host::{Command, Host, World},
};
use std::time::Duration;
use theframework_core::Uuid;
use tokio::sync::mpsc;
fn project() -> Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    project
}
async fn peer(host: &mut Host) -> (Uuid, mpsc::Receiver<Vec<u8>>) {
    let id = Uuid::new_v4();
    let (tx, rx) = mpsc::channel(64);
    host.command(Command::Open {
        peer: id,
        output: tx,
    })
    .await
    .unwrap();
    message(
        host,
        id,
        ClientMessage::Hello {
            version: PROTOCOL_VERSION,
        },
    )
    .await;
    (id, rx)
}
async fn message(host: &mut Host, peer: Uuid, message: ClientMessage) {
    host.command(Command::Message { peer, message })
        .await
        .unwrap();
}
fn drain(rx: &mut mpsc::Receiver<Vec<u8>>) -> Vec<ServerMessage> {
    let mut messages = vec![];
    while let Ok(bytes) = rx.try_recv() {
        messages.push(codec::decode_server(&bytes).unwrap());
    }
    messages
}
fn authorized(rx: &mut mpsc::Receiver<Vec<u8>>) -> (Uuid, String) {
    drain(rx)
        .into_iter()
        .find_map(|m| {
            if let ServerMessage::Authorized { account, token, .. } = m {
                Some((account, token.0))
            } else {
                None
            }
        })
        .unwrap()
}
#[tokio::test]
async fn accounts_ownership_creation_retry_restart_and_demo_isolation() {
    let dir = std::env::temp_dir().join(format!("eldiron-host-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let path = dir.join("world.save");
    let project = project();
    let mut host = Host::open(project.clone(), Some(path.clone()))
        .await
        .unwrap();
    let (alice, mut a) = peer(&mut host).await;
    let (bob, mut b) = peer(&mut host).await;
    let hash = auth::hash("password for testing").unwrap();
    host.command(Command::Register {
        peer: alice,
        username: "alice".into(),
        hash: hash.clone(),
    })
    .await
    .unwrap();
    let (owner, token) = authorized(&mut a);
    host.command(Command::Register {
        peer: bob,
        username: "bob".into(),
        hash,
    })
    .await
    .unwrap();
    let (other, _) = authorized(&mut b);
    assert_ne!(owner, other);
    let request = Uuid::new_v4();
    message(
        &mut host,
        alice,
        ClientMessage::CreateCharacter {
            request,
            name: "Alice".into(),
        },
    )
    .await;
    let character = drain(&mut a)
        .into_iter()
        .find_map(|m| {
            if let ServerMessage::CharacterCreated { character, .. } = m {
                Some(character)
            } else {
                None
            }
        })
        .unwrap();
    let World::Saved(session) = &host.world else {
        panic!()
    };
    let count = session
        .saved()
        .world_state
        .regions
        .iter()
        .flat_map(|r| &r.entities)
        .count();
    let body = session.saved().characters[0].entity;
    message(
        &mut host,
        alice,
        ClientMessage::CreateCharacter {
            request,
            name: "Alice".into(),
        },
    )
    .await;
    assert!(
        drain(&mut a).iter().any(
            |m| matches!(m,ServerMessage::CharacterCreated{character:id,..} if *id==character)
        )
    );
    message(&mut host, bob, ClientMessage::JoinCharacter { character }).await;
    assert!(drain(&mut b).iter().any(|m| matches!(
        m,
        ServerMessage::Rejected {
            code: RejectCode::CharacterUnavailable
        }
    )));
    message(
        &mut host,
        alice,
        ClientMessage::CreateCharacter {
            request,
            name: "Different".into(),
        },
    )
    .await;
    assert!(drain(&mut a).iter().any(|m| matches!(
        m,
        ServerMessage::Rejected {
            code: RejectCode::InvalidRequest
        }
    )));
    message(
        &mut host,
        bob,
        ClientMessage::Guest {
            name: "Guest".into(),
        },
    )
    .await;
    assert!(drain(&mut b).iter().any(|m| matches!(
        m,
        ServerMessage::Rejected {
            code: RejectCode::WrongMode
        }
    )));
    // Reconnect rotates the bearer and closes the previous socket/control.
    let (reconnect, mut r) = peer(&mut host).await;
    message(
        &mut host,
        reconnect,
        ClientMessage::Resume {
            token: Secret(token.clone()),
        },
    )
    .await;
    let (resumed, new_token) = authorized(&mut r);
    assert_eq!(resumed, owner);
    assert_ne!(token, new_token);
    message(
        &mut host,
        reconnect,
        ClientMessage::JoinCharacter { character },
    )
    .await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    let frames = drain(&mut r);
    let (frame, hints) = frames
        .into_iter()
        .find_map(|m| {
            if let ServerMessage::Frame { frame, content } = m {
                Some((frame, content))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(frame.controlled_entity, Some(body));
    assert!(!hints.is_empty());
    let (reply, rx) = tokio::sync::oneshot::channel();
    host.command(Command::Content {
        key: hints[0].key.clone(),
        reply,
    })
    .await
    .unwrap();
    let bytes = rx.await.unwrap().unwrap();
    let package = remote::verify_content(&hints[0], &bytes).unwrap();
    let mut remote = remote::RemoteClient::new(&frame);
    remote.install(&hints[0], package).unwrap();
    remote.receive(&frame).unwrap();
    // Old inputs already in flight must not start an endless baseline cycle.
    message(&mut host, reconnect, ClientMessage::Resync).await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    let generation = drain(&mut r)
        .into_iter()
        .find_map(|m| match m {
            ServerMessage::Frame { frame, .. } => Some(frame.generation),
            _ => None,
        })
        .unwrap();
    assert!(generation > frame.generation);
    message(
        &mut host,
        reconnect,
        ClientMessage::Inputs {
            frames: vec![eldiron_runtime::MovementCommand {
                generation: frame.generation,
                sequence: 1,
                intent: eldiron_runtime::MovementIntent::Stop,
                yaw: 0.,
            }],
        },
    )
    .await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    for message in drain(&mut r) {
        match message {
            ServerMessage::Frame { frame, .. } => assert_eq!(frame.generation, generation),
            ServerMessage::Rejected { .. } => {
                panic!("superseded inputs must not trigger another reset")
            }
            _ => {}
        }
    }

    // Mutations and the sequence receipt commit before the reply. A retry of
    // this exact pickup, even after a new host incarnation, cannot grant twice.
    let item_id;
    {
        let ctx = host.world.game().server.instances[0]
            .lock()
            .unwrap()
            .context()
            .unwrap();
        let mut ctx = ctx.lock().unwrap();
        let position = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == body)
            .unwrap()
            .position;
        let mut item = rusterix::Item::default();
        item_id = ctx.world.next_id();
        item.id = item_id;
        item.position = position;
        ctx.map.items.push(item);
        ctx.map
            .entities
            .iter_mut()
            .find(|e| e.id == body)
            .unwrap()
            .inventory
            .push(None);
    }
    let pickup = GameAction {
        stream: character,
        sequence: 1,
        world: frame.world,
        generation,
        region: frame.current_region.unwrap(),
        request: GameRequest::Transfer {
            source: ItemSource::Direct(ItemAddress {
                item: item_id,
                owner: ItemOwner::World,
            }),
            destination: ItemDestination::Inventory(None),
        },
    };
    for _ in 0..2 {
        message(&mut host, reconnect, ClientMessage::Action(pickup.clone())).await;
        assert!(drain(&mut r).iter().any(|m|matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Outcome(RequestOutcome::Processed) && reply.next_sequence==2)));
    }
    let mut altered = pickup.clone();
    altered.request = GameRequest::Intent("attack".into());
    message(&mut host, reconnect, ClientMessage::Action(altered)).await;
    assert!(drain(&mut r).iter().any(
        |m| matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Conflict)
    ));
    let mut stale = pickup.clone();
    stale.sequence = 2;
    stale.generation = frame.generation;
    message(&mut host, reconnect, ClientMessage::Action(stale)).await;
    assert!(drain(&mut r).iter().any(|m|matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Outcome(RequestOutcome::Denied(RequestError::StaleContext)) && reply.next_sequence==3)));
    host.enable_diagnostics(true);
    let backup = dir.join("checkpoint.snapshot");
    host.backup_to(&backup).await.unwrap();
    assert_eq!(host.diagnostics().backups, 1);
    let image = eldiron_runtime::persistence::read_snapshot(&backup).unwrap();
    assert_eq!(image.action_streams[&character].last, 2);
    let restored = dir.join("restored.save");
    eldiron_runtime::persistence::write_snapshot_new(&restored, &image).unwrap();
    assert!(eldiron_runtime::persistence::write_snapshot_new(&restored, &image).is_err());
    host.shutdown().await.unwrap();
    drop(host);
    let mut store = FileWorldStore::open(&path).unwrap();
    let saved = store.load().await.unwrap().unwrap();
    assert_eq!(saved.action_streams[&character].last, 2);
    assert_eq!(
        saved
            .world_state
            .regions
            .iter()
            .flat_map(|r| &r.entities)
            .find(|e| e.id == body)
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .filter(|i| i.id == item_id)
            .count(),
        1
    );
    assert_eq!(saved.characters.len(), 1);
    assert_eq!(saved.characters[0].account, Some(owner));
    assert_eq!(
        saved
            .world_state
            .regions
            .iter()
            .flat_map(|r| &r.entities)
            .count(),
        count
    );
    drop(store);
    let mut host = Host::open(
        eldiron_server::recovery::load_project(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../test_projects/3DStarter.eldiron"),
        )
        .unwrap(),
        Some(restored.clone()),
    )
    .await
    .unwrap();
    let (reopen, mut r) = peer(&mut host).await;
    host.command(Command::Verified {
        peer: reopen,
        account: owner,
        security_revision:0,
    })
    .await
    .unwrap();
    authorized(&mut r);
    message(
        &mut host,
        reopen,
        ClientMessage::CreateCharacter {
            request,
            name: "Alice".into(),
        },
    )
    .await;
    assert!(
        drain(&mut r).iter().any(
            |m| matches!(m,ServerMessage::CharacterCreated{character:id,..} if *id==character)
        )
    );
    message(
        &mut host,
        reopen,
        ClientMessage::JoinCharacter { character },
    )
    .await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    let reopened_messages = drain(&mut r);
    assert!(reopened_messages.iter().any(
        |m| matches!(m,ServerMessage::Frame{frame,..} if frame.controlled_entity==Some(body))
    ));
    assert!(reopened_messages.iter().any(
        |m| matches!(m,ServerMessage::ActionStream{stream,next_sequence:3} if *stream==character)
    ));
    let World::Saved(session) = &host.world else {
        panic!()
    };
    assert_eq!(session.saved().characters.len(), 1);
    assert_eq!(session.saved().characters[0].entity, body);
    assert_eq!(
        session
            .saved()
            .world_state
            .regions
            .iter()
            .flat_map(|r| &r.entities)
            .count(),
        count
    );
    assert_eq!(session.saved().host_data, image.host_data);
    let live = host.world.game().server.capture_durable_world().unwrap();
    let live_body = live
        .regions
        .iter()
        .flat_map(|r| &r.entities)
        .find(|e| e.id == body)
        .unwrap();
    let saved_body = image
        .world_state
        .regions
        .iter()
        .flat_map(|r| &r.entities)
        .find(|e| e.id == body)
        .unwrap();
    let item_ids = |entity: &eldiron_runtime::persistence::SavedEntity| {
        entity
            .inventory
            .iter()
            .map(|i| i.as_ref().map(|i| i.id))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        item_ids(live_body),
        item_ids(saved_body),
        "restoration must not grant starter items again"
    );
    message(&mut host, reopen, ClientMessage::Action(pickup)).await;
    assert!(drain(&mut r).iter().any(|m|matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Outcome(RequestOutcome::Processed) && reply.next_sequence==3)));
    let live = host.world.game().server.capture_durable_world().unwrap();
    assert_eq!(
        live.regions
            .iter()
            .flat_map(|r| &r.entities)
            .find(|e| e.id == body)
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .filter(|i| i.id == item_id)
            .count(),
        1
    );
    host.shutdown().await.unwrap();
    drop(host);
    let disk = std::fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&disk).contains("password for testing"));
    let before = std::fs::read(&path).unwrap();
    let mut demo = Host::open(project, None).await.unwrap();
    let (guest, mut g) = peer(&mut demo).await;
    message(
        &mut demo,
        guest,
        ClientMessage::Guest {
            name: "Visitor".into(),
        },
    )
    .await;
    demo.tick(Duration::from_millis(20)).await.unwrap();
    assert!(
        drain(&mut g)
            .iter()
            .any(|m| matches!(m, ServerMessage::Frame { .. }))
    );
    demo.command(Command::Close(guest)).await.unwrap();
    demo.tick(Duration::from_millis(20)).await.unwrap();
    assert!(!demo.world.game().server.has_local_player());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    demo.shutdown().await.unwrap();
    drop(demo);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn demo_dialogue_uses_private_opaque_offers_and_rejects_wrong_recipient_or_expired_context() {
    use rusterix::{
        Choice, Entity, MultipleChoice,
        server::{
            message::{NodeChoice, RegionMessage},
            regionctx::ChoiceSession,
        },
    };
    let mut host = Host::open(project(), None).await.unwrap();
    let (alice, mut a) = peer(&mut host).await;
    let (bob, mut b) = peer(&mut host).await;
    for (peer, name) in [(alice, "Alice"), (bob, "Bob")] {
        message(&mut host, peer, ClientMessage::Guest { name: name.into() }).await;
    }
    host.tick(Duration::from_millis(20)).await.unwrap();
    let initial_a = drain(&mut a);
    let initial_b = drain(&mut b);
    let stream = |messages: &[ServerMessage]| {
        messages
            .iter()
            .find_map(|m| match m {
                ServerMessage::ActionStream { stream, .. } => Some(*stream),
                _ => None,
            })
            .unwrap()
    };
    let baseline = |messages: &[ServerMessage]| {
        messages
            .iter()
            .find_map(|m| match m {
                ServerMessage::Frame { frame, .. } => Some(frame.clone()),
                _ => None,
            })
            .unwrap()
    };
    let sa = stream(&initial_a);
    let sb = stream(&initial_b);
    let fa = baseline(&initial_a);
    let fb = baseline(&initial_b);
    let actor = fa.controlled_entity.unwrap();
    let ctx = host.world.game().server.instances[0]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    let npc;
    {
        let mut ctx = ctx.lock().unwrap();
        npc = ctx.world.next_id();
        let mut speaker = Entity::default();
        speaker.id = npc;
        speaker.position = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == actor)
            .unwrap()
            .position;
        ctx.map.entities.push(speaker);
        let expires = ctx.ticks + 10;
        ctx.active_choice_sessions.push(ChoiceSession {
            from: npc,
            to: actor,
            expires_at_tick: expires,
            max_distance: 3.,
        });
        let mut group = MultipleChoice::new(0, npc, actor, expires, 3.);
        group.add(Choice::NodeChoice(NodeChoice {
            label: "Accept quest".into(),
            index: 1,
            from: npc,
            to: actor,
            expires_at_tick: expires,
            max_distance: 3.,
        }));
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::MultipleChoice(group))
            .unwrap();
    }
    host.tick(Duration::from_millis(20)).await.unwrap();
    let am = drain(&mut a);
    let bm = drain(&mut b);
    let offer = am
        .iter()
        .find_map(|m| match m {
            ServerMessage::Offers { offers, .. } => offers
                .iter()
                .find(|o| matches!(o.choice, Choice::NodeChoice(_)))
                .map(|o| o.id),
            _ => None,
        })
        .unwrap();
    assert!(
        !bm.iter()
            .any(|m| matches!(m,ServerMessage::Offers{offers,..} if !offers.is_empty()))
    );
    let action = |frame: &eldiron_runtime::ClientFrame, stream, sequence, offer| GameAction {
        stream,
        sequence,
        world: frame.world,
        generation: frame.generation,
        region: frame.current_region.unwrap(),
        request: GameRequest::Choose { offer },
    };
    message(
        &mut host,
        bob,
        ClientMessage::Action(action(&fb, sb, 1, offer)),
    )
    .await;
    assert!(drain(&mut b).iter().any(|m|matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Outcome(RequestOutcome::Denied(RequestError::ExpiredOffer)))));
    // A new group may reuse identical NPC/session timing within the same tick.
    // Another player's action can produce it before the next timer tick. Its
    // replacement must revoke the old opaque UUID immediately after commit.
    {
        let ctx = ctx.lock().unwrap();
        let session = ctx
            .active_choice_sessions
            .iter()
            .find(|s| s.from == npc && s.to == actor)
            .unwrap();
        let expires = session.expires_at_tick;
        let mut group = MultipleChoice::new(0, npc, actor, expires, 3.);
        group.add(Choice::NodeChoice(NodeChoice {
            label: "Updated quest".into(),
            index: 2,
            from: npc,
            to: actor,
            expires_at_tick: expires,
            max_distance: 3.,
        }));
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::MultipleChoice(group))
            .unwrap();
    }
    let mut refresh = action(&fb, sb, 2, offer);
    refresh.request = GameRequest::Intent("look".into());
    message(&mut host, bob, ClientMessage::Action(refresh)).await;
    drain(&mut b);
    let fresh = drain(&mut a)
        .iter()
        .find_map(|m| match m {
            ServerMessage::Offers { offers, .. } => offers
                .iter()
                .find(|o| matches!(o.choice,Choice::NodeChoice(ref choice) if choice.index==2))
                .map(|o| o.id),
            _ => None,
        })
        .expect("new offers must be published before the next timer tick");
    assert_ne!(offer, fresh);
    message(
        &mut host,
        alice,
        ClientMessage::Action(action(&fa, sa, 1, offer)),
    )
    .await;
    assert!(drain(&mut a).iter().any(|m|matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Outcome(RequestOutcome::Denied(RequestError::ExpiredOffer)))));
    let chosen = action(&fa, sa, 2, fresh);
    let mut revoked = false;
    for _ in 0..2 {
        message(&mut host, alice, ClientMessage::Action(chosen.clone())).await;
        let received = drain(&mut a);
        revoked |= received
            .iter()
            .any(|m| matches!(m,ServerMessage::Offers{offers,..} if offers.is_empty()));
        assert!(received.iter().any(|m|matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Outcome(RequestOutcome::Processed))));
    }
    assert!(matches!(
        ctx.lock().unwrap().dialog_choices.get(&npc),
        Some(rusterix::server::nodes::DialogChoiceMade::Index(2))
    ));
    message(
        &mut host,
        alice,
        ClientMessage::Action(action(&fa, sa, 3, fresh)),
    )
    .await;
    assert!(drain(&mut a).iter().any(|m|matches!(m,ServerMessage::ActionReply(reply) if reply.result==ActionResult::Outcome(RequestOutcome::Denied(RequestError::ExpiredOffer)))));
    assert!(
        revoked,
        "consumed offers are revoked before another request can run"
    );
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn admission_slow_peer_and_replica_overflow_preserve_healthy_peer_and_private_metrics() {
    let mut host = Host::open(project(), None).await.unwrap();
    host.enable_diagnostics(true);
    host.set_player_limit(1).unwrap();
    let (alice, mut a) = peer(&mut host).await;
    message(
        &mut host,
        alice,
        ClientMessage::Guest {
            name: "Private name canary".into(),
        },
    )
    .await;
    let (bob, mut b) = peer(&mut host).await;
    message(&mut host, bob, ClientMessage::Guest { name: "Bob".into() }).await;
    assert!(drain(&mut b).iter().any(|m| matches!(
        m,
        ServerMessage::Rejected {
            code: RejectCode::ServerBusy
        }
    )));
    host.set_player_limit(2).unwrap();
    message(&mut host, bob, ClientMessage::Guest { name: "Bob".into() }).await;
    host.tick(Duration::from_secs(1)).await.unwrap();
    let frames = drain(&mut a);
    drain(&mut b);
    assert!(
        frames
            .iter()
            .any(|m| matches!(m, ServerMessage::Frame { .. }))
    );
    assert_eq!(host.diagnostics().wall_time_clamped_us, 750_000);
    // Saturate only Alice's outbound queue. No socket draining from that peer.
    for nonce in 0..70 {
        message(&mut host, alice, ClientMessage::Ping { nonce }).await;
    }
    host.tick(Duration::from_millis(20)).await.unwrap();
    assert!(
        drain(&mut b)
            .iter()
            .any(|m| matches!(m, ServerMessage::Frame { .. }))
    );
    assert_eq!(host.diagnostics().players, 1);
    assert_eq!(host.diagnostics().slow_disconnects, 1);
    // A replication overflow also disconnects its subscriber, never the host.
    let (slow, mut s) = peer(&mut host).await;
    message(
        &mut host,
        slow,
        ClientMessage::Guest {
            name: "Slow".into(),
        },
    )
    .await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    drain(&mut s);
    let healthy = drain(&mut b)
        .into_iter()
        .find_map(|m| match m {
            ServerMessage::Frame { frame, .. } => Some(frame.connection),
            _ => None,
        })
        .unwrap();
    let healthy = host
        .world
        .game()
        .server
        .player_connections()
        .into_iter()
        .find(|(id, _)| id.uuid() == healthy)
        .unwrap()
        .0;
    for _ in 0..70 {
        host.world.game().poll();
        host.world.game().take_client_frames(healthy).unwrap();
    }
    host.tick(Duration::from_millis(20)).await.unwrap();
    assert!(
        drain(&mut b)
            .iter()
            .any(|m| matches!(m, ServerMessage::Frame { .. }))
    );
    let stats = host.diagnostics();
    assert_eq!(stats.players, 1);
    assert_eq!(stats.replica_disconnects, 1);
    assert!(stats.busy_rejections > 0);
    assert!(stats.host_tick.calls > 0);
    assert!(stats.encoding.calls > 0);
    assert_eq!(stats.runtime.as_ref().unwrap().queues.pending_events, 0);
    let json = serde_json::to_string(&stats).unwrap();
    assert!(!json.contains("Private name canary"));
    assert!(!json.contains("password"));
    assert!(!json.contains("token"));
    let dir = std::env::temp_dir().join(format!("eldiron-metrics-{}", Uuid::new_v4()));
    let path = dir.join("metrics.json");
    let mut operations = eldiron_server::operations::Operations::default();
    operations.metrics_file = Some(path.clone());
    host.configure_operations(operations).unwrap();
    host.write_metrics_now();
    let exported: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(exported["schema"], 1);
    assert_eq!(exported["players"], 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn sequential_join_burst_delivers_between_commands_without_retiring_existing_players() {
    let mut host = Host::open(project(), None).await.unwrap();
    host.enable_diagnostics(true);
    let mut clients = vec![];
    for n in 0..20 {
        let (id, rx) = peer(&mut host).await;
        clients.push((id, rx));
        message(
            &mut host,
            id,
            ClientMessage::Guest {
                name: format!("Burst {n}"),
            },
        )
        .await;
        // Real socket workers drain concurrently; model their progress between
        // commands while deliberately withholding the simulation timer tick.
        for (_, rx) in &mut clients {
            drain(rx);
            assert!(!rx.is_closed());
        }
    }
    host.tick(Duration::from_millis(20)).await.unwrap();
    for (_, rx) in &mut clients {
        assert!(
            drain(rx)
                .iter()
                .any(|m| matches!(m, ServerMessage::Frame { .. }))
        );
        assert!(!rx.is_closed());
    }
    let diagnostics = host.diagnostics();
    assert_eq!(diagnostics.players, 20);
    assert_eq!(diagnostics.replica_disconnects, 0);
    assert_eq!(diagnostics.slow_disconnects, 0);
    host.shutdown().await.unwrap();
}

#[tokio::test]
async fn grouped_saved_actions_publish_only_after_one_commit_and_hold_outputs_on_failure() {
    let dir = std::env::temp_dir().join(format!("eldiron-group-{}", Uuid::new_v4()));
    let path = dir.join("world.save");
    let mut host = Host::open(project(), Some(path.clone())).await.unwrap();
    let (id, mut rx) = peer(&mut host).await;
    host.command(Command::Register {
        peer: id,
        username: "grouped".into(),
        hash: auth::hash("password for test").unwrap(),
    })
    .await
    .unwrap();
    authorized(&mut rx);
    message(
        &mut host,
        id,
        ClientMessage::CreateCharacter {
            request: Uuid::new_v4(),
            name: "Grouped".into(),
        },
    )
    .await;
    let character = drain(&mut rx)
        .into_iter()
        .find_map(|m| {
            if let ServerMessage::CharacterCreated { character, .. } = m {
                Some(character)
            } else {
                None
            }
        })
        .unwrap();
    message(&mut host, id, ClientMessage::JoinCharacter { character }).await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    let frame = drain(&mut rx)
        .into_iter()
        .find_map(|m| {
            if let ServerMessage::Frame { frame, .. } = m {
                Some(frame)
            } else {
                None
            }
        })
        .unwrap();
    let action = |sequence| Command::Message {
        peer: id,
        message: ClientMessage::Action(GameAction {
            stream: character,
            sequence,
            world: frame.world,
            generation: frame.generation,
            region: frame.current_region.unwrap(),
            request: GameRequest::Intent("look".into()),
        }),
    };
    let revision = if let World::Saved(s) = &host.world {
        s.saved().revision
    } else {
        panic!()
    };
    host.command_group(vec![action(1), action(2), action(2)])
        .await
        .unwrap();
    let messages = drain(&mut rx);
    assert_eq!(
        messages
            .iter()
            .filter(|m| matches!(m, ServerMessage::ActionReply(_)))
            .count(),
        3
    );
    let World::Saved(s) = &host.world else {
        panic!()
    };
    assert_eq!(s.saved().revision, revision + 1);
    assert_eq!(s.saved().action_streams[&character].last, 2);
    let persisted = eldiron_runtime::persistence::read_snapshot(&path).unwrap();
    assert_eq!(persisted.action_streams[&character].last, 2);
    // Remove the referenced parts while an action produces replies/frames. A
    // failed commit must discard the whole output gate, including cached replies.
    let parts = path.with_file_name("world.save.parts");
    let parked = path.with_file_name("parked.parts");
    std::fs::rename(&parts, &parked).unwrap();
    assert!(
        host.command_group(vec![action(2), action(3)])
            .await
            .is_err()
    );
    assert!(drain(&mut rx).is_empty());
    let World::Saved(s) = &host.world else {
        panic!()
    };
    assert!(s.needs_reload());
    drop(host);
    std::fs::rename(&parked, &parts).unwrap();
    let restored = eldiron_runtime::persistence::read_snapshot(&path).unwrap();
    assert_eq!(restored.action_streams[&character].last, 2);
    assert_eq!(restored.revision, revision + 1);
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn opt_in_sleep_wakes_for_guest_and_policy_survives_demo_reset() {
    let mut project = project();
    let names = toml::Value::Array(project.regions.iter().map(|r| toml::Value::String(r.name.clone())).collect());
    project.config.push_str(&format!("\n[multiplayer.simulation]\npause_unoccupied_regions={names}\nidle_after_ms=20\n"));
    let mut host = Host::open(project, None).await.unwrap(); host.enable_diagnostics(true);
    for _ in 0..5 { host.tick(Duration::from_millis(20)).await.unwrap(); }
    assert!(host.diagnostics().scheduling.iter().all(|r| r.eligible && r.suspended));
    let (id, mut rx) = peer(&mut host).await;
    message(&mut host, id, ClientMessage::Guest { name: "Wake guest".into() }).await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    assert!(drain(&mut rx).iter().any(|m| matches!(m, ServerMessage::Frame { .. })));
    let d = host.diagnostics(); assert_eq!(d.players, 1); assert!(d.scheduling.iter().any(|r| !r.suspended));
    host.command(Command::Close(id)).await.unwrap();
    for _ in 0..5 { host.tick(Duration::from_millis(20)).await.unwrap(); }
    assert!(host.diagnostics().scheduling.iter().all(|r| r.eligible && r.suspended));
    assert!(!host.world.game().server.has_local_player());
}
