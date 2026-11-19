//! Focused journal inspection, pinning and authoritative crafting in both play modes.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use rusterix::{
    Entity, Item, RegionCtx, Value,
    client::screen_model::{JournalModel, JournalTab},
    learning,
};
use theframework_core::prelude::Vec3;
fn actor(game: &GameSession, id: u32) -> Entity {
    let context = game.server.instances[0].lock().unwrap().context().unwrap();
    let ctx = context.lock().unwrap();
    ctx.map
        .entities
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone()
}
fn spawn(game: &mut GameSession, connection: PlayerConnectionId) -> u32 {
    let region = game.project.regions[0].map.id;
    let context = game.server.instances[0].lock().unwrap().context().unwrap();
    let id;
    {
        let mut ctx = context.lock().unwrap();
        let mut player = Entity::new();
        id = ctx.world.next_id();
        player.id = id;
        player.position = theframework_core::prelude::Vec3::zero();
        player.inventory = vec![None; 8];
        for (key, value) in [
            ("player", Value::Bool(true)),
            ("HP", Value::Int(100)),
            ("MAX_HP", Value::Int(100)),
            ("STR", Value::Int(12)),
            ("ARMOR", Value::Int(5)),
        ] {
            player.set_attribute(key, value);
        }
        player.set_attribute("skill_boneworking", Value::Int(10));
        ctx.entity_classes.insert(id, "Fixture".into());
        ctx.map.entities.push(player);
    }
    game.server
        .attach_saved_character(connection, region, id)
        .unwrap();
    for _ in 0..3 {
        game.poll();
    }
    game.subscribe_replication(connection).unwrap();
    id
}
fn submit(
    game: &mut GameSession,
    connection: PlayerConnectionId,
    request: GameRequest,
) -> Result<serde_json::Value, SaveError> {
    let replica = game.client_world(connection).unwrap();
    let actions = game
        .prepare_game_request(
            connection,
            game.driver().session_id(),
            replica.generation(),
            replica.current_region().unwrap(),
            &request,
            None,
        )
        .map_err(|e| SaveError::Invalid(format!("{e:?}")))?;
    for action in actions {
        game.connected_player_action(connection, action)
            .map_err(SaveError::Invalid)?;
    }
    for _ in 0..3 {
        game.poll();
    }
    Ok(serde_json::json!("processed"))
}

fn make_item(
    ctx: &RegionCtx,
    templates: &[shared::rulesets::RulesetItemTemplate],
    id: &str,
) -> Item {
    let template = templates.iter().find(|t| t.id == id).unwrap();
    let mut item = Item::new();
    item.id = ctx.world.next_id();
    rusterix::server::data::apply_item_data(&mut item, &template.data);
    item
}
fn quantity(entity: &Entity, id: &str) -> i32 {
    entity
        .iter_inventory()
        .map(|(_, i)| i)
        .filter(|i| i.attributes.get_str("ruleset_id") == Some(id))
        .map(Item::stack_quantity)
        .sum()
}
fn progression_snapshot(entity: Entity) -> serde_json::Value {
    let mut snapshot = serde_json::to_value(entity).unwrap();
    // Dispatch clears the transient intent even when an operation is denied.
    snapshot["attributes"]["values"]
        .as_object_mut()
        .unwrap()
        .remove("intent");
    snapshot
}

#[test]
fn journal_runtime_uses_owner_replication_and_preserves_pins_knowledge_crafts_and_receipts_in_both_modes()
 {
    pollster_block(async {
        for purpose in [SavePurpose::SinglePlayer, SavePurpose::Dedicated] {
            let mut project = Project::new();
            project.regions[0].name = "Journal".into();
            project.regions[0].map.name = "Journal".into();
            project.config = "[game]\nstart_region='Journal'\nauto_create_player=false".into();
            let templates = shared::rulesets::ruleset_item_templates_from_source(
                &project.rules_document.current,
            )
            .unwrap();
            let mut session = DurableSession::open(
                project.clone(),
                MemoryWorldStore::default(),
                purpose.clone(),
                SessionOptions {
                    create_local_player: false,
                    ..SessionOptions::terminal()
                },
            )
            .await
            .unwrap();
            let local = purpose == SavePurpose::SinglePlayer;
            let connection = if local {
                session.game.server.local_connection()
            } else {
                session.game.open_player_connection().unwrap()
            };
            let student = spawn(&mut session.game, connection);
            let context = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let teacher;
            let station;
            {
                let mut ctx = context.lock().unwrap();
                let mut items = Vec::new();
                for id in ["raw_bone", "wooden_bowl", "stone_knife"] {
                    let mut item = make_item(&ctx, &templates, id);
                    if id == "raw_bone" {
                        item.set_stack_quantity(3);
                    }
                    if id == "wooden_bowl" {
                        item.set_attribute("liquid_item", Value::Str("fresh_water".into()));
                        item.set_attribute("liquid_quantity", Value::Int(3));
                    }
                    items.push(item);
                }
                let actor = ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|actor| actor.id == student)
                    .unwrap();
                for item in items {
                    actor.add_item(item).unwrap();
                }
                let mut bench = make_item(&ctx, &templates, "bone_workbench");
                bench.position = Vec3::zero();
                station = bench.id;
                ctx.map.items.push(bench);
                let mut npc = Entity::new();
                npc.id = ctx.world.next_id();
                teacher = npc.id;
                npc.set_attribute("profession", Value::Str("Boneworker".into()));
                npc.set_attribute("name", Value::Str("Mara".into()));
                ctx.entity_classes.insert(teacher, "Fixture".into());
                ctx.map.entities.push(npc);
            }
            for _ in 0..3 {
                session.game.poll();
            }
            let mut journal = JournalModel::default();
            {
                let map = session
                    .game
                    .client_world(connection)
                    .unwrap()
                    .current_map()
                    .unwrap();
                let actor = map
                    .entities
                    .iter()
                    .find(|actor| actor.id == student)
                    .unwrap();
                journal
                    .bind(&session.game.assets, actor, map, false)
                    .unwrap();
                assert!(
                    journal
                        .entries(&session.game.assets, JournalTab::Recipes, "batch", "all")
                        .unwrap()
                        .is_empty()
                );
                assert!(!journal.inspect("craft_shape_bone_batch", None).ready);
            }
            submit(
                &mut session.game,
                connection,
                GameRequest::Entity {
                    target: teacher,
                    intent: Some("action:learn_boneworking".into()),
                },
            )
            .unwrap();
            let learned = learning::journal(&actor(&session.game, student)).unwrap();
            assert_eq!(learned.acquisitions.len(), 1);
            {
                let map = session
                    .game
                    .client_world(connection)
                    .unwrap()
                    .current_map()
                    .unwrap();
                let actor = map
                    .entities
                    .iter()
                    .find(|actor| actor.id == student)
                    .unwrap();
                journal
                    .bind(&session.game.assets, actor, map, false)
                    .unwrap();
                assert_eq!(
                    journal
                        .entries(
                            &session.game.assets,
                            JournalTab::Recipes,
                            "batch",
                            "boneworking"
                        )
                        .unwrap()
                        .len(),
                    1
                );
                assert!(
                    journal
                        .entries(
                            &session.game.assets,
                            JournalTab::Knowledge,
                            "Mara",
                            "permanent"
                        )
                        .unwrap()[0]
                        .lines
                        .iter()
                        .any(|line| line.contains("Mara"))
                );
                assert!(
                    journal.inspect("craft_shape_bone_batch", None).ready,
                    "public station usability must reach remote previews"
                );
            }
            session.checkpoint().await.unwrap();
            let stream = session
                .saved()
                .characters
                .iter()
                .find(|character| character.entity == student)
                .unwrap()
                .id;
            session
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |game| {
                    submit(
                        game,
                        connection,
                        GameRequest::CommandSlot {
                            slot: "main.0".into(),
                            command: Some("rules.craft_shape_bone_batch".into()),
                        },
                    )
                })
                .await
                .unwrap();
            assert_eq!(
                actor(&session.game, student)
                    .attributes
                    .get_str("command_slot_main_0"),
                Some("rules.craft_shape_bone_batch")
            );
            // A remote preview is advisory. Remove a bench after inspection;
            // authority must refuse the stale craft without spending anything.
            let removed = {
                let mut ctx = context.lock().unwrap();
                let index = ctx
                    .map
                    .items
                    .iter()
                    .position(|item| item.id == station)
                    .unwrap();
                ctx.map.items.remove(index)
            };
            let before = progression_snapshot(actor(&session.game, student));
            let craft = GameRequest::Entity {
                target: student,
                intent: Some("action:craft_shape_bone_batch".into()),
            };
            submit(&mut session.game, connection, craft.clone()).unwrap();
            assert_eq!(progression_snapshot(actor(&session.game, student)), before);
            {
                context.lock().unwrap().map.items.push(removed);
            }
            for _ in 0..3 {
                session.game.poll();
            }
            session
                .commit_sequenced_live(stream, 2, &format!("{:064x}", 2), |game| {
                    submit(game, connection, craft.clone())
                })
                .await
                .unwrap();
            assert_eq!(quantity(&actor(&session.game, student), "bone_plate"), 3);
            {
                let map = session
                    .game
                    .client_world(connection)
                    .unwrap()
                    .current_map()
                    .unwrap();
                let actor = map
                    .entities
                    .iter()
                    .find(|actor| actor.id == student)
                    .unwrap();
                journal
                    .bind(&session.game.assets, actor, map, false)
                    .unwrap();
                assert!(!journal.inspect("craft_shape_bone_batch", None).ready);
                assert_eq!(learning::journal(actor).unwrap(), learned);
            }
            session.game.close_player_connection(connection);
            let reconnect = session.game.open_player_connection().unwrap();
            session.join_character(reconnect, stream).unwrap();
            for _ in 0..3 {
                session.game.poll();
            }
            session.checkpoint().await.unwrap();
            let store = session.store().clone();
            drop(session);
            let mut restored = DurableSession::open(
                project,
                store,
                purpose,
                SessionOptions {
                    create_local_player: false,
                    ..SessionOptions::terminal()
                },
            )
            .await
            .unwrap();
            if local {
                restored
                    .game
                    .close_player_connection(restored.game.server.local_connection());
            }
            let reconnect = restored.game.open_player_connection().unwrap();
            restored.join_character(reconnect, stream).unwrap();
            for _ in 0..3 {
                restored.game.poll();
            }
            restored.game.subscribe_replication(reconnect).unwrap();
            {
                let map = restored
                    .game
                    .client_world(reconnect)
                    .unwrap()
                    .current_map()
                    .unwrap();
                let actor = map
                    .entities
                    .iter()
                    .find(|actor| actor.id == student)
                    .unwrap();
                journal
                    .bind(&restored.game.assets, actor, map, false)
                    .unwrap();
                assert_eq!(learning::journal(actor).unwrap(), learned);
                assert_eq!(quantity(actor, "bone_plate"), 3);
                assert_eq!(
                    rusterix::client::screen_model::command_for_slot(
                        "main.0",
                        &restored.game.assets,
                        Some(actor),
                        &Default::default()
                    )
                    .as_deref(),
                    Some("rules.craft_shape_bone_batch")
                );
                assert_eq!(
                    journal
                        .entries(
                            &restored.game.assets,
                            JournalTab::Knowledge,
                            "",
                            "permanent"
                        )
                        .unwrap()
                        .len(),
                    1
                );
            }
            for sequence in [1, 2] {
                restored
                    .commit_sequenced_live(stream, sequence, &format!("{sequence:064x}"), |_| {
                        panic!("restored pin/craft receipts must not execute twice")
                    })
                    .await
                    .unwrap();
            }
        }
    });
}
fn pollster_block<F: std::future::Future<Output = ()>>(future: F) {
    futures::executor::block_on(future);
}
