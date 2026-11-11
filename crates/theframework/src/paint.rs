//! Ordered backend-neutral painting. No destination pixel buffer or GPU API.
use std::sync::Arc;
pub use theframework_core::ImageHandle;
pub type Point = [f32; 2];
pub type Color = [f32; 4]; // straight sRGB input; backends convert before compositing
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub fn inset(self, amount: f32) -> Self {
        Self::new(
            self.x + amount,
            self.y + amount,
            (self.width - amount * 2.).max(0.),
            (self.height - amount * 2.).max(0.),
        )
    }
    pub fn contains(self, p: Point) -> bool {
        p[0] >= self.x
            && p[1] >= self.y
            && p[0] < self.x + self.width
            && p[1] < self.y + self.height
    }
    pub fn intersection(self, other: Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        Self::new(
            x,
            y,
            (self.x + self.width).min(other.x + other.width).max(x) - x,
            (self.y + self.height).min(other.y + other.height).max(y) - y,
        )
    }
}
/// Affine [a,b,c,d,tx,ty]. Logical geometry and hit testing use the same transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform(pub [f32; 6]);
impl Default for Transform {
    fn default() -> Self {
        Self([1., 0., 0., 1., 0., 0.])
    }
}
impl Transform {
    pub fn translate(x: f32, y: f32) -> Self {
        Self([1., 0., 0., 1., x, y])
    }
    pub fn scale(x: f32, y: f32) -> Self {
        Self([x, 0., 0., y, 0., 0.])
    }
    pub fn point(self, p: Point) -> Point {
        let [a, b, c, d, x, y] = self.0;
        [a * p[0] + c * p[1] + x, b * p[0] + d * p[1] + y]
    }
    pub fn then(self, parent: Self) -> Self {
        let [a, b, c, d, x, y] = self.0;
        let [e, f, g, h, u, v] = parent.0;
        Self([
            e * a + g * b,
            f * a + h * b,
            e * c + g * d,
            f * c + h * d,
            e * x + g * y + u,
            f * x + h * y + v,
        ])
    }
    pub fn inverse(self) -> Option<Self> {
        let [a, b, c, d, x, y] = self.0;
        let det = a * d - b * c;
        if !det.is_finite() || det.abs() < 1e-8 {
            return None;
        }
        Some(Self([
            d / det,
            -b / det,
            -c / det,
            a / det,
            (c * y - d * x) / det,
            (b * x - a * y) / det,
        ]))
    }
}
#[derive(Clone, Debug)]
pub enum Brush {
    Solid(Color),
    LinearGradient {
        start: Point,
        end: Point,
        start_color: Color,
        end_color: Color,
    },
}
impl From<Color> for Brush {
    fn from(c: Color) -> Self {
        Self::Solid(c)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum PathElement {
    Move(Point),
    Line(Point),
    Quadratic(Point, Point),
    Cubic(Point, Point, Point),
    Close,
}
#[derive(Clone, Debug, Default)]
pub struct Path {
    pub elements: Vec<PathElement>,
}
#[derive(Clone, Copy, Debug)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}
#[derive(Clone, Copy, Debug)]
pub enum LineCap {
    Butt,
    Square,
    Round,
}
#[derive(Clone, Copy, Debug)]
pub enum LineJoin {
    Miter,
    Bevel,
    Round,
}
#[derive(Clone, Debug)]
pub struct Stroke {
    pub width: f32,
    pub cap: LineCap,
    pub join: LineJoin,
    pub brush: Brush,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sampling {
    Nearest,
    Linear,
    /// Repeat addressing is explicit for tiled source assets; one quad can
    /// cover any viewport without allocating per-pixel or per-tile geometry.
    RepeatNearest,
    RepeatLinear,
    RepeatNearestX,
    RepeatNearestY,
    RepeatLinearX,
    RepeatLinearY,
}
#[derive(Clone, Debug)]
pub struct FontFace {
    pub identity: (u64, u32),
    pub data: Arc<[u8]>,
}
#[derive(Clone, Copy, Debug)]
pub struct Glyph {
    pub id: u32,
    pub position: Point,
    pub advance: f32,
}
#[derive(Clone, Debug)]
pub struct GlyphRun {
    pub font: Arc<FontFace>,
    pub size: f32,
    pub variations: Arc<[i16]>,
    pub glyphs: Arc<[Glyph]>,
}
/// A colored logical-space triangle mesh. Colors interpolate as straight sRGB
/// before the renderer converts to linear premultiplied output. This supports
/// color controls and other continuous data without rasterizing a source image.
#[derive(Clone, Copy, Debug)]
pub struct PaintVertex {
    pub position: Point,
    pub color: Color,
}
pub use theframework_core::ImageColorFilter;
/// A source-image vertex in logical coordinates, independent of GPU buffers.
#[derive(Clone, Copy, Debug)]
pub struct ImageVertex {
    pub position: Point,
    pub uv: Point,
}
/// Analytic distance geometry, in local logical coordinates. A field chooses
/// the nearest primitive at each sample; this preserves union/material behavior
/// when primitives overlap rather than changing it to painter-order fills.
#[derive(Clone, Debug)]
pub enum DistanceGeometry {
    Circle { center: Point, radius: f32 },
    Hexagon { center: Point, radius: f32 },
    Rhombus { center: Point, half_size: Point },
    RoundedRect { rect: Rect, radii: [f32; 4] },
}
#[derive(Clone, Debug)]
pub struct DistancePrimitive {
    pub geometry: DistanceGeometry,
    pub fill: Color,
    pub border: Color,
    pub border_width: f32,
}
#[derive(Clone, Debug)]
pub struct DistanceField {
    pub background: Color,
    pub primitives: Arc<[DistancePrimitive]>,
}
#[derive(Clone, Debug)]
pub enum PaintOp {
    /// Circular alpha dilation behind an image. Width is in local logical units;
    /// samples outside the supplied UV crop are transparent. The caller paints
    /// the image separately and clips the silhouette to its widget if required.
    ImageOutline {
        image: ImageHandle,
        rect: Rect,
        uv: Rect,
        sampling: Sampling,
        width: f32,
        color: Color,
    },
    FilteredImage {
        image: ImageHandle,
        rect: Rect,
        uv: Rect,
        sampling: Sampling,
        tint: Color,
        filter: ImageColorFilter,
    },
    ImageMesh {
        vertices: Arc<[ImageVertex]>,
        indices: Arc<[u32]>,
        image: ImageHandle,
        sampling: Sampling,
        tint: Color,
    },
    DistanceField {
        rect: Rect,
        field: Arc<DistanceField>,
    },
    Mesh {
        vertices: Arc<[PaintVertex]>,
        indices: Arc<[u32]>,
    },
    Rect {
        rect: Rect,
        radius: f32,
        brush: Brush,
    },
    Fill {
        path: Arc<Path>,
        rule: FillRule,
        brush: Brush,
    },
    Stroke {
        path: Arc<Path>,
        stroke: Stroke,
    },
    Image {
        image: ImageHandle,
        rect: Rect,
        uv: Rect,
        sampling: Sampling,
        tint: Color,
    },
    GlyphRun {
        run: Arc<GlyphRun>,
        origin: Point,
        color: Color,
    },
    PushTransform(Transform),
    PopTransform,
    PushClip(Rect),
    PushPathClip(Arc<Path>, FillRule),
    PopClip,
    PushOpacity(f32),
    PopOpacity,
}
#[derive(Clone, Debug, Default)]
pub struct PaintList {
    pub ops: Vec<PaintOp>,
}
impl PaintList {
    pub fn rect(&mut self, rect: Rect, radius: f32, brush: impl Into<Brush>) {
        self.ops.push(PaintOp::Rect {
            rect,
            radius,
            brush: brush.into(),
        });
    }
    pub fn image(&mut self, image: ImageHandle, rect: Rect, sampling: Sampling) {
        self.ops.push(PaintOp::Image {
            image,
            rect,
            uv: Rect::new(0., 0., 1., 1.),
            sampling,
            tint: [1.; 4],
        });
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        let mut scopes = Vec::new();
        for op in &self.ops {
            match op {
                PaintOp::ImageOutline {
                    rect,
                    uv,
                    width,
                    color,
                    ..
                } => {
                    if [
                        rect.x,
                        rect.y,
                        rect.width,
                        rect.height,
                        uv.x,
                        uv.y,
                        uv.width,
                        uv.height,
                        *width,
                    ]
                    .iter()
                    .chain(color.iter())
                    .any(|v| !v.is_finite())
                        || *width < 0.
                        || rect.width <= 0.
                        || rect.height <= 0.
                        || uv.width <= 0.
                        || uv.height <= 0.
                    {
                        return Err("invalid image outline geometry");
                    }
                }
                PaintOp::FilteredImage {
                    filter,
                    tint,
                    rect,
                    uv,
                    ..
                } => {
                    if filter
                        .scale
                        .iter()
                        .chain(filter.bias.iter())
                        .chain(tint.iter())
                        .chain([
                            &filter.saturation,
                            &filter.alpha_scale,
                            &filter.minimum,
                            &rect.x,
                            &rect.y,
                            &rect.width,
                            &rect.height,
                            &uv.x,
                            &uv.y,
                            &uv.width,
                            &uv.height,
                        ])
                        .any(|x| !x.is_finite())
                    {
                        return Err("nonfinite image color adjustment");
                    }
                    if !(0. ..=1.).contains(&filter.alpha_scale) {
                        return Err("invalid image opacity attenuation");
                    }
                }
                PaintOp::ImageMesh {
                    vertices,
                    indices,
                    tint,
                    ..
                } => {
                    if indices.len() % 3 != 0
                        || indices.iter().any(|i| *i as usize >= vertices.len())
                    {
                        return Err("invalid image mesh indices");
                    }
                    if vertices
                        .iter()
                        .any(|v| v.position.iter().chain(v.uv.iter()).any(|x| !x.is_finite()))
                        || tint.iter().any(|x| !x.is_finite())
                    {
                        return Err("nonfinite image mesh data");
                    }
                }
                PaintOp::Mesh { vertices, indices } => {
                    if indices.len() % 3 != 0
                        || indices.iter().any(|i| *i as usize >= vertices.len())
                    {
                        return Err("invalid triangle mesh indices");
                    }
                    if vertices.iter().any(|v| {
                        v.position
                            .iter()
                            .chain(v.color.iter())
                            .any(|x| !x.is_finite())
                    }) {
                        return Err("nonfinite triangle mesh vertex");
                    }
                }
                PaintOp::PushTransform(_) => scopes.push(0),
                PaintOp::PushClip(_) | PaintOp::PushPathClip(..) => scopes.push(1),
                PaintOp::PushOpacity(alpha) => {
                    if !alpha.is_finite() {
                        return Err("invalid opacity");
                    }
                    scopes.push(2);
                }
                PaintOp::PopTransform => {
                    if scopes.pop() != Some(0) {
                        return Err("unbalanced transform");
                    }
                }
                PaintOp::PopClip => {
                    if scopes.pop() != Some(1) {
                        return Err("unbalanced clip");
                    }
                }
                PaintOp::PopOpacity => {
                    if scopes.pop() != Some(2) {
                        return Err("unbalanced opacity");
                    }
                }
                _ => {}
            }
        }
        if scopes.is_empty() {
            Ok(())
        } else {
            Err("unclosed paint scope")
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hit_testing_inverts_nested_paint_transform() {
        let t = Transform::scale(1.5, 1.5).then(Transform::translate(21., 17.));
        let p = [10., 20.];
        assert_eq!(t.inverse().unwrap().point(t.point(p)), p);
        assert!(Rect::new(0., 0., 20., 30.).contains(p));
    }
    #[test]
    fn paint_rejects_unbalanced_scopes() {
        let mut p = PaintList::default();
        p.ops.push(PaintOp::PopClip);
        assert!(p.validate().is_err());
        p.ops = vec![PaintOp::PushClip(Rect::default()), PaintOp::PopClip];
        assert!(p.validate().is_ok());
    }
}
