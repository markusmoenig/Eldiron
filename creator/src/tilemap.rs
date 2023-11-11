use crate::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Tilemap {
    pub id: Uuid,

    pub name: String,
    pub buffer: TheRGBABuffer,

    pub scroll_offset: Vec2i,
    pub zoom: f32,
}

impl Tilemap {
    pub fn default() -> Self {
        Self {
            id: Uuid::new_v4(),

            name: String::new(),
            buffer: TheRGBABuffer::empty(),

            scroll_offset: Vec2i::zero(),
            zoom: 1.0,
        }
    }

    /// Set the buffer
    pub fn set_buffer(&mut self, buffer: TheRGBABuffer) {
        self.buffer = buffer;
    }
}