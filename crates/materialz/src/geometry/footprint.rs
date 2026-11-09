//! Portable compound footprints. Regions are a union; holes subtract from their own outer ring.
use super::{Result, fail, validate_contour};
use serde::{Deserialize, Serialize};
pub const MAX_REGIONS: usize = 128;
pub const MAX_RINGS: usize = 256;
pub const MAX_POINTS: usize = 4096;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "P: Deserialize<'de>"))]
pub struct Region<P> {
    pub outer: Vec<P>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holes: Vec<Vec<P>>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, bound(deserialize = "P: Deserialize<'de>"))]
pub struct Shape<P> {
    pub regions: Vec<Region<P>>,
}
impl<P> Shape<P> {
    pub fn single(outer: Vec<P>) -> Self {
        Self {
            regions: vec![Region {
                outer,
                holes: vec![],
            }],
        }
    }
    pub fn points(&self) -> impl Iterator<Item = &P> {
        self.regions
            .iter()
            .flat_map(|r| r.outer.iter().chain(r.holes.iter().flatten()))
    }
    pub fn points_mut(&mut self) -> impl Iterator<Item = &mut P> {
        self.regions
            .iter_mut()
            .flat_map(|r| r.outer.iter_mut().chain(r.holes.iter_mut().flatten()))
    }
    pub fn map<Q>(&self, mut f: impl FnMut(&P) -> Q) -> Shape<Q> {
        Shape {
            regions: self
                .regions
                .iter()
                .map(|r| Region {
                    outer: r.outer.iter().map(&mut f).collect(),
                    holes: r
                        .holes
                        .iter()
                        .map(|h| h.iter().map(&mut f).collect())
                        .collect(),
                })
                .collect(),
        }
    }
    pub fn layout(&self) -> Vec<Vec<usize>> {
        self.regions
            .iter()
            .map(|r| {
                std::iter::once(r.outer.len())
                    .chain(r.holes.iter().map(Vec::len))
                    .collect()
            })
            .collect()
    }
    pub fn limits(&self) -> Result<()> {
        if self.regions.is_empty()
            || self.regions.len() > MAX_REGIONS
            || self
                .regions
                .iter()
                .map(|r| 1 + r.holes.len())
                .sum::<usize>()
                > MAX_RINGS
            || self.points().count() > MAX_POINTS
        {
            return Err(fail(
                "Footprint supports 1–128 regions, at most 256 rings and 4096 points",
            ));
        }
        Ok(())
    }
}
pub fn in_ring(ring: &[[f32; 2]], p: [f32; 2]) -> bool {
    let mut inside = false;
    for (a, b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        if (a[1] > p[1]) != (b[1] > p[1])
            && (p[0] as f64)
                < a[0] as f64
                    + (p[1] as f64 - a[1] as f64) * (b[0] as f64 - a[0] as f64)
                        / (b[1] as f64 - a[1] as f64)
        {
            inside = !inside;
        }
    }
    inside
}
pub fn contains(shape: &Shape<[f32; 2]>, p: [f32; 2]) -> bool {
    shape
        .regions
        .iter()
        .any(|r| in_ring(&r.outer, p) && !r.holes.iter().any(|h| in_ring(h, p)))
}
fn cross(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f64 {
    (b[0] as f64 - a[0] as f64) * (c[1] as f64 - a[1] as f64)
        - (b[1] as f64 - a[1] as f64) * (c[0] as f64 - a[0] as f64)
}
fn rings_touch(a: &[[f32; 2]], b: &[[f32; 2]]) -> bool {
    a.iter()
        .zip(a.iter().cycle().skip(1))
        .take(a.len())
        .any(|(&a, &b0)| {
            b.iter()
                .zip(b.iter().cycle().skip(1))
                .take(b.len())
                .any(|(&c, &d)| {
                    if (0..2).any(|k| {
                        a[k].max(b0[k]) + 1e-7 < c[k].min(d[k])
                            || c[k].max(d[k]) + 1e-7 < a[k].min(b0[k])
                    }) {
                        return false;
                    }
                    let x = [
                        cross(a, b0, c),
                        cross(a, b0, d),
                        cross(c, d, a),
                        cross(c, d, b0),
                    ];
                    x[0] * x[1] <= 1e-14 && x[2] * x[3] <= 1e-14
                })
        })
}
pub fn validate(shape: &Shape<[f32; 2]>) -> Result<()> {
    shape.limits()?;
    for r in &shape.regions {
        validate_contour(&r.outer)?;
        for (i, h) in r.holes.iter().enumerate() {
            validate_contour(h)?;
            if !in_ring(&r.outer, h[0]) || rings_touch(&r.outer, h) {
                return Err(fail(
                    "A footprint hole must lie strictly inside its outer contour without touching it",
                ));
            }
            for other in &r.holes[..i] {
                if rings_touch(h, other) || in_ring(h, other[0]) || in_ring(other, h[0]) {
                    return Err(fail(
                        "Footprint holes may not overlap, touch or contain each other; use another region for an island",
                    ));
                }
            }
        }
    }
    Ok(())
}
pub fn merge(target: &mut Option<Shape<[f32; 3]>>, other: Option<Shape<[f32; 3]>>) -> Result<()> {
    if let Some(mut other) = other {
        if let Some(target) = target {
            target.regions.append(&mut other.regions);
            target.limits()?;
        } else {
            *target = Some(other);
        }
    }
    Ok(())
}
