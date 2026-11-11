//! Transfer prepared 2D geometry as textured triangles, retaining source pixels only.
use super::{ScreenPainter, artwork::segments};
use rusterix::{Assets, Batch2D, PixelSource, PrimitiveMode, RepeatMode, Scene, Tile};
use theframework::paint::{ImageVertex, PaintOp, Sampling};

fn clip(vertices: &[ImageVertex], axis: usize, edge: f32, greater: bool) -> Vec<ImageVertex> {
    let inside = |v: &ImageVertex| {
        if greater {
            v.uv[axis] >= edge
        } else {
            v.uv[axis] <= edge
        }
    };
    let mut output = Vec::new();
    if let Some(mut last) = vertices.last() {
        for next in vertices {
            if inside(last) != inside(next) {
                let t = (edge - last.uv[axis]) / (next.uv[axis] - last.uv[axis]);
                output.push(ImageVertex {
                    position: std::array::from_fn(|i| {
                        last.position[i] + (next.position[i] - last.position[i]) * t
                    }),
                    uv: std::array::from_fn(|i| last.uv[i] + (next.uv[i] - last.uv[i]) * t),
                });
            }
            if inside(next) {
                output.push(*next);
            }
            last = next;
        }
    }
    output
}
impl ScreenPainter<'_> {
    /// CPU builders may prepare geometry; all coverage and source sampling happen
    /// on the GPU. Lit/3D scenes need their scene renderer and are rejected here.
    pub fn unlit_scene(&mut self, scene: &Scene, assets: &Assets) -> Result<(), String> {
        if !scene.lights.is_empty()
            || !scene.dynamic_lights.is_empty()
            || !scene.d3_static.is_empty()
            || !scene.d3_dynamic.is_empty()
            || !scene.d3_overlay.is_empty()
            || !scene.chunks.is_empty()
        {
            return Err("lit or 3D screen geometry requires its scene presenter".into());
        }
        for batch in scene.d2_static.iter().chain(&scene.d2_dynamic) {
            let tile = match batch.source {
                PixelSource::StaticTileIndex(i) => assets.tile_list.get(i as usize),
                PixelSource::DynamicTileIndex(i) => scene.dynamic_textures.get(i as usize),
                _ => batch.source.tile_ref_from_tile_list(assets),
            };
            if let Some(tile) = tile {
                self.screen_batch(batch, tile, scene.animation_frame)?;
            }
        }
        Ok(())
    }
    fn screen_batch(&mut self, batch: &Batch2D, tile: &Tile, frame: usize) -> Result<(), String> {
        if batch.vertices.is_empty() || batch.indices.is_empty() {
            return Ok(());
        }
        if batch.mode != PrimitiveMode::Triangles {
            return Err("screen line primitives require path conversion".into());
        }
        if batch.vertices.len() != batch.uvs.len() {
            return Err("screen mesh position/UV count mismatch".into());
        }
        if batch
            .indices
            .iter()
            .any(|&(a, b, c)| [a, b, c].iter().any(|i| *i >= batch.vertices.len()))
        {
            return Err("screen mesh index out of range".into());
        }
        let vertices = batch
            .vertices
            .iter()
            .zip(&batch.uvs)
            .map(|(&p, &uv)| {
                let p = batch.transform * scenevm::prelude::Vec3::new(p[0], p[1], 1.);
                ImageVertex {
                    position: [p.x, p.y],
                    uv,
                }
            })
            .collect::<Vec<_>>();
        if vertices
            .iter()
            .any(|v| v.position.iter().chain(v.uv.iter()).any(|v| !v.is_finite()))
        {
            return Err("nonfinite screen geometry".into());
        }
        let indices = batch
            .indices
            .iter()
            .flat_map(|&(a, b, c)| [a, b, c])
            .map(|i| u32::try_from(i).map_err(|_| "screen mesh exceeds index limit".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let phase = tile.frame_index(frame);
        let sampling = match batch.repeat_mode {
            RepeatMode::ClampXY => Sampling::Nearest,
            RepeatMode::RepeatXY => Sampling::RepeatNearest,
            RepeatMode::RepeatX => Sampling::RepeatNearestX,
            RepeatMode::RepeatY => Sampling::RepeatNearestY,
        };
        if batch.repeat_mode == RepeatMode::RepeatXY {
            if let Some(pages) = &tile.uv_pages {
                if pages.repeat_period.iter().all(|n| n.is_finite() && *n > 0.) {
                    let vertices = vertices
                        .into_iter()
                        .map(|mut v| {
                            for i in 0..2 {
                                v.uv[i] /= pages.repeat_period[i];
                            }
                            v
                        })
                        .collect();
                    return self.textured_triangles(tile, phase, vertices, indices, sampling);
                }
                if pages.size.iter().any(|n| *n == 0) {
                    return Err("empty screen spatial-page grid".into());
                }
                let mut min = [f32::INFINITY; 2];
                let mut max = [f32::NEG_INFINITY; 2];
                for v in &vertices {
                    for i in 0..2 {
                        min[i] = min[i].min(v.uv[i]);
                        max[i] = max[i].max(v.uv[i]);
                    }
                }
                let units = pages.units_per_page.max(1) as f32;
                let xs = segments(
                    min[0],
                    max[0] - min[0],
                    pages.origin[0],
                    pages.size[0],
                    units,
                );
                let ys = segments(
                    min[1],
                    max[1] - min[1],
                    pages.origin[1],
                    pages.size[1],
                    units,
                );
                for &(y0, y1, j) in &ys {
                    for &(x0, x1, i) in &xs {
                        let mut page_vertices = Vec::new();
                        let mut page_indices = Vec::new();
                        for triangle in indices.chunks_exact(3) {
                            let mut polygon = triangle
                                .iter()
                                .map(|i| vertices[*i as usize])
                                .collect::<Vec<_>>();
                            for (axis, edge, greater) in
                                [(0, x0, true), (0, x1, false), (1, y0, true), (1, y1, false)]
                            {
                                polygon = clip(&polygon, axis, edge, greater);
                            }
                            if polygon.len() < 3 {
                                continue;
                            }
                            let base = page_vertices.len() as u32;
                            for v in &mut polygon {
                                v.uv = v.uv.map(|v| v / units);
                            }
                            for k in 1..polygon.len() - 1 {
                                page_indices.extend([base, base + k as u32, base + k as u32 + 1]);
                            }
                            page_vertices.extend(polygon);
                        }
                        self.textured_triangles(
                            tile,
                            phase + j * pages.size[0] as usize + i,
                            page_vertices,
                            page_indices,
                            sampling,
                        )?;
                    }
                }
                return Ok(());
            }
        }
        self.textured_triangles(tile, phase, vertices, indices, sampling)
    }
    fn textured_triangles(
        &mut self,
        tile: &Tile,
        frame: usize,
        vertices: Vec<ImageVertex>,
        indices: Vec<u32>,
        sampling: Sampling,
    ) -> Result<(), String> {
        if indices.is_empty() {
            return Ok(());
        }
        if let Some(texture) = tile.textures.get(frame) {
            let image = self.images.rgba(
                &format!("tile/{}/{frame}", tile.id),
                texture.width as u32,
                texture.height as u32,
                &texture.data,
            )?;
            self.list.ops.push(PaintOp::ImageMesh {
                vertices: vertices.into(),
                indices: indices.into(),
                image,
                sampling,
                tint: [1.; 4],
            });
        }
        Ok(())
    }
}
