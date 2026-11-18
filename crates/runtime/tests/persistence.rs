#![cfg(not(target_arch = "wasm32"))]

use eldiron_runtime::{GameSession, PlayerStartOptions, Project, SessionOptions, persistence::*};
use pollster::block_on;
use rusterix::{Entity, EntityAction, Item, Value};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    },
    time::Duration,
};
use theframework_core::{
    Uuid,
    prelude::{Vec2, Vec3},
};

fn starter() -> Project {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test_projects/3DStarter.eldiron");
    let mut project = shared::project_io::decode_project(&std::fs::read(path).unwrap()).unwrap();
    project.migrate_default_ruleset();
    let mut next = project.regions[0].clone();
    next.id = Uuid::new_v4();
    next.map.id = Uuid::new_v4();
    next.name = "Saved destination".into();
    next.map.name = next.name.clone();
    project.regions.push(next);
    project
}
fn count_items(world: &WorldRecord, id: u32) -> usize {
    fn count(item: &SavedItem, id: u32) -> usize {
        usize::from(item.id == id)
            + item
                .contents
                .as_ref()
                .map(|items| items.iter().map(|i| count(i, id)).sum::<usize>())
                .unwrap_or(0)
    }
    world
        .regions
        .iter()
        .map(|r| {
            r.items.iter().map(|i| count(i, id)).sum::<usize>()
                + r.entities
                    .iter()
                    .chain(&r.stashed)
                    .map(|e| {
                        e.inventory
                            .iter()
                            .flatten()
                            .chain(e.equipped.iter().map(|(_, i)| i))
                            .map(|i| count(i, id))
                            .sum::<usize>()
                    })
                    .sum::<usize>()
        })
        .sum()
}
fn empty_save() -> WorldSave {
    WorldSave {
        version: SAVE_VERSION,
        world: Uuid::new_v4(),
        purpose: SavePurpose::SinglePlayer,
        content: "a".repeat(64),
        revision: 1,
        characters: vec![],
        local_character: None,
        requests: Default::default(),
        host_data: Default::default(),
        action_streams: Default::default(),
        world_state: WorldRecord {
            next_id: 1,
            values: Default::default(),
            regions: vec![],
            render: Default::default(),
            region_render: vec![],
        },
        clock: rusterix::simulation::SimulationClock::new(Duration::from_millis(250)).save(),
        tick: 0,
        movement_tick: 0,
    }
}
#[test]
fn stores_validate_checksums_schema_compare_and_swap_and_native_exclusive_lock() {
    block_on(async {
        let saved = empty_save();
        let bytes = saved.encode().unwrap();
        assert_eq!(WorldSave::decode(&bytes).unwrap().world, saved.world);
        let mut bad = bytes.clone();
        let i = bad.len() / 2;
        bad[i] ^= 1;
        assert!(WorldSave::decode(&bad).is_err());
        let mut store = MemoryWorldStore::default();
        store.commit(None, &saved).await.unwrap();
        assert_eq!(store.commit(None, &saved).await, Err(SaveError::Conflict));
        let mut changed = saved.clone();
        changed.revision = 2;
        changed.world = Uuid::new_v4();
        assert_eq!(
            store.commit(Some(1), &changed).await,
            Err(SaveError::Conflict)
        );
        changed = saved.clone();
        changed.version += 1;
        assert!(matches!(changed.encode(), Err(SaveError::Incompatible(_))));
        let dir = std::env::temp_dir().join(format!("eldiron-save-{}", Uuid::new_v4()));
        let path = dir.join("slot.worldsave");
        let mut disk = FileWorldStore::open(&path).unwrap();
        assert!(matches!(FileWorldStore::open(&path), Err(SaveError::Busy)));
        disk.commit(None, &saved).await.unwrap();
        let mut second = saved.clone();
        second.revision = 2;
        disk.commit(Some(1), &second).await.unwrap();
        // An interrupted pre-rename write is not a recovery head.
        std::fs::write(dir.join(".orphan.tmp"), b"partial").unwrap();
        drop(disk);
        let mut disk = FileWorldStore::open(&path).unwrap();
        assert_eq!(disk.load().await.unwrap().unwrap().revision, 2);
        drop(disk);
        assert!(FileWorldStore::open(dir.join("project.eldiron")).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    });
}

#[test]
fn durable_restart_restores_inventory_world_and_location_without_starter_grants() {
    block_on(async {
        let project = starter();
        let authored = content_revision(&project).unwrap();
        let dir = std::env::temp_dir().join(format!("eldiron-restore-{}", Uuid::new_v4()));
        let path = dir.join("solo.worldsave");
        let mut session = DurableSession::open(
            project.clone(),
            FileWorldStore::open(&path).unwrap(),
            SavePurpose::SinglePlayer,
            SessionOptions::default(),
        )
        .await
        .unwrap();
        session.game.server.print_log_messages = false;
        if !session.game.has_started_player() {
            session
                .game
                .start_player(PlayerStartOptions::default())
                .unwrap();
        }
        for _ in 0..3 {
            session.game.tick_once();
        }
        let connection = session.game.server.local_connection();
        let control = session.game.server.player_control(connection).unwrap();
        let context = session
            .game
            .server
            .instances
            .iter()
            .find(|i| i.lock().unwrap().id == control.region_id)
            .unwrap()
            .lock()
            .unwrap()
            .context()
            .unwrap();
        let (bag_id, nested_id, world_item_id, npc_id);
        {
            let mut ctx = context.lock().unwrap();
            bag_id = ctx.world.next_id();
            nested_id = ctx.world.next_id();
            world_item_id = ctx.world.next_id();
            npc_id = ctx.world.next_id();
            let mut nested = Item::default();
            nested.id = nested_id;
            nested.item_type = "Saved gem".into();
            nested.set_attribute("custom", Value::Int(71));
            let mut bag = Item::default();
            bag.id = bag_id;
            bag.item_type = "Saved bag".into();
            bag.container = Some(vec![nested]);
            let actor = ctx.get_entity_mut(control.entity_id).unwrap();
            actor.inventory.push(Some(bag));
            actor.wallet.balances.insert("Saved gold".into(), 1234);
            actor.set_attribute("saved_xp", Value::Int(456));
            let mut item = Item::default();
            item.id = world_item_id;
            item.item_type = "Dropped marker".into();
            ctx.map.items.push(item);
            let mut npc = Entity::default();
            npc.id = npc_id;
            npc.set_attribute("mode", Value::Str("dead".into()));
            ctx.stashed_entities.insert(npc_id, npc.clone());
            ctx.entity_respawn_snapshots.insert(npc_id, npc);
            ctx.entity_state_data
                .entry(control.entity_id)
                .or_default()
                .set("__condition_remaining:sample", Value::Int(9));
            let deadline = ctx.ticks + 100;
            ctx.notifications_entities
                .push((control.entity_id, deadline, "saved_timer".into()));
            ctx.set_world_value("saved_world", Value::Int(88));
            ctx.set_region_value("saved_region", Value::Int(99));
            let mut prop = rusterix::BlockPropInstance::new(Uuid::new_v4());
            prop.runtime_state.set("open", Value::Bool(true));
            ctx.map.block_prop_instances.push(prop);
        }
        session
            .game
            .server
            .player_teleport(connection, "entrance".into(), "Saved destination".into())
            .unwrap();
        for _ in 0..4 {
            session.game.poll();
        }
        session
            .game
            .server
            .player_teleport_pos(connection, Vec2::new(7.5, 8.5))
            .unwrap();
        session.game.poll();
        session.checkpoint().await.unwrap();
        let before = session.saved().clone();
        let player = before
            .characters
            .iter()
            .find(|c| c.entity == control.entity_id)
            .unwrap()
            .clone();
        assert_eq!(count_items(&before.world_state, bag_id), 1);
        assert_eq!(count_items(&before.world_state, nested_id), 1);
        let old_connection = connection;
        drop(session);
        let mut restored = DurableSession::open(
            project.clone(),
            FileWorldStore::open(&path).unwrap(),
            SavePurpose::SinglePlayer,
            SessionOptions::default(),
        )
        .await
        .unwrap();
        assert_ne!(restored.game.server.local_connection(), old_connection);
        assert!(
            restored
                .game
                .server
                .connection_state(old_connection)
                .is_none()
        );
        assert!(
            restored
                .game
                .start_player(PlayerStartOptions::default())
                .unwrap()
                .is_none()
        );
        assert_eq!(restored.game.current_map, "Saved destination");
        let actor = restored.game.view().player_entity().unwrap().clone();
        assert_eq!(actor.position.x, 7.5);
        assert_eq!(actor.position.z, 8.5);
        assert_eq!(actor.wallet.balances["Saved gold"], 1234);
        assert_eq!(actor.attributes.get_int_default("saved_xp", 0), 456);
        assert!(matches!(actor.action, EntityAction::Off));
        restored.checkpoint().await.unwrap();
        let after = restored.saved();
        assert_eq!(after.characters.len(), before.characters.len());
        assert_eq!(after.local_character, Some(player.id));
        assert_eq!(count_items(&after.world_state, bag_id), 1);
        assert_eq!(count_items(&after.world_state, nested_id), 1);
        assert_eq!(count_items(&after.world_state, world_item_id), 1);
        assert_eq!(
            after.world_state.next_id, before.world_state.next_id,
            "no startup items or duplicate authored NPCs allocated on restore"
        );
        assert_eq!(
            serde_json::to_value(&after.world_state).unwrap(),
            serde_json::to_value(&before.world_state).unwrap()
        );
        assert_eq!(after.tick, before.tick, "offline time is paused");
        let mut wrong = project.clone();
        wrong.config.push_str("\n# incompatible edit\n");
        let bytes = after.encode().unwrap();
        let changed = DurableSession::open(
            wrong,
            MemoryWorldStore::from_bytes(bytes.clone()).unwrap(),
            SavePurpose::SinglePlayer,
            SessionOptions::default(),
        )
        .await;
        assert!(matches!(changed, Err(SaveError::Incompatible(_))));
        let wrong_slot = DurableSession::open(
            project.clone(),
            MemoryWorldStore::from_bytes(bytes).unwrap(),
            SavePurpose::CreatorTest("test".into()),
            SessionOptions::default(),
        )
        .await;
        assert!(matches!(wrong_slot, Err(SaveError::Incompatible(_))));
        assert_eq!(
            content_revision(&project).unwrap(),
            authored,
            "authored input remains independent"
        );
        drop(restored);
        std::fs::remove_dir_all(dir).unwrap();
    });
}

struct FaultStore {
    inner: MemoryWorldStore,
    fault: Arc<AtomicU8>,
}
impl WorldStore for FaultStore {
    async fn load(&mut self) -> Result<Option<WorldSave>, SaveError> {
        self.inner.load().await
    }
    async fn commit(&mut self, expected: Option<u64>, saved: &WorldSave) -> Result<(), SaveError> {
        let mode = self.fault.load(Ordering::SeqCst);
        if mode == 1 {
            return Err(SaveError::Store("injected before commit".into()));
        }
        self.inner.commit(expected, saved).await?;
        if mode == 2 {
            return Err(SaveError::Store(
                "injected after commit, before reply".into(),
            ));
        }
        Ok(())
    }
}
#[test]
fn durable_operations_recover_atomic_item_and_region_transfers_and_deduplicate_retries() {
    block_on(async {
        let mut project = Project::new();
        project.regions[0].name = "Persistence test".into();
        project.regions[0].map.name = "Persistence test".into();
        project.config = "[game]\nstart_region='Persistence test'\nauto_create_player=false".into();
        let mut destination = project.regions[0].clone();
        destination.id = Uuid::new_v4();
        destination.map.id = Uuid::new_v4();
        destination.name = "Other".into();
        destination.map.name = "Other".into();
        project.regions.push(destination);
        let fault = Arc::new(AtomicU8::new(0));
        let mut session = DurableSession::open(
            project.clone(),
            FaultStore {
                inner: Default::default(),
                fault: fault.clone(),
            },
            SavePurpose::CreatorTest("isolated".into()),
            SessionOptions::default(),
        )
        .await
        .unwrap();
        session.game.server.print_log_messages = false;
        let context = session.game.server.instances[0]
            .lock()
            .unwrap()
            .context()
            .unwrap();
        let (actor_id, other_id, item_id);
        {
            let mut ctx = context.lock().unwrap();
            actor_id = ctx.world.next_id();
            other_id = ctx.world.next_id();
            item_id = ctx.world.next_id();
            let mut item = Item::default();
            item.id = item_id;
            item.item_type = "Unique".into();
            let mut actor = Entity::default();
            actor.id = actor_id;
            actor.position = Vec3::new(0.5, 0., 0.5);
            actor.set_attribute("player", Value::Bool(true));
            actor.inventory = vec![Some(item)];
            actor.wallet.balances.insert("G".into(), 20);
            let mut other = Entity::default();
            other.id = other_id;
            other.position = Vec3::new(10., 0., 10.);
            other.inventory = vec![None];
            other.wallet.balances.insert("G".into(), 3);
            ctx.map.entities.extend([actor, other]);
        }
        session
            .game
            .server
            .attach_saved_character(
                session.game.server.local_connection(),
                project.regions[0].map.id,
                actor_id,
            )
            .unwrap();
        session.checkpoint().await.unwrap();
        let character = session.saved().local_character.unwrap();
        let before = session.saved().clone();
        let duplicate = Uuid::new_v4();
        let rejected = session
            .commit_operation(duplicate, "copy-item", |s| {
                let r = &mut s.world_state.regions[0];
                let item = r
                    .entities
                    .iter()
                    .find(|e| e.id == actor_id)
                    .unwrap()
                    .inventory[0]
                    .clone();
                r.entities
                    .iter_mut()
                    .find(|e| e.id == other_id)
                    .unwrap()
                    .inventory[0] = item;
                Ok(serde_json::json!("bad"))
            })
            .await;
        assert!(matches!(rejected, Err(SaveError::Invalid(_))));
        assert_eq!(session.saved().revision, before.revision);
        let transfer = Uuid::new_v4();
        fn move_item(
            s: &mut WorldSave,
            from: u32,
            to: u32,
        ) -> Result<serde_json::Value, SaveError> {
            let region = s
                .world_state
                .regions
                .iter_mut()
                .find(|r| r.entities.iter().any(|e| e.id == from))
                .unwrap();
            let item = region
                .entities
                .iter_mut()
                .find(|e| e.id == from)
                .unwrap()
                .inventory[0]
                .take()
                .unwrap();
            let id = item.id;
            region
                .entities
                .iter_mut()
                .find(|e| e.id == to)
                .unwrap()
                .inventory[0] = Some(item);
            *region
                .entities
                .iter_mut()
                .find(|e| e.id == from)
                .unwrap()
                .wallet
                .balances
                .get_mut("G")
                .unwrap() -= 2;
            *region
                .entities
                .iter_mut()
                .find(|e| e.id == to)
                .unwrap()
                .wallet
                .balances
                .get_mut("G")
                .unwrap() += 2;
            Ok(serde_json::json!({"item":id}))
        }
        fault.store(1, Ordering::SeqCst);
        assert!(
            session
                .commit_operation(transfer, "transfer-item", |s| move_item(
                    s, actor_id, other_id
                ))
                .await
                .is_err()
        );
        assert_eq!(
            count_items(
                &session.game.server.capture_durable_world().unwrap(),
                item_id
            ),
            1
        );
        assert_eq!(session.saved().revision, before.revision);
        assert_eq!(session.checkpoint().await, Err(SaveError::NeedsReload));
        let recovered = session.store().inner.bytes().unwrap().to_vec();
        drop(session);
        fault.store(0, Ordering::SeqCst);
        let mut session = DurableSession::open(
            project.clone(),
            FaultStore {
                inner: MemoryWorldStore::from_bytes(recovered).unwrap(),
                fault: fault.clone(),
            },
            SavePurpose::CreatorTest("isolated".into()),
            SessionOptions::default(),
        )
        .await
        .unwrap();
        let outcome = session
            .commit_operation(transfer, "transfer-item", |s| {
                move_item(s, actor_id, other_id)
            })
            .await
            .unwrap();
        assert_eq!(
            count_items(
                &session.game.server.capture_durable_world().unwrap(),
                item_id
            ),
            1
        );
        assert_eq!(
            session
                .commit_operation(transfer, "transfer-item", |_| panic!(
                    "committed retry executed twice"
                ))
                .await
                .unwrap(),
            outcome
        );
        assert_eq!(
            session
                .commit_operation(transfer, "changed-quantity", |_| panic!(
                    "conflicting retry executed"
                ))
                .await,
            Err(SaveError::Conflict)
        );
        let region_transfer = Uuid::new_v4();
        let target = project.regions[1].map.id;
        fault.store(2, Ordering::SeqCst);
        assert!(
            session
                .commit_operation(region_transfer, "transfer-region", |s| {
                    let from = s
                        .world_state
                        .regions
                        .iter()
                        .position(|r| r.entities.iter().any(|e| e.id == actor_id))
                        .unwrap();
                    let index = s.world_state.regions[from]
                        .entities
                        .iter()
                        .position(|e| e.id == actor_id)
                        .unwrap();
                    let mut actor = s.world_state.regions[from].entities.remove(index);
                    actor.position.x = 25.5;
                    s.world_state
                        .regions
                        .iter_mut()
                        .find(|r| r.geometry.id == target)
                        .unwrap()
                        .entities
                        .push(actor);
                    s.characters
                        .iter_mut()
                        .find(|c| c.id == character)
                        .unwrap()
                        .region = target;
                    Ok(serde_json::json!({"region":target}))
                })
                .await
                .is_err()
        );
        assert_eq!(
            session.game.view().player_entity().unwrap().position.x,
            0.5,
            "unacknowledged commit never published a speculative live result"
        );
        let recovered = session.store().inner.bytes().unwrap().to_vec();
        drop(session);
        let mut session = DurableSession::open(
            project.clone(),
            MemoryWorldStore::from_bytes(recovered).unwrap(),
            SavePurpose::CreatorTest("isolated".into()),
            SessionOptions::default(),
        )
        .await
        .unwrap();
        assert_eq!(session.game.current_map, "Other");
        assert_eq!(
            session.game.view().player_entity().unwrap().position.x,
            25.5
        );
        assert_eq!(
            session
                .saved()
                .characters
                .iter()
                .find(|c| c.id == character)
                .unwrap()
                .region,
            target
        );
        session
            .commit_operation(region_transfer, "transfer-region", |_| {
                panic!("region transfer repeated after lost reply")
            })
            .await
            .unwrap();
        session
            .commit_operation(transfer, "transfer-item", |_| {
                panic!("inventory transfer repeated after restart")
            })
            .await
            .unwrap();
        assert_eq!(
            count_items(
                &session.game.server.capture_durable_world().unwrap(),
                item_id
            ),
            1
        );
        let other = session.game.open_player_connection().unwrap();
        assert!(
            session.join_character(other, character).is_err(),
            "one body cannot be controlled twice"
        );
        let local = session.game.server.local_connection();
        session.game.close_player_connection(local);
        session.join_character(other, character).unwrap();
        assert_eq!(
            session.game.server.player_control(other).unwrap().entity_id,
            actor_id
        );
        let revision = session.saved().revision;
        session.set_checkpoint_interval(Duration::from_millis(10));
        session.advance(Duration::from_millis(9)).await.unwrap();
        assert_eq!(session.saved().revision, revision);
        session.advance(Duration::from_millis(1)).await.unwrap();
        assert_eq!(session.saved().revision, revision + 1);
        // Normal temporary Creator play creates no store and changes no saved slot.
        let saved = session.store().bytes().unwrap().to_vec();
        let mut temporary = GameSession::new(project);
        temporary.start(SessionOptions::default()).unwrap();
        temporary.tick_once();
        temporary.stop();
        assert_eq!(session.store().bytes().unwrap(), saved);
        session.shutdown().await.unwrap();
        assert!(!session.game.is_started());
        let bytes = session.store().bytes().unwrap().to_vec();
        session.shutdown().await.unwrap();
        assert!(session.checkpoint().await.is_err());
        assert_eq!(session.store().bytes().unwrap(), bytes);
        session.game.start(SessionOptions::default()).unwrap();
        assert_eq!(session.checkpoint().await, Err(SaveError::NeedsReload));
        assert_eq!(
            session.store().bytes().unwrap(),
            bytes,
            "a newly started temporary incarnation cannot overwrite the saved world"
        );
    });
}

#[test]
fn content_fingerprint_is_stable_and_includes_authored_configuration() {
    let mut project = Project::new();
    let first = content_revision(&project).unwrap();
    assert_eq!(first, content_revision(&project.clone()).unwrap());
    project.config.push_str("\n# edited content\n");
    assert_ne!(first, content_revision(&project).unwrap());
}

#[test]
fn live_host_operations_fail_closed_and_retry_after_ambiguous_commit() {
    block_on(async {
        let mut project = Project::new();
        project.regions[0].name = "Live test".into();
        project.regions[0].map.name = "Live test".into();
        project.config = "[game]\nstart_region='Live test'\nauto_create_player=false".into();
        for mode in [1, 2] {
            let fault = Arc::new(AtomicU8::new(0));
            let mut session = DurableSession::open(
                project.clone(),
                FaultStore {
                    inner: Default::default(),
                    fault: fault.clone(),
                },
                SavePurpose::Dedicated,
                SessionOptions {
                    create_local_player: false,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            session.enable_diagnostics(true);
            let request = Uuid::new_v4();
            fault.store(mode, Ordering::SeqCst);
            let result = session
                .commit_live_operation(
                    request,
                    "live:test",
                    |game| {
                        let ctx = game.server.instances[0].lock().unwrap().context().unwrap();
                        let mut ctx = ctx.lock().unwrap();
                        let mut item = Item::default();
                        item.id = ctx.world.next_id();
                        item.attributes
                            .set("name", Value::Str("Committed once".into()));
                        ctx.map.items.push(item);
                        Ok(serde_json::json!("created"))
                    },
                    |saved, value| {
                        saved.host_data.insert("test".into(), value.clone());
                        Ok(value)
                    },
                )
                .await;
            assert!(matches!(result, Err(SaveError::Store(_))));
            assert_eq!(session.checkpoint().await, Err(SaveError::NeedsReload));
            let metrics = session.diagnostics().unwrap();
            assert!(metrics.poisoned);
            assert_eq!(metrics.failures, 1);
            assert_eq!(metrics.commit.calls, 1);
            assert!(metrics.capture.calls > 0);
            let store = session.store().inner.clone();
            drop(session);
            let mut reopened = DurableSession::open(
                project.clone(),
                store,
                SavePurpose::Dedicated,
                SessionOptions {
                    create_local_player: false,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
            assert_eq!(reopened.saved().requests.contains_key(&request), mode == 2);
            assert_eq!(
                reopened.saved().world_state.regions[0].items.len(),
                usize::from(mode == 2)
            );
            if mode == 2 {
                assert_eq!(
                    reopened
                        .commit_live_operation(
                            request,
                            "live:test",
                            |_| panic!("receipt must suppress callback"),
                            |_, _| panic!()
                        )
                        .await
                        .unwrap(),
                    serde_json::json!("created")
                );
            }
        }
    });
}

#[test]
fn character_action_effects_and_receipts_restore_together_after_failed_or_lost_ack() {
    block_on(async {
        let mut project = Project::new();
        project.regions[0].name = "Action test".into();
        project.regions[0].map.name = "Action test".into();
        project.config = "[game]\nstart_region='Action test'\nauto_create_player=false".into();
        for mode in [1, 2] {
            let fault = Arc::new(AtomicU8::new(0));
            let mut session = DurableSession::open(
                project.clone(),
                FaultStore {
                    inner: Default::default(),
                    fault: fault.clone(),
                },
                SavePurpose::SinglePlayer,
                SessionOptions::default(),
            )
            .await
            .unwrap();
            let ctx = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let actor_id;
            {
                let mut ctx = ctx.lock().unwrap();
                let mut actor = Entity::default();
                actor_id = ctx.world.next_id();
                actor.id = actor_id;
                actor.set_attribute("player", Value::Bool(true));
                actor.inventory = vec![None];
                actor.wallet.balances.insert("c".into(), 20);
                ctx.map.entities.push(actor);
            }
            session
                .game
                .server
                .attach_saved_character(
                    session.game.server.local_connection(),
                    project.regions[0].map.id,
                    actor_id,
                )
                .unwrap();
            session.checkpoint().await.unwrap();
            let stream = session.saved().local_character.unwrap();
            fault.store(mode, Ordering::SeqCst);
            let fingerprint = "a".repeat(64);
            let apply = |game: &mut GameSession| {
                let ctx = game.server.instances[0].lock().unwrap().context().unwrap();
                let mut ctx = ctx.lock().unwrap();
                let mut grant = Item::default();
                grant.id = ctx.world.next_id();
                let actor = ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == actor_id)
                    .unwrap();
                actor.inventory[0] = Some(grant);
                actor.wallet.balances.insert("c".into(), 15);
                Ok(serde_json::json!("purchased"))
            };
            assert!(matches!(
                session
                    .commit_sequenced_live(stream, 1, &fingerprint, apply)
                    .await,
                Err(SaveError::Store(_))
            ));
            assert_eq!(session.checkpoint().await, Err(SaveError::NeedsReload));
            let store = session.store().inner.clone();
            drop(session);
            let mut restored = DurableSession::open(
                project.clone(),
                store,
                SavePurpose::SinglePlayer,
                SessionOptions::default(),
            )
            .await
            .unwrap();
            let actor = restored.saved().world_state.regions[0]
                .entities
                .iter()
                .find(|e| e.id == actor_id)
                .unwrap();
            assert_eq!(actor.inventory[0].is_some(), mode == 2);
            assert_eq!(actor.wallet.balances["c"], if mode == 2 { 15 } else { 20 });
            if mode == 2 {
                assert_eq!(
                    restored
                        .commit_sequenced_live(stream, 1, &fingerprint, |_| panic!(
                            "retry must never grant/pay again"
                        ))
                        .await
                        .unwrap(),
                    serde_json::json!("purchased")
                );
                assert_eq!(
                    restored
                        .commit_sequenced_live(stream, 1, &"b".repeat(64), |_| panic!())
                        .await,
                    Err(SaveError::Conflict)
                );
                assert_eq!(
                    restored
                        .commit_sequenced_live(stream, 3, &fingerprint, |_| panic!())
                        .await,
                    Err(SaveError::ActionSequence { next: 2 })
                );
            } else {
                restored
                    .commit_sequenced_live(stream, 1, &fingerprint, apply)
                    .await
                    .unwrap();
            }
            // Compaction must survive an actual save/reopen, including old
            // receipts which no longer fit in the bounded recent window.
            for sequence in 2..=131 {
                restored
                    .commit_sequenced_live(stream, sequence, &fingerprint, |_| {
                        Ok(serde_json::json!("denied"))
                    })
                    .await
                    .unwrap();
            }
            let bytes = restored.store().bytes().unwrap().to_vec();
            drop(restored);
            let mut restored = DurableSession::open(
                project.clone(),
                MemoryWorldStore::from_bytes(bytes).unwrap(),
                SavePurpose::SinglePlayer,
                SessionOptions::default(),
            )
            .await
            .unwrap();
            assert_eq!(
                restored.saved().action_streams[&stream].recent.len(),
                MAX_ACTION_RECEIPTS
            );
            assert_eq!(
                restored
                    .commit_sequenced_live(stream, 1, &fingerprint, |_| panic!(
                        "compacted action must stay spent"
                    ))
                    .await,
                Err(SaveError::AlreadyCommitted)
            );
            let reset = restored
                .commit_operation(Uuid::new_v4(), "reset-watermark", |saved| {
                    saved.action_streams.clear();
                    Ok(serde_json::Value::Null)
                })
                .await;
            assert!(matches!(reset, Err(SaveError::Invalid(_))));
            let actor = restored.saved().world_state.regions[0]
                .entities
                .iter()
                .find(|e| e.id == actor_id)
                .unwrap();
            assert!(actor.inventory[0].is_some());
            assert_eq!(actor.wallet.balances["c"], 15);
        }
    });
}

#[test]
fn recovery_artifacts_are_verified_immutable_and_separate_from_working_saves() {
    let dir = std::env::temp_dir().join(format!("eldiron-archive-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    let mut saved = empty_save();
    saved
        .host_data
        .insert("private".into(), serde_json::json!({"account":"keep me"}));
    let snapshot = dir.join("backup.snapshot");
    write_snapshot_new(&snapshot, &saved).unwrap();
    assert!(FileWorldStore::open(&snapshot).is_err());
    assert!(write_snapshot_new(&snapshot, &saved).is_err());
    assert_eq!(
        read_snapshot(&snapshot).unwrap().encode().unwrap(),
        saved.encode().unwrap()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&snapshot).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let target = dir.join("restored.save");
    let owner = FileWorldStore::open(&target).unwrap();
    assert!(matches!(
        write_snapshot_new(&target, &saved),
        Err(SaveError::Busy)
    ));
    drop(owner);
    write_snapshot_new(&target, &saved).unwrap();
    assert!(write_snapshot_new(&target, &saved).is_err());
    let mut bytes = std::fs::read(&snapshot).unwrap();
    let n = bytes.len() / 2;
    bytes[n] ^= 1;
    std::fs::write(&snapshot, bytes).unwrap();
    assert!(read_snapshot(&snapshot).is_err());
    assert_eq!(read_snapshot(&target).unwrap().world, saved.world);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn prepared_commits_keep_cas_and_detect_external_replacement_or_corruption() {
    block_on(async {
        let dir = std::env::temp_dir().join(format!("eldiron-prepared-{}", Uuid::new_v4()));
        let path = dir.join("world.save");
        let mut store = FileWorldStore::open(&path).unwrap();
        let mut image = empty_save();
        let prepared = PreparedSave::new(&image).unwrap();
        store.commit_prepared(None, &prepared).await.unwrap();
        assert_eq!(
            WorldSave::decode(prepared.bytes()).unwrap().world,
            prepared.image().world
        );
        image.revision = 2;
        store
            .commit_prepared(Some(1), &PreparedSave::new(&image).unwrap())
            .await
            .unwrap();
        let mut external = image.clone();
        external.revision = 3;
        let replacement = dir.join("replacement");
        std::fs::write(&replacement, external.encode().unwrap()).unwrap();
        std::fs::rename(replacement, &path).unwrap();
        assert_eq!(
            store
                .commit_prepared(Some(2), &PreparedSave::new(&external).unwrap())
                .await,
            Err(SaveError::Conflict)
        );
        assert_eq!(store.load().await.unwrap().unwrap().revision, 3);
        image.revision = 4;
        store
            .commit_prepared(Some(3), &PreparedSave::new(&image).unwrap())
            .await
            .unwrap();
        let mut corrupt = std::fs::read(&path).unwrap();
        let n = corrupt.len() / 2;
        corrupt[n] ^= 1;
        std::fs::write(&path, corrupt).unwrap();
        image.revision = 5;
        assert!(
            store
                .commit_prepared(Some(4), &PreparedSave::new(&image).unwrap())
                .await
                .is_err()
        );
        assert!(store.load().await.is_err());
        let mut invalid = empty_save();
        invalid
            .world_state
            .values
            .set("bad", Value::Float(f32::NAN));
        assert!(PreparedSave::new(&invalid).is_err());
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    });
}

#[test]
fn legacy_containers_upgrade_without_changing_world_or_retry_metadata() {
    use sha2::{Digest, Sha256};
    let mut image = empty_save();
    image.host_data.insert(
        "escape".into(),
        serde_json::json!("quotes \" newlines\n unicode 日本語"),
    );
    image.requests.insert(
        Uuid::new_v4(),
        CommittedRequest {
            fingerprint: "old:create".into(),
            outcome: serde_json::json!({"character":"retained"}),
        },
    );
    let stream = Uuid::new_v4();
    let fingerprint = "a".repeat(64);
    let mut journal = ActionJournal::default();
    journal
        .append(1, fingerprint.clone(), serde_json::json!("processed"))
        .unwrap();
    image.action_streams.insert(stream, journal);
    let payload = serde_json::to_string(&image).unwrap();
    let old=serde_json::to_vec(&serde_json::json!({"checksum":format!("{:x}",Sha256::digest(payload.as_bytes())),"payload":payload})).unwrap();
    let loaded = WorldSave::decode(&old).unwrap();
    assert_eq!(loaded.world, image.world);
    assert_eq!(loaded.revision, image.revision);
    assert_eq!(loaded.host_data, image.host_data);
    assert!(loaded.requests == image.requests);
    assert!(loaded.action_streams == image.action_streams);
    let encoded = loaded.encode().unwrap();
    assert!(encoded.len() < old.len());
    let mut envelope: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(envelope["format"], 2);
    assert!(envelope["payload"].is_object());
    let reopened = WorldSave::decode(&encoded).unwrap();
    assert_eq!(reopened.world, loaded.world);
    assert_eq!(reopened.host_data, loaded.host_data);
    assert_eq!(
        reopened.action_streams[&stream]
            .check(1, &fingerprint)
            .unwrap(),
        Some(serde_json::json!("processed"))
    );
    envelope["format"] = serde_json::json!(999);
    assert!(matches!(
        WorldSave::decode(&serde_json::to_vec(&envelope).unwrap()),
        Err(SaveError::Incompatible(_))
    ));
    envelope["format"] = serde_json::json!(2);
    envelope["payload"]["revision"] = serde_json::json!(999);
    assert!(WorldSave::decode(&serde_json::to_vec(&envelope).unwrap()).is_err());
}

#[test]
fn sequenced_groups_commit_once_and_recover_effects_with_each_receipt() {
    block_on(async {
        let mut project = Project::new();
        project.config = "[game]\nstart_region='Region'\nauto_create_player=false".into();
        project.regions[0].name = "Region".into();
        project.regions[0].map.name = "Region".into();
        for mode in [0, 1, 2] {
            let fault = Arc::new(AtomicU8::new(0));
            let mut session = DurableSession::open(
                project.clone(),
                FaultStore {
                    inner: Default::default(),
                    fault: fault.clone(),
                },
                SavePurpose::SinglePlayer,
                SessionOptions::default(),
            )
            .await
            .unwrap();
            let ctx = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let actor_id;
            {
                let mut ctx = ctx.lock().unwrap();
                let mut actor = Entity::default();
                actor_id = ctx.world.next_id();
                actor.id = actor_id;
                actor.set_attribute("player", Value::Bool(true));
                ctx.map.entities.push(actor);
            }
            session
                .game
                .server
                .attach_saved_character(
                    session.game.server.local_connection(),
                    project.regions[0].map.id,
                    actor_id,
                )
                .unwrap();
            session.checkpoint().await.unwrap();
            let stream = session.saved().local_character.unwrap();
            let revision = session.saved().revision;
            session.begin_sequenced_group().unwrap();
            assert_eq!(
                session.advance(Duration::from_millis(1)).await.unwrap_err(),
                SaveError::Busy
            );
            assert_eq!(session.checkpoint().await.unwrap_err(), SaveError::Busy);
            let fingerprint = "c".repeat(64);
            for sequence in 1..=MAX_GROUP_ACTIONS as u64 {
                session
                    .commit_sequenced_live(stream, sequence, &fingerprint, |game| {
                        let ctx = game.server.instances[0].lock().unwrap().context().unwrap();
                        let mut ctx = ctx.lock().unwrap();
                        let mut grant = Item::default();
                        grant.id = ctx.world.next_id();
                        ctx.map
                            .entities
                            .iter_mut()
                            .find(|e| e.id == actor_id)
                            .unwrap()
                            .inventory
                            .push(Some(grant));
                        Ok(serde_json::json!(sequence))
                    })
                    .await
                    .unwrap();
                assert_eq!(session.saved().revision, revision);
                assert!(!session.saved().action_streams.contains_key(&stream));
                assert_eq!(session.action_journal(stream).unwrap().last, sequence);
                assert_eq!(
                    session
                        .commit_sequenced_live(stream, sequence, &fingerprint, |_| panic!(
                            "staged retry cannot grant twice"
                        ))
                        .await
                        .unwrap(),
                    serde_json::json!(sequence)
                );
            }
            assert_eq!(
                session
                    .commit_sequenced_live(stream, 9, &fingerprint, |_| panic!())
                    .await,
                Err(SaveError::Busy)
            );
            fault.store(mode, Ordering::SeqCst);
            let result = session.finish_sequenced_group().await;
            assert_eq!(result.is_ok(), mode == 0);
            assert_eq!(session.needs_reload(), mode != 0);
            let store = session.store().inner.clone();
            drop(session);
            let mut restored = DurableSession::open(
                project.clone(),
                store,
                SavePurpose::SinglePlayer,
                SessionOptions::default(),
            )
            .await
            .unwrap();
            let committed = mode != 1;
            assert_eq!(restored.saved().revision, revision + u64::from(committed));
            let actor = restored.saved().world_state.regions[0]
                .entities
                .iter()
                .find(|e| e.id == actor_id)
                .unwrap();
            assert_eq!(
                actor.inventory.len(),
                if committed { MAX_GROUP_ACTIONS } else { 0 }
            );
            if committed {
                for sequence in 1..=MAX_GROUP_ACTIONS as u64 {
                    assert_eq!(
                        restored
                            .commit_sequenced_live(stream, sequence, &fingerprint, |_| panic!(
                                "recovered retry cannot grant twice"
                            ))
                            .await
                            .unwrap(),
                        serde_json::json!(sequence)
                    );
                }
            } else {
                assert!(restored.action_journal(stream).is_none());
            }
        }
    });
}

#[test]
fn private_metadata_commits_fail_closed_without_publishing_an_uncommitted_head() {
    block_on(async {
        for mode in [1,2] {
            let mut project=Project::new();project.regions[0].name="Private metadata".into();project.regions[0].map.name="Private metadata".into();
            project.config="[game]\nstart_region='Private metadata'\nauto_create_player=false\n[ruleset]\nsource='project'".into();project.rules_document.current=String::new();project.rules_document.original=String::new();
            let fault=Arc::new(AtomicU8::new(0));
            let mut session=DurableSession::open(project,FaultStore{inner:Default::default(),fault:fault.clone()},SavePurpose::Dedicated,SessionOptions{create_local_player:false,..Default::default()}).await.unwrap();
            let revision=session.saved().revision;
            session.commit_host_metadata("accounts.v1",serde_json::json!({"revision":0})).await.unwrap();
            assert_eq!(session.saved().revision,revision+1);assert!(session.saved().requests.is_empty());
            fault.store(mode,Ordering::SeqCst);
            assert!(session.commit_host_metadata("accounts.v1",serde_json::json!({"revision":1})).await.is_err());
            assert_eq!(session.saved().host_data["accounts.v1"],serde_json::json!({"revision":0}));
            assert!(matches!(session.checkpoint().await,Err(SaveError::NeedsReload)));
        }
    });
}
