
pub mod tileset;

use rusttype::{Font};

pub use tileset::*;

pub struct Asset<'a> {
    pub tileset                 : TileSet,
    //gohu_font_11              : Font<'a>,
    //gohu_font_14                : Font<'a>,
    pub open_sans               : Font<'a>,
    pub grid_size               : u32,
}

impl Asset<'_>  {

    pub fn new() -> Self where Self: Sized {

        Self {
            tileset         : tileset::TileSet::new(),
            //gohu_font_11    : Font::try_from_bytes(include_bytes!("../assets/fonts/gohufont-uni-11.ttf") as &[u8]).expect("Error constructing Font"),
            //gohu_font_14    : Font::try_from_bytes(include_bytes!("../assets/fonts/Open_Sans/static/OpenSans/OpenSans-SemiBold.ttf") as &[u8]).expect("Error constructing Font"),
            open_sans       : Font::try_from_bytes(include_bytes!("../../assets/fonts/Open_Sans/static/OpenSans/OpenSans-Regular.ttf") as &[u8]).expect("Error constructing Font"),
            grid_size       : 32,
        }
    }

    /// Returns the tilemap of the given id
    pub fn get_map_of_id(&self, id: usize) -> &TileMap {
        &self.tileset.maps[&id]
    }

    /*
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
    }*/

    /// Returns the tile fo the given id
    pub fn get_tile(&self, id: &(usize, usize, usize)) -> Tile {
        let map = self.get_map_of_id(id.0);
        map.get_tile(&(id.1, id.2))
    }
}
