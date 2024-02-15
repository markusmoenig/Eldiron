use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Asset {
    pub id: Uuid,

    pub name: String,
    pub buffer: AssetBuffer,
}

impl Default for Asset {
    fn default() -> Self {
        Self::new()
    }
}

impl Asset {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),

            name: String::new(),
            buffer: AssetBuffer::Empty,
        }
    }

    /// Set the asset buffer
    pub fn set_buffer(&mut self, buffer: AssetBuffer) {
        self.buffer = buffer;
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum AssetBuffer {
    Empty,
    Image(TheRGBABuffer),
    Font(Vec<u8>),
}

impl AssetBuffer {
    pub fn to_string(self) -> &'static str {
        match self {
            AssetBuffer::Empty => "Empty",
            AssetBuffer::Image(_) => "Image",
            AssetBuffer::Font(_) => "Font",
        }
    }
}
