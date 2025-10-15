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
        let nodeui: TheNodeUI = TheNodeUI::default();

        Self {
            id: TheId::named("Toggle Rect Geometry"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Toggle Rect Geometry (Ctrl + S). Toggle the visibility of geometry created by the Rect tool in the 2D editor."
    }

    fn role(&self) -> &'static str {
        "2D Editor UI"
    }

    fn accel(&self) -> Option<char> {
        Some('S')
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode == EditorViewMode::D2
    }

    fn apply(
        &self,
        _map: &mut Map,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
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

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
