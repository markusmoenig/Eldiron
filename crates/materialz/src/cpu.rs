//! Scalar reference backend. Evaluates callable nodes at the requested coordinates.
#![allow(clippy::approx_constant)] // Preserve the curated shaders' literal constants.
use crate::{
    Error, Result,
    ir::{Binding, Channel, NodeId, Program, Reference},
    metadata::Kernel,
};

#[derive(Clone, Copy)]
pub(crate) struct SampleContext {
    pub position: [f32; 3],
    pub auxiliary: [[f32; 3]; 3],
    pub valid: bool,
    pub normal: [f32; 3],
    pub time: f32,
    pub cell: [f32; 3],
    pub group: [f32; 3],
    pub instance_valid: bool,
    pub origin: [f32; 3],
    pub axis_x: [f32; 3],
    pub axis_y: [f32; 3],
    pub footprint: Option<f32>,
}
impl SampleContext {
    fn at(uv: [f32; 2]) -> Self {
        Self {
            position: [uv[0], uv[1], 0.],
            auxiliary: [[0.; 3]; 3],
            valid: false,
            normal: [0., 0., 1.],
            time: 0.,
            cell: [0.; 3],
            group: [0.; 3],
            instance_valid: false,
            origin: [0.; 3],
            axis_x: [1., 0., 0.],
            axis_y: [0., 1., 0.],
            footprint: None,
        }
    }
}
pub(crate) fn compose_frame(
    context: SampleContext,
    origin: [f32; 3],
    x: [f32; 3],
    y: [f32; 3],
) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let vector = |v: [f32; 3]| {
        std::array::from_fn(|i| {
            context.axis_x[i] * v[0] + context.axis_y[i] * v[1] + if i == 2 { v[2] } else { 0. }
        })
    };
    let o: [f32; 3] = vector(origin);
    (
        std::array::from_fn(|i| context.origin[i] + o[i]),
        vector(x),
        vector(y),
    )
}
pub type Vec3 = [f32; 3];
pub const EPSILON: f32 = 1e-6;
/// Bounded alpha-mask erosion stencil: 16 directions on three concentric rings.
pub(crate) const BORDER_DIRECTIONS: [[f32; 2]; 16] = [
    [1., 0.],
    [0.9238795, 0.38268343],
    [0.70710677, 0.70710677],
    [0.38268343, 0.9238795],
    [0., 1.],
    [-0.38268343, 0.9238795],
    [-0.70710677, 0.70710677],
    [-0.9238795, 0.38268343],
    [-1., 0.],
    [-0.9238795, -0.38268343],
    [-0.70710677, -0.70710677],
    [-0.38268343, -0.9238795],
    [0., -1.],
    [0.38268343, -0.9238795],
    [0.70710677, -0.70710677],
    [0.9238795, -0.38268343],
];
pub fn nonzero(v: f32) -> f32 {
    if v.abs() < EPSILON {
        if v < 0. { -EPSILON } else { EPSILON }
    } else {
        v
    }
}
fn fract(v: f32) -> f32 {
    v - v.floor()
}

/// Port of Brian Sharpe's Wombat Perlin2D as curated in Material-Z.
/// Explicit per-lane operations preserve GLSL's swizzle order.
pub fn perlin(uv: [f32; 2], global: f32, local: [f32; 2]) -> f32 {
    let p = [
        uv[0] / nonzero(global) / nonzero(local[0]) * 10.,
        uv[1] / nonzero(global) / nonzero(local[1]) * 10.,
    ];
    let pi = [p[0].floor(), p[1].floor()];
    let f = [p[0] - pi[0], p[1] - pi[1]];
    let mut pt = [pi[0], pi[1], pi[0] + 1., pi[1] + 1.];
    for (i, v) in pt.iter_mut().enumerate() {
        *v -= (*v * (1. / 71.)).floor() * 71.;
        *v += if i % 2 == 0 { 26. } else { 161. };
        *v *= *v;
    }
    let hash = [pt[0] * pt[1], pt[2] * pt[1], pt[0] * pt[3], pt[2] * pt[3]];
    let dx = [f[0], f[0] - 1., f[0], f[0] - 1.];
    let dy = [f[1], f[1], f[1] - 1., f[1] - 1.];
    let mut g = [0.; 4];
    for i in 0..4 {
        let x = fract(hash[i] * (1. / 951.135_7)) - 0.49999;
        let y = fract(hash[i] * (1. / 642.949_9)) - 0.49999;
        g[i] = (1. / (x * x + y * y).sqrt()) * (x * dx[i] + y * dy[i]);
        g[i] *= std::f32::consts::SQRT_2;
    }
    let fade = |v: f32| v * v * v * (v * (v * 6. - 15.) + 10.);
    let bx = fade(f[0]);
    let by = fade(f[1]);
    let weights = [
        (1. - bx) * (1. - by),
        bx * (1. - by),
        (1. - bx) * by,
        bx * by,
    ];
    let sum = ((g[0] * weights[0] + g[1] * weights[1]) + g[2] * weights[2]) + g[3] * weights[3];
    (sum + 1.) / 2.
}

pub(crate) fn uint_bits(v: f32) -> u32 {
    if v < 0. {
        0u32.wrapping_sub((-v).trunc().min(4_294_967_040.) as u32)
    } else {
        v.trunc().min(4_294_967_040.) as u32
    }
}
pub(crate) fn hash3(mut x: [u32; 3]) -> [f32; 3] {
    for _ in 0..3 {
        x = [
            (x[0] >> 8 ^ x[1]).wrapping_mul(1_103_515_245),
            (x[1] >> 8 ^ x[2]).wrapping_mul(1_103_515_245),
            (x[2] >> 8 ^ x[0]).wrapping_mul(1_103_515_245),
        ];
    }
    x.map(|v| v as f32 * (1. / u32::MAX as f32))
}
pub fn disc(
    uv: [f32; 2],
    bevel: f32,
    gradient: bool,
    strength: f32,
    global: f32,
    local: [f32; 2],
    offset: [f32; 2],
) -> f32 {
    disc_with_context(
        uv, bevel, gradient, strength, global, local, offset, [0.; 3],
    )
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn disc_with_context(
    uv: [f32; 2],
    bevel: f32,
    gradient: bool,
    strength: f32,
    global: f32,
    local: [f32; 2],
    offset: [f32; 2],
    context: [f32; 3],
) -> f32 {
    let u = [
        uv[0] / nonzero(global) * local[0] - offset[0],
        uv[1] / nonzero(global) * local[1] - offset[1],
    ];
    let radius = if context[0] == 0. && context[1] == 0. {
        0.5
    } else {
        context[1] / 2.
    };
    let mut o = ((radius - (u[0] * u[0] + u[1] * u[1]).sqrt()) * 2. / nonzero(bevel)).clamp(0., 1.);
    if gradient {
        // Standalone shape context: Box=(1,1), element ID=0, matching the original default.
        let h = hash3([
            (context[2] * u32::MAX as f32) as u32,
            (0.3f32 * u32::MAX as f32) as u32,
            0,
        ]);
        let r = h.map(|x| -1. + 2. * x);
        let angle = (180. * r[0]).to_radians();
        let amp = 2.0f32.powf(r[1]);
        let grad = [
            (angle + 1.5707).sin() * amp,
            (angle + 2. * 1.5707).sin() * amp,
        ];
        o *= 3. - strength * 3. + grad[0] * u[0] + grad[1] * u[1];
    }
    o
}

/// Straight-alpha image data, plus a compile-time-validated optional distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TileSample {
    pub color: [f32; 4],
    pub distance: Option<f32>,
}
impl TileSample {
    fn rgb(color: [f32; 3]) -> Self {
        Self {
            color: [color[0], color[1], color[2], 1.],
            distance: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewMode {
    Image,
    Shape,
    Distance,
}
impl PreviewMode {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "image" => Ok(Self::Image),
            "shape" => Ok(Self::Shape),
            "distance" => Ok(Self::Distance),
            _ => Err(Error(
                "Preview mode must be image, shape or distance".into(),
            )),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Shape => "shape",
            Self::Distance => "distance",
        }
    }
}
pub fn coverage(distance: f32, pixel_width: f32) -> f32 {
    let t = (0.5 + distance / nonzero(pixel_width)).clamp(0., 1.);
    1. - t * t * (3. - 2. * t)
}
fn over(front: [f32; 4], back: [f32; 4]) -> [f32; 4] {
    let alpha = front[3] + back[3] * (1. - front[3]);
    if alpha <= 0. {
        return [0.; 4];
    }
    [
        (front[0] * front[3] + back[0] * back[3] * (1. - front[3])) / alpha,
        (front[1] * front[3] + back[1] * back[3] * (1. - front[3])) / alpha,
        (front[2] * front[3] + back[2] * back[3] * (1. - front[3])) / alpha,
        alpha,
    ]
}
pub fn combine_distance(a: f32, b: f32, mode: crate::format::Combine, smoothness: f32) -> f32 {
    use crate::format::Combine;
    match mode {
        Combine::Replace => b,
        Combine::Union => a.min(b),
        Combine::Subtract => a.max(-b),
        Combine::Intersect => a.max(b),
        Combine::SmoothUnion => {
            let k = smoothness.max(EPSILON);
            let h = (0.5 + 0.5 * (b - a) / k).clamp(0., 1.);
            b * (1. - h) + a * h - k * h * (1. - h)
        }
    }
}
fn dimensions(width: u32, height: u32) -> Result<usize> {
    let count = u64::from(width) * u64::from(height);
    if width == 0 || height == 0 || count > 16_777_216 {
        return Err(Error(
            "Render needs 1..16777216 pixels and nonzero dimensions".into(),
        ));
    }
    Ok(count as usize)
}
fn uv(x: u32, y: u32, width: u32, height: u32) -> [f32; 2] {
    [
        (x as f32 + 0.5) / width as f32 - 0.5,
        (y as f32 + 0.5) / height as f32 - 0.5,
    ]
}
fn finite(sample: TileSample) -> Result<TileSample> {
    if sample.color.iter().any(|v| !v.is_finite())
        || sample.distance.is_some_and(|v| !v.is_finite())
    {
        return Err(Error("Non-finite tile result".into()));
    }
    Ok(sample)
}

pub struct Evaluator<'a> {
    program: &'a Program,
}
impl<'a> Evaluator<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self { program }
    }
    fn reference(&self, r: &Reference, uv: [f32; 2], aa: f32) -> Vec<f32> {
        self.reference_at(r, uv, aa, SampleContext::at(uv))
    }
    fn reference_at(
        &self,
        r: &Reference,
        uv: [f32; 2],
        aa: f32,
        context: SampleContext,
    ) -> Vec<f32> {
        let sample = self.node_at(r.node, uv, aa, context);
        self.select(r, sample)
    }
    fn select(&self, r: &Reference, sample: TileSample) -> Vec<f32> {
        r.components
            .iter()
            .map(|&i| {
                let v = if r.distance {
                    sample.distance.expect("Validated distance reference")
                } else {
                    sample.color[i]
                };
                if let Some((lo, hi)) = r.mapping {
                    lo + v * (hi - lo)
                } else {
                    v
                }
            })
            .collect()
    }
    fn node(&self, id: NodeId, uv: [f32; 2], aa: f32) -> TileSample {
        self.node_at(id, uv, aa, SampleContext::at(uv))
    }
    fn node_at(
        &self,
        id: NodeId,
        mut uv: [f32; 2],
        mut aa: f32,
        mut context: SampleContext,
    ) -> TileSample {
        let n = &self.program.nodes[id.0];
        if let Some(c) = &n.coordinates {
            let v = self.reference_at(c, uv, aa, context);
            let mut scale = 0f32;
            for axis in 0..2 {
                let mut tap = uv;
                tap[axis] += aa;
                let mut tap_context = context;
                tap_context.position[axis] += aa;
                let next = self.reference_at(c, tap, aa, tap_context);
                let width = v
                    .iter()
                    .zip(next)
                    .map(|(a, b)| (b - a) * (b - a))
                    .sum::<f32>()
                    .sqrt();
                scale = scale.max(width);
            }
            aa = scale.max(0.000001);
            uv = [v[0], v[1]];
            context.position[0] = uv[0];
            context.position[1] = uv[1];
            if v.len() == 3 {
                context.position[2] = v[2];
            }
        }
        let parameter_samples = std::cell::RefCell::new(Vec::<(NodeId, TileSample)>::new());
        let parameter_binding = |binding: &Binding| match binding {
            Binding::Constant(value) => value.clone(),
            Binding::From(reference) => {
                let cached = parameter_samples
                    .borrow()
                    .iter()
                    .find(|(id, _)| *id == reference.node)
                    .map(|(_, sample)| *sample);
                let sample = if let Some(sample) = cached {
                    sample
                } else {
                    let sample = self.node_at(reference.node, uv, aa, context);
                    parameter_samples
                        .borrow_mut()
                        .push((reference.node, sample));
                    sample
                };
                self.select(reference, sample)
            }
        };
        let p = |name| {
            let mut v = parameter_binding(n.parameter(name));
            if let Some(p) = n.parameters.iter().find(|p| p.id == name)
                && p.choice_count > 0
            {
                v[0] = v[0].round().clamp(0., (p.choice_count - 1) as f32);
            }
            v
        };
        let scalar = |name| p(name)[0];
        let rgba = |name| {
            let v = p(name);
            [v[0], v[1], v[2], v[3]]
        };
        let input = || self.node_at(n.input.as_ref().unwrap().node, uv, aa, context);
        // Exact specializations of the canonical MZ tiles. Edited algorithms still
        // use the interpreter; ruleset material bakes avoid interpreting the same
        // scalar Perlin/vector arithmetic hundreds of thousands of times.
        let canonical = n.code == crate::tilelang::source(n.kernel);
        let fast = canonical
            && (matches!(n.kernel, Kernel::Color | Kernel::ColorBlend)
                || (n.kernel == Kernel::Perlin && scalar("dimensions") < 0.5));
        let mut result = if fast {
            let previous = if n.input.is_some() {
                input()
            } else {
                TileSample {
                    color: [1.; 4],
                    distance: None,
                }
            };
            let color = match n.kernel {
                Kernel::Color => {
                    let c = rgba("color");
                    [
                        c[0],
                        c[1],
                        c[2],
                        c[3].clamp(0., 1.) * previous.color[3].clamp(0., 1.),
                    ]
                }
                Kernel::Perlin => {
                    let v = p("localScale");
                    let noise = perlin(uv, scalar("globalScale"), [v[0], v[1]]);
                    [noise, noise, noise, previous.color[3]]
                }
                Kernel::ColorBlend => {
                    let a = rgba("color1");
                    let b = rgba("color2");
                    let factor = p("factor");
                    let rgb = n
                        .input
                        .as_ref()
                        .map(|r| self.select(r, previous))
                        .unwrap_or_else(|| vec![1.; 3]);
                    let w: [f32; 3] = std::array::from_fn(|i| rgb[i] * factor[i]);
                    let alpha = (w.iter().sum::<f32>() / 3.).clamp(0., 1.);
                    [
                        a[0] + (b[0] - a[0]) * w[0],
                        a[1] + (b[1] - a[1]) * w[1],
                        a[2] + (b[2] - a[2]) * w[2],
                        (a[3] + (b[3] - a[3]) * alpha) * previous.color[3],
                    ]
                }
                _ => unreachable!(),
            };
            TileSample {
                color,
                distance: previous.distance,
            }
        } else if !n.algorithm.primitive {
            let parameters = n
                .parameters
                .iter()
                .map(|p| {
                    let mut v = parameter_binding(&p.binding);
                    if p.choice_count > 0 {
                        v[0] = v[0].round().clamp(0., (p.choice_count - 1) as f32);
                    }
                    v
                })
                .collect();
            let scale = if n.kernel == Kernel::Transform && n.uniform_distance {
                (scalar("globalScale") * p("localScale")[0]).abs()
            } else {
                1.
            };
            let input_scale = if n.kernel == Kernel::Transform
                && n.code == crate::tilelang::source(Kernel::Transform)
            {
                let local = p("localScale");
                (scalar("globalScale").abs() * local[0].abs().min(local[1].abs())).max(0.000001)
            } else {
                1.
            };
            let frame = if n.kernel == Kernel::Transform
                && n.code == crate::tilelang::source(Kernel::Transform)
            {
                let a = scalar("rot") * 0.017_453_292_f32;
                let c = crate::math::sin(1.5707963 + a);
                let s = crate::math::sin(a);
                let local = p("localScale");
                let g = scalar("globalScale");
                let offset = p("offset");
                compose_frame(
                    context,
                    [offset[0], offset[1], 0.],
                    [c * g * local[0], s * g * local[1], 0.],
                    [-s * g * local[0], c * g * local[1], 0.],
                )
            } else {
                (context.origin, context.axis_x, context.axis_y)
            };
            let mut previous = TileSample::rgb([1.; 3]);
            let mut captured_input = false;
            let (image, shape) =
                n.algorithm
                    .evaluate(uv, aa, parameters, context, |domain, supplied| {
                        if let Some(reference) = &n.input {
                            let mut c = supplied.unwrap_or(context);
                            if supplied.is_none() {
                                c.origin = frame.0;
                                c.axis_x = frame.1;
                                c.axis_y = frame.2;
                                c.position[0] = domain[0];
                                c.position[1] = domain[1];
                            }
                            let sampled = self.node_at(
                                reference.node,
                                domain,
                                c.footprint.take().unwrap_or(aa / nonzero(input_scale)),
                                c,
                            );
                            // Filter taps must not replace the input's distance field.
                            if !captured_input {
                                previous = sampled;
                                captured_input = true;
                            }
                            let rgb = self.select(reference, sampled);
                            [rgb[0], rgb[1], rgb[2], sampled.color[3]]
                        } else {
                            [1.; 4]
                        }
                    });
            if let Some(d) = shape {
                let d = if n.combine == crate::format::Combine::Replace {
                    d
                } else {
                    combine_distance(
                        previous.distance.unwrap(),
                        d,
                        n.combine,
                        scalar("smoothness"),
                    )
                };
                TileSample {
                    color: [1., 1., 1., coverage(d, aa)],
                    distance: Some(d),
                }
            } else {
                TileSample {
                    color: image.unwrap(),
                    distance: previous.distance.map(|d| d * scale),
                }
            }
        } else {
            match n.kernel {
                Kernel::Border if n.input.is_none() => TileSample::rgb([1.; 3]),
                Kernel::Border => {
                    let mut prev = input();
                    let rgb = self.select(n.input.as_ref().unwrap(), prev);
                    prev.color[..3].copy_from_slice(&rgb);
                    let w = scalar("width").max(0.);
                    let (outer, band) = if let Some(d) = prev.distance {
                        let outer = coverage(d, aa);
                        (outer, (outer - coverage(d + w, aa)).clamp(0., 1.))
                    } else {
                        let outer = prev.color[3].clamp(0., 1.);
                        let mut eroded = outer;
                        if w > 0. {
                            for ring in 1..=3 {
                                for dir in BORDER_DIRECTIONS {
                                    let radius = w * ring as f32 / 3.;
                                    let tap = [uv[0] + dir[0] * radius, uv[1] + dir[1] * radius];
                                    let mut tap_context = context;
                                    tap_context.position[0] = tap[0];
                                    tap_context.position[1] = tap[1];
                                    let alpha = self
                                        .node_at(
                                            n.input.as_ref().unwrap().node,
                                            tap,
                                            aa,
                                            tap_context,
                                        )
                                        .color[3];
                                    eroded = eroded.min(alpha.clamp(0., 1.));
                                }
                            }
                        }
                        (outer, (outer - eroded).clamp(0., 1.))
                    };
                    let mut color = rgba("color");
                    if outer > 0. {
                        color[3] = color[3].clamp(0., 1.) * band / outer;
                        let mut back = prev.color;
                        back[3] = (back[3] / outer).clamp(0., 1.);
                        prev.color = over(color, back);
                        prev.color[3] *= outer;
                    }
                    prev
                }
                _ => unreachable!("Only Border uses the bounded sampling primitive"),
            }
        };
        if !n.has_distance {
            result.distance = None;
        }
        result
    }
    pub fn sample_tile(
        &self,
        material: &str,
        row: &str,
        tile: usize,
        uv: [f32; 2],
        pixel_width: f32,
    ) -> Result<TileSample> {
        if uv.iter().any(|v| !v.is_finite()) || !pixel_width.is_finite() || pixel_width <= 0. {
            return Err(Error(
                "Finite coordinates and positive pixel width required".into(),
            ));
        }
        finite(self.node(self.program.tile(material, row, tile)?, uv, pixel_width))
    }
    pub fn sample(&self, material: &str, channel: Channel, uv: [f32; 2]) -> Result<Vec<f32>> {
        self.sample_with_width(material, channel, uv, 0.001)
    }
    /// Evaluate an animated recipe at normalized loop time. Loop Flow repeats at 1.
    pub fn sample_at_time(
        &self,
        material: &str,
        channel: Channel,
        uv: [f32; 2],
        time: f32,
    ) -> Result<Vec<f32>> {
        if !time.is_finite() || uv.iter().any(|v| !v.is_finite()) {
            return Err(Error("Coordinates and time must be finite".into()));
        }
        let m = self.program.material(material)?;
        let Some((_, reference)) = m.outputs.iter().find(|(c, _)| *c == channel) else {
            return Ok(m.default_value(channel)?.to_vec());
        };
        let mut context = SampleContext::at(uv);
        context.time = time;
        let value = self.reference_at(reference, uv, 0.001, context);
        if value.iter().any(|v| !v.is_finite()) {
            return Err(Error("Non-finite animation result".into()));
        }
        Ok(value)
    }
    /// Reference sampling with the same UV footprint as a region-render request.
    pub fn sample_with_footprint(
        &self,
        material: &str,
        channel: Channel,
        uv: [f32; 2],
        footprint: f32,
    ) -> Result<Vec<f32>> {
        if !footprint.is_finite() || footprint <= 0. {
            return Err(Error("Pixel footprint must be finite and positive".into()));
        }
        self.sample_with_width(material, channel, uv, footprint)
    }
    pub fn sample_with_footprint_at_time(
        &self,
        material: &str,
        channel: Channel,
        uv: [f32; 2],
        footprint: f32,
        time: f32,
    ) -> Result<Vec<f32>> {
        if !footprint.is_finite()
            || footprint <= 0.
            || !time.is_finite()
            || uv.iter().any(|v| !v.is_finite())
        {
            return Err(Error("Invalid animated sample domain".into()));
        }
        let m = self.program.material(material)?;
        let Some((_, r)) = m.outputs.iter().find(|(c, _)| *c == channel) else {
            return Ok(m.default_value(channel)?.to_vec());
        };
        let mut context = SampleContext::at(uv);
        context.time = time;
        let value = self.reference_at(r, uv, footprint, context);
        if value.iter().any(|v| !v.is_finite()) {
            return Err(Error("Non-finite animation result".into()));
        }
        Ok(value)
    }
    fn sample_with_width(
        &self,
        material: &str,
        channel: Channel,
        uv: [f32; 2],
        aa: f32,
    ) -> Result<Vec<f32>> {
        if uv.iter().any(|v| !v.is_finite()) {
            return Err(Error("Coordinates must be finite".into()));
        }
        let m = self.program.material(material)?;
        let Some((_, r)) = m.outputs.iter().find(|(c, _)| *c == channel) else {
            return Ok(m.default_value(channel)?.to_vec());
        };
        let value = self.reference(r, uv, aa);
        if value.iter().any(|v| !v.is_finite()) {
            return Err(Error("Non-finite evaluation result".into()));
        }
        Ok(value)
    }
    pub fn render(
        &self,
        material: &str,
        channel: Channel,
        width: u32,
        height: u32,
    ) -> Result<Vec<Vec<f32>>> {
        let mut pixels = Vec::with_capacity(dimensions(width, height)?);
        let aa = 1. / width.min(height) as f32;
        for y in 0..height {
            for x in 0..width {
                pixels.push(self.sample_with_width(
                    material,
                    channel,
                    uv(x, y, width, height),
                    aa,
                )?);
            }
        }
        Ok(pixels)
    }
    /// The same thumbnail path works for image-only and distance-producing tiles.
    pub fn render_tile(
        &self,
        material: &str,
        row: &str,
        tile: usize,
        mode: PreviewMode,
        width: u32,
        height: u32,
    ) -> Result<Vec<[f32; 4]>> {
        let mut pixels = Vec::with_capacity(dimensions(width, height)?);
        let id = self.program.tile(material, row, tile)?;
        let periodic_program = if mode == PreviewMode::Image
            && self.program.material(material)?.tiling.mode != crate::periodic::Mode::Raw
        {
            let mut copy = self.program.clone();
            let tiling = copy.material(material)?.tiling;
            crate::periodic::append(
                &mut copy,
                crate::ir::Reference {
                    field: None,
                    node: id,
                    components: vec![0, 1, 2],
                    mapping: None,
                    distance: false,
                },
                tiling,
                row,
            )?;
            Some(copy)
        } else {
            None
        };
        let evaluator = Evaluator::new(periodic_program.as_ref().unwrap_or(self.program));
        let id = periodic_program
            .as_ref()
            .map(|p| NodeId(p.nodes.len() - 1))
            .unwrap_or(id);
        if mode != PreviewMode::Image && !self.program.nodes[id.0].has_distance {
            return Err(Error(
                "This tile has no valid distance preview; use image mode".into(),
            ));
        }
        let aa = 1. / width.min(height) as f32;
        for y in 0..height {
            for x in 0..width {
                let sample = finite(evaluator.node(id, uv(x, y, width, height), aa))?;
                pixels.push(match mode {
                    PreviewMode::Image => sample.color,
                    PreviewMode::Shape => {
                        let c = coverage(sample.distance.unwrap(), aa);
                        [c, c, c, 1.]
                    }
                    PreviewMode::Distance => {
                        let c = (0.5 + sample.distance.unwrap() * 4.).clamp(0., 1.);
                        [c, c, c, 1.]
                    }
                });
            }
        }
        Ok(pixels)
    }
}
