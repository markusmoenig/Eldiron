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

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum TileRole {
    Character, // #d9ac8b
    GrassAndWood, // #3e6958
    Hill, // #b1a58d
    Road, // #624c3c
    Water, // #243d5c
    ManMade, // #e0c872
    Dungeon, // #b03a48
    Effect, // #d4804d
    Icon, // #5c8b93
    UI // #e3cfb4
}

impl TileRole {
    pub fn to_color(self) -> TheColor {
        match self {
            TileRole::Character => { TheColor::from_hex("#d9ac8b") }
            TileRole::GrassAndWood => { TheColor::from_hex("#3e6958") }
            TileRole::Hill => { TheColor::from_hex("#b1a58d") }
            TileRole::Road => { TheColor::from_hex("#624c3c") }
            TileRole::Water => { TheColor::from_hex("#243d5c") }
            TileRole::ManMade => { TheColor::from_hex("#e0c872") }
            TileRole::Dungeon => { TheColor::from_hex("#b03a48") }
            TileRole::Effect => { TheColor::from_hex("#d4804d") }
            TileRole::Icon => { TheColor::from_hex("#5c8b93") }
            TileRole::UI => { TheColor::from_hex("#e3cfb4") }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tile {
    pub id: Uuid,
    pub name: String,
    pub role: Vec<TileRole>,

    pub regions: Vec<TheRGBARegion>,
    pub blocking: bool,
}

impl Tile {
    pub fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            role: vec![TileRole::GrassAndWood],

            regions: vec![],
            blocking: false,
        }
    }
}