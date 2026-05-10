use crate::prelude::*;
use rusterix::{PixelSource, Value};
use std::str::FromStr;

pub const EDIT_VERTEX_ACTION_ID: &str = "260fcd81-c456-4ad4-894c-85e7552c856f";

pub struct EditVertex {
    id: TheId,
    nodeui: TheNodeUI,
}

impl EditVertex {
    fn build_nodeui() -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::Text(
            "actionVertexName".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));

        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionVertexX".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionVertexY".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionVertexZ".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        ));

        nodeui.add_item(TheNodeUIItem::OpenTree("billboard".into()));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionSize".into(),
            "".into(),
            "".into(),
            1.0,
            0.0..=0.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui
    }
}

impl Action for EditVertex {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_vertex"),
                Uuid::from_str(EDIT_VERTEX_ACTION_ID).unwrap(),
            ),
            nodeui: Self::build_nodeui(),
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

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode == EditorViewMode::D2 && map.selected_vertices.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(vertex_id) = map.selected_vertices.first()
            && let Some(vertex) = map.find_vertex(*vertex_id)
        {
            self.nodeui
                .set_text_value("actionVertexName", vertex.name.clone());
            self.nodeui.set_f32_value(
                "actionSize",
                vertex.properties.get_float_default("source_size", 1.0),
            );

            let billboard_tile_id =
                if let Some(Value::Source(PixelSource::TileId(id))) = vertex.properties.get("source")
                {
                    *id
                } else {
                    Uuid::nil()
                };
            self.nodeui.set_text_value(
                "actionTileId",
                if billboard_tile_id == Uuid::nil() {
                    String::new()
                } else {
                    billboard_tile_id.to_string()
                },
            );
            if let Some(item) = self.nodeui.get_item_mut("actionTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = billboard_tile_id;
            }

            self.nodeui.set_f32_value("actionVertexX", vertex.x);
            self.nodeui.set_f32_value("actionVertexY", vertex.z);
            self.nodeui.set_f32_value("actionVertexZ", vertex.y);
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();

        if let Some(map) = project.get_map(server_ctx)
            && let Some(vertex_id) = map.selected_vertices.first()
            && let Some(vertex) = map.find_vertex(*vertex_id)
            && let Some(Value::Source(PixelSource::TileId(id))) = vertex.properties.get("source")
            && let Some(tile) = project.tiles.get(id)
            && !tile.is_empty()
        {
            tile_icon = tile.textures[0].to_rgba();
            tile_id = *id;
        }

        if let Some(item) = self.nodeui.get_item_mut("actionTile")
            && let TheNodeUIItem::Icons(_, _, _, items) = item
            && items.len() == 1
        {
            items[0].0 = tile_icon;
            items[0].2 = tile_id;
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
            .unwrap_or_default();
        let tile_size = self.nodeui.get_f32_value("actionSize").unwrap_or(1.0);
        let tile_id = self
            .nodeui
            .get_tile_id("actionTile", 0)
            .unwrap_or(Uuid::nil());
        let tile_id_text = self
            .nodeui
            .get_text_value("actionTileId")
            .unwrap_or_default();
        let source = parse_tile_id_pixelsource(&tile_id_text)
            .or_else(|| (tile_id != Uuid::nil()).then_some(PixelSource::TileId(tile_id)));

        let x = self.nodeui.get_f32_value("actionVertexX").unwrap_or(0.0);
        let y = self.nodeui.get_f32_value("actionVertexY").unwrap_or(0.0);
        let z = self.nodeui.get_f32_value("actionVertexZ").unwrap_or(0.0);

        if let Some(vertex_id) = map.selected_vertices.first()
            && let Some(vertex) = map.find_vertex_mut(*vertex_id)
        {
            let existing_tile_size = vertex.properties.get_float_default("source_size", 1.0);
            if existing_tile_size != tile_size {
                vertex
                    .properties
                    .set("source_size", Value::Float(tile_size));
                changed = true;
            }

            if let Some(source) = source {
                let has_changed = match vertex.properties.get("source") {
                    Some(Value::Source(existing)) => *existing != source,
                    _ => true,
                };
                if has_changed {
                    vertex.properties.set("source", Value::Source(source));
                    changed = true;
                }
            } else if vertex.properties.contains("source") {
                vertex.properties.remove("source");
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
            // World space to vertex space mapping.
            if y != vertex.z {
                vertex.z = y;
                changed = true;
            }
            if z != vertex.y {
                vertex.y = z;
                changed = true;
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

    fn hud_material_slots(
        &self,
        map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        crate::actions::builder_hud_material_slots_for_selected_vertex(map)
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        project: &mut Project,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &mut ServerContext,
    ) -> bool {
        if let TheEvent::TileDropped(id, tile_id, index) = event
            && id.name == "actionTile"
            && let Some(item) = self.nodeui.get_item_mut(&id.name)
            && let TheNodeUIItem::Icons(_, _, _, items) = item
            && *index < items.len()
            && let Some(tile) = project.tiles.get(tile_id)
            && !tile.is_empty()
        {
            items[*index].0 = tile.textures[0].to_rgba();
            items[*index].2 = *tile_id;
            self.nodeui
                .set_text_value("actionTileId", tile_id.to_string());
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Action List"),
                TheValue::Empty,
            ));
            return true;
        }
        self.nodeui.handle_event(event)
    }
}
