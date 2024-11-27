use theframework::prelude::*;

use euc::*;
use vek::*;

pub struct EucDraw {
    view_size: Vec2f,
    buffer: Buffer2d<[u8; 4]>,

    colored_vertices: Vec<([f32; 2], Rgba<f32>)>,
    indices: Vec<usize>,
}

#[allow(clippy::new_without_default)]
impl EucDraw {
    pub fn new(width: usize, height: usize) -> Self {
        let buffer = Buffer2d::fill([width, height], [0; 4]);
        Self {
            view_size: vec2f(width as f32, height as f32),
            buffer,

            colored_vertices: vec![],
            indices: vec![],
        }
    }

    /*
    pub fn add_box(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgba<f32>) {
        let top_left = [self.cx(x), self.cy(y)];
        let top_right = [self.cx(x + width), self.cy(y)];
        let bottom_left = [self.cx(x), self.cy(y + height)];
        let bottom_right = [self.cx(x + width), self.cy(y + height)];

        self.colored_vertices.extend([
            (top_left, color), // Triangle 1
            (bottom_left, color),
            (bottom_right, color),
            (top_left, color), // Triangle 2
            (bottom_right, color),
            (top_right, color),
        ]);
    }*/

    pub fn add_box(&mut self, x: f32, y: f32, width: f32, height: f32, color: Rgba<f32>) {
        let top_left = [self.cx(x), self.cy(y)];
        let top_right = [self.cx(x + width), self.cy(y)];
        let bottom_left = [self.cx(x), self.cy(y + height)];
        let bottom_right = [self.cx(x + width), self.cy(y + height)];

        let base_index = self.colored_vertices.len();

        self.colored_vertices.extend([
            (top_left, color),
            (top_right, color),
            (bottom_left, color),
            (bottom_right, color),
        ]);

        self.indices.extend([
            base_index,     // Top-left
            base_index + 2, // Bottom-left
            base_index + 3, // Bottom-right
            base_index,     // Top-left
            base_index + 3, // Bottom-right
            base_index + 1, // Top-right
        ]);
    }

    pub fn add_line(&mut self, sx: f32, sy: f32, ex: f32, ey: f32, color: Rgba<f32>) {
        self.colored_vertices.extend([
            ([self.cx(sx), self.cy(sy)], color),
            ([self.cx(ex), self.cy(ey)], color),
        ]);
    }

    pub fn draw_as_triangles(&mut self) {
        if !self.colored_vertices.is_empty() {
            let indexed_vertices =
                IndexedVertices::new(self.indices.as_slice(), self.colored_vertices.as_slice());

            ColoredTriangles {}.render(indexed_vertices, &mut self.buffer, &mut Empty::default());
            self.colored_vertices.clear();
            self.indices.clear();
        }
    }

    pub fn draw_as_lines(&mut self) {
        if !self.colored_vertices.is_empty() {
            ColoredLines.render(
                &self.colored_vertices,
                &mut self.buffer,
                &mut Empty::default(),
            );
            self.colored_vertices.clear();
        }
    }

    pub fn blend_into(&mut self, ext: &mut TheRGBABuffer) {
        let b = TheRGBABuffer::from(
            self.buffer
                .raw()
                .iter()
                .flat_map(|&arr| arr.into_iter())
                .collect(),
            self.view_size.x as u32,
            self.view_size.y as u32,
        );

        ext.blend_into(0, 0, &b);
    }

    fn cx(&self, v: f32) -> f32 {
        (v / self.view_size.x) * 2.0 - 1.0
    }

    fn cy(&self, v: f32) -> f32 {
        1.0 - (v / self.view_size.y) * 2.0
    }
}

struct ColoredTriangles;
impl<'r> Pipeline<'r> for ColoredTriangles {
    type Vertex = ([f32; 2], Rgba<f32>);
    type VertexData = Rgba<f32>;
    type Primitives = TriangleList;
    type Fragment = Rgba<f32>;
    type Pixel = [u8; 4];

    fn vertex(&self, (pos, col): &Self::Vertex) -> ([f32; 4], Self::VertexData) {
        ([pos[0], pos[1], 0.0, 1.0], *col)
    }

    fn fragment(&self, col: Self::VertexData) -> Self::Fragment {
        col
    }

    fn blend(&self, _: Self::Pixel, col: Self::Fragment) -> Self::Pixel {
        //u32::from_le_bytes(col.map(|e| (e * 255.0) as u8).into_array())
        [
            (col[0] * 255.0) as u8,
            (col[1] * 255.0) as u8,
            (col[2] * 255.0) as u8,
            (col[3] * 255.0) as u8,
        ]
    }
}

struct ColoredLines;
impl<'r> Pipeline<'r> for ColoredLines {
    type Vertex = ([f32; 2], Rgba<f32>);
    type VertexData = Rgba<f32>;
    type Primitives = LineList;
    type Fragment = Rgba<f32>;
    type Pixel = [u8; 4];

    #[inline(always)]
    fn aa_mode(&self) -> AaMode {
        AaMode::Msaa { level: 1 }
    }

    // #[inline(always)]
    // fn coordinate_mode(&self) -> CoordinateMode {
    //     CoordinateMode::METAL
    // }

    fn vertex(&self, (pos, col): &Self::Vertex) -> ([f32; 4], Self::VertexData) {
        ([pos[0], pos[1], 0.0, 1.0], *col)
    }

    fn fragment(&self, col: Self::VertexData) -> Self::Fragment {
        col
    }

    fn blend(&self, _: Self::Pixel, col: Self::Fragment) -> Self::Pixel {
        //u32::from_le_bytes(col.map(|e| (e * 255.0) as u8).into_array())
        [
            (col[0] * 255.0) as u8,
            (col[1] * 255.0) as u8,
            (col[2] * 255.0) as u8,
            (col[3] * 255.0) as u8,
        ]
    }
}
