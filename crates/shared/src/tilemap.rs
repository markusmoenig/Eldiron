use rusterix::TileRole;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tilemap {
    pub id: Uuid,

    pub name: String,
    pub buffer: TheRGBABuffer,

    pub grid_size: i32,
    pub scroll_offset: Vec2<i32>,
    pub zoom: f32,

    pub tiles: Vec<Tile>,
}

impl Default for Tilemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Tilemap {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            name: String::new(),
            buffer: TheRGBABuffer::empty(),

            grid_size: 24,
            scroll_offset: Vec2::zero(),
            zoom: 1.0,

            tiles: vec![],
        }
    }

    /// Set the buffer
    pub fn set_buffer(&mut self, buffer: TheRGBABuffer) {
        self.buffer = buffer;
    }
}

fn default_tile_scale() -> f32 {
    1.0
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub name: String,
    pub role: TileRole,
    #[serde(default = "default_tile_scale")]
    pub scale: f32,

    pub sequence: TheRGBARegionSequence,
    pub blocking: bool,
    #[serde(default)]
    pub render_mode: u8,
}

impl Default for Tile {
    fn default() -> Self {
        Self::new()
    }
}

impl Tile {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            role: TileRole::Nature,
            scale: 1.0,

            sequence: TheRGBARegionSequence::new(),
            blocking: false,
            render_mode: 0,
        }
    }
}
