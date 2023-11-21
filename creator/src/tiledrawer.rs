use crate::prelude::*;

pub struct TileDrawer {
    pub tiles: FxHashMap<Uuid, TheRGBATile>,
}

#[allow(clippy::new_without_default)]
impl TileDrawer {
    pub fn new() -> Self {
        Self {
            tiles: FxHashMap::default(),
        }
    }

    pub fn draw_single_tile(&mut self, at: Vec2i, buffer: &mut TheRGBABuffer, grid: i32, tile: Uuid, ctx: &mut TheContext) {
        if let Some(data) = self.tiles.get(&tile) {
            let x = (at.x * grid) as usize;
            let y = (at.y * grid) as usize;
            let stride = buffer.stride();
            ctx.draw.copy_slice(buffer.pixels_mut(), data.buffer[0].pixels(), &(x, y, 24, 24), stride);
        }
    }
}
