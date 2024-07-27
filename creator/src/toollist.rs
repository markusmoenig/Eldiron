use crate::editor::RENDERER;
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
        let game_tools: Vec<Box<dyn Tool>> = vec![
            Box::new(TileDrawerTool::new()),
            Box::new(DrawTool::new()),
            Box::new(MapObjectsTool::new()),
            Box::new(CodeTool::new()),
            Box::new(PickerTool::new()),
            Box::new(EraserTool::new()),
            Box::new(SelectionTool::new()),
            Box::new(TilemapTool::new()),
            Box::new(RenderTool::new()),
            Box::new(ZoomTool::new()),
        ];
        let screen_tools: Vec<Box<dyn Tool>> = vec![
            Box::new(ScreenTileDrawerTool::new()),
            Box::new(CodeTool::new()),
            Box::new(ScreenPickerTool::new()),
            Box::new(ScreenEraserTool::new()),
        ];

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
        list: &mut dyn TheVLayoutTrait,
        ctx: &mut TheContext,
    ) {
        self.active_editor = active_editor;
        list.clear();
        ctx.ui.relayout = true;

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
            ScreenEditor => {
                for (index, tool) in self.screen_tools.iter().enumerate() {
                    let mut b = TheToolListButton::new(tool.id());

                    b.set_icon_name(tool.icon_name());
                    b.set_status_text(&tool.info());
                    if index == self.curr_screen_tool {
                        b.set_state(TheWidgetState::Selected);
                    }
                    list.add_widget(Box::new(b));
                }
            }
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
            TheEvent::StateChanged(id, state) => {
                if id.name.contains("Tool") && *state == TheWidgetState::Selected {
                    let mut switched_tool = false;

                    match self.active_editor {
                        GameEditor => {
                            let mut old_tool_index = 0;
                            for (index, tool) in self.game_tools.iter().enumerate() {
                                if tool.id().uuid == id.uuid && index != self.curr_game_tool {
                                    switched_tool = true;
                                    old_tool_index = self.curr_game_tool;
                                    self.curr_game_tool = index;
                                    redraw = true;
                                }
                            }
                            if switched_tool {
                                for tool in self.game_tools.iter() {
                                    if tool.id().uuid != id.uuid {
                                        ctx.ui.set_widget_state(
                                            tool.id().name.clone(),
                                            TheWidgetState::None,
                                        );
                                    }
                                }
                                self.game_tools[old_tool_index].tool_event(
                                    ToolEvent::DeActivate,
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
                        ScreenEditor => {
                            let mut old_tool_index = 0;
                            for (index, tool) in self.screen_tools.iter().enumerate() {
                                if tool.id().uuid == id.uuid && index != self.curr_screen_tool {
                                    switched_tool = true;
                                    old_tool_index = self.curr_screen_tool;
                                    self.curr_screen_tool = index;
                                    redraw = true;
                                }
                            }
                            if switched_tool {
                                for tool in self.screen_tools.iter() {
                                    if tool.id().uuid != id.uuid {
                                        ctx.ui.set_widget_state(
                                            tool.id().name.clone(),
                                            TheWidgetState::None,
                                        );
                                    }
                                }
                                self.screen_tools[old_tool_index].tool_event(
                                    ToolEvent::DeActivate,
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

                    self.get_current_tool().tool_event(
                        ToolEvent::Activate,
                        ToolContext::TwoD,
                        ui,
                        ctx,
                        project,
                        server,
                        client,
                        server_ctx,
                    );

                    ctx.ui.relayout = true;
                }
            }
            TheEvent::TileEditorClicked(id, coord) => {
                if id.name == "Region Editor View" || id.name == "Screen Editor View" {
                    let mut coord_f = Vec2f::from(*coord);
                    if id.name == "Region Editor View" {
                        if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                coord_f = rgba_view.float_pos();
                            }
                        }
                    }

                    self.get_current_tool().tool_event(
                        ToolEvent::TileDown(*coord, coord_f),
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
            TheEvent::TileEditorDragged(id, coord) => {
                if id.name == "Region Editor View" || id.name == "Screen Editor View" {
                    let mut coord_f = Vec2f::from(*coord);
                    if id.name == "Region Editor View" {
                        if let Some(editor) = ui.get_rgba_layout("Region Editor") {
                            if let Some(rgba_view) = editor.rgba_view_mut().as_rgba_view() {
                                coord_f = rgba_view.float_pos();
                            }
                        }
                    }

                    self.get_current_tool().tool_event(
                        ToolEvent::TileDrag(*coord, coord_f),
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
            TheEvent::TileEditorUp(id) => {
                if id.name == "Region Editor View" || id.name == "Screen Editor View" {
                    self.get_current_tool().tool_event(
                        ToolEvent::TileUp,
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
            TheEvent::RenderViewClicked(id, coord) => {
                if id.name == "RenderView" {
                    if let Some(render_view) = ui.get_render_view("RenderView") {
                        let dim = render_view.dim();
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let pos = RENDERER.lock().unwrap().get_hit_position_at(
                                *coord,
                                region,
                                &mut server.get_instance_draw_settings(server_ctx.curr_region),
                                dim.width as usize,
                                dim.height as usize,
                            );

                            if let Some((pos, pos_f)) = pos {
                                redraw = self.get_current_tool().tool_event(
                                    ToolEvent::TileDown(
                                        vec2i(pos.x, pos.z),
                                        vec2f(pos_f.x, pos_f.z),
                                    ),
                                    ToolContext::ThreeD,
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
            }
            TheEvent::RenderViewDragged(id, coord) => {
                if id.name == "RenderView" {
                    if let Some(render_view) = ui.get_render_view("RenderView") {
                        let dim = render_view.dim();
                        if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                            let pos = RENDERER.lock().unwrap().get_hit_position_at(
                                *coord,
                                region,
                                &mut server.get_instance_draw_settings(server_ctx.curr_region),
                                dim.width as usize,
                                dim.height as usize,
                            );

                            if let Some((pos, pos_f)) = pos {
                                redraw = self.get_current_tool().tool_event(
                                    ToolEvent::TileDrag(
                                        vec2i(pos.x, pos.z),
                                        vec2f(pos_f.x, pos_f.z),
                                    ),
                                    ToolContext::ThreeD,
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
            }
            _ => {}
        }

        if !redraw {
            redraw = self
                .get_current_tool()
                .handle_event(event, ui, ctx, project, server, client, server_ctx);
        }

        redraw
    }

    /// Returns the curently active tool.
    fn get_current_tool(&mut self) -> &mut Box<dyn Tool> {
        match &self.active_editor {
            GameEditor => &mut self.game_tools[self.curr_game_tool],
            ScreenEditor => &mut self.screen_tools[self.curr_screen_tool],
        }
    }
}
