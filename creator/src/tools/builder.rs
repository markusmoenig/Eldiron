use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use ToolEvent::*;

pub struct BuilderTool {
    id: TheId,
    previous_dock: Option<String>,
}

impl BuilderTool {
    fn selected_host_tool(project: &Project, server_ctx: &ServerContext) -> Option<MapToolType> {
        let map = project.get_map(server_ctx)?;
        if !map.selected_sectors.is_empty() {
            Some(MapToolType::Sector)
        } else if !map.selected_linedefs.is_empty() {
            Some(MapToolType::Linedef)
        } else if !map.selected_vertices.is_empty() {
            Some(MapToolType::Vertex)
        } else {
            None
        }
    }

    fn selected_builder_tool(project: &Project, server_ctx: &ServerContext) -> Option<MapToolType> {
        let builder_id = server_ctx.curr_builder_graph_id?;
        project
            .builder_graphs
            .get(&builder_id)
            .and_then(|asset| {
                shared::buildergraph::BuilderDocument::from_text(&asset.graph_data).ok()
            })
            .map(|graph| match graph.output_spec().target {
                BuilderOutputTarget::Sector => MapToolType::Sector,
                BuilderOutputTarget::VertexPair => MapToolType::Vertex,
                BuilderOutputTarget::Linedef => MapToolType::Linedef,
            })
    }
}

impl Tool for BuilderTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Builder Tool"),
            previous_dock: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }

    fn info(&self) -> String {
        fl!("tool_builder")
    }

    fn icon_name(&self) -> String {
        "package".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('B')
    }

    fn help_url(&self) -> Option<String> {
        Some("docs/creator/tools/builder".to_string())
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
                server_ctx.builder_tool_active = true;
                server_ctx.curr_map_tool_type = Self::selected_host_tool(project, server_ctx)
                    .or_else(|| Self::selected_builder_tool(project, server_ctx))
                    .unwrap_or(MapToolType::Sector);
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;

                let current_dock = DOCKMANAGER.read().unwrap().dock.clone();
                if current_dock != "Builder" {
                    self.previous_dock = if current_dock.is_empty() {
                        None
                    } else {
                        Some(current_dock)
                    };
                }
                DOCKMANAGER.write().unwrap().set_dock(
                    "Builder".into(),
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            DeActivate => {
                server_ctx.builder_tool_active = false;
                server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.hover_cursor = None;
                server_ctx.hover_cursor_3d = None;
                if DOCKMANAGER.read().unwrap().dock == "Builder"
                    && let Some(prev) = self.previous_dock.take()
                {
                    DOCKMANAGER
                        .write()
                        .unwrap()
                        .set_dock(prev, ui, ctx, project, server_ctx);
                }
                ctx.ui.send(TheEvent::Custom(
                    TheId::named("Update Geometry Overlay 3D"),
                    TheValue::Empty,
                ));
                true
            }
            _ => false,
        }
    }
}
