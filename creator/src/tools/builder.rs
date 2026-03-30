use crate::editor::DOCKMANAGER;
use crate::prelude::*;
use ToolEvent::*;

pub struct BuilderTool {
    id: TheId,
    previous_dock: Option<String>,
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
        "Builder Tool. Select reusable prop and assembly assets from the builder picker."
            .to_string()
    }

    fn icon_name(&self) -> String {
        "package".to_string()
    }

    fn accel(&self) -> Option<char> {
        Some('B')
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
                server_ctx.curr_map_tool_type =
                    if let Some(builder_id) = server_ctx.curr_builder_graph_id {
                        project
                            .builder_graphs
                            .get(&builder_id)
                            .and_then(|asset| BuilderGraph::from_text(&asset.graph_data).ok())
                            .map(|graph| match graph.output_spec().target {
                                BuilderOutputTarget::Sector => MapToolType::Sector,
                                BuilderOutputTarget::VertexPair => MapToolType::Vertex,
                                BuilderOutputTarget::Linedef => MapToolType::Linedef,
                            })
                            .unwrap_or(MapToolType::Sector)
                    } else {
                        MapToolType::Sector
                    };
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
                true
            }
            DeActivate => {
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
                true
            }
            _ => false,
        }
    }
}
