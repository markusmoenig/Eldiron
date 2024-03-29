use crate::prelude::*;

pub struct ScreenEditor<'a> {
    pub rect: (usize, usize, usize, usize),
    pub region_id: usize,

    grid_size: usize,

    offset: (isize, isize),
    screen_offset: (usize, usize),

    pub tile_selector: TileSelectorWidget,

    mouse_wheel_delta: (isize, isize),

    mouse_hover_pos: (usize, usize),
    pub clicked: Option<(isize, isize)>,

    widget_start: Option<(isize, isize)>,
    widget_end: Option<(isize, isize)>,

    selector_size: usize,

    game_render: Option<GameRender<'a>>,

    player_position: Option<Position>,
    player_tile: Option<TileId>,

    hover_rect: Option<(usize, usize, usize, usize)>,

    screen_script_name: String,
}

impl EditorContent for ScreenEditor<'_> {
    fn new(
        _text: Vec<String>,
        rect: (usize, usize, usize, usize),
        _behavior_type: BehaviorType,
        asset: &Asset,
        context: &ScreenContext,
    ) -> Self {
        let bottom_size = 250;

        // Tile Selector
        let mut tile_selector = TileSelectorWidget::new(
            vec![],
            (rect.0, rect.1 + rect.3 - bottom_size, rect.2, bottom_size),
            asset,
            &context,
        );
        tile_selector.set_tile_type(
            vec![TileUsage::UIElement, TileUsage::Icon],
            None,
            None,
            &asset,
        );

        Self {
            rect,
            region_id: 0,
            grid_size: 32,

            offset: (0, 0),
            screen_offset: (0, 0),

            tile_selector,

            mouse_wheel_delta: (0, 0),
            mouse_hover_pos: (0, 0),
            clicked: None,

            widget_start: None,
            widget_end: None,

            selector_size: 250,

            game_render: None,

            player_position: None,
            player_tile: None,

            hover_rect: None,

            screen_script_name: "main.rhai".to_string(),
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &mut ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;

        self.tile_selector.rect = (
            self.rect.0,
            self.rect.1 + self.rect.3 - self.selector_size,
            width,
            self.selector_size,
        );
        self.tile_selector.resize(width, self.selector_size);
    }

    fn draw(
        &mut self,
        frame: &mut [u8],
        anim_counter: usize,
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
    ) {
        context
            .draw2d
            .draw_rect(frame, &self.rect, context.width, &context.color_black);

        let mut tile_size = 32;

        if let Some(render) = &mut self.game_render {
            tile_size = render.tile_size;

            let mut update = GameUpdate::new();

            // If the screen script was changed, update the script in memory
            if context.code_editor_update_from_file {
                context.data.scripts.insert(
                    self.screen_script_name.clone(),
                    context.code_editor_value.clone(),
                );

                update.screen_scripts = Some(context.data.scripts.clone());
                update.screen_script_name = Some(self.screen_script_name.clone());
                context.code_editor_update_from_file = false;
            }

            update.position = self.player_position.clone();
            context.code_editor_error = render.draw(anim_counter, Some(&update));

            let left_offset = 0;
            let top_offset = 0;

            context.draw2d.blend_slice_safe(
                frame,
                &mut render.frame[..],
                &(
                    self.rect.0 as isize
                        + left_offset as isize
                        + self.offset.0 * render.tile_size as isize,
                    self.rect.1 as isize
                        + top_offset as isize
                        + self.offset.1 * render.tile_size as isize,
                    render.width,
                    render.height,
                ),
                context.width,
                &self.rect,
            );
        }

        if self.mouse_hover_pos != (0, 0) {
            if let Some(id) = self.get_tile_id(self.mouse_hover_pos) {
                let pos = (
                    self.rect.0 + ((id.0 + self.offset.0) as usize) * tile_size,
                    self.rect.1 + ((id.1 + self.offset.1) as usize) * tile_size,
                );

                if id.0 >= 0 && id.1 >= 0 {
                    self.hover_rect = Some((
                        id.0 as usize * tile_size,
                        id.1 as usize * tile_size,
                        tile_size,
                        tile_size,
                    ));
                } else {
                    self.hover_rect = None;
                }

                if pos.0 + tile_size < self.rect.0 + self.rect.2
                    && pos.1 + tile_size < self.rect.1 + self.rect.3
                {
                    context.draw2d.draw_rect_outline(
                        frame,
                        &(pos.0, pos.1, tile_size, tile_size),
                        context.width,
                        context.color_light_white,
                    );
                    context.draw2d.draw_rect_outline(
                        frame,
                        &(pos.0 + 1, pos.1 + 1, tile_size - 2, tile_size - 2),
                        context.width,
                        context.color_black,
                    );
                }
            }
        }

        if let Some(options) = options {
            let mode = options.get_screen_editor_mode();

            if mode == ScreenEditorMode::Tiles {
                self.tile_selector
                    .draw(frame, context.width, anim_counter, asset, context);
            }
        }
    }

    fn mouse_down(
        &mut self,
        pos: (usize, usize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        let mut consumed = false;

        self.widget_start = None;
        self.widget_end = None;

        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Tiles {
                if self.tile_selector.mouse_down(pos, asset, context) {
                    consumed = true;
                    if let Some(selected) = &self.tile_selector.selected {
                        context.curr_region_tile = Some(selected.clone());
                    } else {
                        context.curr_region_tile = None;
                    }
                }
            }

            if consumed == false && context.contains_pos_for(pos, self.rect) {
                if let Some(id) = self.get_tile_id(pos) {
                    self.clicked = Some(id);
                }
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_up(
        &mut self,
        _pos: (usize, usize),
        _asset: &mut Asset,
        _context: &mut ScreenContext,
        _options: &mut Option<Box<dyn EditorOptions>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        self.clicked = None;

        let consumed = false;

        self.widget_start = None;
        self.widget_end = None;

        consumed
    }

    fn mouse_hover(
        &mut self,
        pos: (usize, usize),
        _asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        let mut rect = self.rect.clone();

        if let Some(options) = options {
            let mode = options.get_screen_editor_mode();

            if mode != ScreenEditorMode::None {
                rect.3 -= context.code_editor_height;
            }
        }

        if context.contains_pos_for(pos, rect) {
            self.mouse_hover_pos = pos.clone();
            return true;
        }
        false
    }

    fn mouse_dragged(
        &mut self,
        _pos: (usize, usize),
        _asset: &mut Asset,
        _context: &mut ScreenContext,
        _options: &mut Option<Box<dyn EditorOptions>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        let consumed = false;
        consumed
    }

    fn mouse_wheel(
        &mut self,
        delta: (isize, isize),
        asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
        _toolbar: &mut Option<&mut ToolBar>,
    ) -> bool {
        let mut consumed = false;
        if let Some(options) = options {
            let editor_mode = options.get_editor_mode();

            if editor_mode == RegionEditorMode::Tiles {
                if context.contains_pos_for(self.mouse_hover_pos, self.tile_selector.rect)
                    && self.tile_selector.mouse_wheel(delta, asset, context)
                {
                    consumed = true;
                }
            }

            if consumed == false {
                self.mouse_wheel_delta.0 += delta.0;
                self.mouse_wheel_delta.1 += delta.1;

                self.offset.0 += self.mouse_wheel_delta.0 / self.grid_size as isize;
                self.offset.1 += self.mouse_wheel_delta.1 / self.grid_size as isize;

                self.mouse_wheel_delta.0 -=
                    (self.mouse_wheel_delta.0 / self.grid_size as isize) * self.grid_size as isize;
                self.mouse_wheel_delta.1 -=
                    (self.mouse_wheel_delta.1 / self.grid_size as isize) * self.grid_size as isize;
            }
        }
        true
    }

    /// Get the tile id
    fn get_tile_id(&self, pos: (usize, usize)) -> Option<(isize, isize)> {
        let grid_size = self.grid_size;
        if pos.0 > self.rect.0 + self.screen_offset.0
            && pos.1 > self.rect.1 + self.screen_offset.1
            && pos.0 < self.rect.0 + self.rect.2 - self.screen_offset.0
            && pos.1 < self.rect.1 + self.rect.3 - self.screen_offset.1
        //} - self.selector_size
        {
            let x =
                ((pos.0 - self.rect.0 - self.screen_offset.0) / grid_size) as isize - self.offset.0;
            let y =
                ((pos.1 - self.rect.1 - self.screen_offset.0) / grid_size) as isize - self.offset.1;
            return Some((x, y));
        }
        None
    }

    /// Returns the selected tile
    fn get_selected_tile(&self) -> Option<TileData> {
        self.tile_selector.selected.clone()
    }

    /// Return the tile_selector
    fn get_tile_selector(&mut self) -> Option<&mut TileSelectorWidget> {
        Some(&mut self.tile_selector)
    }

    /// Returns the selected tile
    fn get_hover_rect(&self) -> Option<(usize, usize, usize, usize)> {
        self.hover_rect
    }

    /// Screen is opening
    fn opening(
        &mut self,
        _asset: &mut Asset,
        context: &mut ScreenContext,
        options: &mut Option<Box<dyn EditorOptions>>,
    ) {
        self.screen_script_name = "main.rhai".into();

        self.game_render = Some(GameRender::new(
            context.curr_project_path.clone(),
            context.player_id,
        ));

        if let Some(render) = &mut self.game_render {
            let mut update = GameUpdate::new();
            update.screen_scripts = Some(context.data.scripts.clone());

            // Get the region the player is in

            if context.data.behaviors_ids.len() > 0 {
                if let Some(behavior) = context
                    .data
                    .behaviors
                    .get_mut(&context.data.behaviors_ids[0])
                {
                    for (_id, node) in &behavior.data.nodes {
                        if node.behavior_type == BehaviorNodeType::BehaviorType {
                            if let Some(value) = node.values.get(&"position".to_string()) {
                                self.player_position = value.to_position();
                            }
                            if let Some(value) = node.values.get(&"tile".to_string()) {
                                self.player_tile = value.to_tile_id();
                            }
                            break;
                        }
                    }
                }
            }

            if let Some(position) = &self.player_position {
                if let Some(region) = context.data.regions.get(&position.region) {
                    // Send the region to the client_render
                    update.region = Some(region.data.clone());
                }
            }

            let mut id = context.code_editor_node_behavior_id.clone();
            id.2 = "script_name".into();

            if let Some(name) = context
                .data
                .get_behavior_id_value_raw(id, BehaviorType::GameLogic)
            {
                if let Some(script_name) = name.to_string() {
                    update.screen_script_name = Some(script_name.clone());
                    self.screen_script_name = script_name;
                }
            }

            update.position = self.player_position.clone();
            context.code_editor_error = render.process_update(&update);
            self.grid_size = render.tile_size;
        }

        context.data.update_scripts();

        let keys: Vec<&String> = context.data.scripts.keys().collect();

        let mut index = 0;

        for (i, k) in keys.iter().enumerate() {
            if **k == self.screen_script_name {
                index = i;
                break;
            }
        }

        // Set the scripts
        if let Some(options) = options {
            options.set_script_names(keys.clone(), index);
        }

        if keys.is_empty() == false {
            if let Some(script) = context.data.scripts.get(&self.screen_script_name.clone()) {
                let path = context
                    .curr_project_path
                    .join("game")
                    .join("scripts")
                    .join(self.screen_script_name.clone());
                context.code_editor_file_path = Some(path);
                context.open_code_editor(
                    context.code_editor_node_behavior_id.clone(),
                    Value::String(script.clone()),
                    true,
                );
            }
        }
    }

    /// Screen is closing
    fn closing(
        &mut self,
        _asset: &mut Asset,
        context: &mut ScreenContext,
        _options: &mut Option<Box<dyn EditorOptions>>,
    ) {
        self.game_render = None;

        if let Some(path) = &context.code_editor_file_path {
            _ = std::fs::write(path, context.code_editor_value.clone());
            context.data.update_scripts();
        }

        context.code_editor_file_path = None;
    }

    /// Set the current script
    fn set_current_script(&mut self, script: String, context: &mut ScreenContext) {
        let script_name = script + ".rhai";

        // Save the current one
        if let Some(path) = &context.code_editor_file_path {
            _ = std::fs::write(path, context.code_editor_value.clone());
            context.data.update_scripts();
        }

        // Load the new one
        let path = context
            .curr_project_path
            .join("game")
            .join("scripts")
            .join(script_name);
        context.code_editor_file_path = Some(path.clone());

        if let Some(script) = std::fs::read_to_string(path).ok() {
            context.open_code_editor(
                context.code_editor_node_behavior_id.clone(),
                Value::String(script.clone()),
                true,
            );
        }
    }
}
