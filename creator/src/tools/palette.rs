use crate::editor::{DOCKMANAGER, SIDEBARMODE};
use crate::prelude::*;
use crate::sidebar::SidebarMode;
use ToolEvent::*;

pub struct PaletteTool {
    id: TheId,
    previous_dock: Option<String>,
    previous_sidebar_mode: Option<SidebarMode>,
}

impl PaletteTool {
    fn selection_tool_type(project: &Project, server_ctx: &ServerContext) -> MapToolType {
        if let Some(map) = project.get_map(server_ctx) {
            if !map.selected_vertices.is_empty() {
                MapToolType::Vertex
            } else if !map.selected_linedefs.is_empty() {
                MapToolType::Linedef
            } else if !map.selected_sectors.is_empty() {
                MapToolType::Sector
            } else {
                MapToolType::Sector
            }
        } else {
            MapToolType::Sector
        }
    }
}

impl Tool for PaletteTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Palette Tool"),
            previous_dock: None,
            previous_sidebar_mode: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_palette")
    }

    fn icon_name(&self) -> String {
        "droplet".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('P')
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
                server_ctx.palette_tool_active = true;
                server_ctx.curr_map_tool_type = Self::selection_tool_type(project, server_ctx);
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;

                let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Palette" {
                    self.previous_dock = if current_dock.is_empty() {
                        None
                    } else {
                        Some(current_dock)
                    };
                }
                self.previous_sidebar_mode = Some(*SIDEBARMODE.read().unwrap());
                *SIDEBARMODE.write().unwrap() = SidebarMode::Palette;

                DOCKMANAGER.write().unwrap().set_dock(
                    "Palette".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );
                true
            }
            DeActivate => {
                server_ctx.palette_tool_active = false;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if let Some(mode) = self.previous_sidebar_mode.take() {
                    *SIDEBARMODE.write().unwrap() = mode;
                }
                if DOCKMANAGER.read().unwrap().dock == "Palette"
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
}
