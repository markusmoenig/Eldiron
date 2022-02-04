use crate::prelude::*;

use crate::widget::*;

use crate::tab::TabWidget;
use crate::button::ButtonWidget;
use crate::optionsgrid::OptionsGridWidget;
use crate::asset::Asset;
use crate::asset::tileset::TileUsage;
use crate::menu::MenuWidget;
use crate::intslider::IntSliderWidget;

use crate::asset::tileset::TileMap;

use core::cmp::max;
use core::cell::Cell;

pub struct TileMapEditor {
    rect                    : (u32, u32, u32, u32),
    screen_selected         : Cell<Option<(u32, u32)>>,
    screen_end_selected     : Cell<Option<(u32, u32)>>,
    map_selected            : Cell<Option<(u32, u32)>>,
    screen_start            : Cell<(u32, u32)>,
    curr_grid_size          : Cell<u32>,  
    curr_map_tiles          : Cell<(u32, u32)>,  
    tab_widget              : TabWidget,
    options_grid            : OptionsGridWidget,
    set_anim_button         : ButtonWidget,
    clear_anim_button       : ButtonWidget,
    tilemap_menu            : MenuWidget,
    scale_button            : IntSliderWidget,
    scale                   : Cell<f32>,
}

impl Widget for TileMapEditor {
    
    fn new(_text: Vec<String>, rect: (u32, u32, u32, u32), asset: &Asset) -> Self where Self: Sized {

        let set_anim_button = ButtonWidget::new(vec!["Set Anim".to_string()], (20 + 100 + 40, HEIGHT / 2 + 96, 120,  UI_ELEMENT_HEIGHT), asset);
        let clear_anim_button = ButtonWidget::new(vec!["Clear Anim".to_string()], (20 + 100 + 40 + 120 + 8, HEIGHT / 2 + 96, 120,  UI_ELEMENT_HEIGHT), asset);

        let scale_button = IntSliderWidget::new(vec!["Scale".to_string()], (WIDTH - 240 - 20, 0, 120,  UI_ELEMENT_HEIGHT), asset);
        scale_button.set_range((1.0, 4.0));
        scale_button.set_value(2.0);

        let mut names : Vec<String> = vec![];
        for tm in &asset.tileset.maps_names {
            names.push(tm.to_string());
        }

        let tilemap_menu = MenuWidget::new(names, (WIDTH - 120 - 10, 0, 120,  UI_ELEMENT_HEIGHT), asset);

        Self {
            rect,
            screen_selected         : Cell::new(None),
            screen_end_selected     : Cell::new(None),
            map_selected            : Cell::new(None),
            screen_start            : Cell::new((0, 0)),
            curr_grid_size          : Cell::new(0),
            curr_map_tiles          : Cell::new((0,0)),
            tab_widget              : TabWidget::new(vec!(),(0, UI_ELEMENT_HEIGHT, WIDTH, HEIGHT / 2 - UI_ELEMENT_HEIGHT), asset),
            set_anim_button,
            clear_anim_button,
            tilemap_menu,
            scale_button,
            options_grid            : OptionsGridWidget::new(vec!["Unused".to_string(), "Environment".to_string(), "EnvBlocking".to_string(), "Character".to_string(), "UtilityChar".to_string(), "Water".to_string(), "Harmful".to_string()], 
            (20 + 100 + 40, HEIGHT / 2 + 20, WIDTH - 40 - 100 - 40, 2 * UI_ELEMENT_HEIGHT + 16), asset),
            scale                   : Cell::new(2_f32)
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], anim_counter: u32, asset: &mut Asset) {

        let scale = self.scale.get();
    
        let curr_map_id = self.tilemap_menu.selected_index.get();
        let map = asset.get_map_of_id(curr_map_id);

        let scaled_grid_size = (map.settings.grid_size as f32 * scale) as u32;

        let x_tiles = map.width / map.settings.grid_size;
        let y_tiles = map.height / map.settings.grid_size;

        self.curr_map_tiles.set((x_tiles, y_tiles));

        let total_tiles = x_tiles * y_tiles;

        let screen_x = WIDTH / scaled_grid_size;
        let screen_y = (self.tab_widget.get_rect().3 - UI_ELEMENT_HEIGHT) / scaled_grid_size;

        let tiles_per_page = screen_x * screen_y;
        let pages = max( (total_tiles as f32 / tiles_per_page as f32).ceil() as u32, 1);

        //println!("{}", pages);

        self.tab_widget.set_pagination(pages);

        let page = self.tab_widget.curr_page.get();

        let mut x_off = 0_u32;
        let mut y_off = 0_u32;

        let offset = page * tiles_per_page;

        self.screen_start.set((offset % x_tiles, offset / screen_x));

        // Draw the tiles of the current page
        for tile in 0..tiles_per_page {

            if tile + offset >= total_tiles {
                break;
            }

            let x_step = (x_off as f32 * map.settings.grid_size as f32 * scale) as u32;
            let y_step = (y_off as f32 * map.settings.grid_size as f32 * scale) as u32;

            let x = (tile+offset) % x_tiles;
            let y = (tile+offset) / x_tiles;

            let tile = map.get_tile((x, y));

            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len() as u32;

                let p = tile.anim_tiles[index as usize];
                asset.draw_tile(frame, &(x_step, y_step + self.tab_widget.get_rect().1), curr_map_id, &(p.0, p.1), scale);
            } else
            if tile.usage == TileUsage::Unused {
                asset.draw_tile_mixed(frame, &(x_step, y_step + self.tab_widget.get_rect().1), curr_map_id, &(x, y), [128, 128, 128, 255], scale);
            } else {
                asset.draw_tile(frame, &(x_step, y_step + self.tab_widget.get_rect().1), curr_map_id, &(x, y), scale);
            }

            x_off += 1;

            if x_off >= screen_x {
                x_off = 0;
                y_off += 1;
                if y_off >= screen_y {
                    break;
                }
            }
        }

        // Returns the selected range between the start and end selection points
        fn get_selected_range(start: Option<(u32, u32)>, end: Option<(u32, u32)>, screen_x: u32) -> Vec<(u32, u32)> {
            let mut v = Vec::<(u32, u32)>::new();
            //println!("get_selected_range {:?} {:?}", start, end );

            if let Some(s) = start {
    
                if let Some(e) = end {
    
                    let mut smaller = s;
                    let mut bigger = e;
    
                    if smaller.1 > bigger.1 || (smaller.1 == bigger.1 && smaller.0 > bigger.0) {
                        let t = smaller;
                        smaller = bigger;
                        bigger = t;
                    }

                    // Iterate between the two selection points
                    loop {
                        v.push(smaller);
                        if smaller.0 == bigger.0 && smaller.1 == bigger.1 {
                            break;
                        }

                        smaller.0 += 1;

                        if smaller.0 >= screen_x {
                            smaller.0 = 0;
                            smaller.1 += 1;
                        }
                    }
    
                } else {
                    v.push(s);
                }
            }
            v
        }

        // Draw the selection
        if let Some(_s) = self.screen_selected.get() {
            
            let range = get_selected_range(self.screen_selected.get(), self.screen_end_selected.get(), screen_x);

            if range.len() > 1 {
                self.set_anim_button.set_state(1);
            } else {
                self.set_anim_button.set_state(0);
            }

            self.clear_anim_button.set_state(1);

            for s in range {
                let index = s.0 + s.1 * screen_x;

                // Make sure the selected tile is in the current page
                if index >= offset && index < offset + tiles_per_page {
                    let x = (s.0 - self.screen_start.get().0) * scaled_grid_size;
                    let y = (s.1 - self.screen_start.get().1) * scaled_grid_size;

                    asset.draw_rect_outline(frame, &(x, y + UI_ELEMENT_HEIGHT, scaled_grid_size, scaled_grid_size), self.get_color_text());
                }
            }

            self.options_grid.set_state(1);

            if let Some(map_selected) = &self.map_selected.get() {
                // Draw selected tile as 100x100

                let tile = map.get_tile(*map_selected);

                if tile.anim_tiles.len() > 0 {
                    let index = anim_counter % tile.anim_tiles.len() as u32;
                    let p = tile.anim_tiles[index as usize];
                    asset.draw_tile(frame, &(20, HEIGHT / 2 + 20), curr_map_id, &(p.0, p.1), 100.0 / map.settings.grid_size as f32);
                } else {
                    asset.draw_tile(frame, &(20, HEIGHT / 2 + 20), curr_map_id, map_selected, 100.0 / map.settings.grid_size as f32);
                }
                // Draw selection text
                asset.draw_text_rect(frame, &(20, HEIGHT / 2 + 125, 100, UI_ELEMENT_HEIGHT), &format!("({},{})", map_selected.0, map_selected.1), self.get_color_text(), [0,0,0,255], crate::asset::TextAlignment::Center);
            }
        } else {
            asset.draw_rect(frame, &(20, HEIGHT / 2 + 20, 100, 100), self.get_color_background());

            self.set_anim_button.set_state(0);
            self.clear_anim_button.set_state(0);
            self.options_grid.set_state(0);
        }

        // Draw the tab widget
        self.tab_widget.draw(frame, anim_counter, asset);

        // Draw the lower half
        self.options_grid.draw(frame, anim_counter, asset);
        self.set_anim_button.draw(frame, anim_counter, asset);
        self.clear_anim_button.draw(frame, anim_counter, asset);

        // Toolbar
        self.tilemap_menu.draw(frame, anim_counter, asset);
        self.scale_button.draw(frame, anim_counter, asset);

        self.curr_grid_size.set(scaled_grid_size);
    }

    fn mouse_down(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;

        let curr_map_id = self.tilemap_menu.selected_index.get();

        // Convert a screen position to the map position
        let screen_to_map = |map: &TileMap, screen_pos: (u32, u32)| -> (u32, u32) {
            let scale = self.scale.get();
            let scaled_grid_size = (map.settings.grid_size as f32 * scale) as u32;        
            let screen_x = WIDTH / scaled_grid_size;

            let tile_offset = screen_pos.0 + screen_pos.1 * screen_x;                
            let map_tiles = self.curr_map_tiles.get();

            (tile_offset % map_tiles.0, tile_offset / map_tiles.0)
        };

        // Pages
        let curr_page = self.tab_widget.curr_page.get();
        if self.tab_widget.mouse_down(pos, asset) {
            consumed = true;
            // When a new page is displayed clear the selection
            if curr_page != self.tab_widget.curr_page.get() {
                self.screen_selected.set(None);
                self.screen_end_selected.set(None);
            }
        }

        // Upper tile content area
        if consumed == false {
            if self.tab_widget.contains_pos_for(pos, self.tab_widget.get_content_rect()) {

                // On mouse down set the end selection point to None
                self.screen_end_selected.set(None);

                let scaled_grid_size = self.curr_grid_size.get();

                let x = pos.0 / scaled_grid_size + self.screen_start.get().0;
                let y = (pos.1 - self.tab_widget.get_rect().1) / scaled_grid_size + self.screen_start.get().1;

                // convert screen position to map position

                let screen_tiles_x = WIDTH / scaled_grid_size;
                let tile_offset = x + y * screen_tiles_x;
                
                let map_tiles = self.curr_map_tiles.get();

                let total_tiles = map_tiles.0 * map_tiles.1;

                if tile_offset < total_tiles {
                    self.screen_selected.set(Some((x, y)));

                    // Select the right option
                    let map = asset.get_map_of_id(curr_map_id);

                    let map_pos = screen_to_map(map, (x,y));

                    self.map_selected.set(Some(map_pos));

                    let tile = map.get_tile(map_pos);

                    if tile.usage == TileUsage::Unused {
                        self.options_grid.selected_index.set(0);
                    } else
                    if tile.usage == TileUsage::Environment {
                        self.options_grid.selected_index.set(1);
                    } else
                    if tile.usage == TileUsage::EnvBlocking {
                        self.options_grid.selected_index.set(2);
                    } else
                    if tile.usage == TileUsage::Character {
                        self.options_grid.selected_index.set(3);
                    } else
                    if tile.usage == TileUsage::UtilityChar {
                        self.options_grid.selected_index.set(4);
                    } else
                    if tile.usage == TileUsage::Water {
                        self.options_grid.selected_index.set(5);
                    } else   
                    if tile.usage == TileUsage::Harmful {
                        self.options_grid.selected_index.set(6);
                    }
                } else {
                    self.screen_selected.set(None);
                    self.map_selected.set(None);
                }

                consumed = true
            }
        }

        // Check options grid for click
        if consumed == false {
            if self.options_grid.mouse_down(pos, asset) {
                consumed =true;

                if self.options_grid.clicked.get() == true {
                    let index = self.options_grid.selected_index.get();

                    if let Some(map)= asset.tileset.maps.get_mut(&curr_map_id) {

                        let mut tile = map.get_tile(self.map_selected.get().unwrap());

                        if index == 0 {
                            tile.usage = TileUsage::Unused;
                        } else
                        if index == 1 {
                            tile.usage = TileUsage::Environment;
                        } else
                        if index == 2 {
                            tile.usage = TileUsage::EnvBlocking;
                        } else
                        if index == 3 {
                            tile.usage = TileUsage::Character;
                        } else
                        if index == 4 {
                            tile.usage = TileUsage::UtilityChar;
                        }  else
                        if index == 5 {
                            tile.usage = TileUsage::Water;
                        }  else 
                        if index == 6 {
                            tile.usage = TileUsage::Harmful;
                        }                                                                                                                

                        map.set_tile(self.map_selected.get().unwrap(), tile);
                        map.save_settings();
                    }
                    self.options_grid.clicked.set(false);
                }
            }        
        }

        // Check anim button
        if consumed == false && self.set_anim_button.mouse_down(pos, asset) {
            consumed =true;

            // Returns the selected range between the start and end selection points
            fn get_selected_range(start: Option<(u32, u32)>, end: Option<(u32, u32)>, screen_x: u32) -> Vec<(u32, u32)> {
                let mut v = Vec::<(u32, u32)>::new();
                //println!("get_selected_range {:?} {:?}", start, end );

                if let Some(s) = start {
        
                    if let Some(e) = end {
        
                        let mut smaller = s;
                        let mut bigger = e;
        
                        if smaller.1 > bigger.1 || (smaller.1 == bigger.1 && smaller.0 > bigger.0) {
                            let t = smaller;
                            smaller = bigger;
                            bigger = t;
                        }

                        // Iterate between the two selection points
                        loop {
                            v.push(smaller);
                            if smaller.0 == bigger.0 && smaller.1 == bigger.1 {
                                break;
                            }

                            smaller.0 += 1;

                            if smaller.0 >= screen_x {
                                smaller.0 = 0;
                                smaller.1 += 1;
                            }
                        }
        
                    } else {
                        v.push(s);
                    }
                }
                v
            } 

            let scale = self.scale.get();
            if let Some(map)= asset.tileset.maps.get_mut(&curr_map_id) {
        
                let scaled_grid_size = (map.settings.grid_size as f32 * scale) as u32;        
                let screen_x = WIDTH / scaled_grid_size;

                if self.set_anim_button.clicked.get() == true {
                    if let Some(screen_start) = self.screen_selected.get() {
        
                        let start = screen_to_map(map, screen_start);
                        let range = get_selected_range(self.screen_selected.get(), self.screen_end_selected.get(), screen_x);

                        //println!("rr {:?}", start);

                        if range.len() > 1 {

                            let mut tile = map.get_tile(start);

                            tile.anim_tiles = vec![];

                            for s in range {
                                let map_pos = screen_to_map(map, s);
                                tile.anim_tiles.push(map_pos);

                                if map_pos.0 != start.0 || map_pos.1 != start.1 {
                                    let mut unused = map.get_tile(map_pos);
                                    unused.usage = TileUsage::Unused;
                                    map.set_tile(map_pos, unused);
                                }
                            }

                            map.set_tile(start, tile);
                            map.save_settings();                        
                        }
                    }
                    self.set_anim_button.clicked.set(false);
                }
            }
        }

        // Check clear anim button
        if consumed == false && self.clear_anim_button.mouse_down(pos, asset) {
            consumed =true;

            if self.clear_anim_button.clicked.get() == true {

                if let Some(map)= asset.tileset.maps.get_mut(&curr_map_id) {

                    if let Some(selected) = self.screen_selected.get() {
                        let map_pos = screen_to_map(map, selected);

                        //println!("clear {:?}", map_pos);

                        let mut tile = map.get_tile(map_pos);

                        tile.anim_tiles = vec![];

                        map.set_tile(map_pos, tile);
                        map.save_settings();                          
                    }
                }
                
                self.clear_anim_button.clicked.set(false);
            }
        }

        // Tilemap Menu
        if consumed == false && self.tilemap_menu.mouse_down(pos, asset) {
            consumed = true
        }

        if consumed == false && self.scale_button.mouse_down(pos, asset) {
            self.scale.set(self.scale_button.value.get() as f32);
            consumed = true
        }

        consumed
    }

    fn mouse_up(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;

        if self.set_anim_button.mouse_up(pos, asset) {
            consumed = true;
        }
        if self.clear_anim_button.mouse_up(pos, asset) {
            consumed = true;
        }      
        if self.tilemap_menu.mouse_up(pos, asset) {
            consumed = true;

            self.screen_selected.set(None);
            self.screen_end_selected.set(None);
        }           
        consumed
    }

    /// Set the screen_end_selected point
    fn mouse_dragged(&self, pos: (u32, u32), asset: &mut Asset) -> bool {

        if self.tilemap_menu.mouse_dragged(pos, asset) {
            return true
        }

        if self.scale_button.mouse_down(pos, asset) {
            self.scale.set(self.scale_button.value.get() as f32);
            return true;
        }

        if self.tab_widget.contains_pos_for(pos, self.tab_widget.get_content_rect()) {
            if let Some(selected) = self.screen_selected.get() {
                let scaled_grid_size = self.curr_grid_size.get();

                let x = pos.0 / scaled_grid_size + self.screen_start.get().0;
                let y = (pos.1 - self.tab_widget.get_rect().1) / scaled_grid_size + self.screen_start.get().1;

                let screen_tiles_x = WIDTH / scaled_grid_size;
                let tile_offset = x + y * screen_tiles_x;
                
                let map_tiles = self.curr_map_tiles.get();

                let total_tiles = map_tiles.0 * map_tiles.1;

                if tile_offset < total_tiles {
                    if y > selected.1 || (selected.1 == y && x > selected.0) {

                        self.screen_end_selected.set(Some((x, y)));
                        return true;
                    }
                }
            }
        }
        false
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}