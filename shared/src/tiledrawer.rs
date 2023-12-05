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

    pub fn draw_region(
        &mut self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        ctx: &mut TheContext,
    ) {
        for (coord, tile) in &region.tiles{
            for index in 0..2 {
                if let Some(tile_uuid) = tile.layers[index] {
                    self.draw_tile(
                        vec2i(coord.0, coord.1),
                        buffer,
                        region.grid_size,
                        tile_uuid,
                        ctx,
                    );
                }
            }
        }
    }

    pub fn draw_tile(
        &mut self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        tile: Uuid,
        ctx: &mut TheContext,
    ) {
        if let Some(data) = self.tiles.get(&tile) {
            let x = (at.x * grid) as usize;
            let y = (at.y * grid) as usize;
            let stride = buffer.stride();
            ctx.draw.copy_slice(
                buffer.pixels_mut(),
                data.buffer[0].pixels(),
                &(x, y, 24, 24),
                stride,
            );
        }
    }
}
