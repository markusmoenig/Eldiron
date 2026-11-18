use super::*;
use eldiron_runtime::replication::{ClientFrame, REPLICATION_VERSION, WorldUpdate};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn package(revision: u64, map: Option<Uuid>) -> (ContentHint, PhysicsContent) {
    let mut source = rusterix::Map::default();
    if let Some(id) = map {
        source.id = id;
    }
    let reference = RegionContentRef {
        map: source.id,
        map_revision: source.changed,
        runtime_revision: 1,
    };
    let content = PhysicsContent {
        reference,
        physics_revision: revision,
        name: "test".into(),
        mini: Default::default(),
        collision: Default::default(),
        geometry: RegionGeometry::capture(&source).unwrap(),
    };
    let bytes = codec::encode_content(&content).unwrap();
    (
        ContentHint {
            reference,
            physics_revision: revision,
            key: format!("{:x}", Sha256::digest(bytes)),
        },
        content,
    )
}
fn frame(sequence: u64, content: Vec<ContentHint>) -> ServerMessage {
    ServerMessage::Frame {
        frame: ClientFrame {
            version: REPLICATION_VERSION,
            world: Uuid::nil(),
            connection: Uuid::nil(),
            generation: 1,
            sequence,
            tick: sequence,
            movement_tick: sequence,
            motion: None,
            controlled_entity: None,
            current_region: None,
            state: rusterix::ServerState::Running,
            update: WorldUpdate::Baseline(vec![]),
            events: vec![],
        },
        content,
    }
}
async fn http(content: &PhysicsContent, delay: Duration, status: u16) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("ws://{}/play", listener.local_addr().unwrap());
    let bytes = codec::encode_content(content).unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 4096];
        socket.read(&mut request).await.unwrap();
        tokio::time::sleep(delay).await;
        let header = format!(
            "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        );
        if socket.write_all(header.as_bytes()).await.is_ok() {
            let _ = socket.write_all(&bytes).await;
        }
    });
    endpoint
}
async fn next(stream: &mut ContentStream, endpoint: &str) -> Result<ContentDelivery, String> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Some(delivery) = stream.poll(endpoint)? {
                return Ok(delivery);
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("content delivery deadline")
}
#[tokio::test]
async fn slow_required_download_preserves_every_frame_and_control_message_in_order() {
    let (hint, content) = package(1, None);
    let endpoint = http(&content, Duration::from_millis(200), 200).await;
    let mut stream = ContentStream::default();
    stream
        .push(&endpoint, frame(1, vec![hint.clone()]))
        .unwrap();
    assert!(stream.poll(&endpoint).unwrap().is_none());
    stream
        .push(
            &endpoint,
            ServerMessage::Rejected {
                code: RejectCode::InputResync,
            },
        )
        .unwrap();
    for sequence in 2..=200 {
        stream.push(&endpoint, frame(sequence, vec![])).unwrap();
        stream
            .push(
                &endpoint,
                ServerMessage::Pong {
                    nonce: sequence as u32,
                },
            )
            .unwrap();
    }
    assert_eq!(stream.diagnostics().queued_packets, 201);
    assert!(
        matches!(next(&mut stream, &endpoint).await.unwrap(), ContentDelivery::Content(h, _) if h.key == hint.key)
    );
    assert!(
        matches!(next(&mut stream, &endpoint).await.unwrap(), ContentDelivery::Packet(ServerMessage::Frame {frame, ..}) if frame.sequence == 1)
    );
    assert!(matches!(
        next(&mut stream, &endpoint).await.unwrap(),
        ContentDelivery::Packet(ServerMessage::Rejected {
            code: RejectCode::InputResync
        })
    ));
    for sequence in 2..=200 {
        assert!(
            matches!(next(&mut stream, &endpoint).await.unwrap(), ContentDelivery::Packet(ServerMessage::Frame {frame, ..}) if frame.sequence == sequence)
        );
    }
    assert_eq!(stream.diagnostics().queued_packets, 0);
    assert_eq!(stream.diagnostics().required_downloads, 1);
}
#[tokio::test]
async fn newer_preload_cannot_replace_an_older_frames_exact_physics_package() {
    let (old, old_content) = package(1, None);
    let (new, new_content) = package(2, Some(old.reference.map));
    let endpoint = http(&old_content, Duration::from_millis(30), 200).await;
    let mut stream = ContentStream::default();
    stream.cache.insert(new.clone(), new_content).unwrap();
    stream.push(&endpoint, frame(1, vec![old.clone()])).unwrap();
    stream.push(&endpoint, frame(2, vec![new.clone()])).unwrap();
    assert!(
        matches!(next(&mut stream, &endpoint).await.unwrap(), ContentDelivery::Content(h, p) if h.key == old.key && p.physics_revision == 1)
    );
    assert!(
        matches!(next(&mut stream, &endpoint).await.unwrap(), ContentDelivery::Packet(ServerMessage::Frame {frame, ..}) if frame.sequence == 1)
    );
    assert!(
        matches!(next(&mut stream, &endpoint).await.unwrap(), ContentDelivery::Content(h, p) if h.key == new.key && p.physics_revision == 2)
    );
    assert!(
        matches!(next(&mut stream, &endpoint).await.unwrap(), ContentDelivery::Packet(ServerMessage::Frame {frame, ..}) if frame.sequence == 2)
    );
    assert_eq!(stream.diagnostics().required_downloads, 1);
}
#[test]
fn packet_and_byte_limits_fail_explicitly_without_dropping_queued_events() {
    let mut stream = ContentStream::default();
    for i in 0..MAX_WAIT_PACKETS {
        stream.push("", frame(i as u64, vec![])).unwrap();
    }
    assert!(stream.push("", frame(999, vec![])).is_err());
    assert_eq!(stream.diagnostics().queued_packets, MAX_WAIT_PACKETS);
    assert!(
        stream.poll("").is_err(),
        "overflow must remain terminal even if caller keeps polling"
    );
    let mut stream = ContentStream::default();
    let mut rejected = false;
    for _ in 0..100 {
        let packet = ServerMessage::Welcome {
            version: PROTOCOL_VERSION,
            mode: HostMode::Demo,
            content_revision: "x".repeat(100_000),
            account_management:false,
        };
        // These envelopes pass the same transport decoder as real messages.
        codec::decode_server(&codec::encode_server(&packet).unwrap()).unwrap();
        if stream.push("", packet).is_err() {
            rejected = true;
            break;
        }
    }
    assert!(rejected);
    assert!(stream.diagnostics().queued_packets < MAX_WAIT_PACKETS);
    assert!(stream.diagnostics().queued_bytes <= MAX_WAIT_BYTES);
}
#[tokio::test]
async fn missing_content_and_overdue_frame_fail_without_delivering_the_frame() {
    let (hint, content) = package(1, None);
    let endpoint = http(&content, Duration::ZERO, 404).await;
    let mut stream = ContentStream::default();
    stream.push(&endpoint, frame(1, vec![hint])).unwrap();
    let failure = next(&mut stream, &endpoint).await.err().unwrap();
    assert_eq!(stream.poll(&endpoint).err().unwrap(), failure);
    assert_eq!(stream.diagnostics().queued_packets, 1);
    let mut stream = ContentStream::default();
    stream.push(&endpoint, frame(2, vec![])).unwrap();
    stream.waiting_since = Some(instant::Instant::now() - Duration::from_secs(11));
    assert!(stream.poll(&endpoint).err().unwrap().contains("timed out"));
}
#[tokio::test]
async fn dropping_required_download_cancels_the_actual_http_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("ws://{}/play", listener.local_addr().unwrap());
    let (accepted, ready) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = [0; 4096];
        assert!(socket.read(&mut bytes).await.unwrap() > 0);
        accepted.send(()).unwrap();
        assert_eq!(
            socket.read(&mut bytes).await.unwrap(),
            0,
            "canceled HTTP client retained socket"
        );
    });
    let (hint, _) = package(1, None);
    let mut stream = ContentStream::default();
    stream.push(&endpoint, frame(1, vec![hint])).unwrap();
    assert!(stream.poll(&endpoint).unwrap().is_none());
    tokio::time::timeout(Duration::from_secs(2), ready)
        .await
        .unwrap()
        .unwrap();
    drop(stream);
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn streamed_art_is_verified_before_physics_and_frames_and_is_reused() {
    use crate::{
        public_asset_gate::{AssetDelivery, PublicAssetGate},
        public_assets::RegionAssets,
    };
    let (hint, physics) = package(1, None);
    let mut project = shared::project::Project::new();
    project.config="[game]\nmode='multiplayer'\n[multiplayer]\nendpoint='wss://example.com/play'\n[ruleset]\nsource='project'".into();
    project.regions[0].map.id = hint.reference.map;
    let bundle = shared::public_client::streaming::partition(
        shared::public_client::capture_payload(&project).unwrap(),
        "a".repeat(64),
        crate::PROTOCOL_VERSION,
    )
    .unwrap();
    let index = shared::public_client::decode(&bundle.bootstrap, crate::PROTOCOL_VERSION)
        .unwrap()
        .client_release
        .unwrap()
        .manifest
        .packs
        .clone();
    let dir = std::env::temp_dir().join(format!("eldiron-assets-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    std::fs::create_dir(dir.join("packs")).unwrap();
    for (name, bytes) in &bundle.packs {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    let source = url::Url::from_file_path(dir.join("game.eldiron-client"))
        .unwrap()
        .to_string();
    let mut gate = PublicAssetGate::new(Some(RegionAssets {
        source,
        regions: index.into_iter().filter(|r| r.map.is_some()).collect(),
    }));
    let mut stream = ContentStream::default();
    stream.cache.insert(hint.clone(), physics).unwrap();
    stream
        .push("wss://example.com/play", frame(1, vec![hint.clone()]))
        .unwrap();
    assert!(
        gate.poll(&mut stream, "wss://example.com/play")
            .unwrap()
            .is_none()
    );
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        match gate.poll(&mut stream, "wss://example.com/play").unwrap() {
            Some(AssetDelivery::Region(region)) => {
                assert_eq!(region.layout.geometry.id, hint.reference.map);
                break;
            }
            None => {}
            _ => panic!("frame/physics escaped before required art"),
        };
        assert!(std::time::Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(matches!(
        gate.poll(&mut stream, "wss://example.com/play").unwrap(),
        Some(AssetDelivery::Content(ContentDelivery::Content(..)))
    ));
    assert!(matches!(
        gate.poll(&mut stream, "wss://example.com/play").unwrap(),
        Some(AssetDelivery::Content(ContentDelivery::Packet(
            ServerMessage::Frame { .. }
        )))
    ));
    std::fs::remove_dir_all(dir).unwrap();
    stream
        .push("wss://example.com/play", frame(2, vec![hint]))
        .unwrap();
    assert!(matches!(
        gate.poll(&mut stream, "wss://example.com/play").unwrap(),
        Some(AssetDelivery::Content(ContentDelivery::Content(..)))
    ));
}
