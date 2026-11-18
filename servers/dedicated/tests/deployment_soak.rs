//! Opt-in wall-clock deployment soak. Local paced TCP/TLS is not WAN packet loss.
#![cfg(unix)]
#[allow(dead_code)]
mod support {
    pub mod peer;
    pub mod process;
    pub mod relay;
    pub mod state;
}
use eldiron_network::*;
use eldiron_runtime::persistence::{self, WorldSave};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use support::{
    peer::Peer,
    process::{Fixture, Server},
    relay::{Profile, Relay},
    state::items,
};
use theframework_core::Uuid;
struct Avatar {
    peer: Peer,
    character: Uuid,
    creation: Uuid,
    body: u32,
    next: u64,
    last: Option<(GameAction, ActionReply)>,
}
impl Avatar {
    async fn open(fixture: &Fixture, relay: &Relay, user: &str, previous: Option<&Self>) -> Self {
        let mut peer = Peer::connect(&relay.endpoint(), fixture.tls.clone()).await;
        peer.credentials(user, previous.is_none()).await;
        let creation = previous.map(|a| a.creation).unwrap_or_else(Uuid::new_v4);
        peer.send(ClientMessage::CreateCharacter {
            request: creation,
            name: user.into(),
        })
        .await;
        let ServerMessage::CharacterCreated { character, .. } = peer
            .until(|m| matches!(m, ServerMessage::CharacterCreated { .. }))
            .await
        else {
            unreachable!()
        };
        if let Some(old) = previous {
            assert_eq!(character, old.character);
        }
        let next = peer.join(character).await;
        let body = peer.context.lock().unwrap().unwrap().body;
        if let Some(old) = previous {
            assert_eq!(body, old.body);
        }
        Self {
            peer,
            character,
            creation,
            body,
            next,
            last: previous.and_then(|a| a.last.clone()),
        }
    }
    async fn request(&mut self, request: GameRequest) {
        let c = self.peer.context.lock().unwrap().unwrap();
        let action = GameAction {
            world: c.world,
            generation: c.generation,
            region: c.region,
            stream: self.character,
            sequence: self.next,
            request,
        };
        self.peer.send(ClientMessage::Action(action.clone())).await;
        let reply = self.peer.reply(self.next).await;
        assert_eq!(
            reply.result,
            ActionResult::Outcome(RequestOutcome::Processed)
        );
        assert_eq!(reply.next_sequence, self.next + 1);
        self.next = reply.next_sequence;
        self.last = Some((action, reply));
    }
    async fn retry(&mut self) {
        if let Some((action, reply)) = &self.last {
            self.peer.send(ClientMessage::Action(action.clone())).await;
            assert_eq!(
                &self.peer.reply(action.sequence).await,
                reply,
                "exact retry changed committed outcome"
            );
        }
    }
}
fn read(path: &Path) -> WorldSave {
    persistence::read_snapshot(path).unwrap()
}
fn read_metrics(fixture: &Fixture, label: &str) -> serde_json::Value {
    let v: serde_json::Value = serde_json::from_slice(
        &std::fs::read(fixture.dir.join(format!("{label}.metrics.json"))).unwrap(),
    )
    .unwrap();
    v
}
fn metrics(fixture: &Fixture, label: &str) -> serde_json::Value {
    let v = read_metrics(fixture, label);
    assert_eq!(v["healthy"], true);
    assert_eq!(v["backup_failures"], 0);
    assert_eq!(v["metrics_write_failures"], 0);
    assert!(v["peers"].as_u64().unwrap() <= 3);
    assert!(v["peak_socket_queue"].as_u64().unwrap() <= 64);
    assert!(v["transport"]["active"].as_u64().unwrap() <= 8);
    assert!(v["transport"]["command_queue_peak"].as_u64().unwrap() <= 128);
    assert!(v["content_cache_bytes"].as_u64().unwrap() <= 64 * 1024 * 1024);
    v
}
fn rss(server: &Server) -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &server.id().to_string()])
        .output()
        .ok()?;
    String::from_utf8(output.stdout)
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(|n| n * 1024)
}
fn backups(fixture: &Fixture) -> Vec<(u64, PathBuf)> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(fixture.dir.join("backups")) {
        for entry in entries {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "snapshot") {
                let saved = read(&path);
                out.push((saved.revision, path));
            }
        }
    }
    out.sort_by_key(|v| v.0);
    assert!(out.len() <= 3, "backup retention exceeded");
    out
}
async fn owner_lock_refused(fixture: &Fixture, state: &Path) {
    let log = fixture.dir.join("duplicate.log");
    let file = std::fs::File::create(&log).unwrap();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_eldiron-server"))
        .arg("--project")
        .arg(&fixture.project)
        .arg("--state")
        .arg(state)
        .args(["--bind", "127.0.0.1:0", "--insecure-loopback"])
        .stdout(file.try_clone().unwrap())
        .stderr(file)
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(!status.success());
            let reason = std::fs::read_to_string(log).unwrap();
            assert!(
                reason.to_lowercase().contains("busy"),
                "unexpected duplicate-owner refusal: {reason}"
            );
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("duplicate world owner did not refuse the active save");
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
fn check_state(path: &Path, starters: &BTreeMap<u32, usize>, avatars: &[&Avatar]) {
    let saved = read(path);
    assert_eq!(
        &items(&saved),
        starters,
        "starter identity/ownership duplicated or disappeared"
    );
    assert!(items(&saved).values().all(|n| *n == 1));
    assert_eq!(saved.characters.len(), avatars.len());
    for a in avatars {
        assert!(
            saved
                .characters
                .iter()
                .any(|c| c.id == a.character && c.entity == a.body)
        );
        assert_eq!(
            saved
                .action_streams
                .get(&a.character)
                .map(|j| j.last)
                .unwrap_or(0)
                + 1,
            a.next
        );
    }
    let parts = PathBuf::from(format!("{}.parts", path.display()));
    if parts.exists() {
        assert!(std::fs::read_dir(parts).unwrap().count() <= 16);
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "10 minute real process/TLS deployment soak; requires sockets, openssl and ps"]
async fn paced_wss_outages_restarts_backups_and_isolated_worlds() {
    let seconds = std::env::var("ELDIRON_DEPLOYMENT_SECONDS")
        .unwrap_or_else(|_| "600".into())
        .parse::<u64>()
        .unwrap();
    assert!((180..=86400).contains(&seconds));
    let fixture = Fixture::new();
    let isolated = Fixture::new();
    let mut state = fixture.dir.join("world.save");
    let other_state = isolated.dir.join("world.save");
    let mut label = "first";
    let mut server = fixture.start_configured(&state, label, true).await;
    let mut other = isolated.start(&other_state, "isolated").await;
    let relays = [
        Relay::with_profile(server.port, Profile::wan()).await,
        Relay::with_profile(server.port, Profile::wan()).await,
    ];
    let other_relay = Relay::with_profile(other.port, Profile::wan()).await;
    let mut alice = Avatar::open(&fixture, &relays[0], "alice", None).await;
    let mut bob = Avatar::open(&fixture, &relays[1], "bob", None).await;
    let mut sentinel = Avatar::open(&isolated, &other_relay, "sentinel", None).await;
    let starter = items(&read(&state));
    let other_starter = items(&read(&other_state));
    assert!(!starter.is_empty());
    let saved = read(&state);
    let entity = saved
        .world_state
        .regions
        .iter()
        .flat_map(|r| r.entities.iter().chain(&r.stashed))
        .find(|e| e.id == alice.body)
        .unwrap();
    let (slot, item) = entity
        .equipped
        .first()
        .map(|(slot, item)| (slot.clone(), item.id))
        .expect("starter equipment");
    owner_lock_refused(&fixture, &state).await;
    let started = Instant::now();
    let mut health = Instant::now() - Duration::from_secs(10);
    let mut action = health;
    let mut transfer = health;
    let mut phases = [false; 6];
    let mut outage = None;
    let mut samples = Vec::new();
    let mut rtts = Vec::new();
    let mut incidents = Vec::new();
    let mut actions = 0u64;
    let mut heartbeat = 1u32;
    let mut previous_frames = [0; 3];
    let mut memory = BTreeMap::<String, (u64, u64)>::new();
    let mut generation_summaries = Vec::new();
    while started.elapsed() < Duration::from_secs(seconds) {
        server.assert_alive();
        other.assert_alive();
        let elapsed = started.elapsed().as_secs_f64();
        let fraction = elapsed / seconds as f64;
        if !phases[0] && fraction >= 0.10 {
            phases[0] = true;
            for r in &relays {
                r.set_profile(Profile::distant());
            }
            incidents.push(
                serde_json::json!({"kind":"higher-latency-bandwidth-limit","seconds":elapsed}),
            );
        }
        if !phases[1] && fraction >= 0.20 {
            phases[1] = true;
            relays[0].pause_both();
            tokio::time::sleep(Duration::from_secs(2)).await;
            relays[0].resume();
            incidents.push(serde_json::json!({"kind":"two-second-blackout","seconds":elapsed}));
        }
        if !phases[2] && fraction >= 0.35 {
            phases[2] = true;
            relays[0].pause_both();
            outage = Some(Instant::now());
            incidents.push(serde_json::json!({"kind":"35-second-blackout","seconds":elapsed}));
        }
        if outage.is_some_and(|t: Instant| t.elapsed() >= Duration::from_secs(35)) {
            relays[0].resume();
            alice.peer.closed().await;
            let mut replacement = Avatar::open(&fixture, &relays[0], "alice", Some(&alice)).await;
            replacement.retry().await;
            alice = replacement;
            outage = None;
            previous_frames[0] = 0;
            incidents.push(serde_json::json!({"kind":"outage-reauthenticated","seconds":started.elapsed().as_secs_f64()}));
        }
        for (index, threshold, crash, next_label) in [
            (3, 0.55, false, "graceful"),
            (4, 0.75, true, "crash"),
            (5, 0.90, false, "restored"),
        ] {
            if !phases[index] && fraction >= threshold && outage.is_none() {
                phases[index] = true;
                // Receipt publication must precede process loss; retry retains the exact old DTO.
                alice.request(GameRequest::Intent("look".into())).await;
                actions += 1;
                generation_summaries.push(metrics(&fixture, label));
                let restore = if index == 5 {
                    Some(
                        backups(&fixture)
                            .last()
                            .expect("periodic backup missing")
                            .1
                            .clone(),
                    )
                } else {
                    None
                };
                if crash {
                    server.kill();
                } else {
                    server.terminate().await;
                    assert_eq!(read_metrics(&fixture, label)["transport"]["active"], 0);
                }
                if let Some(backup) = restore {
                    let source = read(&backup);
                    let next = fixture.dir.join("restored.save");
                    let status = std::process::Command::new(env!("CARGO_BIN_EXE_eldiron-world"))
                        .arg("restore")
                        .arg("--source")
                        .arg(backup)
                        .arg("--project")
                        .arg(&fixture.project)
                        .arg("--output")
                        .arg(&next)
                        .output()
                        .unwrap();
                    assert!(
                        status.status.success(),
                        "{}",
                        String::from_utf8_lossy(&status.stderr)
                    );
                    assert_eq!(read(&next).revision, source.revision);
                    state = next;
                    alice.last = None;
                    bob.last = None;
                }
                label = next_label;
                server = fixture.start_configured(&state, label, true).await;
                for relay in &relays {
                    relay.retarget(server.port);
                    relay.set_profile(Profile::wan());
                }
                let mut a = Avatar::open(&fixture, &relays[0], "alice", Some(&alice)).await;
                let mut b = Avatar::open(&fixture, &relays[1], "bob", Some(&bob)).await;
                a.retry().await;
                b.retry().await;
                alice = a;
                bob = b;
                previous_frames[0] = 0;
                previous_frames[1] = 0;
                sentinel.peer.ping(heartbeat).await;
                heartbeat += 1;
                check_state(&state, &starter, &[&alice, &bob]);
                incidents.push(serde_json::json!({"kind":next_label,"seconds":started.elapsed().as_secs_f64(),"independent_world_alive":true}));
            }
        }
        if action.elapsed() >= Duration::from_secs(5) {
            if outage.is_none() {
                alice.request(GameRequest::Intent("look".into())).await;
                actions += 1;
            }
            bob.request(GameRequest::Intent("look".into())).await;
            sentinel.request(GameRequest::Intent("look".into())).await;
            actions += 2;
            action = Instant::now();
        }
        if outage.is_none() && transfer.elapsed() >= Duration::from_secs(15) {
            let saved = read(&state);
            let e = saved
                .world_state
                .regions
                .iter()
                .flat_map(|r| r.entities.iter().chain(&r.stashed))
                .find(|e| e.id == alice.body)
                .unwrap();
            let equipped = e.equipped.iter().any(|(_, i)| i.id == item);
            alice
                .request(GameRequest::Transfer {
                    source: ItemSource::Direct(ItemAddress {
                        item,
                        owner: ItemOwner::Owned,
                    }),
                    destination: if equipped {
                        ItemDestination::Inventory(None)
                    } else {
                        ItemDestination::Equipment(slot.clone())
                    },
                })
                .await;
            alice.retry().await;
            actions += 1;
            transfer = Instant::now();
        }
        if health.elapsed() >= Duration::from_secs(10) {
            let measured = Instant::now();
            bob.peer.ping(heartbeat).await;
            rtts.push(measured.elapsed().as_secs_f64() * 1000.);
            heartbeat += 1;
            sentinel.peer.ping(heartbeat).await;
            heartbeat += 1;
            let mut peers = vec![(&bob.peer, 1), (&sentinel.peer, 2)];
            if outage.is_none() {
                peers.push((&alice.peer, 0));
            }
            for (peer, index) in peers {
                let frames = peer.frames.load(Ordering::Relaxed);
                assert!(
                    frames > previous_frames[index],
                    "healthy replica stopped advancing"
                );
                previous_frames[index] = frames;
            }
            let m = metrics(&fixture, label);
            metrics(&isolated, "isolated");
            check_state(&state, &starter, &[&alice, &bob]);
            check_state(&other_state, &other_starter, &[&sentinel]);
            let rss = rss(&server);
            if let Some(bytes) = rss {
                let entry = memory.entry(label.into()).or_insert((bytes, bytes));
                entry.1 = entry.1.max(bytes);
                assert!(
                    entry.1.saturating_sub(entry.0) <= 256 * 1024 * 1024,
                    "RSS grew more than 256 MiB within a generation"
                );
            }
            let backup_count = backups(&fixture).len();
            samples.push(serde_json::json!({"seconds":started.elapsed().as_secs_f64(),"generation":label,"rss_bytes":rss,"peers":m["peers"],"queued":m["socket_packets_queued"],"backups":backup_count}));
            println!(
                "DEPLOYMENT_PROGRESS {}",
                serde_json::json!({"seconds":started.elapsed().as_secs(),"generation":label,"actions":actions,"samples":samples.len()})
            );
            health = Instant::now();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(phases.iter().all(|p| *p));
    assert!(outage.is_none());
    assert!(backups(&fixture).len() >= 1);
    check_state(&state, &starter, &[&alice, &bob]);
    check_state(&other_state, &other_starter, &[&sentinel]);
    generation_summaries.push(metrics(&fixture, label));
    server.terminate().await;
    other.terminate().await;
    assert_eq!(read_metrics(&fixture, label)["transport"]["active"], 0);
    assert_eq!(
        read_metrics(&isolated, "isolated")["transport"]["active"],
        0
    );
    for relay in relays.iter().chain(std::iter::once(&other_relay)) {
        relay.cut();
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        relays.iter().chain(std::iter::once(&other_relay)).all(|r| r
            .counters
            .active
            .load(Ordering::SeqCst)
            == 0)
    );
    rtts.sort_by(f64::total_cmp);
    println!(
        "DEPLOYMENT_SOAK {}",
        serde_json::json!({"requested_seconds":seconds,"elapsed_seconds":started.elapsed().as_secs_f64(),"actions":actions,"incidents":incidents,"rtt_ms":{"p50":rtts[rtts.len()/2],"p95":rtts[(rtts.len()-1)*95/100],"max":rtts.last()},"samples":samples,"rss_generations":memory,"host_generations":generation_summaries,"relays":relays.iter().chain(std::iter::once(&other_relay)).map(|r|serde_json::json!({"connections":r.counters.connections.load(Ordering::Relaxed),"peak_active":r.counters.peak_active.load(Ordering::Relaxed),"active":r.counters.active.load(Ordering::Relaxed),"bytes":r.counters.bytes.load(Ordering::Relaxed),"fragments":r.counters.fragments.load(Ordering::Relaxed)})).collect::<Vec<_>>(),"starter_items":starter.len(),"characters":2,"worlds":2,"restored_to_fresh_path":true,"duplicate_owner_refused":true})
    );
}
