use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Vertex {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

impl Vertex {
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self { id, x, y }
    }

    pub fn as_vec2f(&self) -> Vec2f {
        vec2f(self.x, self.y)
    }
}
