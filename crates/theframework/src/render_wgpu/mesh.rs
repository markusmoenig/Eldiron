use crate::paint::*;
use lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers, math::point, path::Path as LyonPath,
};
use renderkit::{RenderError, Result};
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: Point,
    pub local: Point,
    pub uv: Point,
    pub color: Color,
}
impl Vertex {
    const ATTRIBUTES: [renderkit::wgpu::VertexAttribute; 4] =
        renderkit::wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x2,3=>Float32x4];
    pub(super) fn layout() -> renderkit::wgpu::VertexBufferLayout<'static> {
        renderkit::wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: renderkit::wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
pub fn rect_path(rect: Rect, radius: f32) -> Path {
    let r = radius
        .max(0.)
        .min(rect.width.min(rect.height).max(0.) * 0.5);
    let (x, y, w, h) = (rect.x, rect.y, rect.width, rect.height);
    let k = r * 0.5522848;
    Path {
        elements: vec![
            PathElement::Move([x + r, y]),
            PathElement::Line([x + w - r, y]),
            PathElement::Cubic([x + w - r + k, y], [x + w, y + r - k], [x + w, y + r]),
            PathElement::Line([x + w, y + h - r]),
            PathElement::Cubic(
                [x + w, y + h - r + k],
                [x + w - r + k, y + h],
                [x + w - r, y + h],
            ),
            PathElement::Line([x + r, y + h]),
            PathElement::Cubic([x + r - k, y + h], [x, y + h - r + k], [x, y + h - r]),
            PathElement::Line([x, y + r]),
            PathElement::Cubic([x, y + r - k], [x + r - k, y], [x + r, y]),
            PathElement::Close,
        ],
    }
}
fn path(source: &Path) -> LyonPath {
    let mut p = LyonPath::builder();
    let mut open = false;
    for op in &source.elements {
        match *op {
            PathElement::Move(a) => {
                if open {
                    p.end(false);
                }
                p.begin(point(a[0], a[1]));
                open = true;
            }
            PathElement::Line(a) => {
                p.line_to(point(a[0], a[1]));
            }
            PathElement::Quadratic(a, b) => {
                p.quadratic_bezier_to(point(a[0], a[1]), point(b[0], b[1]));
            }
            PathElement::Cubic(a, b, c) => {
                p.cubic_bezier_to(point(a[0], a[1]), point(b[0], b[1]), point(c[0], c[1]));
            }
            PathElement::Close => {
                if open {
                    p.end(true);
                    open = false;
                }
            }
        }
    }
    if open {
        p.end(false);
    }
    p.build()
}
pub fn tessellate(
    source: &Path,
    rule: FillRule,
    stroke: Option<&Stroke>,
    transform: Transform,
    density: f32,
) -> Result<(Vec<Vertex>, Vec<u32>)> {
    let mut geometry: VertexBuffers<Point, u32> = VertexBuffers::new();
    let path = path(source);
    let tolerance = 0.15 / (density * transform.0[0].hypot(transform.0[1]).max(0.01));
    if let Some(stroke) = stroke {
        let cap = match stroke.cap {
            LineCap::Butt => lyon_tessellation::LineCap::Butt,
            LineCap::Square => lyon_tessellation::LineCap::Square,
            LineCap::Round => lyon_tessellation::LineCap::Round,
        };
        let join = match stroke.join {
            LineJoin::Miter => lyon_tessellation::LineJoin::Miter,
            LineJoin::Bevel => lyon_tessellation::LineJoin::Bevel,
            LineJoin::Round => lyon_tessellation::LineJoin::Round,
        };
        StrokeTessellator::new()
            .tessellate_path(
                &path,
                &StrokeOptions::default()
                    .with_line_width(stroke.width)
                    .with_line_cap(cap)
                    .with_line_join(join)
                    .with_tolerance(tolerance),
                &mut BuffersBuilder::new(&mut geometry, |v: StrokeVertex| v.position().to_array()),
            )
            .map_err(|e| RenderError(format!("stroke tessellation: {e:?}")))?;
    } else {
        let rule = match rule {
            FillRule::NonZero => lyon_tessellation::FillRule::NonZero,
            FillRule::EvenOdd => lyon_tessellation::FillRule::EvenOdd,
        };
        FillTessellator::new()
            .tessellate_path(
                &path,
                &FillOptions::default()
                    .with_fill_rule(rule)
                    .with_tolerance(tolerance),
                &mut BuffersBuilder::new(&mut geometry, |v: FillVertex| v.position().to_array()),
            )
            .map_err(|e| RenderError(format!("fill tessellation: {e:?}")))?;
    }
    let vertices = geometry
        .vertices
        .into_iter()
        .map(|local| {
            let pos = transform.point(local);
            Vertex {
                position: [pos[0] * density, pos[1] * density],
                local,
                uv: [0., 0.],
                color: [1.; 4],
            }
        })
        .collect();
    Ok((vertices, geometry.indices))
}
pub fn quad(rect: Rect, uv: Rect, t: Transform, density: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::with_capacity(4);
    let mut indices = Vec::with_capacity(6);
    append_quad(&mut vertices, &mut indices, rect, uv, t, density);
    (vertices, indices)
}

/// Append ordered textured geometry without allocating once per glyph.
pub fn append_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    rect: Rect,
    uv: Rect,
    t: Transform,
    density: f32,
) {
    let local = [
        [rect.x, rect.y],
        [rect.x + rect.width, rect.y],
        [rect.x + rect.width, rect.y + rect.height],
        [rect.x, rect.y + rect.height],
    ];
    let tex = [
        [uv.x, uv.y],
        [uv.x + uv.width, uv.y],
        [uv.x + uv.width, uv.y + uv.height],
        [uv.x, uv.y + uv.height],
    ];
    let base = u32::try_from(vertices.len()).expect("paint batch vertex count exceeds u32");
    vertices.extend(local.into_iter().zip(tex).map(|(local, uv)| {
        let p = t.point(local);
        Vertex {
            position: [p[0] * density, p[1] * density],
            local,
            uv,
            color: [1.; 4],
        }
    }));
    indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

pub fn colored(vertices: &[PaintVertex], t: Transform, density: f32) -> Vec<Vertex> {
    vertices
        .iter()
        .map(|v| {
            let p = t.point(v.position);
            Vertex {
                position: [p[0] * density, p[1] * density],
                local: v.position,
                uv: [0.; 2],
                color: v.color,
            }
        })
        .collect()
}

/// Textured triangles use the same physical transform as quads and glyphs.
pub fn textured(vertices: &[ImageVertex], transform: Transform, density: f32) -> Vec<Vertex> {
    vertices
        .iter()
        .map(|v| {
            let p = transform.point(v.position);
            Vertex {
                position: [p[0] * density, p[1] * density],
                local: v.position,
                uv: v.uv,
                color: [1.; 4],
            }
        })
        .collect()
}

/// Constant-size support geometry for an integer device-pixel line. Clip its
/// support in f64 before converting to GPU vertices: huge panned coordinates
/// must not lose a narrow strip to f32 cancellation or rasterize its full box.
/// The fragment shader keeps original endpoints for exact coverage/phase.
pub fn pixel_line(
    a: [i32; 2],
    b: [i32; 2],
    viewport: (u32, u32),
) -> Option<(Vec<Vertex>, Vec<u32>)> {
    if viewport.0 == 0 || viewport.1 == 0 {
        return None;
    }
    if a == b {
        if a[0] < 0 || a[1] < 0 || a[0] as u32 >= viewport.0 || a[1] as u32 >= viewport.1 {
            return None;
        }
        return Some(quad(
            Rect::new(a[0] as f32, a[1] as f32, 1., 1.),
            Rect::default(),
            Transform::default(),
            1.,
        ));
    }
    let start = [a[0] as f64 + 0.5, a[1] as f64 + 0.5];
    let delta = [b[0] as f64 - a[0] as f64, b[1] as f64 - a[1] as f64];
    let mut low = 0f64;
    let mut high = 1f64;
    for (p, q) in [
        (-delta[0], start[0] + 2.),
        (delta[0], viewport.0 as f64 + 2. - start[0]),
        (-delta[1], start[1] + 2.),
        (delta[1], viewport.1 as f64 + 2. - start[1]),
    ] {
        if p == 0. {
            if q < 0. {
                return None;
            }
        } else if p < 0. {
            low = low.max(q / p);
        } else {
            high = high.min(q / p);
        }
        if low > high {
            return None;
        }
    }
    let length = delta[0].hypot(delta[1]);
    let direction = [delta[0] / length, delta[1] / length];
    let normal = [-direction[1] * 2., direction[0] * 2.];
    let p = [
        start[0] + delta[0] * low - direction[0] * 2. - normal[0],
        start[1] + delta[1] * low - direction[1] * 2. - normal[1],
    ];
    let d = [
        delta[0] * (high - low) + direction[0] * 4.,
        delta[1] * (high - low) + direction[1] * 4.,
    ];
    Some(quad(
        Rect::new(0., 0., 1., 1.),
        Rect::default(),
        Transform([
            d[0] as f32,
            d[1] as f32,
            (normal[0] * 2.) as f32,
            (normal[1] * 2.) as f32,
            p[0] as f32,
            p[1] as f32,
        ]),
        1.,
    ))
}

/// A constant-size, integer-aligned support quad. Coverage is evaluated by the
/// fragment shader at pixel centers, uniformly across the target's MSAA samples.
/// Keep original subpixel vertices as parameters, never a CPU triangle bitmap.
pub fn pixel_triangle(points: [Point; 3], viewport: (u32, u32)) -> Option<(Vec<Vertex>, Vec<u32>)> {
    let bounds = |axis| {
        let min = points
            .iter()
            .map(|p| p[axis] as f64)
            .fold(f64::INFINITY, f64::min);
        let max = points
            .iter()
            .map(|p| p[axis] as f64)
            .fold(f64::NEG_INFINITY, f64::max);
        let extent = if axis == 0 { viewport.0 } else { viewport.1 } as f64;
        (
            min.floor().max(0.).min(extent),
            (max.ceil() + 1.).max(0.).min(extent),
        )
    };
    let (left, right) = bounds(0);
    let (top, bottom) = bounds(1);
    if right <= left || bottom <= top {
        return None;
    }
    Some(quad(
        Rect::new(
            left as f32,
            top as f32,
            (right - left) as f32,
            (bottom - top) as f32,
        ),
        Rect::default(),
        Transform::default(),
        1.,
    ))
}
