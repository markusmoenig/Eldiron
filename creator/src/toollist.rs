use std::any::Any;

use crate::editor::{PRERENDERTHREAD, TILEDRAWER, UNDOMANAGER};
use crate::prelude::*;

pub use ActiveEditor::*;

pub struct ToolList {
    pub active_editor: ActiveEditor,

    pub game_tools: Vec<Box<dyn Tool>>,
    pub curr_game_tool: usize,

    pub screen_tools: Vec<Box<dyn Tool>>,
    pub curr_screen_tool: usize,
}

impl Default for ToolList {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolList {
    pub fn new() -> Self {
        let game_tools: Vec<Box<dyn Tool>> = vec![Box::new(TileDrawerTool::new())];
        let screen_tools: Vec<Box<dyn Tool>> = vec![Box::new(TileDrawerTool::new())];

        Self {
            active_editor: ActiveEditor::GameEditor,

            game_tools,
            curr_game_tool: 0,

            screen_tools,
            curr_screen_tool: 0,
        }
    }

    /// Build the UI
    pub fn set_active_editor(
        &mut self,
        active_editor: ActiveEditor,
        list: &mut TheVLayout,
        _ctx: &mut TheContext,
    ) {
        self.active_editor = active_editor;

        match active_editor {
            GameEditor => {
                for (index, tool) in self.game_tools.iter().enumerate() {
                    let mut b = TheToolListButton::new(tool.id());

                    b.set_icon_name(tool.icon_name());
                    b.set_status_text(&tool.info());
                    if index == self.curr_game_tool {
                        b.set_state(TheWidgetState::Selected);
                    }
                    list.add_widget(Box::new(b));
                }
            }
            ScreenEditor => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "Region Editor View" {
                    match &self.active_editor {
                        GameEditor => {
                            redraw = self.game_tools[self.curr_game_tool].tool_event(
                                ToolEvent::TileDown(*coord),
                                ToolContext::TwoD,
                                ui,
                                ctx,
                                project,
                                server,
                                client,
                                server_ctx,
                            );
                        }
                        ScreenEditor => {
                            redraw = self.screen_tools[self.curr_screen_tool].tool_event(
                                ToolEvent::TileDown(*coord),
                                ToolContext::TwoD,
                                ui,
                                ctx,
                                project,
                                server,
                                client,
                                server_ctx,
                            );
                        }
                    }
                }
            }
            TheEvent::TileEditorDragged(id, coord) => {
                if id.name == "Region Editor View" {
                    match &self.active_editor {
                        GameEditor => {
                            redraw = self.game_tools[self.curr_game_tool].tool_event(
                                ToolEvent::TileDrag(*coord),
                                ToolContext::TwoD,
                                ui,
                                ctx,
                                project,
                                server,
                                client,
                                server_ctx,
                            );
                        }
                        ScreenEditor => {
                            redraw = self.screen_tools[self.curr_screen_tool].tool_event(
                                ToolEvent::TileDown(*coord),
                                ToolContext::TwoD,
                                ui,
                                ctx,
                                project,
                                server,
                                client,
                                server_ctx,
                            );
                        }
                    }
                }
            }
            _ => {}
        }

        if !redraw {
            redraw = match &self.active_editor {
                GameEditor => self.game_tools[self.curr_game_tool]
                    .handle_event(event, ui, ctx, project, server, client, server_ctx),
                ScreenEditor => self.screen_tools[self.curr_screen_tool]
                    .handle_event(event, ui, ctx, project, server, client, server_ctx),
            };
        }

        redraw
    }
}
