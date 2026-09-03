use crate::actions::geometry_edge_ops::bevel_selected_geometry_edges;
use crate::editor::RUSTERIX;
use crate::prelude::*;

const WIDTH_ID: &str = "actionEdgeBevelWidth";
const SEGMENTS_ID: &str = "actionEdgeBevelSegments";
const PROFILE_ID: &str = "actionEdgeBevelProfile";

pub struct EdgeBevel {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EdgeBevel {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_edge_bevel_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            WIDTH_ID.into(),
            fl!("action_edge_bevel_width"),
            "".into(),
            0.1,
            0.001..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::IntEditSlider(
            SEGMENTS_ID.into(),
            fl!("action_edge_bevel_segments"),
            "".into(),
            3,
            1..=16,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            PROFILE_ID.into(),
            fl!("action_edge_bevel_profile"),
            "".into(),
            1.0,
            0.0..=1.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_edge_bevel")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_edge_bevel_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && map.geometry_selection_mode == 3
            && map.selected_geometry_vertices.len() >= 2
    }

    fn load_params(&mut self, map: &Map) {
        let step = ServerContext::edit_grid_step(map.subdivisions);
        self.nodeui.set_f32_value(WIDTH_ID, (step * 0.1).max(0.001));
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let width = self
            .nodeui
            .get_f32_value(WIDTH_ID)
            .unwrap_or(0.1)
            .max(0.001);
        let segments = self
            .nodeui
            .get_i32_value(SEGMENTS_ID)
            .unwrap_or(3)
            .clamp(1, 16) as usize;
        let profile = self
            .nodeui
            .get_f32_value(PROFILE_ID)
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let previous = map.clone();
        if !bevel_selected_geometry_edges(map, width, segments, profile) {
            return None;
        }

        RUSTERIX.write().unwrap().set_dirty();
        RUSTERIX.write().unwrap().set_overlay_dirty();
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Map Selection Changed"),
            TheValue::Empty,
        ));
        Some(ProjectUndoAtom::MapEdit(
            server_ctx.pc,
            Box::new(previous),
            Box::new(map.clone()),
        ))
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
