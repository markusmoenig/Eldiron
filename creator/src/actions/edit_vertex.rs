use crate::prelude::*;
use rusterix::Value;

pub struct EditVertex {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EditVertex {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "actionVertexName".into(),
            fl!("action_edit_vertex_name"),
            fl!("status_action_edit_vertex_name"),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexX".into(),
            fl!("action_edit_vertex_x"),
            fl!("status_action_edit_vertex_x"),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexY".into(),
            fl!("action_edit_vertex_y"),
            fl!("status_action_edit_vertex_y"),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexZ".into(),
            fl!("action_edit_vertex_z"),
            fl!("status_action_edit_vertex_z"),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionVertexTerrain".into(),
            fl!("action_edit_vertex_terrain_control"),
            fl!("status_action_edit_vertex_terrain_control"),
            false,
        ));

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexTerrainSmoothness".into(),
            fl!("action_edit_vertex_terrain_smoothness"),
            fl!("status_action_edit_vertex_terrain_smoothness"),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_edit_vertex_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_edit_vertex")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_vertex_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        map.selected_vertices.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(vertex_id) = map.selected_vertices.first() {
            if let Some(vertex) = map.find_vertex(*vertex_id) {
                self.nodeui
                    .set_text_value("actionVertexName", vertex.name.clone());
                self.nodeui.set_bool_value(
                    "actionVertexTerrain",
                    vertex.properties.get_bool_default("terrain_control", false),
                );
                self.nodeui.set_f32_value(
                    "actionVertexTerrainSmoothness",
                    vertex.properties.get_float_default("smoothness", 1.0),
                );

                self.nodeui.set_f32_value("actionVertexX", vertex.x);
                self.nodeui.set_f32_value("actionVertexY", vertex.z);
                self.nodeui.set_f32_value("actionVertexZ", vertex.y);
            }
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let mut changed = false;
        let prev = map.clone();

        let name = self
            .nodeui
            .get_text_value("actionVertexName")
            .unwrap_or(String::new());

        let terrain_control = self
            .nodeui
            .get_bool_value("actionVertexTerrain")
            .unwrap_or(false);

        let terrain_smoothness = self
            .nodeui
            .get_f32_value("actionVertexTerrainSmoothness")
            .unwrap_or(1.0);

        let x = self.nodeui.get_f32_value("actionVertexX").unwrap_or(0.0);
        let y = self.nodeui.get_f32_value("actionVertexY").unwrap_or(0.0);
        let z = self.nodeui.get_f32_value("actionVertexZ").unwrap_or(0.0);

        if let Some(vertex_id) = map.selected_vertices.first() {
            if let Some(vertex) = map.find_vertex_mut(*vertex_id) {
                let ex_terrain_control =
                    vertex.properties.get_bool_default("terrain_control", false);

                if ex_terrain_control != terrain_control {
                    vertex
                        .properties
                        .set("terrain_control", Value::Bool(terrain_control));
                    changed = true;
                }

                let ex_terrain_smoothness = vertex.properties.get_float_default("smoothness", 1.0);
                if ex_terrain_smoothness != terrain_smoothness {
                    vertex
                        .properties
                        .set("smoothness", Value::Float(terrain_smoothness));
                    println!("set {}", terrain_smoothness);
                    changed = true;
                }

                if name != vertex.name {
                    vertex.name = name;
                    changed = true;
                }
                if x != vertex.x {
                    vertex.x = x;
                    changed = true;
                }
                // World space to vertex space mapping
                if y != vertex.z {
                    vertex.z = y;
                    changed = true;
                }
                if z != vertex.y {
                    vertex.y = z;
                    changed = true;
                }
            }
        }

        if changed {
            Some(ProjectUndoAtom::MapEdit(
                server_ctx.pc,
                Box::new(prev),
                Box::new(map.clone()),
            ))
        } else {
            None
        }
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
