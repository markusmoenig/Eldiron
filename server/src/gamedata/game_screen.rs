
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{ GameData };

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Copy, Clone)]
pub enum GameScreenWidgetType {
    Game,
    Region,
    Status,
    Text,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameScreenWidget {
    pub name                : String,

    pub widget_type         : GameScreenWidgetType,
    pub top_left            : (isize, isize),
    pub bottom_right        : (isize, isize),
}

impl GameScreenWidget {

    pub fn new() -> Self {
        Self {
            name                : "New Widget".to_string(),
            widget_type         : GameScreenWidgetType::Game,
            top_left            :(0,0),
            bottom_right        :(0,0),
        }
    }

    pub fn draw(&mut self, frame: &mut [u8], rect: (usize, usize, usize, usize), stride: usize, offset: (isize, isize), anim_counter: usize, grid_size: usize, data: &mut GameData) {

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
                                    let pos = (rect.0 + ((x - offset.0) as usize) * grid_size, rect.1 + ((y - offset.1) as usize) * grid_size);
                                    if pos.0 >= rect.0 && pos.1 >= rect.1 && pos.0 + grid_size < rect.0 + rect.2 && pos.1 + grid_size < rect.1 + rect.3 {
                                        let values = region.get_value((position.1 + grid_offset.0, position.2 + grid_offset.1));

                                        for value in values {
                                            let map = data.asset.as_ref().unwrap().get_map_of_id(value.0);
                                            draw2d.draw_animated_tile(&mut frame[..], &pos, map, stride, &(value.1, value.2), anim_counter, grid_size);
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
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq)]
pub struct GameScreen {

    pub widgets             : Vec<GameScreenWidget>,

    #[serde(with = "vectorize")]
    pub tiles               : HashMap<(isize, isize), (usize, usize, usize, crate::asset::TileUsage)>,

    pub grid_size           : usize,
}

impl GameScreen {

    pub fn new() -> Self {
        Self {
            widgets             : vec![],
            tiles               : HashMap::new(),

            grid_size           : 32,
        }
    }
}
