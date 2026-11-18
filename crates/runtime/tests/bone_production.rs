//! Complete supplied production chain, starting empty and withdrawing only world stock.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use rusterix::{
    Entity, GeometryObject, Item, Value,
    server::region::resources::{self, ResourceRequest},
};
use shared::rulesets::{
    equipment_family::BONE_PIECES,
    world_resources::{ResourceBinding, bind_source},
};
use theframework_core::{Uuid, prelude::Vec3};

fn source(project: &Project, profile: &str, interaction: Option<&str>) -> GeometryObject {
    let mut object = GeometryObject::box_(profile, Vec3::zero(), Vec3::new(0.1, 0.1, 0.1));
    object.properties.set(
        "data",
        Value::Str(
            bind_source(
                project.rules_document.document().unwrap().table(),
                "",
                ResourceBinding {
                    profile: profile.into(),
                    interaction: interaction.map(str::to_owned),
                    ..Default::default()
                },
            )
            .unwrap(),
        ),
    );
    object
}
fn project() -> (Project, Vec<Uuid>) {
    let mut project = Project::new();
    project.regions[0].name = "Bone production".into();
    project.regions[0].map.name = "Bone production".into();
    project.config = "[game]\nstart_region='Bone production'\nauto_create_player=false".into();
    let mut sources = [
        source(&project, "loose_flint", None),
        source(&project, "fallen_timber", None),
        source(&project, "wild_flax", None),
        source(&project, "mature_tree", Some("collect_bark")),
        source(&project, "mature_tree", Some("collect_bark")),
        source(&project, "river", None),
    ];
    for (object, (x, z)) in sources.iter_mut().zip([
        (-1., 0.),
        (0., -1.),
        (1., 0.),
        (0., 1.),
        (-1., 1.),
        (1., 1.),
    ]) {
        for vertex in &mut object.vertices {
            *vertex += Vec3::new(x, 0., z);
        }
        object.solid = false;
    }
    let ids = sources.iter().map(|s| s.id).collect();
    project.regions[0].map.geometry_objects.extend(sources);
    // No family generator and no prepared starting stock. Test actual project storage.
    let project =
        shared::project_io::decode_project(&shared::project_io::encode_project(&project).unwrap())
            .unwrap();
    (project, ids)
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
fn item(game: &GameSession, actor_id: u32, name: &str) -> Item {
    let actor = actor(game, actor_id);
    actor
        .inventory
        .into_iter()
        .flatten()
        .chain(actor.equipped.into_values())
        .find(|i| i.attributes.get_str("ruleset_id") == Some(name))
        .unwrap_or_else(|| panic!("Missing {name}"))
}
fn quantity(game: &GameSession, id: u32, name: &str) -> i32 {
    actor(game, id)
        .inventory
        .iter()
        .flatten()
        .filter(|i| i.attributes.get_str("ruleset_id") == Some(name))
        .map(Item::stack_quantity)
        .sum()
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
fn craft(game: &mut GameSession, connection: PlayerConnectionId, id: u32, action: &str) {
    submit(
        game,
        connection,
        GameRequest::Entity {
            target: id,
            intent: Some(format!("action:{action}")),
        },
    )
    .unwrap();
}
fn collect(
    game: &mut GameSession,
    connection: PlayerConnectionId,
    access: Uuid,
    interaction: &str,
    quantity: u32,
    tool: Option<u32>,
    vessel: Option<u32>,
) {
    submit(
        game,
        connection,
        GameRequest::Resource(ResourceRequest {
            access,
            interaction: interaction.into(),
            quantity,
            tool,
            vessel,
        }),
    )
    .unwrap();
}
fn equip(game: &mut GameSession, connection: PlayerConnectionId, id: u32, slot: &str) {
    submit(
        game,
        connection,
        GameRequest::Transfer {
            source: ItemSource::Direct(ItemAddress {
                item: id,
                owner: ItemOwner::Owned,
            }),
            destination: ItemDestination::Equipment(slot.into()),
        },
    )
    .unwrap();
}

#[test]
fn empty_character_produces_equips_repairs_and_restores_the_supplied_set_in_both_modes() {
    pollster::block_on(async {
        for purpose in [SavePurpose::SinglePlayer, SavePurpose::Dedicated] {
            let (project, sources) = project();
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
            let ctx = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let id;
            let mut bones = Vec::new();
            let mut hides = Vec::new();
            {
                let mut ctx = ctx.lock().unwrap();
                let mut player = Entity::default();
                id = ctx.world.next_id();
                player.id = id;
                player.inventory = vec![None; 24];
                player.position = Vec3::zero();
                player.set_attribute("player", Value::Bool(true));
                player.set_attribute("class", Value::Str("Warrior".into()));
                player.set_attribute("race", Value::Str("Human".into()));
                player.set_attribute("HP", Value::Int(100));
                player.set_attribute("MAX_HP", Value::Int(100));
                ctx.entity_classes.insert(id, "Fixture".into());
                ctx.map.entities.push(player);
                for (profile, accesses) in [
                    ("skeleton_remains", &mut bones),
                    ("deer_remains", &mut hides),
                ] {
                    for _ in 0..4 {
                        let mut creature = Entity::default();
                        creature.id = ctx.world.next_id();
                        let dead = creature.id;
                        creature.position = Vec3::zero();
                        creature.set_attribute("mode", Value::Str("dead".into()));
                        creature.set_attribute("remains_profile", Value::Str(profile.into()));
                        ctx.map.entities.push(creature);
                        accesses.push(resources::create_remains(&mut ctx, dead).unwrap().unwrap());
                    }
                }
            }
            session
                .game
                .server
                .attach_saved_character(connection, project.regions[0].map.id, id)
                .unwrap();
            for _ in 0..3 {
                session.game.poll();
            }
            session.game.subscribe_replication(connection).unwrap();
            assert!(
                actor(&session.game, id)
                    .inventory
                    .iter()
                    .all(Option::is_none)
            );
            collect(
                &mut session.game,
                connection,
                sources[0],
                "collect_flint",
                1,
                None,
                None,
            );
            for _ in 0..13 {
                collect(
                    &mut session.game,
                    connection,
                    sources[1],
                    "collect_branches",
                    1,
                    None,
                    None,
                );
            }
            for _ in 0..4 {
                collect(
                    &mut session.game,
                    connection,
                    sources[2],
                    "collect_fiber",
                    2,
                    None,
                    None,
                );
                craft(&mut session.game, connection, id, "craft_twist_cord");
            }
            craft(&mut session.game, connection, id, "craft_make_stone_knife");
            let knife = item(&session.game, id, "stone_knife");
            craft(&mut session.game, connection, id, "craft_carve_bowl");
            let bowl = item(&session.game, id, "wooden_bowl");
            craft(
                &mut session.game,
                connection,
                id,
                "craft_build_tanning_frame",
            );
            craft(
                &mut session.game,
                connection,
                id,
                "craft_build_bone_workbench",
            );
            let mut stations = Vec::new();
            for name in ["tanning_frame", "bone_workbench"] {
                let station = item(&session.game, id, name);
                stations.push((station.id, station.creator_id));
                submit(
                    &mut session.game,
                    connection,
                    GameRequest::Transfer {
                        source: ItemSource::Direct(ItemAddress {
                            item: station.id,
                            owner: ItemOwner::Owned,
                        }),
                        destination: ItemDestination::Ground([0.0, 0.0]),
                    },
                )
                .unwrap();
            }
            for source in &hides {
                collect(
                    &mut session.game,
                    connection,
                    *source,
                    "collect_hide",
                    1,
                    Some(knife.id),
                    None,
                );
            }
            for point in [[0.0, 1.0], [-1.0, 1.0]] {
                // Normal Use follows the access point's selected bark interaction.
                for _ in 0..4 {
                    submit(
                        &mut session.game,
                        connection,
                        GameRequest::Terrain {
                            position: point,
                            intent: "use".into(),
                        },
                    )
                    .unwrap();
                }
            }
            assert_eq!(quantity(&session.game, id, "tree_bark"), 8);
            for source in &bones {
                collect(
                    &mut session.game,
                    connection,
                    *source,
                    "collect_bones",
                    1,
                    None,
                    None,
                );
            }
            for _ in 0..4 {
                collect(
                    &mut session.game,
                    connection,
                    sources[5],
                    "fill_water",
                    4,
                    None,
                    Some(bowl.id),
                );
                craft(&mut session.game, connection, id, "craft_tan_hide");
            }
            assert_eq!(quantity(&session.game, id, "leather"), 8);
            for _ in 0..7 {
                craft(&mut session.game, connection, id, "craft_cut_straps");
            }
            for n in 0..22 {
                if n % 4 == 0 {
                    collect(
                        &mut session.game,
                        connection,
                        sources[5],
                        "fill_water",
                        (22 - n).min(4),
                        None,
                        Some(bowl.id),
                    );
                }
                craft(&mut session.game, connection, id, "craft_shape_bone");
            }
            assert_eq!(quantity(&session.game, id, "bone_plate"), 22);
            session.checkpoint().await.unwrap();
            let character = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == id)
                .unwrap()
                .id;
            let mut members = Vec::new();
            for (n, (suffix, _, slot, _, _, _)) in BONE_PIECES.iter().enumerate() {
                let command = GameRequest::Entity {
                    target: id,
                    intent: Some(format!("action:craft_bone_{suffix}")),
                };
                let fingerprint = format!("{:064x}", n + 1);
                session
                    .commit_sequenced_live(character, (n + 1) as u64, &fingerprint, |game| {
                        submit(game, connection, command)
                    })
                    .await
                    .unwrap();
                let armor = item(&session.game, id, &format!("bone_{suffix}"));
                members.push((slot.to_string(), armor.id, armor.creator_id));
                equip(&mut session.game, connection, armor.id, slot);
                session
                    .commit_sequenced_live(character, (n + 1) as u64, &fingerprint, |_| {
                        panic!("craft retry cannot debit inputs twice")
                    })
                    .await
                    .unwrap();
            }
            for (name, expected) in [
                ("raw_bone", 2),
                ("leather", 1),
                ("leather_strap", 1),
                ("twisted_cord", 1),
                ("bone_plate", 0),
                ("wood_branch", 0),
                ("tree_bark", 0),
                ("raw_hide", 0),
                ("plant_fiber", 0),
                ("flint_stone", 0),
            ] {
                assert_eq!(quantity(&session.game, id, name), expected, "{name}");
            }
            assert_eq!(
                item(&session.game, id, "stone_knife")
                    .attributes
                    .get_int("condition"),
                Some(52)
            );
            assert_eq!(
                item(&session.game, id, "wooden_bowl")
                    .attributes
                    .get_int("liquid_quantity"),
                Some(0)
            );
            assert_eq!(
                item(&session.game, id, "wooden_bowl").creator_id,
                bowl.creator_id
            );
            let client_map = session
                .game
                .client_world(connection)
                .unwrap()
                .current_map()
                .unwrap();
            let visible_actor = client_map.entities.iter().find(|e| e.id == id).unwrap();
            let visible_bowl = visible_actor.get_item(bowl.id).unwrap();
            assert_eq!(visible_bowl.attributes.get_int("liquid_quantity"), Some(0));
            assert_eq!(visible_bowl.attributes.get_str("liquid_item"), Some(""));
            assert_eq!(
                visible_actor
                    .get_item(knife.id)
                    .unwrap()
                    .attributes
                    .get_int("condition"),
                Some(52)
            );
            assert_eq!(
                visible_actor
                    .equipped
                    .values()
                    .map(|i| i.attributes.get_float_default("ARMOR", 0.0))
                    .sum::<f32>(),
                10.0
            );
            // Prepare a repair plate from the world's remaining raw bone and
            // one additional water measure, with no injected material stock.
            collect(
                &mut session.game,
                connection,
                sources[5],
                "fill_water",
                1,
                None,
                Some(bowl.id),
            );
            craft(&mut session.game, connection, id, "craft_shape_bone");
            let cuirass = item(&session.game, id, "bone_cuirass");
            {
                let mut ctx = ctx.lock().unwrap();
                let player = ctx.map.entities.iter_mut().find(|e| e.id == id).unwrap();
                let item = player.equipped.get_mut("torso").unwrap();
                item.set_attribute("condition", Value::Int(0));
                item.set_attribute("quality", Value::Int(73));
                item.set_attribute(
                    "opaque_attachment",
                    Value::Str("unchanged extension data".into()),
                );
                item.set_attribute("charges", Value::Int(3));
                player.dirty_flags |= 0b10000;
            }
            let repair = GameRequest::Item {
                target: ItemAddress {
                    item: cuirass.id,
                    owner: ItemOwner::Owned,
                },
                intent: Some("action:repair_bone_armor".into()),
            };
            session
                .commit_sequenced_live(character, 6, &format!("{:064x}", 6), |game| {
                    submit(game, connection, repair)
                })
                .await
                .unwrap();
            session
                .commit_sequenced_live(character, 6, &format!("{:064x}", 6), |_| {
                    panic!("repair retry cannot execute again")
                })
                .await
                .unwrap();
            let repaired = item(&session.game, id, "bone_cuirass");
            assert_eq!(
                (repaired.id, repaired.creator_id),
                (cuirass.id, cuirass.creator_id)
            );
            assert_eq!(repaired.attributes.get_int("condition"), Some(20));
            assert_eq!(repaired.attributes.get_int("quality"), Some(73));
            assert_eq!(repaired.attributes.get_int("charges"), Some(3));
            assert_eq!(
                repaired.attributes.get_str("opaque_attachment"),
                Some("unchanged extension data")
            );
            assert_eq!(
                item(&session.game, id, "stone_knife")
                    .attributes
                    .get_int("condition"),
                Some(50)
            );
            assert_eq!(quantity(&session.game, id, "raw_bone"), 1);
            assert_eq!(quantity(&session.game, id, "bone_plate"), 0);
            assert_eq!(quantity(&session.game, id, "leather_strap"), 0);
            let client_map = session
                .game
                .client_world(connection)
                .unwrap()
                .current_map()
                .unwrap();
            let repaired = client_map
                .entities
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .get_equipped_item("torso")
                .unwrap();
            assert_eq!(repaired.attributes.get_int("condition"), Some(20));
            assert_eq!(repaired.attributes.get_int("quality"), Some(73));
            session.checkpoint().await.unwrap();
            let high = session.saved().world_state.next_id;
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
            let connection = if local {
                restored.game.server.local_connection()
            } else {
                let c = restored.game.open_player_connection().unwrap();
                restored.join_character(c, character).unwrap();
                c
            };
            for _ in 0..3 {
                restored.game.poll();
            }
            restored.game.subscribe_replication(connection).unwrap();
            for (slot, id_item, creator) in members {
                let worn = actor(&restored.game, id)
                    .get_equipped_item(&slot)
                    .unwrap()
                    .clone();
                assert_eq!((worn.id, worn.creator_id), (id_item, creator));
            }
            assert_eq!(restored.saved().world_state.next_id, high);
            assert_eq!(item(&restored.game, id, "stone_knife").id, knife.id);
            assert_eq!(item(&restored.game, id, "wooden_bowl").id, bowl.id);
            let repaired = item(&restored.game, id, "bone_cuirass");
            assert_eq!(repaired.attributes.get_int("condition"), Some(20));
            assert_eq!(repaired.attributes.get_int("quality"), Some(73));
            assert_eq!(repaired.attributes.get_int("charges"), Some(3));
            assert_eq!(
                repaired.attributes.get_str("opaque_attachment"),
                Some("unchanged extension data")
            );
            assert_eq!(
                item(&restored.game, id, "stone_knife")
                    .attributes
                    .get_int("condition"),
                Some(50)
            );
            restored
                .commit_sequenced_live(character, 6, &format!("{:064x}", 6), |_| {
                    panic!("saved repair receipt cannot execute again")
                })
                .await
                .unwrap();
            restored
                .commit_sequenced_live(character, 1, &format!("{:064x}", 1), |_| {
                    panic!("saved crafting receipt must prevent duplication")
                })
                .await
                .unwrap();
            let ctx = restored.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let ctx = ctx.lock().unwrap();
            for (id, creator) in stations {
                let station = ctx.map.items.iter().find(|i| i.id == id).unwrap();
                assert_eq!(station.creator_id, creator);
            }
            for access in hides.iter().chain(bones.iter()) {
                let view = resources::views(&ctx)
                    .unwrap()
                    .into_iter()
                    .find(|v| v.access == *access)
                    .unwrap();
                assert!(view.supplies.values().all(|q| *q == Some(0)));
            }
        }
    });
}
