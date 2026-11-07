use super::*;
/// Elliptical cross sections in model coordinates, ordered from low Y to high Y.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub center: [f32; 3],
    pub radius: [f32; 2],
}
pub fn build(id: &str, sections: &[Section], segments: u32) -> Result<Mesh> {
    if !(2..=32).contains(&sections.len()) || !(4..=64).contains(&segments) {
        return Err(fail("Loft requires 2–32 sections and 4–64 segments"));
    }
    for (i, s) in sections.iter().enumerate() {
        finite(&s.center, 1000.)?;
        finite(&s.radius, 100.)?;
        if s.radius.iter().any(|r| *r < 0.001)
            || (i > 0 && s.center[1] - sections[i - 1].center[1] < 0.0001)
        {
            return Err(fail(
                "Loft radii must be positive and sections ordered by increasing height",
            ));
        }
    }
    let n = segments as usize;
    let point = |ring: usize, k: usize| {
        let (s, c) = (std::f32::consts::TAU * k as f32 / n as f32).sin_cos();
        let r = &sections[ring];
        [
            r.center[0] + c * r.radius[0],
            r.center[1],
            r.center[2] + s * r.radius[1],
        ]
    };
    let sub = |a: [f32; 3], b: [f32; 3]| std::array::from_fn(|i| a[i] - b[i]);
    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let normal = |ring: usize, k: usize| {
        let a = point(ring.saturating_sub(1), k);
        let b = point((ring + 1).min(sections.len() - 1), k);
        let tangent = sub(point(ring, (k + 1) % n), point(ring, (k + n - 1) % n));
        let v = cross(sub(b, a), tangent);
        let len = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.map(|v| v / len)
    };
    let mut vertices = vec![];
    for ring in 0..sections.len() - 1 {
        for k in 0..n {
            for (r, j) in [
                (ring, k),
                (ring + 1, k),
                (ring + 1, k + 1),
                (ring, k),
                (ring + 1, k + 1),
                (ring, k + 1),
            ] {
                vertices.push(Vertex {
                    position: point(r, j),
                    normal: normal(r, j),
                    // Image rows run downwards: V=0 is the top of a garment.
                    uv: [
                        j as f32 / n as f32,
                        1. - r as f32 / (sections.len() - 1) as f32,
                    ],
                    skin: None,
                });
            }
        }
    }
    for (ring, up) in [(0, false), (sections.len() - 1, true)] {
        for k in 0..n {
            let order = if up {
                [point(ring, (k + 1) % n), point(ring, k)]
            } else {
                [point(ring, k), point(ring, (k + 1) % n)]
            };
            for p in [sections[ring].center, order[0], order[1]] {
                vertices.push(Vertex {
                    position: p,
                    normal: [0., if up { 1. } else { -1. }, 0.],
                    uv: [
                        0.5 + (p[0] - sections[ring].center[0]) / (2. * sections[ring].radius[0]),
                        0.5 + (p[2] - sections[ring].center[2]) / (2. * sections[ring].radius[1]),
                    ],
                    skin: None,
                });
            }
        }
    }
    Ok(Mesh {
        parts: vec![Part {
            id: id.into(),
            label: id.into(),
            vertices,
            surface: None,
            texture: None,
        }],
        ..Default::default()
    })
}
