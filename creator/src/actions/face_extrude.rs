use crate::actions::geometry_face_ops::extrude_selected_geometry_faces;
use crate::editor::RUSTERIX;
use crate::prelude::*;

pub struct FaceExtrude {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for FaceExtrude {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_face_extrude_desc"),
        ));
        nodeui.add_item(TheNodeUIItem::FloatEditSlider(
            "actionFaceExtrudeAmount".into(),
            "Amount".into(),
            "".into(),
            1.0,
            -256.0..=256.0,
            false,
        ));

        Self {
            id: TheId::named(&fl!("action_face_extrude")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_face_extrude_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 'e'))
    }

    fn is_applicable(&self, map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.get_map_context() == MapContext::Region
            && server_ctx.editor_view_mode != EditorViewMode::D2
            && !map.selected_geometry_faces.is_empty()
    }

    fn load_params(&mut self, map: &Map) {
        let step = ServerContext::edit_grid_step(map.subdivisions);
        self.nodeui.set_f32_value("actionFaceExtrudeAmount", step);
    }

    fn apply(
        &self,
        map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        let amount = self
            .nodeui
            .get_f32_value("actionFaceExtrudeAmount")
            .unwrap_or_else(|| ServerContext::edit_grid_step(map.subdivisions));
        let prev = map.clone();
        if !extrude_selected_geometry_faces(map, amount) {
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
