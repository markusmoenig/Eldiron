use crate::prelude::*;
use rusterix::{PixelSource, Value};

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

        let item = TheNodeUIItem::Icons(
            "actionVertexTile".into(),
            fl!("icons"),
            fl!("status_action_recess_tiles"),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
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
                self.nodeui.set_f32_value("actionVertexX", vertex.x);
                self.nodeui.set_f32_value("actionVertexY", vertex.z);
                self.nodeui.set_f32_value("actionVertexZ", vertex.y);
            }
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();

        if let Some(map) = project.get_map(server_ctx) {
            if let Some(vertex_id) = map.selected_vertices.first() {
                if let Some(vertex) = map.find_vertex(*vertex_id) {
                    if let Some(Value::Source(PixelSource::TileId(id))) =
                        vertex.properties.get("source")
                    {
                        if let Some(tile) = project.tiles.get(id)
                            && !tile.is_empty()
                        {
                            tile_icon = tile.textures[0].to_rgba();
                            tile_id = *id;
                        }
                    }
                }
            }
        }

        if let Some(item) = self.nodeui.get_item_mut("actionVertexTile") {
            match item {
                TheNodeUIItem::Icons(_, _, _, items) => {
                    if items.len() == 1 {
                        items[0].0 = tile_icon;
                        items[0].2 = tile_id;
                    }
                }
                _ => {}
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

        let tile_id = self.nodeui.get_tile_id("actionVertexTile", 0);

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

                if let Some(tile_id) = tile_id
                    && tile_id != Uuid::nil()
                {
                    vertex.properties.set(
                        "source",
                        Value::Source(rusterix::PixelSource::TileId(tile_id)),
                    );
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
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::TileDropped(id, tile_id, index) = event {
            if let Some(item) = self.nodeui.get_item_mut(&id.name) {
                match item {
                    TheNodeUIItem::Icons(_, _, _, items) => {
                        if *index < items.len() {
                            if let Some(tile) = project.tiles.get(tile_id)
                                && !tile.is_empty()
                            {
                                items[*index].0 = tile.textures[0].to_rgba();
                                items[*index].2 = *tile_id;
                                ctx.ui.send(TheEvent::Custom(
                                    TheId::named("Update Action List"),
                                    TheValue::Empty,
                                ));
                                return true;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        self.nodeui.handle_event(event)
    }
}
