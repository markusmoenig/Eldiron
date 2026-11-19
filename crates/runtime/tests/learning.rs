//! One focused local/dedicated scenario for permanent learning and recipe access.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use rusterix::{
    Entity, Item, RegionCtx, Value,
    effective_values::ValueRules,
    learning::{self, Origin},
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
fn transfer(item: u32, owner: ItemOwner, destination: ItemDestination) -> GameRequest {
    GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress { item, owner }),
        destination,
    }
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
fn learning_requests_keep_acquisitions_recipes_and_receipts_through_book_transfer_reconnect_and_save()
 {
    pollster::block_on(async {
        for purpose in [SavePurpose::SinglePlayer, SavePurpose::Dedicated] {
            let mut project = Project::new();
            project.regions[0].name = "Learning".into();
            project.regions[0].map.name = "Learning".into();
            project.config = "[game]\nstart_region='Learning'\nauto_create_player=false".into();
            let project = shared::project_io::decode_project(
                &shared::project_io::encode_project(&project).unwrap(),
            )
            .unwrap();
            let templates = shared::rulesets::ruleset_item_templates_from_source(
                &project.rules_document.current,
            )
            .unwrap();
            let values =
                ValueRules::resolve(project.rules_document.document().unwrap().table()).unwrap();
            let options = SessionOptions {
                create_local_player: false,
                ..SessionOptions::terminal()
            };
            let mut session = DurableSession::open(
                project.clone(),
                MemoryWorldStore::default(),
                purpose.clone(),
                options,
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
            let other = session.game.open_player_connection().unwrap();
            let other_id = spawn(&mut session.game, other);
            let context = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let book;
            let teacher;
            {
                let mut ctx = context.lock().unwrap();
                let mut npc = Entity::new();
                teacher = ctx.world.next_id();
                npc.id = teacher;
                npc.position = Vec3::zero();
                npc.set_attribute("name", Value::Str("Tutor".into()));
                npc.set_attribute("profession", Value::Str("Boneworker".into()));
                ctx.entity_classes.insert(teacher, "Fixture".into());
                ctx.map.entities.push(npc);
                let mut manual = make_item(&ctx, &templates, "boneworking_manual");
                book = manual.id;
                manual.position = Vec3::zero();
                ctx.map.items.push(manual);
                let mut inputs = Vec::new();
                for id in ["raw_bone", "wooden_bowl", "stone_knife"] {
                    let mut item = make_item(&ctx, &templates, id);
                    if id == "raw_bone" {
                        item.set_stack_quantity(3);
                    }
                    if id == "wooden_bowl" {
                        item.set_attribute("liquid_item", Value::Str("fresh_water".into()));
                        item.set_attribute("liquid_quantity", Value::Int(3));
                    }
                    inputs.push(item);
                }
                let actor = ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == student)
                    .unwrap();
                for item in inputs {
                    actor.add_item(item).unwrap();
                }
                let mut bench = make_item(&ctx, &templates, "bone_workbench");
                bench.position = Vec3::zero();
                ctx.map.items.push(bench);
            }
            for _ in 0..3 {
                session.game.poll();
            }
            session.checkpoint().await.unwrap();
            let stream = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == student)
                .unwrap()
                .id;
            let craft = GameRequest::Entity {
                target: student,
                intent: Some("action:craft_shape_bone_batch".into()),
            };
            let before = progression_snapshot(actor(&session.game, student));
            submit(&mut session.game, connection, craft.clone()).unwrap();
            assert_eq!(
                progression_snapshot(actor(&session.game, student)),
                before,
                "locked crafting spends no inputs or training"
            );
            submit(
                &mut session.game,
                connection,
                transfer(book, ItemOwner::World, ItemDestination::Inventory(None)),
            )
            .unwrap();
            assert!(actor(&session.game, student).get_item(book).is_some());
            let learn = if local {
                GameRequest::Item {
                    target: ItemAddress {
                        item: book,
                        owner: ItemOwner::Owned,
                    },
                    intent: Some("action:study_boneworking".into()),
                }
            } else {
                GameRequest::Entity {
                    target: teacher,
                    intent: Some("action:learn_boneworking".into()),
                }
            };
            session
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |game| {
                    submit(game, connection, learn.clone())
                })
                .await
                .unwrap();
            let learned = learning::journal(&actor(&session.game, student)).unwrap();
            assert_eq!(learned.acquisitions.len(), 1);
            assert!(if local {
                matches!(learned.acquisitions[0].origin,Origin::Book{item,..} if item==book)
            } else {
                matches!(learned.acquisitions[0].origin,Origin::Teacher{entity,..} if entity==teacher)
            });
            let replica = session
                .game
                .client_world(connection)
                .unwrap()
                .current_map()
                .unwrap()
                .entities
                .iter()
                .find(|e| e.id == student)
                .unwrap();
            assert_eq!(learning::journal(replica).unwrap(), learned);
            assert!(
                values
                    .learning
                    .recipe_grants
                    .contains_key("shape_bone_batch")
            );
            // The book moves; permanent knowledge stays with the original learner.
            submit(
                &mut session.game,
                connection,
                transfer(book, ItemOwner::Owned, ItemDestination::Ground([0., 0.])),
            )
            .unwrap();
            submit(
                &mut session.game,
                other,
                transfer(book, ItemOwner::World, ItemDestination::Inventory(None)),
            )
            .unwrap();
            assert!(
                learning::journal(&actor(&session.game, other_id))
                    .unwrap()
                    .acquisitions
                    .is_empty()
            );
            submit(
                &mut session.game,
                other,
                GameRequest::Item {
                    target: ItemAddress {
                        item: book,
                        owner: ItemOwner::Owned,
                    },
                    intent: Some("action:study_boneworking".into()),
                },
            )
            .unwrap();
            let others = learning::journal(&actor(&session.game, other_id)).unwrap();
            assert_eq!(others.acquisitions.len(), 1);
            assert_ne!(others.acquisitions[0].id, learned.acquisitions[0].id);
            {
                let mut ctx = context.lock().unwrap();
                ctx.map.entities.retain(|e| e.id != teacher);
                ctx.entity_classes.remove(&teacher);
            }
            session
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |_| {
                    panic!("learning receipt must not teach twice or need the old source")
                })
                .await
                .unwrap();
            session
                .commit_sequenced_live(stream, 2, &format!("{:064x}", 2), |game| {
                    submit(game, connection, craft.clone())
                })
                .await
                .unwrap();
            let finished = actor(&session.game, student);
            assert_eq!(quantity(&finished, "bone_plate"), 3);
            assert_eq!(quantity(&finished, "raw_bone"), 0);
            assert!(learning::skill_points(&finished, "boneworking") > 10);
            assert_eq!(learning::journal(&finished).unwrap(), learned);
            let knife = finished
                .iter_inventory()
                .map(|(_, i)| i)
                .find(|i| i.attributes.get_str("ruleset_id") == Some("stone_knife"))
                .unwrap();
            assert_eq!(knife.attributes.get_int("condition"), Some(99));
            session.game.close_player_connection(connection);
            let reconnected = session.game.open_player_connection().unwrap();
            session.join_character(reconnected, stream).unwrap();
            for _ in 0..3 {
                session.game.poll();
            }
            assert_eq!(
                learning::journal(&actor(&session.game, student)).unwrap(),
                learned
            );
            session.checkpoint().await.unwrap();
            let store = session.store().clone();
            drop(session);
            let options = SessionOptions {
                create_local_player: false,
                ..SessionOptions::terminal()
            };
            let mut restored = DurableSession::open(project, store, purpose, options)
                .await
                .unwrap();
            if local {
                // Solo restoration reattaches the saved local character automatically.
                restored
                    .game
                    .close_player_connection(restored.game.server.local_connection());
            }
            let reconnected = restored.game.open_player_connection().unwrap();
            restored.join_character(reconnected, stream).unwrap();
            for _ in 0..3 {
                restored.game.poll();
            }
            restored.game.subscribe_replication(reconnected).unwrap();
            let student = actor(&restored.game, student);
            assert_eq!(learning::journal(&student).unwrap(), learned);
            assert!(learning::recipe_access(&values, &student, "shape_bone_batch").is_ok());
            assert_eq!(quantity(&student, "bone_plate"), 3);
            assert_eq!(
                learning::skill_points(&student, "boneworking"),
                learning::skill_points(&finished, "boneworking")
            );
            restored
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |_| {
                    panic!("restored learning receipt must not repeat")
                })
                .await
                .unwrap();
            restored
                .commit_sequenced_live(stream, 2, &format!("{:064x}", 2), |_| {
                    panic!("restored crafting receipt must not repeat")
                })
                .await
                .unwrap();
        }
    });
}
