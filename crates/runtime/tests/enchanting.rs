//! One focused world-to-enchantment scenario, starting empty in both play modes.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use rusterix::{
    Entity, GeometryObject, Item, Value,
    effective_values::{ValueRules, enchantments},
    learning,
    server::region::resources::{self, ResourceRequest},
};
use shared::rulesets::world_resources::{ResourceBinding, bind_source};
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
    project.regions[0].name = "Bear inscription".into();
    project.regions[0].map.name = "Bear inscription".into();
    project.config = "[game]\nstart_region='Bear inscription'\nauto_create_player=false".into();
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

fn spawn(game: &mut GameSession, connection: PlayerConnectionId) -> u32 {
    let region = game.project.regions[0].map.id;
    let context = game.server.instances[0].lock().unwrap().context().unwrap();
    let id;
    {
        let mut ctx = context.lock().unwrap();
        let mut player = Entity::new();
        id = ctx.world.next_id();
        player.id = id;
        player.inventory = vec![None; 24];
        for (key, value) in [
            ("player", Value::Bool(true)),
            ("HP", Value::Int(100)),
            ("MAX_HP", Value::Int(100)),
            ("STR", Value::Int(12)),
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
fn on_item(id: u32, action: &str) -> GameRequest {
    GameRequest::Item {
        target: ItemAddress {
            item: id,
            owner: ItemOwner::Owned,
        },
        intent: Some(format!("action:{action}")),
    }
}
fn state(game: &GameSession, id: u32) -> serde_json::Value {
    let mut value = serde_json::to_value(actor(game, id)).unwrap();
    value["attributes"]["values"]
        .as_object_mut()
        .unwrap()
        .remove("intent");
    value
}
fn ensure_water(game: &mut GameSession, connection: PlayerConnectionId, actor: u32, river: Uuid) {
    let bowl = item(game, actor, "wooden_bowl");
    if bowl.attributes.get_int_default("liquid_quantity", 0) == 0 {
        collect(
            game,
            connection,
            river,
            "fill_water",
            4,
            None,
            Some(bowl.id),
        );
    }
}
fn strength(game: &GameSession, actor_id: u32) -> f32 {
    let ctx = game.server.instances[0].lock().unwrap().context().unwrap();
    let ctx = ctx.lock().unwrap();
    let rules = ValueRules::resolve(&ctx.rules).unwrap();
    rules
        .explain(
            ctx.map.entities.iter().find(|e| e.id == actor_id).unwrap(),
            "STR",
            0.,
        )
        .unwrap()
        .value
}

#[test]
fn enchanting_world_chain_survives_transfer_reconnect_save_and_receipt_replay_in_both_modes() {
    pollster::block_on(async {
        for purpose in [SavePurpose::SinglePlayer, SavePurpose::Dedicated] {
            let (project, sources) = project();
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
            let connection = if purpose == SavePurpose::SinglePlayer {
                session.game.server.local_connection()
            } else {
                session.game.open_player_connection().unwrap()
            };
            let student = spawn(&mut session.game, connection);
            assert!(
                actor(&session.game, student)
                    .inventory
                    .iter()
                    .all(Option::is_none)
            );
            assert_eq!(
                learning::skill_points(&actor(&session.game, student), "boneworking"),
                0
            );
            let context = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let mut bones = Vec::new();
            let mut hides = Vec::new();
            let teacher;
            let book;
            {
                let mut ctx = context.lock().unwrap();
                // Authored creatures produce remains; recipes receive no prepared materials.
                for (profile, accesses) in [
                    ("skeleton_remains", &mut bones),
                    ("deer_remains", &mut hides),
                ] {
                    for _ in 0..2 {
                        let mut creature = Entity::new();
                        creature.id = ctx.world.next_id();
                        let id = creature.id;
                        creature.set_attribute("mode", Value::Str("dead".into()));
                        creature.set_attribute("remains_profile", Value::Str(profile.into()));
                        ctx.map.entities.push(creature);
                        accesses.push(resources::create_remains(&mut ctx, id).unwrap().unwrap());
                    }
                }
                let mut npc = Entity::new();
                npc.id = ctx.world.next_id();
                teacher = npc.id;
                npc.set_attribute("name", Value::Str("Sigil Keeper".into()));
                npc.set_attribute("profession", Value::Str("Inscriber".into()));
                ctx.entity_classes.insert(teacher, "Fixture".into());
                ctx.map.entities.push(npc);
                let templates = shared::rulesets::ruleset_item_templates_from_source(
                    &project.rules_document.current,
                )
                .unwrap();
                let template = templates
                    .iter()
                    .find(|t| t.id == "bear_inscription_manual")
                    .unwrap();
                let mut manual = Item::new();
                manual.id = ctx.world.next_id();
                book = manual.id;
                rusterix::server::data::apply_item_data(&mut manual, &template.data);
                ctx.map.items.push(manual);
            }
            for _ in 0..3 {
                session.game.poll();
            }
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
                craft(&mut session.game, connection, student, "craft_twist_cord");
            }
            craft(
                &mut session.game,
                connection,
                student,
                "craft_make_stone_knife",
            );
            let knife = item(&session.game, student, "stone_knife").id;
            craft(&mut session.game, connection, student, "craft_carve_bowl");
            craft(
                &mut session.game,
                connection,
                student,
                "craft_build_tanning_frame",
            );
            craft(
                &mut session.game,
                connection,
                student,
                "craft_build_bone_workbench",
            );
            for name in ["tanning_frame", "bone_workbench"] {
                let station = item(&session.game, student, name).id;
                submit(
                    &mut session.game,
                    connection,
                    GameRequest::Transfer {
                        source: ItemSource::Direct(ItemAddress {
                            item: station,
                            owner: ItemOwner::Owned,
                        }),
                        destination: ItemDestination::Ground([0., 0.]),
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
                    Some(knife),
                    None,
                );
            }
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
                    sources[3],
                    "collect_bark",
                    1,
                    Some(knife),
                    None,
                );
            }
            for _ in 0..2 {
                ensure_water(&mut session.game, connection, student, sources[5]);
                craft(&mut session.game, connection, student, "craft_tan_hide");
            }
            for _ in 0..3 {
                craft(&mut session.game, connection, student, "craft_cut_straps");
            }
            for _ in 0..10 {
                ensure_water(&mut session.game, connection, student, sources[5]);
                craft(&mut session.game, connection, student, "craft_shape_bone");
            }
            craft(&mut session.game, connection, student, "craft_bone_cuirass");
            let cuirass = item(&session.game, student, "bone_cuirass");
            let id = cuirass.id;
            assert_eq!(quantity(&session.game, student, "bone_plate"), 2);
            assert!(learning::skill_points(&actor(&session.game, student), "boneworking") >= 10);
            for _ in 0..2 {
                collect(
                    &mut session.game,
                    connection,
                    sources[3],
                    "collect_resin",
                    1,
                    Some(knife),
                    None,
                );
            }
            assert_eq!(quantity(&session.game, student, "tree_resin"), 2);
            let before = state(&session.game, student);
            submit(
                &mut session.game,
                connection,
                on_item(id, "inscribe_bear_cuirass"),
            )
            .unwrap();
            assert_eq!(
                state(&session.game, student),
                before,
                "unlearned inscription spends nothing"
            );
            if purpose == SavePurpose::SinglePlayer {
                submit(
                    &mut session.game,
                    connection,
                    GameRequest::Transfer {
                        source: ItemSource::Direct(ItemAddress {
                            item: book,
                            owner: ItemOwner::World,
                        }),
                        destination: ItemDestination::Inventory(None),
                    },
                )
                .unwrap();
                submit(
                    &mut session.game,
                    connection,
                    on_item(book, "study_bear_inscription"),
                )
                .unwrap();
            } else {
                submit(
                    &mut session.game,
                    connection,
                    GameRequest::Entity {
                        target: teacher,
                        intent: Some("action:learn_bear_inscription".into()),
                    },
                )
                .unwrap();
            }
            let learned = learning::journal(&actor(&session.game, student)).unwrap();
            assert_eq!(learned.acquisitions.len(), 1);
            assert_eq!(learned.acquisitions[0].knowledge, "bear_inscription");
            ensure_water(&mut session.game, connection, student, sources[5]);
            craft(&mut session.game, connection, student, "craft_bear_inlay");
            assert_eq!(quantity(&session.game, student, "bear_inlay"), 1);
            assert_eq!(quantity(&session.game, student, "tree_resin"), 1);
            equip(&mut session.game, connection, id, "torso");
            {
                let mut ctx = context.lock().unwrap();
                let actor = ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == student)
                    .unwrap();
                let armor = actor.get_equipped_item_mut("torso").unwrap();
                armor.set_attribute("quality", Value::Int(73));
                armor.set_attribute("condition", Value::Int(63));
                armor.set_attribute("future_property", Value::Str("keep me".into()));
            }
            for _ in 0..3 {
                session.game.poll();
            }
            ensure_water(&mut session.game, connection, student, sources[5]);
            let wear = item(&session.game, student, "stone_knife")
                .attributes
                .get_int("condition")
                .unwrap();
            let water = item(&session.game, student, "wooden_bowl")
                .attributes
                .get_int("liquid_quantity")
                .unwrap();
            session.checkpoint().await.unwrap();
            let stream = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == student)
                .unwrap()
                .id;
            session
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |game| {
                    submit(game, connection, on_item(id, "inscribe_bear_cuirass"))
                })
                .await
                .unwrap();
            let enchanted = item(&session.game, student, "bone_cuirass");
            assert_eq!(
                (enchanted.id, enchanted.creator_id),
                (cuirass.id, cuirass.creator_id)
            );
            assert_eq!(
                enchanted.attributes.get_str("name"),
                Some("Bone Cuirass of the Bear")
            );
            assert_eq!(enchanted.attributes.get_int("quality"), Some(73));
            assert_eq!(enchanted.attributes.get_int("condition"), Some(63));
            assert_eq!(
                enchanted.attributes.get_str("future_property"),
                Some("keep me")
            );
            let attachments = enchantments(&enchanted).unwrap();
            assert_eq!(attachments.len(), 1);
            assert_eq!(attachments[0].definition, "strength");
            assert_eq!(quantity(&session.game, student, "bear_inlay"), 0);
            assert_eq!(quantity(&session.game, student, "tree_resin"), 0);
            assert_eq!(
                item(&session.game, student, "stone_knife")
                    .attributes
                    .get_int("condition"),
                Some(wear - 2)
            );
            assert_eq!(
                item(&session.game, student, "wooden_bowl")
                    .attributes
                    .get_int("liquid_quantity"),
                Some(water - 1)
            );
            assert_eq!(strength(&session.game, student), 14.);
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
            let replicated = replica.get_equipped_item("torso").unwrap();
            assert_eq!(enchantments(replicated).unwrap(), attachments);
            assert_eq!(
                replicated.attributes.get_str("name"),
                Some("Bone Cuirass of the Bear")
            );
            let committed = state(&session.game, student);
            session
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |_| {
                    panic!("retry must not inscribe twice")
                })
                .await
                .unwrap();
            submit(
                &mut session.game,
                connection,
                on_item(id, "inscribe_bear_cuirass"),
            )
            .unwrap();
            assert_eq!(
                state(&session.game, student),
                committed,
                "a fresh duplicate also spends nothing"
            );
            session
                .commit_sequenced_live(stream, 2, &format!("{:064x}", 2), |game| {
                    submit(game, connection, on_item(id, "repair_bone_armor"))
                })
                .await
                .unwrap();
            assert_eq!(
                item(&session.game, student, "bone_cuirass")
                    .attributes
                    .get_int("condition"),
                Some(83)
            );
            assert_eq!(
                enchantments(&item(&session.game, student, "bone_cuirass")).unwrap(),
                attachments
            );
            let other = session.game.open_player_connection().unwrap();
            let recipient = spawn(&mut session.game, other);
            session.checkpoint().await.unwrap();
            let recipient_stream = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == recipient)
                .unwrap()
                .id;
            submit(
                &mut session.game,
                connection,
                GameRequest::Transfer {
                    source: ItemSource::Direct(ItemAddress {
                        item: id,
                        owner: ItemOwner::Owned,
                    }),
                    destination: ItemDestination::Ground([0., 0.]),
                },
            )
            .unwrap();
            submit(
                &mut session.game,
                other,
                GameRequest::Transfer {
                    source: ItemSource::Direct(ItemAddress {
                        item: id,
                        owner: ItemOwner::World,
                    }),
                    destination: ItemDestination::Inventory(None),
                },
            )
            .unwrap();
            assert_eq!(strength(&session.game, student), 12.);
            assert_eq!(strength(&session.game, recipient), 12.);
            equip(&mut session.game, other, id, "torso");
            assert_eq!(strength(&session.game, recipient), 14.);
            assert!(
                learning::journal(&actor(&session.game, recipient))
                    .unwrap()
                    .acquisitions
                    .is_empty()
            );
            assert_eq!(
                learning::journal(&actor(&session.game, student)).unwrap(),
                learned
            );
            session.game.close_player_connection(other);
            let reconnect = session.game.open_player_connection().unwrap();
            session.join_character(reconnect, recipient_stream).unwrap();
            for _ in 0..3 {
                session.game.poll();
            }
            assert_eq!(strength(&session.game, recipient), 14.);
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
            let reconnect = restored.game.open_player_connection().unwrap();
            restored
                .join_character(reconnect, recipient_stream)
                .unwrap();
            for _ in 0..3 {
                restored.game.poll();
            }
            restored.game.subscribe_replication(reconnect).unwrap();
            let saved = item(&restored.game, recipient, "bone_cuirass");
            assert_eq!(
                (saved.id, saved.creator_id),
                (cuirass.id, cuirass.creator_id)
            );
            assert_eq!(enchantments(&saved).unwrap(), attachments);
            assert_eq!(
                saved.attributes.get_str("name"),
                Some("Bone Cuirass of the Bear")
            );
            assert_eq!(saved.attributes.get_int("condition"), Some(83));
            assert_eq!(saved.attributes.get_int("quality"), Some(73));
            assert_eq!(strength(&restored.game, recipient), 14.);
            assert_eq!(
                actor(&restored.game, recipient).attributes.get_int("STR"),
                Some(12)
            );
            assert_eq!(
                learning::journal(&actor(&restored.game, student)).unwrap(),
                learned
            );
            for sequence in [1, 2] {
                restored.commit_sequenced_live(stream,sequence,&format!("{sequence:064x}"),|_|panic!("restored receipt must not consume materials, reattach, repair or move the item")).await.unwrap();
            }
            assert_eq!(
                enchantments(&item(&restored.game, recipient, "bone_cuirass")).unwrap(),
                attachments
            );
        }
    });
}
