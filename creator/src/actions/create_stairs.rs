use crate::prelude::*;
use rusterix::PixelSource;
use std::str::FromStr;

pub const CREATE_STAIRS_ACTION_ID: &str = "4f4d41d0-7f67-4c1f-a8d2-f0ab4a0be6a1";

pub struct CreateStairs {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CreateStairs {
    fn parse_tile_source(text: &str) -> Option<Value> {
        let id = Uuid::parse_str(text.trim()).ok()?;
        Some(Value::Source(PixelSource::TileId(id)))
    }

    fn apply_sector_stairs(&self, map: &mut Map, sector_id: u32) -> bool {
        let direction = self
            .nodeui
            .get_i32_value("actionStairsDirection")
            .unwrap_or(0)
            .clamp(0, 3);
        let steps = self
            .nodeui
            .get_i32_value("actionStairsSteps")
            .unwrap_or(6)
            .max(1);
        let total_height = self
            .nodeui
            .get_f32_value("actionStairsTotalHeight")
            .unwrap_or(1.0)
            .max(0.0);
        let fill_sides = self
            .nodeui
            .get_bool_value("actionStairsFillSides")
            .unwrap_or(true);

        let tile_id_text = self
            .nodeui
            .get_text_value("actionStairsTileId")
            .unwrap_or_default();
        let tread_tile_id_text = self
            .nodeui
            .get_text_value("actionStairsTreadTileId")
            .unwrap_or_default();
        let riser_tile_id_text = self
            .nodeui
            .get_text_value("actionStairsRiserTileId")
            .unwrap_or_default();
        let side_tile_id_text = self
            .nodeui
            .get_text_value("actionStairsSideTileId")
            .unwrap_or_default();

        let Some(sector) = map.find_sector_mut(sector_id) else {
            return false;
        };

        if total_height <= 0.0 {
            sector
                .properties
                .set("sector_feature", Value::Str("None".to_string()));
            return true;
        }

        sector
            .properties
            .set("sector_feature", Value::Str("Stairs".to_string()));
        sector
            .properties
            .set("stairs_direction", Value::Int(direction));
        sector.properties.set("stairs_steps", Value::Int(steps));
        sector
            .properties
            .set("stairs_total_height", Value::Float(total_height));
        sector
            .properties
            .set("stairs_fill_sides", Value::Bool(fill_sides));

        if let Some(src) = Self::parse_tile_source(&tile_id_text) {
            sector.properties.set("stairs_tile_source", src);
        } else {
            sector.properties.remove("stairs_tile_source");
        }
        if let Some(src) = Self::parse_tile_source(&tread_tile_id_text) {
            sector.properties.set("stairs_tread_source", src);
        } else {
            sector.properties.remove("stairs_tread_source");
        }
        if let Some(src) = Self::parse_tile_source(&riser_tile_id_text) {
            sector.properties.set("stairs_riser_source", src);
        } else {
            sector.properties.remove("stairs_riser_source");
        }
        if let Some(src) = Self::parse_tile_source(&side_tile_id_text) {
            sector.properties.set("stairs_side_source", src);
        } else {
            sector.properties.remove("stairs_side_source");
        }

        true
    }
}

impl Action for CreateStairs {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::OpenTree("stairs".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionStairsDirection".into(),
            "".into(),
            "".into(),
            vec!["north".into(), "east".into(), "south".into(), "west".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionStairsSteps".into(),
            "".into(),
            "".into(),
            6,
            1..=64,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionStairsTotalHeight".into(),
            "".into(),
            "".into(),
            1.0,
            0.0..=16.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionStairsFillSides".into(),
            "".into(),
            "".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsTreadTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsRiserTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionStairsSideTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), "".into()));

        Self {
            id: TheId::named_with_id(
                "Create Stairs",
                Uuid::from_str(CREATE_STAIRS_ACTION_ID).unwrap(),
            ),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Configure non-destructive stairs on selected sectors.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
            return false;
        }
        !map.selected_sectors.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let Some(sector_id) = map.selected_sectors.first() else {
            return;
        };
        let Some(sector) = map.find_sector(*sector_id) else {
            return;
        };

        self.nodeui.set_i32_value(
            "actionStairsDirection",
            sector.properties.get_int_default("stairs_direction", 0),
        );
        self.nodeui.set_i32_value(
            "actionStairsSteps",
            sector.properties.get_int_default("stairs_steps", 6),
        );
        self.nodeui.set_f32_value(
            "actionStairsTotalHeight",
            sector
                .properties
                .get_float_default("stairs_total_height", 1.0),
        );
        self.nodeui.set_bool_value(
            "actionStairsFillSides",
            sector
                .properties
                .get_bool_default("stairs_fill_sides", true),
        );

        let tile_id_text = match sector.properties.get("stairs_tile_source") {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            _ => String::new(),
        };
        let tread_tile_id_text = match sector.properties.get("stairs_tread_source") {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            _ => String::new(),
        };
        let riser_tile_id_text = match sector.properties.get("stairs_riser_source") {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            _ => String::new(),
        };
        let side_tile_id_text = match sector.properties.get("stairs_side_source") {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            _ => String::new(),
        };

        self.nodeui
            .set_text_value("actionStairsTileId", tile_id_text);
        self.nodeui
            .set_text_value("actionStairsTreadTileId", tread_tile_id_text);
        self.nodeui
            .set_text_value("actionStairsRiserTileId", riser_tile_id_text);
        self.nodeui
            .set_text_value("actionStairsSideTileId", side_tile_id_text);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prev = map.clone();
        let mut changed = false;

        for sector_id in map.selected_sectors.clone() {
            changed |= self.apply_sector_stairs(map, sector_id);
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
