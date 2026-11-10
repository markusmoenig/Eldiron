//! Terrain coverage belongs to the selected geometry, independent of its height.
use super::{ActionMaterialSlot, shape_terrain::Kind};
use crate::prelude::*;
use rusterix::{PixelSource, Value, ValueContainer};

pub fn controls(node: &mut TheNodeUI, kind: Kind) {
    node.add_item(TheNodeUIItem::OpenTree("terrain".into()));
    node.add_item(TheNodeUIItem::OpenTree("material".into()));
    for (id, label) in [("terrainMaterialSource", fl!("terrain_material_source"))]
        .into_iter()
        .chain(
            matches!(kind, Kind::Line)
                .then(|| ("terrainMaterialVergeSource", fl!("terrain_material_verge"))),
        )
    {
        node.add_item(TheNodeUIItem::Text(
            id.into(),
            label,
            fl!("terrain_material_source_desc"),
            String::new(),
            None,
            false,
        ));
    }
    let fields = match kind {
        Kind::Line => vec![
            ("Width", fl!("terrain_material_width"), 1.35, 0.1, 64.),
            (
                "VergeWidth",
                fl!("terrain_material_verge_width"),
                3.2,
                0.1,
                64.,
            ),
        ],
        Kind::Sector => vec![("Inset", fl!("terrain_material_inset"), 0., 0., 64.)],
    };
    for (suffix, label, value, min, max) in fields.into_iter().chain([
        ("SoftEdge", fl!("terrain_material_soft_edge"), 0.45, 0., 8.),
        (
            "NoisyEdge",
            fl!("terrain_material_noisy_edge"),
            0.13,
            0.,
            2.,
        ),
        (
            "NoiseScale",
            fl!("terrain_material_noise_scale"),
            2.7,
            0.1,
            16.,
        ),
    ]) {
        node.add_item(TheNodeUIItem::FloatEditSlider(
            format!("terrainMaterial{suffix}"),
            label,
            fl!("terrain_material_edge_desc"),
            value,
            min..=max,
            false,
        ));
    }
    node.add_item(TheNodeUIItem::IntEditSlider(
        "terrainMaterialOrder".into(),
        fl!("terrain_material_order"),
        fl!("terrain_material_order_desc"),
        0,
        -100..=100,
        false,
    ));
    node.add_item(TheNodeUIItem::CloseTree);
    node.add_item(TheNodeUIItem::CloseTree);
}
pub fn load(node: &mut TheNodeUI, p: &ValueContainer, kind: Kind) {
    node.set_text_value(
        "terrainMaterialSource",
        super::source_to_text(p.get("terrain_material_source")),
    );
    if matches!(kind, Kind::Line) {
        node.set_text_value(
            "terrainMaterialVergeSource",
            super::source_to_text(p.get("terrain_material_verge_source")),
        );
    }
    for (suffix, key, default) in [
        ("Width", "terrain_material_width", 1.35),
        ("VergeWidth", "terrain_material_verge_width", 3.2),
        ("Inset", "terrain_material_inset", 0.),
        ("SoftEdge", "terrain_material_soft_edge", 0.45),
        ("NoisyEdge", "terrain_material_noisy_edge", 0.13),
        ("NoiseScale", "terrain_material_noise_scale", 2.7),
    ] {
        node.set_f32_value(
            &format!("terrainMaterial{suffix}"),
            p.get_float_default(key, default),
        );
    }
    node.set_i32_value(
        "terrainMaterialOrder",
        p.get_int_default("terrain_material_order", 0),
    );
}
pub fn validate(node: &TheNodeUI) -> Result<(), String> {
    for field in ["terrainMaterialSource", "terrainMaterialVergeSource"] {
        let text = node.get_text_value(field).unwrap_or_default();
        if !text.trim().is_empty()
            && !matches!(
                super::parse_tile_id_pixelsource(&text),
                Some(PixelSource::TileId(_))
            )
        {
            return Err(format!("{}: {text}", fl!("terrain_material_invalid")));
        }
    }
    Ok(())
}
pub fn enabled(node: &TheNodeUI) -> bool {
    ["terrainMaterialSource", "terrainMaterialVergeSource"]
        .iter()
        .any(|field| {
            !node
                .get_text_value(field)
                .unwrap_or_default()
                .trim()
                .is_empty()
        })
}
pub fn apply(node: &TheNodeUI, p: &mut ValueContainer, kind: Kind) -> bool {
    let before = p.clone();
    for (field, key) in [
        ("terrainMaterialSource", "terrain_material_source"),
        (
            "terrainMaterialVergeSource",
            "terrain_material_verge_source",
        ),
    ] {
        match node
            .get_text_value(field)
            .and_then(|s| super::parse_tile_id_pixelsource(&s))
        {
            Some(source @ PixelSource::TileId(_)) => {
                p.set(key, Value::Source(source));
            }
            _ => {
                p.remove(key);
            }
        }
    }
    if p.get_source("terrain_material_source").is_some()
        || p.get_source("terrain_material_verge_source").is_some()
    {
        for (suffix, key, default, min, max) in [
            ("Width", "terrain_material_width", 1.35, 0.1, 64.),
            ("VergeWidth", "terrain_material_verge_width", 3.2, 0.1, 64.),
            ("Inset", "terrain_material_inset", 0., 0., 64.),
            ("SoftEdge", "terrain_material_soft_edge", 0.45, 0., 8.),
            ("NoisyEdge", "terrain_material_noisy_edge", 0.13, 0., 2.),
            ("NoiseScale", "terrain_material_noise_scale", 2.7, 0.1, 16.),
        ] {
            if matches!(kind, Kind::Sector) && matches!(suffix, "Width" | "VergeWidth")
                || matches!(kind, Kind::Line) && suffix == "Inset"
            {
                continue;
            }
            let v = node
                .get_f32_value(&format!("terrainMaterial{suffix}"))
                .unwrap_or(default);
            p.set(
                key,
                Value::Float(if v.is_finite() {
                    v.clamp(min, max)
                } else {
                    default
                }),
            );
        }
        p.set(
            "terrain_material_order",
            Value::Int(
                node.get_i32_value("terrainMaterialOrder")
                    .unwrap_or(0)
                    .clamp(-100, 100),
            ),
        );
    }
    *p != before
}
pub fn slots(node: &TheNodeUI, kind: Kind) -> Vec<ActionMaterialSlot> {
    [("terrainMaterialSource", fl!("terrain_material_source"))]
        .into_iter()
        .chain(
            matches!(kind, Kind::Line)
                .then(|| ("terrainMaterialVergeSource", fl!("terrain_material_verge"))),
        )
        .map(|(field, label)| ActionMaterialSlot {
            label,
            source: node
                .get_text_value(field)
                .and_then(|s| super::parse_tile_id_pixelsource(&s)),
        })
        .collect()
}
pub fn set_slot(node: &mut TheNodeUI, slot: i32, source: Option<PixelSource>, kind: Kind) -> bool {
    let field = match slot {
        0 => "terrainMaterialSource",
        1 if matches!(kind, Kind::Line) => "terrainMaterialVergeSource",
        _ => return false,
    };
    let text = match source {
        Some(PixelSource::TileId(id)) => id.to_string(),
        None => String::new(),
        _ => return false,
    };
    node.set_text_value(field, text);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{Action, edit_linedef::EditLinedef, edit_sector::EditSector};
    #[test]
    fn geometry_toml_roundtrip_and_material_assignment_have_map_undo_without_changing_height() {
        for line in [false, true] {
            let mut map = Map::default();
            let points =
                [[-4., -4.], [4., -4.], [4., 4.], [-4., 4.]].map(|p| map.add_vertex_at(p[0], p[1]));
            for i in 0..4 {
                map.create_linedef_manual(points[i], points[(i + 1) % 4]);
            }
            let area = map.close_polygon_manual().unwrap();
            map.clear_temp();
            let edge = map.linedefs[0].id;
            map.selected_linedefs = if line { vec![edge] } else { vec![] };
            map.selected_sectors = if line { vec![] } else { vec![area] };
            let source = Uuid::new_v4();
            map.properties.set(
                "default_terrain_tile",
                Value::Source(PixelSource::TileId(source)),
            );
            let mut action: Box<dyn Action> = if line {
                Box::new(EditLinedef::new())
            } else {
                Box::new(EditSector::new())
            };
            action.load_params(&map);
            let text = format!(
                "[terrain.material]\nsource = \"{source}\"\nwidth = 2.5\ninset = 0.4\nsoft_edge = 0.6\nnoisy_edge = 0.2\norder = 12\n"
            );
            let mut parsed = action.params();
            crate::actions::apply_toml_to_nodeui(&mut parsed, &text).unwrap();
            assert_eq!(
                parsed.get_text_value("terrainMaterialSource"),
                Some(source.to_string())
            );
            let roundtrip: toml::Table = crate::actions::nodeui_to_toml(&parsed).parse().unwrap();
            assert_eq!(
                roundtrip["terrain"]["material"]["soft_edge"].as_float(),
                Some(0.6)
            );
            let kind = if line { Kind::Line } else { Kind::Sector };
            let mut props = ValueContainer::default();
            assert!(apply(&parsed, &mut props, kind));
            let mut reopened = action.params();
            load(&mut reopened, &props, kind);
            assert_eq!(reopened.get_f32_value("terrainMaterialSoftEdge"), Some(0.6));
            let mut server = ServerContext::default();
            server.pc = ProjectContext::Region(Uuid::new_v4());
            server.editor_view_mode = EditorViewMode::Orbit;
            assert!(action.is_applicable(&map, &mut TheContext::new(800, 600, 1.), &server));
            assert!(action.set_hud_material_source(&map, &server, 0, PixelSource::TileId(source)));
            let undo = action
                .apply(
                    &mut map,
                    &mut TheUI::new(),
                    &mut TheContext::new(800, 600, 1.),
                    &mut server,
                )
                .unwrap();
            let ProjectUndoAtom::MapEdit(_, before, after) = undo else {
                panic!("map undo expected")
            };
            let props = if line {
                &after.find_linedef(edge).unwrap().properties
            } else {
                &after.find_sector(area).unwrap().properties
            };
            assert_eq!(
                props.get_source("terrain_material_source"),
                Some(&PixelSource::TileId(source))
            );
            assert!(!props.get_bool_default("terrain_shape", false));
            let props = if line {
                &before.find_linedef(edge).unwrap().properties
            } else {
                &before.find_sector(area).unwrap().properties
            };
            assert!(props.get_source("terrain_material_source").is_none());
            assert!(map.vertices.iter().all(|v| v.z == 0.));
            assert!(action.clear_hud_material_slot(&map, &server, 0));
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
            let props = if line {
                &map.find_linedef(edge).unwrap().properties
            } else {
                &map.find_sector(area).unwrap().properties
            };
            assert!(props.get_source("terrain_material_source").is_none());
        }
    }
}
