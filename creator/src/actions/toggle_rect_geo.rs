use crate::editor::DOCKMANAGER;
use crate::prelude::*;

pub struct ToggleRectGeo {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for ToggleRectGeo {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_toggle_rect_geo_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_toggle_rect_geo")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_toggle_rect_geo_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Editor
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode == EditorViewMode::D2
            && server_ctx.editing_surface.is_none()
            && DOCKMANAGER.read().unwrap().get_state() != DockManagerState::Editor
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        server_ctx.no_rect_geo_on_map = !server_ctx.no_rect_geo_on_map;

        ctx.ui.send(TheEvent::Custom(
            TheId::named("Update Client Properties"),
            TheValue::Empty,
        ));

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
        _server_ctx: &mut ServerContext,
    ) -> bool {
        self.nodeui.handle_event(event)
    }
}
