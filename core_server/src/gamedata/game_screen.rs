
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::script_types::ScriptRGB;

use super::{ GameData, behavior::BehaviorInstanceState };

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum GameScreenWidgetType {
    Game,
    Region,
    Status,
    Custom,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GameScreenWidget {
    pub name                : String,

    pub widget_type         : GameScreenWidgetType,
    pub top_left            : (isize, isize),
    pub bottom_right        : (isize, isize),
}

impl GameScreenWidget {

    pub fn new() -> Self {
        Self {
            name                : "Widget".to_string(),
            widget_type         : GameScreenWidgetType::Game,
            top_left            :(0,0),
            bottom_right        :(0,0),
        }
    }

    pub fn draw(&mut self, selected: bool, data: &mut GameData) {

        if data.game_frame.is_empty() { return; }
        let frame_buffer = &mut data.game_frame[..];
        let rect = (0, 0, data.game_screen_width, data.game_screen_height);
        let stride = data.game_screen_width;
        let grid_size = data.game_screen_tile_size;
        let anim_counter = data.game_anim_counter;

        if let Some(draw2d) = &data.draw2d {

            let sx = self.top_left.0;
            let sy = self.top_left.1;

            let width = self.bottom_right.0 - self.top_left.0 + 1;
            let height = self.bottom_right.1 - self.top_left.1 + 1;

            if self.widget_type == GameScreenWidgetType::Game {

                let index = data.player_ids_inst_indices.get(&131313);
                if let Some(player_index) = index {

                    if let Some(position) = data.instances[*player_index].position {

                        if let Some(region) = data.regions.get(&position.0) {
                            let mut grid_offset : (isize, isize) = (-width / 2, -height / 2);

                            for y in sy..sy+height {
                                for x in sx..sx+width {
                                    let pos = (rect.0 + (x as usize) * grid_size, rect.1 + (y as usize) * grid_size);
                                    if pos.0 >= rect.0 && pos.1 >= rect.1 && pos.0 + grid_size <= rect.0 + rect.2 && pos.1 + grid_size <= rect.1 + rect.3 {
                                        let values = region.get_value((position.1 + grid_offset.0, position.2 + grid_offset.1));

                                        for value in values {
                                            let map = data.asset.as_ref().unwrap().get_map_of_id(value.0);
                                            draw2d.draw_animated_tile(frame_buffer, &pos, map, stride, &(value.1, value.2), anim_counter, grid_size);
                                        }

                                        for index in 0..data.instances.len() {

                                            if data.instances[index].state == BehaviorInstanceState::Killed || data.instances[index].state == BehaviorInstanceState::Purged {
                                                continue;
                                            }

                                            if let Some(char_position) = data.instances[index].position {
                                                if let Some(tile) = data.instances[index].tile {
                                                    // In the same region ?
                                                    if char_position.0 == region.data.id {
                                                        if position.1 + grid_offset.0 == char_position.1 && position.2 + grid_offset.1 == char_position.2 {
                                                            let map = data.asset.as_ref().unwrap().get_map_of_id(tile.0);
                                                            draw2d.draw_animated_tile(frame_buffer, &pos, map, stride, &(tile.1, tile.2), anim_counter, grid_size);
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if selected {
                                            draw2d.blend_rect(frame_buffer, &(pos.0, pos.1, grid_size, grid_size), stride, &[255, 255, 255, 50]);
                                        }
                                    }
                                    grid_offset.0 += 1;
                                }
                                grid_offset.0 = - width / 2;
                                grid_offset.1 += 1;
                            }
                        }
                    }
                }
            } else

            if self.widget_type == GameScreenWidgetType::Custom {

                for y in sy..sy+height {
                    for x in sx..sx+width {
                        let pos = (rect.0 + (x as usize) * grid_size, rect.1 + (y as usize) * grid_size);
                        if pos.0 >= rect.0 && pos.1 >= rect.1 && pos.0 + grid_size <= rect.0 + rect.2 && pos.1 + grid_size <= rect.1 + rect.3 {

                            if selected {
                                draw2d.blend_rect(frame_buffer, &(pos.0, pos.1, grid_size, grid_size), stride, &[255, 255, 255, 50]);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct GameScreen {

    /// The widgets for this screen
    pub widgets             : Vec<GameScreenWidget>,

    /// Tiles
    #[serde(with = "vectorize")]
    pub tiles               : HashMap<(isize, isize), (usize, usize, usize, crate::asset::TileUsage)>,

    /// Grid size
    pub grid_size           : usize,

    /// Current widget index, only used by the editor
    pub curr_widget_index   : usize,
}

impl GameScreen {

    pub fn new() -> Self {
        Self {
            widgets             : vec![],
            tiles               : HashMap::new(),

            grid_size           : 32,
            curr_widget_index   : 0,
        }
    }

    pub fn draw(&mut self, node_id: usize, editor: bool, data: &mut GameData) {

        if let Some(draw2d) = &data.draw2d {
            if data.game_frame.is_empty() == false {

                let mut color = [0, 0, 0, 255];

                if let Some(scope) = data.custom_scopes.get_mut(&node_id) {
                    if let Some(background) = scope.get_value::<ScriptRGB>("background") {
                        color = background.value;
                    }
                }

                draw2d.draw_rect(&mut data.game_frame[..], &(0, 0, data.game_screen_width, data.game_screen_height), data.game_screen_width, &color);
            }
        }

        for index in 0..self.widgets.len() {
            self.widgets[index].draw(editor && index == self.curr_widget_index, data);
        }
    }
}
