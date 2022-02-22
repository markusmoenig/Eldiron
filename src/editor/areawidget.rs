use server::asset::{ Asset };
use server::asset::tileset::TileUsage;

use crate::widget::context::ScreenContext;

pub struct AreaWidget {
    rect                    : (usize, usize, usize, usize),
    area_index              : usize,
    scale                   : f32,

    offset                  : (usize, usize),

    pub clicked             : bool,
}

impl AreaWidget {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), asset: &Asset, context: &ScreenContext) -> Self {

        Self {
            rect,
            scale                   : 2.0,

            offset                  : (0, 0),
            area_index              : 0,

            clicked                 : false
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
        let map = &asset.tileset.maps[&(self.area_index)];
        let scaled_grid_size = (map.settings.grid_size as f32 * scale) as usize;

        //context.draw2d.draw_square_pattern(frame, &self.rect, self.rect.2, &[44, 44, 46, 255], &[56, 56, 56, 255], scaled_grid_size);

        let x_tiles = map.width / map.settings.grid_size;
        let y_tiles = map.height / map.settings.grid_size;

        let total_tiles = (x_tiles * y_tiles) as usize;

        let screen_x = self.rect.2 / scaled_grid_size;
        let screen_y = self.rect.3 / scaled_grid_size;

        let left_offset = (self.rect.2 % scaled_grid_size) / 2;
        let top_offset = (self.rect.3 % scaled_grid_size) / 2;


    }

    pub fn mouse_down(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {
        true
    }

    pub fn mouse_up(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        let consumed = false;
        consumed
    }

    pub fn _mouse_hover(&mut self, _pos: (usize, usize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        false
    }

    pub fn mouse_dragged(&mut self, pos: (usize, usize), asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if let Some(curr_id) = context.curr_tile {
        }
        false
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        /*
        self.line_offset_counter += delta.1;
        self.line_offset = (self.line_offset_counter / 40).clamp(0, self.max_line_offset as isize) as usize;
        if delta.1 == 0 {
            self.line_offset_counter = 0;
        }
        */
        true
    }

    /// Sets a new map index
    pub fn set_area_index(&mut self, index: usize) {
        self.area_index = index;
    }
}