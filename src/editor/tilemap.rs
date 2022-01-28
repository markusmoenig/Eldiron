

use crate::prelude::*;

use crate::widget::*;

use crate::tab::TabWidget;
use crate::asset::Asset;

use core::cmp::max;
use core::cell::Cell;

pub struct TileMapEditor {
    rect                    : (u32, u32, u32, u32),
    selected                : Cell<Option<(u32, u32)>>,
    curr_grid_size          : Cell<u32>,  
    tab_widget              : TabWidget,//Box<dyn Widget>
}

impl Widget for TileMapEditor {
    
    fn new(rect: (u32, u32, u32, u32)) -> Self where Self: Sized {

        Self {
            rect,
            selected        : Cell::new(None),
            curr_grid_size  : Cell::new(0),
            tab_widget      : TabWidget::new((0, UI_ELEMENT_HEIGHT, WIDTH, HEIGHT / 2 - UI_ELEMENT_HEIGHT))
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

        let total_tiles = x_tiles * y_tiles;
        //let total_tiles_scaled = ((total_tiles) as f32 * scale) as u32;

        let screen_x = WIDTH / scaled_grid_size;
        let screen_y = (self.tab_widget.get_rect().3 - UI_ELEMENT_HEIGHT) / scaled_grid_size;

        let tiles_per_page = screen_x * screen_y;

        //println!("{} {}", total_tiles, total_tiles_scaled);
        let pages = max( (total_tiles as f32 / tiles_per_page as f32).ceil() as u32, 1);

        //println!("{}", pages);

        self.tab_widget.set_pagination(pages);

        let page = self.tab_widget.curr_page.get();

        let mut x_off = 0_u32;
        let mut y_off = 0_u32;

        let offset = page * tiles_per_page;

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
        if let Some(s) = self.selected.get() {
            
            let index = s.0 + s.1 * screen_x;

            // Make sure the selected tile is in the current page
            if index >= offset && index < offset + tiles_per_page {
                let x = s.0 * scaled_grid_size;
                let y = s.1 * scaled_grid_size;

                asset.draw_rect_outline(frame, &(x, y + UI_ELEMENT_HEIGHT, scaled_grid_size, scaled_grid_size), self.get_color_text());
            }
        }

        // Toolbar
        asset.draw_rect(frame, &(0, 0, WIDTH, UI_ELEMENT_HEIGHT), self.get_color_background());

        self.curr_grid_size.set(scaled_grid_size);
    }

    fn mouse_down(&self, pos: (u32, u32)) -> bool {
        let mut consumed = false;

        // Pages
        if self.tab_widget.mouse_down(pos) {
            consumed = true;
        }

        // Tile content area
        if consumed == false {
            if self.tab_widget.contains_pos_for(pos, self.tab_widget.get_content_rect()) {

                let scaled_grid_size = self.curr_grid_size.get();

                let x = pos.0 / scaled_grid_size;
                let y = (pos.1 - self.tab_widget.get_rect().1) / scaled_grid_size;

                println!("{} {}", x, y);

                self.selected.set(Some((x, y)));
            }
        }

        consumed
    }

    fn mouse_up(&self, _pos: (u32, u32)) {
        //println!("text {:?}", pos);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}