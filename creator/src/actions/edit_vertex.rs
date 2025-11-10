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
            "Vertex Name".into(),
            "Set the name of the vertex.".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexX".into(),
            "X-Position".into(),
            "The x position of the vertex.".into(),
            0.0,
            -1000.0..=1000.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexY".into(),
            "Y-Position".into(),
            "The y position of the vertex.".into(),
            0.0,
            -1000.0..=1000.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexZ".into(),
            "Z-Position".into(),
            "The z position of the vertex.".into(),
            0.0,
            -1000.0..=1000.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Edit the attributes of the selected vertex. The XZ positions are the ground / 2D plane positions. The Y-position is up.".into(),
        );
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
        _server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
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
            Some(RegionUndoAtom::MapEdit(
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

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
