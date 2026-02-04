use crate::{BBox2D, GeoId, LineStrip2D, Poly2D, Poly3D};
use rustc_hash::FxHashMap;
use uuid::Uuid;

use vek::{Mat3, Vec2, Vec3};

#[derive(Debug, Default, Clone)]
pub struct Chunk {
    pub origin: Vec2<i32>,
    pub size: i32,
    pub bbox: BBox2D,

    /// 2D Geometry
    pub polys_map: FxHashMap<GeoId, Poly2D>,

    /// 2D screen-space line strips (constant pixel width)
    pub lines2d_px: FxHashMap<GeoId, LineStrip2D>,

    /// 3D Geometry,
    pub polys3d_map: rustc_hash::FxHashMap<GeoId, Vec<Poly3D>>,

    /// The priority of the chunk.
    pub priority: i32,
}

impl Chunk {
    pub fn new(origin: Vec2<i32>, size: i32) -> Self {
        let bbox = BBox2D::from_pos_size(origin.map(|v| v as f32), Vec2::broadcast(size as f32));
        Self {
            origin,
            size,
            bbox,
            ..Default::default()
        }
    }

    pub fn add(&mut self, poly: Poly2D) {
        self.polys_map.insert(poly.id, poly);
    }

    pub fn add_3d(&mut self, poly: Poly3D) {
        self.polys3d_map.entry(poly.id).or_default().push(poly);
    }

    /// Add a 2D polygon with explicit vertices/uvs/indices. Indices are local to this chunk.
    pub fn add_poly_2d(
        &mut self,
        id: GeoId,
        tile_id: Uuid,
        vertices: Vec<[f32; 2]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
        layer: i32,
        visible: bool,
    ) {
        let poly = Poly2D {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            transform: Mat3::identity(),
            layer,
            visible,
        };
        self.polys_map.insert(id, poly);
    }

    /// Add a 2D line strip tessellated into thick quads (no caps/joins) as one poly.
    /// `points` are in world coords; `width` is in world units.
    pub fn add_line_strip_2d(
        &mut self,
        id: GeoId,
        tile_id: Uuid,
        points: Vec<[f32; 2]>,
        width: f32,
        layer: i32,
    ) {
        if points.len() < 2 {
            return;
        }
        let half = 0.5 * width;
        let mut vertices: Vec<[f32; 2]> = Vec::with_capacity(points.len() * 4);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(points.len() * 4);
        let mut indices: Vec<(usize, usize, usize)> = Vec::with_capacity((points.len() - 1) * 2);

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
            indices.push((base + 0, base + 1, base + 2));
            indices.push((base + 0, base + 2, base + 3));
        }

        if vertices.is_empty() {
            return;
        }

        let poly = Poly2D {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            transform: Mat3::identity(),
            layer,
            visible: true,
        };
        self.polys_map.insert(id, poly);
    }

    /// Add a 2D line strip rendered with a constant pixel width (screen-space).
    /// `points` are in **world** coordinates; expansion to pixel-thick quads happens later.
    pub fn add_line_strip_2d_px(
        &mut self,
        id: GeoId,
        tile_id: Uuid,
        points: Vec<[f32; 2]>,
        width_px: f32,
        layer: i32,
    ) {
        if points.len() < 2 {
            return;
        }
        let line = LineStrip2D {
            id,
            tile_id,
            points,
            width_px,
            layer,
            visible: true,
        };
        self.lines2d_px.insert(id, line);
    }

    /// Add a square (axis-aligned) centered at `center` with edge length `size`.
    /// Inserts a new Poly2D using `tile_id` and `id`. UVs cover the full tile.
    pub fn add_square_2d(
        &mut self,
        id: GeoId,
        tile_id: Uuid,
        center: [f32; 2],
        size: f32,
        layer: i32,
        visible: bool,
    ) {
        if size <= 0.0 {
            return;
        }
        let half = 0.5 * size;
        let (cx, cy) = (center[0], center[1]);
        let x0 = cx - half; // left
        let x1 = cx + half; // right
        let y0 = cy - half; // bottom
        let y1 = cy + half; // top

        let vertices = vec![
            [x0, y0], // bottom-left
            [x0, y1], // top-left
            [x1, y1], // top-right
            [x1, y0], // bottom-right
        ];
        let uvs = vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
        let indices = vec![(0, 1, 2), (0, 2, 3)];

        let poly = Poly2D {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            transform: Mat3::identity(),
            layer,
            visible,
        };
        self.polys_map.insert(id, poly);
    }

    /// Add a 3D polygon
    pub fn add_poly_3d(
        &mut self,
        id: GeoId,
        tile_id: uuid::Uuid,
        vertices: Vec<[f32; 4]>,
        uvs: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
        layer: i32,
        visible: bool,
    ) {
        self.polys3d_map.entry(id).or_default().push(Poly3D {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer,
            visible,
            tile_id2: None,
            blend_weights: Vec::new(),
        });
    }

    /// Add a 3D polygon with texture blending
    pub fn add_poly_3d_blended(
        &mut self,
        id: GeoId,
        tile_id: uuid::Uuid,
        tile_id2: uuid::Uuid,
        vertices: Vec<[f32; 4]>,
        uvs: Vec<[f32; 2]>,
        blend_weights: Vec<f32>,
        indices: Vec<(usize, usize, usize)>,
        layer: i32,
        visible: bool,
    ) {
        self.polys3d_map.entry(id).or_default().push(Poly3D {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer,
            visible,
            tile_id2: Some(tile_id2),
            blend_weights,
        });
    }

    /// Add a camera-facing quad billboard centered at `center` with side length `size`.
    pub fn add_billboard_3d(
        &mut self,
        id: GeoId,
        tile_id: Uuid,
        center: Vec3<f32>,
        view_right: Vec3<f32>,
        view_up: Vec3<f32>,
        size: f32,
        visible: bool,
    ) {
        if !size.is_finite() || size <= 0.0 {
            return;
        }

        let right_len = view_right.magnitude();
        let right = if !right_len.is_finite() || right_len < 1e-6 {
            Vec3::unit_x()
        } else {
            view_right / right_len
        };

        let up_len = view_up.magnitude();
        let mut up = if !up_len.is_finite() || up_len < 1e-6 {
            Vec3::unit_y()
        } else {
            view_up / up_len
        };

        // Ensure the basis is not degenerate; re-orthogonalize `up` if needed.
        if right.cross(up).magnitude() < 1e-6 {
            let mut fallback = if right.y.abs() < 0.9 {
                Vec3::unit_y()
            } else {
                Vec3::unit_z()
            };
            fallback = fallback - right * fallback.dot(right);
            let fb_len = fallback.magnitude();
            up = if !fb_len.is_finite() || fb_len < 1e-6 {
                Vec3::unit_z()
            } else {
                fallback / fb_len
            };
        }

        let h = 0.5 * size;
        let p0 = center - right * h - up * h;
        let p1 = center + right * h - up * h;
        let p2 = center + right * h + up * h;
        let p3 = center - right * h + up * h;

        let vertices = vec![
            [p0.x, p0.y, p0.z, 1.0],
            [p1.x, p1.y, p1.z, 1.0],
            [p2.x, p2.y, p2.z, 1.0],
            [p3.x, p3.y, p3.z, 1.0],
        ];
        let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let indices = vec![(0usize, 1usize, 2usize), (0usize, 2usize, 3usize)];

        let poly = Poly3D {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer: 0,
            visible,
            tile_id2: None,
            blend_weights: Vec::new(),
        };
        self.polys3d_map.entry(id).or_default().push(poly);
    }

    /// Add a 3D line as a camera-independent quad based on thickness and a reference normal.
    pub fn add_line_3d(
        &mut self,
        id: GeoId,
        tile_id: Uuid,
        a: Vec3<f32>,
        b: Vec3<f32>,
        thickness: f32,
        normal: Vec3<f32>,
        layer: i32,
    ) {
        // Reject degenerate segments
        let dir = b - a;
        let dir_len = dir.magnitude();
        if dir_len < 1e-6 || !dir_len.is_finite() {
            return;
        }
        let dir_n = dir / dir_len;

        // Pick a stable face normal `n`:
        // - use provided `normal` if valid
        // - if nearly parallel to dir, pick an axis least aligned with dir
        let mut n = if normal.magnitude() < 1e-6 || !normal.magnitude().is_finite() {
            Vec3::unit_y()
        } else {
            normal.normalized()
        };
        if dir_n.dot(n).abs() > 0.999 {
            let ax = dir_n.x.abs();
            let ay = dir_n.y.abs();
            let az = dir_n.z.abs();
            n = if ax <= ay && ax <= az {
                Vec3::unit_x()
            } else if ay <= az {
                Vec3::unit_y()
            } else {
                Vec3::unit_z()
            };
        }

        // Side vector perpendicular to both n and dir
        let mut side = n.cross(dir_n);
        if !side.x.is_finite()
            || !side.y.is_finite()
            || !side.z.is_finite()
            || side.magnitude() < 1e-6
        {
            // Fallback if n ~ dir or numerical issue
            side = dir_n.cross(Vec3::unit_y());
            if side.magnitude() < 1e-6 {
                side = dir_n.cross(Vec3::unit_x());
            }
        }
        let side_n = side.normalized();

        // Half thickness along the side; small caps so thick lines look nicer
        let half = side_n * (thickness * 0.5);
        let cap = dir_n * (thickness * 0.5);

        let a_ext = a - cap;
        let b_ext = b + cap;

        let v0 = a_ext - half; // bottom-left
        let v1 = a_ext + half; // top-left
        let v2 = b_ext + half; // top-right
        let v3 = b_ext - half; // bottom-right

        // Pack into Poly3D (positions as [x,y,z,1], simple UVs, two triangles)
        let vertices = vec![
            [v0.x, v0.y, v0.z, 1.0],
            [v1.x, v1.y, v1.z, 1.0],
            [v2.x, v2.y, v2.z, 1.0],
            [v3.x, v3.y, v3.z, 1.0],
        ];
        let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        let indices = vec![(0usize, 1usize, 2usize), (0usize, 2usize, 3usize)];

        let poly = Poly3D {
            id,
            tile_id,
            vertices,
            uvs,
            indices,
            layer,
            visible: true,
            tile_id2: None,
            blend_weights: Vec::new(),
        };
        self.polys3d_map.entry(id).or_default().push(poly);
    }
}
