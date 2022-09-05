use crate::widget::*;

#[derive(PartialEq, Clone, Debug)]
pub struct CharacterMetaData {
    pub id                  : Uuid,
    pub tile                : TileId,
    pub name                : String,
}

pub struct CharacterSelectorWidget {
    pub rect                : (usize, usize, usize, usize),
    screen_offset           : (usize, usize),

    characters              : Vec<CharacterMetaData>,

    pub grid_size           : usize,
    pub selected            : Option<CharacterMetaData>,

    mouse_wheel_delta       : isize,

    line_offset             : isize,
    max_line_offset         : usize,
}

impl CharacterSelectorWidget {

    pub fn new(_text: Vec<String>, rect: (usize, usize, usize, usize), _asset: &Asset, _context: &ScreenContext) -> Self {

        Self {
            rect,
            screen_offset               : (0, 0),

            characters                  : vec![],

            grid_size                   : 32,

            selected                    : None,

            mouse_wheel_delta           : 0,

            line_offset                 : 0,
            max_line_offset             : 0,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.rect.2 = width;
        self.rect.3 = height;
        self.line_offset = 0;
    }

    pub fn draw(&mut self, frame: &mut [u8], stride: usize, anim_counter: usize, asset: &mut Asset, context: &mut ScreenContext) {

        context.draw2d.draw_rect(frame, &self.rect, stride, &context.color_black);

        let grid_size = self.grid_size;
        let left_offset = (self.rect.2 % grid_size) / 2;
        let top_offset = (self.rect.3 % grid_size) / 2;

        self.screen_offset = (left_offset, top_offset);

        let grid = (self.rect.2 / self.grid_size, self.rect.3 / self.grid_size);
        let max_tiles = grid.0 * grid.1;

        self.max_line_offset = 0;

        let chars = &self.characters;

        if chars.len() > max_tiles {
            self.max_line_offset = (chars.len() - max_tiles) / grid_size;
            if (chars.len() - max_tiles) % grid_size != 0 {
                self.max_line_offset += 1;
            }
        }

        let mut x = self.rect.0 + left_offset;
        let mut y = self.rect.1 + top_offset;

        let offset = self.line_offset as usize * grid.0;

        for index in 0..max_tiles {

            if index + offset >= chars.len() {
                break;
            }

            let tile = &chars[index + offset];

            if let Some(map) = asset.get_map_of_id(tile.tile.map) {
                context.draw2d.draw_animated_tile(frame, &(x, y), map, stride, &(tile.tile.x_off as usize, tile.tile.y_off as usize), anim_counter, self.grid_size);

                if let Some(selected) = &self.selected {
                    if selected.tile.map == map.settings.id && selected.tile.x_off == tile.tile.x_off && selected.tile.y_off == tile.tile.y_off {
                        context.draw2d.draw_rect_outline(frame, &(x, y, grid_size, grid_size), stride, context.color_white);
                    }
                }
            }

            x += self.grid_size;
            if x + self.grid_size > self.rect.0 + self.rect.2 {
                x = self.rect.0 + left_offset;
                y += self.grid_size;
            }
        }

    }

    pub fn mouse_down(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {

        if context.contains_pos_for(pos, self.rect) {
            let grid_size = self.grid_size;

            let screen_x = self.rect.2 / grid_size;

            if pos.0 >= self.rect.0 + self.screen_offset.0 && pos.1 > self.rect.1 + self.screen_offset.1 {

                let x = (pos.0 - self.rect.0 - self.screen_offset.0) / grid_size;
                let y = (pos.1 - self.rect.1 - self.screen_offset.1) / grid_size;
                let tile_offset = x + y * screen_x + self.line_offset as usize * screen_x;

                let chars = &self.characters;
                if tile_offset < chars.len() {
                    let tile_ref = chars[tile_offset].clone();
                    self.selected = Some(tile_ref);
                }

                return true;
            }
        }
        false
    }

    pub fn _mouse_up(&mut self, pos: (usize, usize), _asset: &mut Asset, context: &mut ScreenContext) -> bool {
        if context.contains_pos_for(pos, self.rect) {
            return true;
        }
        false
    }

    pub fn mouse_wheel(&mut self, delta: (isize, isize), _asset: &mut Asset, _context: &mut ScreenContext) -> bool {
        self.mouse_wheel_delta += delta.1;
        self.line_offset += self.mouse_wheel_delta / self.grid_size as isize;
        self.line_offset = self.line_offset.clamp(0, self.max_line_offset as isize);
        self.mouse_wheel_delta -= (self.mouse_wheel_delta / self.grid_size as isize) * self.grid_size as isize;
        true
    }

    pub fn collect(&mut self,  context: &mut ScreenContext) {
        self.characters = vec![];
        for id in &context.data.behaviors_ids {
            if let Some(behavior) = context.data.behaviors.get(&id) {
                if behavior.name != "Player" {
                    if let Some(tile) = context.data.get_behavior_default_tile(*id) {

                        let meta = CharacterMetaData {id: *id, tile: tile.clone(), name: behavior.name.clone() };
                        self.characters.push(meta);
                    }
                }
            }
        }

        if self.characters.is_empty() == false {
            self.selected = Some(self.characters[0].clone());
        }
    }

}