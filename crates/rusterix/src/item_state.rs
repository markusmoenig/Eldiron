//! Shared item states and lock policy. Cameras only render the selected recipe output.
use crate::{Assets, Entity, Item, Value, procedural_avatar::Runtime};
use denrim_materials::geometry::footprint;
use vek::{Vec2, Vec3};

pub fn canonical_intent(item: &Item, intent: &str) -> String {
    if intent.eq_ignore_ascii_case("open_close") {
        if item.attributes.get_bool_default("active", false) {
            "close"
        } else {
            "open"
        }
        .into()
    } else {
        intent.into()
    }
}

fn key_matches(item: &Item, reference: &str) -> bool {
    item.creator_id.to_string().eq_ignore_ascii_case(reference)
        || item.item_type.eq_ignore_ascii_case(reference)
        || [
            "creator_template_id",
            "ruleset_id",
            "ruleset_path",
            "source_id",
            "class_name",
            "name",
        ]
        .iter()
        .any(|key| {
            item.attributes
                .get_str(key)
                .is_some_and(|v| v.eq_ignore_ascii_case(reference))
        })
        || item
            .container
            .as_ref()
            .is_some_and(|items| items.iter().any(|i| key_matches(i, reference)))
}
pub fn has_key(actor: &Entity, reference: &str) -> bool {
    !reference.trim().is_empty()
        && actor
            .iter_inventory()
            .any(|(_, i)| key_matches(i, reference.trim()))
}

/// Returns None for ordinary items, leaving their existing behavior untouched.
/// A matching key unlocks once; the key is retained. Node graphs may author other policies.
pub fn request(item: &mut Item, actor: &Entity, intent: &str) -> Option<Result<bool, String>> {
    if !item.attributes.get_bool_default("openable", false) {
        return None;
    }
    let intent = canonical_intent(item, intent);
    let active = match intent.as_str() {
        "open" => true,
        "close" => false,
        _ => return None,
    };
    if item.attributes.get_bool_default("active", false) == active {
        return Some(Ok(false));
    }
    let key = item
        .attributes
        .get_str("required_key")
        .unwrap_or("")
        .trim()
        .to_string();
    if active && item.attributes.get_bool_default("locked", !key.is_empty()) {
        let found = has_key(actor, &key);
        if !found {
            return Some(Err(item
                .attributes
                .get_str("locked_message")
                .unwrap_or("{system.door_locked}")
                .into()));
        }
        item.set_attribute("locked", Value::Bool(false));
    }
    item.set_attribute("active", Value::Bool(active));
    Some(Ok(true))
}

/// Exact authored footprint for both iso and 3D movement; None retains legacy item circles.
pub fn intersects(
    item: &Item,
    assets: &Assets,
    position: Vec2<f32>,
    y: f32,
    height: f32,
    radius: f32,
) -> Option<bool> {
    let model = Runtime::resolve_item(item, assets)?;
    let shape = model.footprint.as_ref()?;
    let scale = item
        .attributes
        .get_float_default("appearance_scale", 1.)
        .max(0.001);
    let floor = model.bounds.0[1];
    let top = item.position.y + (model.bounds.1[1] - floor) * scale;
    if y + height <= item.position.y + 0.001 || y >= top - 0.001 {
        return Some(false);
    }
    let yaw = item
        .attributes
        .get_float_default("appearance_yaw", 0.)
        .to_radians();
    let (s, c) = yaw.sin_cos();
    let p = (position - item.get_pos_xz()) / scale;
    let p = [c * p.x - s * p.y, s * p.x + c * p.y];
    let radius = radius / scale;
    let shape = shape.map(|p| [p[0], p[2]]);
    if footprint::contains(&shape, p) {
        return Some(true);
    }
    for ring in shape
        .regions
        .iter()
        .flat_map(|r| std::iter::once(&r.outer).chain(r.holes.iter()))
    {
        for (a, b) in ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
        {
            let a = Vec2::<f32>::from(*a);
            let b = Vec2::<f32>::from(*b);
            let p = Vec2::<f32>::from(p);
            let d = b - a;
            let t = ((p - a).dot(d) / d.magnitude_squared().max(1e-12)).clamp(0., 1.);
            if (p - (a + d * t)).magnitude_squared() < radius * radius {
                return Some(true);
            }
        }
    }
    Some(false)
}

pub fn actor_intersects(item: &Item, assets: &Assets, actor: &Entity) -> bool {
    intersects(
        item,
        assets,
        actor.get_pos_xz(),
        actor.position.y,
        actor.attributes.get_float_default("height", 1.8),
        actor.attributes.get_float_default("radius", 0.5),
    )
    .unwrap_or_else(|| {
        item.get_pos_xz().distance(actor.get_pos_xz())
            < actor.attributes.get_float_default("radius", 0.5)
    })
}

/// Interaction reach is measured to the leaf rather than its hinge/pivot.
pub fn distance(item: &Item, assets: &Assets, actor: &Entity) -> f32 {
    let radius = actor.attributes.get_float_default("radius", 0.5).max(0.);
    if let Some(shape) = Runtime::resolve_item(item, assets).and_then(|m| m.footprint.as_ref()) {
        let shape = shape.map(|p| {
            let p = world_point(item, Vec3::from(*p));
            [p.x, p.z]
        });
        let p = actor.get_pos_xz();
        if footprint::contains(&shape, [p.x, p.y]) {
            return 0.;
        }
        let mut distance = f32::INFINITY;
        for ring in shape
            .regions
            .iter()
            .flat_map(|r| std::iter::once(&r.outer).chain(r.holes.iter()))
        {
            for (a, b) in ring
                .iter()
                .zip(ring.iter().cycle().skip(1))
                .take(ring.len())
            {
                let a = Vec2::<f32>::from(*a);
                let d = Vec2::<f32>::from(*b) - a;
                let t = ((p - a).dot(d) / d.magnitude_squared().max(1e-12)).clamp(0., 1.);
                distance = distance.min((p - a - d * t).magnitude());
            }
        }
        return (distance - radius).max(0.);
    }
    (actor.get_pos_xz().distance(item.get_pos_xz())
        - radius
        - item.attributes.get_float_default("radius", 0.5))
    .max(0.)
}

/// Shared presentation transform, also useful to author/debug a hinge in world space.
pub fn world_point(item: &Item, local: Vec3<f32>) -> Vec3<f32> {
    let yaw = item
        .attributes
        .get_float_default("appearance_yaw", 0.)
        .to_radians();
    let (s, c) = yaw.sin_cos();
    item.position
        + Vec3::new(
            c * local.x + s * local.z,
            local.y,
            -s * local.x + c * local.z,
        ) * item.attributes.get_float_default("appearance_scale", 1.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use theframework::prelude::Uuid;
    fn door() -> Item {
        let mut item = Item::new();
        item.set_attribute("openable", Value::Bool(true));
        item.set_attribute("active", Value::Bool(false));
        item.set_attribute("appearance_geometry", Value::Str("door".into()));
        item
    }
    fn assets() -> Assets {
        let mut assets = Assets::default();
        let source =
            include_str!("../../../creator/embedded/meshes/recipes/hinged-door.mzgeometry.json");
        let model = crate::procedural_avatar::Appearance::new(source, &[], 7).unwrap();
        assets
            .procedural_avatars
            .insert("door".into(), Arc::new(model));
        assets
    }
    #[test]
    fn item_state_open_is_idempotent_and_instances_are_independent() {
        let actor = Entity::default();
        let mut a = door();
        let b = door();
        assert_eq!(request(&mut a, &actor, "open"), Some(Ok(true)));
        assert_eq!(request(&mut a, &actor, "open"), Some(Ok(false)));
        assert!(!b.attributes.get_bool_default("active", true));
        assert_eq!(canonical_intent(&a, "open_close"), "close");
        assert_eq!(request(&mut a, &actor, "open_close"), Some(Ok(true)));
        assert!(!a.attributes.get_bool_default("active", true));
    }
    #[test]
    fn item_state_keys_use_stable_template_references_and_unlock_without_consuming() {
        let mut actor = Entity::default();
        actor.inventory = vec![None; 8];
        let mut a = door();
        let key = Uuid::new_v4().to_string();
        a.set_attribute("locked", Value::Bool(true));
        a.set_attribute("required_key", Value::Str(key.clone()));
        assert!(request(&mut a, &actor, "open").unwrap().is_err());
        let mut matching = Item::new();
        matching.set_attribute("creator_template_id", Value::Str(key));
        let mut bag = Item::new();
        bag.container = Some(vec![matching]);
        actor.add_item(bag).unwrap();
        assert_eq!(request(&mut a, &actor, "open"), Some(Ok(true)));
        assert!(!a.attributes.get_bool_default("locked", true));
        assert_eq!(actor.iter_inventory().count(), 1);
        request(&mut a, &actor, "close");
        assert_eq!(request(&mut a, &Entity::default(), "open"), Some(Ok(true)));
    }
    #[test]
    fn item_state_node_changes_select_the_same_mesh_and_footprint_in_both_cameras() {
        let assets = assets();
        let mut item = door();
        let mut runtime = Runtime::default();
        let off = Runtime::resolve_item(&item, &assets).unwrap().clone();
        assert_eq!(
            intersects(&item, &assets, Vec2::new(0.8, 0.), 0., 1.8, 0.15),
            Some(true)
        );
        // The existing Set Attribute node writes this exact flag; there is no camera state.
        item.set_attribute("active", Value::Bool(true));
        let on = Runtime::resolve_item(&item, &assets).unwrap();
        assert!(!Arc::ptr_eq(&off, on));
        assert_eq!(
            intersects(&item, &assets, Vec2::new(0.8, 0.), 0., 1.8, 0.15),
            Some(false)
        );
        assert_eq!(
            intersects(&item, &assets, Vec2::new(0., -0.8), 0., 1.8, 0.15),
            Some(false)
        );
        assert_eq!(
            intersects(&item, &assets, Vec2::new(-0.8, 0.35), 0., 1.8, 0.15),
            Some(true)
        );
        // World yaw, elevation and scale transform the same footprint used by the native mesh.
        item.position = Vec3::new(4., 2., 7.);
        item.set_attribute("appearance_yaw", Value::Float(90.));
        item.set_attribute("appearance_scale", Value::Float(2.));
        let point = world_point(&item, Vec3::new(-0.8, 0., 0.35));
        assert_eq!(
            intersects(&item, &assets, Vec2::new(point.x, point.z), 2., 1.8, 0.15),
            Some(true)
        );
        assert_eq!(
            intersects(&item, &assets, Vec2::new(point.x, point.z), 7., 1.8, 0.15),
            Some(false)
        );
        let sprite = runtime.isometric_item_sprite(&item, &assets).unwrap();
        assert!(sprite.pixels.chunks_exact(4).any(|p| p[3] > 0));
    }
}
