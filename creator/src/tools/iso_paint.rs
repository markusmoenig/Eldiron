use crate::editor::{DOCKMANAGER, RUSTERIX};
use crate::prelude::*;
use MapEvent::*;
use ToolEvent::*;

pub struct IsoPaintTool {
    id: TheId,
    painting: bool,
    previous_dock: Option<String>,
}

impl IsoPaintTool {
    fn hit_status(server_ctx: &ServerContext) -> String {
        if server_ctx.geo_hit.is_some() {
            fl!("status_iso_paint_hit")
        } else if server_ctx.hover_cursor_3d.is_some() {
            fl!("status_iso_paint_ground")
        } else {
            fl!("status_iso_paint_active")
        }
    }
}

impl Tool for IsoPaintTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Iso Paint Tool"),
            painting: false,
            previous_dock: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_iso_paint")
    }

    fn icon_name(&self) -> String {
        "paint-brush".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('I')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/iso-paint".to_string())
    }

    fn tool_event(
        &mut self,
        tool_event: ToolEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                self.painting = false;
                server_ctx.curr_map_tool_type = MapToolType::IsoPaint;
                server_ctx.editor_view_mode = EditorViewMode::Iso;
                server_ctx.geometry_edit_mode = GeometryEditMode::Geometry;
                server_ctx.hover_cursor = None;

                if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                    region.map.camera = MapCamera::ThreeDIso;
                    region.map.clear_selection();
                    region.map.clear_temp();
                }

                let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Iso Paint" {
                    self.previous_dock = if current_dock.is_empty() {
                        None
                    } else {
                        Some(current_dock)
                    };
                }
                DOCKMANAGER.write().unwrap().set_dock(
                    "Iso Paint".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );

                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
                RUSTERIX.write().unwrap().set_overlay_dirty();
                ctx.ui.redraw_all = true;
                true
            }
            DeActivate => {
                self.painting = false;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if DOCKMANAGER.read().unwrap().dock == "Iso Paint"
                    && let Some(prev) = self.previous_dock.take()
                {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .set_dock(prev, ui, ctx, project, server_ctx);
                }
                true
            }
            _ => false,
        }
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        _ui: &mut TheUI,
        ctx: &mut TheContext,
        _map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<ProjectUndoAtom> {
        match map_event {
            MapClicked(_) => {
                self.painting = true;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    Self::hit_status(server_ctx),
                ));
            }
            MapDragged(_) => {
                if self.painting {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapHover(_) => {
                if !self.painting {
                    ctx.ui.send(TheEvent::SetStatusText(
                        TheId::empty(),
                        Self::hit_status(server_ctx),
                    ));
                }
            }
            MapUp(_) => {
                self.painting = false;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            MapEscape => {
                self.painting = false;
                ctx.ui.send(TheEvent::SetStatusText(
                    TheId::empty(),
                    fl!("status_iso_paint_active"),
                ));
            }
            _ => {}
        }

        None
    }
}
