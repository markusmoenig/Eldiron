use crate::atom::AtomData;
use server::asset::Asset;

use server::asset::tileset::TileUsage;

use crate::widget::atom::{AtomWidget, AtomWidgetType};
use crate::widget::context::ScreenContext;
use crate::widget::WidgetState;

use crate::editor::traits::{ EditorOptions, EditorContent };

pub struct TileMapOptions {
    rect                    : (usize, usize, usize, usize),
    widgets                 : Vec<AtomWidget>,
}

impl EditorOptions for TileMapOptions {

    fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self where Self: Sized {

        let mut widgets : Vec<AtomWidget> = vec![];

        let mut group_list = AtomWidget::new(vec![], AtomWidgetType::GroupedList,
    AtomData::new_as_int("GroupedList".to_string(), 0));

        group_list.state = WidgetState::Disabled;

        group_list.add_group_list(context.color_blue, context.color_light_blue, vec!["Unused".to_string(), "Environment".to_string(), "Road".to_string(), "Blocking".to_string(), "Character".to_string(), "Utility".to_string(), "Water".to_string(), "Effect".to_string(), "Icon".to_string(), "UI Element".to_string()]);
        group_list.set_rect(rect, asset, context);
        widgets.push(group_list);

        let mut tags_button = AtomWidget::new(vec!["".to_string()], AtomWidgetType::TagsButton,
            AtomData::new_as_int("Tags".to_string(), 0));
        tags_button.state = WidgetState::Disabled;
        tags_button.set_rect((rect.0 + 10, rect.1 + 310 + 30, rect.2 - 20, 40), asset, context);
        widgets.push(tags_button);

        let mut set_anim_button = AtomWidget::new(vec!["Set Anim".to_string()], AtomWidgetType::Button,
            AtomData::new_as_int("Set Anim".to_string(), 0));
        set_anim_button.state = WidgetState::Disabled;
        set_anim_button.set_rect((rect.0 + 10, rect.1 + 305 + 80, rect.2 - 20, 40), asset, context);
        widgets.push(set_anim_button);

        let mut clear_anim_button = AtomWidget::new(vec!["Clear Anim".to_string()], AtomWidgetType::Button,
        AtomData::new_as_int("Clear Anim".to_string(), 0));
        clear_anim_button.state = WidgetState::Disabled;
        clear_anim_button.set_rect((rect.0 + 10, rect.1 + 340 + 80, rect.2 - 20, 40), asset, context);
        widgets.push(clear_anim_button);

        let mut set_default_button = AtomWidget::new(vec!["Set Default".to_string()], AtomWidgetType::Button,
        AtomData::new_as_int("Set Default".to_string(), 0));
        set_default_button.state = WidgetState::Disabled;
        set_default_button.set_rect((rect.0 + 10, rect.1 + 15 + 370 + 80, rect.2 - 20, 40), asset, context);
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
            context.draw2d.draw_animated_tile(frame, &((self.rect.2 - 80) / 2, self.rect.1 + self.rect.3 - 102), asset.get_map_of_id(asset.tileset.maps_ids[context.curr_tileset_index]), context.width, &grid_pos, anim_counter, 80);

            context.draw2d.draw_text_rect(frame, &(0, self.rect.1 + self.rect.3 - 22, self.rect.2, 20), context.width, &asset.get_editor_font("OpenSans"), 15.0, &format!("({}, {})", grid_pos.0, grid_pos.1), &context.color_white, &[0,0,0,255], crate::draw2d::TextAlignment::Center);
        }
    }

    // Sets the state of the widgets
    fn set_state(&mut self, state: WidgetState) {
        for a in &mut self.widgets {
            a.state = state.clone();
            a.dirty = true;
        }
    }

    fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext, _content: &mut Option<Box<dyn EditorContent>>) -> bool {
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
                                let mut tile = asset.get_tile(&(asset.tileset.maps_ids[context.curr_tileset_index], id.0, id.1));
                                tile.usage = usage.clone();

                                if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                                    map.set_tile(*id, tile);
                                    map.save_settings();
                                }
                            }
                        }

                        atom.clicked = false;
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
            let tile = asset.get_tile(&(asset.tileset.maps_ids[context.curr_tileset_index], tile_id.0, tile_id.1));
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
            self.widgets[1].text[0] = tile.tags;
        } else {
            self.widgets[0].curr_item_index = 0;
            self.widgets[1].text[0] = "".to_string();
        }
        self.widgets[0].dirty = true;
        self.widgets[1].dirty = true;
    }

    /// Sets the tile anim for the current tile
    fn set_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {
        if let Some(selection) = context.curr_tile {
            if let Some(selection_end) = context.selection_end {
                if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                    let mut tile = map.get_tile(&selection);

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

                        let mut tile = map.get_tile(&i);
                        tile.usage = TileUsage::Unused;
                        map.set_tile(i, tile);
                    }

                    tile.anim_tiles = anim_tiles;

                    map.set_tile(selection, tile);
                    map.save_settings();
                }
            }
        }
    }

    /// Clears the tile anim for the current tile
    fn clear_anim(&mut self, asset: &mut Asset, context: &ScreenContext) {
        if let Some(selection) = context.curr_tile {
            if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
                let mut tile = map.get_tile(&selection);

                tile.anim_tiles = vec![];

                map.set_tile(selection, tile);
                map.save_settings();
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
    fn set_tags(&mut self, tags: String, asset: &mut Asset, context: &ScreenContext) {
        let mut tiles : Vec<(usize, usize)> = vec![];

        if let Some(tile_id) = context.curr_tile {
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
        }

        self.widgets[1].text[0] = tags.clone().to_lowercase();
        self.widgets[1].dirty = true;

        if let Some(map)= asset.tileset.maps.get_mut(&asset.tileset.maps_ids[context.curr_tileset_index]) {
            for tiles in &tiles {
                let mut tile = map.get_tile(&tiles);
                tile.tags = tags.clone().to_lowercase();
                map.set_tile(*tiles, tile);
            }
            map.save_settings();
        }
    }
}