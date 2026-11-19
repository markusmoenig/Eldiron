//! Generated content uses the same authoritative request and save paths for
//! local play and dedicated multiplayer. Ingredients here are explicit test
//! fixtures, not vendor restocks or a replacement for authored world sources.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use pollster::block_on;
use rusterix::{Entity, Item, Value};
use shared::rulesets::equipment_family::{BONE_PIECES, BoneArmorFamily};

fn project() -> Project {
    let mut project = Project::new();
    let mut document = project.rules_document.document().unwrap();
    let mut spec = BoneArmorFamily::default();
    spec.id = "test_bone".into();
    spec.classes.push("Warrior".into());
    spec.preview(&document)
        .unwrap()
        .apply(&mut document)
        .unwrap();
    project.rules_document.current = document.to_toml().unwrap();
    project.preview_workbench_rules();
    project.regions[0].name = "Bone family".into();
    project.regions[0].map.name = "Bone family".into();
    project.config = "[game]\nstart_region='Bone family'\nauto_create_player=false".into();
    // Exercise actual project archive persistence before any game starts.
    shared::project_io::decode_project(&shared::project_io::encode_project(&project).unwrap())
        .unwrap()
}
fn actor(game: &GameSession, id: u32) -> Entity {
    let ctx = game.server.instances[0].lock().unwrap().context().unwrap();
    let ctx = ctx.lock().unwrap();
    ctx.map
        .entities
        .iter()
        .find(|e| e.id == id)
        .unwrap()
        .clone()
}
fn spawn(game: &mut GameSession, connection: PlayerConnectionId, materials: bool) -> u32 {
    let region = game.project.regions[0].map.id;
    let context = game.server.instances[0].lock().unwrap().context().unwrap();
    let id;
    {
        let mut ctx = context.lock().unwrap();
        let mut entity = Entity::default();
        id = ctx.world.next_id();
        entity.id = id;
        entity.inventory = vec![None; 16];
        for (key, value) in [
            ("player", Value::Bool(true)),
            ("class", Value::Str("Warrior".into())),
            ("race", Value::Str("Human".into())),
            ("HP", Value::Int(100)),
            ("MAX_HP", Value::Int(100)),
            ("armorsmithing", Value::Int(100)),
            ("STR", Value::Int(20)),
            ("DEX", Value::Int(20)),
        ] {
            entity.set_attribute(key, value);
        }
        if materials {
            for (name, count) in [("bone_plate", 25), ("leather_strap", 15)] {
                let mut item = Item::default();
                item.id = ctx.world.next_id();
                item.set_attribute("ruleset_id", Value::Str(name.into()));
                item.set_attribute(
                    "ruleset_path",
                    Value::Str(format!("items.materials.{name}")),
                );
                item.set_attribute("ruleset_kind", Value::Str("material".into()));
                item.set_attribute("quantity", Value::Int(count));
                item.set_attribute("stackable", Value::Bool(true));
                entity.add_item(item).unwrap();
            }
        }
        ctx.entity_classes.insert(id, "Fixture".into());
        ctx.map.entities.push(entity);
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
fn request(
    game: &mut GameSession,
    connection: PlayerConnectionId,
    request: &GameRequest,
) -> Result<serde_json::Value, SaveError> {
    let region = game.project.regions[0].map.id;
    let generation = game.client_world(connection).unwrap().generation();
    let actions = game
        .prepare_game_request(
            connection,
            game.driver().session_id(),
            generation,
            region,
            request,
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
fn craft(actor: u32, suffix: &str) -> GameRequest {
    GameRequest::Entity {
        target: actor,
        intent: Some(format!("action:craft_test_bone_{suffix}")),
    }
}
fn equip(id: u32, slot: &str) -> GameRequest {
    GameRequest::Transfer {
        source: ItemSource::Direct(ItemAddress {
            item: id,
            owner: ItemOwner::Owned,
        }),
        destination: ItemDestination::Equipment(slot.into()),
    }
}

#[test]
fn generated_family_crafts_equips_reconnects_and_restores_with_stable_item_ids_in_both_modes() {
    block_on(async {
        for purpose in [SavePurpose::SinglePlayer, SavePurpose::Dedicated] {
            let project = project();
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
            let id = spawn(&mut session.game, connection, true);
            let observer = session.game.open_player_connection().unwrap();
            let observer_id = spawn(&mut session.game, observer, false);
            session.checkpoint().await.unwrap();
            let stream = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == id)
                .unwrap()
                .id;
            // A second player cannot produce armor without the inputs.
            request(&mut session.game, observer, &craft(observer_id, "helm")).unwrap();
            assert!(
                actor(&session.game, observer_id)
                    .inventory
                    .iter()
                    .all(Option::is_none)
            );
            let mut members = Vec::new();
            for (index, (suffix, _, slot, _, _, _)) in BONE_PIECES.into_iter().enumerate() {
                let sequence = index as u64 * 2 + 1;
                let command = craft(id, suffix);
                session
                    .commit_sequenced_live(stream, sequence, &format!("{sequence:064x}"), |game| {
                        request(game, connection, &command)
                    })
                    .await
                    .unwrap();
                let crafted = actor(&session.game, id)
                    .inventory
                    .into_iter()
                    .flatten()
                    .find(|item| {
                        item.attributes.get_str("ruleset_id")
                            == Some(format!("test_bone_{suffix}").as_str())
                    })
                    .expect("recipe output");
                members.push((slot.to_string(), crafted.id, crafted.creator_id));
                // Retransmission returns the receipt without crafting twice.
                session
                    .commit_sequenced_live(stream, sequence, &format!("{sequence:064x}"), |_| {
                        panic!("duplicate crafting must not execute")
                    })
                    .await
                    .unwrap();
                let command = equip(crafted.id, slot);
                session
                    .commit_sequenced_live(
                        stream,
                        sequence + 1,
                        &format!("{:064x}", sequence + 1),
                        |game| request(game, connection, &command),
                    )
                    .await
                    .unwrap();
                let worn = actor(&session.game, id);
                assert_eq!(worn.get_equipped_item(slot).map(|i| i.id), Some(crafted.id));
                let replica = session
                    .game
                    .client_world(connection)
                    .unwrap()
                    .current_map()
                    .unwrap();
                let replica_actor = replica.entities.iter().find(|e| e.id == id).unwrap();
                let replica_item = replica_actor
                    .get_equipped_item(slot)
                    .expect("recipient sees equipped armor");
                assert_eq!(
                    (replica_item.id, replica_item.creator_id),
                    (crafted.id, crafted.creator_id)
                );
                assert!(
                    request(&mut session.game, observer, &equip(crafted.id, slot)).is_err(),
                    "another player cannot address an owned item as their own"
                );
            }
            let equipped = actor(&session.game, id);
            assert_eq!(members.len(), 5);
            assert_eq!(
                members
                    .iter()
                    .map(|p| p.1)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                5
            );
            assert_eq!(
                equipped
                    .inventory
                    .iter()
                    .flatten()
                    .find(|i| i.attributes.get_str("ruleset_id") == Some("bone_plate"))
                    .unwrap()
                    .attributes
                    .get_int_default("quantity", 0),
                3
            );
            assert_eq!(
                equipped
                    .inventory
                    .iter()
                    .flatten()
                    .find(|i| i.attributes.get_str("ruleset_id") == Some("leather_strap"))
                    .unwrap()
                    .attributes
                    .get_int_default("quantity", 0),
                2
            );
            if !local {
                session.game.close_player_connection(connection);
                let reconnected = session.game.open_player_connection().unwrap();
                session.join_character(reconnected, stream).unwrap();
                for _ in 0..3 {
                    session.game.poll();
                }
                assert_eq!(
                    session
                        .game
                        .server
                        .player_control(reconnected)
                        .unwrap()
                        .entity_id,
                    id
                );
                for (slot, item, creator) in &members {
                    let worn = actor(&session.game, id);
                    let worn = worn.get_equipped_item(slot).unwrap();
                    assert_eq!((worn.id, worn.creator_id), (*item, *creator));
                }
            }
            session.checkpoint().await.unwrap();
            let high_water = session.saved().world_state.next_id;
            let store = session.store().clone();
            drop(session);
            let options = SessionOptions {
                create_local_player: false,
                ..SessionOptions::terminal()
            };
            let mut restored = DurableSession::open(project, store, purpose, options)
                .await
                .unwrap();
            let connection = if local {
                restored.game.server.local_connection()
            } else {
                let connection = restored.game.open_player_connection().unwrap();
                restored.join_character(connection, stream).unwrap();
                connection
            };
            for _ in 0..3 {
                restored.game.poll();
            }
            restored.game.subscribe_replication(connection).unwrap();
            for (slot, item, creator) in &members {
                let worn = actor(&restored.game, id);
                let worn = worn.get_equipped_item(slot).unwrap();
                assert_eq!((worn.id, worn.creator_id), (*item, *creator));
                assert_eq!(
                    worn.attributes.get_str("ruleset_id"),
                    Some(
                        format!(
                            "test_bone_{}",
                            BONE_PIECES.iter().find(|p| p.2 == slot).unwrap().0
                        )
                        .as_str()
                    )
                );
            }
            assert_eq!(restored.saved().world_state.next_id, high_water);
            restored
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |_| {
                    panic!("receipt survives restart")
                })
                .await
                .unwrap();
            let command = craft(id, "helm");
            restored
                .commit_sequenced_live(stream, 11, &format!("{:064x}", 11), |game| {
                    request(game, connection, &command)
                })
                .await
                .unwrap();
            let helm = actor(&restored.game, id)
                .inventory
                .into_iter()
                .flatten()
                .find(|i| i.attributes.get_str("ruleset_id") == Some("test_bone_helm"))
                .unwrap();
            assert!(
                helm.id >= high_water,
                "allocator cannot reuse any saved item ID"
            );
            assert!(!members.iter().any(|m| m.1 == helm.id));
        }
    });
}
