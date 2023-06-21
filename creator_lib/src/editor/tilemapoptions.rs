use crate::prelude::*;

pub struct TileMapOptions {
    rect                    : (usize, usize, usize, usize),
    widgets                 : Vec<AtomWidget>,
}

impl EditorOptions for TileMapOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, context: &ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut group_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new("GroupedList", Value::Empty()));

        group_list.state = WidgetState::Disabled;

        group_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Unused".to_string(), "Environment".to_string(), "Road".to_string(), "Blocking".to_string(), "Character".to_string(), "Utility".to_string(), "Water".to_string(), "Effect".to_string(), "Icon".to_string(), "UI Element".to_string()]);
        group_list.set_rect(rect);
        widgets.push(group_list);

        let mut settings_button = AtomWidget::new(vec!["Settings".to_string()], AtomWidgetType::Button,
            AtomData::new("Settings", Value::Empty()));
        settings_button.state = WidgetState::Disabled;
        settings_button.set_rect((rect.0 + 10, rect.1 + 310 + 30, rect.2 - 20, 40));
        widgets.push(settings_button);

        let mut set_anim_button = AtomWidget::new(vec!["Set Anim".to_string()], AtomWidgetType::Button,
            AtomData::new("Set Anim", Value::Empty()));
        set_anim_button.state = WidgetState::Disabled;
        set_anim_button.set_rect((rect.0 + 10, rect.1 + 305 + 80, rect.2 - 20, 40));
        widgets.push(set_anim_button);

        let mut clear_anim_button = AtomWidget::new(vec!["Clear Anim".to_string()], AtomWidgetType::Button,
        AtomData::new("Clear Anim", Value::Empty()));
        clear_anim_button.state = WidgetState::Disabled;
        clear_anim_button.set_rect((rect.0 + 10, rect.1 + 340 + 80, rect.2 - 20, 40));
        widgets.push(clear_anim_button);

        let mut set_default_button = AtomWidget::new(vec!["Set Default".to_string()], AtomWidgetType::Button,
        AtomData::new("Set Default", Value::Empty()));
        set_default_button.state = WidgetState::Disabled;
        set_default_button.set_rect((rect.0 + 10, rect.1 + 15 + 370 + 80, rect.2 - 20, 40));
        widgets.push(set_default_button);

        Self {
            rect,
            widgets             : widgets,
        }
    }

    fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) {
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        for atom in &mut self.widgets {
           atom.draw(frame, context.width, anim_counter, asset, context);
        }

        if let Some(grid_pos) = context.curr_tile {
            if let Some(map) = asset.get_map_of_id(asset.tileset.maps_ids[context.curr_tileset_index]) {
                context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 80) / 2, self.rect.1 + self.rect.3 - 102), map, context.width, &grid_pos, anim_counter, 80);

                context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + self.rect.3 - 22, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("({}, {})", grid_pos.0, grid_pos.1), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
            }
        }
    }

    // Sets the state of the widgets
    fn set_state(&mut self, state: WidgetState) {
        for a in &mut self.widgets {
            a.state = state.clone();
            a.dirty = true;
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>, _toolbar: &mut Option<&mut ToolBar>) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_down(pos, asset, context) {
                if atom.clicked {

                    if atom.atom_data.id == "GroupedList" {

                        if let Some(tile_id) = context.curr_tile {

                            let usage : TileUsage;
                            match atom.curr_item_index {
                                1 => usage = TileUsage::Environment,
                                2 => usage = TileUsage::EnvRoad,
                                3 => usage = TileUsage::EnvBlocking,
                                4 => usage = TileUsage::Character,
                                5 => usage = TileUsage::UtilityChar,
                                6 => usage = TileUsage::Water,
                                7 => usage = TileUsage::Effect,
                                8 => usage = TileUsage::Icon,
                                9 => usage = TileUsage::UIElement,
                                _ => usage = TileUsage::Unused,
                            }

                            let mut tiles : Vec<(usize, usize)> = vec![];
                            let mut i = tile_id.clone();

                            tiles.push(i);

                            // Collect all tiles in the selection
                            if let Some(selection_end) = context.selection_end {
                                if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                                    while i.0 != selection_end.0 || i.1 != selection_end.1 {
                                        i.0 += 1;
                                        if i.0 >= map.max_tiles_per_row() {
                                            i.0 = 0;
                                            i.1 += 1;
                                        }
                                        tiles.push(i);
                                    }
                                }
                            }

                            for id in &tiles {
                                let tid = TileId::new(asset.tileset.maps_ids[context.curr_tileset_index], id.0 as u16, id.1 as u16);
                                if let Some(tile) = &mut asset.get_mut_tile(&tid) {
                                    tile.usage = usage.clone();

                                    if let Some(map) = asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                                        map.save_settings();
                                    }
                                } else {
                                    let mut tile = Tile::new();
                                    tile.usage = usage.clone();
                                    if let Some(map) = asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                                        map.set_tile(*id, tile);
                                        map.save_settings();
                                    }
                                }
                            }
                        }

                        atom.clicked = false;
                    } else
                    if atom.atom_data.id == "Settings" {
                        self.set_tile_settings(true, asset, context);
                    } else
                    if atom.atom_data.id == "Set Anim" {
                        self.set_anim(asset, context);
                    } else
                    if atom.atom_data.id == "Clear Anim" {
                        self.clear_anim(asset, context);
                    } else
                    if atom.atom_data.id == "Set Default" {
                        self.set_default_tile(asset, context);
                    }
                }
                return true;
            }
        }
        false
    }

    fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        let mut consumed = false;

        for atom in &mut self.widgets {
            if atom.mouse_up(pos, asset, context) {
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        let mut consumed = false;

        for atom in &mut self.widgets {
            if atom.mouse_dragged(pos, asset, context) {
                consumed = true;
            }
        }
        consumed
    }

    fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
        for atom in &mut self.widgets {
            if atom.mouse_hover(pos, asset, context) {
                return true;
            }
        }
        false
    }

    /// Updates the group widget based on the selected tile
    fn adjust_tile_usage(&mut self, asset: &Asset, context: &ScreenContext) {
        if let Some(tile_id) = context.curr_tile {
            let tid = TileId::new(asset.tileset.maps_ids[context.curr_tileset_index], tile_id.0 as u16, tile_id.1 as u16);

            if let Some(tile) = asset.get_tile(&tid) {
                match tile.usage {
                    TileUsage::Unused => self.widgets[0].curr_item_index = 0,
                    TileUsage::Environment => self.widgets[0].curr_item_index = 1,
                    TileUsage::EnvRoad => self.widgets[0].curr_item_index = 2,
                    TileUsage::EnvBlocking => self.widgets[0].curr_item_index = 3,
                    TileUsage::Character => self.widgets[0].curr_item_index = 4,
                    TileUsage::UtilityChar => self.widgets[0].curr_item_index = 5,
                    TileUsage::Water => self.widgets[0].curr_item_index = 6,
                    TileUsage::Effect => self.widgets[0].curr_item_index = 7,
                    TileUsage::Icon => self.widgets[0].curr_item_index = 8,
                    TileUsage::UIElement => self.widgets[0].curr_item_index = 9,
                }
                //self.widgets[1].text[0] = tile.tags.clone();
            } else {
                self.widgets[0].curr_item_index = 0;
                // self.widgets[1].text[0] = "".to_string();
            }
        }
        self.widgets[0].dirty = true;
        // self.widgets[1].dirty = true;
    }

    /// Sets the tile anim for the current tile
    fn set_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {
        if let Some(selection) = context.curr_tile {
            if let Some(selection_end) = context.selection_end {
                if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {

                    let mut anim_tiles : Vec<(usize, usize)> = vec![];
                    let mut i = selection.clone();

                    anim_tiles.push(i);

                    while i.0 != selection_end.0 || i.1 != selection_end.1 {
                        i.0 += 1;
                        if i.0 >= map.max_tiles_per_row() {
                            i.0 = 0;
                            i.1 += 1;
                        }
                        anim_tiles.push(i);

                        if let Some(tile) = map.get_mut_tile(&i) {
                            tile.usage = TileUsage::Unused;
                        } else {
                            let mut tile = Tile::new();
                            tile.usage = TileUsage::Unused;
                            map.set_tile(i, tile);
                        }
                    }

                    if let Some(tile) = map.get_mut_tile(&selection) {
                        tile.anim_tiles = anim_tiles;
                        map.save_settings();
                    } else {
                        let mut tile = Tile::new();
                        tile.anim_tiles = anim_tiles;
                        map.set_tile(selection, tile);
                        map.save_settings();
                    }
                }
            }
        }
    }

    /// Clears the tile anim for the current tile
    fn clear_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {
        if let Some(selection) = context.curr_tile {
            if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                if let Some(tile) = map.get_mut_tile(&selection) {
                    tile.anim_tiles = vec![];
                    map.save_settings();
                } else {
                    let mut tile = Tile::new();
                    tile.anim_tiles = vec![];
                    map.set_tile(selection, tile);
                    map.save_settings();
                }
            }
        }
    }

    /// Sets the default tile for the current map
    fn set_default_tile(&mut self, asset: &mut Asset, context: &ScreenContext) {
        if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
            map.settings.default_tile = context.curr_tile;
            map.save_settings();
        }
    }

    /// Set the tags
    fn set_tile_settings(&mut self, open_editor: bool, asset: &mut Asset, context: &mut ScreenContext) {
        if open_editor == false && context.code_editor_is_active == false {
            return;
        }

        if let Some(selection) = context.curr_tile {
            if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                if let Some(tile) = map.get_mut_tile(&selection) {

                    let value;

                    if let Some(properties) = &tile.settings {
                        value = Value::String(properties.to_string(generate_tile_settings_sink_descriptions()));
                    } else {
                        let mut properties = PropertySink::new();
                        update_tile_settings_sink(&mut properties);
                        value = Value::String(properties.to_string(generate_tile_settings_sink_descriptions()));
                    }

                    //value = Value::String(region.data.settings.to_string(generate_region_sink_descriptions()));
                    let id = context.create_property_id("tile_settings");
                    context.code_editor_mode = CodeEditorMode::Settings;
                    context.open_code_editor(id, value, false);

                    // Clear the debug renderer in case the settings change
                    context.debug_render = None;
                }
            }
        }
    }

    /// Updates a value from the dialog
    fn update_from_dialog(&mut self, _id: (Uuid, Uuid, String), value: Value, asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) {
        if let Some(selection) = context.curr_tile {
            if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                if let Some(tile) = map.get_mut_tile(&selection) {
                    let mut properties = PropertySink::new();
                    properties.load_from_string(value.to_string_value());
                    if let Some(tags ) = properties.get("tags") {
                        tile.tags = tags.to_string();
                    }
                    tile.settings = Some(properties);
                    map.save_settings();
                }
            }
        }
    }
}