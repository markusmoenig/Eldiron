use crate::Texture;
use crate::{Assets, Map};

pub struct D2MaterialBuilder {}

impl Default for D2MaterialBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl D2MaterialBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build_texture(&self, _map: &Map, _assets: &Assets, _texture: &mut Texture) {}
}
