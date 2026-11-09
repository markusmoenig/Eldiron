//! Project the shared terrain mesh, sampling ordinary MZ material pages in world
//! X/Z. Pixel height/depth remains cached for sprite occlusion on hills.
use super::*;
use crate::chunkbuilder::terrain_surface::{TerrainSurface, barycentric};
use rayon::prelude::*;

pub(super) fn render(
    map: &Map,
    assets: &Assets,
    surface: &TerrainSurface,
    buffer: &mut TheRGBABuffer,
    p: Projection,
    seconds: f32,
) -> Vec<[f32; 5]> {
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    let mut depth = vec![[f32::NEG_INFINITY, 0., 0., 1., 0.]; width * height];
    if width == 0 || height == 0 {
        return depth;
    }
    let resolve = |source: &crate::PixelSource| {
        source
            .render_tile_id(assets)
            .and_then(|id| assets.tile_index(&id))
            .and_then(|i| assets.tile_list.get(i as usize))
    };
    let default = map
        .properties
        .get_source("default_terrain_tile")
        .and_then(resolve);
    let mut tiles = std::collections::HashMap::new();
    for s in &map.sectors {
        if s.properties.get_float_default("floor_height", 0.).abs() > 0.001 {
            continue;
        }
        if let Some(tile) = s
            .properties
            .get_default_source()
            .and_then(resolve)
            .filter(|t| t.isometric.as_ref().is_some_and(|s| s.ground))
        {
            let b = s.bounding_box(map);
            if (b.max - b.min - 1.).magnitude_squared() < 0.00001 {
                tiles.insert((b.min.x.floor() as i32, b.min.y.floor() as i32), tile);
            }
        }
    }
    if let Some(Value::TileOverrides(overrides)) = map.properties.get("tiles") {
        tiles.extend(
            overrides
                .iter()
                .filter_map(|(cell, s)| resolve(s).map(|t| (*cell, t))),
        );
    }
    let mut blends = std::collections::HashMap::new();
    if let Some(Value::BlendOverrides(overrides)) = map.properties.get("blend_tiles") {
        blends.extend(
            overrides.iter().filter_map(|(cell, (preset, s))| {
                resolve(s).map(|t| (*cell, (preset.weights(), t)))
            }),
        );
    }
    let mut clip = surface.bounds;
    let corners = [
        Vec2::zero(),
        Vec2::new(width as f32, 0.),
        Vec2::new(0., height as f32),
        Vec2::new(width as f32, height as f32),
    ]
    .into_iter()
    .flat_map(|q| {
        [
            p.ground(q),
            p.ground(q) + Vec2::new(-1.2247449, 1.2247449) * surface.max_height,
        ]
    })
    .collect::<Vec<_>>();
    clip.min = clip.min.map2(
        corners
            .iter()
            .fold(Vec2::broadcast(f32::INFINITY), |a, b| a.map2(*b, f32::min)),
        f32::max,
    );
    clip.max = clip.max.map2(
        corners
            .iter()
            .fold(Vec2::broadcast(f32::NEG_INFINITY), |a, b| {
                a.map2(*b, f32::max)
            }),
        f32::min,
    );
    let frame = (seconds.max(0.) / 0.25) as usize;
    struct Facet<'a> {
        tri: [Vec3<f32>; 3],
        screen: [Vec2<f32>; 3],
        min: [usize; 2],
        max: [usize; 2],
        normal: Vec3<f32>,
        tile: &'a Tile,
        mix: Option<([f32; 4], &'a Tile)>,
    }
    let mut facets = Vec::new();
    surface.triangles(clip, |tri| {
        let center = (tri[0] + tri[1] + tri[2]) / 3.;
        let cell = (center.x.floor() as i32, center.z.floor() as i32);
        let Some(tile) = tiles.get(&cell).copied().or(default) else {
            return;
        };
        let screen = tri.map(|v| p.screen(Vec2::new(v.x, v.z), v.y));
        let min = screen
            .iter()
            .fold(Vec2::broadcast(f32::INFINITY), |a, b| a.map2(*b, f32::min));
        let max = screen
            .iter()
            .fold(Vec2::broadcast(f32::NEG_INFINITY), |a, b| {
                a.map2(*b, f32::max)
            });
        let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalized();
        let min = [
            (min.x.floor().max(0.) as usize).min(width),
            (min.y.floor().max(0.) as usize).min(height),
        ];
        let max = [
            (max.x.ceil().max(0.) as usize).min(width),
            (max.y.ceil().max(0.) as usize).min(height),
        ];
        if min[0] < max[0] && min[1] < max[1] {
            facets.push(Facet {
                tri,
                screen,
                min,
                max,
                normal: if normal.y < 0. { -normal } else { normal },
                tile,
                mix: blends.get(&cell).copied(),
            });
        }
    });
    let paint = assets.terrain_paint.get(&map.id);
    // Rows are independent: retain facet order within each band so translucent
    // overlaps and terrain depth match the serial raster exactly.
    let rows = 32;
    buffer
        .pixels_mut()
        .par_chunks_mut(width * rows * 4)
        .zip(depth.par_chunks_mut(width * rows))
        .enumerate()
        .for_each(|(band, (pixels, depth))| {
            let top = band * rows;
            let bottom = top + depth.len() / width;
            for facet in &facets {
                let y0 = facet.min[1].max(top);
                let y1 = facet.max[1].min(bottom);
                for y in y0..y1 {
                    for x in facet.min[0]..facet.max[0] {
                        let Some(w) =
                            barycentric(facet.screen, Vec2::new(x as f32 + 0.5, y as f32 + 0.5))
                        else {
                            continue;
                        };
                        let world = facet.tri[0] * w[0] + facet.tri[1] * w[1] + facet.tri[2] * w[2];
                        let d = world.z - world.x + world.y * 0.8164966;
                        let at = (y - top) * width + x;
                        if d <= depth[at][0] {
                            continue;
                        }
                        let sample = |tile: &Tile| {
                            tile.sample_2d(
                                [world.x, world.z],
                                frame,
                                crate::SampleMode::Nearest,
                                crate::RepeatMode::RepeatXY,
                            )
                        };
                        let mut color = sample(facet.tile);
                        if let Some((weights, tile)) = facet.mix {
                            let u = world.x.rem_euclid(1.);
                            let v = world.z.rem_euclid(1.);
                            let a = weights[0] * (1. - v) + weights[3] * v;
                            let b = weights[1] * (1. - v) + weights[2] * v;
                            color = mix_floor_colors(color, sample(tile), a * (1. - u) + b * u);
                        }
                        if color[3] == 0 {
                            continue;
                        }
                        if let Some(paint) = paint {
                            color = paint.composite(color, [world.x, world.z]);
                        }
                        blend(pixels, at * 4, color);
                        depth[at] = [d, world.y, facet.normal.x, facet.normal.y, facet.normal.z];
                    }
                }
            }
        });
    depth
}

pub(super) struct Occlusion<'a> {
    pub pixels: &'a [[f32; 5]],
    pub width: usize,
    pub offset: Vec2<usize>,
    pub foot_y: f32,
    pub foot_height: f32,
    pub foot_depth: f32,
    pub unit: f32,
}
impl Occlusion<'_> {
    pub fn hides(&self, x: usize, y: usize) -> bool {
        let at = (y + self.offset.y) * self.width + x + self.offset.x;
        let Some(pixel) = self.pixels.get(at) else {
            return false;
        };
        // Supporting ground cannot erase the boot pixels below the sprite's
        // foot anchor. A genuinely higher slope can still hide the character.
        pixel[1] > self.foot_height + 0.06
            && pixel[0]
                > self.foot_depth + (self.foot_y - y as f32 - 0.5) / self.unit * 0.6666667 + 0.02
    }
}
