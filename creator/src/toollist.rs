use crate::editor::{PRERENDERTHREAD, RENDERER, RENDERMODE};
use crate::prelude::*;

pub use ActiveEditor::*;

pub struct ToolList {
    pub server_time: TheTime,
    pub render_button_text: String,

    pub active_editor: ActiveEditor,

    pub game_tools: Vec<Box<dyn Tool>>,
    pub curr_game_tool: usize,

    pub screen_tools: Vec<Box<dyn Tool>>,
    pub curr_screen_tool: usize,

    pub terrain_tools: Vec<Box<dyn Tool>>,
    pub curr_terrain_tool: usize,
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
            Box::new(FXTool::new()),
            Box::new(PickerTool::new()),
            Box::new(EraserTool::new()),
            Box::new(SelectionTool::new()),
            Box::new(TilemapTool::new()),
            Box::new(RenderTool::new()),
            Box::new(ZoomTool::new()),
            Box::new(GameTool::new()),
        ];
        let screen_tools: Vec<Box<dyn Tool>> = vec![
            Box::new(ScreenTileDrawerTool::new()),
            Box::new(CodeTool::new()),
            Box::new(ScreenPickerTool::new()),
            Box::new(ScreenEraserTool::new()),
            Box::new(ScreenGameTool::new()),
        ];
        let terrain_tools: Vec<Box<dyn Tool>> = vec![Box::new(TerrainDrawTool::new())];
        Self {
            server_time: TheTime::default(),
            render_button_text: "Finished".to_string(),

            active_editor: ActiveEditor::GameEditor,

            game_tools,
            curr_game_tool: 0,

            screen_tools,
            curr_screen_tool: 0,

            terrain_tools,
            curr_terrain_tool: 0,
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
            TerrainEditor => {
                for (index, tool) in self.terrain_tools.iter().enumerate() {
                    let mut b = TheToolListButton::new(tool.id());

                    b.set_icon_name(tool.icon_name());
                    b.set_status_text(&tool.info());
                    if index == self.curr_terrain_tool {
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
            TheEvent::KeyDown(TheValue::Char(c)) => {
                let mut acc = !ui.focus_widget_supports_text_input(ctx);

                if self.get_current_tool().id().name == "Game Tool" {
                    acc = false;
                }

                if acc {
                    match self.active_editor {
                        GameEditor => {
                            if (*c == '-' || *c == '=' || *c == '+') && (ui.ctrl || ui.logo) {
                                // Global Zoom In / Zoom Out
                                if let Some(region) =
                                    project.get_region_mut(&server_ctx.curr_region)
                                {
                                    if *c == '=' || *c == '+' {
                                        region.zoom += 0.2;
                                    } else {
                                        region.zoom -= 0.2;
                                    }
                                    region.zoom = region.zoom.clamp(1.0, 5.0);
                                    server.set_zoom(region.id, region.zoom);
                                    if let Some(layout) = ui.get_rgba_layout("Region Editor") {
                                        layout.set_zoom(region.zoom);
                                        layout.relayout(ctx);
                                    }
                                    if let Some(edit) = ui.get_text_line_edit("Editor Zoom") {
                                        edit.set_value(TheValue::Float(region.zoom));
                                    }
                                    return true;
                                }
                            }

                            let mut tool_uuid = None;
                            for tool in self.game_tools.iter() {
                                if tool.accel() == Some(*c) {
                                    tool_uuid = Some(tool.id().uuid);
                                    ctx.ui.set_widget_state(
                                        self.game_tools[self.curr_game_tool].id().name,
                                        TheWidgetState::None,
                                    );
                                    ctx.ui
                                        .set_widget_state(tool.id().name, TheWidgetState::Selected);
                                }
                            }
                            if let Some(uuid) = tool_uuid {
                                self.set_tool(uuid, ui, ctx, project, server, client, server_ctx);
                            }
                        }
                        ScreenEditor => {
                            let mut tool_uuid = None;
                            for tool in self.screen_tools.iter() {
                                if tool.accel() == Some(*c) {
                                    tool_uuid = Some(tool.id().uuid);
                                    ctx.ui.set_widget_state(
                                        self.screen_tools[self.curr_screen_tool].id().name,
                                        TheWidgetState::None,
                                    );
                                    ctx.ui
                                        .set_widget_state(tool.id().name, TheWidgetState::Selected);
                                }
                            }
                            if let Some(uuid) = tool_uuid {
                                self.set_tool(uuid, ui, ctx, project, server, client, server_ctx);
                            }
                        }
                        TerrainEditor => {
                            let mut tool_uuid = None;
                            for tool in self.terrain_tools.iter() {
                                if tool.accel() == Some(*c) {
                                    tool_uuid = Some(tool.id().uuid);
                                    ctx.ui.set_widget_state(
                                        self.terrain_tools[self.curr_terrain_tool].id().name,
                                        TheWidgetState::None,
                                    );
                                    ctx.ui
                                        .set_widget_state(tool.id().name, TheWidgetState::Selected);
                                }
                            }
                            if let Some(uuid) = tool_uuid {
                                self.set_tool(uuid, ui, ctx, project, server, client, server_ctx);
                            }
                        }
                    }
                }
            }
            TheEvent::StateChanged(id, state) => {
                if id.name.contains("Tool") && *state == TheWidgetState::Selected {
                    redraw = self.set_tool(id.uuid, ui, ctx, project, server, client, server_ctx);
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
            TheEvent::ContextMenuSelected(widget_id, item_id) => {
                if widget_id.name == "Render Button" {
                    if let Some(region) = project.get_region_mut(&server_ctx.curr_region) {
                        if item_id.name == "Start Renderer" {
                            PRERENDERTHREAD.lock().unwrap().set_paused(false);
                        } else if item_id.name == "Pause Renderer" {
                            PRERENDERTHREAD.lock().unwrap().set_paused(true);
                        } else if item_id.name == "Restart Renderer" {
                            PRERENDERTHREAD.lock().unwrap().set_paused(false);
                            PRERENDERTHREAD
                                .lock()
                                .unwrap()
                                .render_region(region.clone(), None);
                        }
                        redraw = true;
                    }
                }
            }
            TheEvent::Custom(id, value) => {
                if id.name == "Set Game Tool" {
                    if let TheValue::Text(name) = value {
                        if let Some(tool_id) = self.get_game_tool_uuid_of_name(name) {
                            self.set_tool(tool_id, ui, ctx, project, server, client, server_ctx);
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
            TerrainEditor => &mut self.terrain_tools[self.curr_terrain_tool],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_tool(
        &mut self,
        tool_id: Uuid,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server: &mut Server,
        client: &mut Client,
        server_ctx: &mut ServerContext,
    ) -> bool {
        let mut redraw = false;
        let mut switched_tool = false;
        let layout_name;

        match self.active_editor {
            GameEditor => {
                layout_name = "Game Tool Params";
                let mut old_tool_index = 0;
                for (index, tool) in self.game_tools.iter().enumerate() {
                    if tool.id().uuid == tool_id && index != self.curr_game_tool {
                        switched_tool = true;
                        old_tool_index = self.curr_game_tool;
                        self.curr_game_tool = index;
                        redraw = true;
                    }
                }
                if switched_tool {
                    for tool in self.game_tools.iter() {
                        if tool.id().uuid != tool_id {
                            ctx.ui
                                .set_widget_state(tool.id().name.clone(), TheWidgetState::None);
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
                layout_name = "Screen Tool Params";
                let mut old_tool_index = 0;
                for (index, tool) in self.screen_tools.iter().enumerate() {
                    if tool.id().uuid == tool_id && index != self.curr_screen_tool {
                        switched_tool = true;
                        old_tool_index = self.curr_screen_tool;
                        self.curr_screen_tool = index;
                        redraw = true;
                    }
                }
                if switched_tool {
                    for tool in self.screen_tools.iter() {
                        if tool.id().uuid != tool_id {
                            ctx.ui
                                .set_widget_state(tool.id().name.clone(), TheWidgetState::None);
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
            TerrainEditor => {
                layout_name = "Terrain Tool Params";
                let mut old_tool_index = 0;
                for (index, tool) in self.terrain_tools.iter().enumerate() {
                    if tool.id().uuid == tool_id && index != self.curr_terrain_tool {
                        switched_tool = true;
                        old_tool_index = self.curr_terrain_tool;
                        self.curr_terrain_tool = index;
                        redraw = true;
                    }
                }
                if switched_tool {
                    for tool in self.terrain_tools.iter() {
                        if tool.id().uuid != tool_id {
                            ctx.ui
                                .set_widget_state(tool.id().name.clone(), TheWidgetState::None);
                        }
                    }
                    self.terrain_tools[old_tool_index].tool_event(
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

        if let Some(layout) = ui.get_hlayout(layout_name) {
            layout.clear();
            layout.set_reverse_index(None);
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

        if let Some(layout) = ui.get_hlayout(layout_name) {
            if layout.widgets().is_empty() {
                // Add default widgets

                let mut gb = TheGroupButton::new(TheId::named("2D3D Group"));
                gb.add_text("2D Map".to_string());
                gb.add_text("Mixed".to_string());
                gb.add_text("3D Map".to_string());

                match *RENDERMODE.lock().unwrap() {
                    EditorDrawMode::Draw2D => gb.set_index(0),
                    EditorDrawMode::DrawMixed => gb.set_index(1),
                    EditorDrawMode::Draw3D => gb.set_index(2),
                }

                let mut time_slider = TheTimeSlider::new(TheId::named("Server Time Slider"));
                time_slider.set_continuous(true);
                time_slider.limiter_mut().set_max_width(400);
                time_slider.set_value(TheValue::Time(self.server_time));

                let mut spacer = TheSpacer::new(TheId::empty());
                spacer.limiter_mut().set_max_width(30);

                let mut render_button = TheTraybarButton::new(TheId::named("Render Button"));
                render_button.set_text(self.render_button_text.clone());
                render_button.set_status_text("Controls the 3D background renderer. During rendering it displays how many tiles are left to render.");
                render_button.set_fixed_size(true);
                render_button.limiter_mut().set_max_width(80);

                render_button.set_context_menu(Some(TheContextMenu {
                    items: vec![
                        TheContextMenuItem::new(
                            "Start Renderer".to_string(),
                            TheId::named("Start Renderer"),
                        ),
                        TheContextMenuItem::new(
                            "Pause".to_string(),
                            TheId::named("Pause Renderer"),
                        ),
                        TheContextMenuItem::new(
                            "Restart".to_string(),
                            TheId::named("Restart Renderer"),
                        ),
                    ],
                    ..Default::default()
                }));

                layout.add_widget(Box::new(gb));
                layout.add_widget(Box::new(spacer));
                layout.add_widget(Box::new(time_slider));
                layout.add_widget(Box::new(render_button));
                layout.set_reverse_index(Some(1));
            }
        }

        ctx.ui.relayout = true;

        redraw
    }

    // Return the uuid given game tool.
    pub fn get_game_tool_uuid_of_name(&self, name: &str) -> Option<Uuid> {
        for tool in self.game_tools.iter() {
            if tool.id().name == name {
                return Some(tool.id().uuid);
            }
        }
        None
    }
}
