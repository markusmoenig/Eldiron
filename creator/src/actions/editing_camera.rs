use crate::prelude::*;

pub struct EditingCamera {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for EditingCamera {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_editing_camera_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_editing_camera")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_editing_camera_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Camera
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::CTRLCMD, '2'))
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode != EditorViewMode::D2
            && server_ctx.get_map_context() == MapContext::Region
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        server_ctx.editor_view_mode = EditorViewMode::D2;

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
