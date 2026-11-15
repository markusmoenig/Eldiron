//! Conservative dispatch bounds for the floor shader's analytic cell clip.
use super::{SurfaceKind, SurfaceTriangle};
use rusterix::isometric::Projection;

pub(super) fn clip_floor(
    region: [i32; 4],
    triangle: &SurfaceTriangle<'_>,
    projection: Projection,
) -> [i32; 4] {
    let Some(cell) = triangle
        .clip_cell
        .filter(|_| triangle.kind == SurfaceKind::Floor)
    else {
        return region;
    };
    // Invert the floor shader's world calculation, including its minimum unit
    // and first-vertex height. Terrain uses barycentric heights and stays on its
    // original bounds. The margin covers pixel centers, snapped source boundaries
    // and conservative floating-point roundoff at large world/origin magnitudes.
    let scale = projection.unit.max(0.001);
    let height = triangle.vertices[0][1];
    let x = cell[0] as f32;
    let z = cell[1] as f32;
    let magnitude = x.abs().max(z.abs()) + 1.;
    let margin = 1.
        + scale * 0.000002
        + 32.
            * f32::EPSILON
            * (projection.origin.x.abs()
                + projection.origin.y.abs()
                + magnitude * scale * 4.
                + height.abs() * projection.unit.abs() * 2.);
    let corners = [[x, z], [x + 1., z], [x, z + 1.], [x + 1., z + 1.]].map(|[x, z]| {
        [
            projection.origin.x + (x + z) * scale,
            projection.origin.y + (z - x) * scale * 0.5 - height * projection.unit * 1.2247449,
        ]
    });
    if !margin.is_finite() || !corners.iter().flatten().all(|v| v.is_finite()) {
        return region;
    }
    let min = [0, 1].map(|axis| {
        corners
            .iter()
            .map(|p| p[axis])
            .fold(f32::INFINITY, f32::min)
    });
    let max = [0, 1].map(|axis| {
        corners
            .iter()
            .map(|p| p[axis])
            .fold(f32::NEG_INFINITY, f32::max)
    });
    [
        region[0].max((min[0] - margin).floor() as i32),
        region[1].max((min[1] - margin).floor() as i32),
        region[2].min((max[0] + margin).ceil() as i32),
        region[3].min((max[1] + margin).ceil() as i32),
    ]
}
