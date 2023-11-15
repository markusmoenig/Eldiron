use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tilemap {
    pub id: Uuid,

    pub name: String,
    pub buffer: TheRGBABuffer,

    pub grid_size: i32,
    pub scroll_offset: Vec2i,
    pub zoom: f32,

    pub tiles: Vec<Tile>,
}

impl Tilemap {
    pub fn default() -> Self {
        Self {
            id: Uuid::new_v4(),

            name: String::new(),
            buffer: TheRGBABuffer::empty(),

            grid_size: 24,
            scroll_offset: Vec2i::zero(),
            zoom: 1.0,

            tiles: vec![],
        }
    }

    /// Set the buffer
    pub fn set_buffer(&mut self, buffer: TheRGBABuffer) {
        self.buffer = buffer;
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TileRole {
    GrassAndWood,
    Hill,
    Road,
    Water,
    Icon,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub name: String,
    pub role: TileRole,

    pub regions: Vec<TheRGBARegion>,
    pub blocking: bool,
}

impl Tile {
    pub fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            role: TileRole::GrassAndWood,

            regions: vec![],
            blocking: false,
        }
    }
}