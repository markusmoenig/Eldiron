//! In-process host benchmark: authored rules/map, real players/encoding and
//! optional file commits. Synthetic crowd actors do not model scripted AI.
use eldiron_network::*;
use eldiron_runtime::{MovementCommand, MovementIntent};
use eldiron_server::{
    auth,
    host::{Command, Host},
    recovery,
};
use rusterix::diagnostics::Timing;
use rusterix::{Entity, EntityAction, Value};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use theframework_core::{
    Uuid,
    prelude::{Vec2, Vec3},
};
use tokio::sync::mpsc;
struct Peer {
    id: Uuid,
    rx: mpsc::Receiver<Vec<u8>>,
    generation: u64,
    sequence: u64,
    stream: Option<Uuid>,
    world: Uuid,
    region: Uuid,
    action: u64,
}
fn drain(peer: &mut Peer) -> Result<(), String> {
    while let Ok(bytes) = peer.rx.try_recv() {
        match codec::decode_server(&bytes).map_err(|e| e.to_string())? {
            ServerMessage::Frame { frame, .. } => {
                if peer.generation != frame.generation {
                    peer.generation = frame.generation;
                    peer.sequence = frame
                        .motion
                        .as_ref()
                        .map_or(1, |m| m.processed_sequence + 1);
                }
                peer.world = frame.world;
                peer.region = frame.current_region.unwrap_or_default();
            }
            ServerMessage::ActionStream {
                stream,
                next_sequence,
            } => {
                peer.stream = Some(stream);
                peer.action = next_sequence;
            }
            ServerMessage::Rejected { code } => {
                return Err(format!("Benchmark rejected: {code:?}"));
            }
            ServerMessage::ActionReply(reply) => {
                if !matches!(reply.result, ActionResult::Outcome(_)) {
                    return Err(format!("Benchmark action: {:?}", reply.result));
                }
                peer.action = reply.next_sequence;
            }
            _ => {}
        }
    }
    if peer.rx.is_closed() {
        return Err("Benchmark client disconnected; requested population was not sustained".into());
    }
    Ok(())
}
async fn message(host: &mut Host, peer: Uuid, message: ClientMessage) -> Result<(), String> {
    host.command(Command::Message { peer, message })
        .await
        .map_err(|e| e.to_string())
}
// Hash-map iteration order and dismissed transient dialogues need not survive
// reconstruction. Ownership, actor/item state, region time and allocator must.
fn restored_ownership(
    world: &eldiron_runtime::persistence::WorldRecord,
) -> Result<serde_json::Value, String> {
    use std::collections::BTreeMap;
    fn actors(
        values: &[eldiron_runtime::persistence::SavedEntity],
    ) -> BTreeMap<u32, eldiron_runtime::persistence::SavedEntity> {
        values
            .iter()
            .map(|e| {
                let mut e = e.clone();
                e.sequences.sort_by(|a, b| a.0.cmp(&b.0));
                (e.id, e)
            })
            .collect()
    }
    let regions: BTreeMap<_, _> = world
        .regions
        .iter()
        .map(|r| {
            (
                r.geometry.id,
                serde_json::json!({
                    "entities":actors(&r.entities), "stashed":actors(&r.stashed),
                    "items":r.items.iter().map(|i|(i.id,i)).collect::<BTreeMap<_,_>>(),
                    "ticks":r.ticks, "time":r.time,
                }),
            )
        })
        .collect();
    Ok(serde_json::json!({"next_id":world.next_id,"values":world.values,"regions":regions}))
}
struct Temp(PathBuf);
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("Eldiron benchmark: {error}");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), String> {
    let mut project = None;
    let mut players = 4usize;
    let mut entities = 100usize;
    let mut ticks = 200u32;
    let mut moving = false;
    let mut idle_entities = 0usize;
    let mut pause_idle = false;
    let mut persistent = false;
    let mut verify_restart = false;
    let mut saved_actions = 0u32;
    let mut snapshot_commits = false;
    let mut ungrouped_actions = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => project = Some(PathBuf::from(args.next().ok_or("Missing project")?)),
            "--players" => {
                players = args
                    .next()
                    .ok_or("Missing players")?
                    .parse()
                    .map_err(|_| "Invalid players")?
            }
            "--entities" => {
                entities = args
                    .next()
                    .ok_or("Missing entities")?
                    .parse()
                    .map_err(|_| "Invalid entities")?
            }
            "--ticks" => {
                ticks = args
                    .next()
                    .ok_or("Missing ticks")?
                    .parse()
                    .map_err(|_| "Invalid ticks")?
            }
            "--moving-entities" => moving = true,
            "--idle-entities" => {
                idle_entities = args
                    .next()
                    .ok_or("Missing idle entities")?
                    .parse()
                    .map_err(|_| "Invalid idle entities")?
            }
            "--pause-idle" => pause_idle = true,
            "--persistent" => persistent = true,
            "--verify-restart" => verify_restart = true,
            "--snapshot-commits" => snapshot_commits = true,
            "--ungrouped-actions" => ungrouped_actions = true,
            "--saved-actions" => {
                saved_actions = args
                    .next()
                    .ok_or("Missing saved action count")?
                    .parse()
                    .map_err(|_| "Invalid saved action count")?
            }
            "--help" => {
                println!(
                    "eldiron-server-bench --project game.eldiron [--players 1..64] [--entities 100] [--ticks 200] [--moving-entities] [--idle-entities 1000 [--pause-idle]] [--persistent --verify-restart --saved-actions 2 [--snapshot-commits | --ungrouped-actions]]\nRun an optimized build. This measures an in-process host, not TLS/network capacity or full NPC AI."
                );
                return Ok(());
            }
            _ => return Err(format!("Unknown option {arg}")),
        }
    }
    if !(1..=64).contains(&players)
        || entities > 10_000
        || idle_entities > 10_000
        || pause_idle && idle_entities == 0
        || !(20..=10_000).contains(&ticks)
        || saved_actions > 4
        || saved_actions > 0 && !persistent
        || snapshot_commits && !persistent
        || verify_restart && !persistent
    {
        return Err("Players 1..64, entities <=10000, ticks 20..10000; at most four saved actions per player in persistent mode".into());
    }
    let project_path = project.ok_or("--project is required")?;
    let mut project = recovery::load_project(&project_path)?;
    // Deterministic extra region in both scheduling runs: the policy switch is
    // supplied through the trusted host API, so their content fingerprints match.
    let idle_map = Uuid::from_u128(0x12345678_0000_4000_8000_123456789abc);
    if idle_entities > 0 {
        if project.regions.iter().any(|r| r.map.id == idle_map) {
            return Err("Benchmark map ID collision".into());
        }
        let mut idle = project.regions[0].clone();
        idle.id = idle_map;
        idle.map.id = idle_map;
        idle.name = "Benchmark idle region".into();
        idle.map.name = idle.name.clone();
        project.regions.push(idle);
    }
    let revision = project_content_revision(&project)?;
    let restart_project = verify_restart.then(|| project.clone());
    let temp = Temp(std::env::temp_dir().join(format!("eldiron-bench-{}", Uuid::new_v4())));
    std::fs::create_dir(&temp.0).map_err(|e| e.to_string())?;
    let started = Instant::now();
    let state = persistent.then(|| temp.0.join("world.save"));
    let mut host = if snapshot_commits {
        let store = state
            .map(eldiron_runtime::persistence::FileWorldStore::open)
            .transpose()
            .map_err(|e| e.to_string())?;
        Host::with_store(project, store).await
    } else {
        Host::open(project, state).await
    }
    .map_err(|e| e.to_string())?;
    host.world.game().server.print_log_messages = false;
    let hash = if persistent {
        Some(auth::hash("isolated benchmark password").map_err(|_| "Benchmark hash failed")?)
    } else {
        None
    };
    let mut peers = vec![];
    for n in 0..players {
        let id = Uuid::new_v4();
        let (output, rx) = mpsc::channel(64);
        host.command(Command::Open { peer: id, output })
            .await
            .map_err(|e| e.to_string())?;
        message(
            &mut host,
            id,
            ClientMessage::Hello {
                version: PROTOCOL_VERSION,
            },
        )
        .await?;
        if let Some(hash) = &hash {
            host.command(Command::Register {
                peer: id,
                username: format!("bench_{n}"),
                hash: hash.clone(),
            })
            .await
            .map_err(|e| e.to_string())?;
            message(
                &mut host,
                id,
                ClientMessage::CreateCharacter {
                    request: Uuid::new_v4(),
                    name: format!("Bench {n}"),
                },
            )
            .await?;
        } else {
            message(
                &mut host,
                id,
                ClientMessage::Guest {
                    name: format!("Bench {n}"),
                },
            )
            .await?;
        }
        peers.push(Peer {
            id,
            rx,
            generation: 0,
            sequence: 1,
            stream: None,
            world: Uuid::nil(),
            region: Uuid::nil(),
            action: 1,
        });
        for peer in &mut peers {
            drain(peer)?;
        }
    }
    host.tick(Duration::from_millis(20))
        .await
        .map_err(|e| e.to_string())?;
    for peer in &mut peers {
        drain(peer)?;
    }
    let control = host
        .world
        .game()
        .server
        .player_connections()
        .into_iter()
        .find_map(|(_, state)| state.control())
        .ok_or("No controlled player")?;
    let context = host
        .world
        .game()
        .server
        .instances
        .iter()
        .find_map(|instance| {
            let region = instance.lock().unwrap();
            (region.id == control.region_id)
                .then(|| region.context())
                .flatten()
        })
        .ok_or("Missing controlled region")?;
    let crowd_region = context.lock().unwrap().map.name.clone();
    let mut synthetic = vec![];
    {
        let mut ctx = context.lock().unwrap();
        let center = ctx
            .map
            .entities
            .iter()
            .find(|e| e.id == control.entity_id)
            .map(|e| e.position)
            .unwrap_or_default();
        let side = (entities as f64).sqrt().ceil() as usize;
        for n in 0..entities {
            let mut e = Entity::default();
            e.id = ctx.world.next_id();
            e.position = center
                + Vec3::new(
                    (n % side) as f32 * 0.6 - 3.,
                    0.,
                    (n / side) as f32 * 0.6 - 3.,
                );
            e.orientation = Vec2::new(1., 0.);
            e.set_attribute("name", Value::Str("Synthetic crowd actor".into()));
            e.set_attribute("radius", Value::Float(0.2));
            e.set_attribute("visible", Value::Bool(true));
            e.set_attribute("blocking", Value::Bool(true));
            if moving {
                e.action = EntityAction::Forward;
            }
            synthetic.push((e.id, e.position));
            ctx.map.entities.push(e);
        }
    }
    let idle_context = if idle_entities > 0 {
        let region_id = host.world.game().server.region_id_map[&idle_map];
        let context = host
            .world
            .game()
            .server
            .instances
            .iter()
            .find_map(|r| {
                let r = r.lock().unwrap();
                (r.id == region_id).then(|| r.context()).flatten()
            })
            .ok_or("Missing idle region")?;
        {
            let mut ctx = context.lock().unwrap();
            let side = (idle_entities as f64).sqrt().ceil() as usize;
            for n in 0..idle_entities {
                let mut e = Entity::default();
                e.id = ctx.world.next_id();
                e.position = Vec3::new((n % side) as f32 * 0.6, 0., (n / side) as f32 * 0.6);
                e.orientation = Vec2::new(1., 0.);
                e.set_attribute("radius", Value::Float(0.2));
                e.action = EntityAction::Forward;
                ctx.map.entities.push(e);
            }
        }
        if pause_idle {
            host.world
                .game()
                .server
                .set_region_scheduling(rusterix::server::scheduling::RegionSchedulingPolicy {
                    pause_unoccupied_regions: [idle_map].into(),
                    idle_after: Duration::from_millis(100),
                })
                .map_err(|e| e.to_string())?;
        }
        Some(context)
    } else {
        None
    };
    for _ in 0..20 {
        host.tick(Duration::from_millis(20))
            .await
            .map_err(|e| e.to_string())?;
        for p in &mut peers {
            drain(p)?;
        }
    }
    if host.diagnostics().players != players {
        return Err("Benchmark did not admit all requested players".into());
    }
    let idle_before = idle_context.as_ref().map(|c| {
        let c = c.lock().unwrap();
        (
            c.ticks,
            c.map
                .entities
                .iter()
                .map(|e| (e.id, e.position))
                .collect::<Vec<_>>(),
        )
    });
    let setup_ms = started.elapsed().as_millis();
    host.enable_diagnostics(true);
    let storage_before = host
        .diagnostics()
        .saves
        .map(|s| s.storage)
        .unwrap_or_default();
    let mut slice = Timing::default();
    let measured = Instant::now();
    for tick in 0..ticks {
        let started = Instant::now();
        let mut actions = vec![];
        for p in &mut peers {
            message(
                &mut host,
                p.id,
                ClientMessage::Inputs {
                    frames: vec![MovementCommand {
                        generation: p.generation,
                        sequence: p.sequence,
                        intent: if tick % 100 < 50 {
                            MovementIntent::Forward
                        } else {
                            MovementIntent::Backward
                        },
                        yaw: 0.,
                    }],
                },
            )
            .await?;
            p.sequence += 1;
            if saved_actions > 0
                && tick % (ticks / saved_actions).max(1) == 0
                && tick / (ticks / saved_actions).max(1) < saved_actions
            {
                actions.push(Command::Message {
                    peer: p.id,
                    message: ClientMessage::Action(GameAction {
                        stream: p.stream.ok_or("No action stream")?,
                        sequence: p.action,
                        world: p.world,
                        generation: p.generation,
                        region: p.region,
                        request: GameRequest::Intent("look".into()),
                    }),
                });
            }
        }
        let mut actions = actions.into_iter();
        loop {
            let group: Vec<_> = actions
                .by_ref()
                .take(eldiron_runtime::persistence::MAX_GROUP_ACTIONS)
                .collect();
            if group.is_empty() {
                break;
            }
            if snapshot_commits || ungrouped_actions {
                for command in group {
                    host.command(command).await.map_err(|e| e.to_string())?;
                }
            } else {
                host.command_group(group).await.map_err(|e| e.to_string())?;
            }
        }
        host.tick(Duration::from_millis(20))
            .await
            .map_err(|e| e.to_string())?;
        slice.record(started.elapsed());
        for p in &mut peers {
            drain(p)?;
        }
    }
    let measured_ms = measured.elapsed().as_millis();
    let diagnostics = host.diagnostics();
    let storage_measured = diagnostics.saves.as_ref().map(|s| {
        serde_json::json!({
            "bytes_written":s.storage.bytes_written.saturating_sub(storage_before.bytes_written),
            "appends":s.storage.transactions.saturating_sub(storage_before.transactions),
            "checkpoints":s.storage.checkpoints.saturating_sub(storage_before.checkpoints),
            "groups":s.groups, "actions":s.grouped_actions,
        })
    });
    let timing = slice.summary();
    let moved = {
        let ctx = context.lock().unwrap();
        synthetic
            .iter()
            .filter(|(id, start)| {
                ctx.map
                    .entities
                    .iter()
                    .find(|e| e.id == *id)
                    .is_some_and(|e| e.position != *start)
            })
            .count()
    };
    let players_in_crowd_region = host
        .world
        .game()
        .server
        .player_connections()
        .into_iter()
        .filter(|(_, state)| {
            state
                .control()
                .is_some_and(|c| c.region_id == control.region_id)
        })
        .count();
    let idle_unchanged = idle_context
        .as_ref()
        .zip(idle_before.as_ref())
        .map(|(c, before)| {
            let c = c.lock().unwrap();
            c.ticks == before.0
                && c.map
                    .entities
                    .iter()
                    .map(|e| (e.id, e.position))
                    .collect::<Vec<_>>()
                    == before.1
        });
    if pause_idle && idle_unchanged != Some(true) {
        return Err("Suspended region changed during measurement".into());
    }
    let report = serde_json::json!({"schema":1,"project":project_path.file_name().unwrap_or_default().to_string_lossy(),"content_revision":revision,"build":if cfg!(debug_assertions){"debug"}else{"optimized"},"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"logical_cpus":std::thread::available_parallelism().map_or(1,usize::from),"players":players,"synthetic_entities":entities,"idle_entities":idle_entities,"pause_idle":pause_idle,"idle_unchanged_during_measurement":idle_unchanged,"crowd_region":crowd_region,"players_in_crowd_region_at_end":players_in_crowd_region,"moving_entities_requested":moving,"synthetic_entities_observed_moved":moved,"persistent":persistent,"saved_actions_per_player":saved_actions,"ticks":ticks,"simulated_ms":ticks as u64*20,"setup_ms":setup_ms,"measured_wall_ms":measured_ms,"action_groups_max":if snapshot_commits || ungrouped_actions {1} else {eldiron_runtime::persistence::MAX_GROUP_ACTIONS},"snapshot_commits":snapshot_commits,"ungrouped_actions":ungrouped_actions,"storage_measured":storage_measured,"server_slice":timing,"within_test_budget":diagnostics.players==players&&timing.p95_us<=20_000&&timing.max_us<=250_000&&diagnostics.slow_disconnects==0&&diagnostics.replica_disconnects==0,"scope":"in-process host with real projection/encoding; synthetic crowd, not full scripted AI or TLS/load-generator traffic","diagnostics":diagnostics});
    host.shutdown().await.map_err(|e| e.to_string())?;
    let expected = if verify_restart {
        let eldiron_server::host::World::Saved(session) = &host.world else {
            return Err("Restart requires saved host".into());
        };
        Some(serde_json::to_value(session.saved()).map_err(|e| e.to_string())?)
    } else {
        None
    };
    drop(host);
    if let Some(expected) = expected {
        let restored = Host::open(restart_project.unwrap(), Some(temp.0.join("world.save")))
            .await
            .map_err(|e| e.to_string())?;
        let eldiron_server::host::World::Saved(session) = &restored.world else {
            return Err("Missing saved host".into());
        };
        if serde_json::to_value(session.saved()).map_err(|e| e.to_string())? != expected {
            return Err("Restart changed durable state/characters/items/receipts".into());
        }
        // The live restored engine must also capture the same actors/items and
        // region clocks, rather than merely retaining a correct metadata head.
        let live = session
            .game
            .server
            .capture_durable_world()
            .map_err(|e| e.to_string())?;
        live.validate().map_err(|e| e.to_string())?;
        if restored_ownership(&live)? != restored_ownership(&session.saved().world_state)? {
            return Err(
                "Live restoration changed actor/item ownership, clocks or allocator".into(),
            );
        }
    }
    let mut report = report;
    report["restart_verified"] = serde_json::json!(verify_restart);
    println!(
        "ELDIRON_BENCH {}",
        serde_json::to_string(&report).map_err(|e| e.to_string())?
    );
    Ok(())
}
