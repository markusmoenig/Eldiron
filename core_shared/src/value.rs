use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub enum Value {
    // Empty
    Empty(),
    // Number
    Float(f32),
    // Number, can be both float or integer
    Integer(i32),
    // Uuid of region, and 2D position
    Position(Position),
    // Uuid of region, and Uuid of area
    Area(Uuid, Uuid),
    // Uuid of tilemap and 2D offset
    Tile(Uuid, u16, u16),
    // Text (or script)
    String(String),
    // Tile
    TileId(TileId),
    //
    TileData(TileData),
}

impl Value {

    pub fn to_float(&self ) -> Option<f32> {
        match self {
            Value::Integer(value) => return Some(*value as f32),
            Value::Float(value) => return Some(*value),
            _ => None,
        }
    }

    pub fn to_integer(&self ) -> Option<i32> {
        match self {
            Value::Float(value) => return Some(*value as i32),
            Value::Integer(value) => return Some(*value),
            _ => None,
        }
    }

    pub fn to_string(&self ) -> Option<String> {
        match self {
            Value::String(value) => return Some(value.clone()),
            _ => None,
        }
    }

    pub fn to_string_value(&self ) -> String {
        match self {
            Value::String(value) => return value.clone(),
            _ => "".to_string(),
        }
    }

    pub fn to_position(&self ) -> Option<Position> {
        match self {
            Value::Position(value) => return Some(value.clone()),
            _ => None,
        }
    }

    pub fn to_tile_data(&self ) -> Option<TileData> {
        match self {
            Value::TileData(value) => return Some(value.clone()),
            _ => None,
        }
    }

    pub fn to_tile_id(&self ) -> Option<TileId> {
        match self {
            Value::Tile(value, x, y) => {
                return Some(TileId::new(*value, *x, *y));
            },
            Value::TileId(value) => return Some(value.clone()),
            _ => None,
        }
    }
}