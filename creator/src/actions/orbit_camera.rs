use crate::prelude::*;

pub struct OrbitCamera {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for OrbitCamera {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();
        let item = TheNodeUIItem::Markdown(
            "desc".into(),
            "Render the scene using a 3D orbit camera.".into(),
        );
        nodeui.add_item(item);

        Self {
            id: TheId::named("3D Orbit Camera"),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> &'static str {
        "Render the scene using a 3D orbit camera."
    }

    fn role(&self) -> ActionRole {
        ActionRole::Camera
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::CTRLCMD, '3'))
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        server_ctx.editor_view_mode != EditorViewMode::Orbit
            && server_ctx.get_map_context() == MapContext::Region
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        server_ctx.editor_view_mode = EditorViewMode::Orbit;
        if server_ctx.editing_surface.is_some() {
            ctx.ui.send(TheEvent::Custom(
                TheId::named("Render SceneManager Map"),
                TheValue::Empty,
            ));
            server_ctx.editing_surface = None;
        }
        None
    }

    fn params(&self) -> TheNodeUI {
        self.nodeui.clone()
    }

    fn handle_event(&mut self, event: &TheEvent) -> bool {
        self.nodeui.handle_event(event)
    }
}
