use crate::widget::*;
use crate::Asset;

use crate::asset::tileset::TileUsage;

use crate::widget::atom::AtomWidget;
use crate::widget::atom::AtomWidgetType;
use crate::widget::context::ScreenContext;

pub struct TileMap {
    rect                    : (usize, usize, usize, usize),
    tilemap_index           : usize,
    scale                   : f32,

    line_offset             : usize,
    max_line_offset         : usize,
    line_offset_counter     : isize
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
            scale                   : 2.0,

            line_offset             : 0,
            max_line_offset         : 0,
            line_offset_counter     : 0
        }
    }

    pub fn resize(&mut self, width: usize, height: usize, _context: &ScreenContext) {
        self.rect.2 = width;
        self.rect.3 = height;
    }

    pub fn draw(&mut self, frame: &mut [u8], anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {
        //context.draw2d.draw_rect(frame, &self.rect, context.width, &[44, 44, 44, 255]);
        context.draw2d.draw_rect(frame, &self.rect, context.width, &context.color_black);
        if asset.tileset.maps.is_empty() { return }

        let scale = self.scale;
        let map = &asset.tileset.maps[&(self.tilemap_index)];
        let scaled_grid_size = (map.settings.grid_size as f32 * scale) as usize;

        //context.draw2d.draw_square_pattern(frame, &self.rect, self.rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], scaled_grid_size);

        let x_tiles = map.width / map.settings.grid_size;
        let y_tiles = map.height / map.settings.grid_size;

        let total_tiles = (x_tiles * y_tiles) as usize;

        let screen_x = self.rect.2 / scaled_grid_size;
        let screen_y = self.rect.3 / scaled_grid_size;

        let left_offset = (self.rect.2 % scaled_grid_size) / 2;
        let top_offset = (self.rect.3 % scaled_grid_size) / 2;

        let tiles_per_page = screen_x * screen_y;

        self.max_line_offset = 0;

        if total_tiles > tiles_per_page {
            self.max_line_offset = (total_tiles - tiles_per_page) / screen_x;
            if (total_tiles - tiles_per_page) % screen_x != 0 {
                self.max_line_offset += 1;
            }
        }

        let mut x_off = 0_usize;
        let mut y_off = 0_usize;

        let offset = self.line_offset * screen_x;

        // Draw the tiles
        for tile in 0..tiles_per_page {

            if tile + offset >= total_tiles {
                break;
            }

            let x_step = (x_off as f32 * map.settings.grid_size as f32 * scale) as usize;
            let y_step = (y_off as f32 * map.settings.grid_size as f32 * scale) as usize;

            let x = (tile+offset) % x_tiles as usize;
            let y = (tile+offset) / x_tiles as usize;

            let tile = map.get_tile(&(x, y));

            let pp = &(x_step + self.rect.0 + left_offset, y_step + self.rect.1 + top_offset);

            if tile.anim_tiles.len() > 0 {
                let index = anim_counter % tile.anim_tiles.len() as usize;
                context.draw2d.draw_tile(frame, pp, map, context.width, &tile.anim_tiles[index], scale);
            } else
            if tile.usage == TileUsage::Unused {
                context.draw2d.draw_tile_mixed(frame, pp, map, context.width, &(x, y), [128, 128, 128, 255], scale);
            } else {
                context.draw2d.draw_tile(frame, pp, map, context.width, &(x, y), scale);
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

        let grid_pos = self.screen_to_map(asset, pos);

        context.curr_tile = Some(grid_pos);
        //println!("{:?}", grid_pos);

        false
    }

    pub fn mouse_up(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        let mut consumed = false;
        consumed
    }

    pub fn mouse_hover(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        self.line_offset_counter += delta.1;
        self.line_offset = (self.line_offset_counter / 40).clamp(0, self.max_line_offset as isize) as usize;
        if delta.1 == 0 {
            self.line_offset_counter = 0;
        }
        true
    }

    /// Sets a new map index
    pub fn set_tilemap_index(&mut self, index: usize) {
        self.tilemap_index = index;
        self.line_offset = 0;
    }

    /// Converts a screen position to a map grid position
    fn screen_to_map(&self, asset: &Asset, screen_pos: (usize, usize)) -> (usize, usize) {

        let scale = self.scale;

        let map = asset.get_map_of_id(self.tilemap_index as usize);

        let scaled_grid_size = (map.settings.grid_size as f32 * scale) as usize;

        let x_tiles = (map.width / map.settings.grid_size) as usize;
        let y_tiles = (map.height / map.settings.grid_size) as usize;

        let screen_x = self.rect.2 / scaled_grid_size;

        let x = (screen_pos.0 - self.rect.0)/ scaled_grid_size;// + self.screen_start.0;
        let y = (screen_pos.1 - self.rect.1) / scaled_grid_size;// + self.screen_start.1;

        let tile_offset = x + y * screen_x;

        ((tile_offset % x_tiles), (tile_offset / y_tiles))
    }
}