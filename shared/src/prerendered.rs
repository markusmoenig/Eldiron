use core::f32;
//use rayon::prelude::*;
//use std::sync::atomic::{AtomicUsize, Ordering};
//use std::sync::Arc;

//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRenderedLight {
    pub pos: Vec2i,
    pub brdf: Vec3f,
}

fn default_grid_map() -> TheFlattenedMap<(half::f16, half::f16)> {
    TheFlattenedMap::new(0, 0)
}

fn default_prerendered_lights() -> TheFlattenedMap<Vec<PreRenderedLight>> {
    TheFlattenedMap::new(0, 0)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRendered {
    pub albedo: TheRGBBuffer,
    pub sky_absorption: TheRGBBuffer,
    pub distance: TheFlattenedMap<half::f16>,

    #[serde(default = "default_grid_map")]
    pub grid_map: TheFlattenedMap<(half::f16, half::f16)>,

    #[serde(default = "default_prerendered_lights")]
    pub lights: TheFlattenedMap<Vec<PreRenderedLight>>,

    #[serde(default)]
    #[serde(with = "vectorize")]
    pub tile_samples: FxHashMap<Vec2i, u16>,
}

impl Default for PreRendered {
    fn default() -> Self {
        Self::zero()
    }
}

impl PreRendered {
    pub fn new(albedo: TheRGBBuffer, sky_absorption: TheRGBBuffer) -> Self {
        Self {
            distance: TheFlattenedMap::new(albedo.dim().width, albedo.dim().height),
            lights: TheFlattenedMap::new(albedo.dim().width, albedo.dim().height),
            grid_map: TheFlattenedMap::new(albedo.dim().width, albedo.dim().height),

            albedo,
            sky_absorption,

            tile_samples: FxHashMap::default(),
        }
    }

    pub fn zero() -> Self {
        Self {
            albedo: TheRGBBuffer::default(),
            sky_absorption: TheRGBBuffer::default(),
            distance: TheFlattenedMap::new(0, 0),
            grid_map: TheFlattenedMap::new(0, 0),

            lights: TheFlattenedMap::new(0, 0),

            tile_samples: FxHashMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.albedo.fill([0, 0, 0]);
        self.sky_absorption.fill([0, 0, 0]);
        self.distance.clear();
        self.lights.clear();
        self.tile_samples.clear();
        //self.grid_map.clear();
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.albedo.resize(width, height);
        self.sky_absorption.resize(width, height);
        self.distance.resize(width, height);
        self.lights.resize(width, height);
        self.grid_map.resize(width, height);
    }

    pub fn invalidate(&mut self) {
        self.albedo.fill([0, 0, 0]);
        self.sky_absorption.fill([0, 0, 0]);
        self.distance.clear();
        self.lights.clear();
        //self.grid_map.clear();
    }

    /// Add the given tiles to be rendered in grid space, we map them to local space.
    pub fn remove_tiles(&mut self, tiles: Vec<Vec2i>, grid_size: i32) {
        for tile in tiles {
            if let Some(data) = self.get_pixel_coord(vec2f(tile.x as f32, tile.y as f32)) {
                let tile = Vec2i::new(data.x / grid_size, data.y / grid_size);
                for y in tile.y - 2..=tile.y + 2 {
                    for x in tile.x - 2..=tile.x + 2 {
                        let t = Vec2i::new(x, y);
                        self.tile_samples.remove(&t);
                    }
                }
            } else {
                println!("Could not map tile coord {tile}");
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_tile(
        &mut self,
        grid_size: i32,
        size: &Vec2i,
        tile: &Vec2i,
        sample: u16,
        albedo: &TheRGBBuffer,
        sky_absorption: &TheRGBBuffer,
        distance: &TheFlattenedMap<half::f16>,
        lights: &TheFlattenedMap<Vec<PreRenderedLight>>,
        grid_map: &TheFlattenedMap<(half::f16, half::f16)>,
    ) {
        self.resize(size.x, size.y);

        let tile_x = tile.x * grid_size;
        let tile_y = tile.y * grid_size;

        let s = 1.0 / (sample as f32 + 1.0);

        for h in 0..grid_size {
            for w in 0..grid_size {
                // albedo
                if let Some(existing) = self.albedo.at_vec3(vec2i(w + tile_x, h + tile_y)) {
                    if let Some(new_samp) = albedo.at_vec3(vec2i(w, h)) {
                        let p = lerp(existing, new_samp, s);
                        self.albedo.set_pixel_vec3f(w + tile_x, h + tile_y, &p);
                    }
                }

                // sky abso
                if let Some(existing) = self.sky_absorption.at_vec3(vec2i(w + tile_x, h + tile_y)) {
                    if let Some(new_samp) = sky_absorption.at_vec3(vec2i(w, h)) {
                        let p = lerp(existing, new_samp, s);
                        self.sky_absorption
                            .set_pixel_vec3f(w + tile_x, h + tile_y, &p);
                    }
                }

                // distance
                if let Some(new_samp) = distance.get((w, h)) {
                    if let Some(existing) = self.distance.get_mut((w + tile_x, h + tile_y)) {
                        let d = lerp(existing.to_f32(), new_samp.to_f32(), s);
                        *existing = half::f16::from_f32(d);
                    } else {
                        self.distance.set((w + tile_x, h + tile_y), *new_samp);
                    }
                }

                // lights
                if let Some(new_samp) = lights.get((w, h)) {
                    if let Some(existing) = self.lights.get_mut((w + tile_x, h + tile_y)) {
                        for nl in new_samp {
                            for ex in existing.iter_mut() {
                                if nl.pos == ex.pos {
                                    let e = ex.brdf;
                                    let n = nl.brdf;

                                    ex.brdf = lerp(e, n, s);
                                }
                            }
                        }
                    } else {
                        self.lights.set((w + tile_x, h + tile_y), new_samp.clone());
                    }
                }

                // gridmap
                // if let Some(new_samp) = grid_map.get((w, h)) {
                //     if let Some(existing) = self.grid_map.get_mut((w + tile_x, h + tile_y)) {
                //         let x = lerp(existing.0.to_f32(), new_samp.0.to_f32(), s);
                //         let y = lerp(existing.1.to_f32(), new_samp.1.to_f32(), s);
                //         *existing = (half::f16::from_f32(x), half::f16::from_f32(y));
                //     } else {
                //         self.grid_map.set((w + tile_x, h + tile_y), *new_samp);
                //     }
                // }

                if let Some(new_samp) = grid_map.get((w, h)) {
                    self.grid_map.set((w + tile_x, h + tile_y), *new_samp);
                }
            }
        }

        self.tile_samples.insert(*tile, sample);
    }

    // pub fn get_pixel_coord(&self, mut pos: Vec2f) -> Option<Vec2i> {
    //     pos.x = (pos.x * 1000.0).trunc() / 1000.0;
    //     pos.y = (pos.y * 1000.0).trunc() / 1000.0;

    //     let max_dist = Arc::new(AtomicUsize::new(f32::to_bits(f32::MAX) as usize));
    //     let res_x = Arc::new(AtomicUsize::new(usize::MAX));
    //     let res_y = Arc::new(AtomicUsize::new(usize::MAX));

    //     (0..self.grid_map.width).into_par_iter().for_each(|x| {
    //         for y in 0..self.grid_map.height {
    //             if let Some(tupe_f16) = self.grid_map.get((x, y)) {
    //                 let p = vec2f(tupe_f16.0.to_f32(), tupe_f16.1.to_f32());
    //                 let t = distance(pos, p);

    //                 if t < 0.005 {
    //                     res_x.store(x as usize, Ordering::SeqCst);
    //                     res_y.store(y as usize, Ordering::SeqCst);
    //                     max_dist.store(f32::to_bits(0.0) as usize, Ordering::SeqCst);
    //                     return;
    //                 }

    //                 let current_max_dist = f32::from_bits(max_dist.load(Ordering::SeqCst) as u32);
    //                 if t < current_max_dist {
    //                     res_x.store(x as usize, Ordering::SeqCst);
    //                     res_y.store(y as usize, Ordering::SeqCst);
    //                     max_dist.store(f32::to_bits(t) as usize, Ordering::SeqCst);
    //                 }
    //             }
    //         }
    //     });

    //     let x = res_x.load(Ordering::SeqCst);
    //     let y = res_y.load(Ordering::SeqCst);

    //     if (x == usize::MAX && y == usize::MAX)
    //         || f32::from_bits(max_dist.load(Ordering::SeqCst) as u32) > 5.0
    //     {
    //         None
    //     } else {
    //         Some(vec2i(x as i32, y as i32))
    //     }
    // }

    pub fn get_pixel_coord(&self, pos: Vec2f) -> Option<Vec2i> {
        let mut max_dist = f32::MAX;
        let mut res = None;

        for x in 0..self.grid_map.width {
            for y in 0..self.grid_map.height {
                if let Some(tupe_f16) = self.grid_map.get((x, y)) {
                    let p = vec2f(tupe_f16.0.to_f32(), tupe_f16.1.to_f32());
                    let t = distance(pos, p);

                    if t < 0.005 {
                        return Some(vec2i(x, y));
                    }

                    if t < max_dist {
                        res = Some(vec2i(x, y));
                        max_dist = t;
                    }
                }
            }
        }

        res
    }
}
