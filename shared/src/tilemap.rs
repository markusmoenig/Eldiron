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

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum TileRole {
    Character, // #d9ac8b
    Nature,    // #3e6958
    Mountain,  // #b1a58d
    Road,      // #624c3c
    Water,     // #243d5c
    ManMade,   // #e0c872
    Dungeon,   // #b03a48
    Effect,    // #d4804d
    Icon,      // #5c8b93
    UI,        // #e3cfb4
}

impl TileRole {
    pub fn to_color(self) -> TheColor {
        match self {
            TileRole::Character => TheColor::from_hex("#d9ac8b"),
            TileRole::Nature => TheColor::from_hex("#3e6958"),
            TileRole::Mountain => TheColor::from_hex("#b1a58d"),
            TileRole::Road => TheColor::from_hex("#624c3c"),
            TileRole::Water => TheColor::from_hex("#243d5c"),
            TileRole::ManMade => TheColor::from_hex("#e0c872"),
            TileRole::Dungeon => TheColor::from_hex("#b03a48"),
            TileRole::Effect => TheColor::from_hex("#d4804d"),
            TileRole::Icon => TheColor::from_hex("#5c8b93"),
            TileRole::UI => TheColor::from_hex("#e3cfb4"),
        }
    }
    pub fn to_string(self) -> &'static str {
        match self {
            TileRole::Character => "Character",
            TileRole::Nature => "Nature",
            TileRole::Mountain => "Mountain",
            TileRole::Road => "Road",
            TileRole::Water => "Water",
            TileRole::ManMade => "Man Made",
            TileRole::Dungeon => "Dungeon",
            TileRole::Effect => "Effect",
            TileRole::Icon => "Icon",
            TileRole::UI => "UI",
        }
    }
    pub fn iterator() -> impl Iterator<Item = TileRole> {
        [
            TileRole::Character,
            TileRole::Nature,
            TileRole::Mountain,
            TileRole::Road,
            TileRole::Water,
            TileRole::ManMade,
            TileRole::Dungeon,
            TileRole::Effect,
            TileRole::Icon,
            TileRole::UI,
        ]
        .iter()
        .copied()
    }
    pub fn from_index(index: u8) -> Option<TileRole> {
        match index {
            0 => Some(TileRole::Character),
            1 => Some(TileRole::Nature),
            2 => Some(TileRole::Mountain),
            3 => Some(TileRole::Road),
            4 => Some(TileRole::Water),
            5 => Some(TileRole::ManMade),
            6 => Some(TileRole::Dungeon),
            7 => Some(TileRole::Effect),
            8 => Some(TileRole::Icon),
            9 => Some(TileRole::UI),
            _ => None,
        }
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
