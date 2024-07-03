//use crate::prelude::*;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct PreRenderedData {
    pub location: (f32, f32),
    pub pixel_location: (i32, i32),
}

impl PointDistance for PreRenderedData {
    // Calculate the squared distance to a point
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        let dx = self.location.0 - point[0];
        let dy = self.location.1 - point[1];
        dx * dx + dy * dy
    }

    // This optional method improves performance by eliminating objects quickly from consideration
    fn contains_point(&self, point: &[f32; 2]) -> bool {
        self.location.0 == point[0] && self.location.1 == point[1]
    }
}

impl RTreeObject for PreRenderedData {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point([self.location.0, self.location.1])
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRenderedLight {
    pub pos: Vec2i,
    pub brdf: Vec3f,
}

fn default_prerendered_lights() -> TheFlattenedMap<Vec<PreRenderedLight>> {
    TheFlattenedMap::new(0, 0)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreRendered {
    pub albedo: TheRGBBuffer,
    pub sky_absorption: TheRGBBuffer,
    pub distance: TheFlattenedMap<f32>,

    #[serde(default = "default_prerendered_lights")]
    pub lights: TheFlattenedMap<Vec<PreRenderedLight>>,

    #[serde(skip)]
    pub tree: RTree<PreRenderedData>,

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

            albedo,
            sky_absorption,

            tree: RTree::new(),

            tile_samples: FxHashMap::default(),
        }
    }

    pub fn zero() -> Self {
        Self {
            albedo: TheRGBBuffer::default(),
            sky_absorption: TheRGBBuffer::default(),
            distance: TheFlattenedMap::new(0, 0),

            lights: TheFlattenedMap::new(0, 0),

            tree: RTree::new(),

            tile_samples: FxHashMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.albedo.fill([0, 0, 0]);
        self.sky_absorption.fill([0, 0, 0]);
        self.distance.clear();
        self.lights.clear();
        self.tile_samples.clear();
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        self.albedo.resize(width, height);
        self.sky_absorption.resize(width, height);
        self.distance.resize(width, height);
        self.lights.resize(width, height);
    }

    pub fn invalidate(&mut self) {
        self.albedo.fill([0, 0, 0]);
        self.sky_absorption.fill([0, 0, 0]);
        self.distance.clear();
        self.lights.clear();
    }

    /// Add the given tiles to be rendered in grid space, we map them to local space.
    pub fn remove_tiles(&mut self, tiles: Vec<Vec2i>, grid_size: i32) {
        for tile in tiles {
            if let Some(data) = self.tree.nearest_neighbor(&[tile.x as f32, tile.y as f32]) {
                let tile = Vec2i::new(
                    data.pixel_location.0 / grid_size,
                    data.pixel_location.1 / grid_size,
                );

                for y in tile.y - 2..=tile.y + 2 {
                    for x in tile.x - 2..=tile.x + 2 {
                        let t = Vec2i::new(x, y);
                        self.tile_samples.remove(&t);
                    }
                }
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
        distance: &TheFlattenedMap<f32>,
        lights: &TheFlattenedMap<Vec<PreRenderedLight>>,
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
                        let d = lerp(*existing, *new_samp, s);
                        *existing = d;
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
            }
        }

        self.tile_samples.insert(*tile, sample);
    }
}
