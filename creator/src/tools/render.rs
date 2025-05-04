use crate::editor::{NODEEDITOR, RENDEREDITOR};
use crate::prelude::*;
use ToolEvent::*;

pub struct RenderTool {
    id: TheId,
    last_mouse: Option<Vec2<i32>>,
}

impl Tool for RenderTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("Render Tool"),
            last_mouse: None,
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!("Render Tool (Shift + N). Setup rendering for the global and local render graphs.")
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
        _tool_context: ToolContext,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match tool_event {
            Activate => {
                //server_ctx.curr_map_tool_type = MapToolType::World;

                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();

                    let mut switch = TheGroupButton::new(TheId::named("Render Helper Switch"));
                    switch.add_text_status(
                        "Global".to_string(),
                        "Edit the global render graph.".to_string(),
                    );
                    switch.set_item_width(80);
                    switch.set_index(server_ctx.curr_render_tool_helper as i32);
                    layout.add_widget(Box::new(switch));

                    let mut camera_switch =
                        TheGroupButton::new(TheId::named("Render Camera Helper Switch"));
                    camera_switch
                        .add_text_status("FirstP".to_string(), str!("First Person Camera."));
                    camera_switch.add_text_status(
                        "Isometric".to_string(),
                        "Apply procedural materials.".to_string(),
                    );
                    camera_switch.set_index(server_ctx.curr_render_tool_camera as i32);
                    layout.add_widget(Box::new(camera_switch));

                    if server_ctx.curr_render_tool_helper == RenderToolHelper::GlobalRender {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::NodeEditor as usize,
                        ));
                    }

                    layout.set_reverse_index(Some(1));
                }

                if server_ctx.curr_render_tool_helper == RenderToolHelper::GlobalRender {
                    NODEEDITOR.write().unwrap().set_context(
                        NodeContext::GlobalRender,
                        ui,
                        ctx,
                        project,
                        server_ctx,
                    );
                }

                // WORLDEDITOR.write().unwrap().first_draw = true;

                server_ctx.render_mode = true;
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
                server_ctx.render_mode = false;
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
                RENDEREDITOR.write().unwrap().move_action = None;
            }
            TheEvent::KeyDown(TheValue::Char(c)) => {
                if *c == 'w' {
                    RENDEREDITOR.write().unwrap().move_action = Some(RenderMoveAction::Forward);
                }
                if *c == 's' {
                    RENDEREDITOR.write().unwrap().move_action = Some(RenderMoveAction::Backward);
                }
                if *c == 'a' {
                    RENDEREDITOR.write().unwrap().move_action = Some(RenderMoveAction::Left);
                }
                if *c == 'd' {
                    RENDEREDITOR.write().unwrap().move_action = Some(RenderMoveAction::Right);
                }
                redraw = true;
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "PolyView"
                    && server_ctx.curr_render_tool_camera == RenderToolCamera::FirstP
                {
                    if let Some(region) = project.get_region_ctx_mut(server_ctx) {
                        let sens_yaw = 0.15; // deg per pixel horizontally
                        let sens_pitch = 0.15; // deg per pixel vertically
                        let max_pitch = 85.0; // don’t let camera flip

                        let curr = *coord;

                        if let Some(prev) = self.last_mouse {
                            let dx = (curr.x - prev.x) as f32;
                            let dy = (curr.y - prev.y) as f32;

                            // Yaw   (left / right)
                            if dx.abs() > 0.0 {
                                region.editing_look_at_3d =
                                    RENDEREDITOR.read().unwrap().rotate_camera_y(
                                        region.editing_position_3d,
                                        region.editing_look_at_3d,
                                        -dx * sens_yaw, // screen → world: left = +yaw
                                    );
                            }
                            // Pitch (up / down)
                            if dy.abs() > 0.0 {
                                let look = RENDEREDITOR.read().unwrap().rotate_camera_pitch(
                                    region.editing_position_3d,
                                    region.editing_look_at_3d,
                                    -dy * sens_pitch, // screen up = pitch up
                                );
                                region.editing_look_at_3d = RENDEREDITOR
                                    .read()
                                    .unwrap()
                                    .clamp_pitch(region.editing_position_3d, look, max_pitch);
                            }
                        }

                        self.last_mouse = Some(curr);
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
                    }
                }
            }
            _ => {}
        }
        redraw
    }
}
