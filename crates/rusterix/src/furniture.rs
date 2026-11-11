//! Furniture use is an ordinary item intent. MZ connectors author pose origins.
use crate::{Assets, Entity, EntityAction, Item, Value, procedural_avatar::Runtime};
use vek::{Vec2, Vec3};

#[derive(Clone, Debug)]
pub struct Place {
    pub slot: String,
    pub posture: &'static str,
    pub position: Vec3<f32>,
    pub direction: Vec2<f32>,
}

pub fn places(item: &Item, assets: &Assets) -> Vec<Place> {
    let Some(mesh) = Runtime::resolve_item(item, assets) else {
        return vec![];
    };
    let yaw = item
        .attributes
        .get_float_default("appearance_yaw", 0.)
        .to_radians();
    let scale = item.attributes.get_float_default("appearance_scale", 1.);
    let rotate = |x: f32, z: f32| {
        Vec2::new(
            x * yaw.cos() + z * yaw.sin(),
            -x * yaw.sin() + z * yaw.cos(),
        )
    };
    mesh.connectors
        .iter()
        .filter_map(|c| {
            let posture = match c.family.as_str() {
                "rest.sit" => "sitting",
                "rest.lie" => "lying",
                _ => return None,
            };
            let p = rotate(c.position[0] * scale, c.position[2] * scale);
            let direction = rotate(c.direction[0], c.direction[2]).try_normalized()?;
            Some(Place {
                slot: c.id.clone(),
                posture,
                position: item.position + Vec3::new(p.x, c.position[1] * scale, p.y),
                direction,
            })
        })
        .collect()
}

pub fn is_resting(entity: &Entity) -> bool {
    matches!(
        entity.attributes.get_str("posture"),
        Some("sitting" | "lying")
    )
}

/// Returns to the approach position, so standing never leaves a player inside the furniture.
pub fn stand(entity: &mut Entity) {
    if !is_resting(entity) {
        return;
    }
    if let Some(Value::Vec3(p)) = entity.attributes.get("_furniture_return") {
        entity.position = Vec3::new(p[0], p[1], p[2]);
    }
    entity.set_attribute("posture", Value::Str("standing".into()));
    entity.set_attribute("resting", Value::Bool(false));
    entity.set_attribute("furniture_id", Value::Int64(-1));
    entity.set_attribute("furniture_slot", Value::Str(String::new()));
    entity.set_attribute("avatar_animation", Value::Str("Idle".into()));
    entity.set_attribute("avatar_pose_only", Value::Bool(false));
    entity.action = EntityAction::Off;
    entity.mark_all_dirty();
}

/// Camera-independent, exclusive occupancy. Intent permission/range is checked by the caller.
pub fn use_item(
    actor: &mut Entity,
    item: &Item,
    others: &[Entity],
    assets: &Assets,
) -> Option<Result<(), String>> {
    let places = places(item, assets);
    if places.is_empty() {
        return None;
    }
    if is_resting(actor) && actor.attributes.get("furniture_id") == Some(&Value::UInt(item.id)) {
        stand(actor);
        return Some(Ok(()));
    }
    let occupied = |slot: &str| {
        others.iter().any(|e| {
            e.id != actor.id
                && is_resting(e)
                && e.attributes.get("furniture_id") == Some(&Value::UInt(item.id))
                && e.attributes.get_str("furniture_slot") == Some(slot)
        })
    };
    let Some(place) = places
        .into_iter()
        .filter(|p| !occupied(&p.slot))
        .min_by(|a, b| {
            (a.position - actor.position)
                .magnitude_squared()
                .total_cmp(&(b.position - actor.position).magnitude_squared())
        })
    else {
        return Some(Err("Someone is already using this furniture.".into()));
    };
    if is_resting(actor) {
        stand(actor);
    }
    actor.set_attribute(
        "_furniture_return",
        Value::Vec3(actor.position.into_array()),
    );
    actor.position = place.position;
    actor.orientation = place.direction;
    actor.action = EntityAction::Off;
    actor.set_attribute("posture", Value::Str(place.posture.into()));
    actor.set_attribute("resting", Value::Bool(true));
    actor.set_attribute("avatar_pose_only", Value::Bool(place.posture == "lying"));
    actor.set_attribute("furniture_id", Value::UInt(item.id));
    actor.set_attribute("furniture_slot", Value::Str(place.slot));
    actor.set_attribute(
        "avatar_animation",
        Value::Str(
            if place.posture == "sitting" {
                "Sit"
            } else {
                "Lie"
            }
            .into(),
        ),
    );
    actor.mark_all_dirty();
    Some(Ok(()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    fn assets() -> Assets {
        let mut a = Assets::default();
        for (name, source) in [
            (
                "bed",
                include_str!("../../../creator/embedded/meshes/recipes/inn-bed.mzgeometry.json"),
            ),
            (
                "bench",
                include_str!("../../../creator/embedded/meshes/recipes/inn-bench.mzgeometry.json"),
            ),
        ] {
            a.procedural_avatars.insert(
                name.into(),
                Arc::new(crate::procedural_avatar::Appearance::new(source, &[], 1).unwrap()),
            );
        }
        a
    }
    fn item(name: &str) -> Item {
        let mut i = Item::new();
        i.id = 12;
        i.set_attribute("appearance_geometry", Value::Str(name.into()));
        i
    }
    #[test]
    fn furniture_use_is_exclusive_and_reversible_in_each_camera() {
        let a = assets();
        for (name, posture) in [("bed", "lying"), ("bench", "sitting")] {
            let i = item(name);
            let mut actor = Entity::default();
            actor.id = 1;
            actor.position = Vec3::new(1., 0., 1.);
            let start = actor.position;
            assert_eq!(use_item(&mut actor, &i, &[], &a), Some(Ok(())));
            assert_eq!(actor.attributes.get_str("posture"), Some(posture));
            assert_eq!(
                actor.attributes.get_bool_default("avatar_pose_only", false),
                posture == "lying"
            );
            let mut other = Entity::default();
            other.id = 2;
            assert!(
                use_item(&mut other, &i, &[actor.clone()], &a)
                    .unwrap()
                    .is_err()
            );
            assert_eq!(use_item(&mut actor, &i, &[], &a), Some(Ok(())));
            assert_eq!(actor.position, start);
            assert!(!is_resting(&actor));
            assert!(!actor.attributes.get_bool_default("avatar_pose_only", true));
            assert_eq!(actor.attributes.get_str("posture"), Some("standing"));
        }
    }
    #[test]
    fn furniture_pose_origins_follow_mz_geometry_world_rotation_and_scale() {
        let a = assets();
        let mut i = item("bed");
        i.position = Vec3::new(4., 2., 7.);
        i.set_attribute("appearance_yaw", Value::Float(90.));
        i.set_attribute("appearance_scale", Value::Float(2.));
        let p = places(&i, &a).remove(0);
        assert!((p.position.y - 3.2).abs() < 0.001);
        assert!((p.direction - Vec2::unit_x()).magnitude() < 0.001);
        assert_eq!(p.posture, "lying");
    }
    #[test]
    fn humanoid_rest_clips_are_distinct_editable_poses() {
        use denrim_materials::geometry::{Document, Value as GeometryValue};
        let d = Document::load(include_str!(
            "../../ruleset/rulesets/eldiron/v1/assets/humanoid.mzgeometry.json"
        ))
        .unwrap();
        let e = d.evaluate().unwrap();
        let standing = e.mesh.bounds();
        for name in ["sit", "lie"] {
            let GeometryValue::Motion(m) = &e.rows[name] else {
                panic!("Missing {name} motion")
            };
            let pose = e.mesh.posed(Some(m), m.duration).unwrap().mesh;
            let bounds = pose.bounds();
            assert!(bounds.1[1] < standing.1[1] - 0.2);
            if name == "lie" {
                // The standalone humanoid includes preview sword/shield parts;
                // their protruding equipment is not the body's lying thickness.
                let body = pose
                    .parts
                    .iter()
                    .filter(|p| !p.id.starts_with("sword/") && !p.id.starts_with("shield-"));
                let (lo, hi) = body
                    .flat_map(|p| &p.vertices)
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), v| {
                        (lo.min(v.position[1]), hi.max(v.position[1]))
                    });
                assert!(hi - lo < 0.5, "{name} body: {lo}..{hi}");
            }
        }
    }
}
