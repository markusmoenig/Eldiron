
pub mod tileset;
pub mod tilearea;

use rusttype::{point, Font, Scale};

use crate::prelude::*;
use tileset::*;

use std::collections::HashMap;

use crate::tilearea::TileArea;

#[derive(PartialEq)]
pub enum TextAlignment {
    Left,
    Center,
    //Right
}

pub struct Asset<'a> {
    pub tileset                 : TileSet,
    //gohu_font_11              : Font<'a>,
    gohu_font_14                : Font<'a>,
    pub grid_size               : u32,
    pub areas                   : HashMap<String, TileArea>,
    pub curr_area               : String
}

impl Asset<'_>  {
    
    pub fn new() -> Self where Self: Sized {

        // Create the tile areas
        let mut areas = HashMap::new();

        let world = TileArea::new();
        areas.insert("world".to_string(),world);

        Self {
            tileset         : tileset::TileSet::new(),
            //gohu_font_11    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-11.ttf") as &[u8]).expect("Error constructing Font"),
            gohu_font_14    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-14.ttf") as &[u8]).expect("Error constructing Font"),
            grid_size       : 32,
            areas,
            curr_area       : "world".to_string()
        }
    }

    /// Returns the tilemap of the given id
    pub fn get_map_of_id(&self, id: u32) -> &TileMap {
        &self.tileset.maps[&id]
    }

    /// Draws the given rectangle
    pub fn draw_rect(&self, frame: &mut [u8], rect: &(u32, u32, u32, u32), color: [u8; 4]) {
        for y in rect.1..rect.1+rect.3 {
            for x in rect.0..rect.0+rect.2 {

                let i = x as usize * 4 + y as usize * (WIDTH as usize) * 4;

                frame[i..i + 4].copy_from_slice(&color);
            }
        }
    }

    /// Draws the outline of a given rectangle
    pub fn draw_rect_outline(&self, frame: &mut [u8], rect: &(u32, u32, u32, u32), color: [u8; 4]) {

        let y = rect.1;
        for x in rect.0..rect.0+rect.2 {
            let mut i = x as usize * 4 + y as usize * (WIDTH as usize) * 4;
            frame[i..i + 4].copy_from_slice(&color);

            i = x as usize * 4 + (y + rect.3- 1) as usize * (WIDTH as usize) * 4;
            frame[i..i + 4].copy_from_slice(&color);
        }

        let x = rect.0;
        for y in rect.1..rect.1+rect.3 {
            let mut i = x as usize * 4 + y as usize * (WIDTH as usize) * 4;
            frame[i..i + 4].copy_from_slice(&color);

            i = (x + rect.2 - 1) as usize * 4 + y as usize * (WIDTH as usize) * 4;
            frame[i..i + 4].copy_from_slice(&color);
        }
    }

    /// Draws a rect with a text
    pub fn draw_text_rect(&self, frame: &mut [u8], rect: &(u32, u32, u32, u32), text: &str, color: [u8; 4], background: [u8;4], align: TextAlignment) {
        self.draw_rect(frame, rect, background);
        if align == TextAlignment::Left {
            self.draw_text(frame, &(rect.0 + UI_ELEMENT_MARGIN, rect.1 + UI_ELEMENT_MARGIN), text, color, background);
        } else
        if align == TextAlignment::Center {
            let size = self.get_text_size(text);
            let left_center =  rect.0 + (rect.2 - size.0) / 2;
            self.draw_text(frame, &(left_center, rect.1 + UI_ELEMENT_MARGIN), text, color, background);
        }
    }

    /// Draws a rect with a text blended with the background
    pub fn draw_text_rect_blend(&self, frame: &mut [u8], rect: &(u32, u32, u32, u32), text: &str, color: [u8; 4], align: TextAlignment) {
        if align == TextAlignment::Left {
            self.draw_text_blend(frame, &(rect.0 + UI_ELEMENT_MARGIN, rect.1 + UI_ELEMENT_MARGIN), text, color);
        } else
        if align == TextAlignment::Center {
            let size = self.get_text_size(text);
            let left_center =  rect.0 + (rect.2 - size.0) / 2;
            self.draw_text_blend(frame, &(left_center, rect.1 + UI_ELEMENT_MARGIN), text, color);
        }
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

                let d = pos.0 as usize * 4 + (sx as usize) * 4 + (sy as usize + pos.1 as usize) * (WIDTH as usize) * 4;
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

                let d = pos.0 as usize * 4 + (sx as usize) * 4 + (sy as usize + pos.1 as usize) * (WIDTH as usize) * 4;
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

                let d = pos.0 as usize * 4 + (sx as usize) * 4 + (sy as usize + pos.1 as usize) * (WIDTH as usize) * 4;
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

    /// Draws the given text
    pub fn draw_text(&self,  frame: &mut [u8], pos: &(u32, u32), text: &str, color: [u8; 4], background: [u8; 4]) {

        let def = self.get_default_font();

        let font = def.0;
        let scale = Scale::uniform(def.1);

        let v_metrics = font.v_metrics(scale);

        let glyphs: Vec<_> = font
            .layout( text, scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();

        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let d = ((x + bounding_box.min.x as u32) as usize + pos.0 as usize) * 4 + ((y + bounding_box.min.y as u32) as usize + pos.1 as usize) * (WIDTH as usize) * 4;
                    if v > 0.0 {
                        frame[d..d + 4].copy_from_slice(&self.mix(&background, &color, v));
                    }
                });
            }
        }
    }

    /// Draws the given text
    pub fn draw_text_blend(&self,  frame: &mut [u8], pos: &(u32, u32), text: &str, color: [u8; 4]) {

        let def = self.get_default_font();

        let font = def.0;
        let scale = Scale::uniform(def.1);

        let v_metrics = font.v_metrics(scale);

        let glyphs: Vec<_> = font
            .layout( text, scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();

        for glyph in glyphs {
            if let Some(bounding_box) = glyph.pixel_bounding_box() {
                glyph.draw(|x, y, v| {
                    let d = ((x + bounding_box.min.x as u32) as usize + pos.0 as usize) * 4 + ((y + bounding_box.min.y as u32) as usize + pos.1 as usize) * (WIDTH as usize) * 4;
                    if v > 0.0 {
                        let background = &[frame[d], frame[d+1], frame[d+2], frame[d]+3];
                        frame[d..d + 4].copy_from_slice(&self.mix(&background, &color, v));
                    }
                });
            }
        }
    }

    /// Returns the size of the given text
    fn get_text_size(&self, text: &str) -> (u32, u32) {
        
        let def = self.get_default_font();

        let font = def.0;
        let scale = Scale::uniform(def.1);
        let v_metrics = font.v_metrics(scale);

        let glyphs: Vec<_> = font
            .layout(text, scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();
        
        let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
        let glyphs_width = {
            let min_x = glyphs
                .first()
                .map(|g| g.pixel_bounding_box().unwrap().min.x)
                .unwrap();
            let max_x = glyphs
                .last()
                .map(|g| g.pixel_bounding_box().unwrap().max.x)
                .unwrap();
            (max_x - min_x) as u32
        };

        (glyphs_width, glyphs_height)
    }

    /// Returns the default font and the default rendering size
    pub fn get_default_font(&self) -> (&Font, f32) {
        (&self.gohu_font_14, 16.0)
    }

    /// Returns the default font and the default rendering size
    pub fn get_text_element_height(&self) -> u32 {
        16 + 4
    }

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
