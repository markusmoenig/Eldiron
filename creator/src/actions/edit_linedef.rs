use crate::prelude::*;
use rusterix::PixelSource;
use std::str::FromStr;

pub const EDIT_LINEDEF_ACTION_ID: &str = "284638fa-5769-442a-a55e-88121a37f193";

pub struct EditLinedef {
    id: TheId,
    nodeui: TheNodeUI,
    show_terrain: bool,
}

impl EditLinedef {
    fn build_nodeui(show_terrain: bool) -> TheNodeUI {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::Text(
            "actionLinedefName".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        );
        nodeui.add_item(item);

        if show_terrain {
            nodeui.add_item(TheNodeUIItem::OpenTree("terrain".into()));
            nodeui.add_item(TheNodeUIItem::Checkbox(
                "actionTerrainSmooth".into(),
                "".into(),
                "".into(),
                false,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "actionTerrainWidth".into(),
                "".into(),
                "".into(),
                2.0,
                0.0..=128.0,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "actionTerrainFalloffDistance".into(),
                "".into(),
                "".into(),
                3.0,
                0.0..=128.0,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                "actionTerrainFalloffSteepness".into(),
                "".into(),
                "".into(),
                2.0,
                0.1..=16.0,
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
                "actionTileId".into(),
                "".into(),
                "".into(),
                "".into(),
                None,
                false,
            ));
            nodeui.add_item(TheNodeUIItem::CloseTree);
        }

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        nodeui
    }
}

impl Action for EditLinedef {
    fn new() -> Self
    where
        Self: Sized,
    {
        let nodeui = Self::build_nodeui(true);

        Self {
            id: TheId::named_with_id(
                &fl!("action_edit_linedef"),
                Uuid::from_str(EDIT_LINEDEF_ACTION_ID).unwrap(),
            ),
            nodeui,
            show_terrain: true,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edit_linedef_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, _server_ctx: &ServerContext) -> bool {
        map.selected_linedefs.len() == 1
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(linedef_id) = map.selected_linedefs.first() {
            if let Some(linedef) = map.find_linedef(*linedef_id) {
                self.nodeui
                    .set_text_value("actionLinedefName", linedef.name.clone());
                self.nodeui.set_bool_value(
                    "actionTerrainSmooth",
                    linedef.properties.get_bool_default("terrain_smooth", false),
                );
                self.nodeui.set_f32_value(
                    "actionTerrainWidth",
                    linedef.properties.get_float_default("terrain_width", 2.0),
                );
                self.nodeui.set_f32_value(
                    "actionTerrainFalloffDistance",
                    linedef
                        .properties
                        .get_float_default("terrain_falloff_distance", 3.0),
                );
                self.nodeui.set_f32_value(
                    "actionTerrainFalloffSteepness",
                    linedef
                        .properties
                        .get_float_default("terrain_falloff_steepness", 2.0),
                );
                self.nodeui.set_f32_value(
                    "actionTerrainTileFalloff",
                    linedef
                        .properties
                        .get_float_default("terrain_tile_falloff", 1.0),
                );
                let terrain_tile_id = if let Some(Value::Source(PixelSource::TileId(id))) =
                    linedef.properties.get("terrain_source")
                {
                    *id
                } else {
                    Uuid::nil()
                };
                self.nodeui.set_text_value(
                    "actionTileId",
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
            }
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let show_terrain = server_ctx.get_map_context() == MapContext::Region;
        if show_terrain != self.show_terrain {
            let name = self
                .nodeui
                .get_text_value("actionLinedefName")
                .unwrap_or_default();
            let terrain_smooth = self
                .nodeui
                .get_bool_value("actionTerrainSmooth")
                .unwrap_or(false);
            let terrain_width = self
                .nodeui
                .get_f32_value("actionTerrainWidth")
                .unwrap_or(2.0);
            let terrain_falloff_distance = self
                .nodeui
                .get_f32_value("actionTerrainFalloffDistance")
                .unwrap_or(3.0);
            let terrain_falloff_steepness = self
                .nodeui
                .get_f32_value("actionTerrainFalloffSteepness")
                .unwrap_or(2.0);
            let terrain_tile_falloff = self
                .nodeui
                .get_f32_value("actionTerrainTileFalloff")
                .unwrap_or(1.0);
            let tile_id_text = self
                .nodeui
                .get_text_value("actionTileId")
                .unwrap_or_default();
            let terrain_tile_id = self
                .nodeui
                .get_tile_id("actionTerrainTile", 0)
                .unwrap_or(Uuid::nil());

            self.nodeui = Self::build_nodeui(show_terrain);
            self.show_terrain = show_terrain;

            self.nodeui.set_text_value("actionLinedefName", name);
            self.nodeui
                .set_bool_value("actionTerrainSmooth", terrain_smooth);
            self.nodeui
                .set_f32_value("actionTerrainWidth", terrain_width);
            self.nodeui
                .set_f32_value("actionTerrainFalloffDistance", terrain_falloff_distance);
            self.nodeui
                .set_f32_value("actionTerrainFalloffSteepness", terrain_falloff_steepness);
            self.nodeui
                .set_f32_value("actionTerrainTileFalloff", terrain_tile_falloff);
            self.nodeui.set_text_value("actionTileId", tile_id_text);
            if let Some(item) = self.nodeui.get_item_mut("actionTerrainTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = terrain_tile_id;
            }
        }

        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();
        if let Some(map) = project.get_map(server_ctx)
            && let Some(linedef_id) = map.selected_linedefs.first()
            && let Some(linedef) = map.find_linedef(*linedef_id)
            && let Some(Value::Source(PixelSource::TileId(id))) =
                linedef.properties.get("terrain_source")
            && let Some(tile) = project.tiles.get(id)
            && !tile.is_empty()
        {
            tile_icon = tile.textures[0].to_rgba();
            tile_id = *id;
        }

        if let Some(item) = self.nodeui.get_item_mut("actionTerrainTile")
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
            .get_text_value("actionLinedefName")
            .unwrap_or(String::new());
        let terrain_smooth = self
            .nodeui
            .get_bool_value("actionTerrainSmooth")
            .unwrap_or(false);
        let terrain_width = self
            .nodeui
            .get_f32_value("actionTerrainWidth")
            .unwrap_or(2.0);
        let terrain_falloff_distance = self
            .nodeui
            .get_f32_value("actionTerrainFalloffDistance")
            .unwrap_or(3.0);
        let terrain_falloff_steepness = self
            .nodeui
            .get_f32_value("actionTerrainFalloffSteepness")
            .unwrap_or(2.0);
        let terrain_tile_falloff = self
            .nodeui
            .get_f32_value("actionTerrainTileFalloff")
            .unwrap_or(1.0);
        let terrain_tile_id = self.nodeui.get_tile_id("actionTerrainTile", 0);
        let tile_id_text = self
            .nodeui
            .get_text_value("actionTileId")
            .unwrap_or_default();
        let terrain_tile_id = if let Ok(id) = Uuid::parse_str(tile_id_text.trim()) {
            Some(id)
        } else {
            terrain_tile_id
        };

        if let Some(linedef_id) = map.selected_linedefs.first() {
            if let Some(linedef) = map.find_linedef_mut(*linedef_id) {
                if name != linedef.name {
                    linedef.name = name;
                    changed = true;
                }
                if linedef.properties.get_bool_default("terrain_smooth", false) != terrain_smooth {
                    linedef
                        .properties
                        .set("terrain_smooth", Value::Bool(terrain_smooth));
                    changed = true;
                }
                if (linedef.properties.get_float_default("terrain_width", 2.0) - terrain_width)
                    .abs()
                    > 0.0001
                {
                    linedef
                        .properties
                        .set("terrain_width", Value::Float(terrain_width));
                    changed = true;
                }
                if (linedef
                    .properties
                    .get_float_default("terrain_falloff_distance", 3.0)
                    - terrain_falloff_distance)
                    .abs()
                    > 0.0001
                {
                    linedef.properties.set(
                        "terrain_falloff_distance",
                        Value::Float(terrain_falloff_distance),
                    );
                    changed = true;
                }
                if (linedef
                    .properties
                    .get_float_default("terrain_falloff_steepness", 2.0)
                    - terrain_falloff_steepness)
                    .abs()
                    > 0.0001
                {
                    linedef.properties.set(
                        "terrain_falloff_steepness",
                        Value::Float(terrain_falloff_steepness),
                    );
                    changed = true;
                }
                if (linedef
                    .properties
                    .get_float_default("terrain_tile_falloff", 1.0)
                    - terrain_tile_falloff)
                    .abs()
                    > 0.0001
                {
                    linedef.properties.set(
                        "terrain_tile_falloff",
                        Value::Float(terrain_tile_falloff.max(0.0)),
                    );
                    changed = true;
                }
                match terrain_tile_id {
                    Some(id) if id != Uuid::nil() => {
                        let has_changed = match linedef.properties.get("terrain_source") {
                            Some(Value::Source(PixelSource::TileId(existing))) => *existing != id,
                            _ => true,
                        };
                        if has_changed {
                            linedef
                                .properties
                                .set("terrain_source", Value::Source(PixelSource::TileId(id)));
                            changed = true;
                        }
                    }
                    _ => {
                        if linedef.properties.contains("terrain_source") {
                            linedef.properties.remove("terrain_source");
                            changed = true;
                        }
                    }
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
        if let TheEvent::TileDropped(id, tile_id, index) = event
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
