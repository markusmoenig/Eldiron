//! Run with `cargo test --release -p eldiron-server --test release_network`.
//! Requires loopback sockets and openssl. Private fixtures are disposable.
mod support {
    pub mod peer;
    pub mod process;
    pub mod relay;
    pub mod state;
}
use eldiron_network::*;
use eldiron_runtime::persistence::{self, SavedEntity, SavedItem};
use std::{collections::BTreeMap, sync::atomic::Ordering, time::Duration};
use support::state::{committed, items, owns};
use support::{
    peer::{Context, Peer},
    process::Fixture,
    relay::Relay,
};
use theframework_core::Uuid;
fn action(context: Context, stream: Uuid, sequence: u64, request: GameRequest) -> GameAction {
    GameAction {
        world: context.world,
        generation: context.generation,
        region: context.region,
        stream,
        sequence,
        request,
    }
}
async fn reply(peer: &mut Peer, sequence: u64) -> ActionReply {
    peer.reply(sequence).await
}
async fn pong(peer: &mut Peer, nonce: u32) {
    peer.ping(nonce).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delayed_wss_contests_lost_replies_process_crashes_and_peer_isolation() {
    let fixture = Fixture::new();
    let initial = fixture.dir.join("initial.save");
    let mut first = fixture.start(&initial, "first").await;
    let mut avatars = vec![];
    let mut starter = BTreeMap::new();
    let users = ["alice", "bob", "charlie"];
    for user in users {
        let relay = Relay::open(first.port).await;
        let mut peer = Peer::connect(&relay.endpoint(), fixture.tls.clone()).await;
        peer.credentials(user, true).await;
        let request = Uuid::new_v4();
        peer.send(ClientMessage::CreateCharacter {
            request,
            name: user.into(),
        })
        .await;
        let ServerMessage::CharacterCreated { character, .. } = peer
            .until(|m| matches!(m, ServerMessage::CharacterCreated { .. }))
            .await
        else {
            unreachable!()
        };
        assert_eq!(peer.stream(character).await, 1);
        avatars.push((
            character,
            request,
            peer.context.lock().unwrap().unwrap().body,
        ));
    }
    let original = persistence::read_snapshot(&initial).unwrap();
    assert_eq!(original.characters.len(), 3);
    for (id, n) in items(&original) {
        assert_eq!(n, 1);
        starter.insert(id, n);
    }
    first.kill(); // SIGKILL, no checkpoint/shutdown opportunity.
    let mut saved = persistence::read_snapshot(&initial).unwrap();
    assert_eq!(items(&saved), starter);
    assert_eq!(saved.characters.len(), 3);
    let region = saved.characters[0].region;
    let r = saved
        .world_state
        .regions
        .iter_mut()
        .find(|r| r.geometry.id == region)
        .unwrap();
    let a = r
        .entities
        .iter()
        .find(|e| e.id == avatars[0].2)
        .unwrap()
        .position;
    let b = r
        .entities
        .iter()
        .find(|e| e.id == avatars[1].2)
        .unwrap()
        .position;
    assert!(
        (a - b).magnitude() < 3.,
        "fixture players must share pickup reach"
    );
    for e in &mut r.entities {
        if avatars.iter().any(|(_, _, body)| *body == e.id)
            && !e.inventory.iter().any(Option::is_none)
        {
            e.inventory.push(None);
        }
    }
    let contest = saved.world_state.next_id;
    saved.world_state.next_id += 1;
    let mut item = rusterix::Item::default();
    item.id = contest;
    item.position = (a + b) * 0.5;
    item.set_attribute("name", rusterix::Value::Str("Contested item".into()));
    r.items.push(SavedItem::capture(&item));
    // A valid, large public baseline gives the slow-reader test real pressure.
    for n in 0..256 {
        let mut e = rusterix::Entity::default();
        e.id = saved.world_state.next_id;
        saved.world_state.next_id += 1;
        e.position.x = a.x + 10. + (n % 16) as f32 * 0.6;
        e.position.z = a.z + 10. + (n / 16) as f32 * 0.6;
        e.position.y = a.y;
        e.set_attribute(
            "name",
            rusterix::Value::Str(format!("Soak NPC {n} {}", "x".repeat(512))),
        );
        e.set_attribute("visible", rusterix::Value::Bool(true));
        r.entities.push(SavedEntity::capture(&e));
    }
    saved.revision += 1;
    let state = fixture.dir.join("world.save");
    persistence::write_snapshot_new(&state, &saved).unwrap();
    let mut server = fixture.start(&state, "second").await;
    let mut relays = vec![];
    let mut peers = vec![];
    let mut tokens = vec![];
    for (index, user) in users.into_iter().enumerate() {
        let relay = Relay::open(server.port).await;
        let mut peer = Peer::connect(&relay.endpoint(), fixture.tls.clone()).await;
        tokens.push(peer.credentials(user, false).await);
        // Creation retry after process loss must not allocate starters or a body.
        peer.send(ClientMessage::CreateCharacter {
            request: avatars[index].1,
            name: user.into(),
        })
        .await;
        let ServerMessage::CharacterCreated { character, .. } = peer
            .until(|m| matches!(m, ServerMessage::CharacterCreated { .. }))
            .await
        else {
            unreachable!()
        };
        assert_eq!(character, avatars[index].0);
        assert_eq!(peer.join(character).await, 1);
        assert_eq!(peer.context.lock().unwrap().unwrap().body, avatars[index].2);
        relays.push(relay);
        peers.push(peer);
    }
    let pickup = |index: usize| {
        action(
            peers[index].context.lock().unwrap().unwrap(),
            avatars[index].0,
            1,
            GameRequest::Transfer {
                source: ItemSource::Direct(ItemAddress {
                    item: contest,
                    owner: ItemOwner::World,
                }),
                destination: ItemDestination::Inventory(None),
            },
        )
    };
    let a = pickup(0);
    let b = pickup(1);
    tokio::join!(
        peers[0].send(ClientMessage::Action(a.clone())),
        peers[1].send(ClientMessage::Action(b.clone()))
    );
    let ar = reply(&mut peers[0], 1).await;
    let br = reply(&mut peers[1], 1).await;
    let processed = ActionResult::Outcome(RequestOutcome::Processed);
    assert_ne!(
        ar.result == processed,
        br.result == processed,
        "exactly one competing pickup must succeed"
    );
    let winner = usize::from(br.result == processed);
    let loser = 1 - winner;
    let durable = committed(&state, avatars[winner].0, 1).await;
    assert_eq!(items(&durable)[&contest], 1);
    assert!(owns(&durable, avatars[winner].2, contest));
    assert!(!owns(&durable, avatars[loser].2, contest));
    let context = peers[winner].context.lock().unwrap().unwrap();
    let position = durable
        .world_state
        .regions
        .iter()
        .flat_map(|r| &r.entities)
        .find(|e| e.id == context.body)
        .unwrap()
        .position;
    let drop_request = action(
        context,
        avatars[winner].0,
        2,
        GameRequest::Transfer {
            source: ItemSource::Direct(ItemAddress {
                item: contest,
                owner: ItemOwner::Owned,
            }),
            destination: ItemDestination::Ground([position.x, position.z]),
        },
    );
    relays[winner].pause();
    peers[winner]
        .send(ClientMessage::Action(drop_request.clone()))
        .await;
    let durable = committed(&state, avatars[winner].0, 2).await;
    assert_eq!(items(&durable)[&contest], 1);
    assert!(!owns(&durable, context.body, contest));
    relays[winner].cut();
    pong(&mut peers[loser], 101).await;
    let mut resumed = Peer::connect(&relays[winner].endpoint(), fixture.tls.clone()).await;
    resumed
        .send(ClientMessage::Resume {
            token: Secret(tokens[winner].clone()),
        })
        .await;
    let ServerMessage::Authorized { token, .. } = resumed
        .until(|m| matches!(m, ServerMessage::Authorized { .. }))
        .await
    else {
        unreachable!()
    };
    assert_eq!(resumed.join(avatars[winner].0).await, 3);
    // A second resumed socket takes exclusive control and rotates the bearer.
    let mut takeover = Peer::connect(&relays[winner].endpoint(), fixture.tls.clone()).await;
    takeover.send(ClientMessage::Resume { token }).await;
    let ServerMessage::Authorized {
        token: restart_token,
        ..
    } = takeover
        .until(|m| matches!(m, ServerMessage::Authorized { .. }))
        .await
    else {
        unreachable!()
    };
    resumed.closed().await;
    assert_eq!(takeover.join(avatars[winner].0).await, 3);
    takeover
        .send(ClientMessage::Action(drop_request.clone()))
        .await;
    assert_eq!(reply(&mut takeover, 2).await.result, processed);
    let mut conflict = drop_request.clone();
    conflict.request = GameRequest::Intent("look".into());
    takeover.send(ClientMessage::Action(conflict)).await;
    assert_eq!(reply(&mut takeover, 2).await.result, ActionResult::Conflict);
    let new_context = takeover.context.lock().unwrap().unwrap();
    let pickup = action(
        new_context,
        avatars[winner].0,
        3,
        GameRequest::Transfer {
            source: ItemSource::Direct(ItemAddress {
                item: contest,
                owner: ItemOwner::World,
            }),
            destination: ItemDestination::Inventory(None),
        },
    );
    relays[winner].pause();
    takeover.send(ClientMessage::Action(pickup.clone())).await;
    let durable = committed(&state, avatars[winner].0, 3).await;
    assert!(owns(&durable, avatars[winner].2, contest));
    assert_eq!(items(&durable)[&contest], 1);
    let head: serde_json::Value = serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    assert_eq!(head["format"], 3);
    assert!(
        head["head"]["records"].as_u64().unwrap() > 0,
        "crash must exercise journal replay"
    );
    server.kill();
    drop(takeover);
    drop(peers);
    drop(relays);
    let mut server = fixture.start(&state, "third").await;
    let mut relays = vec![];
    let mut peers = vec![];
    let mut live_tokens = vec![];
    for (index, user) in users.into_iter().enumerate() {
        let relay = Relay::open(server.port).await;
        let mut peer = Peer::connect(&relay.endpoint(), fixture.tls.clone()).await;
        if index == winner {
            peer.send(ClientMessage::Resume {
                token: Secret(restart_token.0.clone()),
            })
            .await;
            peer.until(|m| {
                matches!(
                    m,
                    ServerMessage::Rejected {
                        code: RejectCode::InvalidCredentials
                    }
                )
            })
            .await;
        }
        live_tokens.push(peer.credentials(user, false).await);
        let next = peer.join(avatars[index].0).await;
        assert_eq!(
            next,
            if index == winner {
                4
            } else if index == loser {
                2
            } else {
                1
            }
        );
        assert_ne!(peer.context.lock().unwrap().unwrap().world, context.world);
        relays.push(relay);
        peers.push(peer);
    }
    peers[winner]
        .send(ClientMessage::Action(pickup.clone()))
        .await;
    assert_eq!(reply(&mut peers[winner], 3).await.result, processed);
    peers[winner]
        .send(ClientMessage::Action(drop_request))
        .await;
    assert_eq!(reply(&mut peers[winner], 2).await.result, processed);
    let restored = persistence::read_snapshot(&state).unwrap();
    assert_eq!(restored.characters.len(), 3);
    assert!(owns(&restored, avatars[winner].2, contest));
    assert_eq!(items(&restored)[&contest], 1);
    for id in starter.keys() {
        assert_eq!(
            items(&restored).get(id),
            Some(&1),
            "starter item duplicated/lost on restart"
        );
    }
    // Full admission and foreign character selection are checked over WSS.
    let relay = Relay::open(server.port).await;
    let mut fourth = Peer::connect(&relay.endpoint(), fixture.tls.clone()).await;
    fourth.credentials("dave", true).await;
    fourth
        .send(ClientMessage::CreateCharacter {
            request: Uuid::new_v4(),
            name: "Dave".into(),
        })
        .await;
    fourth
        .until(|m| {
            matches!(
                m,
                ServerMessage::Rejected {
                    code: RejectCode::ServerBusy
                }
            )
        })
        .await;
    malformed_peers_do_not_stop_world(server.port, fixture.tls.clone()).await;
    pong(&mut peers[winner], 202).await;
    // Block only Charlie's downstream. Large repeated baselines must evict that
    // socket while healthy peers continue receiving/playing in the same world.
    let before = peers[winner].frames.load(Ordering::Relaxed);
    relays[2].pause();
    for _ in 0..90 {
        if peers[2].try_send(ClientMessage::Resync).await.is_err() {
            break;
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        let m: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.dir.join("third.metrics.json")).unwrap())
                .unwrap();
        if m["slow_disconnects"].as_u64().unwrap() > 0 {
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "slow socket was not retired"
        );
        pong(&mut peers[winner], 303).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    fourth
        .send(ClientMessage::JoinCharacter {
            character: avatars[0].0,
        })
        .await;
    fourth
        .until(|m| {
            matches!(
                m,
                ServerMessage::Rejected {
                    code: RejectCode::CharacterUnavailable
                }
            )
        })
        .await;
    pong(&mut peers[winner], 304).await;
    assert!(peers[winner].frames.load(Ordering::Relaxed) > before);
    // Same release/certificate, different world owner and directory: a valid
    // bearer and matching username/password from this game grant no access.
    let isolated_path = fixture.dir.join("isolated.save");
    let mut isolated = fixture.start(&isolated_path, "isolated").await;
    let isolated_relay = Relay::open(isolated.port).await;
    let mut outsider = Peer::connect(&isolated_relay.endpoint(), fixture.tls.clone()).await;
    outsider
        .send(ClientMessage::Resume {
            token: Secret(live_tokens[winner].clone()),
        })
        .await;
    outsider
        .until(|m| {
            matches!(
                m,
                ServerMessage::Rejected {
                    code: RejectCode::InvalidCredentials
                }
            )
        })
        .await;
    outsider
        .send(ClientMessage::Login {
            username: users[winner].into(),
            password: Secret("release test password".into()),
        })
        .await;
    outsider
        .until(|m| {
            matches!(
                m,
                ServerMessage::Rejected {
                    code: RejectCode::InvalidCredentials
                }
            )
        })
        .await;
    let other = persistence::read_snapshot(&isolated_path).unwrap();
    assert_ne!(other.world, restored.world);
    assert!(other.characters.is_empty());
    isolated.kill();
    pong(&mut peers[winner], 305).await;
    #[cfg(unix)]
    {
        let revision = persistence::read_snapshot(&state).unwrap().revision;
        server.terminate().await;
        assert!(persistence::read_snapshot(&state).unwrap().revision > revision);
        let metrics: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.dir.join("third.metrics.json")).unwrap())
                .unwrap();
        assert_eq!(metrics["transport"]["active"], 0);
    }
    #[cfg(not(unix))]
    server.kill();
    let bytes: usize = relays
        .iter()
        .map(|r| r.counters.bytes.load(Ordering::Relaxed) as usize)
        .sum();
    assert!(bytes > 0);
    println!(
        "RELEASE_NETWORK {{\"transport\":\"end-to-end verified WSS through jittered fragmented TCP\",\"accounts\":4,\"characters\":3,\"forced_process_restarts\":2,\"contested_item_copies\":1,\"lost_ack_retry\":true,\"private_frames_checked\":true,\"slow_peer_isolated\":true,\"world_isolation\":true,\"sigterm_checkpoint\":{}}}",
        cfg!(unix)
    );
}
async fn malformed_peers_do_not_stop_world(
    port: u16,
    tls: std::sync::Arc<tokio_rustls::rustls::ClientConfig>,
) {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    // TLS verification is end-to-end through the relay, including hostname.
    assert!(
        native::connect_with_connector(
            &format!("wss://127.0.0.1:{port}/play"),
            false,
            Some(tokio_tungstenite::Connector::Rustls(tls.clone()))
        )
        .await
        .is_err()
    );
    for origin in [false, true] {
        let mut request = format!("wss://localhost:{port}/play")
            .into_client_request()
            .unwrap();
        request.headers_mut().insert(
            "Sec-WebSocket-Protocol",
            if origin { SUBPROTOCOL } else { "eldiron.v999" }
                .parse()
                .unwrap(),
        );
        if origin {
            request
                .headers_mut()
                .insert("Origin", "https://denied.invalid".parse().unwrap());
        }
        assert!(
            tokio_tungstenite::connect_async_tls_with_config(
                request,
                None,
                true,
                Some(tokio_tungstenite::Connector::Rustls(tls.clone()))
            )
            .await
            .is_err()
        );
    }
    use futures_util::SinkExt;
    for bytes in [vec![0u8; 3], vec![0u8; MAX_CLIENT_BYTES + 1]] {
        let endpoint = format!("wss://localhost:{port}/play");
        let mut socket = native::connect_with_connector(
            &endpoint,
            false,
            Some(tokio_tungstenite::Connector::Rustls(tls.clone())),
        )
        .await
        .unwrap();
        native::send(
            &mut socket,
            &ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        )
        .await
        .unwrap();
        native::receive(&mut socket).await.unwrap();
        let _ = socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                bytes.into(),
            ))
            .await;
        assert!(
            tokio::time::timeout(Duration::from_secs(5), native::receive(&mut socket))
                .await
                .unwrap()
                .is_err()
        );
    }
}
