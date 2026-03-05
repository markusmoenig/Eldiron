use crate::prelude::*;
use rusterix::{PixelSource, Value};

pub struct CreateProp {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for CreateProp {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::OpenTree("table".into()));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionTableCreate".into(),
            "".into(),
            "".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTableHeight".into(),
            "".into(),
            "".into(),
            0.75,
            0.0..=10.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionTableChairs".into(),
            "".into(),
            "".into(),
            false,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionTableChairCount".into(),
            "".into(),
            "".into(),
            4,
            0..=8,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTableChairOffset".into(),
            "".into(),
            "".into(),
            0.45,
            0.0..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTableChairWidth".into(),
            "".into(),
            "".into(),
            0.85,
            0.20..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTableChairBackHeight".into(),
            "".into(),
            "".into(),
            1.0,
            0.25..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionTableChairTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionTableChairTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionMaterialTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionMaterialTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        Self {
            id: TheId::named("Create Prop"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Create or edit parametric props on selected sectors.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        !map.selected_sectors.is_empty()
            && server_ctx.editor_view_mode == EditorViewMode::D2
            && server_ctx.editing_surface.is_some()
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
        {
            let is_table = sector.properties.get_bool_default("profile_table", false)
                || sector.properties.get_int_default("profile_op", -1) == 1;
            self.nodeui.set_bool_value("actionTableCreate", is_table);
            self.nodeui.set_f32_value(
                "actionTableHeight",
                sector.properties.get_float_default("profile_amount", 0.75),
            );
            self.nodeui.set_bool_value(
                "actionTableChairs",
                sector.properties.get_bool_default("table_chairs", false),
            );
            self.nodeui.set_i32_value(
                "actionTableChairCount",
                sector.properties.get_int_default("table_chair_count", 4),
            );
            self.nodeui.set_f32_value(
                "actionTableChairOffset",
                sector
                    .properties
                    .get_float_default("table_chair_offset", 0.45),
            );
            self.nodeui.set_f32_value(
                "actionTableChairWidth",
                sector
                    .properties
                    .get_float_default("table_chair_width", 0.85),
            );
            self.nodeui.set_f32_value(
                "actionTableChairBackHeight",
                sector
                    .properties
                    .get_float_default("table_chair_back_height", 1.0),
            );

            let tile_id = if let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("cap_source")
            {
                *id
            } else {
                Uuid::nil()
            };
            self.nodeui.set_text_value(
                "actionMaterialTileId",
                if tile_id == Uuid::nil() {
                    String::new()
                } else {
                    tile_id.to_string()
                },
            );
            if let Some(item) = self.nodeui.get_item_mut("actionMaterialTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = tile_id;
            }

            let chair_tile_id = if let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("chair_source")
            {
                *id
            } else {
                Uuid::nil()
            };
            self.nodeui.set_text_value(
                "actionTableChairTileId",
                if chair_tile_id == Uuid::nil() {
                    String::new()
                } else {
                    chair_tile_id.to_string()
                },
            );
            if let Some(item) = self.nodeui.get_item_mut("actionTableChairTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = chair_tile_id;
            }
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();
        let mut chair_tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut chair_tile_id = Uuid::nil();

        if let Some(map) = project.get_map(server_ctx)
            && let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
            && let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("cap_source")
            && let Some(tile) = project.tiles.get(id)
            && !tile.is_empty()
        {
            tile_icon = tile.textures[0].to_rgba();
            tile_id = *id;
        }
        if let Some(map) = project.get_map(server_ctx)
            && let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
            && let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("chair_source")
            && let Some(tile) = project.tiles.get(id)
            && !tile.is_empty()
        {
            chair_tile_icon = tile.textures[0].to_rgba();
            chair_tile_id = *id;
        }

        if let Some(item) = self.nodeui.get_item_mut("actionMaterialTile")
            && let TheNodeUIItem::Icons(_, _, _, items) = item
            && items.len() == 1
        {
            items[0].0 = tile_icon;
            items[0].2 = tile_id;
        }
        if let Some(item) = self.nodeui.get_item_mut("actionTableChairTile")
            && let TheNodeUIItem::Icons(_, _, _, items) = item
            && items.len() == 1
        {
            items[0].0 = chair_tile_icon;
            items[0].2 = chair_tile_id;
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let create_table = self
            .nodeui
            .get_bool_value("actionTableCreate")
            .unwrap_or(true);
        if !create_table {
            return None;
        }

        let prev = map.clone();

        let height = self
            .nodeui
            .get_f32_value("actionTableHeight")
            .unwrap_or(0.75)
            .max(0.0);
        let chairs_enabled = self
            .nodeui
            .get_bool_value("actionTableChairs")
            .unwrap_or(false);
        let chair_count = self
            .nodeui
            .get_i32_value("actionTableChairCount")
            .unwrap_or(4)
            .clamp(0, 8);
        let chair_offset = self
            .nodeui
            .get_f32_value("actionTableChairOffset")
            .unwrap_or(0.45)
            .max(0.0);
        let chair_width = self
            .nodeui
            .get_f32_value("actionTableChairWidth")
            .unwrap_or(0.85)
            .max(0.20);
        let chair_back_height = self
            .nodeui
            .get_f32_value("actionTableChairBackHeight")
            .unwrap_or(1.0)
            .max(0.25);

        let tile_id = self
            .nodeui
            .get_tile_id("actionMaterialTile", 0)
            .unwrap_or(Uuid::nil());
        let tile_text = self
            .nodeui
            .get_text_value("actionMaterialTileId")
            .unwrap_or_default();
        let tile_id = if let Ok(id) = Uuid::parse_str(tile_text.trim()) {
            id
        } else {
            tile_id
        };
        let chair_tile_id = self
            .nodeui
            .get_tile_id("actionTableChairTile", 0)
            .unwrap_or(Uuid::nil());
        let chair_tile_text = self
            .nodeui
            .get_text_value("actionTableChairTileId")
            .unwrap_or_default();
        let chair_tile_id = if let Ok(id) = Uuid::parse_str(chair_tile_text.trim()) {
            id
        } else {
            chair_tile_id
        };

        let mut changed = false;
        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set("profile_op", Value::Int(1));
                sector
                    .properties
                    .set("profile_amount", Value::Float(height));
                sector.properties.set("profile_table", Value::Bool(true));
                sector
                    .properties
                    .set("table_chairs", Value::Bool(chairs_enabled));
                sector
                    .properties
                    .set("table_chair_count", Value::Int(chair_count));
                sector
                    .properties
                    .set("table_chair_offset", Value::Float(chair_offset));
                sector
                    .properties
                    .set("table_chair_width", Value::Float(chair_width));
                sector
                    .properties
                    .set("table_chair_back_height", Value::Float(chair_back_height));

                if tile_id != Uuid::nil() {
                    let src = Value::Source(PixelSource::TileId(tile_id));
                    sector.properties.set("cap_source", src.clone());
                    sector.properties.set("jamb_source", src);
                } else {
                    sector.properties.remove("cap_source");
                    sector.properties.remove("jamb_source");
                }
                if chair_tile_id != Uuid::nil() {
                    let src = Value::Source(PixelSource::TileId(chair_tile_id));
                    sector.properties.set("chair_source", src);
                } else {
                    sector.properties.remove("chair_source");
                }
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
            if id.name == "actionMaterialTile" {
                self.nodeui
                    .set_text_value("actionMaterialTileId", tile_id.to_string());
            } else if id.name == "actionTableChairTile" {
                self.nodeui
                    .set_text_value("actionTableChairTileId", tile_id.to_string());
            }
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Update Action List"),
                TheValue::Empty,
            ));
            return true;
        }
        self.nodeui.handle_event(event)
    }
}
