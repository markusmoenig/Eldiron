use crate::prelude::*;
use rusterix::{PixelSource, Value};

pub struct CreateProp {
    id: TheId,
    nodeui: TheNodeUI,
}

fn parse_source_from_text_or_tile(text: &str, fallback_tile_id: Uuid) -> Option<PixelSource> {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
        if let Some(source) = parse_tile_id_pixelsource(trimmed) {
            return Some(source);
        }
    }
    if fallback_tile_id != Uuid::nil() {
        Some(PixelSource::TileId(fallback_tile_id))
    } else {
        None
    }
}

fn source_to_text_and_uuid(source: Option<&Value>) -> (String, Uuid) {
    if let Some(Value::Source(ps)) = source {
        match ps {
            PixelSource::TileId(id) => (id.to_string(), *id),
            PixelSource::PaletteIndex(i) => (i.to_string(), Uuid::nil()),
            _ => (String::new(), Uuid::nil()),
        }
    } else {
        (String::new(), Uuid::nil())
    }
}

impl Action for CreateProp {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::OpenTree("prop".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionPropType".into(),
            "".into(),
            "".into(),
            vec![
                "table".into(),
                "bookcase".into(),
                "crate".into(),
                "barrel".into(),
                "bed".into(),
            ],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("table".into()));
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

        nodeui.add_item(TheNodeUIItem::OpenTree("bookcase".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionBookcaseHeight".into(),
            "".into(),
            "".into(),
            2.0,
            0.2..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionBookcaseShelves".into(),
            "".into(),
            "".into(),
            4,
            1..=12,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionBookcaseBooks".into(),
            "".into(),
            "".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("crate".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionCrateHeight".into(),
            "".into(),
            "".into(),
            1.0,
            0.2..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("barrel".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionBarrelHeight".into(),
            "".into(),
            "".into(),
            1.0,
            0.2..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionBarrelBulge".into(),
            "".into(),
            "".into(),
            1.12,
            1.0..=1.5,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionBarrelSegments".into(),
            "".into(),
            "".into(),
            12,
            6..=32,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("bed".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionBedHeight".into(),
            "".into(),
            "".into(),
            0.55,
            0.2..=3.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Checkbox(
            "actionBedHeadboard".into(),
            "".into(),
            "".into(),
            true,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionBedHeadboardSide".into(),
            "".into(),
            "".into(),
            vec!["start".into(), "end".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionBedHeadboardHeight".into(),
            "".into(),
            "".into(),
            0.7,
            0.2..=2.5,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Icons(
            "actionBedMattressTile".into(),
            "".into(),
            "".into(),
            vec![(
                TheRGBABuffer::new(TheDim::sized(36, 36)),
                "".to_string(),
                Uuid::nil(),
            )],
        ));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionBedMattressTileId".into(),
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
            let prop_kind = sector.properties.get_int_default("profile_prop_kind", 0);
            self.nodeui
                .set_i32_value("actionPropType", prop_kind.clamp(0, 4));
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
            self.nodeui.set_f32_value(
                "actionBookcaseHeight",
                sector.properties.get_float_default("profile_amount", 2.0),
            );
            self.nodeui.set_f32_value(
                "actionCrateHeight",
                sector.properties.get_float_default("profile_amount", 1.0),
            );
            self.nodeui.set_f32_value(
                "actionBarrelHeight",
                sector.properties.get_float_default("profile_amount", 1.0),
            );
            self.nodeui.set_f32_value(
                "actionBarrelBulge",
                sector.properties.get_float_default("barrel_bulge", 1.12),
            );
            self.nodeui.set_i32_value(
                "actionBarrelSegments",
                sector.properties.get_int_default("barrel_segments", 12),
            );
            self.nodeui.set_f32_value(
                "actionBedHeight",
                sector.properties.get_float_default("profile_amount", 0.55),
            );
            self.nodeui.set_bool_value(
                "actionBedHeadboard",
                sector.properties.get_bool_default("bed_headboard", true),
            );
            self.nodeui.set_i32_value(
                "actionBedHeadboardSide",
                sector
                    .properties
                    .get_int_default("bed_headboard_side", 0)
                    .clamp(0, 1),
            );
            self.nodeui.set_f32_value(
                "actionBedHeadboardHeight",
                sector
                    .properties
                    .get_float_default("bed_headboard_height", 0.7),
            );
            self.nodeui.set_i32_value(
                "actionBookcaseShelves",
                sector.properties.get_int_default("bookcase_shelves", 4),
            );
            self.nodeui.set_bool_value(
                "actionBookcaseBooks",
                sector.properties.get_bool_default("bookcase_books", true),
            );

            let (tile_text, tile_id) = source_to_text_and_uuid(sector.properties.get("cap_source"));
            self.nodeui
                .set_text_value("actionMaterialTileId", tile_text);
            if let Some(item) = self.nodeui.get_item_mut("actionMaterialTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = tile_id;
            }

            let (chair_tile_text, chair_tile_id) =
                source_to_text_and_uuid(sector.properties.get("chair_source"));
            self.nodeui
                .set_text_value("actionTableChairTileId", chair_tile_text);
            if let Some(item) = self.nodeui.get_item_mut("actionTableChairTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = chair_tile_id;
            }

            let (bed_mattress_tile_text, bed_mattress_tile_id) =
                source_to_text_and_uuid(sector.properties.get("bed_mattress_source"));
            self.nodeui
                .set_text_value("actionBedMattressTileId", bed_mattress_tile_text);
            if let Some(item) = self.nodeui.get_item_mut("actionBedMattressTile")
                && let TheNodeUIItem::Icons(_, _, _, items) = item
                && items.len() == 1
            {
                items[0].2 = bed_mattress_tile_id;
            }
        }
    }

    fn load_params_project(&mut self, project: &Project, server_ctx: &mut ServerContext) {
        let mut tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut tile_id = Uuid::nil();
        let mut chair_tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut chair_tile_id = Uuid::nil();
        let mut bed_mattress_tile_icon = TheRGBABuffer::new(TheDim::sized(36, 36));
        let mut bed_mattress_tile_id = Uuid::nil();

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
        if let Some(map) = project.get_map(server_ctx)
            && let Some(sector_id) = map.selected_sectors.first()
            && let Some(sector) = map.find_sector(*sector_id)
            && let Some(Value::Source(PixelSource::TileId(id))) =
                sector.properties.get("bed_mattress_source")
            && let Some(tile) = project.tiles.get(id)
            && !tile.is_empty()
        {
            bed_mattress_tile_icon = tile.textures[0].to_rgba();
            bed_mattress_tile_id = *id;
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
        if let Some(item) = self.nodeui.get_item_mut("actionBedMattressTile")
            && let TheNodeUIItem::Icons(_, _, _, items) = item
            && items.len() == 1
        {
            items[0].0 = bed_mattress_tile_icon;
            items[0].2 = bed_mattress_tile_id;
        }
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let prop_type = self.nodeui.get_i32_value("actionPropType").unwrap_or(0);

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
        let bookcase_height = self
            .nodeui
            .get_f32_value("actionBookcaseHeight")
            .unwrap_or(2.0)
            .max(0.2);
        let crate_height = self
            .nodeui
            .get_f32_value("actionCrateHeight")
            .unwrap_or(1.0)
            .max(0.2);
        let barrel_height = self
            .nodeui
            .get_f32_value("actionBarrelHeight")
            .unwrap_or(1.0)
            .max(0.2);
        let barrel_bulge = self
            .nodeui
            .get_f32_value("actionBarrelBulge")
            .unwrap_or(1.12)
            .clamp(1.0, 1.5);
        let barrel_segments = self
            .nodeui
            .get_i32_value("actionBarrelSegments")
            .unwrap_or(12)
            .clamp(6, 32);
        let bed_height = self
            .nodeui
            .get_f32_value("actionBedHeight")
            .unwrap_or(0.55)
            .clamp(0.2, 3.0);
        let bed_headboard = self
            .nodeui
            .get_bool_value("actionBedHeadboard")
            .unwrap_or(true);
        let bed_headboard_side = self
            .nodeui
            .get_i32_value("actionBedHeadboardSide")
            .unwrap_or(0)
            .clamp(0, 1);
        let bed_headboard_height = self
            .nodeui
            .get_f32_value("actionBedHeadboardHeight")
            .unwrap_or(0.7)
            .clamp(0.2, 2.5);
        let bookcase_shelves = self
            .nodeui
            .get_i32_value("actionBookcaseShelves")
            .unwrap_or(4)
            .clamp(1, 12);
        let bookcase_books = self
            .nodeui
            .get_bool_value("actionBookcaseBooks")
            .unwrap_or(true);

        let tile_id = self
            .nodeui
            .get_tile_id("actionMaterialTile", 0)
            .unwrap_or(Uuid::nil());
        let tile_text = self
            .nodeui
            .get_text_value("actionMaterialTileId")
            .unwrap_or_default();
        let table_source = parse_source_from_text_or_tile(&tile_text, tile_id);
        let chair_tile_id = self
            .nodeui
            .get_tile_id("actionTableChairTile", 0)
            .unwrap_or(Uuid::nil());
        let chair_tile_text = self
            .nodeui
            .get_text_value("actionTableChairTileId")
            .unwrap_or_default();
        let chair_source = parse_source_from_text_or_tile(&chair_tile_text, chair_tile_id);
        let bed_mattress_tile_id = self
            .nodeui
            .get_tile_id("actionBedMattressTile", 0)
            .unwrap_or(Uuid::nil());
        let bed_mattress_tile_text = self
            .nodeui
            .get_text_value("actionBedMattressTileId")
            .unwrap_or_default();
        let bed_mattress_source =
            parse_source_from_text_or_tile(&bed_mattress_tile_text, bed_mattress_tile_id);

        let mut changed = false;
        for sector_id in &map.selected_sectors.clone() {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set("profile_op", Value::Int(1));
                sector
                    .properties
                    .set("profile_prop_kind", Value::Int(prop_type));
                sector.properties.set(
                    "profile_amount",
                    Value::Float(match prop_type {
                        1 => bookcase_height,
                        2 => crate_height,
                        3 => barrel_height,
                        4 => bed_height,
                        _ => height,
                    }),
                );
                sector.properties.set("profile_table", Value::Bool(true));
                if prop_type == 0 {
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
                    sector.properties.remove("bookcase_shelves");
                    sector.properties.remove("bookcase_books");
                    sector.properties.remove("book_source");
                    sector.properties.remove("barrel_bulge");
                    sector.properties.remove("barrel_segments");
                    sector.properties.remove("bed_headboard");
                    sector.properties.remove("bed_headboard_side");
                    sector.properties.remove("bed_headboard_height");
                    sector.properties.remove("bed_mattress_source");
                } else if prop_type == 1 {
                    sector
                        .properties
                        .set("bookcase_shelves", Value::Int(bookcase_shelves));
                    sector
                        .properties
                        .set("bookcase_books", Value::Bool(bookcase_books));
                    sector.properties.remove("table_chairs");
                    sector.properties.remove("table_chair_count");
                    sector.properties.remove("table_chair_offset");
                    sector.properties.remove("table_chair_width");
                    sector.properties.remove("table_chair_back_height");
                    sector.properties.remove("chair_source");
                    sector.properties.remove("barrel_bulge");
                    sector.properties.remove("barrel_segments");
                    sector.properties.remove("bed_headboard");
                    sector.properties.remove("bed_headboard_side");
                    sector.properties.remove("bed_headboard_height");
                    sector.properties.remove("bed_mattress_source");
                    sector.properties.remove("builder_graph_id");
                    sector.properties.remove("builder_graph_name");
                    sector.properties.remove("builder_graph_data");

                    if bookcase_books {
                        let palette_index = (8 + (sector.id % 24)) as u16;
                        sector.properties.set(
                            "book_source",
                            Value::Source(PixelSource::PaletteIndex(palette_index)),
                        );
                    } else {
                        sector.properties.remove("book_source");
                    }
                } else if prop_type == 3 {
                    sector
                        .properties
                        .set("barrel_bulge", Value::Float(barrel_bulge));
                    sector
                        .properties
                        .set("barrel_segments", Value::Int(barrel_segments));
                    sector.properties.remove("table_chairs");
                    sector.properties.remove("table_chair_count");
                    sector.properties.remove("table_chair_offset");
                    sector.properties.remove("table_chair_width");
                    sector.properties.remove("table_chair_back_height");
                    sector.properties.remove("chair_source");
                    sector.properties.remove("bookcase_shelves");
                    sector.properties.remove("bookcase_books");
                    sector.properties.remove("book_source");
                    sector.properties.remove("bed_headboard");
                    sector.properties.remove("bed_headboard_side");
                    sector.properties.remove("bed_headboard_height");
                    sector.properties.remove("bed_mattress_source");
                    sector.properties.remove("builder_graph_id");
                    sector.properties.remove("builder_graph_name");
                    sector.properties.remove("builder_graph_data");
                } else if prop_type == 4 {
                    sector
                        .properties
                        .set("bed_headboard", Value::Bool(bed_headboard));
                    sector
                        .properties
                        .set("bed_headboard_side", Value::Int(bed_headboard_side));
                    sector
                        .properties
                        .set("bed_headboard_height", Value::Float(bed_headboard_height));
                    sector.properties.remove("table_chairs");
                    sector.properties.remove("table_chair_count");
                    sector.properties.remove("table_chair_offset");
                    sector.properties.remove("table_chair_width");
                    sector.properties.remove("table_chair_back_height");
                    sector.properties.remove("chair_source");
                    sector.properties.remove("bookcase_shelves");
                    sector.properties.remove("bookcase_books");
                    sector.properties.remove("book_source");
                    sector.properties.remove("barrel_bulge");
                    sector.properties.remove("barrel_segments");
                    sector.properties.remove("builder_graph_id");
                    sector.properties.remove("builder_graph_name");
                    sector.properties.remove("builder_graph_data");
                    if let Some(source) = bed_mattress_source.clone().or(table_source.clone()) {
                        sector
                            .properties
                            .set("bed_mattress_source", Value::Source(source));
                    } else {
                        sector.properties.remove("bed_mattress_source");
                    }
                } else {
                    // Crate and any unknown future prop kinds: clear unrelated settings.
                    sector.properties.remove("table_chairs");
                    sector.properties.remove("table_chair_count");
                    sector.properties.remove("table_chair_offset");
                    sector.properties.remove("table_chair_width");
                    sector.properties.remove("table_chair_back_height");
                    sector.properties.remove("chair_source");
                    sector.properties.remove("bookcase_shelves");
                    sector.properties.remove("bookcase_books");
                    sector.properties.remove("book_source");
                    sector.properties.remove("barrel_bulge");
                    sector.properties.remove("barrel_segments");
                    sector.properties.remove("bed_headboard");
                    sector.properties.remove("bed_headboard_side");
                    sector.properties.remove("bed_headboard_height");
                    sector.properties.remove("bed_mattress_source");
                    sector.properties.remove("builder_graph_id");
                    sector.properties.remove("builder_graph_name");
                    sector.properties.remove("builder_graph_data");
                }

                if let Some(source) = table_source.clone() {
                    let src = Value::Source(source);
                    sector.properties.set("cap_source", src.clone());
                    sector.properties.set("jamb_source", src);
                } else {
                    sector.properties.remove("cap_source");
                    sector.properties.remove("jamb_source");
                }
                if let Some(source) = chair_source.clone() {
                    let src = Value::Source(source);
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

    fn hud_material_slots(
        &self,
        _map: &Map,
        _server_ctx: &ServerContext,
    ) -> Option<Vec<ActionMaterialSlot>> {
        let main_icon = self
            .nodeui
            .get_tile_id("actionMaterialTile", 0)
            .unwrap_or(Uuid::nil());
        let chair_icon = self
            .nodeui
            .get_tile_id("actionTableChairTile", 0)
            .unwrap_or(Uuid::nil());
        let mattress_icon = self
            .nodeui
            .get_tile_id("actionBedMattressTile", 0)
            .unwrap_or(Uuid::nil());
        Some(vec![
            ActionMaterialSlot {
                label: "MAIN".to_string(),
                source: parse_source_from_text_or_tile(
                    &self
                        .nodeui
                        .get_text_value("actionMaterialTileId")
                        .unwrap_or_default(),
                    main_icon,
                ),
            },
            ActionMaterialSlot {
                label: "CHAIR".to_string(),
                source: parse_source_from_text_or_tile(
                    &self
                        .nodeui
                        .get_text_value("actionTableChairTileId")
                        .unwrap_or_default(),
                    chair_icon,
                ),
            },
            ActionMaterialSlot {
                label: "MATT".to_string(),
                source: parse_source_from_text_or_tile(
                    &self
                        .nodeui
                        .get_text_value("actionBedMattressTileId")
                        .unwrap_or_default(),
                    mattress_icon,
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
                    .set_text_value("actionMaterialTileId", tile_id.to_string());
                set_nodeui_icon_tile_id(&mut self.nodeui, "actionMaterialTile", 0, tile_id);
            }
            1 => {
                self.nodeui
                    .set_text_value("actionTableChairTileId", tile_id.to_string());
                set_nodeui_icon_tile_id(&mut self.nodeui, "actionTableChairTile", 0, tile_id);
            }
            2 => {
                self.nodeui
                    .set_text_value("actionBedMattressTileId", tile_id.to_string());
                set_nodeui_icon_tile_id(&mut self.nodeui, "actionBedMattressTile", 0, tile_id);
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
                    .set_text_value("actionMaterialTileId", String::new());
                clear_nodeui_icon_tile_id(&mut self.nodeui, "actionMaterialTile", 0);
            }
            1 => {
                self.nodeui
                    .set_text_value("actionTableChairTileId", String::new());
                clear_nodeui_icon_tile_id(&mut self.nodeui, "actionTableChairTile", 0);
            }
            2 => {
                self.nodeui
                    .set_text_value("actionBedMattressTileId", String::new());
                clear_nodeui_icon_tile_id(&mut self.nodeui, "actionBedMattressTile", 0);
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
            if id.name == "actionMaterialTile" {
                self.nodeui
                    .set_text_value("actionMaterialTileId", tile_id.to_string());
            } else if id.name == "actionTableChairTile" {
                self.nodeui
                    .set_text_value("actionTableChairTileId", tile_id.to_string());
            } else if id.name == "actionBedMattressTile" {
                self.nodeui
                    .set_text_value("actionBedMattressTileId", tile_id.to_string());
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
