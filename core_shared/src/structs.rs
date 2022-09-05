use crate::prelude::*;

// A position in a 2D map
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Position {
    pub map                     : Uuid,
    pub x                       : i32,
    pub y                       : i32,
}

impl Position {
    pub fn new(map: Uuid, x: i32, y: i32) -> Self {
        Self {
            map,
            x,
            y,
        }
    }
}

// A tile in a tilemap or image
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TileId {
    pub map                     : Uuid,
    pub x_off                   : u16,
    pub y_off                   : u16,
    pub size                    : Option<(u16, u16)>,
}

impl TileId {
    pub fn new(map: Uuid, x_off: u16, y_off: u16) -> Self {
        Self {
            map,
            x_off,
            y_off,
            size                : None,
        }
    }
}

// References a tile in a region
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TileData {
    pub tilemap                 : Uuid,
    pub grid_x                  : u16,
    pub grid_y                  : u16,
    pub usage                   : TileUsage,
}
// pub struct TileRef {
//     pub map                     : Uuid,
//     pub x                       : u16,
//     pub y                       : u16,
//     pub size                    : Option<(u16, u16)>,
//     pub usage                   : Option<TileUsage>
// }

// impl TileRef {

// }