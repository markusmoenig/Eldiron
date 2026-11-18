//! Passive item sources follow real ownership, replication and durable sessions.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use rusterix::{
    Entity, Item, Value,
    effective_values::{ValueRules, enchantments},
};

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
fn totals(rules: &ValueRules, entity: &Entity, strength: f32, armor: f32) {
    assert_eq!(rules.explain(entity, "STR", 0.).unwrap().value, strength);
    assert_eq!(rules.explain(entity, "ARMOR", 0.).unwrap().value, armor);
    assert_eq!(entity.attributes.get_int("STR"), Some(12));
    assert_eq!(entity.attributes.get_int("ARMOR"), Some(5));
}
#[test]
fn supplied_enchantment_survives_transfer_reconnect_and_save_without_saved_bonus_totals() {
    pollster::block_on(async {
        for purpose in [SavePurpose::SinglePlayer, SavePurpose::Dedicated] {
            let mut project = Project::new();
            project.regions[0].name = "Contributions".into();
            project.regions[0].map.name = "Contributions".into();
            project.config =
                "[game]\nstart_region='Contributions'\nauto_create_player=false".into();
            let project = shared::project_io::decode_project(
                &shared::project_io::encode_project(&project).unwrap(),
            )
            .unwrap();
            let rules =
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
            let alice = if local {
                session.game.server.local_connection()
            } else {
                session.game.open_player_connection().unwrap()
            };
            let alice_id = spawn(&mut session.game, alice);
            let bob = session.game.open_player_connection().unwrap();
            let bob_id = spawn(&mut session.game, bob);
            let template = shared::rulesets::ruleset_item_templates_from_source(
                &project.rules_document.current,
            )
            .unwrap()
            .into_iter()
            .find(|t| t.id == "strength_bone_cuirass")
            .unwrap();
            let context = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let item_id;
            let creator;
            let attachments;
            {
                let mut ctx = context.lock().unwrap();
                let mut item = Item::new();
                item_id = ctx.world.next_id();
                item.id = item_id;
                rusterix::server::data::apply_item_data(&mut item, &template.data);
                item.set_attribute("quality", Value::Int(73));
                creator = item.creator_id;
                attachments = enchantments(&item).unwrap();
                assert_eq!(attachments.len(), 1);
                ctx.map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == alice_id)
                    .unwrap()
                    .add_item(item)
                    .unwrap();
            }
            for _ in 0..3 {
                session.game.poll();
            }
            session.checkpoint().await.unwrap();
            let bob_stream = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == bob_id)
                .unwrap()
                .id;
            submit(
                &mut session.game,
                alice,
                transfer(
                    item_id,
                    ItemOwner::Owned,
                    ItemDestination::Equipment("torso".into()),
                ),
            )
            .unwrap();
            totals(&rules, &actor(&session.game, alice_id), 14., 9.);
            assert!(
                submit(
                    &mut session.game,
                    bob,
                    transfer(
                        item_id,
                        ItemOwner::Owned,
                        ItemDestination::Equipment("torso".into())
                    )
                )
                .is_err()
            );
            submit(
                &mut session.game,
                alice,
                transfer(item_id, ItemOwner::Owned, ItemDestination::Inventory(None)),
            )
            .unwrap();
            totals(&rules, &actor(&session.game, alice_id), 12., 5.);
            // Ownership transfer uses the ordinary drop/pickup requests. There
            // is no second transfer-only bonus implementation.
            submit(
                &mut session.game,
                alice,
                transfer(item_id, ItemOwner::Owned, ItemDestination::Ground([0., 0.])),
            )
            .unwrap();
            submit(
                &mut session.game,
                bob,
                transfer(item_id, ItemOwner::World, ItemDestination::Inventory(None)),
            )
            .unwrap();
            totals(&rules, &actor(&session.game, bob_id), 12., 5.);
            let equip = transfer(
                item_id,
                ItemOwner::Owned,
                ItemDestination::Equipment("torso".into()),
            );
            session
                .commit_sequenced_live(bob_stream, 1, &format!("{:064x}", 1), |game| {
                    submit(game, bob, equip.clone())
                })
                .await
                .unwrap();
            session
                .commit_sequenced_live(bob_stream, 1, &format!("{:064x}", 1), |_| {
                    panic!("equip retry must not run")
                })
                .await
                .unwrap();
            totals(&rules, &actor(&session.game, alice_id), 12., 5.);
            totals(&rules, &actor(&session.game, bob_id), 14., 9.);
            let replica = session
                .game
                .client_world(bob)
                .unwrap()
                .current_map()
                .unwrap()
                .entities
                .iter()
                .find(|e| e.id == bob_id)
                .unwrap();
            totals(&rules, replica, 14., 9.);
            session.game.close_player_connection(bob);
            let bob = session.game.open_player_connection().unwrap();
            session.join_character(bob, bob_stream).unwrap();
            for _ in 0..3 {
                session.game.poll();
            }
            totals(&rules, &actor(&session.game, bob_id), 14., 9.);
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
            let bob = restored.game.open_player_connection().unwrap();
            restored.join_character(bob, bob_stream).unwrap();
            for _ in 0..3 {
                restored.game.poll();
            }
            restored.game.subscribe_replication(bob).unwrap();
            let worn = actor(&restored.game, bob_id);
            totals(&rules, &worn, 14., 9.);
            let item = worn.get_equipped_item("torso").unwrap();
            assert_eq!((item.id, item.creator_id), (item_id, creator));
            assert_eq!(item.attributes.get_int("quality"), Some(73));
            assert_eq!(enchantments(item).unwrap(), attachments);
            let replica = restored
                .game
                .client_world(bob)
                .unwrap()
                .current_map()
                .unwrap()
                .entities
                .iter()
                .find(|e| e.id == bob_id)
                .unwrap();
            totals(&rules, replica, 14., 9.);
            restored
                .commit_sequenced_live(bob_stream, 1, &format!("{:064x}", 1), |_| {
                    panic!("restored receipt must not equip twice")
                })
                .await
                .unwrap();
        }
    });
}
