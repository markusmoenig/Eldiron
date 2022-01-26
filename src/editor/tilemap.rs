

use crate::prelude::*;

use crate::widget::*;

use crate::tab::TabWidget;
use crate::asset::Asset;

pub struct TileMapEditor {
    title           : String,
    rect            : (u32, u32, u32, u32),
    tab_widget      : Box<dyn Widget>
}

impl Widget for TileMapEditor {
    
    fn new(title: String, rect: (u32, u32, u32, u32)) -> Self where Self: Sized {

        Self {
            title           : title,
            rect,
            tab_widget      : Box::new(TabWidget::new("TabWidget".to_string(), (0,0, WIDTH, HEIGHT / 2)))
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], asset: &Asset) {

        self.tab_widget.set_pagination(2);

        let scale = 2_f32;
        let map = asset.get_map_of_id(0);

        let x_tiles = map.width / map.settings.grid_size;
        let y_tiles = map.height / map.settings.grid_size;

        //let total_tiles = x_tiles * y_tiles;

        let screen_x = WIDTH / (map.settings.grid_size as f32 * scale) as u32;
        let screen_y = self.tab_widget.get_content_rect().3 / (map.settings.grid_size as f32 * scale) as u32;

        let mut x_off = 0_u32;
        let mut y_off = 0_u32;
        //println!("{}", c);

        for y in 0..y_tiles {
            for x in 0..x_tiles {

                let x_step = (x_off as f32 * map.settings.grid_size as f32 * scale) as u32;
                let y_step = (y_off as f32 * map.settings.grid_size as f32 * scale) as u32;

                asset.draw_tile(frame, &(x_step, y_step), 0_u32, &(x, y), scale);
                x_off += 1;

                if x_off >= screen_x {
                    x_off = 0;
                    y_off += 1;
                    if y_off >= screen_y {
                        break;
                    }
                }
            }
            if y_off >= screen_y {
                break;
            }            
        }

        self.tab_widget.draw(frame, asset);
    }

    fn mouse_down(&self, pos: (u32, u32)) {
        println!("mouse down text {:?}", pos);
    }

    fn mouse_up(&self, pos: (u32, u32)) {
        println!("text {:?}", pos);
    }

    fn get_rect(&self) -> &(u32, u32, u32, u32) {
        return &self.rect;
    }
}