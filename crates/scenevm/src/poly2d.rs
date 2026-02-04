use crate::GeoId;
use uuid::Uuid;
use vek::Mat3;

#[derive(Debug, Clone)]
pub struct Poly2D {
    pub id: GeoId,
    pub tile_id: Uuid,
    pub vertices: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<(usize, usize, usize)>, // triangle list, LOCAL to its chunk
    pub transform: Mat3<f32>,                // per-poly local transform
    pub layer: i32,                          // visual layer; higher draws on top
    pub visible: bool,                       // if false, skipped during draw
}

impl Default for Poly2D {
    fn default() -> Self {
        Self {
            id: GeoId::Unknown(0),
            tile_id: Uuid::nil(),
            vertices: Vec::new(),
            uvs: Vec::new(),
            indices: Vec::new(),
            transform: Mat3::identity(),
            layer: 0,
            visible: true,
        }
    }
}

impl Poly2D {
    pub fn poly(
        id: GeoId,
        tile_id: Uuid,
        vertices: Vec<[f32; 2]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
    ) -> Self {
        Self {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            transform: Mat3::identity(),
            layer: 0,
            visible: true,
        }
    }

    /// Construct a 2D line strip tessellated into thick quads (no caps/joins) as one poly.
    /// `points` are in world coords; `width` is in world units.
    /// Returns `None` if there are fewer than 2 valid points or all segments are degenerate.
    pub fn line(id: GeoId, tile_id: Uuid, points: Vec<[f32; 2]>, width: f32, layer: i32) -> Self {
        let half = 0.5 * width;
        let valid_segments = points.len().saturating_sub(1);
        let mut vertices: Vec<[f32; 2]> = Vec::with_capacity(valid_segments * 4);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(valid_segments * 4);
        let mut indices: Vec<(usize, usize, usize)> = Vec::with_capacity(valid_segments * 2);

        for seg in 0..(points.len() - 1) {
            let p0 = points[seg];
            let p1 = points[seg + 1];
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let len = (dx * dx + dy * dy).sqrt();
            if len == 0.0 {
                continue;
            }
            let nx = -dy / len; // left-hand normal (perp)
            let ny = dx / len;
            let ox = nx * half;
            let oy = ny * half;

            // Quad corners (consistent winding: 0-1-2, 0-2-3)
            let v0 = [p0[0] - ox, p0[1] - oy]; // bottom-left
            let v1 = [p0[0] + ox, p0[1] + oy]; // top-left
            let v2 = [p1[0] + ox, p1[1] + oy]; // top-right
            let v3 = [p1[0] - ox, p1[1] - oy]; // bottom-right

            let base = vertices.len();
            vertices.extend_from_slice(&[v0, v1, v2, v3]);
            // Simple UVs per quad (stretch along segment)
            uvs.extend_from_slice(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
            indices.extend_from_slice(&[
                (base + 0, base + 1, base + 2),
                (base + 0, base + 2, base + 3),
            ]);
        }

        Self {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            transform: Mat3::identity(),
            layer,
            visible: true,
        }
    }

    /// Construct a square (axis-aligned) centered at `center` with edge length `size`.
    /// UVs cover the full tile. Returns `None` if `size` <= 0.
    pub fn quad(
        id: GeoId,
        tile_id: Uuid,
        center: [f32; 2],
        size: f32,
        layer: i32,
        visible: bool,
    ) -> Self {
        let half = 0.5 * size;
        let (cx, cy) = (center[0], center[1]);
        let x0 = cx - half; // left
        let x1 = cx + half; // right
        let y0 = cy - half; // bottom
        let y1 = cy + half; // top

        let mut vertices = Vec::with_capacity(4);
        let mut uvs = Vec::with_capacity(4);
        let mut indices = Vec::with_capacity(2);

        vertices.extend_from_slice(&[
            [x0, y0], // bottom-left
            [x0, y1], // top-left
            [x1, y1], // top-right
            [x1, y0], // bottom-right
        ]);
        uvs.extend_from_slice(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        indices.extend_from_slice(&[(0, 1, 2), (0, 2, 3)]);

        Self {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            transform: Mat3::identity(),
            layer,
            visible,
        }
    }

    pub fn with_id(mut self, id: GeoId) -> Self {
        self.id = id;
        self
    }
    pub fn with_tile_id(mut self, tile_id: Uuid) -> Self {
        self.tile_id = tile_id;
        self
    }
    pub fn with_vertices(mut self, vertices: Vec<[f32; 2]>) -> Self {
        self.vertices = vertices;
        self
    }
    pub fn with_uvs(mut self, uvs: Vec<[f32; 2]>) -> Self {
        self.uvs = uvs;
        self
    }
    pub fn with_indices(mut self, indices: Vec<(usize, usize, usize)>) -> Self {
        self.indices = indices;
        self
    }
    pub fn with_transform(mut self, transform: Mat3<f32>) -> Self {
        self.transform = transform;
        self
    }
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}
