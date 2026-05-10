use crate::actions::geometry_face_ops::{
    CutoutLoopValidation, cut_opening_selected_geometry_faces, cutout_selected_surface_loop,
    validate_selected_cutout_loops,
};
use crate::editor::RUSTERIX;
use crate::prelude::*;

pub struct CreateCutout {
    id: TheId,
    nodeui: TheNodeUI,
}

pub struct FaceCutOpening {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for CreateCutout {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_create_cutout_desc"),
        ));

        Self {
            id: TheId::named(&fl!("action_create_cutout")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_create_cutout_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && (!map.selected_geometry_surface_segments.is_empty()
                || !map.selected_geometry_surface_points.is_empty())
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match validate_selected_cutout_loops(map) {
            CutoutLoopValidation::Valid { loops } => {
                let _ = loops;
            }
            CutoutLoopValidation::Empty => {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_create_cutout_failed"),
                ));
                return None;
            }
            CutoutLoopValidation::MultipleFaces => {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_create_cutout_multiple_faces"),
                ));
                return None;
            }
            CutoutLoopValidation::OpenLoop => {
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_create_cutout_open_loop"),
                ));
                return None;
            }
        }

        let prev = map.clone();
        if !cutout_selected_surface_loop(map) {
            ctx.ui.send(TheEvent::SetStatusText(
                TheId::empty(),
                fl!("status_create_cutout_failed"),
            ));
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
            Box::new(prev),
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

impl Action for FaceCutOpening {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_face_cut_opening_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionFaceCutOpeningWidth".into(),
            "Width".into(),
            "".into(),
            1.0,
            0.01..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionFaceCutOpeningHeight".into(),
            "Height".into(),
            "".into(),
            1.0,
            0.01..=256.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_face_cut_opening")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_face_cut_opening_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && !map.selected_geometry_faces.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let step = ServerContext::edit_grid_step(map.subdivisions);
        self.nodeui
            .set_f32_value("actionFaceCutOpeningWidth", (step * 2.0).max(0.01));
        self.nodeui
            .set_f32_value("actionFaceCutOpeningHeight", (step * 2.0).max(0.01));
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
            .get_f32_value("actionFaceCutOpeningWidth")
            .unwrap_or_else(|| ServerContext::edit_grid_step(map.subdivisions) * 2.0)
            .max(0.01);
        let height = self
            .nodeui
            .get_f32_value("actionFaceCutOpeningHeight")
            .unwrap_or_else(|| ServerContext::edit_grid_step(map.subdivisions) * 2.0)
            .max(0.01);
        let prev = map.clone();
        if !cut_opening_selected_geometry_faces(map, width, height) {
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
            Box::new(prev),
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
