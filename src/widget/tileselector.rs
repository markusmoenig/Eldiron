use crate::widget::*;

use server::asset::Asset;
use crate::tileset::TileUsage;
use itertools::Itertools;

pub struct TileSelectorWidget {
    pub rect                : (usize, usize, usize, usize),
    screen_offset           : (usize, usize),

    tiles                   : Option<Vec<(usize, usize, usize, TileUsage)>>,

    pub grid_size           : usize,
    pub selected            : Option<(usize, usize, usize, TileUsage)>
}

impl TileSelectorWidget {
    
    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) -> Self {

        Self {
            rect,
            screen_offset               : (0, 0),

            tiles                       : None,
            grid_size                   : 32,

            selected                    : None,
        }
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);

        let grid_size = self.grid_size;
        let left_offset = (self.rect.2 % grid_size) / 2;
        let top_offset = (self.rect.3 % grid_size) / 2;

        self.screen_offset = (left_offset, top_offset);

        let grid = (self.rect.2 / self.grid_size, self.rect.3 / self.grid_size);
        let max_tiles = grid.0 * grid.1;

        if let Some(tiles) = &self.tiles {

            let mut x = self.rect.0 + left_offset;
            let mut y = self.rect.1 + top_offset;

            for (index, tile) in tiles.iter().enumerate() {
                if index < max_tiles {
                    let map = asset.get_map_of_id(tile.0);
                    context.draw2d.draw_animated_tile(frame, &(x, y), map, context.width, &(tile.1, tile.2), anim_counter, self.grid_size);

                    if let Some(selected) = &self.selected {
                        if selected.0 == map.settings.id && selected.1 == tile.1 && selected.2 == tile.2 {
                            context.draw2d.draw_rect_outline(frame, &(x, y, grid_size, grid_size), context.width, context.color_white);
                        }
                    }

                    x += self.grid_size;
                    if x + self.grid_size > self.rect.0 + self.rect.2 {
                        x = self.rect.0 + left_offset;
                        y += self.grid_size;
                    }
                }
            }
        }
    }

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if context.contains_pos_for(pos, self.rect) {
            let grid_size = self.grid_size;

            let screen_x = self.rect.2 / grid_size;

            let x = (pos.0 - self.rect.0 - self.screen_offset.0) / grid_size;
            let y = (pos.1 - self.rect.1 - self.screen_offset.0) / grid_size;// + self.line_offset;

            let tile_offset = x + y * screen_x;

            if let Some(tiles) = &self.tiles {
                if tile_offset < tiles.len() {
                    let tile_ref = tiles[tile_offset].clone();
                    self.selected = Some(tile_ref);
                }
            }
            return true;
        }
        false
    }

    pub fn _mouse_up(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if context.contains_pos_for(pos, self.rect) {
            return true;
        }
        false
    }

    /// Collects the tiles of the given type
    pub fn set_tile_type(&mut self, tile_usage: TileUsage, asset: &Asset) {
        let mut tiles : Vec<(usize, usize, usize, TileUsage)> = vec![];
        let sorted_keys= asset.tileset.maps.keys().sorted();

        for key in sorted_keys {
            let map = &asset.tileset.maps[key];
            let amount = map.max_tiles();
            for offset in 0..amount {
                let id = map.offset_to_id(offset);
                let tile = map.get_tile(&id);

                if tile.usage == tile_usage {
                    tiles.push((map.settings.id, id.0, id.1, tile.usage));
                }
            }
        }
        self.tiles = Some(tiles);
    }
}