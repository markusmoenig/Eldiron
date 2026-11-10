//! Shared inspector controls for non-destructive terrain geometry modifiers.
use crate::prelude::*;
use rusterix::{PixelSource, Value, ValueContainer};

#[derive(Clone, Copy)]
pub enum Kind {
    Line,
    Sector,
}

pub fn controls(node: &mut TheNodeUI, kind: Kind) {
    node.add_item(TheNodeUIItem::OpenTree("terrain".into()));
    node.add_item(TheNodeUIItem::Checkbox(
        "terrainEnabled".into(),
        fl!("shape_terrain"),
        fl!("shape_terrain_desc"),
        false,
    ));
    if matches!(kind, Kind::Sector) {
        node.add_item(TheNodeUIItem::Selector(
            "terrainMode".into(),
            fl!("terrain_shape_mode"),
            fl!("shape_terrain_desc"),
            vec![fl!("terrain_shape_plateau"), fl!("terrain_shape_hill")],
            0,
        ));
    }
    let heights: &[(&str, &str)] = match kind {
        Kind::Line => &[
            ("terrainStartHeight", "terrain_shape_start"),
            ("terrainEndHeight", "terrain_shape_end"),
        ],
        Kind::Sector => &[("terrainHeight", "terrain_shape_height")],
    };
    for &(id, label) in heights {
        let text = match label {
            "terrain_shape_start" => fl!("terrain_shape_start"),
            "terrain_shape_end" => fl!("terrain_shape_end"),
            _ => fl!("terrain_shape_height"),
        };
        node.add_item(TheNodeUIItem::FloatEditSlider(
            id.into(),
            text,
            fl!("terrain_shape_height_desc"),
            1.,
            0.0..=32.0,
            false,
        ));
    }
    if matches!(kind, Kind::Line) {
        node.add_item(TheNodeUIItem::FloatEditSlider(
            "terrainWidth".into(),
            fl!("terrain_shape_width"),
            fl!("terrain_shape_width_desc"),
            2.,
            0.0..=64.0,
            false,
        ));
    }
    node.add_item(TheNodeUIItem::FloatEditSlider(
        "terrainBlend".into(),
        fl!("terrain_shape_blend"),
        fl!("terrain_shape_blend_desc"),
        3.,
        0.01..=64.0,
        false,
    ));
    node.add_item(TheNodeUIItem::CloseTree);
}

pub fn load(node: &mut TheNodeUI, props: &ValueContainer, kind: Kind) {
    node.set_bool_value(
        "terrainEnabled",
        props.get_bool_default("terrain_shape", false),
    );
    node.set_f32_value(
        "terrainBlend",
        props.get_float_default("terrain_shape_blend", 3.),
    );
    match kind {
        Kind::Line => {
            node.set_f32_value(
                "terrainStartHeight",
                props.get_float_default("terrain_start_height", 1.),
            );
            node.set_f32_value(
                "terrainEndHeight",
                props.get_float_default("terrain_end_height", 1.),
            );
            node.set_f32_value(
                "terrainWidth",
                props.get_float_default("terrain_shape_width", 2.),
            );
        }
        Kind::Sector => {
            node.set_i32_value(
                "terrainMode",
                props.get_int_default("terrain_shape_mode", 0),
            );
            node.set_f32_value(
                "terrainHeight",
                props.get_float_default("terrain_shape_height", 1.),
            );
        }
    }
}

pub fn apply(node: &TheNodeUI, props: &mut ValueContainer, kind: Kind) -> bool {
    let enabled = node.get_bool_value("terrainEnabled").unwrap_or(false);
    if !enabled && !props.get_bool_default("terrain_shape", false) {
        return false;
    }
    let mut values = vec![
        ("terrain_shape", Value::Bool(enabled)),
        (
            "terrain_shape_blend",
            Value::Float(
                node.get_f32_value("terrainBlend")
                    .unwrap_or(3.)
                    .clamp(0.01, 64.),
            ),
        ),
    ];
    let mut height = |id: &str, key: &'static str| {
        values.push((
            key,
            Value::Float(node.get_f32_value(id).unwrap_or(1.).clamp(0., 32.)),
        ))
    };
    match kind {
        Kind::Line => {
            height("terrainStartHeight", "terrain_start_height");
            height("terrainEndHeight", "terrain_end_height");
            values.push((
                "terrain_shape_width",
                Value::Float(
                    node.get_f32_value("terrainWidth")
                        .unwrap_or(2.)
                        .clamp(0., 64.),
                ),
            ));
        }
        Kind::Sector => {
            height("terrainHeight", "terrain_shape_height");
            values.push((
                "terrain_shape_mode",
                Value::Int(node.get_i32_value("terrainMode").unwrap_or(0).clamp(0, 1)),
            ));
        }
    }
    let mut changed = false;
    for (key, value) in values {
        if props.get(key) != Some(&value) {
            props.set(key, value);
            changed = true;
        }
    }
    changed
}

pub fn enable(map: &mut Map, server: &ServerContext) {
    map.properties.set("terrain_enabled", Value::Bool(true));
    let subdivisions = map
        .properties
        .get_int_default("terrain_subdivisions", 4)
        .clamp(4, 8);
    map.properties
        .set("terrain_subdivisions", Value::Int(subdivisions));
    if !map.properties.contains("terrain_edge_falloff") {
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
    }
    if !map.properties.contains("default_terrain_tile") {
        let source = server
            .curr_tile_id
            .filter(|id| {
                let runtime = crate::editor::RUSTERIX.read().unwrap();
                runtime.assets.tiles.get(id).is_some_and(|tile| {
                    tile.isometric
                        .as_ref()
                        .is_none_or(|layout| layout.ground && !layout.bitmap)
                })
            })
            .map(PixelSource::TileId)
            .unwrap_or(PixelSource::Color([65, 85, 65, 255].into()));
        map.properties
            .set("default_terrain_tile", Value::Source(source));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, edit_linedef::EditLinedef, edit_sector::EditSector};
    use rusterix::chunkbuilder::terrain_generator::{TerrainConfig, TerrainGenerator};

    fn set(action: &mut dyn Action, id: &str, value: TheValue, server: &mut ServerContext) {
        action.handle_event(
            &TheEvent::ValueChanged(TheId::named(id), value),
            &mut Project::default(),
            &mut TheUI::new(),
            &mut TheContext::new(800, 600, 1.),
            server,
        );
    }
    fn sample(map: &Map, p: Vec2<f32>) -> f32 {
        TerrainGenerator::sample_height_at(map, p, &TerrainConfig::for_map(map))
    }
    #[test]
    fn line_inspector_shapes_ground_with_undo_redo_and_disable() {
        let mut map = Map::default();
        let a = map.add_vertex_at(-3., 0.);
        let b = map.add_vertex_at(3., 0.);
        let id = map.create_linedef_manual(a, b);
        map.clear_temp();
        map.selected_linedefs = vec![id];
        let mut server = ServerContext::default();
        server.pc = ProjectContext::Region(Uuid::new_v4());
        let mut action = EditLinedef::new();
        action.load_params(&map);
        for (id, value) in [
            ("terrainEnabled", TheValue::Bool(true)),
            ("terrainStartHeight", TheValue::Float(0.)),
            ("terrainEndHeight", TheValue::Float(1.2)),
            ("terrainWidth", TheValue::Float(2.)),
            ("terrainBlend", TheValue::Float(3.)),
        ] {
            set(&mut action, id, value, &mut server);
        }
        let source = crate::actions::nodeui_to_toml(&action.params());
        let parsed: toml::Table = source.parse().unwrap();
        assert_eq!(parsed["terrain"]["enabled"].as_bool(), Some(true));
        assert_eq!(parsed["terrain"]["end_height"].as_float(), Some(1.2));
        let mut reopened_params = action.params();
        crate::actions::apply_toml_to_nodeui(&mut reopened_params, &source).unwrap();
        assert_eq!(reopened_params.get_bool_value("terrainEnabled"), Some(true));
        assert_eq!(reopened_params.get_f32_value("terrainEndHeight"), Some(1.2));
        let undo = action
            .apply(
                &mut map,
                &mut TheUI::new(),
                &mut TheContext::new(800, 600, 1.),
                &mut server,
            )
            .unwrap();
        assert!(map.properties.get_bool_default("terrain_enabled", false));
        assert!((sample(&map, Vec2::zero()) - 0.6).abs() < 1e-5);
        let ProjectUndoAtom::MapEdit(_, before, after) = undo else {
            panic!("expected map undo")
        };
        assert_eq!(sample(&before, Vec2::zero()), 0.);
        assert!((sample(&after, Vec2::zero()) - 0.6).abs() < 1e-5);
        action.load_params(&map);
        set(
            &mut action,
            "terrainEnabled",
            TheValue::Bool(false),
            &mut server,
        );
        assert!(
            action
                .apply(
                    &mut map,
                    &mut TheUI::new(),
                    &mut TheContext::new(800, 600, 1.),
                    &mut server
                )
                .is_some()
        );
        assert_eq!(sample(&map, Vec2::zero()), 0.);
        assert_eq!(map.find_vertex(a).unwrap().z, 0.);
        assert_eq!(map.find_vertex(b).unwrap().z, 0.);
    }
    #[test]
    fn sector_inspector_shapes_filled_ground_without_changing_floor_or_construction_height() {
        let mut map = Map::default();
        map.camera = rusterix::MapCamera::TwoDIso;
        let v = [[-4., -4.], [4., -4.], [4., 4.], [-4., 4.]].map(|p| map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        map.selected_sectors = vec![id];
        map.properties.set("iso_edit_height", Value::Float(0.25));
        let mut server = ServerContext::default();
        server.pc = ProjectContext::Region(Uuid::new_v4());
        let mut action = EditSector::new();
        action.load_params(&map);
        set(
            &mut action,
            "terrainEnabled",
            TheValue::Bool(true),
            &mut server,
        );
        set(
            &mut action,
            "terrainHeight",
            TheValue::Float(1.2),
            &mut server,
        );
        assert!(
            action
                .apply(
                    &mut map,
                    &mut TheUI::new(),
                    &mut TheContext::new(800, 600, 1.),
                    &mut server
                )
                .is_some()
        );
        assert!((sample(&map, Vec2::zero()) - 1.2).abs() < 1e-5);
        assert_eq!(
            map.find_sector(id)
                .unwrap()
                .properties
                .get_float_default("floor_height", 0.),
            0.
        );
        assert_eq!(
            map.properties.get_float_default("iso_edit_height", 0.),
            0.25
        );
        let bytes = serde_json::to_vec(&map).unwrap();
        let reopened: Map = serde_json::from_slice(&bytes).unwrap();
        assert!((sample(&reopened, Vec2::zero()) - 1.2).abs() < 1e-5);
    }
}
