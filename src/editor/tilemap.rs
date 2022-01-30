

use crate::prelude::*;

use crate::widget::*;

use crate::tab::TabWidget;
use crate::button::ButtonWidget;
use crate::optionsgrid::OptionsGridWidget;
use crate::asset::Asset;
use crate::asset::tileset::TileUsage;

use core::cmp::max;
use core::cell::Cell;

pub struct TileMapEditor {
    rect                    : (u32, u32, u32, u32),
    screen_selected         : Cell<Option<(u32, u32)>>,
    map_selected            : Cell<Option<(u32, u32)>>,
    screen_start            : Cell<(u32, u32)>,
    curr_grid_size          : Cell<u32>,  
    curr_map_tiles          : Cell<(u32, u32)>,  
    tab_widget              : TabWidget,
    //add_tiles_button        : ButtonWidget,
    options_grid            : OptionsGridWidget,
    button_widgets          : Vec<ButtonWidget>,
}

impl Widget for TileMapEditor {
    
    fn new(_text: Vec<String>, rect: (u32, u32, u32, u32)) -> Self where Self: Sized {

        let add_tiles_button = ButtonWidget::new(vec!["Add Tile(s)".to_string()], (20, HEIGHT / 2 + 10, 120,  UI_ELEMENT_HEIGHT));

        Self {
            rect,
            screen_selected         : Cell::new(None),
            map_selected            : Cell::new(None),
            screen_start            : Cell::new((0, 0)),
            curr_grid_size          : Cell::new(0),
            curr_map_tiles          : Cell::new((0,0)),
            tab_widget              : TabWidget::new(vec!(),(0, UI_ELEMENT_HEIGHT, WIDTH, HEIGHT / 2 - UI_ELEMENT_HEIGHT)),
            button_widgets          : vec![add_tiles_button],
            options_grid            : OptionsGridWidget::new(vec!["Unused".to_string(), "Environment".to_string(), "EnvBlocking".to_string(), "Character".to_string(), "Water".to_string(), "Harmful".to_string()], 
            (10, HEIGHT / 2 + 10, WIDTH - 20, HEIGHT / 2 - 20))
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], asset: &Asset) {

        asset.draw_rect(frame, &self.tab_widget.get_content_rect(), [0,0,0,255]);

        let scale = 2_f32;
        let map = asset.get_map_of_id(0);

        let scaled_grid_size = (map.settings.grid_size as f32 * scale) as u32;

        let x_tiles = map.width / map.settings.grid_size;
        let y_tiles = map.height / map.settings.grid_size;

        self.curr_map_tiles.set((x_tiles, y_tiles));

        let total_tiles = x_tiles * y_tiles;
        //let total_tiles_scaled = ((total_tiles) as f32 * scale) as u32;

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

        //println!("start {:?}", self.screen_start.get());

        // Draw the tiles of the current page
        for tile in 0..tiles_per_page {

            if tile + offset >= total_tiles {
                break;
            }

            let x_step = (x_off as f32 * map.settings.grid_size as f32 * scale) as u32;
            let y_step = (y_off as f32 * map.settings.grid_size as f32 * scale) as u32;

            let x = (tile+offset) % x_tiles;
            let y = (tile+offset) / x_tiles;

            asset.draw_tile(frame, &(x_step, y_step + self.tab_widget.get_rect().1), 0_u32, &(x, y), scale);
            x_off += 1;

            if x_off >= screen_x {
                x_off = 0;
                y_off += 1;
                if y_off >= screen_y {
                    break;
                }
            }
        }

        // Draw the tab widget
        self.tab_widget.draw(frame, asset);

        // Draw the selection
        if let Some(s) = self.screen_selected.get() {
            
            let index = s.0 + s.1 * screen_x;

            // Make sure the selected tile is in the current page
            if index >= offset && index < offset + tiles_per_page {
                let x = (s.0 - self.screen_start.get().0) * scaled_grid_size;
                let y = (s.1 - self.screen_start.get().1) * scaled_grid_size;

                asset.draw_rect_outline(frame, &(x, y + UI_ELEMENT_HEIGHT, scaled_grid_size, scaled_grid_size), self.get_color_text());
            }

            self.options_grid.set_state(1);

            self.button_widgets[0].set_state(1);
        } else {
            self.button_widgets[0].set_state(0);
            self.options_grid.set_state(0);
        }

        // Draw the lower half

        //for b in &self.button_widgets {
        //    b.draw(frame, asset);
        //}

        self.options_grid.draw(frame, asset);

        // Toolbar
        asset.draw_rect(frame, &(0, 0, WIDTH, UI_ELEMENT_HEIGHT), self.get_color_background());

        self.curr_grid_size.set(scaled_grid_size);
    }

    fn mouse_down(&self, pos: (u32, u32), asset: &mut Asset) -> bool {
        let mut consumed = false;

        // Pages
        if self.tab_widget.mouse_down(pos, asset) {
            consumed = true;
        }

        // Upper tile content area
        if consumed == false {
            if self.tab_widget.contains_pos_for(pos, self.tab_widget.get_content_rect()) {

                let scaled_grid_size = self.curr_grid_size.get();

                let x = pos.0 / scaled_grid_size + self.screen_start.get().0;
                let y = (pos.1 - self.tab_widget.get_rect().1) / scaled_grid_size + self.screen_start.get().1;

                // convert screen position to map position

                let screen_tiles_x = WIDTH / scaled_grid_size;
                let tile_offset = x + y * screen_tiles_x;
                
                let map_tiles =  self.curr_map_tiles.get();

                let total_tiles = map_tiles.0 * map_tiles.1;

                if tile_offset < total_tiles {
                    self.screen_selected.set(Some((x, y)));

                    //println!("selected {} {}", x, y);

                    // Select the right option
                    let map = asset.get_map_of_id(0);

                    let map_x = tile_offset % map_tiles.0; 
                    let map_y = tile_offset / map_tiles.0; 

                    self.map_selected.set(Some((map_x, map_y)));

                    let tile = map.get_tile((map_x, map_y));

                    if tile.usage == TileUsage::Unused {

                    }

                } else {
                    self.screen_selected.set(None);
                    self.map_selected.set(None);
                }

                consumed = true
            }
        }

        if consumed == false {
            if self.options_grid.mouse_down(pos, asset) {
                consumed =true;

                if self.options_grid.clicked.get() == true {
                    let index = self.options_grid.selected_index.get();

                    let map = &mut asset.tileset.maps.get_mut(&0).unwrap();        

                    let mut tile = map.get_tile(self.map_selected.get().unwrap());

                    if index == 0 {
                        tile.usage = TileUsage::Unused;
                    }

                    map.set_tile(self.map_selected.get().unwrap(), tile);

                     println!("option {}", index);
                }
                //     // Add tiles
                //     println!("{}", "option");
                //     self.button_widgets[0].clicked.set(false);
                // }

                // if self.button_widgets[0].clicked.get() == true {
                //     // Add tiles
                //     println!("{}", "option");
                //     self.button_widgets[0].clicked.set(false);
                // }
            }        
        }


        /*
        if consumed == false {
            for b in &self.button_widgets {
                if b.mouse_down(pos) {
                    consumed =true;
                    if self.button_widgets[0].clicked.get() == true {
                        // Add tiles
                        println!("{}", "add tiles");
                        self.button_widgets[0].clicked.set(false);
                    }
                }
            }
        }*/

        consumed
    }

    fn mouse_up(&self, pos: (u32, u32), asset: &Asset) -> bool {
        let mut consumed = false;
        for b in &self.button_widgets {
            if b.mouse_up(pos, asset) {
                consumed = true
            }
        }
        consumed
    }

    fn mouse_dragged(&self, pos: (u32, u32), _asset: &Asset) {
        println!("dragged {:?}", pos);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}