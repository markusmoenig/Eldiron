//! Pixel-buffer placement rounds physical endpoints before left-edge resizing.
use super::mesh::{self, Vertex};
use crate::paint::{Rect, Transform};

pub(super) fn quad(
    rect: Rect,
    transform: Transform,
    density: f32,
) -> Option<(Vec<Vertex>, Vec<u32>, [f32; 2])> {
    let (mut vertices, indices) = mesh::quad(rect, Rect::new(0., 0., 1., 1.), transform, density);
    let size = if transform.0[1] == 0.
        && transform.0[2] == 0.
        && transform.0[0] > 0.
        && transform.0[3] > 0.
    {
        // Match RasterScale::pixel_rect: round both endpoints, then subtract.
        // Keep logical/UV coordinates intact for source lookup and clipping.
        for vertex in &mut vertices {
            vertex.position = transform
                .point(vertex.local)
                .map(|v| (f64::from(v) * f64::from(density)).round() as f32);
        }
        [
            vertices[1].position[0] - vertices[0].position[0],
            vertices[3].position[1] - vertices[0].position[1],
        ]
    } else {
        // Rotated/skewed/flipped images retain their affine sampling contract.
        [
            rect.width * transform.0[0].hypot(transform.0[1]) * density,
            rect.height * transform.0[2].hypot(transform.0[3]) * density,
        ]
    };
    (size[0] > 0. && size[1] > 0.).then_some((vertices, indices, size))
}
