use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RegionUpdate {
    pub id: Uuid,

    #[serde(with = "vectorize")]
    pub wallfx: FxHashMap<(i32, i32), WallFxUpdate>,
    pub characters: FxHashMap<Uuid, CharacterUpdate>,
    pub items: FxHashMap<Uuid, ItemUpdate>,

    #[serde(skip)]
    // The pixel position of the characters, their tile id, their character id and facing.
    pub characters_pixel_pos: Vec<(Vec2<f32>, Uuid, Uuid, Vec2<f32>)>,

    pub server_tick: i64,
    pub daylight: Vec3<f32>,
}

impl Default for RegionUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionUpdate {
    pub fn new() -> Self {
        Self {
            id: Uuid::nil(),
            wallfx: FxHashMap::default(),
            characters: FxHashMap::default(),
            items: FxHashMap::default(),
            server_tick: 0,
            daylight: Vec3::one(),
            characters_pixel_pos: vec![],
        }
    }

    /// Clear the update.
    pub fn clear(&mut self) {
        self.characters.clear();
    }

    /// Generate the map pixel positions of each character between ticks.
    pub fn generate_character_pixel_positions(
        &mut self,
        _grid_size: f32,
        tiles: &FxHashMap<Uuid, TheRGBATile>,
        _buffer_size: Vec2<i32>,
        _region_height: i32,
        draw_settings: &mut RegionDrawSettings,
    ) {
        // Position, tile id, character id, facing
        let mut characters_pixel_pos: Vec<(Vec2<f32>, Uuid, Uuid, Vec2<f32>)> = vec![];

        #[allow(clippy::for_kv_map)]
        for (_id, character) in &mut self.characters {
            let draw_pos = if let Some((start, end)) = &mut character.moving {
                // pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
                //     let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
                //     t * t * (3.0 - 2.0 * t)
                // }

                let sum = (draw_settings.delta + character.move_delta).clamp(0.0, 1.0);
                // let d = smoothstep(0.0, 1.0, sum);//.clamp(0.0, 1.0);
                let d = sum;
                // let d = if sum < 0.5 {
                //     2.0 * sum * sum
                // } else {
                //     1.0 - (-2.0 * sum + 2.0).powi(2) / 2.0
                // };
                let x = start.x * (1.0 - d) + end.x * d;
                let y = start.y * (1.0 - d) + end.y * d;
                character.move_delta = sum;
                Vec2::new(
                    x, //(x * grid_size).round() as i32,
                    y, //(y * grid_size).round() as i32,
                )
            } else {
                Vec2::new(
                    character.position.x, //(character.position.x * grid_size) as i32,
                    character.position.y, //(character.position.y * grid_size) as i32,
                )
            };

            let facing = if let Some((start, end)) = &mut character.facing_anim {
                // pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
                //     let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
                //     t * t * (3.0 - 2.0 * t)
                // }

                let sum = (draw_settings.delta + character.move_delta).clamp(0.0, 1.0);
                // let d = smoothstep(0.0, 1.0, sum);//.clamp(0.0, 1.0);
                let d = sum;
                // let d = if sum < 0.5 {
                //     2.0 * sum * sum
                // } else {
                //     1.0 - (-2.0 * sum + 2.0).powi(2) / 2.0
                // };
                let x = start.x * (1.0 - d) + end.x * d;
                let y = start.y * (1.0 - d) + end.y * d;
                character.move_delta = sum;
                Vec2::new(x, y)
            } else {
                character.facing
            };

            /*
            if Some(*id) == draw_settings.center_on_character {
                let center_x = (buffer_size.x as f32 / 2.0) as i32 - grid_size as i32 / 2;
                let center_y = (buffer_size.y as f32 / 2.0) as i32 + grid_size as i32 / 2;
                draw_settings.offset.x = draw_pos.x - center_x;
                draw_settings.offset.y = region_height - (draw_pos.y + center_y);
                draw_settings.center_3d = vec3f(
                    draw_pos.x as f32 / grid_size,
                    0.5,
                    draw_pos.y as f32 / grid_size,
                );
                draw_settings.facing_3d = vec3f(facing.x, 0.0, facing.y);
            }*/

            // Find the tile id for the character.
            for (id, tile) in tiles {
                if tile.name == character.tile_name {
                    characters_pixel_pos.push((draw_pos, *id, character.id, facing));
                }
            }
        }

        self.characters_pixel_pos = characters_pixel_pos;
    }

    /// Create an update from json.
    pub fn from_json(json: &str) -> Option<Self> {
        if let Ok(update) = serde_json::from_str(json) {
            Some(update)
        } else {
            None
        }
    }

    /// Convert the update to json.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).unwrap_or_default()
    }
}

/// A character as described by the server for consumption by the client.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CharacterUpdate {
    pub id: Uuid,
    pub tile_name: String,

    pub name: String,

    pub position: Vec2<f32>,
    pub moving: Option<(Vec2<f32>, Vec2<f32>)>,

    pub facing: Vec2<f32>,
    pub facing_anim: Option<(Vec2<f32>, Vec2<f32>)>,

    #[serde(skip)]
    pub move_delta: f32,
}

impl Default for CharacterUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl CharacterUpdate {
    pub fn new() -> Self {
        Self {
            id: Uuid::nil(),
            tile_name: "".to_string(),

            name: "".to_string(),

            position: Vec2::new(0.0, 0.0),
            moving: None,

            facing: Vec2::new(0.0, -1.0),
            facing_anim: None,

            move_delta: 0.0,
        }
    }
}

/// An item as described by the server for consumption by the client.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ItemUpdate {
    pub tile_id: Uuid,
    pub tile_name: String,

    pub name: String,
    pub position: Vec2<f32>,
}

impl Default for ItemUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl ItemUpdate {
    pub fn new() -> Self {
        Self {
            tile_id: Uuid::nil(),
            tile_name: "".to_string(),

            name: "".to_string(),
            position: Vec2::new(0.0, 0.0),
        }
    }
}

/// Update structure for the current wall effects in the region.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WallFxUpdate {
    /// When this effect got inserted.
    pub at_tick: i64,

    pub fx: WallFX,
    pub prev_fx: WallFX,
}

impl Default for WallFxUpdate {
    fn default() -> Self {
        Self::new()
    }
}

impl WallFxUpdate {
    pub fn new() -> Self {
        Self {
            at_tick: 0,
            fx: WallFX::Normal,
            prev_fx: WallFX::Normal,
        }
    }
}
