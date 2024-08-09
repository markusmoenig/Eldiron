//use rayon::prelude::*;
//use std::sync::atomic::{AtomicUsize, Ordering};
//use std::sync::Arc;

use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRenderedLight {
    pub pos: Vec2i,
    pub brdf: (half::f16, half::f16, half::f16),
}

/// Contains the data for a prerendered tile.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRenderedTileData {
    pub albedo: TheRGBBuffer,
    pub distance: TheFlattenedMap<half::f16>,

    pub lights: TheFlattenedMap<Vec<PreRenderedLight>>,
}

impl PreRenderedTileData {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            albedo: TheRGBBuffer::new(TheDim::sized(width, height)),
            distance: TheFlattenedMap::new(width, height),
            lights: TheFlattenedMap::new(width, height),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRendered {
    #[serde(default)]
    #[serde(with = "vectorize")]
    pub tiles: FxHashMap<Vec2i, PreRenderedTileData>,

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
    pub fn zero() -> Self {
        Self {
            tiles: FxHashMap::default(),
            tile_samples: FxHashMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.tiles.clear();
        self.tile_samples.clear();
        //self.grid_map.clear();
    }

    pub fn invalidate(&mut self) {
        //self.grid_map.clear();
    }

    /// Add the given tiles to be rendered in grid space, we map them to local space.
    pub fn remove_tiles(&mut self, region: &Region, tiles: Vec<Vec2i>) {
        for tile in tiles {
            let data = region
                .regionfx
                .cam_world_to_canvas(region, vec3f(tile.x as f32, 0.0, tile.y as f32));
            println!("tile {:?} {:?}", data, tile);
            let tile = Vec2i::new(data.x / region.tile_size, data.y / region.tile_size);
            for y in tile.y - 2..=tile.y + 2 {
                for x in tile.x - 2..=tile.x + 2 {
                    let t = Vec2i::new(x, y);
                    self.tile_samples.remove(&t);
                }
            }
            /*
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
            }*/
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn merge_tile_data(
        &mut self,
        grid_size: i32,
        tile: &Vec2i,
        sample: u16,
        tile_data: &PreRenderedTileData,
    ) {
        // let tile_x = tile.x * grid_size;
        // let tile_y = tile.y * grid_size;

        let s = 1.0 / (sample as f32 + 1.0);

        if let Some(existing_tile) = self.tiles.get_mut(tile) {
            for h in 0..grid_size {
                for w in 0..grid_size {
                    // albedo
                    if let Some(existing) = existing_tile.albedo.at_vec3(vec2i(w, h)) {
                        if let Some(new_samp) = tile_data.albedo.at_vec3(vec2i(w, h)) {
                            let p = lerp(existing, new_samp, s);
                            existing_tile.albedo.set_pixel_vec3f(w, h, &p);
                        }
                    }

                    // distance
                    if let Some(new_samp) = tile_data.distance.get((w, h)) {
                        if let Some(existing) = existing_tile.distance.get_mut((w, h)) {
                            let d = lerp(existing.to_f32(), new_samp.to_f32(), s);
                            *existing = half::f16::from_f32(d);
                        } else {
                            existing_tile.distance.set((w, h), *new_samp);
                        }
                    }

                    // lights
                    if let Some(new_samp) = tile_data.lights.get((w, h)) {
                        if let Some(existing) = existing_tile.lights.get_mut((w, h)) {
                            for nl in new_samp {
                                for ex in existing.iter_mut() {
                                    if nl.pos == ex.pos {
                                        let e = vec3f(
                                            ex.brdf.0.to_f32(),
                                            ex.brdf.1.to_f32(),
                                            ex.brdf.2.to_f32(),
                                        );
                                        let n = vec3f(
                                            nl.brdf.0.to_f32(),
                                            nl.brdf.1.to_f32(),
                                            nl.brdf.2.to_f32(),
                                        );

                                        let rc = lerp(e, n, s);
                                        ex.brdf = (
                                            half::f16::from_f32(rc.x),
                                            half::f16::from_f32(rc.y),
                                            half::f16::from_f32(rc.z),
                                        );
                                    }
                                }
                            }
                        } else {
                            existing_tile.lights.set((w, h), new_samp.clone());
                        }
                    }
                }
            }
        } else {
            self.tiles.insert(*tile, tile_data.clone());
        }

        self.tile_samples.insert(*tile, sample);
    }
}
