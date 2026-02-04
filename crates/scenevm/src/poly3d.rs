use crate::GeoId;
use uuid::Uuid;
use vek::Vec3;

#[derive(Debug, Clone)]
pub struct Poly3D {
    pub id: GeoId,
    pub tile_id: uuid::Uuid,
    pub vertices: Vec<[f32; 4]>, // world-space XYZ(W)
    pub uvs: Vec<[f32; 2]>,      // per-vertex UV
    pub indices: Vec<(usize, usize, usize)>,
    pub layer: i32, // for future (not used by ray depth)
    pub visible: bool,
    // Vertex blending support (optional)
    pub tile_id2: Option<uuid::Uuid>, // Secondary texture for blending
    pub blend_weights: Vec<f32>,      // Per-vertex blend factor (0.0=primary, 1.0=secondary)
}

impl Poly3D {
    /// Construct a Poly3D manually from geometry arrays.
    #[inline]
    pub fn poly(
        id: GeoId,
        tile_id: Uuid,
        vertices: Vec<[f32; 4]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
    ) -> Self {
        Self {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer: 0,
            visible: true,
            tile_id2: None,
            blend_weights: Vec::new(),
        }
    }

    /// Construct a cube centered at `center` with edge length `size`.
    #[inline]
    pub fn cube(id: GeoId, tile_id: Uuid, center: Vec3<f32>, size: f32) -> Self {
        let h = 0.5 * size;
        let (cx, cy, cz) = (center[0], center[1], center[2]);
        let p = |x: f32, y: f32, z: f32| -> [f32; 4] { [cx + x * h, cy + y * h, cz + z * h, 1.0] };

        // 24 verts (4 per face) in the order: -Z(front), +Z(back), -X(left), +X(right), +Y(top), -Y(bottom)
        // Each face wound CCW looking at the face.
        let mut vertices: Vec<[f32; 4]> = Vec::with_capacity(24);
        vertices.extend_from_slice(&[
            // front (-Z)
            p(-1.0, -1.0, -1.0),
            p(1.0, -1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            // back (+Z)
            p(-1.0, -1.0, 1.0),
            p(-1.0, 1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(1.0, -1.0, 1.0),
            // left (-X)
            p(-1.0, -1.0, 1.0),
            p(-1.0, -1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            p(-1.0, 1.0, 1.0),
            // right (+X)
            p(1.0, -1.0, -1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(1.0, 1.0, -1.0),
            // top (+Y)
            p(-1.0, 1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(1.0, 1.0, 1.0),
            p(-1.0, 1.0, 1.0),
            // bottom (-Y)
            p(-1.0, -1.0, 1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, -1.0, -1.0),
            p(-1.0, -1.0, -1.0),
        ]);

        // Per-face UVs (full 0..1 quad per face)
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(24);
        for _ in 0..6 {
            uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        }

        // Two triangles per face
        let mut indices: Vec<(usize, usize, usize)> = Vec::with_capacity(12);
        for f in 0..6 {
            let b = f * 4;
            indices.extend_from_slice(&[(b + 0, b + 1, b + 2), (b + 0, b + 2, b + 3)]);
        }

        Self {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer: 0,
            visible: true,
            tile_id2: None,
            blend_weights: Vec::new(),
        }
    }

    /// Construct a box centered at `center` with different sizes along each axis.
    #[inline]
    pub fn box_(
        id: GeoId,
        tile_id: Uuid,
        center: Vec3<f32>,
        size_x: f32,
        size_y: f32,
        size_z: f32,
    ) -> Self {
        let (cx, cy, cz) = (center[0], center[1], center[2]);
        let hx = 0.5 * size_x;
        let hy = 0.5 * size_y;
        let hz = 0.5 * size_z;
        let p =
            |x: f32, y: f32, z: f32| -> [f32; 4] { [cx + x * hx, cy + y * hy, cz + z * hz, 1.0] };

        // 24 verts (4 per face) in the order: -Z(front), +Z(back), -X(left), +X(right), +Y(top), -Y(bottom)
        // Each face wound CCW looking at the face.
        let mut vertices: Vec<[f32; 4]> = Vec::with_capacity(24);
        vertices.extend_from_slice(&[
            // front (-Z)
            p(-1.0, -1.0, -1.0),
            p(1.0, -1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            // back (+Z)
            p(-1.0, -1.0, 1.0),
            p(-1.0, 1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(1.0, -1.0, 1.0),
            // left (-X)
            p(-1.0, -1.0, 1.0),
            p(-1.0, -1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            p(-1.0, 1.0, 1.0),
            // right (+X)
            p(1.0, -1.0, -1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(1.0, 1.0, -1.0),
            // top (+Y)
            p(-1.0, 1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(1.0, 1.0, 1.0),
            p(-1.0, 1.0, 1.0),
            // bottom (-Y)
            p(-1.0, -1.0, 1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, -1.0, -1.0),
            p(-1.0, -1.0, -1.0),
        ]);

        // Per-face UVs (full 0..1 quad per face)
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(24);
        for _ in 0..6 {
            uvs.extend_from_slice(&[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        }

        // Two triangles per face
        let mut indices: Vec<(usize, usize, usize)> = Vec::with_capacity(12);
        for f in 0..6 {
            let b = f * 4;
            indices.extend_from_slice(&[(b + 0, b + 1, b + 2), (b + 0, b + 2, b + 3)]);
        }

        Self {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer: 0,
            visible: true,
            tile_id2: None,
            blend_weights: Vec::new(),
        }
    }

    /// Construct a sphere centered at `center` with given `radius`.
    /// Uses UV sphere generation with `stacks` and `slices` subdivisions.
    #[inline]
    pub fn sphere(
        id: GeoId,
        tile_id: Uuid,
        center: Vec3<f32>,
        radius: f32,
        stacks: usize,
        slices: usize,
    ) -> Self {
        let (cx, cy, cz) = (center[0], center[1], center[2]);

        let mut vertices: Vec<[f32; 4]> = Vec::new();
        let mut uvs: Vec<[f32; 2]> = Vec::new();
        let mut indices: Vec<(usize, usize, usize)> = Vec::new();

        // Generate vertices
        for stack in 0..=stacks {
            let v = stack as f32 / stacks as f32;
            let phi = v * std::f32::consts::PI;

            for slice in 0..=slices {
                let u = slice as f32 / slices as f32;
                let theta = u * 2.0 * std::f32::consts::PI;

                let x = theta.sin() * phi.sin();
                let y = phi.cos();
                let z = theta.cos() * phi.sin();

                vertices.push([cx + x * radius, cy + y * radius, cz + z * radius, 1.0]);
                uvs.push([u, v]);
            }
        }

        // Generate indices
        for stack in 0..stacks {
            for slice in 0..slices {
                let first = (stack * (slices + 1)) + slice;
                let second = first + slices + 1;

                indices.push((first, second + 1, second));
                indices.push((first, first + 1, second + 1));
            }
        }

        Self {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer: 0,
            visible: true,
            tile_id2: None,
            blend_weights: Vec::new(),
        }
    }

    #[inline]
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    #[inline]
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    #[inline]
    pub fn with_vertices(mut self, vertices: Vec<[f32; 4]>) -> Self {
        self.vertices = vertices;
        self
    }

    #[inline]
    pub fn with_uvs(mut self, uvs: Vec<[f32; 2]>) -> Self {
        self.uvs = uvs;
        self
    }

    #[inline]
    pub fn with_indices(mut self, indices: Vec<(usize, usize, usize)>) -> Self {
        self.indices = indices;
        self
    }

    #[inline]
    pub fn with_tile_id(mut self, tile_id: Uuid) -> Self {
        self.tile_id = tile_id;
        self
    }

    /// Set the secondary texture for vertex blending
    #[inline]
    pub fn with_blend_texture(mut self, tile_id2: Uuid) -> Self {
        self.tile_id2 = Some(tile_id2);
        self
    }

    /// Set per-vertex blend weights (0.0 = primary texture, 1.0 = secondary texture)
    #[inline]
    pub fn with_blend_weights(mut self, blend_weights: Vec<f32>) -> Self {
        self.blend_weights = blend_weights;
        self
    }
}
