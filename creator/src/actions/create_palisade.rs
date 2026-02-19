use crate::prelude::*;
use rusterix::PixelSource;
use std::str::FromStr;

pub const CREATE_PALISADE_ACTION_ID: &str = "f6f4df4c-2cde-4ab5-98ff-2f7f4f62383e";

pub struct CreatePalisade {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CreatePalisade {
    fn apply_linedef_feature(&self, map: &mut Map, linedef_id: u32) -> bool {
        let spacing = self
            .nodeui
            .get_f32_value("actionLayoutSpacing")
            .unwrap_or(1.0)
            .max(0.1);
        let segment_size = self
            .nodeui
            .get_f32_value("actionLayoutSegmentSize")
            .unwrap_or(0.75)
            .max(0.05);
        let shape = self
            .nodeui
            .get_i32_value("actionShapeStakeShape")
            .unwrap_or(1);
        let depth = self
            .nodeui
            .get_f32_value("actionShapeDepth")
            .unwrap_or(0.12)
            .max(0.02);
        let round_segments = self
            .nodeui
            .get_i32_value("actionShapeRoundSegments")
            .unwrap_or(8)
            .max(3);
        let height = self.nodeui.get_f32_value("actionHeightBase").unwrap_or(2.0);
        let top_mode = self.nodeui.get_i32_value("actionTopMode").unwrap_or(0);
        let top_height = self
            .nodeui
            .get_f32_value("actionTopHeight")
            .unwrap_or(0.5)
            .max(0.0);
        let height_variation = self
            .nodeui
            .get_f32_value("actionHeightVariation")
            .unwrap_or(0.35)
            .max(0.0);
        let lean_amount = self
            .nodeui
            .get_f32_value("actionLeanAmount")
            .unwrap_or(0.0)
            .max(0.0);
        let lean_randomness = self
            .nodeui
            .get_f32_value("actionLeanRandomness")
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let tile_id_text = self
            .nodeui
            .get_text_value("actionMaterialTileId")
            .unwrap_or_default();
        let tile_id = Uuid::parse_str(tile_id_text.trim()).ok();

        let Some(linedef) = map.find_linedef_mut(linedef_id) else {
            return false;
        };

        // Height 0 disables feature generation.
        if height <= 0.0 {
            linedef
                .properties
                .set("linedef_feature", Value::Str("None".to_string()));
            return true;
        }

        linedef
            .properties
            .set("linedef_feature", Value::Str("Palisade".to_string()));
        linedef
            .properties
            .set("feature_layout_spacing", Value::Float(spacing));
        linedef
            .properties
            .set("feature_segment_size", Value::Float(segment_size));
        linedef.properties.set("feature_shape", Value::Int(shape));
        linedef.properties.set("feature_depth", Value::Float(depth));
        linedef
            .properties
            .set("feature_round_segments", Value::Int(round_segments));
        linedef
            .properties
            .set("feature_height", Value::Float(height));
        linedef
            .properties
            .set("feature_top_mode", Value::Int(top_mode));
        linedef
            .properties
            .set("feature_top_height", Value::Float(top_height));
        linedef
            .properties
            .set("feature_height_variation", Value::Float(height_variation));
        linedef
            .properties
            .set("feature_lean_amount", Value::Float(lean_amount));
        linedef
            .properties
            .set("feature_lean_randomness", Value::Float(lean_randomness));

        if let Some(id) = tile_id {
            linedef
                .properties
                .set("feature_source", Value::Source(PixelSource::TileId(id)));
        } else {
            linedef.properties.remove("feature_source");
        }

        true
    }
}

impl Action for CreatePalisade {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();

        nodeui.add_item(TheNodeUIItem::OpenTree("material".into()));
        nodeui.add_item(TheNodeUIItem::Text(
            "actionMaterialTileId".into(),
            "".into(),
            "".into(),
            "".into(),
            None,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("layout".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionLayoutSpacing".into(),
            "".into(),
            "".into(),
            1.0,
            0.1..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionLayoutSegmentSize".into(),
            "".into(),
            "".into(),
            0.75,
            0.05..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("shape".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionShapeStakeShape".into(),
            "".into(),
            "".into(),
            vec!["flat".into(), "square".into(), "round".into()],
            1,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionShapeDepth".into(),
            "".into(),
            "".into(),
            0.12,
            0.02..=2.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionShapeRoundSegments".into(),
            "".into(),
            "".into(),
            8,
            3..=24,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("height".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionHeightBase".into(),
            "".into(),
            "".into(),
            2.0,
            0.0..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionHeightVariation".into(),
            "".into(),
            "".into(),
            0.35,
            0.0..=4.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("top".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionTopMode".into(),
            "".into(),
            "".into(),
            vec![
                "flat".into(),
                "spike".into(),
                "bevel".into(),
                "random".into(),
            ],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionTopHeight".into(),
            "".into(),
            "".into(),
            0.5,
            0.0..=4.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("lean".into()));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionLeanAmount".into(),
            "".into(),
            "".into(),
            0.0,
            0.0..=1.5,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionLeanRandomness".into(),
            "".into(),
            "".into(),
            1.0,
            0.0..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::Markdown("desc".into(), "".into()));

        Self {
            id: TheId::named_with_id(
                "Create Palisade",
                Uuid::from_str(CREATE_PALISADE_ACTION_ID).unwrap(),
            ),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Configure a non-destructive palisade feature on selected linedefs.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 && server_ctx.editing_surface.is_some()
        {
            return false;
        }
        map.selected_sectors.is_empty() && !map.selected_linedefs.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let Some(linedef_id) = map.selected_linedefs.first() else {
            return;
        };
        let Some(linedef) = map.find_linedef(*linedef_id) else {
            return;
        };

        self.nodeui.set_f32_value(
            "actionLayoutSpacing",
            linedef
                .properties
                .get_float_default("feature_layout_spacing", 1.0),
        );
        self.nodeui.set_f32_value(
            "actionLayoutSegmentSize",
            linedef
                .properties
                .get_float_default("feature_segment_size", 0.75),
        );
        self.nodeui.set_i32_value(
            "actionShapeStakeShape",
            linedef.properties.get_int_default("feature_shape", 1),
        );
        self.nodeui.set_f32_value(
            "actionShapeDepth",
            linedef.properties.get_float_default("feature_depth", 0.12),
        );
        self.nodeui.set_i32_value(
            "actionShapeRoundSegments",
            linedef
                .properties
                .get_int_default("feature_round_segments", 8),
        );
        self.nodeui.set_f32_value(
            "actionHeightBase",
            linedef.properties.get_float_default("feature_height", 2.0),
        );
        self.nodeui.set_f32_value(
            "actionHeightVariation",
            linedef
                .properties
                .get_float_default("feature_height_variation", 0.35),
        );
        self.nodeui.set_i32_value(
            "actionTopMode",
            linedef.properties.get_int_default("feature_top_mode", 0),
        );
        self.nodeui.set_f32_value(
            "actionTopHeight",
            linedef
                .properties
                .get_float_default("feature_top_height", 0.5),
        );
        self.nodeui.set_f32_value(
            "actionLeanAmount",
            linedef
                .properties
                .get_float_default("feature_lean_amount", 0.0),
        );
        self.nodeui.set_f32_value(
            "actionLeanRandomness",
            linedef
                .properties
                .get_float_default("feature_lean_randomness", 1.0),
        );

        let tile_id_text = match linedef.properties.get("feature_source") {
            Some(Value::Source(PixelSource::TileId(id))) => id.to_string(),
            _ => String::new(),
        };
        self.nodeui
            .set_text_value("actionMaterialTileId", tile_id_text);
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

        for linedef_id in map.selected_linedefs.clone() {
            changed |= self.apply_linedef_feature(map, linedef_id);
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
