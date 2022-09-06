use crate::prelude::*;

// A position in a 2D map
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Position {
    pub region                  : Uuid,
    pub x                       : isize,
    pub y                       : isize,
}

impl Position {
    pub fn new(region: Uuid, x: isize, y: isize) -> Self {
        Self {
            region,
            x,
            y,
        }
    }
}

// A tile in a tilemap or image
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TileId {
    pub tilemap                 : Uuid,
    pub x_off                   : u16,
    pub y_off                   : u16,
    pub size                    : Option<(u16, u16)>,
}

impl TileId {
    pub fn new(tilemap: Uuid, x_off: u16, y_off: u16) -> Self {
        Self {
            tilemap,
            x_off,
            y_off,
            size                : None,
        }
    }

    pub fn new_from_tile_data(tile_data: TileData) -> Self {
        Self {
            tilemap             : tile_data.tilemap,
            x_off               : tile_data.x_off,
            y_off               : tile_data.y_off,
            size                : None,
        }
    }
}

// References a tile in a region
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TileData {
    pub tilemap                 : Uuid,
    pub x_off                   : u16,
    pub y_off                   : u16,
    pub size                    : Option<(u16, u16)>,
    pub usage                   : TileUsage,
}

impl TileData {
    pub fn new(tilemap: Uuid, x_off: u16, y_off: u16) -> Self {
        Self {
            tilemap,
            x_off,
            y_off,
            size                : None,
            usage               : TileUsage::Environment
        }
    }
}