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
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionTableConnectionMode".into(),
            "".into(),
            "".into(),
            vec!["hard".into(), "smooth".into(), "bevel".into()],
            2,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionTableBevelSegments".into(),
            "".into(),
            "".into(),
            4,
            1..=16,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTableBevelRadius".into(),
            "".into(),
            "".into(),
            0.25,
            0.0..=5.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("billboard".into()));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionBillboardTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionBillboardTileId".into(),
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
            let is_table = sector.properties.get_int_default("profile_op", -1) == 1;
            self.nodeui.set_bool_value("actionTableCreate", is_table);
            self.nodeui.set_f32_value(
                "actionTableHeight",
                sector.properties.get_float_default("profile_amount", 0.75),
            );
            self.nodeui.set_i32_value(
                "actionTableConnectionMode",
                sector.properties.get_int_default("connection_mode", 2),
            );
            self.nodeui.set_i32_value(
                "actionTableBevelSegments",
                sector.properties.get_int_default("bevel_segments", 4),
            );
            self.nodeui.set_f32_value(
                "actionTableBevelRadius",
                sector.properties.get_float_default("bevel_radius", 0.25),
            );

            let tile_id = if let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("cap_source")
            {
                *id
            } else {
                Uuid::nil()
            };
            self.nodeui.set_text_value(
                "actionBillboardTileId",
                if tile_id == Uuid::nil() {
                    String::new()
                } else {
                    tile_id.to_string()
                },
            );
            if let Some(item) = self.nodeui.get_item_mut("actionBillboardTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = tile_id;
            }
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();

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

        if let Some(item) = self.nodeui.get_item_mut("actionBillboardTile")
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
        let connection_mode = self
            .nodeui
            .get_i32_value("actionTableConnectionMode")
            .unwrap_or(2)
            .clamp(0, 2);
        let bevel_segments = self
            .nodeui
            .get_i32_value("actionTableBevelSegments")
            .unwrap_or(4)
            .clamp(1, 16);
        let bevel_radius = self
            .nodeui
            .get_f32_value("actionTableBevelRadius")
            .unwrap_or(0.25)
            .max(0.0);

        let tile_id = self
            .nodeui
            .get_tile_id("actionBillboardTile", 0)
            .unwrap_or(Uuid::nil());
        let tile_text = self
            .nodeui
            .get_text_value("actionBillboardTileId")
            .unwrap_or_default();
        let tile_id = if let Ok(id) = Uuid::parse_str(tile_text.trim()) {
            id
        } else {
            tile_id
        };

        let mut changed = false;
        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set("profile_op", Value::Int(1));
                sector
                    .properties
                    .set("profile_amount", Value::Float(height));
                sector
                    .properties
                    .set("connection_mode", Value::Int(connection_mode));
                sector
                    .properties
                    .set("bevel_segments", Value::Int(bevel_segments));
                sector
                    .properties
                    .set("bevel_radius", Value::Float(bevel_radius));

                if tile_id != Uuid::nil() {
                    let src = Value::Source(PixelSource::TileId(tile_id));
                    sector.properties.set("cap_source", src.clone());
                    sector.properties.set("jamb_source", src);
                } else {
                    sector.properties.remove("cap_source");
                    sector.properties.remove("jamb_source");
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
            if id.name == "actionBillboardTile" {
                self.nodeui
                    .set_text_value("actionBillboardTileId", tile_id.to_string());
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
