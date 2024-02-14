use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

/// Settings for the region draw operation.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionDrawSettings {
    pub anim_counter: usize,
    pub delta_in_tick: f32,
    pub offset: Vec2i,
    pub delta: f32,

    pub center_on_character: Option<Uuid>,
}

#[allow(clippy::new_without_default)]
impl RegionDrawSettings {
    pub fn new() -> Self {
        Self {
            anim_counter: 0,
            delta_in_tick: 0.0,
            offset: Vec2i::zero(),
            delta: 0.0,
            center_on_character: None,
        }
    }
}

pub struct TileDrawer {
    pub tiles: FxHashMap<Uuid, TheRGBATile>,
    tiles_by_name: FxHashMap<String, TheRGBATile>,
}

#[allow(clippy::new_without_default)]
impl TileDrawer {
    pub fn new() -> Self {
        Self {
            tiles: FxHashMap::default(),
            tiles_by_name: FxHashMap::default(),
        }
    }

    /// Set the tiles.
    pub fn set_tiles(&mut self, tiles: FxHashMap<Uuid, TheRGBATile>) {
        self.tiles = tiles;
        self.tiles_by_name.clear();
        for tile in self.tiles.values() {
            self.tiles_by_name.insert(tile.name.clone(), tile.clone());
        }
    }

    /// Draw the region
    pub fn draw_region(
        &self,
        buffer: &mut TheRGBABuffer,
        region: &Region,
        update: &mut RegionUpdate,
        settings: &RegionDrawSettings
    ) -> Vec<(Vec2i, Uuid, Uuid)> {
        let _start = self.get_time();

        let server_tick = update.server_tick;

        let width = buffer.dim().width as usize;

        let region_width = (region.width * region.grid_size) as usize;
        let region_height = region.height * region.grid_size;

        let grid_size = region.grid_size as f32;
        let mut offset = settings.offset;

        // The pixel position of the characters with their tile id.
        let mut characters : Vec<(Vec2i, Uuid, Uuid)> = vec![];

        for (id, character) in &mut update.characters {
            let draw_pos = if let Some((start, end)) = &mut character.moving {
                // pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
                //     let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
                //     t * t * (3.0 - 2.0 * t)
                // }

                let sum = (settings.delta + character.move_delta).clamp(0.0, 1.0);
                // let d = smoothstep(0.0, 1.0, sum);//.clamp(0.0, 1.0);
                let d = sum;
                // let d = if sum < 0.5 {
                //     2.0 * sum * sum
                // } else {
                //     1.0 - (-2.0 * sum + 2.0).powi(2) / 2.0
                // };
                let x = start.x * (1.0 - d) + end.x * d;
                let y = start.y * (1.0 - d) + end.y * d;
                character.move_delta = sum;
                vec2i(
                    (x * grid_size).round() as i32,
                    (y * grid_size).round() as i32,
                )
            } else {
                vec2i(
                    (character.position.x * grid_size) as i32,
                    (character.position.y * grid_size) as i32,
                )
            };

            if Some(*id) == settings.center_on_character {
                let center_x = (buffer.dim().width as f32 / 2.0) as i32 - region.grid_size / 2;
                let center_y = (buffer.dim().height as f32 / 2.0) as i32 + region.grid_size / 2;
                offset.x += draw_pos.x - center_x;
                offset.y += region_height - (draw_pos.y + center_y);
            }

            if let Some(tile_id) = self.get_tile_id_by_name(character.tile_name.clone()) {
                characters.push((draw_pos, tile_id, character.id));
            }
        }

        let pixels = buffer.pixels_mut();
        pixels
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = (j + offset.y as usize) * region_width + i + offset.x as usize;

                    let x = (i % region_width) as i32;
                    let y = region_height - (i / region_width) as i32 - 1;

                    let tile_x = x / region.grid_size;
                    let tile_y = y / region.grid_size;

                    let mut xx = x % region.grid_size;
                    let mut yy = y % region.grid_size;

                    let mut color = BLACK;

                    if let Some(tile) = region.tiles.get(&(tile_x, tile_y)) {
                        for tile_index in 0..tile.layers.len() {
                            if let Some(tile_uuid) = tile.layers[tile_index] {
                                if let Some(data) = self.tiles.get(&tile_uuid) {
                                    let index = settings.anim_counter % data.buffer.len();

                                    if tile_index == Layer2DRole::Wall as usize {
                                        let mut alpha: f32 = 1.0;

                                        let mut valid = true;
                                        if let Some(wallfx) = update.wallfx.get(&(tile_x, tile_y)) {
                                            let d = (server_tick - wallfx.at_tick) as f32
                                                + settings.delta_in_tick
                                                - 1.0;
                                            if d < 1.0 {
                                                let t = (d * region.grid_size as f32) as i32;
                                                if wallfx.prev_fx != WallFX::Normal {
                                                    wallfx.prev_fx.apply(
                                                        &mut xx,
                                                        &mut yy,
                                                        &mut alpha,
                                                        &(region.grid_size - t),
                                                        &(1.0 - d),
                                                    );
                                                } else {
                                                    wallfx.fx.apply(
                                                        &mut xx, &mut yy, &mut alpha, &t, &d,
                                                    );
                                                }
                                            } else if wallfx.fx != WallFX::Normal {
                                                valid = false;
                                            }
                                        }

                                        if valid {
                                            if let Some(c) = data.buffer[index].at(vec2i(xx, yy)) {
                                                color = self.mix_color(
                                                    &color,
                                                    &c,
                                                    c[3] as f32 / 255.0 * alpha,
                                                );
                                            }
                                        }
                                    }
                                    else if let Some(c) =
                                        data.buffer[index].at(vec2i(xx, yy))
                                    {
                                        color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
                                    }
                                }
                            }
                        }
                    }

                    // Items
                    for item in update.items.values() {
                        if tile_x == item.position.x as i32 && tile_y == item.position.y as i32 {
                            if let Some(tile_uuid) =
                                self.get_tile_id_by_name(item.tile_name.clone())
                            {
                                if let Some(data) = self.tiles.get(&tile_uuid) {
                                    let index = settings.anim_counter % data.buffer.len();

                                    if let Some(c) =
                                        data.buffer[index].at(vec2i(xx, yy))
                                    {
                                        color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
                                    }
                                }
                            }
                        }
                    }


                    // Characters
                    for (pos, tile, _) in &characters {
                        if let Some(data) = self.tiles.get(tile) {
                            let index = settings.anim_counter % data.buffer.len();

                            let xx = x - pos.x;
                            let yy = y - pos.y;

                            if let Some(c) =
                                data.buffer[index].at(vec2i(xx, yy))
                            {
                                color = self.mix_color(&color, &c, c[3] as f32 / 255.0);
                            }
                        }
                    }

                    pixel.copy_from_slice(&color);
                }
            });

        let _stop = self.get_time();
        //println!("drawing time {:?}", _stop - start);

        characters
    }

    /*
    pub fn draw_tile(
        &self,
        at: Vec2i,
        buffer: &mut TheRGBABuffer,
        grid: i32,
        tile: Uuid,
        anim_counter: &usize,
    ) -> bool {
        if let Some(data) = self.tiles.get(&tile) {
            let x = (at.x * grid) as usize;
            let y = (at.y * grid) as usize;
            let stride = buffer.stride();
            self.blend_slice(
                buffer,
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
    ) -> bool {
        if let Some(data) = self.tiles.get(&tile) {
            let off = anim_counter % data.buffer.len();
            /*
            let stride = buffer.stride();
            self.blend_slice(
                buffer,
                data.buffer[off].pixels(),
                &(
                    x,
                    y,
                    data.buffer[off].dim().width as usize,
                    data.buffer[off].dim().height as usize,
                ),
                stride,
            );*/
            buffer.blend_into(at.x, at.y, &data.buffer[off]);
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
    }*/

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

    /// Mixes two colors.
    #[inline(always)]
    pub fn mix_color(&self, a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
        [
            (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
            (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
        ]
    }

    /// Blends rect from the source frame into the dest frame
    pub fn blend_slice(
        &self,
        buffer: &mut TheRGBABuffer,
        source: &[u8],
        rect: &(usize, usize, usize, usize),
        dest_stride: usize,
    ) {
        for y in 0..rect.3 {
            let d = rect.0 * 4 + (y + rect.1) * dest_stride * 4;
            let s = y * rect.2 * 4;

            for x in 0..rect.2 {
                let dd = d + x * 4;
                let ss = s + x * 4;

                if let Some(background) = buffer.at(vec2i(x as i32, y as i32)) {
                    let color = &[source[ss], source[ss + 1], source[ss + 2], source[ss + 3]];
                    buffer.pixels_mut()[dd..dd + 4].copy_from_slice(&self.mix_color(
                        &background,
                        color,
                        (color[3] as f32) / 255.0,
                    ));
                }
            }
        }
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
