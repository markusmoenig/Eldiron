
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{Draw2D, GameData, behavior::BehaviorNodeType};

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

                if data.instances.is_empty() {

                    // Show the Player behavior location if the game is not running

                    let mut player_id : Option<usize> = None;
                    for (index, name) in data.behaviors_names.iter().enumerate() {
                        if name == "Player" {
                            player_id = Some(data.behaviors_ids[index]);
                        }
                    }

                    if let Some(player_id) = player_id {

                        let mut position : Option<(usize, isize, isize)> = None;
                        let mut tile     : Option<(usize, usize, usize)> = None;

                        if let Some(behavior) = data.behaviors.get_mut(&player_id) {
                            for (id, node) in &behavior.data.nodes {
                                if node.behavior_type == BehaviorNodeType::BehaviorType {
                                    if let Some(value )= node.values.get(&"position".to_string()) {
                                        position = Some((value.0 as usize, value.1 as isize, value.2 as isize));
                                    }
                                    if let Some(value )= node.values.get(&"tile".to_string()) {
                                        tile = Some((value.0 as usize, value.1 as usize, value.2 as usize));
                                    }
                                }
                            }
                        }

                        if let Some(position) = position {

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

                                // Draw Behaviors
                                /*
                                for (id, _behavior) in &data.behaviors {
                                    if let Some(position) = data.get_behavior_default_position(*id) {
                                        // In the same region ?
                                        if position.0 == region.data.id {

                                            // Row check
                                            if position.1 >= offset.0 && position.1 < offset.0 + x_tiles {
                                                // Column check
                                                if position.2 >= offset.1 && position.2 < offset.1 + y_tiles {
                                                    // Visible
                                                    if let Some(tile) = data.get_behavior_default_tile(*id) {

                                                        let pos = (rect.0 + left_offset + ((position.1 - offset.0) as usize) * tile_size, rect.1 + top_offset + ((position.2 - offset.1) as usize) * tile_size);

                                                        let map = asset.get_map_of_id(tile.0);
                                                        self.draw_animated_tile(frame, &pos, map, stride, &(tile.1, tile.2), anim_counter, tile_size);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }*/
                            }
                        }
                    }
                } else {
                    for y in sy..sy+height {
                        for x in sx..sx+width {
                            let pos = (rect.0 + ((x - offset.0) as usize) * grid_size, rect.1 + ((y - offset.1) as usize) * grid_size);
                            if pos.0 >= rect.0 && pos.1 >= rect.1 && pos.0 + grid_size < rect.0 + rect.2 && pos.1 + grid_size < rect.1 + rect.3 {
                                draw2d.draw_rect(frame, &(pos.0, pos.1, grid_size, grid_size), stride, &[255, 0, 0, 255]);
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
