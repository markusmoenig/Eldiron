use crate::prelude::*;

pub struct ToggleGrid {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ToggleGrid {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui = TheNodeUI::default();
        nodeui.add_item(TheNodeUIItem::Markdown(
            "desc".into(),
            fl!("action_toggle_grid_desc"),
        ));

        Self {
            id: TheId::named(&fl!("action_toggle_grid")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_toggle_grid_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::CTRLCMD, 't'))
    }

    fn is_applicable(
        &self,
        _map: &Map,
        _ctx: &mut TheContext,
        _server_ctx: &CreatorContext,
    ) -> bool {
        true
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut CreatorContext,
    ) -> Option<ProjectUndoAtom> {
        server_ctx.shared.show_editor_grid = !server_ctx.shared.show_editor_grid;
        server_ctx.reference.write().set_overlay_dirty();
        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Geometry Overlay 3D"),
            TheValue::Empty,
        ));
        ctx.ui.invalidate_paint();
        None
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
        _server_ctx: &mut CreatorContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
