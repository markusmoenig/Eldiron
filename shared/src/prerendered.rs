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
pub struct PreRendered {
    pub albedo: TheRGBABuffer,
    pub color: FxHashMap<(i32, i32), RGBA>,

    pub tiles_to_render: Vec<Vec2i>,

    pub tree: RTree<PreRenderedData>,
}

impl Default for PreRendered {
    fn default() -> Self {
        Self::zero()
    }
}

impl PreRendered {
    pub fn new(albedo: TheRGBABuffer) -> Self {
        Self {
            albedo,
            color: FxHashMap::default(),

            tiles_to_render: Vec::new(),

            tree: RTree::new(),
        }
    }

    pub fn zero() -> Self {
        Self {
            albedo: TheRGBABuffer::default(),
            color: FxHashMap::default(),

            tiles_to_render: Vec::new(),
            tree: RTree::new(),
        }
    }

    /// Add all tiles to be rendered.
    pub fn add_all_tiles(&mut self, grid_size: i32) {
        let mut tiles = Vec::new();
        let w = self.albedo.dim().width / grid_size;
        let h = self.albedo.dim().height / grid_size;
        for x in 0..w {
            for y in 0..h {
                let tile = Vec2i::new(x, y);
                tiles.push(tile);
            }
        }
        self.tiles_to_render = tiles;
    }

    /// Add the given tiles to be rendered in grid space, we map them to local space.
    pub fn add_tiles(&mut self, tiles: Vec<Vec2i>, grid_size: i32) {
        for t in tiles {
            let coord = [t.x as f32, t.y as f32];
            self.add_mapped_tile(coord, grid_size);
        }
    }

    /// Maps a tile to local camera space and adds the pixel region to be rendered.
    pub fn add_mapped_tile(&mut self, coord: [f32; 2], grid_size: i32) {
        if let Some(data) = self.tree.nearest_neighbor(&coord) {
            let local_tile = Vec2i::new(
                data.pixel_location.0 / grid_size,
                data.pixel_location.1 / grid_size,
            );
            if !self.tiles_to_render.contains(&local_tile) {
                self.tiles_to_render.push(local_tile);
            }

            let temp = vec2i(local_tile.x + 1, local_tile.y);
            if !self.tiles_to_render.contains(&temp) {
                self.tiles_to_render.push(temp);
            }

            let temp = vec2i(local_tile.x - 1, local_tile.y);
            if !self.tiles_to_render.contains(&temp) {
                self.tiles_to_render.push(temp);
            }

            let temp = vec2i(local_tile.x, local_tile.y + 1);
            if !self.tiles_to_render.contains(&temp) {
                self.tiles_to_render.push(temp);
            }

            let temp = vec2i(local_tile.x, local_tile.y - 1);
            if !self.tiles_to_render.contains(&temp) {
                self.tiles_to_render.push(temp);
            }
        }
    }
}
