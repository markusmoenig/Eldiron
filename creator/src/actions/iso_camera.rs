use rusterix::D3Camera;

use crate::editor::EDITCAMERA;
use crate::prelude::*;

pub struct IsoCamera {
    id: TheId,
    nodeui: TheNodeUI,
}

impl Action for IsoCamera {
    fn new() -> Self
    where
        Self: Sized,
    {
        let mut nodeui: TheNodeUI = TheNodeUI::default();

        let item = TheNodeUIItem::FloatEditSlider(
            "actionIsoCameraAzimuth".into(),
            "".into(),
            "".into(),
            135.0,
            0.0..=360.0,
            true,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::FloatEditSlider(
            "actionIsoCameraElevation".into(),
            "".into(),
            "".into(),
            35.264,
            0.0..=90.0,
            true,
        );
        nodeui.add_item(item);

        let item = TheNodeUIItem::Markdown("desc".into(), "".into());
        nodeui.add_item(item);

        Self {
            id: TheId::named(&fl!("action_iso_camera")),
            nodeui,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("action_iso_camera_desc")
    }

    fn role(&self) -> ActionRole {
        ActionRole::Camera
    }

    fn accel(&self) -> Option<TheAccelerator> {
        Some(TheAccelerator::new(TheAcceleratorKey::CTRLCMD, '4'))
    }

    fn is_applicable(&self, _map: &Map, _ctx: &mut TheContext, server_ctx: &ServerContext) -> bool {
        // server_ctx.editor_view_mode != EditorViewMode::Iso &&
        server_ctx.get_map_context() == MapContext::Region
    }

    fn apply(
        &self,
        _map: &mut Map,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        server_ctx.editor_view_mode = EditorViewMode::Iso;

        let azimuth = self
            .nodeui
            .get_f32_value("actionIsoCameraAzimuth")
            .unwrap_or(0.0);

        let elevation = self
            .nodeui
            .get_f32_value("actionIsoCameraElevation")
            .unwrap_or(0.0);

        EDITCAMERA
            .write()
            .unwrap()
            .iso_camera
            .set_parameter_f32("azimuth_deg", azimuth);

        EDITCAMERA
            .write()
            .unwrap()
            .iso_camera
            .set_parameter_f32("elevation_deg", elevation);

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
