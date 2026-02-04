use crate::{
    Assets, BBox, Chunk, Map, Pixel, PixelSource, Ray, TerrainBlendMode, TerrainChunk, Texture,
};
use rayon::prelude::*;
use theframework::prelude::*;
use vek::Vec2;

const CHUNKSIZE: i32 = 16;

#[derive(Clone, Debug)]
pub struct TerrainHit {
    pub world_pos: Vec3<f32>,
    pub grid_pos: Vec2<i32>,
    pub height: f32,
}

pub mod chunk;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Terrain {
    pub scale: Vec2<f32>, // world units per tile
    pub chunk_size: i32,  // number of tiles per chunk
    #[serde(with = "vectorize")]
    pub chunks: FxHashMap<(i32, i32), TerrainChunk>,
}

impl Terrain {
    /// Creates an empty terrain instance
    pub fn empty() -> Self {
        Self {
            scale: Vec2::one(),
            chunk_size: CHUNKSIZE,
            chunks: FxHashMap::default(),
        }
    }

    /// Returns the coordinates for the chunk at the given world pos
    pub fn get_chunk_coords(&self, x: i32, y: i32) -> (i32, i32) {
        (x.div_euclid(self.chunk_size), y.div_euclid(self.chunk_size))
    }

    /// Gets the chunk at the given coords (or create it)
    fn get_or_create_chunk(&mut self, x: i32, y: i32) -> &mut TerrainChunk {
        let coords = self.get_chunk_coords(x, y);
        self.chunks.entry(coords).or_insert_with(|| {
            TerrainChunk::new(
                Vec2::new(coords.0, coords.1) * self.chunk_size,
                self.chunk_size,
            )
        })
    }

    /// Get the unprocessed height at the given world coordinate
    pub fn get_height_unprocessed(&self, x: i32, y: i32) -> Option<f32> {
        let chunk_coords = self.get_chunk_coords(x, y);
        self.chunks
            .get(&chunk_coords)
            .and_then(|chunk| chunk.get_height_unprocessed(x, y))
    }

    /// Smooth steepness estimate using bilinear sampling.
    /// Returns normalized steepness in [0.0, 1.0], where
    /// 0.0 = flat (0°), 1.0 = vertical (90°)
    pub fn compute_steepness(&self, world_pos: Vec2<f32>) -> f32 {
        const EPSILON: f32 = 0.5;

        let h = self.sample_height_bilinear(world_pos.x, world_pos.y);
        let h_x = self.sample_height_bilinear(world_pos.x + EPSILON, world_pos.y);
        let h_y = self.sample_height_bilinear(world_pos.x, world_pos.y + EPSILON);

        let dx = (h_x - h) / self.scale.x;
        let dy = (h_y - h) / self.scale.y;

        let normal = Vec3::new(-dx, 1.0, -dy).normalized();
        let dot = normal.dot(Vec3::unit_y()).clamp(-1.0, 1.0);

        let angle_degrees = dot.acos().to_degrees(); // 0° (flat) to 90° (vertical)
        (angle_degrees / 90.0).clamp(0.0, 1.0)
    }

    /// Get height at given cell
    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        let chunk_coords = self.get_chunk_coords(x, y);
        if let Some(chunk) = self.chunks.get(&chunk_coords) {
            chunk.get_height(x, y)
        } else {
            0.0
        }
    }

    /// Set height at given cell
    pub fn set_height(&mut self, x: i32, y: i32, height: f32) {
        let chunk = self.get_or_create_chunk(x, y);
        chunk.set_height(x, y, height);
    }

    /// Remove height at given cell
    pub fn remove_height(&mut self, x: i32, y: i32) {
        let coords = self.get_chunk_coords(x, y);
        if let Some(chunk) = self.chunks.get_mut(&coords) {
            let world = Vec2::new(x, y);
            let local = world - chunk.origin;
            chunk.heights.remove(&(local.x, local.y));
            chunk.mark_dirty();

            // If chunk is now completely empty, remove it
            if chunk.heights.is_empty() && chunk.sources.is_empty() && chunk.blend_modes.is_empty()
            {
                self.chunks.remove(&coords);
            }
        }
    }

    // Get the blend mode at the given cell
    pub fn get_blend_mode(&self, x: i32, y: i32) -> TerrainBlendMode {
        let chunk_coords = self.get_chunk_coords(x, y);
        if let Some(chunk) = self.chunks.get(&chunk_coords) {
            let local = Vec2::new(x, y) - chunk.origin;
            if let Some(mode) = chunk.blend_modes.get(&(local.x, local.y)) {
                return *mode;
            }
        }
        TerrainBlendMode::None
    }

    /// Set blend mode at given cell
    pub fn set_blend_mode(&mut self, x: i32, y: i32, mode: TerrainBlendMode) {
        let chunk = self.get_or_create_chunk(x, y);
        chunk.set_blend_mode(x, y, mode);
    }

    /// Set source material at given cell
    pub fn set_source(&mut self, x: i32, y: i32, source: PixelSource) {
        let chunk = self.get_or_create_chunk(x, y);
        chunk.set_source(x, y, source);
        //self.mark_neighbors_dirty(x, y);
    }

    /// Get source material at given cell
    pub fn get_source(&self, x: i32, y: i32) -> Option<&PixelSource> {
        let chunk_coords = self.get_chunk_coords(x, y);
        self.chunks
            .get(&(chunk_coords.0, chunk_coords.1))
            .and_then(|chunk| chunk.get_source(x, y))
    }

    /// Sample height at a world position (nearest neighbor)
    pub fn sample_height(&self, x: f32, y: f32) -> f32 {
        let xi = x.round() as i32;
        let yi = y.round() as i32;
        self.get_height(xi, yi)
    }

    /// Bilinearly interpolate the height at a world position (x, y)
    pub fn sample_height_bilinear(&self, x: f32, y: f32) -> f32 {
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;

        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let h00 = self.get_height(x0, y0);
        let h10 = self.get_height(x1, y0);
        let h01 = self.get_height(x0, y1);
        let h11 = self.get_height(x1, y1);

        let h0 = h00 * (1.0 - tx) + h10 * tx;
        let h1 = h01 * (1.0 - tx) + h11 * tx;

        h0 * (1.0 - ty) + h1 * ty
    }

    /// Computes the bounding box of the heightmap
    pub fn compute_bounds(&self) -> Option<BBox> {
        let mut min = Vec2::new(i32::MAX, i32::MAX);
        let mut max = Vec2::new(i32::MIN, i32::MIN);

        for chunk in self.chunks.values() {
            let origin = chunk.origin;

            min.x = min.x.min(origin.x);
            min.y = min.y.min(origin.y);
            max.x = max.x.max(origin.x + self.chunk_size - 1);
            max.y = max.y.max(origin.y + self.chunk_size - 1);
        }

        if min.x <= max.x && min.y <= max.y {
            Some(BBox::new(min.map(|v| v as f32), max.map(|v| v as f32)))
        } else {
            None
        }
    }

    /// Sample the pixel source at the given world position
    pub fn sample_source(&self, world_pos: Vec2<f32>, assets: &Assets) -> (Pixel, bool) {
        // Map world position to tile grid position
        let x = (world_pos.x / self.scale.x).floor() as i32;
        let y = (world_pos.y / self.scale.y).floor() as i32;

        // Local UV inside the tile (0..1)
        let local_x = (world_pos.x / self.scale.x).fract();
        let local_y = (world_pos.y / self.scale.y).fract();
        let uv = Vec2::new(
            if local_x < 0.0 {
                local_x + 1.0
            } else {
                local_x
            },
            if local_y < 0.0 {
                local_y + 1.0
            } else {
                local_y
            },
        );

        if let Some(source) = self.get_source(x, y) {
            match source {
                PixelSource::TileId(id) => {
                    if let Some(tile) = assets.tiles.get(id) {
                        if let Some(texture) = tile.textures.first() {
                            return (texture.sample_nearest(uv.x, uv.y), true);
                        }
                    }
                }
                PixelSource::MaterialId(id) => {
                    if let Some(index) = assets.tile_indices.get(id) {
                        if let Some(texture) = assets.tile_list[*index as usize].textures.first() {
                            return (texture.sample_nearest(uv.x, uv.y), true);
                        }
                    }
                }
                _ => {}
            }
        }

        // Checkerboard fallback based on tile position
        let checker = ((x & 1) ^ (y & 1)) == 0;
        if checker {
            ([135, 135, 135, 255], false)
        } else {
            ([120, 120, 120, 255], false)
        }
    }

    pub fn sample_source_blended_radius(
        &self,
        world_pos: Vec2<f32>,
        assets: &Assets,
        radius: f32,
    ) -> Pixel {
        let mut sum = Vec3::zero();
        let mut weight_sum = 0.0;

        let step = self.scale.x.min(self.scale.y) * 0.5;
        let radius_squared = radius * radius;
        let steps = (radius / step).ceil() as i32;

        for dy in -steps..=steps {
            for dx in -steps..=steps {
                let offset = Vec2::new(dx as f32 * step, dy as f32 * step);
                let dist2 = offset.magnitude_squared();
                if dist2 > radius_squared {
                    continue;
                }

                let sample_pos = world_pos + offset;
                let (pixel, valid) = self.sample_source(sample_pos, assets);
                if valid {
                    let t = 1.0 - (dist2 / radius_squared);
                    let weight = t * t;

                    sum += Vec3::new(pixel[0] as f32, pixel[1] as f32, pixel[2] as f32) * weight;
                    weight_sum += weight;
                }
            }
        }

        if weight_sum > 0.0 {
            let avg = sum / weight_sum;
            [
                avg.x.round() as u8,
                avg.y.round() as u8,
                avg.z.round() as u8,
                255,
            ]
        } else {
            // fallback: checker pattern
            let x = (world_pos.x / self.scale.x).floor() as i32;
            let y = (world_pos.y / self.scale.y).floor() as i32;
            if ((x ^ y) & 1) == 0 {
                [120, 120, 120, 255]
            } else {
                [135, 135, 135, 255]
            }
        }
    }

    /// Approximate the normal at a world position by sampling neighboring heights
    pub fn sample_normal(&self, world_pos: Vec2<f32>) -> Vec3<f32> {
        const EPSILON: f32 = 0.5; // Fixed sampling distance

        let h_l = self.sample_height(world_pos.x - EPSILON, world_pos.y);
        let h_r = self.sample_height(world_pos.x + EPSILON, world_pos.y);
        let h_d = self.sample_height(world_pos.x, world_pos.y - EPSILON);
        let h_u = self.sample_height(world_pos.x, world_pos.y + EPSILON);

        Vec3::new(
            (h_l - h_r) * 0.5 / EPSILON,
            1.0,
            (h_d - h_u) * 0.5 / EPSILON,
        )
        .normalized()
    }

    /// Bake an individual chunk
    pub fn bake_chunk(
        &self,
        chunk_coords: &Vec2<i32>,
        assets: &Assets,
        pixels_per_tile: i32,
    ) -> Texture {
        let chunk_min_tile = *chunk_coords * self.chunk_size;

        let chunk_tex_width = self.chunk_size * pixels_per_tile;
        let chunk_tex_height = self.chunk_size * pixels_per_tile;

        let mut pixels = vec![0u8; (chunk_tex_width * chunk_tex_height * 4) as usize];

        pixels
            .par_chunks_exact_mut((chunk_tex_width * 4) as usize)
            .enumerate()
            .for_each(|(y, line)| {
                for (x, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let tile_x = chunk_min_tile.x as f32 + (x as f32 / pixels_per_tile as f32);
                    let tile_y = chunk_min_tile.y as f32 + (y as f32 / pixels_per_tile as f32);

                    let world_x = tile_x * self.scale.x;
                    let world_y = tile_y * self.scale.y;
                    let world_pos = Vec2::new(world_x, world_y);

                    let tile_pos = Vec2::new(tile_x.floor() as i32, tile_y.floor() as i32);
                    let blend_mode = self.get_blend_mode(tile_pos.x, tile_pos.y);

                    let color = match blend_mode {
                        TerrainBlendMode::None => self.sample_source(world_pos, assets).0,
                        TerrainBlendMode::Blend(radius) => {
                            self.sample_source_blended_radius(world_pos, assets, radius as f32)
                        }
                        TerrainBlendMode::BlendOffset(radius, offset) => self
                            .sample_source_blended_radius(
                                world_pos + offset,
                                assets,
                                radius as f32,
                            ),
                        TerrainBlendMode::Custom(radius, _, offset) => self
                            .sample_source_blended_radius(
                                world_pos + offset,
                                assets,
                                radius as f32,
                            ),
                    };
                    pixel.copy_from_slice(&color);
                }
            });

        Texture::new(pixels, chunk_tex_width as usize, chunk_tex_height as usize)
    }

    /// Build the chunk at the given coordinate.
    pub fn build_chunk_at(
        &mut self,
        coord: (i32, i32),
        assets: &Assets,
        map: &Map,
        pixels_per_tile: i32,
        chunk: &mut Chunk,
        modifiers: bool,
    ) {
        let mut baked = self.bake_chunk(&Vec2::new(coord.0, coord.1), assets, pixels_per_tile);

        let processed_heights = if let Some(chunk_mut) = self.chunks.get(&coord) {
            if modifiers {
                chunk_mut.process_batch_modifiers(self, map, assets, &mut baked)
            } else {
                chunk_mut.heights.clone()
            }
        } else {
            return;
        };

        if let Some(chunk_mut) = self.chunks.get_mut(&coord) {
            chunk_mut.processed_heights = Some(processed_heights);
            chunk_mut.clear_dirty();

            chunk.terrain_texture = Some(baked);
        }
    }

    /// Counts dirty chunks
    pub fn count_dirty_chunks(&self) -> i32 {
        let mut dirty = 0;
        for chunk in self.chunks.values() {
            if chunk.dirty {
                dirty += 1;
            }
        }
        dirty
    }

    /// Mark all chunks clean
    pub fn mark_clean(&mut self) {
        for chunk in self.chunks.values_mut() {
            chunk.clear_dirty();
        }
    }

    /// Mark all chunks dirty
    pub fn mark_dirty(&mut self) {
        for chunk in self.chunks.values_mut() {
            chunk.mark_dirty();
        }
    }

    /// Ray / terrain hit used for editing
    pub fn ray_terrain_hit(&self, ray: &Ray, max_distance: f32) -> Option<TerrainHit> {
        let mut t = 0.0;
        let step_size = 0.1;

        for _ in 0..1500 {
            let point = ray.origin + ray.dir * t;
            let world_pos = Vec2::new(point.x, point.z);
            let terrain_height = self.sample_height(world_pos.x, world_pos.y);

            if point.y - terrain_height < 0.01 {
                // Detected a hit; refine using binary search between previous and current t
                let t_prev = (t - step_size).max(0.0); // Ensure t_prev isn't negative
                let mut low = t_prev;
                let mut high = t;

                // Perform binary search for higher accuracy
                for _ in 0..4 {
                    let mid = (low + high) * 0.5;
                    let point_mid = ray.origin + ray.dir * mid;
                    let terrain_mid_height = self.sample_height_bilinear(point_mid.x, point_mid.z);
                    if point_mid.y - terrain_mid_height < 0.01 {
                        high = mid; // Intersection is in the lower half
                    } else {
                        low = mid; // Intersection is in the upper half
                    }
                }

                // Final refined t is the midpoint after binary search
                let t_hit = (low + high) * 0.5;
                let hit_point = ray.origin + ray.dir * t_hit;
                let world_pos_hit = Vec2::new(hit_point.x, hit_point.z);
                let terrain_hit_height =
                    self.sample_height_bilinear(world_pos_hit.x, world_pos_hit.y);

                // Snap hit point to terrain to avoid floating inaccuracies
                let final_hit_point = Vec3::new(hit_point.x, terrain_hit_height, hit_point.z);
                let grid_x = (final_hit_point.x / self.scale.x).floor() as i32;
                let grid_y = (final_hit_point.z / self.scale.y).floor() as i32;

                return Some(TerrainHit {
                    world_pos: final_hit_point,
                    grid_pos: Vec2::new(grid_x, grid_y),
                    height: terrain_hit_height,
                });
            }

            t += step_size;
            if t > max_distance {
                break;
            }
        }
        None
    }

    /// Returns true if a height value exists at (x, y)
    pub fn exists(&self, x: i32, y: i32) -> bool {
        let chunk_coords = self.get_chunk_coords(x, y);
        if let Some(chunk) = self.chunks.get(&chunk_coords) {
            chunk.exists(x, y)
        } else {
            false
        }
    }

    /// Mark the chunk at (x,y) and all neighboring chunks dirty
    fn _mark_neighbors_dirty(&mut self, x: i32, y: i32) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let coords = self.get_chunk_coords(x + dx, y + dy);
                if let Some(chunk) = self.chunks.get_mut(&coords) {
                    chunk.mark_dirty();
                }
            }
        }
    }

    /// Returns a cleaned clone of the chunks (used for undo / redo)
    pub fn clone_chunks(&self) -> FxHashMap<(i32, i32), TerrainChunk> {
        self.chunks.clone()
    }

    /// Returns a clone of all dirty chunks
    pub fn clone_dirty_chunks(&self) -> Vec<TerrainChunk> {
        self.chunks
            .values()
            .filter(|chunk| chunk.dirty)
            .cloned()
            .collect()
    }

    /// Iterator
    #[allow(mismatched_lifetime_syntaxes)]
    pub fn iter_tiles_mut(&mut self) -> TerrainTileIterMut {
        TerrainTileIterMut {
            chunks: self.chunks.iter_mut(),
            current_chunk: None,
            x: 0,
            y: 0,
            scale: self.scale,
            chunk_size: self.chunk_size,
        }
    }
}

impl Default for Terrain {
    fn default() -> Self {
        Self::empty()
    }
}

pub struct TerrainTileRefMut {
    pub chunk_coords: (i32, i32),
    pub local_coords: (usize, usize),
    pub world_coords: (f32, f32),
    pub chunk: *mut TerrainChunk, // raw pointer due to Rust borrowing rules
}

pub struct TerrainTileIterMut<'a> {
    chunks: std::collections::hash_map::IterMut<'a, (i32, i32), TerrainChunk>,
    current_chunk: Option<((i32, i32), *mut TerrainChunk)>,
    x: usize,
    y: usize,
    scale: Vec2<f32>,
    chunk_size: i32,
}

impl<'a> Iterator for TerrainTileIterMut<'a> {
    type Item = TerrainTileRefMut;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((chunk_coords, chunk_ptr)) = self.current_chunk {
                let size = self.chunk_size as usize;

                if self.y < size {
                    let local_coords = (self.x, self.y);

                    unsafe {
                        let origin = (*chunk_ptr).origin;
                        let world_x = (origin.x + self.x as i32) as f32 * self.scale.x;
                        let world_y = (origin.y + self.y as i32) as f32 * self.scale.y;

                        let result = TerrainTileRefMut {
                            chunk_coords,
                            local_coords,
                            world_coords: (world_x, world_y),
                            chunk: chunk_ptr,
                        };

                        self.x += 1;
                        if self.x >= size {
                            self.x = 0;
                            self.y += 1;
                        }

                        return Some(result);
                    }
                } else {
                    // Finished this chunk, move to next
                    self.current_chunk = None;
                    self.y = 0;
                }
            }

            match self.chunks.next() {
                Some((chunk_coords, chunk)) => {
                    self.current_chunk = Some((*chunk_coords, chunk as *mut _));
                    self.x = 0;
                    self.y = 0;
                }
                None => return None,
            }
        }
    }
}
