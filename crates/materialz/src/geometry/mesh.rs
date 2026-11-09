use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub skin: Option<super::rig::Weights>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub id: String,
    pub label: String,
    pub vertices: Vec<Vertex>,
    pub surface: Option<Surface>,
    pub texture: Option<Texture>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Texture {
    pub source: String,
    pub bundle: Option<String>,
    pub size: u32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub parts: Vec<Part>,
    pub connectors: Vec<connectors::Connector>,
    pub rig: Option<super::rig::Rig>,
    pub attachments: Vec<super::rig::Attachment>,
    pub grip: Option<Frame>,
    pub effects: Vec<effects::EffectAttachment>,
    /// Authored placement contour in mesh coordinates; never rendered as triangles.
    pub footprint: Option<footprint::Shape<[f32; 3]>>,
}
pub fn rotate(mut p: [f32; 3], angles: [f32; 3]) -> [f32; 3] {
    for (axis, a) in angles.into_iter().enumerate() {
        let (s, c) = a.to_radians().sin_cos();
        let i = (axis + 1) % 3;
        let j = (axis + 2) % 3;
        let (x, y) = (p[i], p[j]);
        p[i] = c * x - s * y;
        p[j] = s * x + c * y;
    }
    p
}
impl Mesh {
    /// Replace semantic parts on a shared rig, retaining each layer's sockets.
    pub fn replace_parts(&mut self, mut other: Mesh) -> Result<()> {
        self.align_rigs(&mut other)?;
        for part in other.parts {
            if let Some(target) = self.parts.iter_mut().find(|p| p.id == part.id) {
                *target = part;
            } else {
                self.parts.push(part);
            }
        }
        self.effects.extend(other.effects);
        for connector in other.connectors {
            if let Some(old) = self.connectors.iter_mut().find(|c| c.id == connector.id) {
                *old = connector;
            } else {
                self.connectors.push(connector);
            }
        }
        connectors::validate(&self.connectors)?;
        if other.footprint.is_some() {
            self.footprint = other.footprint;
        }
        for attachment in other.attachments {
            if let Some(target) = self.attachments.iter_mut().find(|a| a.id == attachment.id) {
                *target = attachment;
            } else {
                self.attachments.push(attachment);
            }
        }
        if self.vertex_count() > 200_000 {
            return Err(fail("Combined geometry exceeds 200000 vertices"));
        }
        Ok(())
    }
    /// Merge compatible skeleton branches and remap all skin/socket indices.
    pub fn align_rigs(&mut self, other: &mut Mesh) -> Result<()> {
        let rigs = self
            .rig
            .iter()
            .chain(other.rig.iter())
            .cloned()
            .collect::<Vec<_>>();
        if rigs.is_empty() {
            return Ok(());
        }
        let merged = super::rig::Rig::merge(&rigs)?;
        for mesh in [self, other] {
            if let Some(old) = &mesh.rig {
                let indices = old
                    .joints
                    .iter()
                    .map(|j| merged.index(&j.id))
                    .collect::<Result<Vec<_>>>()?;
                for v in mesh.parts.iter_mut().flat_map(|p| &mut p.vertices) {
                    if let Some(w) = &mut v.skin {
                        for j in &mut w.joints {
                            *j = indices[*j as usize] as u16;
                        }
                    }
                }
                for effect in &mut mesh.effects {
                    effect.joint = effect.joint.map(|j| indices[j]);
                }
                for a in &mut mesh.attachments {
                    a.joint = indices[a.joint];
                }
            }
            mesh.rig = Some(merged.clone());
        }
        Ok(())
    }
    pub fn vertex_count(&self) -> usize {
        self.parts.iter().map(|p| p.vertices.len()).sum()
    }
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::INFINITY; 3];
        let mut hi = [f32::NEG_INFINITY; 3];
        for v in self.parts.iter().flat_map(|p| &p.vertices) {
            for i in 0..3 {
                lo[i] = lo[i].min(v.position[i]);
                hi[i] = hi[i].max(v.position[i]);
            }
        }
        if !lo[0].is_finite() {
            if let Some(r) = &self.rig {
                for p in r.rest() {
                    for k in 0..3 {
                        lo[k] = lo[k].min(p.translation[k] - 0.5);
                        hi[k] = hi[k].max(p.translation[k] + 0.5);
                    }
                }
            } else {
                lo = [-0.5; 3];
                hi = [0.5; 3];
            }
        }
        (lo, hi)
    }
    pub fn transform(&mut self, t: [f32; 3], r: [f32; 3], s: [f32; 3]) {
        for connector in &mut self.connectors {
            connector.position = rotate(std::array::from_fn(|i| connector.position[i] * s[i]), r);
            for i in 0..3 {
                connector.position[i] += t[i];
            }
            let n = rotate(std::array::from_fn(|i| connector.direction[i] / s[i]), r);
            let len = n.iter().map(|v| v * v).sum::<f32>().sqrt();
            connector.direction = n.map(|v| v / len);
        }
        if let Some(points) = &mut self.footprint {
            for p in points.points_mut() {
                let rotated = rotate(std::array::from_fn(|i| p[i] * s[i]), r);
                *p = std::array::from_fn(|i| rotated[i] + t[i]);
            }
        }
        for effect in &mut self.effects {
            let p = rotate(
                std::array::from_fn(|i| effect.definition.position[i] * s[i]),
                r,
            );
            effect.definition.position = std::array::from_fn(|i| p[i] + t[i]);
            let scale = s.iter().copied().fold(0f32, f32::max);
            effect.definition.size *= scale;
            effect.definition.speed *= scale;
        }
        for v in self.parts.iter_mut().flat_map(|p| &mut p.vertices) {
            v.position = rotate(std::array::from_fn(|i| v.position[i] * s[i]), r);
            for i in 0..3 {
                v.position[i] += t[i];
            }
            let n = rotate(std::array::from_fn(|i| v.normal[i] / s[i]), r);
            let len = n.iter().map(|x| x * x).sum::<f32>().sqrt();
            v.normal = n.map(|x| x / len);
        }
    }
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn triangle(out: &mut Vec<Vertex>, p: [[f32; 3]; 3], uv: [[f32; 2]; 3]) {
    let n = cross(
        std::array::from_fn(|i| p[1][i] - p[0][i]),
        std::array::from_fn(|i| p[2][i] - p[0][i]),
    );
    let len = n.iter().map(|x| x * x).sum::<f32>().sqrt();
    if len < 1e-10 {
        return;
    }
    for i in 0..3 {
        out.push(Vertex {
            position: p[i],
            normal: n.map(|x| x / len),
            uv: uv[i],
            skin: None,
        });
    }
}
fn area(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
pub fn validate_contour(points: &[[f32; 2]]) -> Result<()> {
    if !(3..=64).contains(&points.len()) {
        return Err(fail("Contour needs 3–64 points"));
    }
    for p in points {
        finite(p, 1000.)?;
    }
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        if (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-5 {
            return Err(fail("Contour has coincident points"));
        }
        if area(points[(i + points.len() - 1) % points.len()], a, b).abs() < 1e-8 {
            return Err(fail("Remove collinear contour points"));
        }
        for j in i + 1..points.len() {
            if j == i + 1 || (i == 0 && j == points.len() - 1) {
                continue;
            }
            let c = points[j];
            let d = points[(j + 1) % points.len()];
            let orientations = [area(a, b, c), area(a, b, d), area(c, d, a), area(c, d, b)];
            if orientations[0] * orientations[1] <= 0. && orientations[2] * orientations[3] <= 0. {
                // Include touching and overlapping boundaries, but not separated collinear spans.
                if (0..2).all(|k| {
                    a[k].min(b[k]) <= c[k].max(d[k]) + 1e-7
                        && c[k].min(d[k]) <= a[k].max(b[k]) + 1e-7
                }) {
                    return Err(fail("Contour intersects or touches itself"));
                }
            }
        }
    }
    let signed = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f32>();
    if signed.abs() < 1e-7 {
        return Err(fail("Contour has no area"));
    }
    Ok(())
}
fn contour(points: &[[f32; 2]]) -> Result<Vec<[f32; 2]>> {
    validate_contour(points)?;
    let mut p = points.to_vec();
    if p.iter()
        .zip(p.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f32>()
        < 0.
    {
        p.reverse();
    }
    Ok(p)
}
fn triangulate(p: &[[f32; 2]]) -> Result<Vec<[usize; 3]>> {
    let mut indices = (0..p.len()).collect::<Vec<_>>();
    let mut triangles = vec![];
    while indices.len() > 3 {
        let mut ear = None;
        for i in 0..indices.len() {
            let a = indices[(i + indices.len() - 1) % indices.len()];
            let b = indices[i];
            let c = indices[(i + 1) % indices.len()];
            if area(p[a], p[b], p[c]) <= 1e-8 {
                continue;
            }
            let inside = indices.iter().any(|&k| {
                k != a
                    && k != b
                    && k != c
                    && area(p[a], p[b], p[k]) >= -1e-8
                    && area(p[b], p[c], p[k]) >= -1e-8
                    && area(p[c], p[a], p[k]) >= -1e-8
            });
            if !inside {
                ear = Some((i, [a, b, c]));
                break;
            }
        }
        let Some((i, t)) = ear else {
            return Err(fail("Could not triangulate contour"));
        };
        triangles.push(t);
        indices.remove(i);
    }
    triangles.push([indices[0], indices[1], indices[2]]);
    Ok(triangles)
}
fn finish(id: &str, vertices: Vec<Vertex>) -> Mesh {
    Mesh {
        parts: vec![Part {
            id: id.into(),
            label: id.into(),
            vertices,
            surface: None,
            texture: None,
        }],
        ..Default::default()
    }
}
pub fn extrude(id: &str, points: &[[f32; 2]], depth: f32) -> Result<Mesh> {
    if !depth.is_finite() || !(0.001..=100.).contains(&depth) {
        return Err(fail("Extrusion depth must be 0.001–100 meters"));
    }
    let p = contour(points)?;
    let triangles = triangulate(&p)?;
    let mut out = vec![];
    let lo = [
        p.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min),
        p.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min),
    ];
    let hi = [
        p.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max),
        p.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max),
    ];
    let uv = |q: [f32; 2]| {
        [
            (q[0] - lo[0]) / (hi[0] - lo[0]),
            (q[1] - lo[1]) / (hi[1] - lo[1]),
        ]
    };
    for t in triangles {
        for front in [true, false] {
            let ids = if front { t } else { [t[2], t[1], t[0]] };
            triangle(
                &mut out,
                ids.map(|i| {
                    [
                        p[i][0],
                        p[i][1],
                        if front { depth / 2. } else { -depth / 2. },
                    ]
                }),
                ids.map(|i| uv(p[i])),
            );
        }
    }
    for i in 0..p.len() {
        let a = p[i];
        let b = p[(i + 1) % p.len()];
        let v = [
            [a[0], a[1], -depth / 2.],
            [b[0], b[1], -depth / 2.],
            [b[0], b[1], depth / 2.],
            [a[0], a[1], depth / 2.],
        ];
        triangle(&mut out, [v[0], v[1], v[2]], [[0., 0.], [1., 0.], [1., 1.]]);
        triangle(&mut out, [v[0], v[2], v[3]], [[0., 0.], [1., 1.], [0., 1.]]);
    }
    Ok(finish(id, out))
}
pub fn box_mesh(id: &str, size: [f32; 3]) -> Result<Mesh> {
    finite(&size, 100.)?;
    if size.iter().any(|v| *v < 0.001) {
        return Err(fail("Box dimensions must be 0.001–100 meters"));
    }
    let mut m = extrude(
        id,
        &[
            [-size[0] / 2., -size[1] / 2.],
            [size[0] / 2., -size[1] / 2.],
            [size[0] / 2., size[1] / 2.],
            [-size[0] / 2., size[1] / 2.],
        ],
        size[2],
    )?;
    m.parts[0].id = id.into();
    Ok(m)
}
pub fn revolve(id: &str, points: &[[f32; 2]], segments: u32) -> Result<Mesh> {
    let p = contour(points)?;
    if !(3..=64).contains(&segments) || p.iter().any(|p| p[0] < 0.) {
        return Err(fail("Revolve requires nonnegative radii and 3–64 segments"));
    }
    let mut out = vec![];
    for i in 0..p.len() {
        let a = p[i];
        let b = p[(i + 1) % p.len()];
        for s in 0..segments {
            let point = |p: [f32; 2], s: u32| {
                let (sn, cs) = (std::f32::consts::TAU * s as f32 / segments as f32).sin_cos();
                [p[0] * cs, p[1], p[0] * sn]
            };
            let v = [point(a, s), point(b, s), point(b, s + 1), point(a, s + 1)];
            let u = s as f32 / segments as f32;
            let u1 = (s + 1) as f32 / segments as f32;
            triangle(&mut out, [v[0], v[1], v[2]], [[u, 0.], [u, 1.], [u1, 1.]]);
            triangle(&mut out, [v[0], v[2], v[3]], [[u, 0.], [u1, 1.], [u1, 0.]]);
        }
    }
    Ok(finish(id, out))
}
