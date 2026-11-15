//! Adjacent identical-color DDA lines may share a draw: every covered sample
//! applies the same source/alpha, so their order commutes without losing repeated
//! endpoints. Transform/clip/group boundaries and color changes end the batch.
use super::mesh::{self, Vertex};
use crate::paint::*;
use renderkit::{RenderError, Result};

pub(super) struct Batch {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub consumed: usize,
    pub lines: usize,
    pub samples: u32,
    pub endpoints: [f32; 4],
}
pub(super) fn prepare(ops: &[PaintOp], transform: Transform, density: f32) -> Result<Batch> {
    let PaintOp::SampledLine { color, .. } = &ops[0] else {
        unreachable!()
    };
    let mut batch = Batch {
        vertices: Vec::new(),
        indices: Vec::new(),
        consumed: 0,
        lines: 0,
        samples: 0,
        endpoints: [0.; 4],
    };
    let mut minimum = u32::MAX;
    let invertible = transform.inverse().is_some();
    let (quad, quad_indices) = mesh::quad(
        Rect::new(0., 0., 1., 1.),
        Rect::default(),
        transform,
        density,
    );
    for op in ops.iter().take(512) {
        let PaintOp::SampledLine {
            from,
            to,
            color: next,
        } = op
        else {
            break;
        };
        if next != color && batch.consumed != 0 {
            break;
        }
        let delta = [to[0] - from[0], to[1] - from[1]];
        let transformed = [transform.point(*from), transform.point(*to)];
        if delta
            .iter()
            .chain(transformed.iter().flatten())
            .any(|v| !v.is_finite() || !(v * density).is_finite())
        {
            return Err(RenderError("nonfinite sampled line transform".into()));
        }
        let steps = delta[0].abs().max(delta[1].abs()).ceil().min(8192.) as u32;
        if steps > 0 && invertible {
            let samples = steps + 1;
            // One instance number visits every line. Bound unused vertex work
            // when adjacent lines have very different sample counts.
            if batch.lines > 0 && (samples > minimum * 2 || batch.samples > samples * 2) {
                break;
            }
            minimum = minimum.min(samples);
            batch.samples = batch.samples.max(samples);
            if batch.lines == 0 {
                batch.endpoints = [from[0], from[1], to[0], to[1]];
            }
            let offset = batch.vertices.len() as u32;
            batch
                .vertices
                .extend(quad.iter().copied().map(|mut vertex| {
                    vertex.color = [from[0], from[1], to[0], to[1]];
                    vertex
                }));
            batch
                .indices
                .extend(quad_indices.iter().map(|index| index + offset));
            batch.lines += 1;
        }
        batch.consumed += 1;
    }
    Ok(batch)
}
