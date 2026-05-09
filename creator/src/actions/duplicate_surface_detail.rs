use crate::actions::geometry_face_ops::duplicate_selected_surface_detail;
use crate::editor::RUSTERIX;
use crate::prelude::*;

pub struct DuplicateSurfaceDetail {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for DuplicateSurfaceDetail {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_duplicate_surface_detail_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionDuplicateSurfaceDetailU".into(),
            "U".into(),
            "".into(),
            1.0,
            -256.0..=256.0,
            false,
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionDuplicateSurfaceDetailV".into(),
            "V".into(),
            "".into(),
            0.0,
            -256.0..=256.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_duplicate_surface_detail")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_duplicate_surface_detail_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(
            TheAcceleratorKey::CTRLCMD | TheAcceleratorKey::SHIFT,
            'd',
        ))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && (!map.selected_geometry_surface_segments.is_empty()
                || !map.selected_geometry_surface_points.is_empty())
    }

    fn load_params(&mut self, map: &Map) {
        let step = 1.0 / map.subdivisions.max(1.0);
        self.nodeui
            .set_f32_value("actionDuplicateSurfaceDetailU", step);
        self.nodeui
            .set_f32_value("actionDuplicateSurfaceDetailV", 0.0);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let offset_u = self
            .nodeui
            .get_f32_value("actionDuplicateSurfaceDetailU")
            .unwrap_or_else(|| 1.0 / map.subdivisions.max(1.0));
        let offset_v = self
            .nodeui
            .get_f32_value("actionDuplicateSurfaceDetailV")
            .unwrap_or(0.0);

        let prev = map.clone();
        if !duplicate_selected_surface_detail(map, offset_u, offset_v) {
            return None;
        }

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
