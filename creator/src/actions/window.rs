use crate::prelude::*;
use rusterix::{PixelSource, Value};

pub struct Window {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for Window {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::OpenTree("window".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionWindowInset".into(),
            "".into(),
            "".into(),
            0.0,
            -1.0..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionWindowFrameWidth".into(),
            "".into(),
            "".into(),
            0.08,
            0.01..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionMaterialFrameTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionMaterialFrameTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionMaterialGlassTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionMaterialGlassTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), "".into()));

        Self {
            id: TheId::named(&fl!("action_window")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_window_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        let profile_edit_active = (server_ctx.editor_view_mode == EditorViewMode::D2
            && server_ctx.editing_surface.is_some())
            || (server_ctx.editor_view_mode != EditorViewMode::D2
                && server_ctx.geometry_edit_mode == GeometryEditMode::Detail
                && server_ctx.active_detail_surface.is_some());
        !map.selected_sectors.is_empty() && profile_edit_active
    }

    fn load_params(&mut self, map: &Map) {
        if let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
        {
            self.nodeui.set_f32_value(
                "actionWindowInset",
                sector.properties.get_float_default("profile_inset", 0.0),
            );
            self.nodeui.set_f32_value(
                "actionWindowFrameWidth",
                sector
                    .properties
                    .get_float_default("window_frame_width", 0.08),
            );
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut frame_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut frame_id = Uuid::nil();
        let mut frame_text = String::new();
        let mut glass_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut glass_id = Uuid::nil();
        let mut glass_text = String::new();

        if let Some(map) = project.get_map(server_ctx)
            && let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
        {
            if let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("window_frame_source")
            {
                if let Some(tile) = project.tiles.get(id)
                    && !tile.is_empty()
                {
                    frame_icon = tile.textures[0].to_rgba();
                    frame_id = *id;
                    frame_text = id.to_string();
                }
            }
            if let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("window_glass_source")
            {
                if let Some(tile) = project.tiles.get(id)
                    && !tile.is_empty()
                {
                    glass_icon = tile.textures[0].to_rgba();
                    glass_id = *id;
                    glass_text = id.to_string();
                }
            }
            if let Some(Value::Source(PixelSource::PaletteIndex(index))) =
                sector.properties.get("window_frame_source")
            {
                frame_id = Uuid::nil();
                frame_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
                frame_text = index.to_string();
            }
            if let Some(Value::Source(PixelSource::PaletteIndex(index))) =
                sector.properties.get("window_glass_source")
            {
                glass_id = Uuid::nil();
                glass_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
                glass_text = index.to_string();
            }
        }
        self.nodeui
            .set_text_value("actionMaterialFrameTileId", frame_text);
        self.nodeui
            .set_text_value("actionMaterialGlassTileId", glass_text);

        if let Some(TheNodeUIItem::Icons(_, _, _, items)) =
            self.nodeui.get_item_mut("actionMaterialFrameTile")
            && items.len() == 1
        {
            items[0].0 = frame_icon;
            items[0].2 = frame_id;
        }

        if let Some(TheNodeUIItem::Icons(_, _, _, items)) =
            self.nodeui.get_item_mut("actionMaterialGlassTile")
            && items.len() == 1
        {
            items[0].0 = glass_icon;
            items[0].2 = glass_id;
        }
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

        let inset = self
            .nodeui
            .get_f32_value("actionWindowInset")
            .unwrap_or(0.0);
        let frame_w = self
            .nodeui
            .get_f32_value("actionWindowFrameWidth")
            .unwrap_or(0.08);

        let frame_icon_id = self
            .nodeui
            .get_tile_id("actionMaterialFrameTile", 0)
            .unwrap_or(Uuid::nil());
        let glass_icon_id = self
            .nodeui
            .get_tile_id("actionMaterialGlassTile", 0)
            .unwrap_or(Uuid::nil());

        let frame_text = self
            .nodeui
            .get_text_value("actionMaterialFrameTileId")
            .unwrap_or_default();
        let glass_text = self
            .nodeui
            .get_text_value("actionMaterialGlassTileId")
            .unwrap_or_default();

        let parse_source = |text: &str, fallback: Uuid| -> Option<PixelSource> {
            let t = text.trim();
            if !t.is_empty() {
                if let Some(source) = parse_tile_id_pixelsource(t) {
                    return Some(source);
                }
            }
            if fallback != Uuid::nil() {
                Some(PixelSource::TileId(fallback))
            } else {
                None
            }
        };
        let frame_source = parse_source(&frame_text, frame_icon_id);
        let glass_source = parse_source(&glass_text, glass_icon_id);

        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set("profile_op", Value::Int(4));
                sector.properties.set("profile_inset", Value::Float(inset));
                sector
                    .properties
                    .set("window_frame_width", Value::Float(frame_w.max(0.01)));

                if let Some(source) = frame_source.clone() {
                    sector
                        .properties
                        .set("window_frame_source", Value::Source(source));
                } else {
                    sector.properties.remove("window_frame_source");
                }

                if let Some(source) = glass_source.clone() {
                    sector
                        .properties
                        .set("window_glass_source", Value::Source(source));
                } else {
                    sector.properties.remove("window_glass_source");
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

    fn hud_material_slots(
        &self,
        _map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        let frame_icon_id = self
            .nodeui
            .get_tile_id("actionMaterialFrameTile", 0)
            .unwrap_or(Uuid::nil());
        let glass_icon_id = self
            .nodeui
            .get_tile_id("actionMaterialGlassTile", 0)
            .unwrap_or(Uuid::nil());
        let parse_source = |text: &str, fallback: Uuid| -> Option<PixelSource> {
            let t = text.trim();
            if !t.is_empty() {
                if let Some(source) = parse_tile_id_pixelsource(t) {
                    return Some(source);
                }
            }
            if fallback != Uuid::nil() {
                Some(PixelSource::TileId(fallback))
            } else {
                None
            }
        };

        Some(vec![
            ActionMaterialSlot {
                label: "FRAME".to_string(),
                source: parse_source(
                    &self
                        .nodeui
                        .get_text_value("actionMaterialFrameTileId")
                        .unwrap_or_default(),
                    frame_icon_id,
                ),
            },
            ActionMaterialSlot {
                label: "GLASS".to_string(),
                source: parse_source(
                    &self
                        .nodeui
                        .get_text_value("actionMaterialGlassTileId")
                        .unwrap_or_default(),
                    glass_icon_id,
                ),
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
            0 => {
                self.nodeui
                    .set_text_value("actionMaterialFrameTileId", tile_id.to_string());
                set_nodeui_icon_tile_id(&mut self.nodeui, "actionMaterialFrameTile", 0, tile_id);
            }
            1 => {
                self.nodeui
                    .set_text_value("actionMaterialGlassTileId", tile_id.to_string());
                set_nodeui_icon_tile_id(&mut self.nodeui, "actionMaterialGlassTile", 0, tile_id);
            }
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
            0 => {
                self.nodeui
                    .set_text_value("actionMaterialFrameTileId", String::new());
                clear_nodeui_icon_tile_id(&mut self.nodeui, "actionMaterialFrameTile", 0);
            }
            1 => {
                self.nodeui
                    .set_text_value("actionMaterialGlassTileId", String::new());
                clear_nodeui_icon_tile_id(&mut self.nodeui, "actionMaterialGlassTile", 0);
            }
            _ => return false,
        }
        true
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

            if id.name == "actionMaterialFrameTile" {
                self.nodeui
                    .set_text_value("actionMaterialFrameTileId", tile_id.to_string());
            } else if id.name == "actionMaterialGlassTile" {
                self.nodeui
                    .set_text_value("actionMaterialGlassTileId", tile_id.to_string());
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
