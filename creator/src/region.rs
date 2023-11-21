use crate::prelude::*;
use std::ops::{Index, IndexMut};

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum RegionType {
    Region2D,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Region {
    pub id: Uuid,
    pub region_type: RegionType,

    pub name: String,
    pub layers: Vec<Layer2D>,

    pub width: i32,
    pub height: i32,
    pub grid_size: i32,
    pub scroll_offset: Vec2i,
    pub zoom: f32,
}

impl Default for Region {
    fn default() -> Self {
        Self::new()
    }
}

impl Region {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            region_type: RegionType::Region2D,

            name: "New Region".to_string(),
            layers: vec![
                Layer2D::new(),
                Layer2D::new(),
                Layer2D::new(),
                Layer2D::new(),
            ],

            width: 80,
            height: 80,
            grid_size: 24,
            scroll_offset: Vec2i::zero(),
            zoom: 1.0,
        }
    }
}

// Implement Index and IndexMut
impl Index<Layer2DRole> for Region {
    type Output = Layer2D;

    fn index(&self, index: Layer2DRole) -> &Self::Output {
        if index == Layer2DRole::Ground {
            &self.layers[0]
        } else if index == Layer2DRole::Wall {
            &self.layers[1]
        } else if index == Layer2DRole::Ceiling {
            &self.layers[2]
        } else {
            &self.layers[3]
        }
    }
}

impl IndexMut<Layer2DRole> for Region {
    fn index_mut(&mut self, index: Layer2DRole) -> &mut Self::Output {
        if index == Layer2DRole::Ground {
            &mut self.layers[0]
        } else if index == Layer2DRole::Wall {
            &mut self.layers[1]
        } else if index == Layer2DRole::Ceiling {
            &mut self.layers[2]
        } else {
            &mut self.layers[3]
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub enum Layer2DRole {
    Ground,
    Wall,
    Ceiling,
    Overlay,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Layer2D {
    #[serde(with = "vectorize")]
    pub tiles: FxHashMap<(u32, u32), Uuid>,
}

impl Default for Layer2D {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer2D {
    pub fn new() -> Self {
        Self {
            tiles: FxHashMap::default(),
        }
    }
}
