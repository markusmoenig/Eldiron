use eldiron_network::*;
use eldiron_runtime::{
    Project,
    persistence::{FileWorldStore, SavePurpose, WorldStore},
};
use eldiron_server::{
    auth,
    host::{Command, Host, World},
    recovery,
    transport::{self, TransportConfig},
};
use theframework_core::Uuid;
use tokio::sync::mpsc;
fn project() -> Project {
    let mut p = Project::new();
    p.regions[0].name = "Accounts".into();
    p.regions[0].map.name = "Accounts".into();
    p.config =
        "[game]\nstart_region='Accounts'\nauto_create_player=false\n[ruleset]\nsource='project'"
            .into();
    p.rules_document.current = String::new();
    p.rules_document.original = String::new();
    p.preview_workbench_rules();
    let mut c = shared::character::Character::new();
    c.name = "Hero".into();
    c.data = "[attributes]\nplayer=true\ninventory_slots=8".into();
    let mut instance = shared::character::Character::new();
    instance.character_id = c.id;
    p.regions[0].characters.insert(instance.id, instance);
    p.characters.insert(c.id, c);
    // Use the same decoded authoring representation as the operator CLI.
    let mut p =
        shared::project_io::decode_project(&shared::project_io::encode_project(&p).unwrap())
            .unwrap();
    p.migrate_default_ruleset();
    p
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
    let mut out = Vec::new();
    while let Ok(bytes) = rx.try_recv() {
        out.push(codec::decode_server(&bytes).unwrap());
    }
    out
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
        .expect("authorization")
}
fn directory(host: &Host) -> auth::Directory {
    host.directory.read().unwrap().clone()
}
#[tokio::test]
async fn revisions_revoke_tokens_and_stale_verification_without_changing_characters() {
    let root = std::env::temp_dir().join(format!("eldiron-account-security-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("world.save");
    let p = project();
    let mut host = Host::open(p.clone(), Some(path.clone())).await.unwrap();
    let (a, mut rx) = peer(&mut host).await;
    host.command(Command::Register {
        peer: a,
        username: "alice".into(),
        hash: auth::hash("original testing password").unwrap(),
    })
    .await
    .unwrap();
    let (id, token) = authorized(&mut rx);
    message(
        &mut host,
        a,
        ClientMessage::CreateCharacter {
            request: Uuid::new_v4(),
            name: "Alice".into(),
        },
    )
    .await;
    let World::Saved(saved) = &host.world else {
        panic!()
    };
    let original_characters = saved.saved().characters.clone();
    assert_eq!(original_characters.len(), 1);
    let receipts = saved.saved().requests.len();
    let (pending, mut pending_rx) = peer(&mut host).await;
    drain(&mut pending_rx);
    message(&mut host, a, ClientMessage::LogoutAll).await;
    assert!(drain(&mut rx).iter().any(|m| matches!(
        m,
        ServerMessage::AccountUpdated(AccountUpdate::LoggedOutEverywhere)
    )));
    assert!(rx.is_closed());
    assert_eq!(directory(&host).account(id).unwrap().security_revision, 1);
    host.command(Command::Verified {
        peer: pending,
        account: id,
        security_revision: 0,
    })
    .await
    .unwrap();
    assert!(
        !drain(&mut pending_rx)
            .iter()
            .any(|m| matches!(m, ServerMessage::Authorized { .. })),
        "in-flight verification predating revocation cannot authorize"
    );
    message(
        &mut host,
        pending,
        ClientMessage::Resume {
            token: Secret(token),
        },
    )
    .await;
    assert!(drain(&mut pending_rx).iter().any(|m| matches!(
        m,
        ServerMessage::Rejected {
            code: RejectCode::InvalidCredentials
        }
    )));
    host.command(Command::Verified {
        peer: pending,
        account: id,
        security_revision: 1,
    })
    .await
    .unwrap();
    let (_, token) = authorized(&mut pending_rx);
    host.command(Command::PasswordVerified {
        peer: pending,
        account: id,
        security_revision: 0,
        hash: auth::hash("should never be installed").unwrap(),
    })
    .await
    .unwrap();
    assert!(matches!(
        drain(&mut pending_rx).last(),
        Some(ServerMessage::AccountUpdated(AccountUpdate::Rejected(
            RejectCode::InvalidCredentials
        )))
    ));
    let new_hash = auth::hash("replacement testing password").unwrap();
    host.command(Command::PasswordVerified {
        peer: pending,
        account: id,
        security_revision: 1,
        hash: new_hash,
    })
    .await
    .unwrap();
    assert!(matches!(
        drain(&mut pending_rx).last(),
        Some(ServerMessage::AccountUpdated(
            AccountUpdate::PasswordChanged
        ))
    ));
    assert_eq!(directory(&host).account(id).unwrap().security_revision, 2);
    assert!(!auth::verify(
        "original testing password",
        &directory(&host).account(id).unwrap().password_hash
    ));
    let World::Saved(saved) = &host.world else {
        panic!()
    };
    assert_eq!(
        saved.saved().requests.len(),
        receipts,
        "security changes cannot exhaust the creation ledger"
    );
    assert_eq!(
        serde_json::to_value(&saved.saved().characters).unwrap(),
        serde_json::to_value(&original_characters).unwrap()
    );
    host.shutdown().await.unwrap();
    drop(host);
    let mut host = Host::open(p.clone(), Some(path.clone())).await.unwrap();
    let (b, mut b_rx) = peer(&mut host).await;
    message(
        &mut host,
        b,
        ClientMessage::Resume {
            token: Secret(token),
        },
    )
    .await;
    assert!(drain(&mut b_rx).iter().any(|m| matches!(
        m,
        ServerMessage::Rejected {
            code: RejectCode::InvalidCredentials
        }
    )));
    let account = directory(&host).account(id).unwrap().clone();
    assert_eq!(account.security_revision, 2);
    assert!(auth::verify(
        "replacement testing password",
        &account.password_hash
    ));
    host.shutdown().await.unwrap();
    drop(host);
    let mut store = FileWorldStore::open_journaled(&path).unwrap();
    let saved = store.load().await.unwrap().unwrap();
    drop(store);
    let before = serde_json::to_value(&saved).unwrap();
    let recovered =
        recovery::reset_password(&saved, &p, "ALICE", "operator recovery password").unwrap();
    assert_eq!(serde_json::to_value(&saved).unwrap(), before);
    assert_eq!(recovered.world, saved.world);
    assert_eq!(recovered.revision, saved.revision + 1);
    assert_eq!(
        serde_json::to_value(&recovered.characters).unwrap(),
        serde_json::to_value(&saved.characters).unwrap()
    );
    assert!(recovered.requests == saved.requests);
    assert!(recovered.action_streams == saved.action_streams);
    let accounts: auth::Directory =
        serde_json::from_value(recovered.host_data["accounts.v1"].clone()).unwrap();
    assert_eq!(accounts.account(id).unwrap().security_revision, 3);
    assert!(auth::verify(
        "operator recovery password",
        &accounts.account(id).unwrap().password_hash
    ));
    assert!(recovery::reset_password(&saved, &p, "unknown", "operator recovery password").is_err());
    let mut wrong = saved.clone();
    wrong.purpose = SavePurpose::SinglePlayer;
    assert!(recovery::reset_password(&wrong, &p, "alice", "operator recovery password").is_err());
    // Exercise the operator executable, including pipe-only secrets and fresh outputs.
    let project_path = root.join("private.eldiron");
    std::fs::write(
        &project_path,
        shared::project_io::encode_project(&p).unwrap(),
    )
    .unwrap();
    let cli_path = root.join("cli-recovered.save");
    let source_bytes = std::fs::read(&path).unwrap();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_eldiron-world"))
        .arg("reset-password")
        .arg("--source")
        .arg(&path)
        .arg("--project")
        .arg(&project_path)
        .args(["--username", "alice", "--password-stdin", "--output"])
        .arg(&cli_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"cli recovery testing password\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["revision"], saved.revision + 1);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("password"));
    assert_eq!(std::fs::read(&path).unwrap(), source_bytes);
    let cli = eldiron_runtime::persistence::read_snapshot(&cli_path).unwrap();
    assert_eq!(
        serde_json::to_value(&cli.characters).unwrap(),
        serde_json::to_value(&saved.characters).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&cli.world_state).unwrap(),
        serde_json::to_value(&saved.world_state).unwrap()
    );
    let accounts: auth::Directory =
        serde_json::from_value(cli.host_data["accounts.v1"].clone()).unwrap();
    assert_eq!(accounts.account(id).unwrap().security_revision, 3);
    assert!(auth::verify(
        "cli recovery testing password",
        &accounts.account(id).unwrap().password_hash
    ));
    let overwrite = std::process::Command::new(env!("CARGO_BIN_EXE_eldiron-world"))
        .arg("reset-password")
        .arg("--source")
        .arg(&path)
        .arg("--project")
        .arg(&project_path)
        .args(["--username", "alice", "--password-stdin", "--output"])
        .arg(&cli_path)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(!overwrite.status.success());
    assert!(String::from_utf8_lossy(&overwrite.stderr).contains("Output exists"));
    let recovered_path = root.join("recovered.save");
    eldiron_runtime::persistence::write_snapshot_new(&recovered_path, &recovered).unwrap();
    let mut recovered_host = Host::open(p, Some(recovered_path)).await.unwrap();
    assert_eq!(
        directory(&recovered_host)
            .account(id)
            .unwrap()
            .security_revision,
        3
    );
    recovered_host.shutdown().await.unwrap();
    drop(recovered_host);
    std::fs::remove_dir_all(root).unwrap();
}
async fn next(socket: &mut native::Socket) -> ServerMessage {
    tokio::time::timeout(std::time::Duration::from_secs(5), native::receive(socket))
        .await
        .unwrap()
        .unwrap()
}
async fn connect(endpoint: &str) -> native::Socket {
    let mut s = native::connect(endpoint, true).await.unwrap();
    native::send(
        &mut s,
        &ClientMessage::Hello {
            version: PROTOCOL_VERSION,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        next(&mut s).await,
        ServerMessage::Welcome {
            account_management: true,
            ..
        }
    ));
    s
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_socket_password_change_checks_old_password_and_durably_signs_out() {
    let root = std::env::temp_dir().join(format!("eldiron-account-socket-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("world.save");
    let host = Host::open(project(), Some(path.clone())).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("ws://{}/play", listener.local_addr().unwrap());
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(transport::serve(
        listener,
        host,
        TransportConfig {
            tls: None,
            origins: vec![],
        },
        async {
            let _ = shutdown.await;
        },
    ));
    let mut s = connect(&endpoint).await;
    native::send(
        &mut s,
        &ClientMessage::ChangePassword {
            current_password: Secret("original testing password".into()),
            new_password: Secret("replacement testing password".into()),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        next(&mut s).await,
        ServerMessage::AccountUpdated(AccountUpdate::Rejected(RejectCode::AuthenticationRequired))
    ));
    native::send(
        &mut s,
        &ClientMessage::Register {
            username: "alice".into(),
            password: Secret("original testing password".into()),
        },
    )
    .await
    .unwrap();
    let ServerMessage::Authorized { account, token, .. } = next(&mut s).await else {
        panic!()
    };
    native::send(
        &mut s,
        &ClientMessage::ChangePassword {
            current_password: Secret("incorrect testing password".into()),
            new_password: Secret("replacement testing password".into()),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        next(&mut s).await,
        ServerMessage::AccountUpdated(AccountUpdate::Rejected(RejectCode::InvalidCredentials))
    ));
    native::send(&mut s, &ClientMessage::Characters)
        .await
        .unwrap();
    assert!(
        matches!(next(&mut s).await, ServerMessage::Characters(_)),
        "failed password change keeps the session authorized"
    );
    native::send(
        &mut s,
        &ClientMessage::ChangePassword {
            current_password: Secret("original testing password".into()),
            new_password: Secret("replacement testing password".into()),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        next(&mut s).await,
        ServerMessage::AccountUpdated(AccountUpdate::PasswordChanged)
    ));
    assert!(native::receive(&mut s).await.is_err());
    let mut other = connect(&endpoint).await;
    native::send(&mut other, &ClientMessage::Resume { token })
        .await
        .unwrap();
    assert!(matches!(
        next(&mut other).await,
        ServerMessage::Rejected {
            code: RejectCode::InvalidCredentials
        }
    ));
    native::send(
        &mut other,
        &ClientMessage::Login {
            username: "alice".into(),
            password: Secret("original testing password".into()),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        next(&mut other).await,
        ServerMessage::Rejected {
            code: RejectCode::InvalidCredentials
        }
    ));
    native::send(
        &mut other,
        &ClientMessage::Login {
            username: "alice".into(),
            password: Secret("replacement testing password".into()),
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(next(&mut other).await,ServerMessage::Authorized{account:id,..} if id==account)
    );
    native::send(&mut other, &ClientMessage::LogoutAll)
        .await
        .unwrap();
    assert!(matches!(
        next(&mut other).await,
        ServerMessage::AccountUpdated(AccountUpdate::LoggedOutEverywhere)
    ));
    let _ = stop.send(());
    server.await.unwrap().unwrap();
    let mut store = FileWorldStore::open_journaled(&path).unwrap();
    let saved = store.load().await.unwrap().unwrap();
    let a: auth::Directory =
        serde_json::from_value(saved.host_data["accounts.v1"].clone()).unwrap();
    assert_eq!(a.account(account).unwrap().security_revision, 2);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
