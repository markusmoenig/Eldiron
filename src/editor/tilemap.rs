use crate::widget::*;
use crate::Asset;

use crate::asset::tileset::TileUsage;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;

pub struct TileMap {
    rect                    : (usize, usize, usize, usize),
    tilemap_index           : usize,
}

impl TileMap {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        let mut widgets : Vec<AtomWidget> = vec![];
        let mut game_button = AtomWidget::new(vec!["Game".to_string()], AtomWidgetType::ToolBarButton, 
            AtomData::new_as_button("Game".to_string()));
        game_button.set_rect((rect.0 + 10, rect.1, 100, rect.3), asset, context);
        widgets.push(game_button);

        Self {
            rect,
            tilemap_index           : 0,
        }
    }    

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {        
        context.draw2d.draw_rect(frame, &self.rect, context.width, &[25, 25, 25, 255]);
        if asset.tileset.maps.is_empty() { return }

        let scale = 2.0;
        let map = &asset.tileset.maps[&(self.tilemap_index as u32)];
        let scaled_grid_size = (map.settings.grid_size as f32 * scale) as usize;

        let x_tiles = map.width / map.settings.grid_size;
        let y_tiles = map.height / map.settings.grid_size;

        let total_tiles = (x_tiles * y_tiles) as usize;

        let screen_x = self.rect.2 / scaled_grid_size;
        let screen_y = self.rect.3 / scaled_grid_size;

        let left_offset = (self.rect.2 % scaled_grid_size) / 2;
        let top_offset = (self.rect.3 % scaled_grid_size) / 2;

        let tiles_per_page = screen_x * screen_y;

        let mut x_off = 0_usize;
        let mut y_off = 0_usize;

        let offset = 0;//page * tiles_per_page;

        // Draw the tiles
        for tile in 0..tiles_per_page {

            if tile + offset >= total_tiles {
                break;
            }

            let x_step = (x_off as f32 * map.settings.grid_size as f32 * scale) as usize;
            let y_step = (y_off as f32 * map.settings.grid_size as f32 * scale) as usize;

            let x = (tile+offset) % x_tiles as usize;
            let y = (tile+offset) / x_tiles as usize;

            let tile = map.get_tile(&(x as u32, y as u32));

            let pp = &(x_step as u32 + self.rect.0 as u32 + left_offset as u32, y_step as u32 + self.rect.1 as u32 + top_offset as u32);

            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len() as usize;

                let p = tile.anim_tiles[index as usize];
                asset.draw_tile(frame, pp, self.tilemap_index as u32, &(p.0, p.1), scale);
            } else
            if tile.usage == TileUsage::Unused {
                asset.draw_tile_mixed(frame, pp, self.tilemap_index as u32, &(x as u32, y as u32), [128, 128, 128, 255], scale);
            } else {
                asset.draw_tile(frame, pp, self.tilemap_index as u32, &(x as u32, y as u32), scale);
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

    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;
        consumed
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        false
    }

    pub fn set_tilemap_index(&mut self, index: usize) {
        self.tilemap_index = index;
    }
}