use crate::prelude::*;
use rusterix::PixelSource;
use std::str::FromStr;

pub const CREATE_CAMPFIRE_ACTION_ID: &str = "0f3a940e-5f6d-4b82-b73d-123c49f9c8a1";

pub struct CreateCampfire {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CreateCampfire {
    fn parse_tile_source(text: &str) -> Option<Value> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if let Ok(id) = Uuid::parse_str(trimmed) {
            return Some(Value::Source(PixelSource::TileId(id)));
        }
        if let Ok(index) = trimmed.parse::<u16>() {
            return Some(Value::Source(PixelSource::PaletteIndex(index)));
        }
        None
    }

    fn source_to_text(source: Option<&Value>) -> String {
        match source {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            Some(Value::Source(PixelSource::PaletteIndex(i))) => i.to_string(),
            _ => String::new(),
        }
    }

    fn apply_sector_campfire(&self, map: &mut Map, sector_id: u32) -> bool {
        let flame_height = self
            .nodeui
            .get_f32_value("actionCampfireFlameHeight")
            .unwrap_or(0.8)
            .max(0.0);
        let flame_width = self
            .nodeui
            .get_f32_value("actionCampfireFlameWidth")
            .unwrap_or(0.45)
            .max(0.05);
        let light_intensity = self
            .nodeui
            .get_f32_value("actionCampfireLightIntensity")
            .unwrap_or(2.2)
            .max(0.0);
        let light_range = self
            .nodeui
            .get_f32_value("actionCampfireLightRange")
            .unwrap_or(5.0)
            .max(0.0);
        let light_flicker = self
            .nodeui
            .get_f32_value("actionCampfireLightFlicker")
            .unwrap_or(0.2)
            .clamp(0.0, 1.0);
        let light_lift = self
            .nodeui
            .get_f32_value("actionCampfireLightLift")
            .unwrap_or(0.2)
            .max(0.0);
        let log_count = self
            .nodeui
            .get_i32_value("actionCampfireLogCount")
            .unwrap_or(10)
            .clamp(3, 24);
        let log_length = self
            .nodeui
            .get_f32_value("actionCampfireLogLength")
            .unwrap_or(0.55)
            .max(0.05);
        let log_thickness = self
            .nodeui
            .get_f32_value("actionCampfireLogThickness")
            .unwrap_or(0.10)
            .max(0.01);
        let log_radius = self
            .nodeui
            .get_f32_value("actionCampfireLogRadius")
            .unwrap_or(0.55)
            .max(0.05);

        let flame_tile_id_text = self
            .nodeui
            .get_text_value("actionCampfireFlameTileId")
            .unwrap_or_default();
        let base_tile_id_text = self
            .nodeui
            .get_text_value("actionCampfireBaseTileId")
            .unwrap_or_default();

        let Some(sector) = map.find_sector_mut(sector_id) else {
            return false;
        };

        if flame_height <= 0.0 || light_range <= 0.0 {
            sector
                .properties
                .set("sector_feature", Value::Str("None".to_string()));
            sector.properties.remove("campfire_flame_height");
            sector.properties.remove("campfire_flame_width");
            sector.properties.remove("campfire_light_intensity");
            sector.properties.remove("campfire_light_range");
            sector.properties.remove("campfire_light_flicker");
            sector.properties.remove("campfire_light_lift");
            sector.properties.remove("campfire_log_count");
            sector.properties.remove("campfire_log_length");
            sector.properties.remove("campfire_log_thickness");
            sector.properties.remove("campfire_log_radius");
            sector.properties.remove("campfire_flame_source");
            sector.properties.remove("campfire_base_source");
            return true;
        }

        sector
            .properties
            .set("sector_feature", Value::Str("Campfire".to_string()));
        sector
            .properties
            .set("campfire_flame_height", Value::Float(flame_height));
        sector
            .properties
            .set("campfire_flame_width", Value::Float(flame_width));
        sector
            .properties
            .set("campfire_light_intensity", Value::Float(light_intensity));
        sector
            .properties
            .set("campfire_light_range", Value::Float(light_range));
        sector
            .properties
            .set("campfire_light_flicker", Value::Float(light_flicker));
        sector
            .properties
            .set("campfire_light_lift", Value::Float(light_lift));
        sector
            .properties
            .set("campfire_log_count", Value::Int(log_count));
        sector
            .properties
            .set("campfire_log_length", Value::Float(log_length));
        sector
            .properties
            .set("campfire_log_thickness", Value::Float(log_thickness));
        sector
            .properties
            .set("campfire_log_radius", Value::Float(log_radius));

        if let Some(src) = Self::parse_tile_source(&flame_tile_id_text) {
            sector.properties.set("campfire_flame_source", src);
        } else {
            sector.properties.remove("campfire_flame_source");
        }
        if let Some(src) = Self::parse_tile_source(&base_tile_id_text) {
            sector.properties.set("campfire_base_source", src);
        } else {
            sector.properties.remove("campfire_base_source");
        }

        true
    }

    fn parse_source_pixelsource(text: &str) -> Option<PixelSource> {
        match Self::parse_tile_source(text) {
            Some(Value::Source(source)) => Some(source),
            _ => None,
        }
    }
}

impl Action for CreateCampfire {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::OpenTree("campfire".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireFlameHeight".into(),
            "".into(),
            "".into(),
            0.8,
            0.0..=4.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireFlameWidth".into(),
            "".into(),
            "".into(),
            0.45,
            0.05..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionCampfireLogCount".into(),
            "".into(),
            "".into(),
            10,
            3..=24,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireLogLength".into(),
            "".into(),
            "".into(),
            0.55,
            0.05..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireLogThickness".into(),
            "".into(),
            "".into(),
            0.10,
            0.01..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireLogRadius".into(),
            "".into(),
            "".into(),
            0.55,
            0.05..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireLightIntensity".into(),
            "".into(),
            "".into(),
            2.2,
            0.0..=12.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireLightRange".into(),
            "".into(),
            "".into(),
            5.0,
            0.0..=30.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireLightFlicker".into(),
            "".into(),
            "".into(),
            0.2,
            0.0..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCampfireLightLift".into(),
            "".into(),
            "".into(),
            0.2,
            0.0..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionCampfireFlameTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionCampfireBaseTileId".into(),
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
                "Create Campfire",
                Uuid::from_str(CREATE_CAMPFIRE_ACTION_ID).unwrap(),
            ),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Configure a procedural campfire and light on selected sectors.".to_string()
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

        self.nodeui.set_f32_value(
            "actionCampfireFlameHeight",
            sector
                .properties
                .get_float_default("campfire_flame_height", 0.8),
        );
        self.nodeui.set_f32_value(
            "actionCampfireFlameWidth",
            sector
                .properties
                .get_float_default("campfire_flame_width", 0.45),
        );
        self.nodeui.set_f32_value(
            "actionCampfireLightIntensity",
            sector
                .properties
                .get_float_default("campfire_light_intensity", 2.2),
        );
        self.nodeui.set_f32_value(
            "actionCampfireLightRange",
            sector
                .properties
                .get_float_default("campfire_light_range", 5.0),
        );
        self.nodeui.set_f32_value(
            "actionCampfireLightFlicker",
            sector
                .properties
                .get_float_default("campfire_light_flicker", 0.2),
        );
        self.nodeui.set_f32_value(
            "actionCampfireLightLift",
            sector
                .properties
                .get_float_default("campfire_light_lift", 0.2),
        );
        self.nodeui.set_i32_value(
            "actionCampfireLogCount",
            sector.properties.get_int_default("campfire_log_count", 10),
        );
        self.nodeui.set_f32_value(
            "actionCampfireLogLength",
            sector
                .properties
                .get_float_default("campfire_log_length", 0.55),
        );
        self.nodeui.set_f32_value(
            "actionCampfireLogThickness",
            sector
                .properties
                .get_float_default("campfire_log_thickness", 0.10),
        );
        self.nodeui.set_f32_value(
            "actionCampfireLogRadius",
            sector
                .properties
                .get_float_default("campfire_log_radius", 0.55),
        );

        self.nodeui.set_text_value(
            "actionCampfireFlameTileId",
            Self::source_to_text(sector.properties.get("campfire_flame_source")),
        );
        self.nodeui.set_text_value(
            "actionCampfireBaseTileId",
            Self::source_to_text(sector.properties.get("campfire_base_source")),
        );
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
            changed |= self.apply_sector_campfire(map, sector_id);
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
        _map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        let flame = self
            .nodeui
            .get_text_value("actionCampfireFlameTileId")
            .unwrap_or_default();
        let base = self
            .nodeui
            .get_text_value("actionCampfireBaseTileId")
            .unwrap_or_default();
        Some(vec![
            ActionMaterialSlot {
                label: "FLAME".to_string(),
                source: Self::parse_source_pixelsource(&flame),
            },
            ActionMaterialSlot {
                label: "BASE".to_string(),
                source: Self::parse_source_pixelsource(&base),
            },
        ])
    }

    fn set_hud_material_from_tile(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
        tile_id: Uuid,
    ) -> bool {
        match slot_index {
            0 => self
                .nodeui
                .set_text_value("actionCampfireFlameTileId", tile_id.to_string()),
            1 => self
                .nodeui
                .set_text_value("actionCampfireBaseTileId", tile_id.to_string()),
            _ => return false,
        }
        true
    }

    fn clear_hud_material_slot(
        &mut self,
        _map: &Map,
        _server_ctx: &ServerContext,
        slot_index: i32,
    ) -> bool {
        match slot_index {
            0 => self
                .nodeui
                .set_text_value("actionCampfireFlameTileId", String::new()),
            1 => self
                .nodeui
                .set_text_value("actionCampfireBaseTileId", String::new()),
            _ => return false,
        }
        true
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
