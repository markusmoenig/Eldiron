use crate::prelude::*;
use ToolEvent::*;

use crate::editor::WORLDEDITOR;

pub struct WorldTool {
    id: TheId,
}

impl Tool for WorldTool {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            id: TheId::named("World Tool"),
        }
    }

    fn id(&self) -> TheId {
        self.id.clone()
    }
    fn info(&self) -> String {
        str!(
            "World Tool (S). Hold 'Shift' to add. 'Alt' to subtract. Click and drag for multi-selection."
        )
    }
    fn icon_name(&self) -> String {
        str!("world")
    }
    fn accel(&self) -> Option<char> {
        Some('w')
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

                    let mut switch = TheGroupButton::new(TheId::named("World Helper Switch"));
                    switch.add_text_status("Camera".to_string(), "Camera.".to_string());
                    switch.add_text_status(
                        "Terrain".to_string(),
                        "Apply procedural materials.".to_string(),
                    );
                    switch.set_index(server_ctx.curr_world_tool_helper as i32);
                    layout.add_widget(Box::new(switch));

                    let mut camera_switch =
                        TheGroupButton::new(TheId::named("World Camera Helper Switch"));
                    camera_switch.add_text_status("Orbit".to_string(), "Orbit.".to_string());
                    camera_switch.add_text_status(
                        "FirstP".to_string(),
                        "Apply procedural materials.".to_string(),
                    );
                    camera_switch.set_index(server_ctx.curr_world_tool_camera as i32);
                    layout.add_widget(Box::new(camera_switch));

                    if server_ctx.curr_world_tool_helper == WorldToolHelper::Camera {
                        ctx.ui.send(TheEvent::SetStackIndex(
                            TheId::named("Main Stack"),
                            PanelIndices::TilePicker as usize,
                        ));
                    }

                    layout.set_reverse_index(Some(1));
                }

                server_ctx.world_mode = true;
                return true;
            }
            DeActivate => {
                if let Some(layout) = ui.get_hlayout("Game Tool Params") {
                    layout.clear();
                    layout.set_reverse_index(None);
                }
                // server_ctx.curr_map_tool_type = MapToolType::General;
                server_ctx.world_mode = false;
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
        WORLDEDITOR
            .write()
            .unwrap()
            .map_event(map_event, ui, ctx, map, server_ctx)
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let redraw = false;
        #[allow(clippy::single_match)]
        match event {
            TheEvent::RenderViewScrollBy(id, coord) => {
                if id.name == "PolyView" {
                    WORLDEDITOR
                        .write()
                        .unwrap()
                        .scroll_by(ui, ctx, server_ctx, *coord);
                }
            }
            _ => {}
        }
        redraw
    }
}
