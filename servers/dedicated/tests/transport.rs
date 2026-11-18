#[allow(dead_code)] // Shared impairment helper; this case uses delay only.
#[path = "support/relay.rs"]
mod relay;
use eldiron_network::{native, remote::RemoteClient, *};
use eldiron_runtime::Project;
use eldiron_server::{
    host::Host,
    transport::{self, TransportConfig},
};
use std::{sync::Arc, time::Duration};
use theframework_core::Uuid;
use tokio_rustls::rustls;
fn project() -> Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    project
}
async fn next(socket: &mut native::Socket) -> ServerMessage {
    tokio::time::timeout(Duration::from_secs(10), native::receive(socket))
        .await
        .unwrap()
        .unwrap()
}
async fn frame(socket: &mut native::Socket) -> (eldiron_runtime::ClientFrame, Vec<ContentHint>) {
    loop {
        if let ServerMessage::Frame { frame, content } = next(socket).await {
            return (frame, content);
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_wss_clients_share_private_snapshots_and_https_collision() {
    let dir = std::env::temp_dir().join(format!("eldiron-tls-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    let output = std::process::Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-sha256",
            "-nodes",
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName=DNS:localhost",
            "-addext",
            "basicConstraints=critical,CA:FALSE",
            "-keyout",
        ])
        .arg(&key)
        .arg("-out")
        .arg(&cert)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tls = transport::load_tls(&cert, &key).unwrap();
    let certbytes = std::fs::read(&cert).unwrap();
    let certder = rustls_pemfile::certs(&mut std::io::Cursor::new(&certbytes))
        .next()
        .unwrap()
        .unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(certder).unwrap();
    let client = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let connector = || {
        Some(tokio_tungstenite::Connector::Rustls(Arc::new(
            client.clone(),
        )))
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let relay = relay::Relay::open(port).await;
    let endpoint = relay.endpoint();
    let host = Host::open(project(), Some(dir.join("world.save")))
        .await
        .unwrap();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(transport::serve(
        listener,
        host,
        TransportConfig {
            tls: Some(tls),
            origins: vec![],
        },
        async {
            let _ = shutdown.await;
        },
    ));
    // Public roots must reject this self-signed host; explicit trust succeeds.
    assert!(native::connect(&endpoint, false).await.is_err());
    let mut a = native::connect_with_connector(&endpoint, false, connector())
        .await
        .unwrap();
    let mut b = native::connect_with_connector(&endpoint, false, connector())
        .await
        .unwrap();
    for socket in [&mut a, &mut b] {
        native::send(
            socket,
            &ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            next(socket).await,
            ServerMessage::Welcome {
                mode: HostMode::Persistent,
                ..
            }
        ));
    }
    for (socket, username) in [(&mut a, "alice"), (&mut b, "bob")] {
        native::send(
            socket,
            &ClientMessage::Login {
                username: username.into(),
                password: Secret("incorrect password".into()),
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            next(socket).await,
            ServerMessage::Rejected {
                code: RejectCode::InvalidCredentials
            }
        ));
        native::send(
            socket,
            &ClientMessage::Register {
                username: username.into(),
                password: Secret("account test password".into()),
            },
        )
        .await
        .unwrap();
        assert!(matches!(
            next(socket).await,
            ServerMessage::Authorized { .. }
        ));
    }
    native::send(
        &mut a,
        &ClientMessage::CreateCharacter {
            request: Uuid::new_v4(),
            name: "Alice".into(),
        },
    )
    .await
    .unwrap();
    native::send(
        &mut b,
        &ClientMessage::CreateCharacter {
            request: Uuid::new_v4(),
            name: "Bob".into(),
        },
    )
    .await
    .unwrap();
    async fn activated(
        socket: &mut native::Socket,
    ) -> (Uuid, eldiron_runtime::ClientFrame, Vec<ContentHint>) {
        let stream = loop {
            if let ServerMessage::ActionStream {
                stream,
                next_sequence: 1,
            } = next(socket).await
            {
                break stream;
            }
        };
        let (frame, content) = frame(socket).await;
        (stream, frame, content)
    }
    let (sa, fa, ca) = activated(&mut a).await;
    let (sb, fb, _) = activated(&mut b).await;
    assert_eq!(fa.world, fb.world);
    assert_ne!(fa.connection, fb.connection);
    assert_ne!(fa.controlled_entity, fb.controlled_entity);
    let http = reqwest::Client::builder()
        .tls_built_in_root_certs(false)
        .add_root_certificate(reqwest::Certificate::from_pem(&certbytes).unwrap())
        .build()
        .unwrap();
    let mut remote = RemoteClient::new(&fa);
    for hint in ca {
        let content = native::fetch_content_with_client(&endpoint, &hint, &http)
            .await
            .unwrap();
        remote.install(&hint, content).unwrap();
    }
    remote.receive(&fa).unwrap();
    remote
        .motion
        .set_intent(eldiron_runtime::MovementIntent::Forward);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
        remote
            .motion
            .set_intent(eldiron_runtime::MovementIntent::Forward);
        native::send(&mut a, &remote.advance(Duration::from_millis(20)))
            .await
            .unwrap();
        let (frame, hints) = frame(&mut a).await;
        for hint in hints {
            if remote.needs_content(&hint) {
                let content = native::fetch_content_with_client(&endpoint, &hint, &http)
                    .await
                    .unwrap();
                remote.install(&hint, content).unwrap();
            }
        }
        remote.receive(&frame).unwrap();
        if remote
            .motion
            .authority()
            .motion()
            .unwrap()
            .processed_sequence
            > 0
        {
            break;
        }
    }
    assert!(remote.motion.diagnostics().predicted_steps > 0);
    assert!(
        remote
            .motion
            .authority()
            .motion()
            .unwrap()
            .processed_sequence
            > 0
    );
    // Eventually each recipient sees the other's public body, never inventory/wallet.
    for (socket, stream) in [(&mut a, sa), (&mut b, sb)] {
        native::send(socket, &ClientMessage::Resync).await.unwrap();
        loop {
            let (frame, _) = frame(socket).await;
            if let eldiron_runtime::replication::WorldUpdate::Baseline(regions) = frame.update {
                if regions
                    .iter()
                    .flat_map(|r| &r.entities)
                    .filter(|e| {
                        e.id == fa.controlled_entity.unwrap()
                            || e.id == fb.controlled_entity.unwrap()
                    })
                    .count()
                    < 2
                {
                    continue;
                }
                for entity in regions.iter().flat_map(|r| &r.entities) {
                    if Some(entity.id) != frame.controlled_entity {
                        assert!(entity.inventory.is_none());
                        assert!(entity.wallet.is_none());
                    }
                }
                let action = GameAction {
                    stream,
                    sequence: 1,
                    world: frame.world,
                    generation: frame.generation,
                    region: frame.current_region.unwrap(),
                    request: GameRequest::Intent("look".into()),
                };
                for _ in 0..2 {
                    native::send(socket, &ClientMessage::Action(action.clone()))
                        .await
                        .unwrap();
                    loop {
                        if let ServerMessage::ActionReply(reply) = next(socket).await {
                            assert_eq!(
                                reply.result,
                                ActionResult::Outcome(RequestOutcome::Processed)
                            );
                            assert_eq!(reply.next_sequence, 2);
                            break;
                        }
                    }
                }
                break;
            }
        }
    }
    let _ = stop.send(());
    server.await.unwrap().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}
