use crate::prelude::*;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum RegionEditorMode {
    Tiles,
    Areas,
    Characters,
    Loot,
    Settings
}

pub struct RegionOptions {
    rect                    : (usize, usize, usize, usize),
    tile_rect               : (usize, usize, usize, usize),

    pub layouts             : Vec<VLayout>,

    curr_layer              : usize,

    mode                    : RegionEditorMode,
}

impl EditorOptions for RegionOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut layouts : Vec<VLayout> = vec![];

        let mut tile_layout = VLayout::new(rect);

        // Tile Widgets
        let mut tilemap_names = asset.tileset.maps_names.clone();
        tilemap_names.insert(0, "All".to_string());

        let mut tilemaps_button = AtomWidget::new(tilemap_names, AtomWidgetType::SliderButton,
        AtomData::new("Tilemaps", Value::Empty()));
        tilemaps_button.atom_data.text = "Tilemaps".to_string();
        tilemaps_button.set_rect((0, 0, rect.2 - 20, 40), asset, context);
        tile_layout.add(tilemaps_button, 0);

        let mut tags_button = AtomWidget::new(vec!["".to_string()], AtomWidgetType::TagsButton,
            AtomData::new("Tags", Value::Empty()));
        tags_button.set_rect((0, 0, rect.2 - 20, 40), asset, context);
        tile_layout.add(tags_button, 0);

        let mut usage_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new("UsageList", Value::Empty()));
        usage_list.add_group_list(context.color_blue, context.color_light_blue, vec!["All".to_string(), "Environment".to_string(), "Road".to_string(), "Blocking".to_string(), "Water".to_string(), "UI Element".to_string()]);
        usage_list.set_rect((0, 0, rect.2 - 20, 210), asset, context);
        tile_layout.add(usage_list, 3);

        let mut layer_button = AtomWidget::new(vec!["F".to_string(), "W".to_string(), "C".to_string(), "O".to_string()], AtomWidgetType::LayerNumberRow, AtomData::new("Layer", Value::Empty()));
        layer_button.set_rect((0, 0, rect.2 - 20, 30), asset, context);
        tile_layout.add(layer_button, 3);

        let mut remap_button = AtomWidget::new(vec!["Remap".to_string()], AtomWidgetType::Button,
        AtomData::new("Remap", Value::Empty()));
        remap_button.set_rect((0, 0, rect.2 - 20, 40), asset, context);
        tile_layout.add(remap_button, 5);

        tile_layout.layout();
        layouts.push(tile_layout);

        // Area Behavior

        let mut area_layout = VLayout::new(rect);
        // area_layout.margin.0 = 0;
        // area_layout.margin.2 = 0;

        let mut node_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new("NodeList", Value::Empty()));
        node_list.drag_enabled = true;

        node_list.add_group_list(context.color_green, context.color_light_green, vec!["Action".to_string(), "Always".to_string(), "Enter Area".to_string(), "Leave Area".to_string(), "Inside Area".to_string()]);

        node_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Audio".to_string(), "Light".to_string(), "Message".to_string(), "Overlay Tiles".to_string(), "Spawn".to_string(), "Teleport".to_string()]);

        node_list.set_rect((0, 0, rect.2 - 20, rect.3 - 200), asset, context);
        area_layout.add(node_list, 0);

        area_layout.layout();
        layouts.push(area_layout);

        Self {
            rect,
            tile_rect                   : (0, 0, 0, 0),

            layouts,

            curr_layer                  : 1,

            mode                        : RegionEditorMode::Tiles,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;

        for index in 0..self.layouts.len() {
            self.layouts[index].set_rect(self.rect);
            self.layouts[index].layout();
        }
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        let mode = self.get_editor_mode();

        if mode == RegionEditorMode::Tiles {
            if let Some(content) = content {
                let mask = content.get_layer_mask(context);
                self.layouts[self.mode as usize].widgets[3].button_mask = mask;
                self.layouts[self.mode as usize].widgets[3].dirty = true;
            }
        }

        if mode == RegionEditorMode::Tiles {
            self.layouts[self.mode as usize].draw(frame, anim_counter, asset, context);
        } else
        if mode == RegionEditorMode::Areas {
            self.layouts[self.mode as usize].draw(frame, anim_counter, asset, context);
        }

        if mode == RegionEditorMode::Tiles {
            if let Some(content) = content {
                if let Some(tile) = content.get_selected_tile() {

                    let mut y = self.layouts[self.mode as usize].end + 20;
                    let mut name = "".to_string();

                    if let Some(map) = asset.get_map_of_id(tile.tilemap) {
                        context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 100) / 2, y), map, context.width, &(tile.x_off as usize, tile.y_off as usize), anim_counter, 100);
                        name = map.get_name();
                        self.tile_rect = ((self.rect.2 - 100) / 2, y, 100, 100);
                    }

                    y += 105;
                    context.draw2d.draw_text_rect(frame, &(0, y, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("{}: {}, {}", name, tile.x_off, tile.y_off), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
                }
            }
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {

        let mode = self.get_editor_mode();

        if context.contains_pos_for(pos, self.tile_rect) {
            context.switch_editor_state = Some(EditorState::TilesDetail);
            if let Some(content) = content {
                context.switch_tilemap_to_tile = content.get_selected_tile().clone();
            }
            return true;
        }

        if mode == RegionEditorMode::Tiles {

            if let Some(id) = self.layouts[self.mode as usize].mouse_down(pos, asset, context) {
                if let Some(content) = content {
                    if id.1 == "UsageList".to_string() {
                        if let Some(tile_selector) = content.get_tile_selector() {
                            tile_selector.set_tile_type(self.get_tile_usage(), self.get_tilemap_index(), self.get_tags(), &asset);
                        }
                    } else
                    if id.1 == "Layer".to_string() {
                        self.curr_layer = self.layouts[self.mode as usize].widgets[3].curr_index + 1;
                    } else
                    if id.1 == "Remap".to_string() {
                        if let Some(region) = context.data.regions.get_mut(&content.get_region_id()) {
                            region.remap(asset);
                        }
                    }
                }
                return true;
            }
        } else
        if mode == RegionEditorMode::Areas {
            if let Some(_id) = self.layouts[self.mode as usize].mouse_down(pos, asset, context) {
                return true;
            }
        }

        false
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) -> bool {

        let mode = self.get_editor_mode();
        let tags = self.get_tags();
        let usage = self.get_tile_usage();

        // Tiles Mode
        if mode == RegionEditorMode::Tiles {

            if let Some(id) = self.layouts[self.mode as usize].mouse_up(pos, asset, context) {

                if id.1 == "Tilemaps".to_string() {
                    if let Some(el_content) = content {
                        if let Some(tile_selector) = el_content.get_tile_selector() {
                            if id.0 == 0 {
                                tile_selector.set_tile_type(usage, None, tags, &asset);
                            } else {
                                tile_selector.set_tile_type(usage, Some(id.0 - 1), tags, &asset);
                            }
                            self.layouts[self.mode as usize].widgets[id.0].dirty = true;
                        }
                    }
                }

            }
        } else
        if mode == RegionEditorMode::Areas {
            if let Some(_id) = self.layouts[self.mode as usize].mouse_down(pos, asset, context) {
                return true;
            }
        }

        false
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {

        let mode = self.get_editor_mode();

        if mode == RegionEditorMode::Tiles || mode == RegionEditorMode::Areas  {
            if let Some(_id) = self.layouts[self.mode as usize].mouse_hover(pos, asset, context) {
                return true;
            }
        }

        false
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {

        let mode = self.get_editor_mode();

        if mode == RegionEditorMode::Areas {
            if let Some(drag_context) = &self.layouts[self.mode as usize].widgets[0].drag_context {
                if context.drag_context == None {

                    let mut buffer = [0; 180 * 32 * 4];

                    context.draw2d.draw_rect(&mut buffer[..], &(0, 0, 180, 32), 180, &drag_context.color.clone());
                    context.draw2d.draw_text_rect(&mut buffer[..], &(0, 0, 180, 32), 180, &asset.get_editor_font("OpenSans"), context.toolbar_button_text_size, drag_context.text.as_str(), &context.color_white, &drag_context.color.clone(), draw2d::TextAlignment::Center);

                    context.drag_context = Some(ScreenDragContext {
                        text    : drag_context.text.clone(),
                        color   : drag_context.color.clone(),
                        offset  : drag_context.offset.clone(),
                        buffer  : Some(buffer)
                    });
                    context.target_fps = 60;
                }
                self.layouts[self.mode as usize].widgets[0].drag_context = None;
            }
        } else
        if mode == RegionEditorMode::Tiles {
            if let Some(_id) = self.layouts[self.mode as usize].mouse_dragged(pos, asset, context) {
                return true;
            }
        }

        false
    }

    /// Returns the current editor mode
    fn get_editor_mode(&self) -> RegionEditorMode {
        self.mode
    }

    fn set_editor_mode(&mut self, mode: RegionEditorMode) {
        self.mode = mode;
    }

    /// Get the current tile usage
    fn get_tile_usage(&self) -> Vec<TileUsage> {
        match self.layouts[RegionEditorMode::Tiles as usize].widgets[2].curr_item_index {
            1 => vec![TileUsage::Environment],
            2 => vec![TileUsage::EnvRoad],
            3 => vec![TileUsage::EnvBlocking],
            4 => vec![TileUsage::Water],
            5 => vec![TileUsage::UIElement],
            _ => vec![],
        }
    }

    /// Get the current tile_id if any
    fn get_tilemap_index(&self) -> Option<usize> {
        if self.layouts[RegionEditorMode::Tiles as usize].widgets[0].curr_index > 0 {
            return Some(self.layouts[RegionEditorMode::Tiles as usize].widgets[0].curr_index - 1);
        }
        None
    }

    /// Get the current tags
    fn get_tags(&self) -> Option<String> {
        if self.layouts[RegionEditorMode::Tiles as usize].widgets[1].text[0].len() > 0 {
            return Some(self.layouts[RegionEditorMode::Tiles as usize].widgets[1].text[0].clone());
        }
        None
    }

    /// Get the current layer
    fn get_layer(&self) -> usize {
        self.curr_layer
    }

    /// Set the current layer
    fn set_layer(&mut self, layer: usize)  {
        self.curr_layer = layer;
        self.layouts[self.mode as usize].widgets[3].dirty = true;
        self.layouts[self.mode as usize].widgets[3].curr_index = layer - 1;
    }

    /// Updates a value from the dialog
    fn update_from_dialog(&mut self, id: (Uuid, Uuid, String), value: Value, asset: &mut Asset, _context: &mut ScreenContext, content: &mut Option<Box<dyn EditorContent>>) {
        if id.2 == "tags".to_string() {
            self.layouts[RegionEditorMode::Tiles as usize].widgets[1].text[0] = value.to_string_value().to_lowercase();
            self.layouts[RegionEditorMode::Tiles as usize].widgets[1].dirty = true;

            if let Some(content) = content {
                if let Some(tile_selector) = content.get_tile_selector() {
                    tile_selector.set_tile_type(self.get_tile_usage(), self.get_tilemap_index(), self.get_tags(), &asset);
                }
            }
        }
    }
}