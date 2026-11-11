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
            }
        })
        .collect();
    Ok((vertices, geometry.indices))
}
pub fn quad(rect: Rect, uv: Rect, t: Transform, density: f32) -> (Vec<Vertex>, Vec<u32>) {
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
    (
        local
            .into_iter()
            .zip(tex)
            .map(|(local, uv)| {
                let p = t.point(local);
                Vertex {
                    position: [p[0] * density, p[1] * density],
                    local,
                    uv,
                }
            })
            .collect(),
        vec![0, 1, 2, 0, 2, 3],
    )
}
