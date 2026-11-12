use super::*;
use crate::{
    BlockPropAsset, BlockPropComponent, BlockPropInstance, BlockPropInteractionTarget,
    BlockPropSemanticShape, Entity, GeometryObject, block_prop_instance_object_id,
    set_block_prop_door_open,
};
fn entity(id: u32, x: f32, z: f32, dead: bool) -> Entity {
    let mut e = Entity::new();
    e.id = id;
    e.position = Vec3::new(x, 0., z);
    if dead {
        e.set_attribute("mode", Value::Str("dead".into()));
    }
    e
}
fn item(id: u32, x: f32, z: f32) -> Item {
    let mut i = Item::new();
    i.id = id;
    i.position = Vec3::new(x, 0., z);
    i
}
#[test]
fn flat_cell_priority_matches_rendered_stack_and_drag_intent_policy_in_negative_cells() {
    let mut map = Map::default();
    map.entities = vec![entity(7, -0.2, 3.4, true), entity(8, -0.4, 3.2, false)];
    map.items = vec![item(9, -0.3, 3.3), item(10, -0.3, 3.3)];
    let point = Vec2::new(-0.1, 3.1);
    let hit = cell_hit(&map, point, false).unwrap();
    assert_eq!(hit.cell, Some(Vec2::new(-1., 3.)));
    assert_eq!(hit.entity, Some(8));
    assert_eq!(cell_hit(&map, point, true).unwrap().item, Some(10));
    map.entities[1].set_attribute("mode", Value::Str("dead".into()));
    assert_eq!(cell_hit(&map, point, false).unwrap().item, Some(10));
    map.items.clear();
    assert_eq!(cell_hit(&map, point, false).unwrap().entity, Some(7));
    assert!(cell_hit(&map, Vec2::new(f32::NAN, 3.), false).is_none());
}
#[test]
fn flat_and_geometry_clicks_preserve_container_drag_distance_and_intent_semantics() {
    let mut map = Map::default();
    let mut leader = entity(1, 0., 0., false);
    leader.set_attribute("party_index", Value::Int(0));
    map.entities.push(leader);
    let mut bag = item(7, 3., 4.);
    bag.set_attribute("container_slots", Value::Int(4));
    map.items.push(bag);
    let assets = Assets::default();
    let cell = cell_hit(&map, Vec2::new(3.1, 4.2), false).unwrap();
    assert_eq!(
        click(&map, &assets, cell, None, false),
        Some(WorldClick::Action {
            action: EntityAction::ItemClicked(7, 5., None, None),
            uses_intent: false
        })
    );
    let ray = geometry_hit(
        &map,
        &assets,
        GeoId::Item(7),
        Vec3::new(3., 1., 4.),
        123.,
        None,
    )
    .unwrap();
    assert_eq!(
        click(&map, &assets, ray, None, false),
        Some(WorldClick::OpenContainer(7))
    );
    assert_eq!(
        click(&map, &assets, ray, None, true),
        Some(WorldClick::BeginItemDrag(7))
    );
    assert_eq!(
        click(&map, &assets, ray, Some("look"), true),
        Some(WorldClick::Action {
            action: EntityAction::ItemClicked(7, 5., Some("look".into()), None),
            uses_intent: true
        })
    );
    let ground = cell_hit(&map, Vec2::new(10.9, -2.1), false).unwrap();
    assert_eq!(
        click(&map, &assets, ground, Some("use"), true),
        Some(WorldClick::Action {
            action: EntityAction::TerrainClicked(Vec2::new(10., -3.)),
            uses_intent: false
        })
    );
}
#[test]
fn geometry_identity_maps_profiles_doors_and_authored_objects_without_a_renderer() {
    let mut map = Map::default();
    let assets = Assets::default();
    let object = Uuid::new_v4();
    let group = Uuid::new_v4();
    let mut bound = item(4, 0., 0.);
    bound.set_attribute("profile_host_sector_id", Value::UInt(12));
    bound.set_attribute("profile_sector_id", Value::UInt(34));
    bound.set_attribute("geometry_object_id", Value::Id(object));
    bound.set_attribute("door_group_id", Value::Id(group));
    map.items.push(bound);
    let mut sector = crate::Sector::new(5, vec![]);
    sector.properties.set("door_group_id", Value::Id(group));
    map.sectors.push(sector);
    for geo in [
        GeoId::Hole(12, 34),
        GeoId::Sector(5),
        GeoId::GeometryObject(object),
    ] {
        assert_eq!(
            geometry_hit(&map, &assets, geo, Vec3::zero(), 1., None)
                .unwrap()
                .item,
            Some(4)
        );
    }
    map.items[0].set_attribute("sector_id", Value::Int(6));
    assert_eq!(find_item_by_sector_id(&map, 6).unwrap().id, 4);
    assert!(find_item_by_profile_attrs(&map, None, Some(34)).is_none());
    assert!(
        geometry_hit(
            &map,
            &assets,
            GeoId::Item(4),
            Vec3::zero(),
            f32::INFINITY,
            None
        )
        .is_none()
    );
    assert!(
        geometry_hit(
            &map,
            &assets,
            GeoId::Item(4),
            Vec3::new(f32::NAN, 0., 0.),
            1.,
            None
        )
        .is_none()
    );
    // An unknown geometry identity is a valid surface, not an invented item.
    assert_eq!(
        geometry_hit(&map, &assets, GeoId::Unknown(99), Vec3::zero(), 1., None)
            .unwrap()
            .item,
        None
    );
}
#[test]
fn linked_prop_contextual_verbs_precede_item_bindings_and_explicit_intents_remain_exact() {
    let object = GeometryObject::box_("Leaf", Vec3::zero(), Vec3::one());
    let source_id = object.id;
    let mut asset = BlockPropAsset::new_authored("Door", vec![object]);
    let part = asset.parts[0].id;
    let mut component = BlockPropComponent::new("Door");
    component.properties.set("part_id", Value::Id(part));
    let component_id = component.id;
    asset.components.push(component);
    let target = Uuid::new_v4();
    asset.interaction_targets.push(BlockPropInteractionTarget {
        id: target,
        name: "Door".into(),
        part_id: part,
        shape: BlockPropSemanticShape::Part,
        interaction_anchor: [0.; 3],
        facing_direction: [0., 0., 1.],
        component_id: Some(component_id),
    });
    let instance = BlockPropInstance::new(asset.id);
    let instance_id = instance.id;
    let object_id = block_prop_instance_object_id(instance.id, part, source_id);
    let mut map = Map::default();
    map.block_prop_instances.push(instance);
    let mut bound = item(5, 0., 0.);
    bound.set_attribute("geometry_object_id", Value::Id(object_id));
    map.items.push(bound);
    let mut assets = Assets::default();
    assets.block_props.insert(asset.id, asset);
    let hit = geometry_hit(
        &map,
        &assets,
        GeoId::GeometryObject(object_id),
        Vec3::zero(),
        1.,
        None,
    )
    .unwrap();
    assert_eq!(hit.item, Some(5));
    assert!(hit.prop.is_some());
    for (explicit, verb) in [(None, "open"), (Some("inspect"), "inspect")] {
        assert_eq!(
            click(&map, &assets, hit, explicit, true),
            Some(WorldClick::Action {
                action: EntityAction::BlockPropInteract {
                    instance_id,
                    part_id: part,
                    target_id: Some(target),
                    verb: verb.into(),
                    explicit: explicit.is_some()
                },
                uses_intent: explicit.is_some()
            })
        );
    }
    set_block_prop_door_open(&mut map.block_prop_instances[0], component_id, true);
    assert!(
        matches!(click(&map, &assets, hit, None, false), Some(WorldClick::Action { action: EntityAction::BlockPropInteract { verb, .. }, .. }) if verb == "close")
    );
    let asset_id = map.block_prop_instances[0].asset_id;
    map.block_prop_instances[0].asset_id = Uuid::new_v4();
    assert_eq!(
        click(&map, &assets, hit, Some("inspect"), false),
        None,
        "an instance re-linked since the hit cannot accept the old asset identity"
    );
    map.block_prop_instances[0].asset_id = asset_id;
    let original_part = assets.block_props[&asset_id].interaction_targets[0].part_id;
    assets
        .block_props
        .get_mut(&asset_id)
        .unwrap()
        .interaction_targets[0]
        .part_id = Uuid::new_v4();
    assert_eq!(
        click(&map, &assets, hit, Some("inspect"), false),
        None,
        "the same target ID cannot migrate to another part behind a retained hit"
    );
    assets
        .block_props
        .get_mut(&asset_id)
        .unwrap()
        .interaction_targets[0]
        .part_id = original_part;
    let target = assets
        .block_props
        .get_mut(&asset_id)
        .unwrap()
        .interaction_targets
        .pop()
        .unwrap();
    assert_eq!(click(&map, &assets, hit, Some("inspect"), false), None);
    assets
        .block_props
        .get_mut(&asset_id)
        .unwrap()
        .interaction_targets
        .push(target);
    let part = assets
        .block_props
        .get_mut(&asset_id)
        .unwrap()
        .parts
        .pop()
        .unwrap();
    assert_eq!(click(&map, &assets, hit, Some("inspect"), false), None);
    assets
        .block_props
        .get_mut(&asset_id)
        .unwrap()
        .parts
        .push(part);
    map.block_prop_instances.clear();
    assert_eq!(click(&map, &assets, hit, Some("inspect"), false), None);
}
#[test]
fn stale_target_ids_never_retarget_the_ground_and_namespaces_remain_distinct() {
    let mut map = Map::default();
    map.entities.push(entity(7, 1., 1., false));
    map.items.push(item(7, 2., 2.));
    let assets = Assets::default();
    let entity_hit =
        geometry_hit(&map, &assets, GeoId::Character(7), Vec3::zero(), 2., None).unwrap();
    let item_hit = geometry_hit(&map, &assets, GeoId::Item(7), Vec3::zero(), 2., None).unwrap();
    assert!(matches!(
        click(&map, &assets, entity_hit, None, false),
        Some(WorldClick::Action {
            action: EntityAction::EntityClicked(7, 2., None),
            ..
        })
    ));
    assert!(matches!(
        click(&map, &assets, item_hit, None, false),
        Some(WorldClick::Action {
            action: EntityAction::ItemClicked(7, _, None, None),
            ..
        })
    ));
    let mut another_region = map.clone();
    another_region.id = Uuid::new_v4();
    assert_eq!(
        click(&another_region, &assets, entity_hit, None, false),
        None,
        "numeric IDs cannot cross a region change"
    );
    assert_eq!(click(&another_region, &assets, item_hit, None, false), None);
    map.entities.clear();
    map.items.clear();
    assert_eq!(click(&map, &assets, entity_hit, None, false), None);
    assert_eq!(click(&map, &assets, item_hit, None, true), None);
}
