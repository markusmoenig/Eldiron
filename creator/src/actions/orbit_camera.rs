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
        let item = TheNodeUIItem::Markdown("desc".into(), fl!("action_orbit_camera_desc"));
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_orbit_camera")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_orbit_camera_desc")
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
    ) -> Option<ProjectUndoAtom> {
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

    fn apply_project(
        &self,
        project: &mut Project,
        _ui: &mut TheUI,
        _ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) {
        crate::editor::TOOLLIST
            .write()
            .unwrap()
            .update_geometry_overlay_3d(project, server_ctx);
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
