use eldiron_network::*;
use eldiron_runtime::replication::{SpatialPolicy, SpatialSubscriptions, WorldUpdate};
use eldiron_server::host::{Command, Host};
use rusterix::{Value, server::message::RegionMessage};
use std::time::Duration;
use theframework_core::Uuid;
use tokio::sync::mpsc;
fn drain(rx: &mut mpsc::Receiver<Vec<u8>>) -> Vec<ServerMessage> {
    let mut out = vec![];
    while let Ok(bytes) = rx.try_recv() {
        out.push(codec::decode_server(&bytes).unwrap());
    }
    out
}
async fn send(host: &mut Host, peer: Uuid, message: ClientMessage) {
    host.command(Command::Message { peer, message })
        .await
        .unwrap();
}
async fn guest(host: &mut Host, name: &str) -> (Uuid, mpsc::Receiver<Vec<u8>>) {
    let id = Uuid::new_v4();
    let (output, rx) = mpsc::channel(64);
    host.command(Command::Open { peer: id, output })
        .await
        .unwrap();
    send(
        host,
        id,
        ClientMessage::Hello {
            version: PROTOCOL_VERSION,
        },
    )
    .await;
    send(host, id, ClientMessage::Guest { name: name.into() }).await;
    (id, rx)
}
async fn content(host: &mut Host, hint: &ContentHint) -> PhysicsContent {
    let (reply, rx) = tokio::sync::oneshot::channel();
    host.command(Command::Content {
        key: hint.key.clone(),
        reply,
    })
    .await
    .unwrap();
    remote::verify_content(hint, &rx.await.unwrap().unwrap()).unwrap()
}
#[tokio::test]
async fn dedicated_interest_and_preloads_do_not_grant_distant_state_and_geometry_matches_collision_revision()
 {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    let mut other = project.regions[0].clone();
    other.id = Uuid::new_v4();
    other.map.id = Uuid::new_v4();
    other.name = "Preload map".into();
    other.map.name = other.name.clone();
    let destination = other.map.id;
    project.regions.push(other);
    let mut config = project.config.parse::<toml::Table>().unwrap();
    config.insert(
        "multiplayer".into(),
        toml::Value::Table(
            toml::from_str("[spatial]\nradius=8\nmargin=4\nprefetch_regions=['Preload map']")
                .unwrap(),
        ),
    );
    project.config = toml::to_string(&config).unwrap();
    let mut host = Host::open(project, None).await.unwrap();
    let (alice, mut a) = guest(&mut host, "Alice").await;
    let (_bob, mut b) = guest(&mut host, "Bob").await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    let am = drain(&mut a);
    let bm = drain(&mut b);
    let (frame, hints) = am
        .iter()
        .find_map(|m| match m {
            ServerMessage::Frame { frame, content } => Some((frame.clone(), content.clone())),
            _ => None,
        })
        .unwrap();
    let bob = bm
        .iter()
        .find_map(|m| match m {
            ServerMessage::Frame { frame, .. } => frame.controlled_entity,
            _ => None,
        })
        .unwrap();
    assert!(matches!(&frame.update,WorldUpdate::Baseline(rs) if rs.len()==1));
    assert_eq!(hints.len(), 1);
    let preload = am
        .iter()
        .find_map(|m| match m {
            ServerMessage::Preload { content } => content.first(),
            _ => None,
        })
        .unwrap();
    assert_eq!(preload.reference.map, destination);
    let loaded = content(&mut host, preload).await;
    let mut cache = ContentCache::default();
    cache.insert(preload.clone(), loaded).unwrap();
    assert!(cache.get(preload).is_some());
    let connection = host
        .world
        .game()
        .server
        .player_connections()
        .find(|(id, _)| id.uuid() == frame.connection)
        .unwrap()
        .0;
    assert!(
        host.world
            .game()
            .client_world(connection)
            .unwrap()
            .map(destination)
            .is_none()
    );
    let ctx = host.world.game().server.instances[0]
        .lock()
        .unwrap()
        .context()
        .unwrap();
    {
        let mut ctx = ctx.lock().unwrap();
        let position = ctx
            .map
            .entities
            .iter()
            .find(|e| Some(e.id) == frame.controlled_entity)
            .unwrap()
            .position;
        let other = ctx.map.entities.iter_mut().find(|e| e.id == bob).unwrap();
        other.position = position;
        other.position.x += 100.;
        other.set_attribute("private_note", Value::Str("BOB_PRIVATE".into()));
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::Say(
                0,
                Some(bob),
                None,
                "DISTANT_SPEECH".into(),
                "say".into(),
            ))
            .unwrap();
    }
    host.tick(Duration::from_millis(20)).await.unwrap();
    let messages = drain(&mut a);
    drain(&mut b);
    assert!(!messages.iter().any(|m| {
        match m {
            ServerMessage::Frame { frame, .. } => serde_json::to_string(frame)
                .unwrap()
                .contains("DISTANT_SPEECH"),
            _ => false,
        }
    }));
    assert!(
        !host
            .world
            .game()
            .client_world(connection)
            .unwrap()
            .current_map()
            .unwrap()
            .entities
            .iter()
            .any(|e| e.id == bob)
    );
    let mut subscriptions = SpatialSubscriptions {
        policy: SpatialPolicy {
            radius: 8.,
            margin: 4.,
        },
        ..Default::default()
    };
    subscriptions.entities.insert(bob);
    host.world
        .game()
        .set_spatial_subscriptions(connection, Some(subscriptions))
        .unwrap();
    host.tick(Duration::from_millis(20)).await.unwrap();
    let messages = drain(&mut a);
    assert!(
        !messages
            .iter()
            .any(|m| format!("{m:?}").contains("BOB_PRIVATE"))
    );
    let public = host
        .world
        .game()
        .client_world(connection)
        .unwrap()
        .current_map()
        .unwrap()
        .entities
        .iter()
        .find(|e| e.id == bob)
        .unwrap()
        .clone();
    assert!(public.inventory.is_empty());
    assert!(public.wallet.balances.is_empty());
    let before_generation = host
        .world
        .game()
        .client_world(connection)
        .unwrap()
        .generation();
    let before_ref = host
        .world
        .game()
        .client_world(connection)
        .unwrap()
        .content_reference(hints[0].reference.map)
        .unwrap();
    ctx.lock()
        .unwrap()
        .collision_world
        .update_chunk([999, 999].into(), Default::default());
    host.tick(Duration::from_millis(20)).await.unwrap();
    let physics_messages = drain(&mut a);
    let (physics_frame, physics_hint) = physics_messages
        .iter()
        .find_map(|m| match m {
            ServerMessage::Frame { frame, content } => content
                .iter()
                .find(|h| h.reference == before_ref)
                .map(|hint| (frame, hint.clone())),
            _ => None,
        })
        .expect("off-interest physics changes still publish a package");
    assert_eq!(
        physics_frame.generation, before_generation,
        "physics-only refresh cannot reset held movement/dialogue"
    );
    assert!(matches!(physics_frame.update, WorldUpdate::Delta(_)));
    let physics = content(&mut host, &physics_hint).await;
    assert_eq!(physics.physics_revision, physics_hint.physics_revision);
    // A runtime layout edit publishes a new immutable geometry+physics package,
    // including changes not represented by an interested prop delta.
    let old = host
        .world
        .game()
        .client_world(connection)
        .unwrap()
        .content_reference(hints[0].reference.map)
        .unwrap();
    let changed_x;
    {
        let mut ctx = ctx.lock().unwrap();
        ctx.map.add_vertex_at(123.125, 1.);
        changed_x = ctx.map.vertices.last().unwrap().x;
        ctx.collision_world
            .update_chunk([1000, 1000].into(), Default::default());
        let map = ctx.map.clone();
        ctx.from_sender
            .get()
            .unwrap()
            .send(RegionMessage::MapUpdate(0, map))
            .unwrap();
    }
    host.tick(Duration::from_millis(20)).await.unwrap();
    let messages = drain(&mut a);
    let hint = messages
        .iter()
        .filter_map(|m| match m {
            ServerMessage::Frame { content, .. } => content.first(),
            _ => None,
        })
        .find(|h| h.reference.map == old.map && h.reference != old)
        .unwrap()
        .clone();
    let package = content(&mut host, &hint).await;
    assert_eq!(package.geometry.vertices.last().unwrap().x, changed_x);
    let mut local = rusterix::Map::default();
    package.geometry.apply(&mut local);
    assert_eq!(local.vertices.last().unwrap().x, changed_x);
    assert!(local.entities.is_empty());
    assert!(
        host.world
            .game()
            .server
            .capture_durable_world()
            .unwrap()
            .regions[0]
            .entities
            .iter()
            .any(|e| e.id == bob)
    );
    send(&mut host, alice, ClientMessage::Resync).await;
    host.tick(Duration::from_millis(20)).await.unwrap();
    assert!(drain(&mut a).iter().any(|m|matches!(m,ServerMessage::Frame{frame,..} if matches!(frame.update,WorldUpdate::Baseline(_)))));
    host.shutdown().await.unwrap();
}
