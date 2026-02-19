use crate::prelude::*;
use rusterix::{PixelSource, Value};
use std::str::FromStr;

pub const EDIT_VERTEX_ACTION_ID: &str = "260fcd81-c456-4ad4-894c-85e7552c856f";

pub struct EditVertex {
    id: TheId,
    nodeui: TheNodeUI,
    show_terrain: bool,
}

impl EditVertex {
    fn build_nodeui(show_terrain: bool) -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "actionVertexName".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexX".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexY".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionVertexZ".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=0.0,
            false,
        );
        nodeui.add_item(item);

        if show_terrain {
            nodeui.add_item(TheNodeUIItem::OpenTree("terrain".into()));
            nodeui.add_item(TheNodeUIItem::Checkbox(
                "actionTerrain".into(),
                "".into(),
                "".into(),
                false,
            ));

            let item = TheNodeUIItem::FloatEditSlider(
                "actionTerrainSmoothness".into(),
                "".into(),
                "".into(),
                0.0,
                0.0..=0.0,
                false,
            );
            nodeui.add_item(item);

            nodeui.add_item(TheNodeUIItem::Icons(
                "actionTerrainTile".into(),
                "".into(),
                "".into(),
                vec![(
                    TheRGBABuffer::new(TheDim::sized(36, 36)),
                    "".to_string(),
                    Uuid::nil(),
                )],
            ));

            nodeui.add_item(TheNodeUIItem::Text(
                "actionTerrainTileId".into(),
                "".into(),
                "".into(),
                "".into(),
                None,
                false,
            ));

            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "actionTerrainTileFalloff".into(),
                "".into(),
                "".into(),
                1.0,
                0.0..=16.0,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::CloseTree);
        }

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
        let nodeui = Self::build_nodeui(true);

        // let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_edit_vertex_desc"));
        // nodeui.add_item(item);

        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_vertex"),
                Uuid::from_str(EDIT_VERTEX_ACTION_ID).unwrap(),
            ),
            nodeui,
            show_terrain: true,
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
                    "actionTerrain",
                    vertex.properties.get_bool_default("terrain_control", false),
                );
                self.nodeui.set_f32_value(
                    "actionTerrainSmoothness",
                    vertex.properties.get_float_default("smoothness", 1.0),
                );
                self.nodeui.set_f32_value(
                    "actionTerrainTileFalloff",
                    vertex
                        .properties
                        .get_float_default("terrain_tile_falloff", 1.0),
                );
                let terrain_tile_id = if let Some(Value::Source(PixelSource::TileId(id))) =
                    vertex.properties.get("terrain_source")
                {
                    *id
                } else {
                    Uuid::nil()
                };
                self.nodeui.set_text_value(
                    "actionTerrainTileId",
                    if terrain_tile_id == Uuid::nil() {
                        String::new()
                    } else {
                        terrain_tile_id.to_string()
                    },
                );
                if let Some(item) = self.nodeui.get_item_mut("actionTerrainTile")
                    && let TheNodeUIItem::Icons(_, _, _, items) = item
                    && items.len() == 1
                {
                    items[0].2 = terrain_tile_id;
                }

                self.nodeui.set_f32_value(
                    "actionSize",
                    vertex.properties.get_float_default("source_size", 1.0),
                );
                let billboard_tile_id = if let Some(Value::Source(PixelSource::TileId(id))) =
                    vertex.properties.get("source")
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
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let show_terrain = server_ctx.get_map_context() == MapContext::Region;
        if show_terrain != self.show_terrain {
            let name = self
                .nodeui
                .get_text_value("actionVertexName")
                .unwrap_or_default();
            let x = self.nodeui.get_f32_value("actionVertexX").unwrap_or(0.0);
            let y = self.nodeui.get_f32_value("actionVertexY").unwrap_or(0.0);
            let z = self.nodeui.get_f32_value("actionVertexZ").unwrap_or(0.0);
            let tile_size = self.nodeui.get_f32_value("actionSize").unwrap_or(1.0);
            let tile_id_text = self
                .nodeui
                .get_text_value("actionTileId")
                .unwrap_or_default();
            let tile_id = self
                .nodeui
                .get_tile_id("actionTile", 0)
                .unwrap_or(Uuid::nil());
            let terrain_control = self.nodeui.get_bool_value("actionTerrain").unwrap_or(false);
            let terrain_smoothness = self
                .nodeui
                .get_f32_value("actionTerrainSmoothness")
                .unwrap_or(1.0);
            let terrain_tile_falloff = self
                .nodeui
                .get_f32_value("actionTerrainTileFalloff")
                .unwrap_or(1.0);
            let terrain_tile_id_text = self
                .nodeui
                .get_text_value("actionTerrainTileId")
                .unwrap_or_default();
            let terrain_tile_id = self
                .nodeui
                .get_tile_id("actionTerrainTile", 0)
                .unwrap_or(Uuid::nil());

            self.nodeui = Self::build_nodeui(show_terrain);
            self.show_terrain = show_terrain;

            self.nodeui.set_text_value("actionVertexName", name);
            self.nodeui.set_f32_value("actionVertexX", x);
            self.nodeui.set_f32_value("actionVertexY", y);
            self.nodeui.set_f32_value("actionVertexZ", z);
            self.nodeui.set_f32_value("actionSize", tile_size);
            self.nodeui.set_text_value("actionTileId", tile_id_text);
            if let Some(item) = self.nodeui.get_item_mut("actionTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = tile_id;
            }
            self.nodeui.set_bool_value("actionTerrain", terrain_control);
            self.nodeui
                .set_f32_value("actionTerrainSmoothness", terrain_smoothness);
            self.nodeui
                .set_f32_value("actionTerrainTileFalloff", terrain_tile_falloff);
            self.nodeui
                .set_text_value("actionTerrainTileId", terrain_tile_id_text);
            if let Some(item) = self.nodeui.get_item_mut("actionTerrainTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = terrain_tile_id;
            }
        }

        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();
        let mut terrain_tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut terrain_tile_id = Uuid::nil();

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
                    if let Some(Value::Source(PixelSource::TileId(id))) =
                        vertex.properties.get("terrain_source")
                    {
                        if let Some(tile) = project.tiles.get(id)
                            && !tile.is_empty()
                        {
                            terrain_tile_icon = tile.textures[0].to_rgba();
                            terrain_tile_id = *id;
                        }
                    }
                }
            }
        }

        if let Some(item) = self.nodeui.get_item_mut("actionTile") {
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

        if let Some(item) = self.nodeui.get_item_mut("actionTerrainTile") {
            match item {
                TheNodeUIItem::Icons(_, _, _, items) => {
                    if items.len() == 1 {
                        items[0].0 = terrain_tile_icon;
                        items[0].2 = terrain_tile_id;
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

        let terrain_control = self.nodeui.get_bool_value("actionTerrain").unwrap_or(false);

        let terrain_smoothness = self
            .nodeui
            .get_f32_value("actionTerrainSmoothness")
            .unwrap_or(1.0);
        let terrain_tile_falloff = self
            .nodeui
            .get_f32_value("actionTerrainTileFalloff")
            .unwrap_or(1.0);
        let terrain_tile_id = self
            .nodeui
            .get_tile_id("actionTerrainTile", 0)
            .unwrap_or(Uuid::nil());
        let terrain_tile_id_text = self
            .nodeui
            .get_text_value("actionTerrainTileId")
            .unwrap_or_default();
        let terrain_tile_id = if let Ok(id) = Uuid::parse_str(terrain_tile_id_text.trim()) {
            id
        } else {
            terrain_tile_id
        };
        let is_region_mode = server_ctx.get_map_context() == MapContext::Region;

        let tile_size = self.nodeui.get_f32_value("actionSize").unwrap_or(1.0);

        let tile_id = self
            .nodeui
            .get_tile_id("actionTile", 0)
            .unwrap_or(Uuid::nil());
        let tile_id_text = self
            .nodeui
            .get_text_value("actionTileId")
            .unwrap_or_default();
        let tile_id = if let Ok(id) = Uuid::parse_str(tile_id_text.trim()) {
            id
        } else {
            tile_id
        };

        let x = self.nodeui.get_f32_value("actionVertexX").unwrap_or(0.0);
        let y = self.nodeui.get_f32_value("actionVertexY").unwrap_or(0.0);
        let z = self.nodeui.get_f32_value("actionVertexZ").unwrap_or(0.0);

        if let Some(vertex_id) = map.selected_vertices.first() {
            if let Some(vertex) = map.find_vertex_mut(*vertex_id) {
                if is_region_mode {
                    let ex_terrain_control =
                        vertex.properties.get_bool_default("terrain_control", false);

                    if ex_terrain_control != terrain_control {
                        vertex
                            .properties
                            .set("terrain_control", Value::Bool(terrain_control));
                        changed = true;
                    }

                    let ex_terrain_smoothness =
                        vertex.properties.get_float_default("smoothness", 1.0);
                    if ex_terrain_smoothness != terrain_smoothness {
                        vertex
                            .properties
                            .set("smoothness", Value::Float(terrain_smoothness));
                        changed = true;
                    }

                    let ex_terrain_tile_falloff = vertex
                        .properties
                        .get_float_default("terrain_tile_falloff", 1.0);
                    if (ex_terrain_tile_falloff - terrain_tile_falloff).abs() > f32::EPSILON {
                        vertex.properties.set(
                            "terrain_tile_falloff",
                            Value::Float(terrain_tile_falloff.max(0.0)),
                        );
                        changed = true;
                    }

                    match terrain_tile_id {
                        id if id != Uuid::nil() => {
                            let has_changed = match vertex.properties.get("terrain_source") {
                                Some(Value::Source(PixelSource::TileId(existing))) => {
                                    *existing != id
                                }
                                _ => true,
                            };
                            if has_changed {
                                vertex
                                    .properties
                                    .set("terrain_source", Value::Source(PixelSource::TileId(id)));
                                changed = true;
                            }
                        }
                        _ => {
                            if vertex.properties.contains("terrain_source") {
                                vertex.properties.remove("terrain_source");
                                changed = true;
                            }
                        }
                    }
                }

                let ex_tile_size = vertex.properties.get_float_default("source_size", 1.0);
                if ex_tile_size != tile_size {
                    vertex
                        .properties
                        .set("source_size", Value::Float(tile_size));
                    changed = true;
                }

                if tile_id != Uuid::nil() {
                    let has_changed = match vertex.properties.get("source") {
                        Some(Value::Source(PixelSource::TileId(existing))) => *existing != tile_id,
                        _ => true,
                    };
                    if has_changed {
                        vertex.properties.set(
                            "source",
                            Value::Source(rusterix::PixelSource::TileId(tile_id)),
                        );
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
                                if id.name == "actionTile" {
                                    self.nodeui
                                        .set_text_value("actionTileId", tile_id.to_string());
                                }
                                if id.name == "actionTerrainTile" {
                                    self.nodeui
                                        .set_text_value("actionTerrainTileId", tile_id.to_string());
                                }
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
