use crate::prelude::*;
use rusterix::PixelSource;
use std::str::FromStr;

pub const CREATE_FENCE_ACTION_ID: &str = "1d4cc6df-66fd-4d28-a340-c702f696f7b8";

pub struct CreateFence {
    id: TheId,
    nodeui: TheNodeUI,
}

impl CreateFence {
    fn apply_linedef_feature(&self, map: &mut Map, linedef_id: u32) -> bool {
        let spacing = self
            .nodeui
            .get_f32_value("actionLayoutSpacing")
            .unwrap_or(1.5)
            .max(0.1);
        let post_size = self
            .nodeui
            .get_f32_value("actionPostSize")
            .unwrap_or(0.18)
            .max(0.02);
        let post_shape = self.nodeui.get_i32_value("actionPostShape").unwrap_or(0);
        let post_height = self.nodeui.get_f32_value("actionPostHeight").unwrap_or(2.0);
        let round_segments = self
            .nodeui
            .get_i32_value("actionPostRoundSegments")
            .unwrap_or(8)
            .max(3);
        let connector_count = self
            .nodeui
            .get_i32_value("actionConnectorCount")
            .unwrap_or(2)
            .max(0);
        let connector_style = self
            .nodeui
            .get_i32_value("actionConnectorStyle")
            .unwrap_or(0);
        let connector_size = self
            .nodeui
            .get_f32_value("actionConnectorSize")
            .unwrap_or(0.12)
            .max(0.01);
        let connector_drop = self
            .nodeui
            .get_f32_value("actionConnectorDrop")
            .unwrap_or(1.2)
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

        if post_height <= 0.0 {
            linedef
                .properties
                .set("linedef_feature", Value::Str("None".to_string()));
            return true;
        }

        linedef
            .properties
            .set("linedef_feature", Value::Str("Fence".to_string()));
        linedef
            .properties
            .set("feature_layout_spacing", Value::Float(spacing));
        linedef
            .properties
            .set("feature_post_size", Value::Float(post_size));
        linedef
            .properties
            .set("feature_post_shape", Value::Int(post_shape));
        linedef
            .properties
            .set("feature_height", Value::Float(post_height));
        linedef
            .properties
            .set("feature_round_segments", Value::Int(round_segments));
        linedef
            .properties
            .set("feature_connector_count", Value::Int(connector_count));
        linedef
            .properties
            .set("feature_connector_style", Value::Int(connector_style));
        linedef
            .properties
            .set("feature_connector_size", Value::Float(connector_size));
        linedef
            .properties
            .set("feature_connector_drop", Value::Float(connector_drop));
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

impl Action for CreateFence {
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
            1.5,
            0.1..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("posts".into()));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionPostShape".into(),
            "".into(),
            "".into(),
            vec!["square".into(), "round".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionPostSize".into(),
            "".into(),
            "".into(),
            0.18,
            0.02..=2.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionPostHeight".into(),
            "".into(),
            "".into(),
            2.0,
            0.0..=8.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionPostRoundSegments".into(),
            "".into(),
            "".into(),
            8,
            3..=24,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::CloseTree);

        nodeui.add_item(TheNodeUIItem::OpenTree("connectors".into()));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            "actionConnectorCount".into(),
            "".into(),
            "".into(),
            2,
            0..=8,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::Selector(
            "actionConnectorStyle".into(),
            "".into(),
            "".into(),
            vec!["plank".into(), "square".into(), "round".into()],
            0,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionConnectorSize".into(),
            "".into(),
            "".into(),
            0.12,
            0.01..=1.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionConnectorDrop".into(),
            "".into(),
            "".into(),
            1.2,
            0.0..=6.0,
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
                "Create Fence",
                Uuid::from_str(CREATE_FENCE_ACTION_ID).unwrap(),
            ),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        "Configure a non-destructive fence feature on selected linedefs.".to_string()
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        if server_ctx.editor_view_mode == EditorViewMode::D2 {
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
                .get_float_default("feature_layout_spacing", 1.5),
        );
        self.nodeui.set_i32_value(
            "actionPostShape",
            linedef.properties.get_int_default("feature_post_shape", 0),
        );
        self.nodeui.set_f32_value(
            "actionPostSize",
            linedef
                .properties
                .get_float_default("feature_post_size", 0.18),
        );
        self.nodeui.set_f32_value(
            "actionPostHeight",
            linedef.properties.get_float_default("feature_height", 2.0),
        );
        self.nodeui.set_i32_value(
            "actionPostRoundSegments",
            linedef
                .properties
                .get_int_default("feature_round_segments", 8),
        );
        self.nodeui.set_i32_value(
            "actionConnectorCount",
            linedef
                .properties
                .get_int_default("feature_connector_count", 2),
        );
        self.nodeui.set_i32_value(
            "actionConnectorStyle",
            linedef
                .properties
                .get_int_default("feature_connector_style", 0),
        );
        self.nodeui.set_f32_value(
            "actionConnectorSize",
            linedef
                .properties
                .get_float_default("feature_connector_size", 0.12),
        );
        self.nodeui.set_f32_value(
            "actionConnectorDrop",
            linedef
                .properties
                .get_float_default("feature_connector_drop", 1.2),
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
