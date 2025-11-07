use crate::prelude::*;

pub struct FirstPCamera {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for FirstPCamera {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Render the scene using a 3D first person camera.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("3D First Person Camera"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Render the scene using a 3D first person camera."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Property
    }

    fn accel(&self) -> Option<TheAccelerator> {
        None
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode != EditorViewMode::FirstP
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        server_ctx.editor_view_mode = EditorViewMode::FirstP;

        None
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
