use crate::editor::{EDITCAMERA, NODEEDITOR, RENDEREDITOR, RUSTERIX};
use crate::prelude::*;
use ToolEvent::*;

pub struct RenderTool {
    id: TheId,
}

impl Tool for RenderTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Render Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Render Tool (N). Setup cameras, environment and rendering.")
    }
    fn icon_name(&self) -> String {
        str!("perspective")
    }
    fn accel(&self) -> Option<char> {
        Some('N')
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
                //server_ctx.curr_map_tool_type = MapToolType::World;

                server_ctx.curr_map_tool_helper = MapToolHelper::NodeEditor;
                RUSTERIX.write().unwrap().client.scene.d3_overlay.clear();

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    let mut switch = TheGroupButton::new(TheId::named("Render Helper Switch"));
                    switch.add_text_status(
                        "Render Graph".to_string(),
                        "Edit the global render graph.".to_string(),
                    );
                    switch.add_text_status("Trace".to_string(), "Raytrace the scene.".to_string());
                    switch.set_item_width(100);
                    switch.set_index(server_ctx.curr_render_tool_helper as i32);
                    layout.add_widget(Box::new(switch));

                    EDITCAMERA
                        .write()
                        .unwrap()
                        .setup_toolbar(layout, ctx, project, server_ctx);
                }

                // if server_ctx.curr_render_tool_helper == RenderToolHelper::GlobalRender {
                ctx.ui.send(TheEvent::SetStackIndex(
                    TheId::named("Main Stack"),
                    PanelIndices::NodeEditor as usize,
                ));

                NODEEDITOR.write().unwrap().set_context(
                    NodeContext::GlobalRender,
                    ui,
                    ctx,
                    project,
                    server_ctx,
                );
                // }

                // WORLDEDITOR.write().unwrap().first_draw = true;

                // server_ctx.render_mode = true;
                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                // server_ctx.curr_map_tool_type = MapToolType::General;
                if let Some(map) = project.get_map_mut(server_ctx) {
                    map.terrain.mark_dirty();
                }
                // server_ctx.render_mode = false;
                return true;
            }
            _ => {}
        };
        false
    }

    fn map_event(
        &mut self,
        map_event: MapEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        map: &mut Map,
        server_ctx: &mut ServerContext,
    ) -> Option<RegionUndoAtom> {
        RENDEREDITOR
            .write()
            .unwrap()
            .map_event(map_event, ui, ctx, map, server_ctx)
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::KeyUp(_) => {
                EDITCAMERA.write().unwrap().move_action = None;
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                if *c == 'w' {
                    EDITCAMERA.write().unwrap().move_action = Some(CustomMoveAction::Forward);
                    RENDEREDITOR.write().unwrap().reset_trace();
                }
                if *c == 's' {
                    EDITCAMERA.write().unwrap().move_action = Some(CustomMoveAction::Backward);
                    RENDEREDITOR.write().unwrap().reset_trace();
                }
                if *c == 'a' {
                    EDITCAMERA.write().unwrap().move_action = Some(CustomMoveAction::Left);
                    RENDEREDITOR.write().unwrap().reset_trace();
                }
                if *c == 'd' {
                    EDITCAMERA.write().unwrap().move_action = Some(CustomMoveAction::Right);
                    RENDEREDITOR.write().unwrap().reset_trace();
                }
                redraw = true;
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView" {
                    if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                        EDITCAMERA
                            .write()
                            .unwrap()
                            .mouse_dragged(region, server_ctx, coord);
                        RENDEREDITOR.write().unwrap().reset_trace();
                        redraw = true;
                    }
                }
            }
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == "PolyView" {
                    RENDEREDITOR
                        .write()
                        .unwrap()
                        .scroll_by(ui, ctx, server_ctx, *coord);
                }
            }
            TheEvent::IndexChanged(id, index) => {
                if id.name == "Render Helper Switch" {
                    server_ctx.curr_render_tool_helper.set_from_index(*index);
                    if server_ctx.curr_render_tool_helper == RenderToolHelper::GlobalRender {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::NodeEditor as usize,
                        ));

                        NODEEDITOR.write().unwrap().set_context(
                            NodeContext::GlobalRender,
                            ui,
                            ctx,
                            project,
                            server_ctx,
                        );
                    } else if server_ctx.curr_render_tool_helper == RenderToolHelper::Tracer {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::Trace as usize,
                        ));
                        RENDEREDITOR.write().unwrap().start_trace();
                    }
                }
                // if id.name == "Custom Camera Helper Switch" {
                //     if *index == 0 {
                //         server_ctx.curr_custom_tool_camera = CustomToolCamera::FirstP;
                //     } else if *index == 1 {
                //         server_ctx.curr_custom_tool_camera = CustomToolCamera::Isometric;
                //     }
                // }
            }
            // TheEvent::StateChanged(id, TheWidgetState::Clicked) => {
            //     if id.name == "Trace Button" {
            //         RENDEREDITOR.write().unwrap().switch_trace();
            //     }
            // }
            _ => {}
        }
        redraw
    }
}
