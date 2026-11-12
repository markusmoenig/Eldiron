//! CPU world target resolution and click policy. View/camera/pointer ownership,
//! picking services and panel toggles belong to presentation; actions go to the
//! authoritative session. This module owns no client, renderer or pixel buffer.
use crate::{
    Assets, BlockPropInteractionHit, EntityAction, Item, Map, Value, block_prop_interaction_verb,
    party::resolve_party_entity, resolve_block_prop_interaction_hit,
};
use scenevm::GeoId;
use theframework_core::Uuid;
use vek::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldHit {
    pub region: Uuid,
    pub entity: Option<u32>,
    pub item: Option<u32>,
    pub prop: Option<BlockPropInteractionHit>,
    pub world: Option<Vec3<f32>>,
    pub distance: f32,
    /// Flat cell views retain their terrain-click and item-click semantics.
    pub cell: Option<Vec2<f32>>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum WorldClick {
    Action {
        action: EntityAction,
        uses_intent: bool,
    },
    BeginItemDrag(u32),
    /// Presentation decides whether this means opening or closing its panel.
    OpenContainer(u32),
}
pub fn item_is_container(item: &Item) -> bool {
    item.is_container()
        || item.attributes.get_bool_default("container", false)
        || item.attributes.get_int_default("container_slots", 0) > 0
}
fn leader_position(map: &Map) -> Vec2<f32> {
    resolve_party_entity(map, Some("leader"))
        .or_else(|| map.entities.iter().find(|e| e.is_player()))
        .map(|e| e.get_pos_xz())
        .unwrap_or(Vec2::zero())
}
pub fn item_click_distance(map: &Map, item: u32) -> f32 {
    let Some(leader) = resolve_party_entity(map, Some("leader"))
        .or_else(|| map.entities.iter().find(|e| e.is_player()))
    else {
        return 0.;
    };
    map.items
        .iter()
        .find(|i| i.id == item)
        .map(|i| leader.get_pos_xz().distance(i.get_pos_xz()))
        .unwrap_or(0.)
}
/// Ground conversion is supplied by the displayed view, not reconstructed here.
/// Prefer living entities, then the topmost item, then dead entities. A world
/// item can precede entities when a drag destination exists and no intent is set.
pub fn cell_hit(map: &Map, ground: Vec2<f32>, prefer_item: bool) -> Option<WorldHit> {
    if !ground.x.is_finite() || !ground.y.is_finite() {
        return None;
    }
    let cell = ground.map(f32::floor);
    let item = map
        .items
        .iter()
        .rev()
        .find(|i| i.get_pos_xz().map(f32::floor) == cell);
    let living = map.entities.iter().find(|e| {
        e.get_pos_xz().map(f32::floor) == cell
            && e.attributes.get_str_default("mode", "active".into()) != "dead"
    });
    let entity = living.or_else(|| {
        map.entities
            .iter()
            .find(|e| e.get_pos_xz().map(f32::floor) == cell)
    });
    let mut hit = WorldHit {
        region: map.id,
        cell: Some(cell),
        world: Some(Vec3::new(ground.x, 0., ground.y)),
        ..Default::default()
    };
    if (prefer_item || living.is_none()) && item.is_some() {
        let item = item.unwrap();
        hit.item = Some(item.id);
        hit.distance = leader_position(map).distance(item.get_pos_xz());
    } else if let Some(entity) = entity {
        hit.entity = Some(entity.id);
        hit.distance = leader_position(map).distance(entity.get_pos_xz());
    }
    Some(hit)
}
pub fn find_item_by_profile_attrs(
    map: &Map,
    host: Option<u32>,
    profile: Option<u32>,
) -> Option<&Item> {
    let (host, profile) = (host?, profile?);
    map.items.iter().find(|item| matches!(item.attributes.get("profile_host_sector_id"), Some(Value::UInt(v)) if *v == host)
        && matches!(item.attributes.get("profile_sector_id"), Some(Value::UInt(v)) if *v == profile))
}
pub fn find_item_by_geometry_object_id(map: &Map, object: Uuid) -> Option<&Item> {
    map.items
        .iter()
        .find(|i| i.attributes.get_id("geometry_object_id") == Some(object))
}
pub fn find_item_by_sector_id(map: &Map, sector: u32) -> Option<&Item> {
    map.items
        .iter()
        .find(|i| match i.attributes.get("sector_id") {
            Some(Value::UInt(v)) => *v == sector,
            Some(Value::Int(v)) if *v >= 0 => *v as u32 == sector,
            Some(Value::Int64(v)) if *v >= 0 => *v as u32 == sector,
            _ => false,
        })
        .or_else(|| {
            let group = map
                .find_sector(sector)?
                .properties
                .get_id("door_group_id")?;
            map.items
                .iter()
                .find(|i| i.attributes.get_id("door_group_id") == Some(group))
        })
}
/// Translate rendered identities into runtime identities without constructing a
/// SceneHandler. A linked prop can also have an item binding; prop verbs win.
pub fn geometry_hit(
    map: &Map,
    assets: &Assets,
    geo: GeoId,
    world: Vec3<f32>,
    distance: f32,
    paint: Option<[u32; 4]>,
) -> Option<WorldHit> {
    if !world.iter().all(|v| v.is_finite()) || !distance.is_finite() || distance < 0. {
        return None;
    }
    let mut hit = WorldHit {
        region: map.id,
        world: Some(world),
        distance,
        ..Default::default()
    };
    match geo {
        GeoId::Character(id) => hit.entity = Some(id),
        GeoId::Item(id) => hit.item = Some(id),
        GeoId::Hole(host, profile) => {
            hit.item = find_item_by_profile_attrs(map, Some(host), Some(profile)).map(|i| i.id)
        }
        GeoId::Sector(id) => hit.item = find_item_by_sector_id(map, id).map(|i| i.id),
        GeoId::GeometryObject(id) => {
            hit.item = find_item_by_geometry_object_id(map, id).map(|i| i.id);
            hit.prop = resolve_block_prop_interaction_hit(
                &map.block_prop_instances,
                &assets.block_props,
                id,
                paint,
            );
        }
        _ => {}
    }
    Some(hit)
}
pub fn click(
    map: &Map,
    assets: &Assets,
    hit: WorldHit,
    intent: Option<&str>,
    can_drag: bool,
) -> Option<WorldClick> {
    if hit.region != map.id || !hit.distance.is_finite() || hit.distance < 0. {
        return None;
    }
    let intent = intent.filter(|s| !s.trim().is_empty());
    if let Some(prop) = hit.prop {
        let instance = map
            .block_prop_instances
            .iter()
            .find(|i| i.id == prop.instance_id)?;
        if instance.asset_id != prop.asset_id {
            return None;
        }
        let asset = assets.block_props.get(&prop.asset_id)?;
        if !asset.parts.iter().any(|part| part.id == prop.part_id)
            || prop.target_id.is_some_and(|target| {
                asset
                    .find_interaction_target(target)
                    .is_none_or(|current| current.part_id != prop.part_id)
            })
        {
            return None;
        }
        let verb = intent.map(str::trim).or_else(|| {
            prop.target_id
                .and_then(|target| block_prop_interaction_verb(asset, instance, target))
        });
        if let Some(verb) = verb {
            return Some(WorldClick::Action {
                action: EntityAction::BlockPropInteract {
                    instance_id: prop.instance_id,
                    part_id: prop.part_id,
                    target_id: prop.target_id,
                    verb: verb.into(),
                    explicit: intent.is_some(),
                },
                uses_intent: intent.is_some(),
            });
        }
    }
    if let Some(id) = hit.entity {
        map.entities.iter().find(|e| e.id == id)?;
        return Some(WorldClick::Action {
            action: EntityAction::EntityClicked(id, hit.distance, intent.map(str::to_owned)),
            uses_intent: intent.is_some(),
        });
    }
    if let Some(id) = hit.item {
        let item = map.items.iter().find(|i| i.id == id)?;
        if can_drag && intent.is_none() {
            return Some(WorldClick::BeginItemDrag(id));
        }
        if hit.cell.is_none() && intent.is_none() && item_is_container(item) {
            return Some(WorldClick::OpenContainer(id));
        }
        return Some(WorldClick::Action {
            action: EntityAction::ItemClicked(
                id,
                if hit.cell.is_some() {
                    hit.distance
                } else {
                    item_click_distance(map, id)
                },
                intent.map(str::to_owned),
                None,
            ),
            uses_intent: intent.is_some(),
        });
    }
    hit.cell.map(|cell| WorldClick::Action {
        action: EntityAction::TerrainClicked(cell),
        uses_intent: false,
    })
}

#[cfg(test)]
#[path = "world_interaction_tests.rs"]
mod tests;
