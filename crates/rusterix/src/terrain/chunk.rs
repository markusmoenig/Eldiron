use crate::Terrain;
use crate::{
    Assets, BBox, Batch2D, Batch3D, Linedef, Map, PixelSource, ShapeFXModifierPass, Texture, Value,
};
use theframework::prelude::*;
use vek::Vec2;

fn default_size() -> i32 {
    16
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub enum TerrainBlendMode {
    None,
    Blend(u8),
    BlendOffset(u8, Vec2<f32>),
    Custom(u8, u8, Vec2<f32>),
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct TerrainChunk {
    pub origin: Vec2<i32>,
    #[serde(default = "default_size")]
    pub size: i32,
    #[serde(with = "vectorize")]
    pub heights: FxHashMap<(i32, i32), f32>,
    #[serde(skip, default)]
    pub processed_heights: Option<FxHashMap<(i32, i32), f32>>,
    #[serde(with = "vectorize")]
    pub sources: FxHashMap<(i32, i32), PixelSource>,
    #[serde(with = "vectorize")]
    pub blend_modes: FxHashMap<(i32, i32), TerrainBlendMode>,
    pub dirty: bool,
}

impl TerrainChunk {
    pub fn new(origin: Vec2<i32>, size: i32) -> Self {
        Self {
            origin,
            size,
            heights: FxHashMap::default(),
            processed_heights: None,
            sources: FxHashMap::default(),
            blend_modes: FxHashMap::default(),
            dirty: true,
        }
    }

    pub fn world_to_local(&self, world: Vec2<i32>) -> Vec2<i32> {
        world - self.origin
    }

    pub fn local_to_world(&self, local: Vec2<i32>) -> Vec2<i32> {
        local + self.origin
    }

    pub fn set_height(&mut self, x: i32, y: i32, value: f32) {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.heights.insert((local.x, local.y), value);
        self.mark_dirty();
    }

    pub fn set_blend_mode(&mut self, x: i32, y: i32, mode: TerrainBlendMode) {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        if mode == TerrainBlendMode::None {
            self.blend_modes.remove(&(local.x, local.y));
        } else {
            self.blend_modes.insert((local.x, local.y), mode);
        }
        self.mark_dirty();
    }

    pub fn get_height_unprocessed(&self, x: i32, y: i32) -> Option<f32> {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.heights.get(&(local.x, local.y)).copied()
    }

    pub fn get_height(&self, x: i32, y: i32) -> f32 {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        if let Some(process_heights) = &self.processed_heights {
            process_heights
                .get(&(local.x, local.y))
                .copied()
                .unwrap_or(0.0)
        } else {
            self.heights
                .get(&(local.x, local.y))
                .copied()
                .unwrap_or(0.0)
        }
    }

    pub fn set_source(&mut self, x: i32, y: i32, source: PixelSource) {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.sources.insert((local.x, local.y), source);
        self.mark_dirty();
    }

    pub fn get_source(&self, x: i32, y: i32) -> Option<&PixelSource> {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.sources.get(&(local.x, local.y))
    }

    pub fn sample_normal(&self, world: Vec2<i32>) -> Vec3<f32> {
        const EPSILON: i32 = 1;

        let h_l = self.get_height(world.x - EPSILON, world.y);
        let h_r = self.get_height(world.x + EPSILON, world.y);
        let h_d = self.get_height(world.x, world.y - EPSILON);
        let h_u = self.get_height(world.x, world.y + EPSILON);

        Vec3::new(h_l - h_r, 1.0, h_d - h_u).normalized()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Returns the bounds in world coordinates
    pub fn bounds(&self) -> BBox {
        let origin_f = self.origin.map(|v| v as f32);
        let size_f = Vec2::broadcast(self.size as f32 - 1.0);
        BBox::from_pos_size(origin_f, size_f)
    }

    /// Returns true if the height exists at (x, y) in this chunk
    pub fn exists(&self, x: i32, y: i32) -> bool {
        let world = Vec2::new(x, y);
        let local = self.world_to_local(world);
        self.heights.contains_key(&(local.x, local.y))
    }

    /// Processes the terrain modifiers for this chunk.
    pub fn process_batch_modifiers(
        &self,
        terrain: &Terrain,
        map: &Map,
        assets: &Assets,
        baked_texture: &mut Texture,
    ) -> FxHashMap<(i32, i32), f32> {
        let mut processed_heights = self.heights.clone();

        let bbox = self.bounds();
        let sectors = map.sorted_sectors_by_area();

        // 1, Pass: Modify Terrain
        for sector in &sectors {
            if bbox.intersects(&sector.bounding_box(map).expanded(Vec2::broadcast(2.0))) {
                if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                    sector.properties.get("region_graph")
                {
                    if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                        graph.sector_modify_heightmap(
                            sector,
                            map,
                            terrain,
                            &bbox,
                            self,
                            &mut processed_heights,
                            assets,
                            baked_texture,
                            ShapeFXModifierPass::Height,
                        );
                    }
                }
            }
        }

        // Group all linedefs with the same graph
        let mut linedef_groups: FxHashMap<Uuid, Vec<Linedef>> = FxHashMap::default();
        for linedef in &map.linedefs {
            if bbox.intersects(&linedef.bounding_box(map).expanded(Vec2::broadcast(2.0))) {
                if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                    linedef.properties.get("region_graph")
                {
                    linedef_groups
                        .entry(*graph_id)
                        .or_default()
                        .push(linedef.clone());
                }
            }
        }

        for (graph_id, linedefs) in &linedef_groups {
            if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                graph.linedef_modify_heightmap(
                    linedefs,
                    map,
                    terrain,
                    &bbox,
                    self,
                    &mut processed_heights,
                    assets,
                    baked_texture,
                    ShapeFXModifierPass::Height,
                );
            }
        }

        // 2, Pass: Colorize Terrain
        for sector in &sectors {
            if bbox.intersects(&sector.bounding_box(map).expanded(Vec2::broadcast(2.0))) {
                if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                    sector.properties.get("region_graph")
                {
                    if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                        graph.sector_modify_heightmap(
                            sector,
                            map,
                            terrain,
                            &bbox,
                            self,
                            &mut processed_heights,
                            assets,
                            baked_texture,
                            ShapeFXModifierPass::Colorize,
                        );
                    }
                }
            }
        }

        for (graph_id, linedefs) in &linedef_groups {
            if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                graph.linedef_modify_heightmap(
                    linedefs,
                    map,
                    terrain,
                    &bbox,
                    self,
                    &mut processed_heights,
                    assets,
                    baked_texture,
                    ShapeFXModifierPass::Colorize,
                );
            }
        }

        processed_heights
    }

    /// Build the 3D mesh for this chunk.
    pub fn build_mesh(&self, terrain: &Terrain) -> Batch3D {
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_map = FxHashMap::default();

        if let Some(processed_heights) = &self.processed_heights {
            for (&(lx, ly), &_) in processed_heights {
                let world_pos = self.local_to_world(Vec2::new(lx, ly));

                for (dx, dy) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let px = world_pos.x + dx;
                    let py = world_pos.y + dy;

                    if vertex_map.contains_key(&(px, py)) {
                        continue;
                    }

                    let index = vertices.len();
                    vertex_map.insert((px, py), index);

                    vertices.push([
                        px as f32 * terrain.scale.x,
                        terrain.get_height(px, py),
                        py as f32 * terrain.scale.y,
                        1.0,
                    ]);
                    uvs.push([0.0, 0.0]);
                }

                let i0 = vertex_map[&(world_pos.x, world_pos.y)];
                let i1 = vertex_map[&(world_pos.x + 1, world_pos.y)];
                let i2 = vertex_map[&(world_pos.x, world_pos.y + 1)];
                let i3 = vertex_map[&(world_pos.x + 1, world_pos.y + 1)];

                indices.push((i0, i2, i1));
                indices.push((i1, i2, i3));
            }
        }

        let mut batch = Batch3D::new(vertices, indices, uvs);
        batch.source = PixelSource::Terrain;
        batch.compute_vertex_normals();
        batch
    }

    /// Builds a simple 2D rectangle batch mesh for this chunk
    pub fn build_mesh_d2(&self, terrain: &Terrain) -> Batch2D {
        let min = self.origin;
        let max = self.origin + Vec2::new(terrain.chunk_size, terrain.chunk_size) - Vec2::new(1, 1);

        let min_pos = Vec2::new(
            min.x as f32 * terrain.scale.x,
            min.y as f32 * terrain.scale.y,
        );
        let max_pos = Vec2::new(
            (max.x + 1) as f32 * terrain.scale.x,
            (max.y + 1) as f32 * terrain.scale.y,
        );

        let width = max_pos.x - min_pos.x;
        let height = max_pos.y - min_pos.y;

        let mut batch = Batch2D::from_rectangle(min_pos.x, min_pos.y, width, height);
        batch.source = PixelSource::Terrain;
        batch
    }
}
