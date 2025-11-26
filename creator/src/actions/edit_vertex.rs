use crate::prelude::*;

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
            -1000.0..=1000.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexY".into(),
            fl!("action_edit_vertex_y"),
            fl!("status_action_edit_vertex_y"),
            0.0,
            -1000.0..=1000.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexZ".into(),
            fl!("action_edit_vertex_z"),
            fl!("status_action_edit_vertex_z"),
            0.0,
            -1000.0..=1000.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_edit_vertex_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named("Edit Vertex"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Edit vertex attributes."
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
        let x = self.nodeui.get_f32_value("actionVertexX").unwrap_or(0.0);
        let y = self.nodeui.get_f32_value("actionVertexY").unwrap_or(0.0);
        let z = self.nodeui.get_f32_value("actionVertexZ").unwrap_or(0.0);

        if let Some(vertex_id) = map.selected_vertices.first() {
            if let Some(vertex) = map.find_vertex_mut(*vertex_id) {
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
