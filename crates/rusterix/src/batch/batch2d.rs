use crate::prelude::*;
use crate::{Edges, Material, Rect, RepeatMode};
use vek::{Mat3, Vec2, Vec3};

use PrimitiveMode::*;
use RepeatMode::*;

/// A batch of 2D vertices, indices and their UVs which make up 2D polygons.
#[derive(Debug, Clone)]
pub struct Batch2D {
    // Render mode: triangles or lines
    pub mode: PrimitiveMode,

    /// 2D vertices which will get projected into 2D space.
    pub vertices: Vec<[f32; 2]>,

    /// The indices of the vertices of the batch.
    pub indices: Vec<(usize, usize, usize)>,

    /// The UVs of the batch.
    pub uvs: Vec<[f32; 2]>,

    /// Projected vertices
    pub projected_vertices: Vec<[f32; 2]>,

    /// 2D Bounding box of the projected vertices of the batch.
    pub bounding_box: Option<Rect>,

    /// Precomputed edges
    pub edges: Vec<Edges>,

    /// RepeatMode, default is ClampXY.
    pub repeat_mode: RepeatMode,

    /// The source of pixels for this batch.
    pub source: PixelSource,

    // Output after clipping and projection
    pub clipped_indices: Vec<(usize, usize, usize)>,
    pub clipped_uvs: Vec<[f32; 2]>,

    /// Transform matrix
    pub transform: Mat3<f32>,

    /// Indicates whether the batch receives lighting. True by default. Turn off for skybox etc.
    pub receives_light: bool,

    /// The material for the batch.
    pub material: Option<Material>,

    /// Shader
    pub shader: Option<usize>,
}

impl Default for Batch2D {
    fn default() -> Self {
        Self::empty()
    }
}

impl Batch2D {
    /// Empty constructor (the default)
    pub fn empty() -> Self {
        Self {
            mode: Triangles,
            vertices: vec![],
            indices: vec![],
            uvs: vec![],
            projected_vertices: vec![],
            bounding_box: None,
            edges: vec![],
            repeat_mode: ClampXY,
            source: PixelSource::Off,
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform: Mat3::identity(),
            receives_light: true,
            material: None,
            shader: None,
        }
    }

    /// A new batch
    pub fn new(
        vertices: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
    ) -> Self {
        Self {
            mode: Triangles,
            vertices,
            indices,
            uvs,
            projected_vertices: vec![],
            bounding_box: None,
            edges: vec![],
            repeat_mode: ClampXY,
            source: PixelSource::Off,
            clipped_indices: vec![],
            clipped_uvs: vec![],
            transform: Mat3::identity(),
            receives_light: true,
            material: None,
            shader: None,
        }
    }

    /// Create a Batch for a rectangle.
    pub fn from_rectangle(x: f32, y: f32, width: f32, height: f32) -> Self {
        let vertices = vec![
            [x, y],                  // Bottom-left
            [x, y + height],         // Top-left
            [x + width, y + height], // Top-right
            [x + width, y],          // Bottom-right
        ];

        let indices = vec![(0, 1, 2), (0, 2, 3)];

        let uvs = vec![
            [0.0, 0.0], // Top-left
            [0.0, 1.0], // Bottom-left
            [1.0, 1.0], // Bottom-right
            [1.0, 0.0], // Top-right
        ];

        Batch2D::new(vertices, indices, uvs)
    }

    /// Append a rectangle to the existing batch
    pub fn add_rectangle(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let base_index = self.vertices.len();

        self.vertices.reserve(4);
        self.uvs.reserve(4);
        self.indices.reserve(2);

        // Add vertices
        self.vertices.extend(vec![
            [x, y],                  // Bottom-left
            [x, y + height],         // Top-left
            [x + width, y + height], // Top-right
            [x + width, y],          // Bottom-right
        ]);

        // Add UVs
        self.uvs.extend(vec![
            [0.0, 0.0], // Top-left
            [0.0, 1.0], // Bottom-left
            [1.0, 1.0], // Bottom-right
            [1.0, 0.0], // Top-right
        ]);

        // Add indices
        self.indices.extend(vec![
            (base_index, base_index + 1, base_index + 2),
            (base_index, base_index + 2, base_index + 3),
        ]);
    }

    /// Add a set of geometry to the batch.
    pub fn add(
        &mut self,
        vertices: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
    ) {
        let base_index = self.vertices.len();

        self.vertices.reserve(vertices.len());
        self.uvs.reserve(uvs.len());
        self.indices.reserve(indices.len());

        self.vertices.extend(vertices);
        self.uvs.extend(uvs);

        for i in &indices {
            self.indices
                .push((i.0 + base_index, i.1 + base_index, i.2 + base_index));
        }
    }

    /// Add a set of geometry to the batch with wrapping (to create tilable textures).
    pub fn add_wrapped(
        &mut self,
        vertices: Vec<[f32; 2]>,
        indices: Vec<(usize, usize, usize)>,
        uvs: Vec<[f32; 2]>,
        wrap_size: f32,
    ) {
        let wrap_vertex = |v: [f32; 2], offset: [f32; 2]| -> [f32; 2] {
            [v[0] + offset[0] * wrap_size, v[1] + offset[1] * wrap_size]
        };

        let offsets = [
            [0.0, 0.0],
            [1.0, 0.0],
            [-1.0, 0.0],
            [0.0, 1.0],
            [0.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [-1.0, -1.0],
        ];

        let verts_per_tile = vertices.len();
        let uvs_per_tile = uvs.len();
        let tris_per_tile = indices.len();

        // Pre-reserve for all 9 tiles
        self.vertices.reserve(verts_per_tile * offsets.len());
        self.uvs.reserve(uvs_per_tile * offsets.len());
        self.indices.reserve(tris_per_tile * offsets.len());

        for offset in offsets.iter() {
            let base_index = self.vertices.len();

            // Append wrapped vertices directly
            for &v in &vertices {
                self.vertices.push(wrap_vertex(v, *offset));
            }
            // Append UVs without cloning the whole Vec each time
            self.uvs.extend(uvs.iter().copied());

            // Append adjusted indices directly
            for &(i0, i1, i2) in &indices {
                self.indices
                    .push((base_index + i0, base_index + i1, base_index + i2));
            }
        }
    }

    /// Append a line to the existing batch
    pub fn add_line(&mut self, start: Vec2<f32>, end: Vec2<f32>, thickness: f32) {
        let start = [start.x, start.y];
        let end = [end.x, end.y];

        let direction = [end[0] - start[0], end[1] - start[1]];
        let length = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();

        let base_index = self.vertices.len();

        // Avoid division by zero for zero-length lines
        if length == 0.0 {
            return;
        }

        self.vertices.reserve(4);
        self.uvs.reserve(4);
        self.indices.reserve(2);

        let normalized = [direction[0] / length, direction[1] / length];
        let normal = [
            -normalized[1] * thickness / 2.0,
            normalized[0] * thickness / 2.0,
        ];

        if self.mode == PrimitiveMode::Lines {
            // In line mode we add the start / end vertices directly.
            let vertices = vec![
                [start[0], start[1]],
                [end[0], end[1]],
                [end[0], end[1]],     // Repeated to ensure valid triangles
                [start[0], start[1]], // Repeated to ensure valid triangles
            ];

            let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

            self.vertices.extend(vertices);
            self.uvs.extend(uvs);

            self.indices.extend(vec![
                (base_index, base_index + 1, base_index + 2),
                (base_index, base_index + 2, base_index + 3),
            ]);
        } else {
            let vertices = vec![
                [start[0] - normal[0], start[1] - normal[1]],
                [start[0] + normal[0], start[1] + normal[1]],
                [end[0] + normal[0], end[1] + normal[1]],
                [end[0] - normal[0], end[1] - normal[1]],
            ];

            let uvs = vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];

            self.vertices.extend(vertices);
            self.uvs.extend(uvs);

            self.indices.extend(vec![
                (base_index, base_index + 1, base_index + 2),
                (base_index, base_index + 2, base_index + 3),
            ]);
        }
    }

    /// Add a line which wraps around the wrap_size parameter
    pub fn add_wrapped_line(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        thickness: f32,
        wrap_size: f32,
    ) {
        let wrap_point = |p: Vec2<f32>, offset: [f32; 2]| -> Vec2<f32> {
            Vec2::new(p.x + offset[0] * wrap_size, p.y + offset[1] * wrap_size)
        };

        let offsets = [
            [0.0, 0.0],
            [1.0, 0.0],
            [-1.0, 0.0],
            [0.0, 1.0],
            [0.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [-1.0, -1.0],
        ];

        // Each line produces 4 verts, 4 uvs, and 2 triangles
        self.vertices
            .reserve(self.vertices.len() + offsets.len() * 4);
        self.uvs.reserve(self.uvs.len() + offsets.len() * 4);
        self.indices.reserve(self.indices.len() + offsets.len() * 2);

        for offset in offsets.iter() {
            // Wrap start and end points
            let wrapped_start = wrap_point(start, *offset);
            let wrapped_end = wrap_point(end, *offset);

            // Add the wrapped line using the standard add_line logic
            self.add_line(wrapped_start, wrapped_end, thickness);
        }
    }

    /// Sets the drawing mode for the batch using the builder pattern.
    pub fn mode(mut self, mode: PrimitiveMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the repeat mode for the batch using the builder pattern.
    pub fn repeat_mode(mut self, repeat_mode: RepeatMode) -> Self {
        self.repeat_mode = repeat_mode;
        self
    }

    /// Set the source of pixels for this batch.
    pub fn source(mut self, pixel_source: PixelSource) -> Self {
        self.source = pixel_source;
        self
    }

    /// Set the shader index for this batch
    pub fn shader(mut self, shader: usize) -> Self {
        self.shader = Some(shader);
        self
    }

    /// Set the 3D transform matrix for this batch
    pub fn transform(mut self, transform: Mat3<f32>) -> Self {
        self.transform = transform;
        self
    }

    /// Set if the batch receives light
    pub fn receives_light(mut self, receives_light: bool) -> Self {
        self.receives_light = receives_light;
        self
    }

    /// Project 2D vertices using a optional Mat3 transformation matrix
    pub fn project(&mut self, matrix: Option<Mat3<f32>>) {
        self.projected_vertices.clear();
        self.projected_vertices.reserve(self.vertices.len());

        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        match matrix {
            Some(m) => {
                for &v in &self.vertices {
                    let r = m * Vec3::new(v[0], v[1], 1.0);
                    let p = [r.x, r.y];
                    min_x = min_x.min(p[0]);
                    max_x = max_x.max(p[0]);
                    min_y = min_y.min(p[1]);
                    max_y = max_y.max(p[1]);
                    self.projected_vertices.push(p);
                }
            }
            None => {
                for &p in &self.vertices {
                    min_x = min_x.min(p[0]);
                    max_x = max_x.max(p[0]);
                    min_y = min_y.min(p[1]);
                    max_y = max_y.max(p[1]);
                    self.projected_vertices.push(p);
                }
            }
        }

        self.bounding_box = Some(Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        });

        // Precompute edges for each triangle
        self.edges.clear();
        self.edges.reserve(self.indices.len());
        for &(i0, i1, i2) in &self.indices {
            let v0 = self.projected_vertices[i0];
            let v1 = self.projected_vertices[i1];
            let v2 = self.projected_vertices[i2];
            self.edges.push(crate::Edges::new(
                [[v0[0], v0[1]], [v1[0], v1[1]], [v2[0], v2[1]]],
                [[v1[0], v1[1]], [v2[0], v2[1]], [v0[0], v0[1]]],
                true,
            ));
        }
    }

    /// Calculate the bounding box for the projected vertices
    fn _calculate_bounding_box(&self) -> Rect {
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for v in &self.projected_vertices {
            min_x = min_x.min(v[0]); // `x` coordinate
            max_x = max_x.max(v[0]);
            min_y = min_y.min(v[1]); // `y` coordinate
            max_y = max_y.max(v[1]);
        }

        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
}
