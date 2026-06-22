use crate::editor::RUSTERIX;
use crate::prelude::*;

const MATERIAL_PRESET_VALUES: [&str; 11] = [
    "default", "stone", "wood", "metal", "glass", "water", "mirror", "emissive", "dirt", "fabric",
    "plastic",
];
const MATERIAL_FINISH_VALUES: [&str; 4] = ["natural", "matte", "polished", "wet"];

pub struct EditGeometry {
    id: TheId,
    nodeui: TheNodeUI,
}

#[derive(Clone, Copy)]
struct GeometryBounds {
    center: Vec3<f32>,
    size: Vec3<f32>,
}

impl EditGeometry {
    fn material_preset_labels() -> Vec<String> {
        vec![
            fl!("material_preset_default"),
            fl!("material_preset_stone"),
            fl!("material_preset_wood"),
            fl!("material_preset_metal"),
            fl!("material_preset_glass"),
            fl!("material_preset_water"),
            fl!("material_preset_mirror"),
            fl!("material_preset_emissive"),
            fl!("material_preset_dirt"),
            fl!("material_preset_fabric"),
            fl!("material_preset_plastic"),
        ]
    }

    fn material_finish_labels() -> Vec<String> {
        vec![
            fl!("material_finish_natural"),
            fl!("material_finish_matte"),
            fl!("material_finish_polished"),
            fl!("material_finish_wet"),
        ]
    }

    fn material_index(values: &[&str], value: &str, fallback: usize) -> i32 {
        values
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(value.trim()))
            .unwrap_or(fallback) as i32
    }

    fn bounds(vertices: &[Vec3<f32>]) -> Option<GeometryBounds> {
        let mut min = Vec3::broadcast(f32::INFINITY);
        let mut max = Vec3::broadcast(f32::NEG_INFINITY);
        let mut found = false;
        for vertex in vertices {
            if !vertex.x.is_finite() || !vertex.y.is_finite() || !vertex.z.is_finite() {
                continue;
            }
            min.x = min.x.min(vertex.x);
            min.y = min.y.min(vertex.y);
            min.z = min.z.min(vertex.z);
            max.x = max.x.max(vertex.x);
            max.y = max.y.max(vertex.y);
            max.z = max.z.max(vertex.z);
            found = true;
        }
        found.then(|| GeometryBounds {
            center: (min + max) * 0.5,
            size: max - min,
        })
    }

    fn refit_vertices(vertices: &mut [Vec3<f32>], from: GeometryBounds, to: GeometryBounds) {
        let safe_size = Vec3::new(
            from.size.x.max(0.0001),
            from.size.y.max(0.0001),
            from.size.z.max(0.0001),
        );
        for vertex in vertices {
            let local = (*vertex - from.center) / safe_size;
            *vertex = to.center + local * to.size;
        }
    }
}

impl Action for EditGeometry {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::OpenTree("metadata".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "name".into(),
            "Name".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "group".into(),
            "Group".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "item".into(),
            "Item".into(),
            "Optional item/handler metadata for this 3D area.".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "area".into(),
            "Area".into(),
            "Use this named geometry object as a 3D gameplay area.".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "hide_iso".into(),
            "Hide in Iso".into(),
            "Hide this object in isometric gameplay when the player is inside its area.".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "visible".into(),
            "Visible".into(),
            "Render this geometry object in the scene.".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "solid".into(),
            "Solid".into(),
            "Include this geometry object in mesh collision.".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "materialPreset".into(),
            fl!("material_preset"),
            fl!("status_material_preset"),
            Self::material_preset_labels(),
            0,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "materialFinish".into(),
            fl!("material_finish"),
            fl!("status_material_finish"),
            Self::material_finish_labels(),
            0,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("geometry".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "x".into(),
            "X".into(),
            "".into(),
            0.0,
            -1024.0..=1024.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "y".into(),
            "Y".into(),
            "".into(),
            0.0,
            -1024.0..=1024.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "z".into(),
            "Z".into(),
            "".into(),
            0.0,
            -1024.0..=1024.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "width".into(),
            "Width".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "height".into(),
            "Height".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "depth".into(),
            "Depth".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        Self {
            id: TheId::named(&fl!("action_edit_geometry")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_geometry_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && map.selected_geometry_objects.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        let Some(id) = map.selected_geometry_objects.first() else {
            return;
        };
        let Some(object) = map.geometry_objects.iter().find(|object| object.id == *id) else {
            return;
        };
        let Some(bounds) = Self::bounds(&object.vertices) else {
            return;
        };

        self.nodeui.set_text_value("name", object.name.clone());
        self.nodeui.set_text_value("group", object.group.clone());
        self.nodeui
            .set_text_value("item", object.properties.get_str_default("item", "".into()));
        self.nodeui
            .set_bool_value("area", object.properties.get_bool_default("area", true));
        self.nodeui.set_bool_value(
            "hide_iso",
            object.properties.get_bool_default("hide_iso", false),
        );
        let material_preset = object
            .properties
            .get_str_default("material_preset", "default".to_string());
        let mut material_finish = object
            .properties
            .get_str_default("material_finish", "natural".to_string());
        if material_preset == "default" {
            material_finish = "natural".to_string();
        }
        self.nodeui.set_i32_value(
            "materialPreset",
            Self::material_index(&MATERIAL_PRESET_VALUES, &material_preset, 0),
        );
        self.nodeui.set_i32_value(
            "materialFinish",
            Self::material_index(&MATERIAL_FINISH_VALUES, &material_finish, 0),
        );
        self.nodeui.set_bool_value("visible", object.visible);
        self.nodeui.set_bool_value("solid", object.solid);
        self.nodeui.set_f32_value("x", bounds.center.x);
        self.nodeui.set_f32_value("y", bounds.center.y);
        self.nodeui.set_f32_value("z", bounds.center.z);
        self.nodeui.set_f32_value("width", bounds.size.x.max(0.05));
        self.nodeui.set_f32_value("height", bounds.size.y.max(0.05));
        self.nodeui.set_f32_value("depth", bounds.size.z.max(0.05));
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let id = *map.selected_geometry_objects.first()?;
        let object = map
            .geometry_objects
            .iter_mut()
            .find(|object| object.id == id)?;
        let from = Self::bounds(&object.vertices)?;

        let to = GeometryBounds {
            center: Vec3::new(
                self.nodeui.get_f32_value("x").unwrap_or(from.center.x),
                self.nodeui.get_f32_value("y").unwrap_or(from.center.y),
                self.nodeui.get_f32_value("z").unwrap_or(from.center.z),
            ),
            size: Vec3::new(
                self.nodeui
                    .get_f32_value("width")
                    .unwrap_or(from.size.x)
                    .max(0.05),
                self.nodeui
                    .get_f32_value("height")
                    .unwrap_or(from.size.y)
                    .max(0.05),
                self.nodeui
                    .get_f32_value("depth")
                    .unwrap_or(from.size.z)
                    .max(0.05),
            ),
        };
        let name = self
            .nodeui
            .get_text_value("name")
            .unwrap_or_else(|| object.name.clone());
        let group = self
            .nodeui
            .get_text_value("group")
            .unwrap_or_else(|| object.group.clone())
            .trim()
            .to_string();
        let visible = self
            .nodeui
            .get_bool_value("visible")
            .unwrap_or(object.visible);
        let solid = self.nodeui.get_bool_value("solid").unwrap_or(object.solid);
        let item = self
            .nodeui
            .get_text_value("item")
            .unwrap_or_else(|| object.properties.get_str_default("item", "".into()))
            .trim()
            .to_string();
        let area = self
            .nodeui
            .get_bool_value("area")
            .unwrap_or_else(|| object.properties.get_bool_default("area", true));
        let hide_iso = self
            .nodeui
            .get_bool_value("hide_iso")
            .unwrap_or_else(|| object.properties.get_bool_default("hide_iso", false));
        let material_preset_index = self
            .nodeui
            .get_i32_value("materialPreset")
            .unwrap_or(0)
            .max(0) as usize;
        let material_finish_index = self
            .nodeui
            .get_i32_value("materialFinish")
            .unwrap_or(0)
            .max(0) as usize;
        let material_preset = MATERIAL_PRESET_VALUES
            .get(material_preset_index)
            .copied()
            .unwrap_or("default")
            .to_string();
        let mut material_finish = MATERIAL_FINISH_VALUES
            .get(material_finish_index)
            .copied()
            .unwrap_or("natural")
            .to_string();
        if material_preset == "default" {
            material_finish = "natural".to_string();
        }
        let existing_item = object.properties.get_str_default("item", "".into());
        let existing_area = object.properties.get_bool_default("area", true);
        let existing_hide_iso = object.properties.get_bool_default("hide_iso", false);
        let existing_material_preset = object
            .properties
            .get_str_default("material_preset", "default".to_string());
        let existing_material_finish = object
            .properties
            .get_str_default("material_finish", "natural".to_string());

        if (to.center - from.center).magnitude_squared() <= 0.000001
            && (to.size - from.size).magnitude_squared() <= 0.000001
            && name == object.name
            && group == object.group
            && visible == object.visible
            && solid == object.solid
            && item == existing_item
            && area == existing_area
            && hide_iso == existing_hide_iso
            && material_preset == existing_material_preset
            && material_finish == existing_material_finish
        {
            return None;
        }

        let prev = map.clone();
        let object = map
            .geometry_objects
            .iter_mut()
            .find(|object| object.id == id)?;
        object.name = name;
        object.group = group;
        object.visible = visible;
        object.solid = solid;
        if item.is_empty() {
            object.properties.remove("item");
        } else {
            object.properties.set("item", Value::Str(item));
        }
        object.properties.set("area", Value::Bool(area));
        object.properties.set("hide_iso", Value::Bool(hide_iso));
        if material_preset == "default" {
            object.properties.remove("material_preset");
            object.properties.remove("material_finish");
        } else {
            object
                .properties
                .set("material_preset", Value::Str(material_preset));
            if material_finish == "natural" {
                object.properties.remove("material_finish");
            } else {
                object
                    .properties
                    .set("material_finish", Value::Str(material_finish));
            }
        }
        Self::refit_vertices(&mut object.vertices, from, to);

        map.update_surfaces();
        RUSTERIX.write().unwrap().set_dirty();
        RUSTERIX.write().unwrap().set_overlay_dirty();
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(prev),
            Box::new(map.clone()),
        ))
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        _project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::nodeui_to_toml;

    #[test]
    fn edit_geometry_toml_is_grouped() {
        let action = EditGeometry::new();
        let toml = nodeui_to_toml(&action.params());

        assert!(toml.contains("[metadata]\n"));
        assert!(toml.contains("name = \"\""));
        assert!(toml.contains("visible = true"));
        assert!(toml.contains("solid = true"));
        assert!(toml.contains("[material]\n"));
        assert!(toml.contains("preset = \"Default\""));
        assert!(toml.contains("finish = \"Natural\""));
        assert!(toml.contains("[geometry]\n"));
        assert!(toml.contains("x = 0.0"));
        assert!(toml.contains("width = 1.0"));
        assert!(!toml.contains("action_geometry"));
        assert!(!toml.contains("geometry_name"));
        assert!(!toml.contains("geometry_x"));
        assert!(!toml.contains("geometry_width"));
    }
}
