//! Living, authored animals are the finite source for hides in both host modes.
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

#[test]
fn authored_animals_are_huntable_and_their_hides_remain_finite_after_reload_in_both_modes() {
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
                options.clone(),
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
                player.inventory = vec![None; 16];
                for (key, value) in [
                    ("player", Value::Bool(true)),
                    ("class", Value::Str("Warrior".into())),
                    ("HP", Value::Int(100)),
                    ("MAX_HP", Value::Int(100)),
                    ("STR", Value::Int(400)),
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
            let animals = {
                let mut ctx = context.lock().unwrap();
                let animals = ctx
                    .map
                    .entities
                    .iter()
                    .filter_map(|e| {
                        e.attributes
                            .get_str("animal")
                            .map(|a| (a.to_owned(), e.id, e.position, e.creator_id))
                    })
                    .collect::<Vec<_>>();
                assert_eq!(animals.len(), 7);
                for (name, id, _, _) in &animals {
                    let e = ctx.find_entity(*id).unwrap();
                    assert_eq!(e.attributes.get_str("avatar"), Some(name.as_str()));
                    assert!(e.attributes.get_float_default("size", 0.) > 0.);
                    if name == "chicken" {
                        assert!(
                            e.attributes.get_float_default("size", 2.) < 1.,
                            "chickens must retain animal scale in the 3D renderer"
                        );
                    }
                    assert!(e.inventory.iter().all(Option::is_none));
                    assert!(e.equipped.is_empty());
                    assert_eq!(
                        resources::create_remains(&mut ctx, *id).unwrap(),
                        None,
                        "living animals cannot be skinned"
                    );
                }
                animals
            };
            for _ in 0..30 {
                session.game.tick_once();
            }
            {
                let ctx = context.lock().unwrap();
                assert!(
                    animals
                        .iter()
                        .any(|(_, id, start, _)| ctx.find_entity(*id).unwrap().position != *start),
                    "the authored routine must actually roam: {:?}; errors {:?}",
                    ctx.map
                        .entities
                        .iter()
                        .filter(|e| e.id != hunter)
                        .map(|e| (
                            e.attributes.get_str("animal"),
                            e.position,
                            &e.action,
                            e.attributes.get_str("__node_behavior_status")
                        ))
                        .collect::<Vec<_>>(),
                    ctx.startup_errors
                );
            }
            // Begin empty: hand gather and make the skinning knife, frame and vessel.
            collect(
                &mut session.game,
                connection,
                sources[0],
                "collect_flint",
                1,
                None,
                None,
            );
            for _ in 0..7 {
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
            for _ in 0..2 {
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
            craft(
                &mut session.game,
                connection,
                hunter,
                "craft_make_stone_knife",
            );
            craft(
                &mut session.game,
                connection,
                hunter,
                "craft_build_tanning_frame",
            );
            craft(&mut session.game, connection, hunter, "craft_carve_bowl");
            let knife = item(&session.game, hunter, "stone_knife").id;
            let bowl = item(&session.game, hunter, "wooden_bowl").id;
            let frame = item(&session.game, hunter, "tanning_frame").id;
            submit(
                &mut session.game,
                connection,
                GameRequest::Transfer {
                    source: ItemSource::Direct(ItemAddress {
                        item: frame,
                        owner: ItemOwner::Owned,
                    }),
                    destination: ItemDestination::Ground([0., 0.]),
                },
            )
            .unwrap();
            let mut carcasses = Vec::new();
            for (name, animal, _, identity) in &animals {
                // Hunt through the same authoritative request used by the player UI.
                for _ in 0..3 {
                    let mut ctx = context.lock().unwrap();
                    if ctx.find_entity(*animal).unwrap().get_mode() == "dead" {
                        break;
                    }
                    let pos = ctx.find_entity(*animal).unwrap().position;
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
                            target: *animal,
                            intent: Some("action:basic_attack".into()),
                        },
                    )
                    .unwrap();
                    session.game.advance(std::time::Duration::from_secs(2));
                }
                let mut ctx = context.lock().unwrap();
                assert_eq!(
                    ctx.find_entity(*animal).unwrap().get_mode(),
                    "dead",
                    "{name} must be huntable"
                );
                let access = resources::create_remains(&mut ctx, *animal)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    resources::create_remains(&mut ctx, *animal).unwrap(),
                    Some(access)
                );
                let corpses = ctx
                    .map
                    .items
                    .iter()
                    .filter(|i| i.attributes.get_uint("corpse_entity_id") == Some(*animal))
                    .collect::<Vec<_>>();
                assert_eq!(corpses.len(), 1, "one carcass per animal incarnation");
                assert_eq!(
                    corpses[0].attributes.get_str("appearance_geometry"),
                    Some(format!("ruleset:geometry/{name}_remains").as_str())
                );
                carcasses.push((
                    name.clone(),
                    *animal,
                    *identity,
                    access,
                    corpses[0].id,
                    corpses[0].creator_id,
                ));
            }
            let pig = carcasses.iter().find(|c| c.0 == "pig").unwrap().clone();
            {
                let mut ctx = context.lock().unwrap();
                let pos = ctx
                    .map
                    .items
                    .iter()
                    .find(|i| i.id == pig.4)
                    .unwrap()
                    .position;
                ctx.map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == hunter)
                    .unwrap()
                    .position = pos;
            }
            let wear = item(&session.game, hunter, "stone_knife")
                .attributes
                .get_int("condition")
                .unwrap();
            collect(
                &mut session.game,
                connection,
                pig.3,
                "collect_hide",
                1,
                Some(knife),
                None,
            );
            assert_eq!(quantity(&session.game, hunter, "raw_hide"), 1);
            collect(
                &mut session.game,
                connection,
                pig.3,
                "collect_hide",
                1,
                Some(knife),
                None,
            );
            assert_eq!(
                quantity(&session.game, hunter, "raw_hide"),
                1,
                "no duplicate skinning"
            );
            assert_eq!(
                item(&session.game, hunter, "stone_knife")
                    .attributes
                    .get_int("condition"),
                Some(wear - 1)
            );
            context
                .lock()
                .unwrap()
                .map
                .entities
                .iter_mut()
                .find(|e| e.id == hunter)
                .unwrap()
                .position = Vec3::zero();
            for _ in 0..2 {
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
            collect(
                &mut session.game,
                connection,
                sources[4],
                "fill_water",
                4,
                None,
                Some(bowl),
            );
            craft(&mut session.game, connection, hunter, "craft_tan_hide");
            assert_eq!(quantity(&session.game, hunter, "raw_hide"), 0);
            assert_eq!(quantity(&session.game, hunter, "leather"), 2);
            session.checkpoint().await.unwrap();
            let stream = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == hunter)
                .unwrap()
                .id;
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
            restored.game.subscribe_replication(connection).unwrap();
            let context = restored.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            {
                let mut ctx = context.lock().unwrap();
                for (_, animal, identity, access, item, creator) in &carcasses {
                    assert_eq!(ctx.find_entity(*animal).unwrap().creator_id, *identity);
                    assert_eq!(
                        resources::create_remains(&mut ctx, *animal).unwrap(),
                        Some(*access)
                    );
                    assert_eq!(
                        ctx.map
                            .items
                            .iter()
                            .find(|i| i.id == *item)
                            .unwrap()
                            .creator_id,
                        *creator
                    );
                }
                let pos = ctx
                    .map
                    .items
                    .iter()
                    .find(|i| i.id == pig.4)
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
                &mut restored.game,
                connection,
                pig.3,
                "collect_hide",
                1,
                Some(knife),
                None,
            );
            assert_eq!(
                quantity(&restored.game, hunter, "raw_hide"),
                0,
                "reload cannot replenish a skinned animal"
            );
            assert_eq!(quantity(&restored.game, hunter, "leather"), 2);
        }
    });
}
