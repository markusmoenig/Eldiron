//! Reusable collision/movement kernel. Mutates only the supplied actor and
//! returns contacts; no VM, event delivery, clocks or server routing run here.
use crate::{
    Assets, Entity, Map, ValueContainer, collision_world::CollisionWorld, map::mini::MapMini,
};
use vek::{Vec2, Vec3};
mod controller;
pub use controller::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CollisionMode {
    Tile,
    Mesh,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Entity(u32),
    Item(u32),
}
#[derive(Debug)]
pub struct MovementResult {
    pub geometry_blocked: bool,
    pub dynamic_collision: bool,
    pub contacts: Vec<Contact>,
}
pub struct MovementWorld<'a> {
    pub diagnostics: Option<&'a std::cell::RefCell<crate::diagnostics::RegionDetail>>,
    pub map: &'a Map,
    pub mapmini: &'a MapMini,
    pub collision_world: &'a CollisionWorld,
    pub assets: &'a Assets,
    pub mode: CollisionMode,
}
pub(crate) fn dynamic_collision_height(container: &ValueContainer) -> f32 {
    container
        .get_float("collision_height")
        .or_else(|| container.get_float("entity_height"))
        .or_else(|| container.get_float("height"))
        .unwrap_or(2.0)
        .max(0.1)
}

pub(crate) fn vertical_collision_ranges_overlap(
    a_y: f32,
    a_height: f32,
    b_y: f32,
    b_height: f32,
) -> bool {
    const VERTICAL_COLLISION_TOLERANCE: f32 = 0.25;
    let a_min = a_y;
    let a_max = a_y + a_height;
    let b_min = b_y;
    let b_max = b_y + b_height;
    a_min <= b_max + VERTICAL_COLLISION_TOLERANCE && b_min <= a_max + VERTICAL_COLLISION_TOLERANCE
}

pub fn move_by(
    world: &MovementWorld<'_>,
    entity: &mut Entity,
    move_vector: Vec2<f32>,
    entity_block_mode: i32,
) -> MovementResult {
    let position = entity.get_pos_xz();
    let radius = entity.attributes.get_float_default("radius", 0.5) - 0.01;
    let entity_height = dynamic_collision_height(&entity.attributes);

    let mut new_position = position + move_vector;
    let mut dynamic_collision = false;
    let mut contacts = Vec::new();

    let started = world.diagnostics.map(|_| instant::Instant::now());
    let (mut entity_candidates, mut item_candidates) = (0u64, 0u64);
    const MAX_ITERATIONS: usize = 5;

    for _attempt in 0..MAX_ITERATIONS {
        let mut pushed = false;

        for other in world.map.entities.iter() {
            entity_candidates += 1;
            if other.id == entity.id {
                continue;
            }

            let other_pos = other.get_pos_xz();
            let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
            let combined_radius = radius + other_radius;
            let combined_radius_sq = combined_radius * combined_radius;

            let dist_vec = new_position - other_pos;
            let dist_sq = dist_vec.magnitude_squared();
            // Most actors are far away. Reject by distance before allocating
            // mode strings or reading height attributes; retain map/contact order.
            if !(dist_sq < combined_radius_sq) || other.attributes.get_str("mode") == Some("dead") {
                continue;
            }
            let other_height = dynamic_collision_height(&other.attributes);
            if !vertical_collision_ranges_overlap(
                entity.position.y,
                entity_height,
                other.position.y,
                other_height,
            ) {
                continue;
            }
            {
                dynamic_collision = true;
                contacts.push(Contact::Entity(other.id));

                if entity_block_mode > 0 {
                    let normal = dist_vec.normalized();
                    let total_move = new_position - position;
                    let slide = total_move - normal * total_move.dot(normal);
                    let slide_pos = position + slide;
                    let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                    if slide_dist_sq >= combined_radius_sq {
                        new_position = slide_pos;
                    } else {
                        let actual_dist = (slide_pos - other_pos).magnitude();
                        if actual_dist < combined_radius {
                            let push_amount = combined_radius - actual_dist;
                            new_position = slide_pos + normal * push_amount;
                        }
                    }
                    pushed = true;
                }
            }
        }

        for other in world.map.items.iter() {
            item_candidates += 1;
            if !other.attributes.get_bool_default("visible", false) {
                continue;
            }

            if let Some(hit) = crate::item_state::intersects(
                other,
                world.assets,
                new_position,
                entity.position.y,
                entity_height,
                radius,
            ) {
                if hit && other.attributes.get_bool_default("blocking", false) {
                    // Try each movement axis before stopping. The same footprint is
                    // used for mesh and tile movement; opening never leaves a circle behind.
                    let slide_x = Vec2::new(new_position.x, position.y);
                    let slide_z = Vec2::new(position.x, new_position.y);
                    new_position = [slide_x, slide_z]
                        .into_iter()
                        .find(|p| {
                            crate::item_state::intersects(
                                other,
                                world.assets,
                                *p,
                                entity.position.y,
                                entity_height,
                                radius,
                            ) == Some(false)
                        })
                        .unwrap_or(position);
                    dynamic_collision = true;
                    pushed = true;
                }
                continue;
            }

            let other_pos = other.get_pos_xz();
            let other_radius = other.attributes.get_float_default("radius", 0.5) - 0.01;
            let combined_radius = radius + other_radius;
            let combined_radius_sq = combined_radius * combined_radius;

            let dist_vec = new_position - other_pos;
            let dist_sq = dist_vec.magnitude_squared();
            if dist_sq < combined_radius_sq {
                dynamic_collision = true;
                contacts.push(Contact::Item(other.id));

                if other.attributes.get_bool_default("blocking", false) {
                    let normal = dist_vec.normalized();
                    let total_move = new_position - position;
                    let slide = total_move - normal * total_move.dot(normal);
                    let slide_pos = position + slide;
                    let slide_dist_sq = (slide_pos - other_pos).magnitude_squared();

                    if slide_dist_sq >= combined_radius_sq {
                        new_position = slide_pos;
                    } else {
                        let actual_dist = (slide_pos - other_pos).magnitude();
                        if actual_dist < combined_radius {
                            let push_amount = combined_radius - actual_dist;
                            new_position = slide_pos + normal * push_amount;
                        }
                    }
                    pushed = true;
                }
            }
        }

        if !pushed {
            break;
        }
    }

    if let (Some(started), Some(details)) = (started, world.diagnostics) {
        let mut d = details.borrow_mut();
        d.dynamic_collision.record(started.elapsed());
        d.entity_candidates = d.entity_candidates.saturating_add(entity_candidates);
        d.item_candidates = d.item_candidates.saturating_add(item_candidates);
    }
    let started = world.diagnostics.map(|_| instant::Instant::now());
    entity.set_pos_xz(new_position);

    let blocked = match world.mode {
        CollisionMode::Tile => {
            let (end_position, geometry_blocked) =
                world
                    .mapmini
                    .move_distance(position, new_position - position, radius);
            if !world.mapmini.elevation.is_empty() {
                let (end, height_blocked) = world.mapmini.elevation.move_distance(
                    Vec3::new(position.x, entity.position.y, position.y),
                    end_position - position,
                    radius.max(0.),
                    entity
                        .attributes
                        .get_float_default("max_step_height", crate::isometric::elevation::STEP_UP),
                    entity.attributes.get_float_default(
                        "max_drop_height",
                        crate::isometric::elevation::STEP_DOWN,
                    ),
                );
                entity.position = end;
                geometry_blocked || height_blocked
            } else {
                entity.set_pos_xz(end_position);
                geometry_blocked
            }
        }
        CollisionMode::Mesh => {
            if world.collision_world.has_collision_data() {
                let max_step_height = entity
                    .attributes
                    .get_float_default("max_step_height", 1.0)
                    .max(0.01);
                let move_vec = new_position - position;
                let desired_dist = move_vec.magnitude();
                if desired_dist > 1e-6 {
                    if let Some((end_pos, arrived)) =
                        world.collision_world.move_towards_on_floors_direct(
                            position,
                            new_position,
                            desired_dist,
                            radius,
                            max_step_height,
                            entity.position.y,
                        )
                    {
                        entity.set_pos_xz(vek::Vec2::new(end_pos.x, end_pos.z));
                        entity.position.y = end_pos.y;
                        !arrived
                    } else {
                        entity.set_pos_xz(position);
                        true
                    }
                } else {
                    false
                }
            } else {
                let (end_position, geometry_blocked) =
                    world
                        .mapmini
                        .move_distance(position, new_position - position, radius);
                entity.set_pos_xz(end_position);
                geometry_blocked
            }
        }
    };

    let final_pos = entity.get_pos_xz();
    let base_y = if !world.mapmini.elevation.is_empty() {
        Some(world.mapmini.elevation.height_at(final_pos))
    } else if world.mode == CollisionMode::Mesh && world.collision_world.has_collision_data() {
        let max_step_height = entity
            .attributes
            .get_float_default("max_step_height", 1.0)
            .max(0.01);
        world
            .collision_world
            .sample_reachable_floor_height(
                final_pos,
                radius * 0.5,
                entity.position.y,
                max_step_height,
            )
            .or_else(|| {
                world.collision_world.get_floor_height_reachable(
                    final_pos,
                    entity.position.y,
                    max_step_height,
                )
            })
    } else {
        let config = crate::chunkbuilder::terrain_generator::TerrainConfig::for_map(world.map);
        Some(
            crate::chunkbuilder::terrain_generator::TerrainGenerator::sample_height_at(
                world.map, final_pos, &config,
            ),
        )
    };

    if let Some(y) = base_y {
        entity.position.y = y;
    }

    if let (Some(started), Some(details)) = (started, world.diagnostics) {
        details
            .borrow_mut()
            .geometry_collision
            .record(started.elapsed());
    }
    MovementResult {
        geometry_blocked: blocked,
        dynamic_collision,
        contacts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Value,
        collision_world::{ChunkCollision, WalkableFloor},
    };

    #[test]
    fn tile_collision_replay_returns_contacts_without_mutating_the_world() {
        let mut map = Map::default();
        let mut other = Entity::default();
        other.id = 2;
        other.position = Vec3::new(1.5, 0., 0.5);
        map.entities.push(other);
        let mut mini = MapMini::default();
        mini.blocked_tiles.insert(Vec2::new(2, 0));
        let collision = CollisionWorld::new(10);
        let assets = Assets::default();
        let world = MovementWorld {
            diagnostics: None,
            map: &map,
            mapmini: &mini,
            collision_world: &collision,
            assets: &assets,
            mode: CollisionMode::Tile,
        };
        let mut actor = Entity::default();
        actor.id = 1;
        actor.position = Vec3::new(0.5, 0., 0.5);
        let mut replay = actor.clone();
        let first = move_by(&world, &mut actor, Vec2::new(0.75, 0.), 1);
        let second = move_by(&world, &mut replay, Vec2::new(0.75, 0.), 1);
        assert!(first.dynamic_collision);
        assert!(first.contacts.contains(&Contact::Entity(2)));
        assert_eq!(first.contacts, second.contacts);
        assert_eq!(actor.position, replay.position);
        assert_eq!(map.entities[0].position, Vec3::new(1.5, 0., 0.5));
        let blocked = move_by(&world, &mut actor, Vec2::new(4., 0.), 0);
        assert!(blocked.geometry_blocked);
        assert!(actor.position.x < 2.);
    }

    #[test]
    fn mesh_step_preserves_reachable_floor_height_and_facing() {
        let map = Map::default();
        let mini = MapMini::default();
        let assets = Assets::default();
        let mut collision = CollisionWorld::new(10);
        let mut chunk = ChunkCollision::new();
        for (start, end, height) in [(0., 2., 0.), (2., 4., 0.5)] {
            chunk.walkable_floors.push(WalkableFloor::flat(
                scenevm::GeoId::Sector(1),
                height,
                vec![
                    Vec2::new(0., start),
                    Vec2::new(4., start),
                    Vec2::new(4., end),
                    Vec2::new(0., end),
                ],
            ));
        }
        collision.update_chunk(Vec2::zero(), chunk);
        let world = MovementWorld {
            diagnostics: None,
            map: &map,
            mapmini: &mini,
            collision_world: &collision,
            assets: &assets,
            mode: CollisionMode::Mesh,
        };
        let mut actor = Entity::default();
        actor.id = 1;
        actor.position = Vec3::new(2., 0., 1.5);
        actor.attributes.set("radius", Value::Float(0.1));
        actor.attributes.set("max_step_height", Value::Float(0.6));
        let facing = actor.orientation;
        for _ in 0..20 {
            move_by(&world, &mut actor, Vec2::new(0., 0.08), 1);
        }
        assert!(
            (actor.position.z - 3.1).abs() < 1e-4,
            "{:?}",
            actor.position
        );
        assert!((actor.position.y - 0.5).abs() < 1e-4);
        assert_eq!(actor.orientation, facing);
    }
}

#[cfg(test)]
mod contact_regression {
    use super::*;
    use crate::{Value, diagnostics::RegionDetail};
    #[test]
    fn distant_dead_and_vertical_actors_do_not_change_ordered_contacts() {
        let mut map = Map::default();
        for id in 2..=1005 {
            let mut entity = Entity::default();
            entity.id = id;
            entity.position = Vec3::new(100. + id as f32, 0., 0.5);
            if id % 2 == 0 {
                entity.set_attribute("mode", Value::Str("dead".into()));
            }
            map.entities.push(entity);
        }
        for (index, y, dead) in [
            (0, 0., false),
            (1, 10., false),
            (2, 0., true),
            (3, 0., false),
        ] {
            let e = &mut map.entities[index];
            e.position = Vec3::new(1., y, 0.5);
            e.set_attribute(
                "mode",
                Value::Str(if dead { "dead" } else { "active" }.into()),
            );
        }
        let mut actor = Entity::default();
        actor.id = 1;
        actor.position = Vec3::new(0., 0., 0.5);
        let mini = MapMini::default();
        let collision = CollisionWorld::new(10);
        let assets = Assets::default();
        let d = std::cell::RefCell::new(RegionDetail::default());
        let result = move_by(
            &MovementWorld {
                map: &map,
                mapmini: &mini,
                collision_world: &collision,
                assets: &assets,
                mode: CollisionMode::Tile,
                diagnostics: Some(&d),
            },
            &mut actor,
            Vec2::new(0.8, 0.),
            0,
        );
        assert_eq!(
            result.contacts,
            vec![Contact::Entity(2), Contact::Entity(5)]
        );
        assert!((actor.position.x - 0.8).abs() < 1e-5);
        assert_eq!(d.borrow().entity_candidates, 1004);
        assert_eq!(d.borrow().dynamic_collision.summary().calls, 1);
        assert_eq!(d.borrow().geometry_collision.summary().calls, 1);
    }
}
