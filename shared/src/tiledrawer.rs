use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

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
        &self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        anim_counter: &usize,
        _ctx: &mut TheContext,
    ) {
        let _start = self.get_time();

        /*
        buffer.pixels_mut().fill(0);
        for (coord, tile) in &region.tiles {
            for index in 0..tile.layers.len() {
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
        }*/

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height;

        let pixels = buffer.pixels_mut();

        let tile_size = 24;

        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as i32;
                    let y = height - (i / width) as i32 - 1;

                    let tile_x = x / tile_size;
                    let tile_y = y / tile_size;

                    let mut color = BLACK;

                    if let Some(tile) = region.tiles.get(&(tile_x, tile_y)) {
                        for index in 0..tile.layers.len() {
                            if let Some(tile_uuid) = tile.layers[index] {
                                if let Some(data) = self.tiles.get(&tile_uuid) {
                                    let index = *anim_counter % data.buffer.len();
                                    if let Some(c) =
                                        data.buffer[index].at(vec2i(x % tile_size, y % tile_size))
                                    {
                                        color = c;
                                    }
                                }
                            }
                        }
                    }

                    pixel.copy_from_slice(&color);
                }
            });

        let _stop = self.get_time();
        //println!("drawing time {:?}", _stop - start);
    }

    pub fn draw_tile(
        &self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        tile: Uuid,
        anim_counter: &usize,
        ctx: &mut TheContext,
    ) -> bool {
        if let Some(data) = self.tiles.get(&tile) {
            let x = (at.x * grid) as usize;
            let y = (at.y * grid) as usize;
            let stride = buffer.stride();
            ctx.draw.copy_slice(
                buffer.pixels_mut(),
                data.buffer[anim_counter % data.buffer.len()].pixels(),
                &(x, y, 24, 24),
                stride,
            );
            true
        } else {
            false
        }
    }

    pub fn draw_tile_outline(
        &self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        ctx: &mut TheContext,
    ) {
        let x = (at.x * grid) as usize;
        let y = (at.y * grid) as usize;
        let stride = buffer.stride();
        ctx.draw
            .rect_outline(buffer.pixels_mut(), &(x, y, 24, 24), stride, &WHITE);
    }

    /// Get the tile id of the given name.
    pub fn get_tile_id_by_name(&self, name: String) -> Option<Uuid> {
        for (id, tile) in &self.tiles {
            if tile.name == name {
                return Some(*id);
            }
        }
        None
    }

    /// Gets the current time in milliseconds
    fn get_time(&self) -> u128 {
        let time;
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            let t = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            time = t.as_millis();
        }
        #[cfg(target_arch = "wasm32")]
        {
            time = web_sys::window().unwrap().performance().unwrap().now() as u128;
        }
        time
    }
}
