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

        let tile_size = region.grid_size;

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
                                        color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
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
            ctx.draw.blend_slice(
                buffer.pixels_mut(),
                data.buffer[anim_counter % data.buffer.len()].pixels(),
                &(x, y, grid as usize, grid as usize),
                stride,
            );
            true
        } else {
            false
        }
    }

    pub fn draw_tile_at_pixel(
        &self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        tile: Uuid,
        anim_counter: &usize,
        ctx: &mut TheContext,
    ) -> bool {
        if let Some(data) = self.tiles.get(&tile) {
            let off = anim_counter % data.buffer.len();
            let x = at.x as usize;
            let y = at.y as usize;
            let stride = buffer.stride();
            ctx.draw.blend_slice(
                buffer.pixels_mut(),
                data.buffer[off].pixels(),
                &(
                    x,
                    y,
                    data.buffer[off].dim().width as usize,
                    data.buffer[off].dim().height as usize,
                ),
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
        color: [u8; 4],
        ctx: &mut TheContext,
    ) {
        let x = (at.x * grid) as usize;
        let y = (at.y * grid) as usize;
        let stride = buffer.stride();
        ctx.draw
            .rect_outline(buffer.pixels_mut(), &(x, y, 24, 24), stride, &color);
    }

    pub fn draw_tile_outline_at_pixel(
        &self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        color: [u8; 4],
        ctx: &mut TheContext,
    ) {
        let x = at.x as usize;
        let y = at.y as usize;
        let stride = buffer.stride();
        ctx.draw
            .rect_outline(buffer.pixels_mut(), &(x, y, 24, 24), stride, &color);
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

    pub fn draw_tile_selection(
        &self,
        tiles: &FxHashSet<(i32, i32)>,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        color: [u8; 4],
        ctx: &mut TheContext,
    ) {
        for t in tiles {
            let x = (t.0 * grid) as usize;
            let y = (t.1 * grid) as usize;
            let stride = buffer.stride();
            ctx.draw
                .rect_outline(buffer.pixels_mut(), &(x, y, 24, 24), stride, &color);
        }
    }

    /// Mixes the two colors together.
    #[inline(always)]
    pub fn mix_color(&self, a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
        [
            (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
        ]
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
