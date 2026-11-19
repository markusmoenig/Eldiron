//! World-sourced leather armor, permanent patterns, inscription and repair in both modes.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use rusterix::{
    Entity, GeometryObject, Item, Value,
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
    project.regions[0].name = "Fauna".into();
    project.regions[0].map.name = "Fauna".into();
    project.config = "[game]\nstart_region='Fauna'\nauto_create_player=false".into();
    let map = &mut project.regions[0].map;
    map.geometry_objects.clear();
    let vertices =
        [[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]].map(|p| map.add_vertex_at(p[0], p[1]));
    for i in 0..4 {
        map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
    }
    map.close_polygon_manual().unwrap();
    map.geometry_objects.push(GeometryObject::box_(
        "Ground",
        Vec3::new(0., -0.1, 0.),
        Vec3::new(20., 0.2, 20.),
    ));
    let mut sources = vec![
        source(&project, "loose_flint", None),
        source(&project, "fallen_timber", None),
        source(&project, "wild_flax", None),
        source(&project, "mature_tree", Some("collect_bark")),
        source(&project, "river", None),
        source(&project, "mature_tree", Some("collect_bark")),
        source(&project, "mature_tree", Some("collect_bark")),
    ];
    for source in &mut sources {
        source.solid = false;
    }
    let ids = sources.iter().map(|s| s.id).collect();
    project.regions[0].map.geometry_objects.extend(sources);
    let source = project.rules_source().unwrap();
    for (index, id) in ["deer", "pig", "cow", "sheep", "goat", "horse", "chicken"]
        .iter()
        .enumerate()
    {
        let animal = shared::animals::template(&source, id).unwrap();
        let mut instance = animal.character.clone();
        instance.id = Uuid::new_v4();
        instance.character_id = animal.character.id;
        instance.data.clear();
        instance.position = Vec3::new(index as f32 - 3., 0., 4.);
        animal.install(&mut project);
        project.regions[0].characters.insert(instance.id, instance);
    }
    let mut teacher = shared::character::Character::default();
    teacher.name = "Leatherworker".into();
    teacher.data = "[attributes]\nrace='Human'\nclass='Citizen'\nprofession='Leatherworker'\nstart_equipped_items=[]\nstart_inventory_items=[]".into();
    let mut instance = teacher.clone();
    instance.id = Uuid::new_v4();
    instance.character_id = teacher.id;
    instance.data.clear();
    instance.position = Vec3::new(0., 0., -1.);
    project.characters.insert(teacher.id, teacher);
    project.regions[0].characters.insert(instance.id, instance);
    let definition = shared::rulesets::ruleset_item_templates_from_source(&source)
        .unwrap()
        .into_iter()
        .find(|t| t.id == "fox_inscription_manual")
        .unwrap();
    let mut book = shared::item::Item::default();
    book.name = "Patterns of the Fox".into();
    book.data = definition.data;
    let mut instance = book.clone();
    instance.id = Uuid::new_v4();
    instance.item_id = book.id;
    instance.data.clear();
    instance.position = Vec3::zero();
    project.items.insert(book.id, book);
    project.regions[0].items.insert(instance.id, instance);
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

fn on_item(item: u32, action: &str) -> GameRequest {
    GameRequest::Item {
        target: ItemAddress {
            item,
            owner: ItemOwner::Owned,
        },
        intent: Some(format!("action:{action}")),
    }
}
fn ensure_water(game: &mut GameSession, connection: PlayerConnectionId, hunter: u32, river: Uuid) {
    let bowl = item(game, hunter, "wooden_bowl");
    if bowl.attributes.get_int_default("liquid_quantity", 0) < 4 {
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
fn dex(game: &GameSession, hunter: u32) -> f32 {
    let context = game.server.instances[0].lock().unwrap().context().unwrap();
    let ctx = context.lock().unwrap();
    let rules = rusterix::effective_values::ValueRules::resolve(&ctx.rules).unwrap();
    rules
        .explain(ctx.find_entity(hunter).unwrap(), "DEX", 0.)
        .unwrap()
        .value
}

#[test]
fn leather_family_learning_fox_and_repair_preserve_identity_and_stock_in_both_modes() {
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
            let connection = if purpose == SavePurpose::SinglePlayer {
                session.game.server.local_connection()
            } else {
                session.game.open_player_connection().unwrap()
            };
            let context = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let hunter;
            {
                let mut ctx = context.lock().unwrap();
                let mut player = Entity::default();
                hunter = ctx.world.next_id();
                player.id = hunter;
                player.position = Vec3::zero();
                player.inventory = vec![None; 32];
                for (key, value) in [
                    ("player", Value::Bool(true)),
                    ("class", Value::Str("Warrior".into())),
                    ("HP", Value::Int(100)),
                    ("MAX_HP", Value::Int(100)),
                    ("STR", Value::Int(400)),
                    ("DEX", Value::Int(10)),
                    ("abilities", Value::StrArray(vec!["basic_attack".into()])),
                ] {
                    player.set_attribute(key, value);
                }
                ctx.entity_classes.insert(hunter, "Hunter".into());
                ctx.map.entities.push(player);
            }
            session
                .game
                .server
                .attach_saved_character(connection, project.regions[0].map.id, hunter)
                .unwrap();
            for _ in 0..3 {
                session.game.poll();
            }
            session.game.subscribe_replication(connection).unwrap();
            assert_eq!(
                rusterix::learning::skill_points(&actor(&session.game, hunter), "leatherworking"),
                0
            );
            for _ in 0..2 {
                collect(
                    &mut session.game,
                    connection,
                    sources[0],
                    "collect_flint",
                    1,
                    None,
                    None,
                );
            }
            for _ in 0..14 {
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
            for _ in 0..8 {
                collect(
                    &mut session.game,
                    connection,
                    sources[2],
                    "collect_fiber",
                    2,
                    None,
                    None,
                );
                craft(&mut session.game, connection, hunter, "craft_twist_cord");
            }
            for action in [
                "craft_make_stone_knife",
                "craft_carve_bowl",
                "craft_build_tanning_frame",
                "craft_make_flint_awl",
                "craft_build_leather_workbench",
            ] {
                craft(&mut session.game, connection, hunter, action);
            }
            let knife = item(&session.game, hunter, "stone_knife").id;
            for name in ["tanning_frame", "leather_workbench"] {
                let station = item(&session.game, hunter, name).id;
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
            for species in ["deer", "pig", "cow", "sheep", "goat"] {
                let animal = context
                    .lock()
                    .unwrap()
                    .map
                    .entities
                    .iter()
                    .find(|e| e.attributes.get_str("animal") == Some(species))
                    .unwrap()
                    .id;
                for _ in 0..3 {
                    let mut ctx = context.lock().unwrap();
                    if ctx.find_entity(animal).unwrap().get_mode() == "dead" {
                        break;
                    }
                    let pos = ctx.find_entity(animal).unwrap().position;
                    ctx.map
                        .entities
                        .iter_mut()
                        .find(|e| e.id == hunter)
                        .unwrap()
                        .position = pos + Vec3::new(0.6, 0., 0.);
                    drop(ctx);
                    submit(
                        &mut session.game,
                        connection,
                        GameRequest::Entity {
                            target: animal,
                            intent: Some("action:basic_attack".into()),
                        },
                    )
                    .unwrap();
                    session.game.advance(std::time::Duration::from_secs(2));
                }
                let access;
                {
                    let mut ctx = context.lock().unwrap();
                    assert_eq!(ctx.find_entity(animal).unwrap().get_mode(), "dead");
                    access = resources::create_remains(&mut ctx, animal)
                        .unwrap()
                        .unwrap();
                    let pos = ctx
                        .map
                        .items
                        .iter()
                        .find(|i| i.attributes.get_uint("corpse_entity_id") == Some(animal))
                        .unwrap()
                        .position;
                    ctx.map
                        .entities
                        .iter_mut()
                        .find(|e| e.id == hunter)
                        .unwrap()
                        .position = pos;
                }
                collect(
                    &mut session.game,
                    connection,
                    access,
                    "collect_hide",
                    1,
                    Some(knife),
                    None,
                );
            }
            assert_eq!(quantity(&session.game, hunter, "raw_hide"), 6);
            context
                .lock()
                .unwrap()
                .map
                .entities
                .iter_mut()
                .find(|e| e.id == hunter)
                .unwrap()
                .position = Vec3::zero();
            for tree in [sources[3], sources[5], sources[6]] {
                for _ in 0..4 {
                    collect(
                        &mut session.game,
                        connection,
                        tree,
                        "collect_bark",
                        1,
                        Some(knife),
                        None,
                    );
                }
            }
            for _ in 0..6 {
                ensure_water(&mut session.game, connection, hunter, sources[4]);
                craft(&mut session.game, connection, hunter, "craft_tan_hide");
            }
            assert_eq!(quantity(&session.game, hunter, "leather"), 12);
            let pieces = [
                ("leather_hood", "head"),
                ("leather_vest", "torso"),
                ("leather_leggings", "legs"),
                ("leather_gloves", "hands"),
                ("leather_boots", "feet"),
            ];
            for (piece, slot) in pieces {
                craft(
                    &mut session.game,
                    connection,
                    hunter,
                    &format!("craft_make_{piece}"),
                );
                let item = item(&session.game, hunter, piece);
                assert_eq!(item.attributes.get_int("quality"), Some(100));
                submit(
                    &mut session.game,
                    connection,
                    GameRequest::Transfer {
                        source: ItemSource::Direct(ItemAddress {
                            item: item.id,
                            owner: ItemOwner::Owned,
                        }),
                        destination: ItemDestination::Equipment(slot.into()),
                    },
                )
                .unwrap();
            }
            assert_eq!(quantity(&session.game, hunter, "leather"), 4);
            let actor_before = actor(&session.game, hunter);
            assert_eq!(actor_before.equipped.len(), 5);
            {
                let ctx = context.lock().unwrap();
                let rules = rusterix::effective_values::ValueRules::resolve(&ctx.rules).unwrap();
                assert_eq!(rules.explain(&actor_before, "ARMOR", 0.).unwrap().value, 7.);
            }
            assert!(rusterix::learning::skill_points(&actor_before, "leatherworking") >= 10);
            craft(
                &mut session.game,
                connection,
                hunter,
                "craft_cut_straps_batch",
            );
            assert_eq!(
                quantity(&session.game, hunter, "leather"),
                4,
                "unlearned advanced recipes spend nothing"
            );
            let teacher = context
                .lock()
                .unwrap()
                .map
                .entities
                .iter()
                .find(|e| e.attributes.get_str("profession") == Some("Leatherworker"))
                .unwrap()
                .id;
            submit(
                &mut session.game,
                connection,
                GameRequest::Entity {
                    target: teacher,
                    intent: Some("action:learn_advanced_leatherworking".into()),
                },
            )
            .unwrap();
            craft(
                &mut session.game,
                connection,
                hunter,
                "craft_cut_straps_batch",
            );
            assert_eq!(quantity(&session.game, hunter, "leather_strap"), 4);
            let manual = context
                .lock()
                .unwrap()
                .map
                .items
                .iter()
                .find(|i| i.attributes.get_str("ruleset_id") == Some("fox_inscription_manual"))
                .unwrap()
                .id;
            submit(
                &mut session.game,
                connection,
                GameRequest::Transfer {
                    source: ItemSource::Direct(ItemAddress {
                        item: manual,
                        owner: ItemOwner::World,
                    }),
                    destination: ItemDestination::Inventory(None),
                },
            )
            .unwrap();
            // Ordinary Use learns the owned manual; it needs no class-bar entry.
            submit(
                &mut session.game,
                connection,
                GameRequest::Item {
                    target: ItemAddress {
                        item: manual,
                        owner: ItemOwner::Owned,
                    },
                    intent: Some("use".into()),
                },
            )
            .unwrap();
            let learned = rusterix::learning::journal(&actor(&session.game, hunter)).unwrap();
            assert_eq!(learned.acquisitions.len(), 2);
            assert!(
                learned
                    .acquisitions
                    .iter()
                    .any(|k| k.knowledge == "fox_inscription")
            );
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
            ensure_water(&mut session.game, connection, hunter, sources[4]);
            craft(&mut session.game, connection, hunter, "craft_cut_fox_inlay");
            let vest = item(&session.game, hunter, "leather_vest");
            session.checkpoint().await.unwrap();
            let stream = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == hunter)
                .unwrap()
                .id;
            session
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |game| {
                    submit(game, connection, on_item(vest.id, "inscribe_fox"))
                })
                .await
                .unwrap();
            let enchanted = item(&session.game, hunter, "leather_vest");
            assert_eq!(
                (enchanted.id, enchanted.creator_id),
                (vest.id, vest.creator_id)
            );
            assert_eq!(
                enchanted.attributes.get_str("name"),
                Some("Leather Vest of the Fox")
            );
            assert_eq!(dex(&session.game, hunter), 12.);
            let attachments = rusterix::effective_values::enchantments(&enchanted).unwrap();
            let leather = quantity(&session.game, hunter, "leather");
            submit(
                &mut session.game,
                connection,
                on_item(vest.id, "inscribe_fox"),
            )
            .unwrap();
            assert_eq!(
                rusterix::effective_values::enchantments(&item(
                    &session.game,
                    hunter,
                    "leather_vest"
                ))
                .unwrap(),
                attachments
            );
            assert_eq!(quantity(&session.game, hunter, "leather"), leather);
            {
                let mut ctx = context.lock().unwrap();
                let item = ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == hunter)
                    .unwrap()
                    .equipped
                    .get_mut("torso")
                    .unwrap();
                item.set_attribute("condition", Value::Int(0));
                item.set_attribute("quality", Value::Int(73));
                item.set_attribute("future_property", Value::Str("keep me".into()));
            }
            assert_eq!(dex(&session.game, hunter), 10.);
            submit(
                &mut session.game,
                connection,
                on_item(vest.id, "repair_leather_armor"),
            )
            .unwrap();
            let repaired = item(&session.game, hunter, "leather_vest");
            assert_eq!(repaired.attributes.get_int("condition"), Some(25));
            assert_eq!(repaired.attributes.get_int("quality"), Some(73));
            assert_eq!(
                repaired.attributes.get_str("future_property"),
                Some("keep me")
            );
            assert_eq!(
                rusterix::effective_values::enchantments(&repaired).unwrap(),
                attachments
            );
            assert_eq!(dex(&session.game, hunter), 12.);
            assert_eq!(quantity(&session.game, hunter, "leather"), 0);
            session.checkpoint().await.unwrap();
            let store = session.store().clone();
            drop(session);
            let mut restored = DurableSession::open(project, store, purpose, options)
                .await
                .unwrap();
            let local = restored.game.server.local_connection();
            restored.game.close_player_connection(local);
            let connection = restored.game.open_player_connection().unwrap();
            restored.join_character(connection, stream).unwrap();
            for _ in 0..3 {
                restored.game.poll();
            }
            let saved = item(&restored.game, hunter, "leather_vest");
            assert_eq!((saved.id, saved.creator_id), (vest.id, vest.creator_id));
            assert_eq!(
                rusterix::effective_values::enchantments(&saved).unwrap(),
                attachments
            );
            assert_eq!(
                rusterix::learning::journal(&actor(&restored.game, hunter)).unwrap(),
                learned
            );
            assert_eq!(dex(&restored.game, hunter), 12.);
            restored
                .commit_sequenced_live(stream, 1, &format!("{:064x}", 1), |_| {
                    panic!("a replay cannot reattach or consume more materials")
                })
                .await
                .unwrap();
        }
    });
}
