
use std::time::{SystemTime, UNIX_EPOCH};

use crate::prelude::*;

use crate::widget::*;
use crate::asset::Asset;

//use rusttype::{point, Font, Scale};

pub struct TileMapEditor {
    title           : String,
    rect            : (u32, u32, u32, u32)
}

impl Widget for TileMapEditor {
    
    fn new(title: String, rect: (u32, u32, u32, u32)) -> Self where Self: Sized {
        Self {
            title   : title,
            rect
        }
    }

    /// Update the editor
    fn update(&mut self) {
    }

    fn draw(&self, frame: &mut [u8], asset: &Asset) {

        let scale = 2_u32;
        let map = asset.get_map_of_id(0);

        let x_tiles = map.width / map.settings.grid_size;
        let y_tiles = map.height / map.settings.grid_size;

        //let total_tiles = x_tiles * y_tiles;

        let screen_x = WIDTH / (map.settings.grid_size * scale);
        let screen_y = HEIGHT / (map.settings.grid_size * scale);

        let mut x_off = 0_u32;
        let mut y_off = 0_u32;
        //println!("{}", c);

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                asset.draw_tile(frame, &(x_off * map.settings.grid_size * scale, y_off * map.settings.grid_size * scale), 0_u32, &(x, y), scale);
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