//! Resource accounting is independent of renderer, host mode and access artwork.
use eldiron_runtime::{
    GameSession, PlayerConnectionId, Project, SessionOptions, persistence::*, requests::*,
};
use rusterix::{
    Entity, GeometryObject, Item, Value,
    server::{
        region::resources::{self, ResourceRequest},
        regionctx::RegionCtx,
        world::SimulationWorld,
    },
};
use shared::rulesets::{
    document::RulesState,
    world_resources::{ResourceBinding, bind_source},
};
use std::{sync::Arc, time::Duration};
use theframework_core::{Uuid, prelude::Vec3};

fn rules() -> shared::rulesets::document::RulesDocument {
    RulesState::default().document().unwrap()
}
fn object(profile: &str, pool: Option<&str>) -> GeometryObject {
    let mut object = GeometryObject::box_(profile, Vec3::zero(), Vec3::one());
    object.properties.set(
        "data",
        Value::Str(
            bind_source(
                rules().table(),
                "[look]\nname='World Source'",
                ResourceBinding {
                    profile: profile.into(),
                    pool: pool.map(str::to_owned),
                    ..Default::default()
                },
            )
            .unwrap(),
        ),
    );
    object
}
fn context() -> RegionCtx {
    let mut ctx = RegionCtx::default();
    ctx.set_rules(rules().table().clone()).unwrap();
    ctx
}
fn actor(ctx: &mut RegionCtx, slots: usize) -> u32 {
    let mut actor = Entity::default();
    actor.id = ctx.world.next_id();
    actor.inventory = vec![None; slots];
    actor.position = Vec3::zero();
    actor.set_attribute("player", Value::Bool(true));
    actor.set_attribute("skill_woodworking", Value::Int(10));
    let id = actor.id;
    ctx.map.entities.push(actor);
    ctx.entity_classes.insert(id, "Fixture".into());
    id
}
fn item(ctx: &mut RegionCtx, actor: u32, capability: Option<&str>, vessel: bool) -> u32 {
    let mut item = Item::default();
    item.id = ctx.world.next_id();
    item.set_attribute("condition", Value::Int(100));
    if let Some(cap) = capability {
        item.set_attribute("tool_capabilities", Value::StrArray(vec![cap.into()]));
    }
    if vessel {
        item.set_attribute("liquid_capacity", Value::Int(4));
        item.set_attribute("liquid_quantity", Value::Int(0));
    }
    let id = item.id;
    ctx.map
        .entities
        .iter_mut()
        .find(|e| e.id == actor)
        .unwrap()
        .add_item(item)
        .unwrap();
    id
}
fn request(
    access: Uuid,
    interaction: &str,
    quantity: u32,
    tool: Option<u32>,
    vessel: Option<u32>,
) -> ResourceRequest {
    ResourceRequest {
        access,
        interaction: interaction.into(),
        quantity,
        tool,
        vessel,
    }
}
fn quantity(ctx: &RegionCtx, actor: u32, item: &str) -> i32 {
    ctx.find_entity(actor)
        .unwrap()
        .inventory
        .iter()
        .flatten()
        .filter(|i| i.attributes.get_str("ruleset_id") == Some(item))
        .map(|i| i.stack_quantity() as i32)
        .sum()
}
#[test]
fn tree_parts_share_stock_neighbours_do_not_and_costs_commit_only_with_outputs() {
    let mut ctx = context();
    let tree = object("mature_tree", None);
    let id = tree.id;
    let mut canopy = tree.clone();
    canopy.id = Uuid::new_v4();
    let canopy_id = canopy.id;
    let logical = Uuid::new_v4().to_string();
    for part in [&mut canopy] {
        part.properties.set(
            "data",
            Value::Str(
                bind_source(
                    rules().table(),
                    "",
                    ResourceBinding {
                        profile: "mature_tree".into(),
                        instance: Some(logical.clone()),
                        ..Default::default()
                    },
                )
                .unwrap(),
            ),
        );
    }
    let mut tree = tree;
    tree.properties = canopy.properties.clone();
    let neighbour = object("mature_tree", None);
    let neighbour_id = neighbour.id;
    ctx.map.geometry_objects = vec![tree, canopy, neighbour];
    resources::initialize(&mut ctx, false).unwrap();
    let a = actor(&mut ctx, 4);
    let knife = item(&mut ctx, a, Some("prune"), false);
    let outcome = resources::execute(
        &mut ctx,
        a,
        &request(id, "collect_branches", 2, Some(knife), None),
    )
    .unwrap();
    assert_eq!(outcome.after, Some(10));
    assert_eq!(quantity(&ctx, a, "wood_branch"), 2);
    let owner = ctx.find_entity(a).unwrap();
    assert_eq!(
        owner
            .get_item(knife)
            .unwrap()
            .attributes
            .get_int_default("condition", 0),
        99
    );
    assert_eq!(owner.attributes.get_int_default("skill_woodworking", 0), 11);
    let views = resources::views(&ctx).unwrap();
    let stock = |id| views.iter().find(|v| v.access == id).unwrap().supplies["branches"];
    assert_eq!(stock(id), stock(canopy_id));
    assert_eq!(stock(neighbour_id), Some(12));
    let b = actor(&mut ctx, 1);
    let full_knife = item(&mut ctx, b, Some("prune"), false);
    let before = ctx.world.snapshot();
    assert!(
        resources::execute(
            &mut ctx,
            b,
            &request(id, "collect_branches", 2, Some(full_knife), None)
        )
        .is_err()
    );
    assert_eq!(
        serde_json::to_value(ctx.world.snapshot()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(
        ctx.find_entity(b)
            .unwrap()
            .get_item(full_knife)
            .unwrap()
            .attributes
            .get_int_default("condition", 0),
        100
    );
    ctx.map
        .entities
        .iter_mut()
        .find(|e| e.id == a)
        .unwrap()
        .position
        .y = 20.;
    assert!(
        resources::execute(
            &mut ctx,
            a,
            &request(id, "collect_branches", 2, Some(knife), None)
        )
        .is_err()
    );
}
#[test]
fn shared_well_serializes_competing_regions_and_vessels_preserve_identity() {
    let world = Arc::new(SimulationWorld::default());
    let mut workers = Vec::new();
    for _ in 0..2 {
        let mut ctx = context();
        ctx.world = world.clone();
        let well = object("finite_well", Some("village_well"));
        let access = well.id;
        ctx.map.geometry_objects.push(well);
        resources::initialize(&mut ctx, false).unwrap();
        let actor = actor(&mut ctx, 2);
        let vessel = item(&mut ctx, actor, None, true);
        workers.push(std::thread::spawn(move || {
            let before = ctx
                .find_entity(actor)
                .unwrap()
                .get_item(vessel)
                .unwrap()
                .creator_id;
            let result = resources::execute(
                &mut ctx,
                actor,
                &request(access, "fill_water", 4, None, Some(vessel)),
            );
            let after = ctx.find_entity(actor).unwrap().get_item(vessel).unwrap();
            assert_eq!((after.id, after.creator_id), (vessel, before));
            (
                result.is_ok(),
                after.attributes.get_int_default("liquid_quantity", 0),
            )
        }));
    }
    let results = workers
        .into_iter()
        .map(|w| w.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|r| r.0).count(), 1);
    assert_eq!(results.iter().map(|r| r.1).sum::<i32>(), 3);
}
#[test]
fn river_clamps_contents_rejects_incompatible_liquid_and_never_creates_a_water_stack() {
    let mut ctx = context();
    let river = object("river", Some("river"));
    let access = river.id;
    ctx.map.geometry_objects.push(river);
    resources::initialize(&mut ctx, false).unwrap();
    let a = actor(&mut ctx, 2);
    let bowl = item(&mut ctx, a, None, true);
    {
        let bowl = ctx.map.entities[0].get_item_mut(bowl).unwrap();
        bowl.set_attribute("liquid_item", Value::Str("fresh_water".into()));
        bowl.set_attribute("liquid_quantity", Value::Int(3));
    }
    let result = resources::execute(
        &mut ctx,
        a,
        &request(access, "fill_water", 4, None, Some(bowl)),
    )
    .unwrap();
    assert_eq!(result.stock_units, 1);
    assert_eq!(result.before, None);
    assert_eq!(quantity(&ctx, a, "fresh_water"), 0);
    assert!(
        resources::execute(
            &mut ctx,
            a,
            &request(access, "fill_water", 4, None, Some(bowl))
        )
        .is_err()
    );
    {
        let bowl = ctx.map.entities[0].get_item_mut(bowl).unwrap();
        bowl.set_attribute("liquid_item", Value::Str("salt_water".into()));
        bowl.set_attribute("liquid_quantity", Value::Int(1));
    }
    assert!(
        resources::execute(
            &mut ctx,
            a,
            &request(access, "fill_water", 4, None, Some(bowl))
        )
        .is_err()
    );
}
#[test]
fn death_remains_and_saved_source_tombstone_are_not_recreated_by_callbacks() {
    let mut ctx = context();
    let dead = actor(&mut ctx, 0);
    ctx.map.entities[0].set_attribute("mode", Value::Str("dead".into()));
    ctx.map.entities[0].set_attribute("remains_profile", Value::Str("skeleton_remains".into()));
    let mut existing = Item::default();
    existing.id = ctx.world.next_id();
    let existing_id = existing.id;
    let creator = existing.creator_id;
    existing.set_attribute("corpse_entity_id", Value::UInt(dead));
    existing.set_attribute("name", Value::Str("Authored Remains".into()));
    ctx.map.items.push(existing);
    let id = resources::create_remains(&mut ctx, dead).unwrap().unwrap();
    let count = ctx.map.items.len();
    assert_eq!(
        (ctx.map.items[0].id, ctx.map.items[0].creator_id),
        (existing_id, creator)
    );
    assert_eq!(
        ctx.map.items[0].attributes.get_str("name"),
        Some("Authored Remains")
    );

    assert_eq!(resources::create_remains(&mut ctx, dead).unwrap(), Some(id));
    assert_eq!(ctx.map.items.len(), count);
    let a = actor(&mut ctx, 2);
    resources::execute(&mut ctx, a, &request(id, "collect_bones", 1, None, None)).unwrap();
    assert_eq!(quantity(&ctx, a, "raw_bone"), 6);
    assert!(resources::execute(&mut ctx, a, &request(id, "collect_bones", 1, None, None)).is_err());
    ctx.map.items.clear();
    assert_eq!(resources::create_remains(&mut ctx, dead).unwrap(), Some(id));
    assert!(ctx.map.items.is_empty());
    let record = ctx.world.snapshot();
    let next = ctx.world.high_water();
    let world = SimulationWorld::default();
    world.restore(next, record);
    ctx.world = Arc::new(world);
    assert_eq!(resources::create_remains(&mut ctx, dead).unwrap(), Some(id));
    assert!(ctx.map.items.is_empty());
}

fn submit(
    game: &mut GameSession,
    connection: PlayerConnectionId,
    request: &ResourceRequest,
) -> Result<serde_json::Value, SaveError> {
    let replica = game.client_world(connection).unwrap();
    let region = replica.current_region().unwrap();
    let generation = replica.generation();
    let actions = game
        .prepare_game_request(
            connection,
            game.driver().session_id(),
            generation,
            region,
            &GameRequest::Resource(request.clone()),
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
#[test]
fn local_and_dedicated_requests_restore_depletion_clock_replay_receipts_and_views() {
    pollster::block_on(async {
        for purpose in [SavePurpose::SinglePlayer, SavePurpose::Dedicated] {
            let mut project = Project::new();
            project.config = "[game]\nstart_region='Resources'\nauto_create_player=false".into();
            project.regions[0].name = "Resources".into();
            project.regions[0].map.name = "Resources".into();
            let well = object("finite_well", Some("village_well"));
            let access = well.id;
            project.regions[0].map.geometry_objects.push(well);
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
            let context = session.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            let (actor_id, bowl);
            {
                let mut ctx = context.lock().unwrap();
                actor_id = actor(&mut ctx, 4);
                bowl = item(&mut ctx, actor_id, None, true);
            }
            session
                .game
                .server
                .attach_saved_character(connection, project.regions[0].map.id, actor_id)
                .unwrap();
            for _ in 0..3 {
                session.game.poll();
            }
            session.game.subscribe_replication(connection).unwrap();
            session.checkpoint().await.unwrap();
            let character = session
                .saved()
                .characters
                .iter()
                .find(|c| c.entity == actor_id)
                .unwrap()
                .id;
            let command = request(access, "fill_water", 4, None, Some(bowl));
            let fingerprint = "d".repeat(64);
            session
                .commit_sequenced_live(character, 1, &fingerprint, |g| {
                    submit(g, connection, &command)
                })
                .await
                .unwrap();
            let client = session
                .game
                .client_world(connection)
                .unwrap()
                .current_map()
                .unwrap();
            assert!(
                client
                    .geometry_objects
                    .iter()
                    .find(|object| object.id == access)
                    .expect("bound source in recipient map")
                    .properties
                    .get_str("resource_supplies")
                    .unwrap()
                    .contains("0")
            );
            session
                .commit_sequenced_live(character, 1, &fingerprint, |_| {
                    panic!("duplicate extraction")
                })
                .await
                .unwrap();
            session.checkpoint().await.unwrap();
            let store = session.store().clone();
            drop(session);
            let mut restored = DurableSession::open(
                project,
                store,
                purpose.clone(),
                SessionOptions {
                    create_local_player: false,
                    ..SessionOptions::terminal()
                },
            )
            .await
            .unwrap();
            let connection = if purpose == SavePurpose::SinglePlayer {
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
            let context = restored.game.server.instances[0]
                .lock()
                .unwrap()
                .context()
                .unwrap();
            {
                let mut ctx = context.lock().unwrap();
                assert_eq!(
                    resources::views(&ctx).unwrap()[0].supplies["water"],
                    Some(0)
                );
                let owned = ctx
                    .map
                    .entities
                    .iter_mut()
                    .find(|e| e.id == actor_id)
                    .unwrap()
                    .get_item_mut(bowl)
                    .unwrap();
                assert_eq!(owned.attributes.get_int_default("liquid_quantity", 0), 3);
                owned.set_attribute("liquid_quantity", Value::Int(0));
                ctx.world
                    .advance_resource_time(Duration::from_millis(43_200_000));
            }
            restored
                .commit_sequenced_live(character, 1, &fingerprint, |_| {
                    panic!("old receipt cannot refill after recovery")
                })
                .await
                .unwrap();
            {
                let ctx = context.lock().unwrap();
                assert_eq!(
                    ctx.find_entity(actor_id)
                        .unwrap()
                        .get_item(bowl)
                        .unwrap()
                        .attributes
                        .get_int_default("liquid_quantity", 0),
                    0
                );
                assert_eq!(
                    resources::views(&ctx).unwrap()[0].supplies["water"],
                    Some(3)
                );
            }
        }
    });
}

#[test]
fn initial_defaults_and_display_names_do_not_reset_live_sources_but_policy_edits_require_migration()
{
    let mut ctx = context();
    let object = object("mature_tree", None);
    ctx.map.geometry_objects.push(object);
    resources::initialize(&mut ctx, false).unwrap();
    let mut changed = ctx.rules.clone();
    changed["resource_profiles"]["mature_tree"]["name"] =
        toml::Value::String("Old Growth Tree".into());
    changed["resource_profiles"]["mature_tree"]["supplies"]["branches"]
        .as_table_mut()
        .unwrap()
        .insert("initial".into(), toml::Value::Integer(0));
    ctx.set_rules(changed).unwrap();
    assert_eq!(
        resources::views(&ctx).unwrap()[0].supplies["branches"],
        Some(12)
    );
    let mut changed = ctx.rules.clone();
    changed["resource_profiles"]["mature_tree"]["supplies"]["branches"]["capacity"] =
        toml::Value::Integer(20);
    assert!(ctx.set_rules(changed).unwrap_err().contains("migration"));
    assert_eq!(
        resources::views(&ctx).unwrap()[0].supplies["branches"],
        Some(12)
    );
}
