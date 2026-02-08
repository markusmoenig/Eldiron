//use rayon::prelude::*;
//use std::sync::atomic::{AtomicUsize, Ordering};
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
    pub sunlight: TheRGBBuffer,
    pub distance: TheFlattenedMap<half::f16>,
    // pub occlusion: TheFlattenedMap<half::f16>,

    // pub mat_type: TheFlattenedMap<u8>,
    // pub mat1: TheFlattenedMap<half::f16>,
    // pub mat2: TheFlattenedMap<half::f16>,
    // pub mat3: TheFlattenedMap<half::f16>,
    // pub normal: TheFlattenedMap<(half::f16, half::f16, half::f16)>,
    pub lights: TheFlattenedMap<Vec<PreRenderedLight>>,
}

impl PreRenderedTileData {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            albedo: TheRGBBuffer::new(TheDim::sized(width, height)),
            sunlight: TheRGBBuffer::new(TheDim::sized(width, height)),
            distance: TheFlattenedMap::new(width, height),
            // occlusion: TheFlattenedMap::new(width, height),
            // mat_type: TheFlattenedMap::new(width, height),
            // mat1: TheFlattenedMap::new(width, height),
            // mat2: TheFlattenedMap::new(width, height),
            // mat3: TheFlattenedMap::new(width, height),
            // normal: TheFlattenedMap::new(width, height),
            lights: TheFlattenedMap::new(width, height),
        }
    }

    pub fn set_albedo(&mut self, x: i32, y: i32, color: Vec3f) {
        self.albedo.set_pixel_vec3f(x, y, &color);
    }

    pub fn set_distance(&mut self, x: i32, y: i32, distance: f32) {
        self.distance.set((x, y), half::f16::from_f32(distance));
    }

    // pub fn set_occlusion(&mut self, x: i32, y: i32, occ: f32) {
    //     self.occlusion.set((x, y), half::f16::from_f32(occ));
    // }

    // pub fn set_material(&mut self, x: i32, y: i32, mat_type: u8, mat1: f32, mat2: f32, mat3: f32) {
    //     self.mat_type.set((x, y), mat_type);
    //     self.mat1.set((x, y), half::f16::from_f32(mat1));
    //     self.mat2.set((x, y), half::f16::from_f32(mat2));
    //     self.mat3.set((x, y), half::f16::from_f32(mat3));
    // }

    // pub fn set_normal(&mut self, x: i32, y: i32, normal: Vec3f) {
    //     self.normal.set(
    //         (x, y),
    //         (
    //             half::f16::from_f32(normal.x),
    //             half::f16::from_f32(normal.y),
    //             half::f16::from_f32(normal.z),
    //         ),
    //     );
    // }
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
    }

    /// Add the given tiles to be rendered in grid space, we map them to local space.
    pub fn remove_tiles(&mut self, tiles: &Vec<Vec2i>) {
        for tile in tiles {
            for y in tile.y - 2..=tile.y + 2 {
                for x in tile.x - 2..=tile.x + 2 {
                    let t = Vec2i::new(x, y);
                    self.tile_samples.remove(&t);
                }
            }
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

                    // sunlight
                    if let Some(existing) = existing_tile.sunlight.at_vec3(vec2i(w, h)) {
                        if let Some(new_samp) = tile_data.sunlight.at_vec3(vec2i(w, h)) {
                            let p = lerp(existing, new_samp, s);
                            existing_tile.sunlight.set_pixel_vec3f(w, h, &p);
                        }
                    }

                    if sample == 0 {
                        // distance
                        if let Some(sample) = tile_data.distance.get((w, h)) {
                            existing_tile.distance.set((w, h), *sample);
                        }

                        /*
                        // occlusion
                        if let Some(sample) = tile_data.occlusion.get((w, h)) {
                            existing_tile.occlusion.set((w, h), *sample);
                        }

                        // material
                        if let Some(sample) = tile_data.mat_type.get((w, h)) {
                            existing_tile.mat_type.set((w, h), *sample);
                        }
                        if let Some(sample) = tile_data.mat1.get((w, h)) {
                            existing_tile.mat1.set((w, h), *sample);
                        }
                        if let Some(sample) = tile_data.mat2.get((w, h)) {
                            existing_tile.mat2.set((w, h), *sample);
                        }
                        if let Some(sample) = tile_data.mat3.get((w, h)) {
                            existing_tile.mat3.set((w, h), *sample);
                        }

                        // normal
                        if let Some(sample) = tile_data.normal.get((w, h)) {
                            existing_tile.normal.set((w, h), *sample);
                        }*/
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

    /// Clear the color for the given tile.
    pub fn clear_tile_albedo(&mut self, tile: &Vec2i) {
        if let Some(existing_tile) = self.tiles.get_mut(tile) {
            existing_tile.albedo.fill([0, 0, 0]);
        }
        self.tile_samples.insert(*tile, u16::MAX);
    }
}
