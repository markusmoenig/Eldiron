use super::*;
use crate::map::light::LightType;
use theframework_core::TheColor;

fn color() -> TheColor {
    TheColor::new(0.25, 0.5, 0.75, 1.)
}

// Both edits compare equal for widget editing, but alter serialized input. Test
// every property owner and both face source locations used by overview caching.
#[test]
fn overview_source_observes_exact_nested_colors_without_changing_editing_equality() {
    let mut map = Map::default();
    let mut geometry = GeometryObject::box_("source", Vec3::zero(), Vec3::one());
    geometry.properties.set("tint", Value::Color(color()));
    geometry.faces[0].tile = Some(PixelSource::Color(color()));
    geometry.faces[0]
        .tiles
        .insert((3, -7), PixelSource::Color(color()));
    map.geometry_objects.push(geometry);
    let mut light = Light::new(LightType::Point);
    light.properties.set("tint", Value::Color(color()));
    map.lights.push(light);
    let mut prop = BlockPropInstance::new(Uuid::new_v4());
    prop.parameter_overrides.set("tint", Value::Color(color()));
    prop.runtime_state.set("tint", Value::Color(color()));
    prop.overrides.set("tint", Value::Color(color()));
    map.block_prop_instances.push(prop);
    assert!(map.same_overview_source(&map.clone()));

    for named in [false, true] {
        let mut changed_color = color();
        if named {
            changed_color.name = "authored tint".into();
        } else {
            changed_color.r += 0.00001;
        }
        assert_eq!(color(), changed_color);
        for owner in 0..7 {
            let mut changed = map.clone();
            match owner {
                0 => changed.geometry_objects[0]
                    .properties
                    .set("tint", Value::Color(changed_color.clone())),
                1 => {
                    changed.geometry_objects[0].faces[0].tile =
                        Some(PixelSource::Color(changed_color.clone()))
                }
                2 => {
                    changed.geometry_objects[0].faces[0]
                        .tiles
                        .insert((3, -7), PixelSource::Color(changed_color.clone()));
                }
                3 => changed.lights[0]
                    .properties
                    .set("tint", Value::Color(changed_color.clone())),
                4 => changed.block_prop_instances[0]
                    .parameter_overrides
                    .set("tint", Value::Color(changed_color.clone())),
                5 => changed.block_prop_instances[0]
                    .runtime_state
                    .set("tint", Value::Color(changed_color.clone())),
                6 => changed.block_prop_instances[0]
                    .overrides
                    .set("tint", Value::Color(changed_color.clone())),
                _ => unreachable!(),
            }
            assert_eq!(map.geometry_objects, changed.geometry_objects);
            assert_eq!(map.lights, changed.lights);
            assert_eq!(map.block_prop_instances, changed.block_prop_instances);
            assert!(
                !map.same_overview_source(&changed),
                "owner={owner} named={named}"
            );
            assert!(!changed.same_overview_source(&map));
        }
    }
    let mut navigation = map.clone();
    navigation.offset = Vec2::new(14., -70.);
    navigation.grid_size *= 2.;
    navigation
        .selected_geometry_objects
        .push(map.geometry_objects[0].id);
    assert!(map.same_overview_source(&navigation));
    map.profiles.insert(Uuid::new_v4(), navigation);
    let mut changed = map.clone();
    changed.profiles.values_mut().next().unwrap().lights[0]
        .properties
        .set("tint", Value::Color(TheColor::new(0.25001, 0.5, 0.75, 1.)));
    assert!(!map.same_overview_source(&changed));
}
