
pub mod tileset;
pub mod tilearea;

use rusttype::{Font};

use tileset::*;

use std::collections::HashMap;

use crate::tilearea::TileArea;

pub struct Asset<'a> {
    pub tileset                 : TileSet,
    //gohu_font_11              : Font<'a>,
    //gohu_font_14                : Font<'a>,
    pub open_sans               : Font<'a>,
    pub grid_size               : u32,
    pub areas                   : HashMap<String, TileArea>,
    pub curr_area               : String,

    pub width                   : u32,
    pub height                  : u32,
}

impl Asset<'_>  {

    pub fn new(width: u32, height: u32) -> Self where Self: Sized {

        // Create the tile areas
        let mut areas = HashMap::new();

        let world = TileArea::new("world".to_string());
        areas.insert("world".to_string(),world);

        Self {
            tileset         : tileset::TileSet::new(),
            //gohu_font_11    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-11.ttf") as &[u8]).expect("Error constructing Font"),
            //gohu_font_14    : Font::try_from_bytes(include_bytes!("../assets/fonts/Open_Sans/static/OpenSans/OpenSans-SemiBold.ttf") as &[u8]).expect("Error constructing Font"),
            open_sans       : Font::try_from_bytes(include_bytes!("../assets/fonts/Open_Sans/static/OpenSans/OpenSans-Regular.ttf") as &[u8]).expect("Error constructing Font"),
            grid_size       : 32,
            areas,
            curr_area       : "world".to_string(),

            width,
            height
        }
    }

    /// Returns the tilemap of the given id
    pub fn get_map_of_id(&self, id: u32) -> &TileMap {
        &self.tileset.maps[&id]
    }

    /// Draws the given tile with a given scale
    pub fn draw_tile(&self,  frame: &mut [u8], pos: &(u32, u32), tilemap_id: u32, grid_pos: &(u32, u32), scale: f32) {
        let map = self.get_map_of_id(tilemap_id);
        let pixels = &map.pixels;

        let new_size = ((map.settings.grid_size as f32 * scale) as u32, (map.settings.grid_size as f32 * scale) as u32);

        let g_pos = (grid_pos.0 * map.settings.grid_size, grid_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as u32;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as u32;

                let d = pos.0 as usize * 4 + (sx as usize) * 4 + (sy as usize + pos.1 as usize) * (self.width as usize) * 4;
                let s = (x as usize + g_pos.0 as usize) * 4 + (y as usize + g_pos.1 as usize) * (map.width as usize) * 4;

                frame[d..d + 4].copy_from_slice(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]]);
            }
        }
    }

    /// Draws the given animated tile
    pub fn draw_animated_tile(&self,  frame: &mut [u8], pos: &(u32, u32), tilemap_id: u32, grid_pos: &(u32, u32), anim_counter: u32, target_size: u32) {
        let map = self.get_map_of_id(tilemap_id);
        let pixels = &map.pixels;
        let scale = target_size as f32 / map.settings.grid_size as f32;

        let new_size = ((map.settings.grid_size as f32 * scale) as u32, (map.settings.grid_size as f32 * scale) as u32);

        let tile = map.get_tile(grid_pos);

        let mut cg_pos = grid_pos;

        if tile.anim_tiles.len() > 0 {
            let index = anim_counter % tile.anim_tiles.len() as u32;
            cg_pos = &tile.anim_tiles[index as usize];
        }

        let g_pos = (cg_pos.0 * map.settings.grid_size, cg_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as u32;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as u32;

                let d = pos.0 as usize * 4 + (sx as usize) * 4 + (sy as usize + pos.1 as usize) * (self.width as usize) * 4;
                let s = (x as usize + g_pos.0 as usize) * 4 + (y as usize + g_pos.1 as usize) * (map.width as usize) * 4;

                frame[d..d + 4].copy_from_slice(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]]);
            }
        }
    }

    /// Draws the given tile mixed with a given color
    pub fn draw_tile_mixed(&self,  frame: &mut [u8], pos: &(u32, u32), tilemap_id: u32, grid_pos: &(u32, u32), color: [u8; 4], scale: f32) {
        let map = self.get_map_of_id(tilemap_id);
        let pixels = &map.pixels;

        let new_size = ((map.settings.grid_size as f32 * scale) as u32, (map.settings.grid_size as f32 * scale) as u32);

        let g_pos = (grid_pos.0 * map.settings.grid_size, grid_pos.1 * map.settings.grid_size);

        for sy in 0..new_size.0 {
            let y = (sy as f32 / scale) as u32;
            for sx in 0..new_size.1 {

                let x = (sx as f32 / scale) as u32;

                let d = pos.0 as usize * 4 + (sx as usize) * 4 + (sy as usize + pos.1 as usize) * (self.width as usize) * 4;
                let s = (x as usize + g_pos.0 as usize) * 4 + (y as usize + g_pos.1 as usize) * (map.width as usize) * 4;

                let mixed_color = self.mix(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]], &color, 0.5);

                frame[d..d + 4].copy_from_slice(&mixed_color);
            }
        }
    }

    /// Draws the given tilemap
    // pub fn draw_texture(&self,  frame: &mut [u8], pos: &[usize; 2], map: &tileset::TileMap) {
    //     let pixels = &map.pixels;

    //     for y in 0..map.height {
    //         for x in 0..map.width {

    //             let d = (x as usize + pos[0] as usize) * 4 + (y as usize + pos[1] as usize) * (WIDTH as usize) * 4;
    //             let s = (x as usize) * 4 + (y as usize) * (map.width as usize) * 4;

    //             frame[d..d + 4].copy_from_slice(&[pixels[s], pixels[s+1], pixels[s+2], pixels[s+3]]);
    //         }
    //     }
    // }

    /// Mixes two colors based on v
    pub fn mix(&self, a: &[u8;4], b: &[u8;4], v: f32) -> [u8; 4] {
        [(((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
         (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
         (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
        255]
    }

    /// Returns the tile fo the given id
    pub fn get_tile(&self, id: &(u32, u32, u32)) -> Tile {
        let map = self.get_map_of_id(id.0);
        map.get_tile(&(id.1, id.2))
    }

    /// Sets a value in the current area
    pub fn save_area(&self) {
        let area = &mut self.areas.get(&self.curr_area).unwrap();
        area.save_data();
    }

    /// Sets a value in the current area
    pub fn set_area_value(&mut self, pos: (isize, isize), value: (u32, u32, u32, TileUsage)) {
        let area = &mut self.areas.get_mut(&self.curr_area).unwrap();
        area.set_value(pos, value);
    }

    /// Draw the given area
    pub fn draw_area(&self, frame: &mut [u8], rect: &(u32,u32,u32,u32), anim_counter: u32) {
        let area = self.areas.get(&self.curr_area).unwrap();

        let x_tiles = (rect.2 / self.grid_size) as isize;
        let y_tiles = (rect.3 / self.grid_size) as isize;

        for y in 0..y_tiles {
            for x in 0..x_tiles {
                if let Some(value) = area.get_value((x, y)) {
                    let pos = (rect.0 + (x as u32) * self.grid_size, rect.1 + (y as u32) * self.grid_size);
                    self.draw_animated_tile(frame, &pos, value.0, &(value.1, value.2), anim_counter,self.grid_size);
                }
            }
        }
    }
}
