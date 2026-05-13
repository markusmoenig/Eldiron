use crate::editor::RUSTERIX;
use crate::prelude::*;

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
        nodeui.add_item(TheNodeUIItem::Text(
            "actionGeometryName".into(),
            "Name".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionGeometryGroup".into(),
            "Group".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionGeometryItem".into(),
            "Item".into(),
            "Optional item/handler metadata for this 3D area.".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionGeometryArea".into(),
            "Area".into(),
            "Use this named geometry object as a 3D gameplay area.".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionGeometryHideIso".into(),
            "Hide in Iso".into(),
            "Hide this object in isometric gameplay when the player is inside its area.".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionGeometryVisible".into(),
            "Visible".into(),
            "Render this geometry object in the scene.".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionGeometrySolid".into(),
            "Solid".into(),
            "Include this geometry object in mesh collision.".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryX".into(),
            "X".into(),
            "".into(),
            0.0,
            -1024.0..=1024.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryY".into(),
            "Y".into(),
            "".into(),
            0.0,
            -1024.0..=1024.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryZ".into(),
            "Z".into(),
            "".into(),
            0.0,
            -1024.0..=1024.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryWidth".into(),
            "Width".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryHeight".into(),
            "Height".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionGeometryDepth".into(),
            "Depth".into(),
            "".into(),
            1.0,
            0.05..=256.0,
            false,
        ));

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

        self.nodeui
            .set_text_value("actionGeometryName", object.name.clone());
        self.nodeui
            .set_text_value("actionGeometryGroup", object.group.clone());
        self.nodeui.set_text_value(
            "actionGeometryItem",
            object.properties.get_str_default("item", "".into()),
        );
        self.nodeui.set_bool_value(
            "actionGeometryArea",
            object.properties.get_bool_default("area", true),
        );
        self.nodeui.set_bool_value(
            "actionGeometryHideIso",
            object.properties.get_bool_default("hide_iso", false),
        );
        self.nodeui
            .set_bool_value("actionGeometryVisible", object.visible);
        self.nodeui
            .set_bool_value("actionGeometrySolid", object.solid);
        self.nodeui
            .set_f32_value("actionGeometryX", bounds.center.x);
        self.nodeui
            .set_f32_value("actionGeometryY", bounds.center.y);
        self.nodeui
            .set_f32_value("actionGeometryZ", bounds.center.z);
        self.nodeui
            .set_f32_value("actionGeometryWidth", bounds.size.x.max(0.05));
        self.nodeui
            .set_f32_value("actionGeometryHeight", bounds.size.y.max(0.05));
        self.nodeui
            .set_f32_value("actionGeometryDepth", bounds.size.z.max(0.05));
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
                self.nodeui
                    .get_f32_value("actionGeometryX")
                    .unwrap_or(from.center.x),
                self.nodeui
                    .get_f32_value("actionGeometryY")
                    .unwrap_or(from.center.y),
                self.nodeui
                    .get_f32_value("actionGeometryZ")
                    .unwrap_or(from.center.z),
            ),
            size: Vec3::new(
                self.nodeui
                    .get_f32_value("actionGeometryWidth")
                    .unwrap_or(from.size.x)
                    .max(0.05),
                self.nodeui
                    .get_f32_value("actionGeometryHeight")
                    .unwrap_or(from.size.y)
                    .max(0.05),
                self.nodeui
                    .get_f32_value("actionGeometryDepth")
                    .unwrap_or(from.size.z)
                    .max(0.05),
            ),
        };
        let name = self
            .nodeui
            .get_text_value("actionGeometryName")
            .unwrap_or_else(|| object.name.clone());
        let group = self
            .nodeui
            .get_text_value("actionGeometryGroup")
            .unwrap_or_else(|| object.group.clone())
            .trim()
            .to_string();
        let visible = self
            .nodeui
            .get_bool_value("actionGeometryVisible")
            .unwrap_or(object.visible);
        let solid = self
            .nodeui
            .get_bool_value("actionGeometrySolid")
            .unwrap_or(object.solid);
        let item = self
            .nodeui
            .get_text_value("actionGeometryItem")
            .unwrap_or_else(|| object.properties.get_str_default("item", "".into()))
            .trim()
            .to_string();
        let area = self
            .nodeui
            .get_bool_value("actionGeometryArea")
            .unwrap_or_else(|| object.properties.get_bool_default("area", true));
        let hide_iso = self
            .nodeui
            .get_bool_value("actionGeometryHideIso")
            .unwrap_or_else(|| object.properties.get_bool_default("hide_iso", false));
        let existing_item = object.properties.get_str_default("item", "".into());
        let existing_area = object.properties.get_bool_default("area", true);
        let existing_hide_iso = object.properties.get_bool_default("hide_iso", false);

        if (to.center - from.center).magnitude_squared() <= 0.000001
            && (to.size - from.size).magnitude_squared() <= 0.000001
            && name == object.name
            && group == object.group
            && visible == object.visible
            && solid == object.solid
            && item == existing_item
            && area == existing_area
            && hide_iso == existing_hide_iso
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
