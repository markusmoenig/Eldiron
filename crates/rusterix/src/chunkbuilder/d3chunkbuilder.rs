use crate::chunkbuilder::action::{ControlPoint, MeshTopology, SectorMeshDescriptor};
use crate::chunkbuilder::surface_mesh_builder::{
    GeneratedMesh, SurfaceMeshBuilder, fix_winding as mesh_fix_winding,
};
use crate::chunkbuilder::terrain_generator::{TerrainConfig, TerrainGenerator};
use crate::collision_world::{BlockingVolume, DynamicOpening, OpeningType, WalkableFloor};
use crate::map::organic::OrganicGrowthShape;
use crate::{
    Assets, BBox, Batch3D, Chunk, ChunkBuilder, Item, Map, PixelSource, Value, ValueContainer,
    VertexBlendPreset,
};
use crate::{BillboardAnimation, GeometrySource, LoopOp, ProfileLoop, RepeatMode, Sector};
use buildergraph::{BuilderAssembly, BuilderAttachmentKind, BuilderDocument, BuilderPrimitive};
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::GeoId;
use std::str::FromStr;
use uuid::Uuid;
use vek::{Vec2, Vec3};

/// Default tile UUID for untextured/fallback meshes
pub const DEFAULT_TILE_ID: &str = "27826750-a9e7-4346-994b-fb318b238452";

pub struct D3ChunkBuilder {}

fn matches_preview_hide_pattern(name: &str, pattern: &str) -> bool {
    let name = name.trim();
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return false;
    }
    let name_l = name.to_lowercase();
    let pattern_l = pattern.to_lowercase();
    if let Some(prefix) = pattern_l.strip_suffix('*') {
        return name_l.starts_with(prefix);
    }
    name_l == pattern_l
}

fn profile_sector_item(map: &Map, profile_id: Uuid, sector_id: u32) -> Option<&Item> {
    let profile_map = map.profiles.get(&profile_id)?;
    profile_map
        .items
        .iter()
        .find(|item| match item.attributes.get("profile_sector_id") {
            Some(Value::UInt(id)) => *id == sector_id,
            Some(Value::Int(id)) => *id as u32 == sector_id,
            Some(Value::Int64(id)) if *id >= 0 => *id as u32 == sector_id,
            _ => false,
        })
}

fn profile_sector_item_blocking(map: &Map, profile_id: Uuid, sector_id: u32) -> Option<bool> {
    profile_sector_item(map, profile_id, sector_id)
        .map(|item| item.attributes.get_bool_default("blocking", false))
}

fn build_world_vertices(verts_uv: &[[f32; 2]], surface: &crate::Surface) -> Vec<[f32; 4]> {
    verts_uv
        .iter()
        .map(|uv| {
            let p = surface.uv_to_world(vek::Vec2::new(uv[0], uv[1]));
            [p.x, p.y, p.z, 1.0]
        })
        .collect()
}

fn surface_tile_origin_uv(surface: &crate::Surface, map: &Map) -> Vec2<f32> {
    surface.tile_local_anchor_uv(map)
}

fn uv_to_tile_local_xy(uv: Vec2<f32>, tile_origin_uv: Vec2<f32>, tile_flip_x: bool) -> Vec2<f32> {
    let x = if tile_flip_x {
        tile_origin_uv.x - uv.x
    } else {
        uv.x - tile_origin_uv.x
    };
    let y = uv.y - tile_origin_uv.y;
    Vec2::new(x, y)
}

fn tile_local_to_uv_xy(
    local: Vec2<f32>,
    tile_origin_uv: Vec2<f32>,
    tile_flip_x: bool,
) -> Vec2<f32> {
    let x = if tile_flip_x {
        tile_origin_uv.x - local.x
    } else {
        tile_origin_uv.x + local.x
    };
    let y = tile_origin_uv.y + local.y;
    Vec2::new(x, y)
}

fn build_surface_uvs(
    verts_uv: &[[f32; 2]],
    sector: &Sector,
    surface: &crate::Surface,
) -> Vec<[f32; 2]> {
    if verts_uv.is_empty() {
        return Vec::new();
    }

    let tile_mode = sector.properties.get_int_default("tile_mode", 1);
    let mut minx = f32::INFINITY;
    let mut miny = f32::INFINITY;
    let mut maxx = f32::NEG_INFINITY;
    let mut maxy = f32::NEG_INFINITY;
    for v in verts_uv {
        minx = minx.min(v[0]);
        maxx = maxx.max(v[0]);
        miny = miny.min(v[1]);
        maxy = maxy.max(v[1]);
    }
    let sx = (maxx - minx).max(1e-6);
    let sy = (maxy - miny).max(1e-6);
    let wall_like = surface.plane.normal.y.abs() < 0.25;
    let flip_v = wall_like && surface.edit_uv.up.y < 0.0;
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(verts_uv.len());
    if tile_mode == 0 {
        for v in verts_uv {
            let vv = if flip_v {
                (maxy - v[1]) / sy
            } else {
                (v[1] - miny) / sy
            };
            uvs.push([(v[0] - minx) / sx, vv]);
        }
    } else {
        let tex_scale_x = sector.properties.get_float_default("texture_scale_x", 1.0);
        let tex_scale_y = sector.properties.get_float_default("texture_scale_y", 1.0);
        for v in verts_uv {
            let vv = if flip_v {
                (maxy - v[1]) / tex_scale_y
            } else {
                (v[1] - miny) / tex_scale_y
            };
            uvs.push([(v[0] - minx) / tex_scale_x, vv]);
        }
    }

    uvs
}

fn shrink_polygon(points: &mut [Vec2<f32>], amount: f32) {
    if points.is_empty() || amount <= 0.0 {
        return;
    }
    let centroid =
        points.iter().copied().fold(Vec2::zero(), |acc, p| acc + p) / (points.len() as f32);
    for p in points.iter_mut() {
        let dir = *p - centroid;
        let len = dir.magnitude();
        if len > f32::EPSILON {
            let new_len = (len - amount).max(0.0);
            *p = centroid + dir * (new_len / len);
        }
    }
}

fn distance_point_to_segment_2d(point: Vec2<f32>, seg_start: Vec2<f32>, seg_end: Vec2<f32>) -> f32 {
    let seg = seg_end - seg_start;
    let len_sq = seg.magnitude_squared();

    if len_sq < 1e-8 {
        return (point - seg_start).magnitude();
    }

    let t = ((point - seg_start).dot(seg) / len_sq).clamp(0.0, 1.0);
    let projection = seg_start + seg * t;
    (point - projection).magnitude()
}

fn closest_point_on_segment_2d(
    point: Vec2<f32>,
    seg_start: Vec2<f32>,
    seg_end: Vec2<f32>,
) -> Vec2<f32> {
    let seg = seg_end - seg_start;
    let len_sq = seg.magnitude_squared();
    if len_sq < 1e-8 {
        return seg_start;
    }
    let t = ((point - seg_start).dot(seg) / len_sq).clamp(0.0, 1.0);
    seg_start + seg * t
}

fn closest_point_on_polygon_edges_2d(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> Vec2<f32> {
    if polygon.is_empty() {
        return point;
    }
    let mut best = polygon[0];
    let mut best_d = (point - best).magnitude_squared();
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let c = closest_point_on_segment_2d(point, a, b);
        let d = (point - c).magnitude_squared();
        if d < best_d {
            best_d = d;
            best = c;
        }
    }
    best
}

fn distance_segment_to_segment_2d(
    a0: Vec2<f32>,
    a1: Vec2<f32>,
    b0: Vec2<f32>,
    b1: Vec2<f32>,
) -> f32 {
    let d1 = distance_point_to_segment_2d(a0, b0, b1);
    let d2 = distance_point_to_segment_2d(a1, b0, b1);
    let d3 = distance_point_to_segment_2d(b0, a0, a1);
    let d4 = distance_point_to_segment_2d(b1, a0, a1);
    d1.min(d2).min(d3).min(d4)
}

fn offset_polygon_outward_2d(points: &[Vec2<f32>], amount: f32) -> Vec<Vec2<f32>> {
    if points.len() < 3 || amount <= 0.0 {
        return points.to_vec();
    }

    let n = points.len();
    let mut area = 0.0f32;
    for i in 0..n {
        let p = points[i];
        let q = points[(i + 1) % n];
        area += p.x * q.y - q.x * p.y;
    }
    let ccw = area >= 0.0;

    let outward_normal = |e: Vec2<f32>| -> Vec2<f32> {
        // For CCW polygons interior is on the left, so outward is right normal.
        if ccw {
            Vec2::new(e.y, -e.x)
        } else {
            Vec2::new(-e.y, e.x)
        }
    };

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let p0 = points[(i + n - 1) % n];
        let p1 = points[i];
        let p2 = points[(i + 1) % n];

        let e0 = (p1 - p0).try_normalized().unwrap_or(Vec2::new(1.0, 0.0));
        let e1 = (p2 - p1).try_normalized().unwrap_or(Vec2::new(1.0, 0.0));
        let n0 = outward_normal(e0);
        let n1 = outward_normal(e1);

        // Stable averaged-normal offset (robust on concave corners, avoids giant miters).
        let mut dir = n0 + n1;
        if dir.magnitude_squared() <= 1e-8 {
            dir = n1;
        }
        let dir = dir.try_normalized().unwrap_or(n1);
        out.push(p1 + dir * amount);
    }
    out
}

fn distance_to_sector_edge_2d(point: Vec2<f32>, sector: &Sector, map: &Map) -> f32 {
    let mut min_dist = f32::INFINITY;

    for &linedef_id in &sector.linedefs {
        let Some(linedef) = map.find_linedef(linedef_id) else {
            continue;
        };
        let Some(v0) = map.get_vertex(linedef.start_vertex) else {
            continue;
        };
        let Some(v1) = map.get_vertex(linedef.end_vertex) else {
            continue;
        };

        let a = Vec2::new(v0.x, v0.y);
        let b = Vec2::new(v1.x, v1.y);
        let dist = distance_point_to_segment_2d(point, a, b);
        min_dist = min_dist.min(dist);
    }

    min_dist
}

fn extract_thin_wall_edge_volumes(
    geo_id: GeoId,
    path: &[Vec2<f32>],
    min_y: f32,
    max_y: f32,
) -> Vec<BlockingVolume> {
    let mut points: Vec<Vec2<f32>> = Vec::new();
    for &p in path {
        if points
            .last()
            .is_none_or(|last| (*last - p).magnitude() > 0.01)
        {
            points.push(p);
        }
    }

    if points.len() >= 2 && (points[0] - *points.last().unwrap()).magnitude() <= 0.01 {
        points.pop();
    }

    if points.len() < 2 {
        return Vec::new();
    }

    const HALF_THICKNESS: f32 = 0.08;
    const MAX_SEGMENT_LEN: f32 = 0.35;

    let mut volumes = Vec::new();
    let mut seen: Vec<(Vec2<f32>, Vec2<f32>)> = Vec::new();

    for i in 0..points.len() {
        let start = points[i];
        let end = points[(i + 1) % points.len()];
        if (end - start).magnitude_squared() < 0.01 {
            continue;
        }

        let duplicate = seen.iter().any(|(a, b)| {
            ((start - *a).magnitude() <= 0.01 && (end - *b).magnitude() <= 0.01)
                || ((start - *b).magnitude() <= 0.01 && (end - *a).magnitude() <= 0.01)
        });
        if duplicate {
            continue;
        }

        seen.push((start, end));

        let edge = end - start;
        let edge_len = edge.magnitude();
        let steps = (edge_len / MAX_SEGMENT_LEN).ceil().max(1.0) as usize;

        for step in 0..steps {
            let t0 = step as f32 / steps as f32;
            let t1 = (step + 1) as f32 / steps as f32;
            let a = start + edge * t0;
            let b = start + edge * t1;

            let min_x = a.x.min(b.x) - HALF_THICKNESS;
            let max_x = a.x.max(b.x) + HALF_THICKNESS;
            let min_z = a.y.min(b.y) - HALF_THICKNESS;
            let max_z = a.y.max(b.y) + HALF_THICKNESS;

            volumes.push(BlockingVolume {
                geo_id,
                min: Vec3::new(min_x, min_y, min_z),
                max: Vec3::new(max_x, max_y, max_z),
            });
        }
    }

    volumes
}

/// Split triangles into per-tile batches using 1x1 UV cells. Only routes a triangle
/// to an override if all three vertices fall into the same overridden cell.
fn partition_triangles_with_tile_and_blend_overrides(
    indices: &[(usize, usize, usize)],
    uvs: &[[f32; 2]],
    tile_overrides: Option<&FxHashMap<(i32, i32), PixelSource>>,
    blend_overrides: Option<&FxHashMap<(i32, i32), (VertexBlendPreset, PixelSource)>>,
    assets: &Assets,
    surface: &crate::Surface,
    default_tile_id: Uuid,
    tile_origin_uv: Vec2<f32>,
    tile_flip_x: bool,
) -> (
    Vec<[f32; 2]>,
    Vec<[f32; 4]>,
    Vec<(usize, usize, usize)>,
    Vec<(Uuid, Vec<(usize, usize, usize)>)>,
    Vec<(Uuid, Uuid, Vec<f32>, Vec<(usize, usize, usize)>)>,
    Vec<[f32; 2]>,
) {
    let mut defaults = Vec::new();
    let mut per_tile: FxHashMap<Uuid, Vec<(usize, usize, usize)>> = FxHashMap::default();
    let mut per_blend: FxHashMap<(Uuid, Uuid, VertexBlendPreset), Vec<(usize, usize, usize)>> =
        FxHashMap::default();

    // If no overrides at all, return early
    if tile_overrides.is_none() && blend_overrides.is_none() {
        let world_vertices = build_world_vertices(uvs, surface);
        defaults.extend_from_slice(indices);
        let uvs_local = uvs.to_vec();
        return (
            uvs.to_vec(),
            world_vertices,
            defaults,
            Vec::new(),
            Vec::new(),
            uvs_local,
        );
    };

    // Subdivide each triangle against 1x1 UV tiles - do this ONCE
    let (tiled_uvs, tiled_world, tiled_tris, vertex_cells) =
        subdivide_triangles_into_tiles(indices, uvs, surface, tile_origin_uv, tile_flip_x);

    // Build a per-vertex UV set that is local to each tile (0..1), used for overrides
    let mut uvs_local = tiled_uvs.clone();
    for (i, uv) in uvs_local.iter_mut().enumerate() {
        let (tx, ty) = vertex_cells[i];
        let local = uv_to_tile_local_xy(
            Vec2::new(tiled_uvs[i][0], tiled_uvs[i][1]),
            tile_origin_uv,
            tile_flip_x,
        );
        let eps = 1e-4_f32;
        let cell_u = (local.x - tx as f32).clamp(0.0, 1.0);
        let cell_v = (local.y - ty as f32).clamp(0.0, 1.0);
        uv[0] = cell_u.clamp(eps, 1.0 - eps);
        uv[1] = (1.0 - cell_v).clamp(eps, 1.0 - eps);
    }

    for (tile_cell, tri) in tiled_tris {
        // Check blend overrides first (higher priority)
        if let Some(blend_map) = blend_overrides {
            if let Some((preset, ps)) = blend_map.get(&tile_cell) {
                if let Some(tile2) = ps.tile_from_tile_list(assets) {
                    // Determine the base tile: use tile override if present, otherwise default
                    let base_tile_id = if let Some(tile_map) = tile_overrides {
                        if let Some(base_ps) = tile_map.get(&tile_cell) {
                            if let Some(base_tile) = base_ps.tile_from_tile_list(assets) {
                                base_tile.id
                            } else {
                                default_tile_id
                            }
                        } else {
                            default_tile_id
                        }
                    } else {
                        default_tile_id
                    };

                    // Orient preset to world space based on surface normal
                    let oriented_preset = preset.orient_to_world(surface.plane.normal);

                    per_blend
                        .entry((base_tile_id, tile2.id, oriented_preset))
                        .or_default()
                        .push(tri);
                    continue;
                }
            }
        }

        // Then check tile overrides
        if let Some(tile_map) = tile_overrides {
            if let Some(ps) = tile_map.get(&tile_cell) {
                if let Some(tile) = ps.tile_from_tile_list(assets) {
                    per_tile.entry(tile.id).or_default().push(tri);
                    continue;
                }
            }
        }

        defaults.push(tri);
    }

    // Convert blend batches to final format with calculated weights
    let blend_batches: Vec<(Uuid, Uuid, Vec<f32>, Vec<(usize, usize, usize)>)> = per_blend
        .into_iter()
        .map(|((tile_id, tile_id2, preset), tris)| {
            let weights = preset.weights();
            let mut per_vertex_weights = vec![0.0; tiled_uvs.len()];

            for &(a, b, c) in &tris {
                for &idx in &[a, b, c] {
                    let uv_local = uvs_local[idx];
                    let weight = weights[0] * (1.0 - uv_local[0]) * (1.0 - uv_local[1])
                        + weights[1] * uv_local[0] * (1.0 - uv_local[1])
                        + weights[2] * uv_local[0] * uv_local[1]
                        + weights[3] * (1.0 - uv_local[0]) * uv_local[1];
                    per_vertex_weights[idx] = weight;
                }
            }

            (tile_id, tile_id2, per_vertex_weights, tris)
        })
        .collect();

    (
        tiled_uvs,
        tiled_world,
        defaults,
        per_tile.into_iter().collect(),
        blend_batches,
        uvs_local,
    )
}

fn subdivide_triangles_into_tiles(
    indices: &[(usize, usize, usize)],
    verts_uv: &[[f32; 2]],
    surface: &crate::Surface,
    tile_origin_uv: Vec2<f32>,
    tile_flip_x: bool,
) -> (
    Vec<[f32; 2]>,
    Vec<[f32; 4]>,
    Vec<((i32, i32), (usize, usize, usize))>,
    Vec<(i32, i32)>,
) {
    // Clip a polygon against an axis-aligned plane (x or y) using Sutherland–Hodgman
    const EPS: f32 = 1e-5;
    let clip_axis = |poly: Vec<vek::Vec2<f32>>,
                     axis: usize,
                     keep_min: bool,
                     bound: f32|
     -> Vec<vek::Vec2<f32>> {
        if poly.is_empty() {
            return poly;
        }
        let mut res = Vec::new();
        let mut prev = *poly.last().unwrap();
        let mut prev_inside = if axis == 0 {
            if keep_min {
                prev.x >= bound - EPS
            } else {
                prev.x <= bound + EPS
            }
        } else if keep_min {
            prev.y >= bound - EPS
        } else {
            prev.y <= bound + EPS
        };

        for &curr in &poly {
            let curr_inside = if axis == 0 {
                if keep_min {
                    curr.x >= bound - EPS
                } else {
                    curr.x <= bound + EPS
                }
            } else if keep_min {
                curr.y >= bound - EPS
            } else {
                curr.y <= bound + EPS
            };

            let delta = curr - prev;
            let intersect = |a: vek::Vec2<f32>, d: vek::Vec2<f32>| -> vek::Vec2<f32> {
                let t = if axis == 0 {
                    if d.x.abs() < EPS {
                        0.0
                    } else {
                        (bound - a.x) / d.x
                    }
                } else if d.y.abs() < EPS {
                    0.0
                } else {
                    (bound - a.y) / d.y
                };
                a + d * t.clamp(0.0, 1.0)
            };

            if curr_inside {
                if !prev_inside {
                    res.push(intersect(prev, delta));
                }
                res.push(curr);
            } else if prev_inside {
                res.push(intersect(prev, delta));
            }

            prev = curr;
            prev_inside = curr_inside;
        }
        res
    };

    let mut new_uvs = verts_uv.to_vec();
    let mut new_world = build_world_vertices(verts_uv, surface);
    let mut vertex_cells: Vec<(i32, i32)> = verts_uv
        .iter()
        .map(|uv| {
            let local = uv_to_tile_local_xy(Vec2::new(uv[0], uv[1]), tile_origin_uv, tile_flip_x);
            (local.x.floor() as i32, local.y.floor() as i32)
        })
        .collect();
    let mut tiled_indices = Vec::new();

    for &(a, b, c) in indices {
        let pa = uv_to_tile_local_xy(
            Vec2::new(verts_uv[a][0], verts_uv[a][1]),
            tile_origin_uv,
            tile_flip_x,
        );
        let pb = uv_to_tile_local_xy(
            Vec2::new(verts_uv[b][0], verts_uv[b][1]),
            tile_origin_uv,
            tile_flip_x,
        );
        let pc = uv_to_tile_local_xy(
            Vec2::new(verts_uv[c][0], verts_uv[c][1]),
            tile_origin_uv,
            tile_flip_x,
        );
        let tri = vec![pa, pb, pc];

        let orig_sign = polygon_area(&tri).signum();
        let min_tile_x = pa.x.min(pb.x).min(pc.x).floor() as i32;
        let max_tile_x = pa.x.max(pb.x).max(pc.x).ceil() as i32;
        let min_tile_y = pa.y.min(pb.y).min(pc.y).floor() as i32;
        let max_tile_y = pa.y.max(pb.y).max(pc.y).ceil() as i32;

        for tx in min_tile_x..max_tile_x {
            for ty in min_tile_y..max_tile_y {
                let mut poly = tri.clone();
                let min = vek::Vec2::new(tx as f32, ty as f32);
                let max = vek::Vec2::new(tx as f32 + 1.0, ty as f32 + 1.0);
                poly = clip_axis(poly, 0, true, min.x);
                poly = clip_axis(poly, 0, false, max.x);
                poly = clip_axis(poly, 1, true, min.y);
                poly = clip_axis(poly, 1, false, max.y);

                if poly.len() < 3 {
                    continue;
                }

                let area = polygon_area(&poly);
                if area.abs() < 1e-6 {
                    continue;
                }

                if area.signum() * orig_sign < 0.0 {
                    poly.reverse();
                }

                let base = new_uvs.len();
                for p in &poly {
                    let uv_world = tile_local_to_uv_xy(*p, tile_origin_uv, tile_flip_x);
                    new_uvs.push([uv_world.x, uv_world.y]);
                    let w = surface.uv_to_world(uv_world);
                    new_world.push([w.x, w.y, w.z, 1.0]);
                    vertex_cells.push((tx, ty));
                }

                for i in 1..poly.len() - 1 {
                    tiled_indices.push(((tx, ty), (base, base + i, base + i + 1)));
                }
            }
        }
    }

    (new_uvs, new_world, tiled_indices, vertex_cells)
}

impl Clone for D3ChunkBuilder {
    fn clone(&self) -> Self {
        D3ChunkBuilder {}
    }
}

impl ChunkBuilder for D3ChunkBuilder {
    fn new() -> Self {
        Self {}
    }

    fn boxed_clone(&self) -> Box<dyn ChunkBuilder> {
        Box::new(self.clone())
    }

    fn build(
        &mut self,
        map: &Map,
        assets: &Assets,
        chunk: &mut Chunk,
        vmchunk: &mut scenevm::Chunk,
    ) {
        let mut hidden: FxHashSet<GeoId> = FxHashSet::default();
        let preview_hide_patterns: Vec<String> = match map.properties.get("preview_hide") {
            Some(Value::StrArray(values)) => values.clone(),
            _ => Vec::new(),
        };

        // For each surface in the map
        for surface in map.surfaces.values() {
            let Some(sector) = map.find_sector(surface.sector_id) else {
                continue;
            };
            // Check for invalid surface - this shouldn't happen after sanitization,
            // but acts as a safety net. We can't rebuild here since we only have a reference to map.
            if !surface.is_valid() {
                println!(
                    "[SURFACE SKIP] Sector {} surface has invalid transform (NaN/Inf) - this should have been caught by sanitize()",
                    surface.sector_id
                );
                continue;
            }

            // Keep track of hidden sectors so that we can set them as not visible later.
            // Must happen before early-continue paths below.
            let visible = sector.properties.get_bool_default("visible", true);
            let roof_name = sector
                .properties
                .get_str_default("roof_name", String::new());
            let builder_hide_host = sector
                .properties
                .get_bool_default("builder_hide_host", false)
                && !sector
                    .properties
                    .get_str_default("builder_graph_data", String::new())
                    .trim()
                    .is_empty()
                && sector
                    .properties
                    .get_str_default("builder_graph_target", "sector".to_string())
                    == "sector";
            let hidden_by_preview = preview_hide_patterns.iter().any(|pattern| {
                matches_preview_hide_pattern(&sector.name, pattern)
                    || (!roof_name.is_empty() && matches_preview_hide_pattern(&roof_name, pattern))
            });
            if !visible || hidden_by_preview || builder_hide_host {
                hidden.insert(GeoId::Sector(sector.id));
            }

            // Skip sectors in ridge mode - they only contribute height to terrain, not surfaces
            let terrain_mode = sector.properties.get_int_default("terrain_mode", 0);
            if terrain_mode == 2 {
                continue;
            }
            let sector_feature = sector
                .properties
                .get_str_default("sector_feature", "None".to_string());
            let builder_replace_surface = sector
                .properties
                .get_str_default("builder_surface_mode", String::new())
                == "replace"
                && !sector
                    .properties
                    .get_str_default("builder_graph_data", String::new())
                    .trim()
                    .is_empty()
                && sector
                    .properties
                    .get_str_default("builder_graph_target", "sector".to_string())
                    == "sector";
            // Roof sectors generate their own roof geometry path. Skip the base upward cap
            // so we don't render an extra flat roof layer below gables.
            if sector_feature == "Roof" && surface.plane.normal.y > 0.7 {
                continue;
            }
            if builder_hide_host {
                continue;
            }
            if builder_replace_surface && surface.plane.normal.y > 0.7 {
                continue;
            }

            let bbox = sector.bounding_box(map);
            // Cull with the sector bbox: only use intersection
            if !bbox.intersects(&chunk.bbox) || !chunk.bbox.contains(bbox.center()) {
                continue;
            }

            // Occlusion data
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut occl_bbox = bbox.clone();
                occl_bbox.expand(Vec2::new(0.1, 0.1));
                chunk.occluded_sectors.push((occl_bbox, occlusion));
            }

            // Try to get profile loops from sector/map; if available, run base + features; else fallback.
            if let Some((outer_loop, hole_loops)) = read_profile_loops(surface, sector, map) {
                let dbg = false;
                if dbg {
                    println!(
                        "[DBG] build surface={}, sector={}",
                        surface.sector_id, sector.id
                    );
                    dump_poly("outer_loop", &outer_loop.path);
                    for (i, h) in hole_loops.iter().enumerate() {
                        println!("[DBG]  hole[{}] op={:?}", i, h.op);
                        dump_poly(&format!("hole[{}]", i), &h.path);
                    }
                }
                let extrude_abs = surface.extrusion.depth.abs();
                let (base_holes, feature_loops) =
                    split_loops_for_base(surface, map, &outer_loop, &hole_loops, extrude_abs);
                let profile_bias_vec = if sector
                    .properties
                    .get_bool_default("generated_profile", false)
                {
                    let host = sector
                        .properties
                        .get_int_default("generated_profile_host_linedef", sector.id as i32)
                        .unsigned_abs();
                    let sign = if host % 2 == 0 { 1.0 } else { -1.0 };
                    let mut n = surface.plane.normal;
                    let l = n.magnitude();
                    if l > 1e-6 {
                        n /= l;
                    }
                    let lateral = n * (0.0012 * sign);
                    let mix = host.wrapping_mul(1103515245).wrapping_add(sector.id);
                    let vertical = Vec3::new(0.0, ((mix % 17) as f32) * 0.00012, 0.0);
                    lateral + vertical
                } else {
                    Vec3::zero()
                };
                if dbg {
                    println!(
                        "[DBG] classification: base_holes={}, feature_loops={}",
                        base_holes.len(),
                        feature_loops.len()
                    );
                }

                // 1) BASE WALL from profile loops (outer with holes)
                let mut outer_path = outer_loop.path.clone();

                // Helper: read profile_target for a loop (profile sector → host fallback)
                let loop_profile_target = |pl: &ProfileLoop| -> i32 {
                    if let Some(origin) = pl.origin_profile_sector {
                        if let Some(profile_id) = surface.profile {
                            if let Some(profile_map) = map.profiles.get(&profile_id) {
                                if let Some(ps) = profile_map.find_sector(origin) {
                                    return ps.properties.get_int_default("profile_target", 0);
                                }
                            }
                        }
                    }
                    sector.properties.get_int_default("profile_target", 0)
                };

                // Start with true base holes (cutouts + through recesses)
                let mut holes_paths: Vec<Vec<vek::Vec2<f32>>> =
                    base_holes.iter().map(|h| h.path.clone()).collect();

                // Cut holes in the FRONT cap for recesses/reliefs that extend beyond the front
                // Note: We check depth, not enabled flag, because depth=0 still needs hole logic
                if extrude_abs > 1e-6 {
                    for h in &hole_loops {
                        let target = loop_profile_target(h);
                        match h.op {
                            LoopOp::Recess { depth: d } => {
                                // Cut hole if recess targets front, OR if it's deep enough to extend beyond front cap
                                if target == 0 || d > extrude_abs {
                                    holes_paths.push(h.path.clone());
                                }
                            }
                            LoopOp::Relief { .. } => {
                                // Cut hole if relief targets front
                                if target == 0 {
                                    holes_paths.push(h.path.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if dbg {
                    let total_pts: usize =
                        outer_path.len() + holes_paths.iter().map(|h| h.len()).sum::<usize>();
                    println!(
                        "[DBG] earcut_with_holes: outer_pts={}, holes={}, total_pts={}",
                        outer_path.len(),
                        holes_paths.len(),
                        total_pts
                    );
                }
                // Always use earcut for triangulation
                let triangulation_result = earcut_with_holes(&mut outer_path, &mut holes_paths);

                if let Some((verts_uv, indices)) = triangulation_result {
                    let mut world_vertices_for_fix = build_world_vertices(&verts_uv, surface);
                    if profile_bias_vec != Vec3::zero() {
                        for v in world_vertices_for_fix.iter_mut() {
                            v[0] += profile_bias_vec.x;
                            v[1] += profile_bias_vec.y;
                            v[2] += profile_bias_vec.z;
                        }
                    }

                    if dbg {
                        println!(
                            "[DBG] earcut ok: verts_uv={}, tris={}",
                            verts_uv.len(),
                            indices.len()
                        );
                    }
                    let mut indices = indices; // make mutable copy from earcut
                    let desired_n = surface.plane.normal;
                    fix_winding(&world_vertices_for_fix, &mut indices, desired_n);

                    let tile_overrides = sector.properties.get("tiles").and_then(|v| {
                        if let Value::TileOverrides(map) = v {
                            Some(map)
                        } else {
                            None
                        }
                    });

                    let blend_overrides = sector.properties.get("blend_tiles").and_then(|v| {
                        if let Value::BlendOverrides(map) = v {
                            Some(map)
                        } else {
                            None
                        }
                    });

                    // Get default tile for blending
                    let default_tile_id =
                        if let Some(Value::Source(ps)) = sector.properties.get("source") {
                            if let Some(tile) = ps.tile_from_tile_list(assets) {
                                tile.id
                            } else {
                                Uuid::from_str(DEFAULT_TILE_ID).unwrap()
                            }
                        } else {
                            Uuid::from_str(DEFAULT_TILE_ID).unwrap()
                        };

                    // Apply BOTH tile overrides and blend overrides in a single pass
                    let tile_origin_uv = surface_tile_origin_uv(surface, map);
                    let tile_flip_x = surface.tile_local_flip_x();
                    let (
                        verts_uv,
                        mut world_vertices,
                        default_indices,
                        override_batches,
                        blend_batches,
                        override_uvs,
                    ) = partition_triangles_with_tile_and_blend_overrides(
                        &indices,
                        &verts_uv,
                        tile_overrides,
                        blend_overrides,
                        assets,
                        surface,
                        default_tile_id,
                        tile_origin_uv,
                        tile_flip_x,
                    );
                    let (front_offset, back_offset) = surface.extrusion.offsets();
                    let extrusion_thickness = surface.extrusion.thickness();
                    if profile_bias_vec != Vec3::zero() {
                        for v in world_vertices.iter_mut() {
                            v[0] += profile_bias_vec.x;
                            v[1] += profile_bias_vec.y;
                            v[2] += profile_bias_vec.z;
                        }
                    }

                    if dbg {
                        if let Some((a, b, c)) = indices.get(0).cloned() {
                            let va = vek::Vec3::new(
                                world_vertices_for_fix[a][0],
                                world_vertices_for_fix[a][1],
                                world_vertices_for_fix[a][2],
                            );
                            let vb = vek::Vec3::new(
                                world_vertices_for_fix[b][0],
                                world_vertices_for_fix[b][1],
                                world_vertices_for_fix[b][2],
                            );
                            let vc = vek::Vec3::new(
                                world_vertices_for_fix[c][0],
                                world_vertices_for_fix[c][1],
                                world_vertices_for_fix[c][2],
                            );
                            let n = (vb - va).cross(vc - va);
                            let nn = {
                                let l = n.magnitude();
                                if l > 1e-6 { n / l } else { n }
                            };
                            let dn = {
                                let d = surface.plane.normal;
                                let l = d.magnitude();
                                if l > 1e-6 { d / l } else { d }
                            };
                            println!(
                                "[DBG] base tri[0] normal=({:.3},{:.3},{:.3}) dot surfN={:.3}",
                                nn.x,
                                nn.y,
                                nn.z,
                                nn.dot(dn)
                            );
                        }
                    }
                    if surface.extrusion.enabled && front_offset.abs() > 1e-6 {
                        let mut n = surface.plane.normal;
                        let l = n.magnitude();
                        if l > 1e-6 {
                            n /= l;
                        } else {
                            n = vek::Vec3::unit_y();
                        }
                        for v in world_vertices.iter_mut() {
                            let p = vek::Vec3::new(v[0], v[1], v[2]) + n * front_offset;
                            v[0] = p.x;
                            v[1] = p.y;
                            v[2] = p.z;
                        }
                    }

                    let uvs = build_surface_uvs(&verts_uv, sector, surface);
                    #[derive(Clone, Copy)]
                    enum MaterialKind {
                        Cap,
                        Side,
                    }

                    // Helper function (no captures): push a batch with sector material.
                    fn push_with_material_kind_local(
                        kind: MaterialKind,
                        sector: &Sector,
                        assets: &Assets,
                        vmchunk: &mut scenevm::Chunk,
                        verts: Vec<[f32; 4]>,
                        inds: Vec<(usize, usize, usize)>,
                        uvs_in: Vec<[f32; 2]>,
                    ) {
                        let source_key = match kind {
                            MaterialKind::Side => "jamb_source",
                            MaterialKind::Cap => "cap_source",
                        };
                        let fallback_key = "source";

                        let mut added = false;
                        if let Some(Value::Source(pixelsource)) = sector
                            .properties
                            .get(source_key)
                            .or_else(|| sector.properties.get(fallback_key))
                        {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    tile.id,
                                    verts.clone(),
                                    uvs_in.clone(),
                                    inds.clone(),
                                    0,
                                    true,
                                );
                                added = true;
                            }
                        }

                        if !added {
                            vmchunk.add_poly_3d(
                                GeoId::Sector(sector.id),
                                Uuid::from_str(DEFAULT_TILE_ID).unwrap(),
                                verts.clone(),
                                uvs_in.clone(),
                                inds.clone(),
                                0,
                                true,
                            );
                        }
                    }

                    // Build a side band (jamb) with UVs: U=perimeter distance normalized, V=0..1 across depth
                    let build_jamb_uv = |loop_uv: &Vec<vek::Vec2<f32>>,
                                         front_offset: f32,
                                         back_offset: f32,
                                         depth_abs: f32|
                     -> (
                        Vec<[f32; 4]>,
                        Vec<(usize, usize, usize)>,
                        Vec<[f32; 2]>,
                    ) {
                        let m = loop_uv.len();
                        if m < 2 {
                            return (vec![], vec![], vec![]);
                        }

                        let mut front_ws: Vec<vek::Vec3<f32>> = Vec::with_capacity(m);
                        for i in 0..m {
                            let p = surface.uv_to_world(loop_uv[i]) + profile_bias_vec;
                            front_ws.push(p);
                        }
                        let loop_min_y = front_ws.iter().fold(f32::INFINITY, |acc, p| acc.min(p.y));
                        let mut dists = vec![0.0f32; m + 1];
                        for i in 0..m {
                            let a = front_ws[i];
                            let b = front_ws[(i + 1) % m];
                            dists[i + 1] = dists[i] + (b - a).magnitude();
                        }
                        let perim = dists[m].max(1e-6);

                        // --- UVs: follow sector tiling rules for sides ---
                        let tile_mode_side = sector.properties.get_int_default(
                            "side_tile_mode",
                            sector.properties.get_int_default("tile_mode", 1),
                        );
                        let tex_scale_u = sector.properties.get_float_default(
                            "side_texture_scale_x",
                            sector.properties.get_float_default("texture_scale_x", 1.0),
                        );
                        let tex_scale_v = sector.properties.get_float_default(
                            "side_texture_scale_y",
                            sector.properties.get_float_default("texture_scale_y", 1.0),
                        );
                        let depth_abs = depth_abs.max(1e-6);

                        // Geometry: independent quad per edge (two triangles)
                        let mut verts: Vec<[f32; 4]> = Vec::with_capacity(m * 4);
                        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(m * 4);
                        let mut inds: Vec<(usize, usize, usize)> = Vec::with_capacity(m * 2);

                        // Use surface normal each time so this helper is independent
                        let mut n = surface.plane.normal;
                        let l = n.magnitude();
                        if l > 1e-6 {
                            n /= l;
                        } else {
                            n = vek::Vec3::unit_y();
                        }

                        for i in 0..m {
                            let ia = i;
                            let ib = (i + 1) % m;
                            let a_uv = loop_uv[ia];
                            let b_uv = loop_uv[ib];
                            let a_front =
                                surface.uv_to_world(a_uv) + n * front_offset + profile_bias_vec;
                            let b_front =
                                surface.uv_to_world(b_uv) + n * front_offset + profile_bias_vec;
                            let a_back =
                                surface.uv_to_world(a_uv) + n * back_offset + profile_bias_vec;
                            let b_back =
                                surface.uv_to_world(b_uv) + n * back_offset + profile_bias_vec;

                            // Skip bottom horizontal edges of the loop to prevent coplanar
                            // z-fighting with floor surfaces (door/cutout bottoms).
                            // Use loop-relative min-Y, not absolute world height.
                            let edge_is_horizontal = (a_front.y - b_front.y).abs() < 0.01;
                            let edge_is_bottom = a_front.y.min(b_front.y) <= loop_min_y + 0.01;
                            if edge_is_horizontal && edge_is_bottom {
                                continue;
                            }

                            let base = verts.len();
                            verts.push([a_front.x, a_front.y, a_front.z, 1.0]);
                            verts.push([b_front.x, b_front.y, b_front.z, 1.0]);
                            verts.push([b_back.x, b_back.y, b_back.z, 1.0]);
                            verts.push([a_back.x, a_back.y, a_back.z, 1.0]);

                            // U along edge length (each edge starts at 0), V across depth
                            let edge_len = if ib == 0 {
                                perim - dists[ia] // Wrap-around edge
                            } else {
                                dists[ib] - dists[ia]
                            };
                            let (ua, ub, v0, v1) = if tile_mode_side == 0 {
                                // Fit: normalize to 0..1 in both axes
                                (0.0, 1.0, 0.0, 1.0)
                            } else {
                                // Repeat: scale in world units by texture scales
                                // UVs are kept proportional to world size.
                                // Add small epsilon to avoid UV=0 exactly, which causes
                                // issues with the shader's Y-flip: fract(1.0 - 0.0) = 0,
                                // but fract(1.0 - 0.0001) = 0.9999, creating a seam.
                                let eps = 1e-4_f32;
                                let u_max = edge_len / tex_scale_u.max(1e-6);
                                let v_max = depth_abs / tex_scale_v.max(1e-6);
                                (eps, u_max + eps, eps, v_max + eps)
                            };
                            uvs.push([ua, v0]);
                            uvs.push([ub, v0]);
                            uvs.push([ub, v1]);
                            uvs.push([ua, v1]);

                            inds.push((base + 0, base + 1, base + 2));
                            inds.push((base + 0, base + 2, base + 3));
                        }

                        (verts, inds, uvs)
                    };

                    // Apply optional per-tile overrides for the main surface (jambs/caps unchanged).
                    for (tile_id, inds) in &override_batches {
                        if !inds.is_empty() {
                            vmchunk.add_poly_3d(
                                GeoId::Sector(sector.id),
                                *tile_id,
                                world_vertices.clone(),
                                override_uvs.clone(),
                                inds.clone(),
                                0,
                                true,
                            );
                        }
                    }

                    // Apply blended batches if present
                    if !blend_batches.is_empty() {
                        for (tile_id, tile_id2, blend_weights, inds) in &blend_batches {
                            if !inds.is_empty() {
                                vmchunk.add_poly_3d_blended(
                                    GeoId::Sector(sector.id),
                                    *tile_id,
                                    *tile_id2,
                                    world_vertices.clone(),
                                    override_uvs.clone(),
                                    blend_weights.clone(),
                                    inds.clone(),
                                    0,
                                    true,
                                );
                            }
                        }
                    }

                    if !default_indices.is_empty() {
                        push_with_material_kind_local(
                            MaterialKind::Cap,
                            sector,
                            assets,
                            vmchunk,
                            world_vertices.clone(),
                            default_indices.clone(),
                            uvs.clone(),
                        );
                    }

                    // --- Extrusion: thickness, back cap, side bands ---
                    if surface.extrusion.enabled && extrusion_thickness > 1e-6 {
                        let n = {
                            let nn = surface.plane.normal;
                            let l = nn.magnitude();
                            if l > 1e-6 {
                                nn / l
                            } else {
                                vek::Vec3::unit_y()
                            }
                        };

                        // 1) Back cap at z = depth (offset along normal), with its OWN holes
                        {
                            // Helper: read profile_target for a loop (profile sector → host fallback)
                            let loop_profile_target = |pl: &ProfileLoop| -> i32 {
                                if let Some(origin) = pl.origin_profile_sector {
                                    if let Some(profile_id) = surface.profile {
                                        if let Some(profile_map) = map.profiles.get(&profile_id) {
                                            if let Some(ps) = profile_map.find_sector(origin) {
                                                return ps
                                                    .properties
                                                    .get_int_default("profile_target", 0);
                                            }
                                        }
                                    }
                                }
                                sector.properties.get_int_default("profile_target", 0)
                            };

                            // Decide which holes must be subtracted from the BACK cap:
                            //  - pure cutouts (None)
                            //  - through recesses (depth >= thickness)
                            //  - shallow recesses that TARGET THE BACK CAP (profile_target==1)
                            let mut back_holes_paths: Vec<Vec<vek::Vec2<f32>>> = Vec::new();
                            for h in &hole_loops {
                                let to_back = loop_profile_target(h) == 1;
                                match h.op {
                                    LoopOp::None => {
                                        back_holes_paths.push(h.path.clone());
                                    }
                                    LoopOp::Recess { .. } => {
                                        // Always cut a hole for recesses that target the back side
                                        if to_back {
                                            back_holes_paths.push(h.path.clone());
                                        }
                                    }
                                    LoopOp::Relief { .. } => {
                                        // Cut a hole for reliefs that target the back side
                                        if to_back {
                                            back_holes_paths.push(h.path.clone());
                                        }
                                    }
                                    LoopOp::Billboard { .. } => {
                                        // Billboard is a hole in the surface
                                        back_holes_paths.push(h.path.clone());
                                    }
                                    LoopOp::Window { .. } => {
                                        // Window is also a hole in the base cap (filled by static geometry).
                                        back_holes_paths.push(h.path.clone());
                                    }
                                }
                            }

                            // Triangulate back cap with its holes
                            let mut back_outer = outer_loop.path.clone();
                            if let Some((back_verts_uv, mut back_indices)) =
                                earcut_with_holes(&mut back_outer, &mut back_holes_paths)
                            {
                                // Map UV to world on back plane
                                let back_world_vertices_for_fix: Vec<[f32; 4]> = back_verts_uv
                                    .iter()
                                    .map(|uv| {
                                        let p = surface.uv_to_world(vek::Vec2::new(uv[0], uv[1]))
                                            + n * back_offset
                                            + profile_bias_vec;
                                        [p.x, p.y, p.z, 1.0]
                                    })
                                    .collect();

                                // Faces should point opposite to front cap on the back
                                fix_winding(
                                    &back_world_vertices_for_fix,
                                    &mut back_indices,
                                    -surface.plane.normal,
                                );

                                let tile_overrides = sector.properties.get("tiles").and_then(|v| {
                                    if let Value::TileOverrides(map) = v {
                                        Some(map)
                                    } else {
                                        None
                                    }
                                });

                                // Apply both blend and tile overrides to back cap in a single pass
                                let tile_origin_uv = surface_tile_origin_uv(surface, map);
                                let tile_flip_x = surface.tile_local_flip_x();
                                let (
                                    back_verts_uv,
                                    back_world_vertices,
                                    back_default_indices,
                                    back_override_batches,
                                    back_blend_batches,
                                    back_override_uvs,
                                ) = partition_triangles_with_tile_and_blend_overrides(
                                    &back_indices,
                                    &back_verts_uv,
                                    tile_overrides,
                                    blend_overrides,
                                    assets,
                                    surface,
                                    default_tile_id,
                                    tile_origin_uv,
                                    tile_flip_x,
                                );

                                let mut back_world_vertices = back_world_vertices;
                                for v in back_world_vertices.iter_mut() {
                                    let p = vek::Vec3::new(v[0], v[1], v[2]) + n * back_offset;
                                    v[0] = p.x;
                                    v[1] = p.y;
                                    v[2] = p.z;
                                }
                                if profile_bias_vec != Vec3::zero() {
                                    for v in back_world_vertices.iter_mut() {
                                        v[0] += profile_bias_vec.x;
                                        v[1] += profile_bias_vec.y;
                                        v[2] += profile_bias_vec.z;
                                    }
                                }

                                let back_uvs = build_surface_uvs(&back_verts_uv, sector, surface);

                                for (tile_id, inds) in &back_override_batches {
                                    if !inds.is_empty() {
                                        // shift to back plane (already baked during build)
                                        vmchunk.add_poly_3d(
                                            GeoId::Sector(sector.id),
                                            *tile_id,
                                            back_world_vertices.clone(),
                                            back_override_uvs.clone(),
                                            inds.clone(),
                                            0,
                                            true,
                                        );
                                    }
                                }

                                if !back_blend_batches.is_empty() {
                                    for (tile_id, tile_id2, blend_weights, inds) in
                                        &back_blend_batches
                                    {
                                        if !inds.is_empty() {
                                            vmchunk.add_poly_3d_blended(
                                                GeoId::Sector(sector.id),
                                                *tile_id,
                                                *tile_id2,
                                                back_world_vertices.clone(),
                                                back_override_uvs.clone(),
                                                blend_weights.clone(),
                                                inds.clone(),
                                                0,
                                                true,
                                            );
                                        }
                                    }
                                }

                                if !back_default_indices.is_empty() {
                                    push_with_material_kind_local(
                                        MaterialKind::Cap,
                                        sector,
                                        assets,
                                        vmchunk,
                                        back_world_vertices,
                                        back_default_indices,
                                        back_uvs,
                                    );
                                }
                            }
                        }

                        // Helper to push a side band (outer ring or through-hole tube)
                        let mut push_side_band = |loop_uv: &Vec<vek::Vec2<f32>>| {
                            let (ring_v, mut ring_i, ring_uv) = build_jamb_uv(
                                loop_uv,
                                front_offset,
                                back_offset,
                                extrusion_thickness,
                            );
                            fix_winding(&ring_v, &mut ring_i, surface.plane.normal);
                            push_with_material_kind_local(
                                MaterialKind::Side,
                                sector,
                                assets,
                                vmchunk,
                                ring_v,
                                ring_i,
                                ring_uv,
                            );
                        };

                        // 2) Outer perimeter side band
                        push_side_band(&outer_loop.path);

                        // 3) Through-hole tubes for base holes (cutouts + through-recesses)
                        // Thin edge check automatically handles doors vs windows
                        for h in &base_holes {
                            push_side_band(&h.path);
                        }
                    }
                }

                // 2) FEATURE LOOPS: build caps + jambs using trait-based system
                for fl in &feature_loops {
                    // Use the new trait-based system for processing feature loops
                    process_feature_loop_with_action(
                        surface, map, sector, chunk, vmchunk, assets, fl,
                    );
                }
            } else {
                // Fallback: no profile info; triangulate whole surface as-is
                if let Some((_world_vertices, indices, verts_uv)) = surface.triangulate(sector, map)
                {
                    let profile_bias_vec = if sector
                        .properties
                        .get_bool_default("generated_profile", false)
                    {
                        let host = sector
                            .properties
                            .get_int_default("generated_profile_host_linedef", sector.id as i32)
                            .unsigned_abs();
                        let sign = if host % 2 == 0 { 1.0 } else { -1.0 };
                        let mut n = surface.plane.normal;
                        let l = n.magnitude();
                        if l > 1e-6 {
                            n /= l;
                        }
                        let lateral = n * (0.0012 * sign);
                        let mix = host.wrapping_mul(1103515245).wrapping_add(sector.id);
                        let vertical = Vec3::new(0.0, ((mix % 17) as f32) * 0.00012, 0.0);
                        lateral + vertical
                    } else {
                        Vec3::zero()
                    };
                    let mut world_vertices_for_fix = build_world_vertices(&verts_uv, surface);
                    if profile_bias_vec != Vec3::zero() {
                        for v in world_vertices_for_fix.iter_mut() {
                            v[0] += profile_bias_vec.x;
                            v[1] += profile_bias_vec.y;
                            v[2] += profile_bias_vec.z;
                        }
                    }
                    let mut indices = indices;
                    fix_winding(&world_vertices_for_fix, &mut indices, surface.plane.normal);

                    let tile_overrides = sector.properties.get("tiles").and_then(|v| {
                        if let Value::TileOverrides(map) = v {
                            Some(map)
                        } else {
                            None
                        }
                    });

                    let blend_overrides = sector.properties.get("blend_tiles").and_then(|v| {
                        if let Value::BlendOverrides(map) = v {
                            Some(map)
                        } else {
                            None
                        }
                    });

                    // Get default tile for blending
                    let default_tile_id =
                        if let Some(Value::Source(ps)) = sector.properties.get("source") {
                            if let Some(tile) = ps.tile_from_tile_list(assets) {
                                tile.id
                            } else {
                                Uuid::from_str(DEFAULT_TILE_ID).unwrap()
                            }
                        } else {
                            Uuid::from_str(DEFAULT_TILE_ID).unwrap()
                        };

                    // Apply both blend and tile overrides (fallback path) in a single pass
                    let tile_origin_uv = surface_tile_origin_uv(surface, map);
                    let tile_flip_x = surface.tile_local_flip_x();
                    let (
                        verts_uv,
                        mut world_vertices,
                        default_indices,
                        override_batches,
                        blend_batches,
                        override_uvs,
                    ) = partition_triangles_with_tile_and_blend_overrides(
                        &indices,
                        &verts_uv,
                        tile_overrides,
                        blend_overrides,
                        assets,
                        surface,
                        default_tile_id,
                        tile_origin_uv,
                        tile_flip_x,
                    );
                    if profile_bias_vec != Vec3::zero() {
                        for v in world_vertices.iter_mut() {
                            v[0] += profile_bias_vec.x;
                            v[1] += profile_bias_vec.y;
                            v[2] += profile_bias_vec.z;
                        }
                    }

                    let uvs = build_surface_uvs(&verts_uv, sector, surface);
                    #[allow(dead_code)]
                    #[derive(Clone, Copy)]
                    enum MaterialKind {
                        Cap,
                        Side,
                    }

                    // Helper function (no captures): push a batch with sector material.
                    fn push_with_material_kind_local(
                        kind: MaterialKind,
                        sector: &Sector,
                        assets: &Assets,
                        vmchunk: &mut scenevm::Chunk,
                        verts: Vec<[f32; 4]>,
                        inds: Vec<(usize, usize, usize)>,
                        uvs_in: Vec<[f32; 2]>,
                    ) {
                        let source_key = match kind {
                            MaterialKind::Side => "jamb_source",
                            MaterialKind::Cap => "cap_source",
                        };
                        let fallback_key = "source";

                        let mut added = false;

                        if let Some(Value::Source(pixelsource)) = sector
                            .properties
                            .get(source_key)
                            .or_else(|| sector.properties.get(fallback_key))
                        {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    tile.id,
                                    verts.clone(),
                                    uvs_in.clone(),
                                    inds.clone(),
                                    0,
                                    true,
                                );
                                added = true;
                            }
                        }

                        if !added {
                            vmchunk.add_poly_3d(
                                GeoId::Sector(sector.id),
                                Uuid::from_str(DEFAULT_TILE_ID).unwrap(),
                                verts,
                                uvs_in,
                                inds,
                                0,
                                true,
                            );
                        }
                    }

                    for (tile_id, inds) in &override_batches {
                        if !inds.is_empty() {
                            vmchunk.add_poly_3d(
                                GeoId::Sector(sector.id),
                                *tile_id,
                                world_vertices.clone(),
                                uvs.clone(),
                                inds.clone(),
                                0,
                                true,
                            );
                        }
                    }

                    if !blend_batches.is_empty() {
                        for (tile_id, tile_id2, blend_weights, inds) in &blend_batches {
                            if !inds.is_empty() {
                                vmchunk.add_poly_3d_blended(
                                    GeoId::Sector(sector.id),
                                    *tile_id,
                                    *tile_id2,
                                    world_vertices.clone(),
                                    override_uvs.clone(),
                                    blend_weights.clone(),
                                    inds.clone(),
                                    0,
                                    true,
                                );
                            }
                        }
                    }

                    for (tile_id, inds) in &override_batches {
                        if !inds.is_empty() {
                            vmchunk.add_poly_3d(
                                GeoId::Sector(sector.id),
                                *tile_id,
                                world_vertices.clone(),
                                override_uvs.clone(),
                                inds.clone(),
                                0,
                                true,
                            );
                        }
                    }

                    for (tile_id, tile_id2, blend_weights, inds) in &blend_batches {
                        if !inds.is_empty() {
                            vmchunk.add_poly_3d_blended(
                                GeoId::Sector(sector.id),
                                *tile_id,
                                *tile_id2,
                                world_vertices.clone(),
                                override_uvs.clone(),
                                blend_weights.clone(),
                                inds.clone(),
                                0,
                                true,
                            );
                        }
                    }

                    if !default_indices.is_empty() {
                        push_with_material_kind_local(
                            MaterialKind::Cap,
                            sector,
                            assets,
                            vmchunk,
                            world_vertices,
                            default_indices,
                            uvs,
                        );
                    }
                }
            }
        }

        // Build optional non-destructive linedef features (palisade, fence, ...).
        generate_sector_stairs(map, assets, chunk, vmchunk);
        generate_sector_campfires(map, assets, chunk, vmchunk);
        generate_sector_builder_features(map, assets, chunk, vmchunk);
        generate_sector_roofs(map, assets, chunk, vmchunk);
        generate_vertex_builder_features(map, assets, chunk, vmchunk);
        generate_builder_linedef_features(map, assets, chunk, vmchunk);
        generate_linedef_features(map, assets, chunk, vmchunk);

        // Generate terrain for this chunk
        let terrain_counter = chunk.bbox.min.x as u32 * 10000 + chunk.bbox.min.y as u32;
        generate_terrain(map, assets, chunk, vmchunk, terrain_counter);
        generate_organic_volumes(map, assets, chunk, vmchunk);

        // Set all hidden geometry as not visible.
        // This needs to run after all generators (roofs/features/terrain),
        // otherwise late-added polys remain visible.
        for hidden in hidden {
            if let Some(poly) = vmchunk.polys3d_map.get_mut(&hidden) {
                for p in poly {
                    p.visible = false;
                }
            }
        }
    }

    fn build_collision(
        &mut self,
        map: &Map,
        chunk_origin: Vec2<i32>,
        chunk_size: i32,
    ) -> crate::collision_world::ChunkCollision {
        use crate::BBox;

        let mut collision = crate::collision_world::ChunkCollision::new();
        let chunk_bbox = BBox::from_pos_size(
            (chunk_origin.map(|v| v as f32)) * chunk_size as f32,
            Vec2::broadcast(chunk_size as f32),
        );

        // Process each surface in the map
        for surface in map.surfaces.values() {
            let Some(sector) = map.find_sector(surface.sector_id) else {
                continue;
            };
            let sector_visible = sector.properties.get_bool_default("visible", true);
            let sector_is_cutout_handle =
                sector.properties.get_bool_default("cutout_handle", false);
            if !sector_visible || sector_is_cutout_handle {
                continue;
            }
            let sector_is_ridge = sector.properties.get_int_default("terrain_mode", 0) == 2;
            let stairs_generated = sector
                .properties
                .get_bool_default("stairs_generated", false);
            let stairs_part = sector
                .properties
                .get_str_default("stairs_part", String::new());
            let sector_feature = sector
                .properties
                .get_str_default("sector_feature", "None".to_string());
            let sector_has_stairs = sector_feature == "Stairs";

            let bbox = sector.bounding_box(map);
            // Cull with the sector bbox: only check intersection
            // Don't require center to be in chunk - surfaces can span multiple chunks!
            if !bbox.intersects(&chunk_bbox) {
                continue;
            }

            // Get profile loops (same as rendering)
            if let Some((outer_loop, hole_loops)) = read_profile_loops(surface, sector, map) {
                let extrude_abs = surface.extrusion.depth.abs();
                let walkable_polygons: Vec<Vec<Vec2<f32>>> = if hole_loops.is_empty() {
                    vec![
                        outer_loop
                            .path
                            .iter()
                            .map(|uv| {
                                let world_pos = surface.uv_to_world(*uv);
                                Vec2::new(world_pos.x, world_pos.z)
                            })
                            .collect(),
                    ]
                } else {
                    let mut outer_path = outer_loop.path.clone();
                    let mut holes_paths: Vec<Vec<Vec2<f32>>> =
                        hole_loops.iter().map(|h| h.path.clone()).collect();
                    if let Some((verts_uv, indices)) =
                        earcut_with_holes(&mut outer_path, &mut holes_paths)
                    {
                        indices
                            .into_iter()
                            .map(|(i0, i1, i2)| {
                                [i0, i1, i2]
                                    .into_iter()
                                    .map(|idx| {
                                        let uv = Vec2::new(verts_uv[idx][0], verts_uv[idx][1]);
                                        let world_pos = surface.uv_to_world(uv);
                                        Vec2::new(world_pos.x, world_pos.z)
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .collect()
                    } else {
                        vec![
                            outer_loop
                                .path
                                .iter()
                                .map(|uv| {
                                    let world_pos = surface.uv_to_world(*uv);
                                    Vec2::new(world_pos.x, world_pos.z)
                                })
                                .collect(),
                        ]
                    }
                };

                // Calculate bounds for the surface
                let mut min_x = f32::INFINITY;
                let mut min_z = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut max_z = f32::NEG_INFINITY;

                for uv in &outer_loop.path {
                    let world_pos = surface.uv_to_world(*uv);
                    min_x = min_x.min(world_pos.x);
                    max_x = max_x.max(world_pos.x);
                    min_z = min_z.min(world_pos.z);
                    max_z = max_z.max(world_pos.z);
                }

                let base_y = surface.plane.origin.y;

                // Determine if this is a vertical surface (wall) or horizontal (floor/ceiling)
                // Check the normal vector: if it's mostly horizontal (small Y), it's a wall
                // If it's mostly vertical (large Y), it's a floor/ceiling
                let normal = surface.plane.normal;
                let normal_len = normal.magnitude();
                let normalized_y = if normal_len > 1e-6 {
                    (normal.y / normal_len).abs()
                } else {
                    0.0
                };
                let is_horizontal = normalized_y > 0.7; // If Y component > 0.7, it's horizontal (floor/ceiling)
                let dungeon_part = sector
                    .properties
                    .get_str_default("dungeon_part", String::new());
                let is_dungeon_door_panel = dungeon_part == "door_panel";

                // Only add blocking volumes for VERTICAL surfaces (walls)
                // Horizontal surfaces (floors/ceilings) should not block movement
                if !is_horizontal {
                    if is_dungeon_door_panel {
                        continue;
                    }
                    if stairs_generated && (stairs_part == "riser" || stairs_part == "tread") {
                        continue;
                    }
                    let wall_path_2d: Vec<Vec2<f32>> = outer_loop
                        .path
                        .iter()
                        .map(|uv| {
                            let world_pos = surface.uv_to_world(*uv);
                            Vec2::new(world_pos.x, world_pos.z)
                        })
                        .collect();
                    // Add blocking volume for vertical surfaces (both extruded and non-extruded)
                    if surface.extrusion.enabled && extrude_abs > 1e-6 {
                        // Extruded surface - full volume
                        let (offset_min, offset_max) = surface.extrusion.span();
                        let min_y = base_y + offset_min;
                        let max_y = base_y + offset_max;

                        // Expand paper-thin dimensions (walls that are flat planes)
                        const MIN_THICKNESS: f32 = 0.1;
                        let mut wall_min = Vec3::new(min_x, min_y, min_z);
                        let mut wall_max = Vec3::new(max_x, max_y, max_z);

                        if (wall_max.x - wall_min.x).abs() < MIN_THICKNESS {
                            let mid = (wall_min.x + wall_max.x) * 0.5;
                            wall_min.x = mid - MIN_THICKNESS * 0.5;
                            wall_max.x = mid + MIN_THICKNESS * 0.5;
                        }
                        if (wall_max.z - wall_min.z).abs() < MIN_THICKNESS {
                            let mid = (wall_min.z + wall_max.z) * 0.5;
                            wall_min.z = mid - MIN_THICKNESS * 0.5;
                            wall_max.z = mid + MIN_THICKNESS * 0.5;
                        }

                        let edge_volumes = extract_thin_wall_edge_volumes(
                            GeoId::Sector(sector.id),
                            &wall_path_2d,
                            min_y,
                            max_y,
                        );
                        if edge_volumes.is_empty() {
                            collision.static_volumes.push(BlockingVolume {
                                geo_id: GeoId::Sector(sector.id),
                                min: wall_min,
                                max: wall_max,
                            });
                        } else {
                            collision.static_volumes.extend(edge_volumes);
                        }

                        // Add walkable floor at base level
                        for floor_polygon in &walkable_polygons {
                            let floor_polygon = if stairs_generated && stairs_part == "tread" {
                                inflate_walkable_polygon(floor_polygon, 0.14)
                            } else {
                                floor_polygon.clone()
                            };

                            collision.walkable_floors.push(WalkableFloor {
                                geo_id: GeoId::Sector(sector.id),
                                height: base_y,
                                polygon_2d: floor_polygon,
                            });
                        }
                    } else {
                        // Non-extruded surface - thin wall
                        // Create thin blocking volume (small height to represent wall)
                        const WALL_HEIGHT: f32 = 2.5; // Default wall height for collision
                        const MIN_THICKNESS: f32 = 0.1;

                        let mut wall_min = Vec3::new(min_x, base_y, min_z);
                        let mut wall_max = Vec3::new(max_x, base_y + WALL_HEIGHT, max_z);

                        // Expand paper-thin dimensions
                        if (wall_max.x - wall_min.x).abs() < MIN_THICKNESS {
                            let mid = (wall_min.x + wall_max.x) * 0.5;
                            wall_min.x = mid - MIN_THICKNESS * 0.5;
                            wall_max.x = mid + MIN_THICKNESS * 0.5;
                        }
                        if (wall_max.z - wall_min.z).abs() < MIN_THICKNESS {
                            let mid = (wall_min.z + wall_max.z) * 0.5;
                            wall_min.z = mid - MIN_THICKNESS * 0.5;
                            wall_max.z = mid + MIN_THICKNESS * 0.5;
                        }

                        let edge_volumes = extract_thin_wall_edge_volumes(
                            GeoId::Sector(sector.id),
                            &wall_path_2d,
                            base_y,
                            base_y + WALL_HEIGHT,
                        );
                        if edge_volumes.is_empty() {
                            collision.static_volumes.push(BlockingVolume {
                                geo_id: GeoId::Sector(sector.id),
                                min: wall_min,
                                max: wall_max,
                            });
                        } else {
                            collision.static_volumes.extend(edge_volumes);
                        }
                    }
                } else {
                    // Horizontal surface (floor/ceiling) - add as walkable floor if facing up.
                    if normalized_y > 0.7
                        && (!sector_has_stairs || (stairs_generated && stairs_part == "tread"))
                        && !sector_is_ridge
                    {
                        for floor_polygon in &walkable_polygons {
                            let floor_polygon = if stairs_generated && stairs_part == "tread" {
                                inflate_walkable_polygon(floor_polygon, 0.14)
                            } else {
                                floor_polygon.clone()
                            };

                            collision.walkable_floors.push(WalkableFloor {
                                geo_id: GeoId::Sector(sector.id),
                                height: base_y,
                                polygon_2d: floor_polygon,
                            });
                        }
                    }
                }

                // Process holes/doors/windows as dynamic openings
                for h in &hole_loops {
                    // "Create Props" uses Relief profile loops on horizontal surfaces.
                    // Add a blocking volume so runtime collision matches rendered props.
                    if is_horizontal
                        && let LoopOp::Relief { height } = h.op
                        && height > 0.0
                        && let Some(origin) = h.origin_profile_sector
                        && let Some(profile_id) = surface.profile
                        && let Some(profile_map) = map.profiles.get(&profile_id)
                        && let Some(ps) = profile_map.find_sector(origin)
                        && ps.properties.get_bool_default("profile_table", false)
                    {
                        let profile_target = ps.properties.get_int_default("profile_target", 0);
                        let base_extrusion = if profile_target == 1 {
                            surface.extrusion.depth.abs()
                        } else {
                            0.0
                        };
                        let base_y = surface.plane.origin.y + base_extrusion;
                        // Props should block above the walkable floor regardless of profile extrusion sign.
                        // Using signed direction here can place the volume below the floor plane, making
                        // barrels/chests non-blocking for actors standing on the floor.
                        let min_y = base_y - 0.02;
                        let mut max_y = base_y + height.abs();
                        // Keep tiny heights collidable.
                        if (max_y - min_y).abs() < 0.05 {
                            max_y = min_y + 0.05;
                        }

                        let mut pmin_x = f32::INFINITY;
                        let mut pmin_z = f32::INFINITY;
                        let mut pmax_x = f32::NEG_INFINITY;
                        let mut pmax_z = f32::NEG_INFINITY;
                        for uv in &h.path {
                            let wp = surface.uv_to_world(*uv);
                            pmin_x = pmin_x.min(wp.x);
                            pmin_z = pmin_z.min(wp.z);
                            pmax_x = pmax_x.max(wp.x);
                            pmax_z = pmax_z.max(wp.z);
                        }

                        if pmin_x.is_finite()
                            && pmin_z.is_finite()
                            && pmax_x.is_finite()
                            && pmax_z.is_finite()
                        {
                            collision.static_volumes.push(BlockingVolume {
                                geo_id: GeoId::Hole(sector.id, origin),
                                min: Vec3::new(pmin_x, min_y, pmin_z),
                                max: Vec3::new(pmax_x, max_y, pmax_z),
                            });
                        }
                        continue;
                    }

                    match h.op {
                        LoopOp::None => {
                            // This is a hole (door or window)
                            // The hole boundary is on the wall surface, but we need to expand it
                            // perpendicular to the wall to create a passable volume
                            let hole_points: Vec<Vec2<f32>> = h
                                .path
                                .iter()
                                .map(|uv| {
                                    let world_pos = surface.uv_to_world(*uv);
                                    Vec2::new(world_pos.x, world_pos.z)
                                })
                                .collect();

                            // Expand the hole polygon perpendicular to the wall surface
                            // to create a passable corridor through the wall
                            let normal = surface.plane.normal;
                            let normal_2d = Vec2::new(normal.x, normal.z).normalized();
                            const DOOR_DEPTH: f32 = 0.0; // No expansion; we'll shrink after

                            // Create expanded polygon by offsetting in both directions
                            let mut boundary_2d = Vec::new();
                            for point in &hole_points {
                                // Add point offset in one direction
                                boundary_2d.push(*point + normal_2d * DOOR_DEPTH);
                            }
                            // Add points offset in opposite direction (reverse order for correct winding)
                            for point in hole_points.iter().rev() {
                                boundary_2d.push(*point - normal_2d * DOOR_DEPTH);
                            }

                            // Slightly shrink the boundary to avoid overly generous collision
                            shrink_polygon(&mut boundary_2d, 0.5);

                            // For door/window openings, use a simple approach:
                            // Doors/passages should allow passage from floor level up to a reasonable ceiling height
                            // We don't have easy access to the actual room floor/ceiling from the door frame surface
                            // So use a generous range that will cover typical doorways
                            let floor_height = 0.0; // Allow from ground level
                            let ceiling_height = 10.0; // Up to high ceiling

                            // Determine opening type from properties
                            let opening_type = if let Some(origin) = h.origin_profile_sector {
                                if let Some(profile_id) = surface.profile {
                                    if let Some(profile_map) = map.profiles.get(&profile_id) {
                                        if let Some(ps) = profile_map.find_sector(origin) {
                                            // Check if it's marked as a door
                                            if ps.properties.get_bool_default("is_door", false) {
                                                OpeningType::Door
                                            } else if ps
                                                .properties
                                                .get_bool_default("is_window", false)
                                            {
                                                OpeningType::Window
                                            } else {
                                                OpeningType::Passage
                                            }
                                        } else {
                                            OpeningType::Door // Default
                                        }
                                    } else {
                                        OpeningType::Door
                                    }
                                } else {
                                    OpeningType::Door
                                }
                            } else {
                                OpeningType::Door
                            };

                            let geo_id = if let Some(origin) = h.origin_profile_sector {
                                // Hole: (parent sector id, hole sector id)
                                GeoId::Hole(sector.id, origin)
                            } else {
                                GeoId::Sector(sector.id)
                            };

                            collision.dynamic_openings.push(DynamicOpening {
                                geo_id,
                                boundary_2d,
                                floor_height,
                                ceiling_height,
                                opening_type,
                                item_blocking: surface.profile.and_then(|pid| {
                                    h.origin_profile_sector
                                        .and_then(|sid| profile_sector_item_blocking(map, pid, sid))
                                }),
                            });
                        }
                        LoopOp::Billboard { .. } => {
                            // Billboard creates a hole similar to LoopOp::None
                            // but with visual geometry (handled in rendering)
                            let hole_points: Vec<Vec2<f32>> = h
                                .path
                                .iter()
                                .map(|uv| {
                                    let world_pos = surface.uv_to_world(*uv);
                                    Vec2::new(world_pos.x, world_pos.z)
                                })
                                .collect();

                            let normal = surface.plane.normal;
                            let normal_2d = Vec2::new(normal.x, normal.z).normalized();
                            const DOOR_DEPTH: f32 = 0.0;

                            let mut boundary_2d = Vec::new();
                            for point in &hole_points {
                                boundary_2d.push(*point + normal_2d * DOOR_DEPTH);
                            }
                            for point in hole_points.iter().rev() {
                                boundary_2d.push(*point - normal_2d * DOOR_DEPTH);
                            }

                            shrink_polygon(&mut boundary_2d, 0.5);

                            let floor_height = 0.0;
                            let ceiling_height = 10.0;

                            // Billboards are typically doors/gates
                            let opening_type = OpeningType::Door;

                            let geo_id = if let Some(origin) = h.origin_profile_sector {
                                GeoId::Hole(sector.id, origin)
                            } else {
                                GeoId::Sector(sector.id)
                            };

                            collision.dynamic_openings.push(DynamicOpening {
                                geo_id,
                                boundary_2d,
                                floor_height,
                                ceiling_height,
                                opening_type,
                                item_blocking: surface.profile.and_then(|pid| {
                                    h.origin_profile_sector
                                        .and_then(|sid| profile_sector_item_blocking(map, pid, sid))
                                }),
                            });
                        }
                        LoopOp::Window { .. } => {
                            // Window openings are static blockers (frame + glass), not dynamic doors.
                            let has_glass_source = feature_has_explicit_source(
                                surface,
                                map,
                                sector,
                                h.origin_profile_sector,
                                "window_glass_source",
                            );
                            if !has_glass_source {
                                // No glass material: leave it as a passable hole.
                                continue;
                            }

                            let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
                            let mut max =
                                Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);

                            let (mut z0, mut z1) = surface.extrusion.span();
                            if !surface.extrusion.enabled || (z1 - z0).abs() < 1e-4 {
                                z0 = -0.03;
                                z1 = 0.03;
                            }

                            for uv in &h.path {
                                let p0 = surface.uvw_to_world(*uv, z0);
                                let p1 = surface.uvw_to_world(*uv, z1);
                                for p in [p0, p1] {
                                    min.x = min.x.min(p.x);
                                    min.y = min.y.min(p.y);
                                    min.z = min.z.min(p.z);
                                    max.x = max.x.max(p.x);
                                    max.y = max.y.max(p.y);
                                    max.z = max.z.max(p.z);
                                }
                            }

                            if min.x.is_finite()
                                && min.y.is_finite()
                                && min.z.is_finite()
                                && max.x.is_finite()
                                && max.y.is_finite()
                                && max.z.is_finite()
                            {
                                if (max.x - min.x).abs() < 0.02 {
                                    let c = (max.x + min.x) * 0.5;
                                    min.x = c - 0.01;
                                    max.x = c + 0.01;
                                }
                                if (max.y - min.y).abs() < 0.02 {
                                    let c = (max.y + min.y) * 0.5;
                                    min.y = c - 0.01;
                                    max.y = c + 0.01;
                                }
                                if (max.z - min.z).abs() < 0.02 {
                                    let c = (max.z + min.z) * 0.5;
                                    min.z = c - 0.01;
                                    max.z = c + 0.01;
                                }

                                let geo_id = if let Some(origin) = h.origin_profile_sector {
                                    GeoId::Hole(sector.id, origin)
                                } else {
                                    GeoId::Sector(sector.id)
                                };
                                collision
                                    .static_volumes
                                    .push(BlockingVolume { geo_id, min, max });
                            }
                        }
                        _ => {
                            // Recesses and reliefs are handled as static blocking volumes
                            // For simplicity, we can skip them or add as static volumes
                        }
                    }
                }
            } else {
                // Fallback for surfaces WITHOUT profile loops
                // Use sector boundary directly
                let sector_points: Vec<Vec2<f32>> = sector
                    .linedefs
                    .iter()
                    .filter_map(|&ld_id| {
                        let ld = map.find_linedef(ld_id)?;
                        let v = map.get_vertex_3d(ld.start_vertex)?;
                        Some(Vec2::new(v.x, v.z))
                    })
                    .collect();

                if sector_points.len() >= 3 {
                    // Calculate bounds
                    let mut min_x = f32::INFINITY;
                    let mut min_z = f32::INFINITY;
                    let mut max_x = f32::NEG_INFINITY;
                    let mut max_z = f32::NEG_INFINITY;

                    for p in &sector_points {
                        min_x = min_x.min(p.x);
                        max_x = max_x.max(p.x);
                        min_z = min_z.min(p.y);
                        max_z = max_z.max(p.y);
                    }

                    let base_y = surface.plane.origin.y;

                    // Check if vertical or horizontal
                    let normal = surface.plane.normal;
                    let normal_len = normal.magnitude();
                    let normalized_y = if normal_len > 1e-6 {
                        (normal.y / normal_len).abs()
                    } else {
                        0.0
                    };
                    let is_horizontal = normalized_y > 0.7;
                    let dungeon_part = sector
                        .properties
                        .get_str_default("dungeon_part", String::new());
                    let is_dungeon_door_panel = dungeon_part == "door_panel";

                    if !is_horizontal {
                        if is_dungeon_door_panel {
                            continue;
                        }
                        if stairs_generated && (stairs_part == "riser" || stairs_part == "tread") {
                            continue;
                        }
                        // Vertical wall - add blocking volume
                        let extrude_abs = surface.extrusion.depth.abs();
                        const MIN_THICKNESS: f32 = 0.1;

                        if surface.extrusion.enabled && extrude_abs > 1e-6 {
                            let (offset_min, offset_max) = surface.extrusion.span();
                            let min_y = base_y + offset_min;
                            let max_y = base_y + offset_max;

                            let mut wall_min = Vec3::new(min_x, min_y, min_z);
                            let mut wall_max = Vec3::new(max_x, max_y, max_z);

                            // Expand paper-thin dimensions
                            if (wall_max.x - wall_min.x).abs() < MIN_THICKNESS {
                                let mid = (wall_min.x + wall_max.x) * 0.5;
                                wall_min.x = mid - MIN_THICKNESS * 0.5;
                                wall_max.x = mid + MIN_THICKNESS * 0.5;
                            }
                            if (wall_max.z - wall_min.z).abs() < MIN_THICKNESS {
                                let mid = (wall_min.z + wall_max.z) * 0.5;
                                wall_min.z = mid - MIN_THICKNESS * 0.5;
                                wall_max.z = mid + MIN_THICKNESS * 0.5;
                            }

                            let edge_volumes = extract_thin_wall_edge_volumes(
                                GeoId::Sector(sector.id),
                                &sector_points,
                                min_y,
                                max_y,
                            );
                            if edge_volumes.is_empty() {
                                collision.static_volumes.push(BlockingVolume {
                                    geo_id: GeoId::Sector(sector.id),
                                    min: wall_min,
                                    max: wall_max,
                                });
                            } else {
                                collision.static_volumes.extend(edge_volumes);
                            }
                        } else {
                            // Non-extruded wall
                            const WALL_HEIGHT: f32 = 2.5;

                            let mut wall_min = Vec3::new(min_x, base_y, min_z);
                            let mut wall_max = Vec3::new(max_x, base_y + WALL_HEIGHT, max_z);

                            // Expand paper-thin dimensions
                            if (wall_max.x - wall_min.x).abs() < MIN_THICKNESS {
                                let mid = (wall_min.x + wall_max.x) * 0.5;
                                wall_min.x = mid - MIN_THICKNESS * 0.5;
                                wall_max.x = mid + MIN_THICKNESS * 0.5;
                            }
                            if (wall_max.z - wall_min.z).abs() < MIN_THICKNESS {
                                let mid = (wall_min.z + wall_max.z) * 0.5;
                                wall_min.z = mid - MIN_THICKNESS * 0.5;
                                wall_max.z = mid + MIN_THICKNESS * 0.5;
                            }

                            let edge_volumes = extract_thin_wall_edge_volumes(
                                GeoId::Sector(sector.id),
                                &sector_points,
                                base_y,
                                base_y + WALL_HEIGHT,
                            );
                            if edge_volumes.is_empty() {
                                collision.static_volumes.push(BlockingVolume {
                                    geo_id: GeoId::Sector(sector.id),
                                    min: wall_min,
                                    max: wall_max,
                                });
                            } else {
                                collision.static_volumes.extend(edge_volumes);
                            }
                        }
                    } else if normalized_y > 0.7
                        && (!sector_has_stairs || (stairs_generated && stairs_part == "tread"))
                        && !sector_is_ridge
                    {
                        // Horizontal floor - add as walkable
                        collision.walkable_floors.push(WalkableFloor {
                            geo_id: GeoId::Sector(sector.id),
                            height: base_y,
                            polygon_2d: sector_points,
                        });
                    }
                }
            }
        }

        // Linedef feature collisions (palisade/fence) are generated non-destructively,
        // so add blocking volumes here to match render geometry.
        add_generated_feature_collision(map, &chunk_bbox, &mut collision);
        add_linedef_feature_collision(map, &chunk_bbox, &mut collision);
        add_dungeon_door_collision(map, &chunk_bbox, &mut collision);

        collision
    }
}

fn add_dungeon_door_collision(
    map: &Map,
    chunk_bbox: &crate::BBox,
    collision: &mut crate::collision_world::ChunkCollision,
) {
    for sector in &map.sectors {
        let generated_by = sector
            .properties
            .get_str_default("generated_by", String::new());
        if generated_by != "dungeon_tool" {
            continue;
        }
        let dungeon_part = sector
            .properties
            .get_str_default("dungeon_part", String::new());
        if dungeon_part != "door_panel" {
            continue;
        }

        let bbox = sector.bounding_box(map);
        if !bbox.intersects(chunk_bbox) {
            continue;
        }

        let Some(vertices) = sector.vertices_world(map) else {
            continue;
        };
        if vertices.len() < 3 {
            continue;
        }

        let boundary_2d: Vec<Vec2<f32>> = vertices.iter().map(|v| Vec2::new(v.x, v.z)).collect();
        let floor_height = sector.properties.get_float_default(
            "floor_base",
            vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min),
        );
        let ceiling_height = floor_height
            + sector
                .properties
                .get_float_default("dungeon_door_height", 2.0)
                .max(0.1);

        collision.dynamic_openings.push(DynamicOpening {
            geo_id: GeoId::Sector(sector.id),
            item_blocking: Some(sector.properties.get_bool_default("blocking", true)),
            boundary_2d,
            floor_height,
            ceiling_height,
            opening_type: OpeningType::Door,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::surface::{Basis3, EditPlane, ExtrusionSpec, Plane};

    fn make_wall_surface(up_y: f32) -> crate::Surface {
        let right = Vec3::new(1.0, 0.0, 0.0);
        let up = Vec3::new(0.0, up_y, 0.0);
        let normal = Vec3::new(0.0, 0.0, 1.0);
        crate::Surface {
            id: Uuid::new_v4(),
            sector_id: 1,
            plane: Plane {
                origin: Vec3::zero(),
                normal,
            },
            frame: Basis3 { right, up, normal },
            edit_uv: EditPlane {
                origin: Vec3::zero(),
                right,
                up,
                scale: 1.0,
            },
            extrusion: ExtrusionSpec::default(),
            profile: None,
            organic_layers: indexmap::IndexMap::default(),
            organic_vine_strokes: Vec::new(),
            organic_bush_clusters: Vec::new(),
            world_vertices: vec![],
        }
    }

    #[test]
    fn build_surface_uvs_flips_v_for_wall_with_negative_up_y() {
        let sector = Sector::new(1, vec![]);
        let surface = make_wall_surface(-1.0);
        let verts_uv = [[0.0_f32, 0.0_f32], [0.0, 1.0]];

        let out = build_surface_uvs(&verts_uv, &sector, &surface);
        assert!((out[0][1] - 1.0).abs() < 1e-6);
        assert!((out[1][1] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn build_surface_uvs_keeps_v_for_wall_with_positive_up_y() {
        let sector = Sector::new(1, vec![]);
        let surface = make_wall_surface(1.0);
        let verts_uv = [[0.0_f32, 0.0_f32], [0.0, 1.0]];

        let out = build_surface_uvs(&verts_uv, &sector, &surface);
        assert!((out[0][1] - 0.0).abs() < 1e-6);
        assert!((out[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn tile_local_xy_roundtrip_supports_mirrored_x_mode() {
        let origin = Vec2::new(0.0, 0.0);
        let local = Vec2::new(1.25, 2.75);
        let uv = tile_local_to_uv_xy(local, origin, true);
        let back = uv_to_tile_local_xy(uv, origin, true);
        assert!((back.x - local.x).abs() < 1e-6);
        assert!((back.y - local.y).abs() < 1e-6);
    }
}

fn add_linedef_feature_collision(
    map: &Map,
    chunk_bbox: &crate::BBox,
    collision: &mut crate::collision_world::ChunkCollision,
) {
    for linedef in &map.linedefs {
        let feature = linedef
            .properties
            .get_str_default("linedef_feature", "None".to_string());
        if feature != "Palisade" && feature != "Fence" {
            continue;
        }

        let Some(v0) = map.get_vertex_3d(linedef.start_vertex) else {
            continue;
        };
        let Some(v1) = map.get_vertex_3d(linedef.end_vertex) else {
            continue;
        };

        let mut min2 = Vec2::new(v0.x.min(v1.x), v0.z.min(v1.z));
        let mut max2 = Vec2::new(v0.x.max(v1.x), v0.z.max(v1.z));

        // Keep collision defaults in sync with visual feature generation.
        // Otherwise fences/palisades may render but miss collision volumes.
        let base_height = linedef.properties.get_float_default("feature_height", 2.0);
        if base_height <= 0.0 {
            continue;
        }

        let (half_thickness, extra_height) = if feature == "Palisade" {
            let depth = linedef
                .properties
                .get_float_default("feature_depth", 0.12)
                .max(0.02);
            let segment_size = linedef
                .properties
                .get_float_default("feature_segment_size", 0.75)
                .max(0.05);
            let top_height = linedef
                .properties
                .get_float_default("feature_top_height", 0.5)
                .max(0.0);
            (depth.max(segment_size * 0.25).max(0.05), top_height)
        } else {
            let post_size = linedef
                .properties
                .get_float_default("feature_post_size", 0.18)
                .max(0.02);
            let connector_size = linedef
                .properties
                .get_float_default("feature_connector_size", 0.12)
                .max(0.01);
            ((post_size.max(connector_size) * 0.5).max(0.05), 0.0)
        };

        let lean = linedef
            .properties
            .get_float_default("feature_lean_amount", 0.0)
            .max(0.0);
        let pad = half_thickness + lean;
        min2.x -= pad;
        min2.y -= pad;
        max2.x += pad;
        max2.y += pad;

        let feature_bbox = crate::BBox::new(min2, max2);
        if !feature_bbox.intersects(chunk_bbox) {
            continue;
        }

        // Linedef features are authored as 2D blockers (same semantic as mapmini linedefs).
        // Make them full-height in collision so 3D pathing cannot bypass due to floor-height mismatch.
        let min_y = -1024.0;
        let max_y = 1024.0 + base_height + extra_height;
        collision.static_volumes.push(BlockingVolume {
            geo_id: GeoId::Linedef(linedef.id),
            min: Vec3::new(min2.x, min_y, min2.y),
            max: Vec3::new(max2.x, max_y, max2.y),
        });
    }
}

fn add_generated_feature_collision(
    map: &Map,
    chunk_bbox: &crate::BBox,
    collision: &mut crate::collision_world::ChunkCollision,
) {
    add_stairs_feature_collision(map, chunk_bbox, collision);
    add_built_stairs_collision(map, chunk_bbox, collision);
}

fn inflate_walkable_polygon(polygon: &[Vec2<f32>], padding: f32) -> Vec<Vec2<f32>> {
    if polygon.len() < 3 || padding <= 0.0 {
        return polygon.to_vec();
    }

    let mut centroid = Vec2::zero();
    for p in polygon {
        centroid += *p;
    }
    centroid /= polygon.len() as f32;

    polygon
        .iter()
        .map(|p| {
            let delta = *p - centroid;
            let len = delta.magnitude();
            if len <= 1e-5 {
                *p
            } else {
                *p + delta / len * padding
            }
        })
        .collect()
}

fn lerp3(a: Vec3<f32>, b: Vec3<f32>, t: f32) -> Vec3<f32> {
    a + (b - a) * t
}

fn align_stair_edges(
    bottom: (Vec3<f32>, Vec3<f32>),
    top: (Vec3<f32>, Vec3<f32>),
) -> ((Vec3<f32>, Vec3<f32>), (Vec3<f32>, Vec3<f32>)) {
    let direct = (bottom.0 - top.0).magnitude() + (bottom.1 - top.1).magnitude();
    let flipped = (bottom.0 - top.1).magnitude() + (bottom.1 - top.0).magnitude();
    if flipped < direct {
        (bottom, (top.1, top.0))
    } else {
        (bottom, top)
    }
}

fn add_built_stairs_collision(
    map: &Map,
    chunk_bbox: &crate::BBox,
    collision: &mut crate::collision_world::ChunkCollision,
) {
    let mut seen: FxHashSet<(u32, u32)> = FxHashSet::default();

    for sector in &map.sectors {
        if !sector
            .properties
            .get_bool_default("stairs_generated", false)
        {
            continue;
        }
        if sector
            .properties
            .get_str_default("stairs_part", String::new())
            != "tread"
        {
            continue;
        }

        let a = sector.properties.get_int_default("stairs_source_a", -1);
        let b = sector.properties.get_int_default("stairs_source_b", -1);
        if a < 0 || b < 0 {
            continue;
        }
        let key = if a <= b {
            (a as u32, b as u32)
        } else {
            (b as u32, a as u32)
        };
        if !seen.insert(key) {
            continue;
        }

        let Some(ld_a) = map.find_linedef(key.0) else {
            continue;
        };
        let Some(ld_b) = map.find_linedef(key.1) else {
            continue;
        };
        let (Some(a0), Some(a1), Some(b0), Some(b1)) = (
            map.get_vertex_3d(ld_a.start_vertex),
            map.get_vertex_3d(ld_a.end_vertex),
            map.get_vertex_3d(ld_b.start_vertex),
            map.get_vertex_3d(ld_b.end_vertex),
        ) else {
            continue;
        };

        let a_avg_y = (a0.y + a1.y) * 0.5;
        let b_avg_y = (b0.y + b1.y) * 0.5;
        let (bottom, top) = if a_avg_y <= b_avg_y {
            ((a0, a1), (b0, b1))
        } else {
            ((b0, b1), (a0, a1))
        };
        let ((b0, b1), (t0, t1)) = align_stair_edges(bottom, top);

        let dir_bottom = b1 - b0;
        let dir_top = t1 - t0;
        let len_bottom = dir_bottom.magnitude();
        let len_top = dir_top.magnitude();
        if len_bottom < 0.01 || len_top < 0.01 {
            continue;
        }
        if dir_bottom.normalized().dot(dir_top.normalized()).abs() < 0.9 {
            continue;
        }

        let group_min = Vec2::new(
            b0.x.min(b1.x).min(t0.x).min(t1.x),
            b0.z.min(b1.z).min(t0.z).min(t1.z),
        );
        let group_max = Vec2::new(
            b0.x.max(b1.x).max(t0.x).max(t1.x),
            b0.z.max(b1.z).max(t0.z).max(t1.z),
        );
        let group_bbox = crate::BBox::new(group_min, group_max);
        if !group_bbox.intersects(chunk_bbox) {
            continue;
        }

        let steps = sector.properties.get_int_default("stairs_steps", 6).max(1) as usize;
        let rise = ((t0.y + t1.y) * 0.5 - (b0.y + b1.y) * 0.5) / steps as f32;
        let bottom_y = (b0.y + b1.y) * 0.5;
        let top_y = (t0.y + t1.y) * 0.5;
        // Mark the stair-top lip as a passage so the collision solver can step
        // from the host floor into the first descending tread.
        let strip_norm = (0.9 / steps as f32).clamp(0.05, 0.22);
        let open_left_inner = lerp3(b0, t0, 1.0 - strip_norm);
        let open_right_inner = lerp3(b1, t1, 1.0 - strip_norm);
        let top_extension = 0.75;
        let top_left_dir = (t0 - b0).try_normalized().unwrap_or(Vec3::zero());
        let top_right_dir = (t1 - b1).try_normalized().unwrap_or(Vec3::zero());
        let open_left_outer = t0 + top_left_dir * top_extension;
        let open_right_outer = t1 + top_right_dir * top_extension;
        let top_opening = inflate_walkable_polygon(
            &[
                Vec2::new(open_left_inner.x, open_left_inner.z),
                Vec2::new(open_right_inner.x, open_right_inner.z),
                Vec2::new(open_right_outer.x, open_right_outer.z),
                Vec2::new(open_left_outer.x, open_left_outer.z),
            ],
            0.04,
        );
        collision.dynamic_openings.push(DynamicOpening {
            geo_id: GeoId::Sector(sector.id),
            item_blocking: Some(false),
            boundary_2d: top_opening.clone(),
            floor_height: top_y - 0.10,
            ceiling_height: top_y + 2.5,
            opening_type: OpeningType::Passage,
        });
        collision.walkable_floors.push(WalkableFloor {
            geo_id: GeoId::Sector(sector.id),
            height: top_y,
            polygon_2d: inflate_walkable_polygon(
                &[
                    Vec2::new(open_left_inner.x, open_left_inner.z),
                    Vec2::new(open_right_inner.x, open_right_inner.z),
                    Vec2::new(open_right_outer.x, open_right_outer.z),
                    Vec2::new(open_left_outer.x, open_left_outer.z),
                ],
                0.12,
            ),
        });

        // Do not create a special low-end passage strip.
        // The first tread already reaches the low edge of the stair run.
        // A dedicated passable strip here can bypass intended blockers when
        // the stairs terminate into a wall instead of open floor.

        for i in 0..steps {
            // Use asymmetric overlap:
            // - extend tread fronts a bit farther down the run so upward traversal
            //   can acquire the next step before contacting its riser
            // - keep tread backs tight so descending does not "fall" to lower
            //   treads too early and feel unnaturally fast
            let front_overlap = if i == 0 {
                1.0 / steps as f32 * 0.20
            } else {
                1.0 / steps as f32 * 0.10
            };
            let back_overlap = if i + 1 == steps {
                1.0 / steps as f32 * 0.08
            } else {
                1.0 / steps as f32 * 0.03
            };
            let t0f = ((i as f32 / steps as f32) - front_overlap).clamp(0.0, 1.0);
            let t1f = (((i + 1) as f32 / steps as f32) + back_overlap).clamp(0.0, 1.0);
            let riser_t = i as f32 / steps as f32;
            let tread_y = bottom_y + (i + 1) as f32 * rise;
            let lower_y = bottom_y + i as f32 * rise;

            let front_left = lerp3(b0, t0, t0f);
            let front_right = lerp3(b1, t1, t0f);
            let back_left = lerp3(b0, t0, t1f);
            let back_right = lerp3(b1, t1, t1f);
            let riser_left = lerp3(b0, t0, riser_t);
            let riser_right = lerp3(b1, t1, riser_t);

            collision.walkable_floors.push(WalkableFloor {
                geo_id: GeoId::Sector(sector.id),
                height: tread_y,
                polygon_2d: inflate_walkable_polygon(
                    &[
                        Vec2::new(front_left.x, front_left.z),
                        Vec2::new(front_right.x, front_right.z),
                        Vec2::new(back_right.x, back_right.z),
                        Vec2::new(back_left.x, back_left.z),
                    ],
                    0.04,
                ),
            });

            // Reintroduce physical riser blockers so actors approaching from the
            // low end cannot walk underneath the stair run. Use a single front-face
            // barrier per riser instead of an AABB volume, otherwise the side edges
            // of the box become unintended blockers while traversing the stairs.
            // Keep underside protection to the very top riser only for now.
            // This preserves traversal while still closing off the most obvious
            // "walk under the staircase" case near the landing.
            if i + 1 == steps {
                let barrier_min_y = lower_y.min(tread_y);
                let barrier_max_y = lower_y.max(tread_y);
                collision
                    .static_barriers
                    .push(crate::collision_world::StaticBarrier {
                        geo_id: GeoId::Sector(sector.id),
                        start: Vec2::new(riser_left.x, riser_left.z),
                        end: Vec2::new(riser_right.x, riser_right.z),
                        min_y: barrier_min_y,
                        max_y: barrier_max_y,
                    });
            }
        }
    }
}

fn add_stairs_feature_collision(
    map: &Map,
    chunk_bbox: &crate::BBox,
    collision: &mut crate::collision_world::ChunkCollision,
) {
    for sector in &map.sectors {
        let feature = sector
            .properties
            .get_str_default("sector_feature", "None".to_string());
        if feature != "Stairs" {
            continue;
        }

        let bbox = sector.bounding_box(map);
        if !bbox.intersects(chunk_bbox) {
            continue;
        }

        let steps = sector.properties.get_int_default("stairs_steps", 6).max(1) as usize;
        let total_height = sector
            .properties
            .get_float_default("stairs_total_height", 1.0)
            .max(0.0);
        if total_height <= 0.0 {
            continue;
        }
        let dir = sector
            .properties
            .get_int_default("stairs_direction", 0)
            .clamp(0, 3);

        for (_, surface) in &map.surfaces {
            if surface.sector_id != sector.id {
                continue;
            }
            if surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }

            let Some(loop_uv) = surface.sector_loop_uv(map) else {
                continue;
            };
            if loop_uv.len() < 3 {
                continue;
            }

            let mut min_u = f32::INFINITY;
            let mut min_v = f32::INFINITY;
            let mut max_u = f32::NEG_INFINITY;
            let mut max_v = f32::NEG_INFINITY;
            for p in &loop_uv {
                min_u = min_u.min(p.x);
                min_v = min_v.min(p.y);
                max_u = max_u.max(p.x);
                max_v = max_v.max(p.y);
            }

            let (run_min, run_max, cross_min, cross_max, along_u) = match dir {
                0 => (min_v, max_v, min_u, max_u, false), // north (+V)
                1 => (min_u, max_u, min_v, max_v, true),  // east (+U)
                2 => (min_v, max_v, min_u, max_u, false), // south (-V)
                _ => (min_u, max_u, min_v, max_v, true),  // west (-U)
            };
            let run_len = (run_max - run_min).max(1e-4);
            let step_run = run_len / steps as f32;
            let overlap = (step_run * 0.15).clamp(0.0, 0.08);
            let mut normal = surface.plane.normal;
            if normal.y < 0.0 {
                normal = -normal;
            }
            let normal = {
                let l = normal.magnitude();
                if l > 1e-6 {
                    normal / l
                } else {
                    Vec3::new(0.0, 1.0, 0.0)
                }
            };

            // Carve a passable opening only at the stair-top transition strip.
            // A full-footprint opening lets players bypass walls at side edges.
            let hi_edge = if dir == 0 || dir == 1 {
                run_max
            } else {
                run_min
            };
            let strip_depth = (step_run * 0.9).clamp(0.12, 0.35);
            let eps = 0.03_f32;
            let (open_run_min, open_run_max) = if dir == 0 || dir == 1 {
                (
                    (hi_edge - strip_depth).max(run_min),
                    (hi_edge + eps).min(run_max),
                )
            } else {
                (
                    (hi_edge - eps).max(run_min),
                    (hi_edge + strip_depth).min(run_max),
                )
            };
            // Keep the opening narrow across the stair width to prevent side-edge wall bypass.
            let cross_center = (cross_min + cross_max) * 0.5;
            let cross_width = (cross_max - cross_min).abs();
            let open_cross_half = (cross_width * 0.28).clamp(0.28, 0.62);
            let open_cross_min = (cross_center - open_cross_half).max(cross_min + 0.02);
            let open_cross_max = (cross_center + open_cross_half).min(cross_max - 0.02);

            let (open_uv_a, open_uv_b, open_uv_c, open_uv_d) = if along_u {
                (
                    Vec2::new(open_run_min, open_cross_min),
                    Vec2::new(open_run_max, open_cross_min),
                    Vec2::new(open_run_max, open_cross_max),
                    Vec2::new(open_run_min, open_cross_max),
                )
            } else {
                (
                    Vec2::new(open_cross_min, open_run_min),
                    Vec2::new(open_cross_max, open_run_min),
                    Vec2::new(open_cross_max, open_run_max),
                    Vec2::new(open_cross_min, open_run_max),
                )
            };
            let w_open_a = surface.uv_to_world(open_uv_a);
            let w_open_b = surface.uv_to_world(open_uv_b);
            let w_open_c = surface.uv_to_world(open_uv_c);
            let w_open_d = surface.uv_to_world(open_uv_d);
            let base_h = (w_open_a.y + w_open_b.y + w_open_c.y + w_open_d.y) * 0.25;
            let boundary_2d = vec![
                Vec2::new(w_open_a.x, w_open_a.z),
                Vec2::new(w_open_b.x, w_open_b.z),
                Vec2::new(w_open_c.x, w_open_c.z),
                Vec2::new(w_open_d.x, w_open_d.z),
            ];
            collision.dynamic_openings.push(DynamicOpening {
                geo_id: GeoId::Sector(sector.id),
                item_blocking: Some(false),
                boundary_2d,
                floor_height: base_h + total_height - 0.10,
                ceiling_height: base_h + total_height + 2.5,
                opening_type: OpeningType::Passage,
            });

            for i in 0..steps {
                let t0 = i as f32 / steps as f32;
                let t1 = (i + 1) as f32 / steps as f32;
                let h1 = total_height * t1;

                let (r0, r1) = match dir {
                    0 | 1 => (run_min + run_len * t0, run_min + run_len * t1),
                    2 | 3 => (run_max - run_len * t1, run_max - run_len * t0),
                    _ => (run_min + run_len * t0, run_min + run_len * t1),
                };
                let mut r0 = r0 - overlap;
                let mut r1 = r1 + overlap;
                r0 = r0.max(run_min);
                r1 = r1.min(run_max);

                let (uv_a, uv_b, uv_c, uv_d) = if along_u {
                    (
                        Vec2::new(r0, cross_min),
                        Vec2::new(r1, cross_min),
                        Vec2::new(r1, cross_max),
                        Vec2::new(r0, cross_max),
                    )
                } else {
                    (
                        Vec2::new(cross_min, r0),
                        Vec2::new(cross_max, r0),
                        Vec2::new(cross_max, r1),
                        Vec2::new(cross_min, r1),
                    )
                };

                let w0 = surface.uv_to_world(uv_a) + normal * h1;
                let w1 = surface.uv_to_world(uv_b) + normal * h1;
                let w2 = surface.uv_to_world(uv_c) + normal * h1;
                let w3 = surface.uv_to_world(uv_d) + normal * h1;

                collision.walkable_floors.push(WalkableFloor {
                    geo_id: GeoId::Sector(sector.id),
                    height: (w0.y + w1.y + w2.y + w3.y) * 0.25,
                    polygon_2d: vec![
                        Vec2::new(w0.x, w0.z),
                        Vec2::new(w1.x, w1.z),
                        Vec2::new(w2.x, w2.z),
                        Vec2::new(w3.x, w3.z),
                    ],
                });
            }
        }
    }
}

fn hash01(mut seed: u32) -> f32 {
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(0x7feb352d);
    seed ^= seed >> 15;
    seed = seed.wrapping_mul(0x846ca68b);
    seed ^= seed >> 16;
    (seed as f32) / (u32::MAX as f32)
}

fn add_vertex(
    mesh_vertices: &mut Vec<[f32; 4]>,
    mesh_uvs: &mut Vec<[f32; 2]>,
    p: Vec3<f32>,
) -> usize {
    let idx = mesh_vertices.len();
    mesh_vertices.push([p.x, p.y, p.z, 1.0]);
    mesh_uvs.push([p.x, p.z]);
    idx
}

fn add_tri(mesh_indices: &mut Vec<(usize, usize, usize)>, a: usize, b: usize, c: usize) {
    mesh_indices.push((a, b, c));
}

fn add_quad(mesh_indices: &mut Vec<(usize, usize, usize)>, a: usize, b: usize, c: usize, d: usize) {
    add_tri(mesh_indices, a, b, c);
    add_tri(mesh_indices, a, c, d);
}

fn add_quad_reversed(
    mesh_indices: &mut Vec<(usize, usize, usize)>,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
) {
    add_tri(mesh_indices, a, d, c);
    add_tri(mesh_indices, a, c, b);
}

fn rotate_vec3_y(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c - v.z * s, v.y, v.x * s + v.z * c)
}

fn rotate_vec3_x(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x, v.y * c - v.z * s, v.y * s + v.z * c)
}

fn add_builder_box_mesh(
    mesh_vertices: &mut Vec<[f32; 4]>,
    mesh_uvs: &mut Vec<[f32; 2]>,
    mesh_indices: &mut Vec<(usize, usize, usize)>,
    center: Vec3<f32>,
    size: Vec3<f32>,
    local_rotation_x: f32,
    local_rotation_y: f32,
    along: Vec3<f32>,
    up: Vec3<f32>,
    outward: Vec3<f32>,
) {
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;
    let local = [
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(hx, -hy, -hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(-hx, -hy, hz),
        Vec3::new(-hx, hy, -hz),
        Vec3::new(hx, hy, -hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(-hx, hy, hz),
    ];
    let mut ids = [0usize; 8];
    for (i, p) in local.iter().enumerate() {
        let r = rotate_vec3_y(rotate_vec3_x(*p, local_rotation_x), local_rotation_y);
        let world = center + along * r.x + up * r.y + outward * r.z;
        ids[i] = add_vertex(mesh_vertices, mesh_uvs, world);
    }

    let faces = [
        (0usize, 1usize, 5usize, 4usize),
        (1usize, 2usize, 6usize, 5usize),
        (2usize, 3usize, 7usize, 6usize),
        (3usize, 0usize, 4usize, 7usize),
        (4usize, 5usize, 6usize, 7usize),
        (0usize, 3usize, 2usize, 1usize),
    ];
    for (a, b, c, d) in faces {
        add_quad_reversed(mesh_indices, ids[a], ids[b], ids[c], ids[d]);
    }
}

fn add_builder_cylinder_mesh(
    mesh_vertices: &mut Vec<[f32; 4]>,
    mesh_uvs: &mut Vec<[f32; 2]>,
    mesh_indices: &mut Vec<(usize, usize, usize)>,
    center: Vec3<f32>,
    length: f32,
    radius: f32,
    local_rotation_x: f32,
    local_rotation_y: f32,
    along: Vec3<f32>,
    up: Vec3<f32>,
    outward: Vec3<f32>,
) {
    let segments = 16usize;
    let hy = length * 0.5;
    let mut bottom_ring = Vec::with_capacity(segments);
    let mut top_ring = Vec::with_capacity(segments);

    for i in 0..segments {
        let a = i as f32 / segments as f32 * std::f32::consts::TAU;
        let x = a.cos() * radius;
        let z = a.sin() * radius;

        let local_bottom = rotate_vec3_y(
            rotate_vec3_x(Vec3::new(x, -hy, z), local_rotation_x),
            local_rotation_y,
        );
        let local_top = rotate_vec3_y(
            rotate_vec3_x(Vec3::new(x, hy, z), local_rotation_x),
            local_rotation_y,
        );
        let world_bottom =
            center + along * local_bottom.x + up * local_bottom.y + outward * local_bottom.z;
        let world_top = center + along * local_top.x + up * local_top.y + outward * local_top.z;
        bottom_ring.push(add_vertex(mesh_vertices, mesh_uvs, world_bottom));
        top_ring.push(add_vertex(mesh_vertices, mesh_uvs, world_top));
    }

    for i in 0..segments {
        let next = (i + 1) % segments;
        add_quad_reversed(
            mesh_indices,
            bottom_ring[i],
            top_ring[i],
            top_ring[next],
            bottom_ring[next],
        );
    }

    let bottom_center = add_vertex(mesh_vertices, mesh_uvs, center - up * hy);
    let top_center = add_vertex(mesh_vertices, mesh_uvs, center + up * hy);
    for i in 0..segments {
        let next = (i + 1) % segments;
        add_tri(
            mesh_indices,
            bottom_center,
            bottom_ring[next],
            bottom_ring[i],
        );
        add_tri(mesh_indices, top_center, top_ring[i], top_ring[next]);
    }
}

fn normalize_builder_material_key(name: &str) -> String {
    let mut out = String::new();
    let mut prev_is_sep = false;
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if i > 0 && !prev_is_sep {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push(ch.to_ascii_lowercase());
            }
            prev_is_sep = false;
        } else if !prev_is_sep && !out.is_empty() {
            out.push('_');
            prev_is_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn builder_item_graph_data_key(name: &str) -> String {
    format!(
        "builder_item_{}_graph_data",
        normalize_builder_material_key(name)
    )
}

fn resolve_builder_material_tile_id(
    properties: &ValueContainer,
    fallback_source: Option<&Value>,
    material_slot: Option<&str>,
    assets: &Assets,
) -> Uuid {
    let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();
    if let Some(material_slot) = material_slot {
        let key = format!(
            "builder_material_{}",
            normalize_builder_material_key(material_slot)
        );
        if let Some(Value::Source(ps)) = properties.get(&key) {
            return ps
                .tile_from_tile_list(assets)
                .map(|tile| tile.id)
                .unwrap_or(default_tile_id);
        }
    }
    match fallback_source {
        Some(Value::Source(ps)) => ps
            .tile_from_tile_list(assets)
            .map(|tile| tile.id)
            .unwrap_or(default_tile_id),
        _ => default_tile_id,
    }
}

fn emit_builder_attached_item_graphs(
    assembly: &BuilderAssembly,
    host_properties: &ValueContainer,
    parent_host_dims: Vec2<f32>,
    origin: Vec3<f32>,
    along: Vec3<f32>,
    up: Vec3<f32>,
    outward: Vec3<f32>,
    geo_id: GeoId,
    assets: &Assets,
    vmchunk: &mut scenevm::Chunk,
) {
    for anchor in &assembly.anchors {
        if anchor.kind != BuilderAttachmentKind::Item {
            continue;
        }
        let Some(graph_text) = host_properties.get_str(&builder_item_graph_data_key(&anchor.name))
        else {
            continue;
        };
        let Ok(graph) = BuilderDocument::from_text(graph_text) else {
            continue;
        };
        let Ok(child_assembly) = graph.evaluate() else {
            continue;
        };
        for primitive in child_assembly.primitives {
            match primitive {
                BuilderPrimitive::Box {
                    size,
                    transform,
                    material_slot,
                    host_position_normalized,
                    host_position_y_normalized,
                    host_scale_y_normalized,
                    host_scale_x_normalized,
                    host_scale_z_normalized,
                } => {
                    let sx = if host_scale_x_normalized {
                        transform.scale.x
                    } else {
                        transform.scale.x
                    };
                    let sz = if host_scale_z_normalized {
                        transform.scale.z
                    } else {
                        transform.scale.z
                    };
                    let sy = if host_scale_y_normalized {
                        transform.scale.y * parent_host_dims.y
                    } else {
                        transform.scale.y
                    };
                    let scaled = Vec3::new(size.x * sx, size.y * sy, size.z * sz);
                    let anchor_tx = if anchor.host_position_normalized {
                        anchor.transform.translation.x * parent_host_dims.x
                    } else {
                        anchor.transform.translation.x
                    };
                    let anchor_tz = if anchor.host_position_normalized {
                        anchor.transform.translation.z * parent_host_dims.y
                    } else {
                        anchor.transform.translation.z
                    };
                    let child_tx = if host_position_normalized {
                        transform.translation.x
                    } else {
                        transform.translation.x
                    };
                    let child_tz = if host_position_normalized {
                        transform.translation.z
                    } else {
                        transform.translation.z
                    };
                    let local_anchor = rotate_vec3_y(
                        Vec3::new(
                            anchor_tx,
                            if anchor.host_position_y_normalized {
                                anchor.transform.translation.y * parent_host_dims.y
                            } else {
                                anchor.transform.translation.y
                            },
                            anchor_tz,
                        ),
                        0.0,
                    );
                    let local_child = rotate_vec3_y(
                        Vec3::new(
                            child_tx,
                            if host_position_y_normalized {
                                transform.translation.y * parent_host_dims.y
                            } else {
                                transform.translation.y
                            } + scaled.y * 0.5,
                            child_tz,
                        ),
                        anchor.transform.rotation_y,
                    );
                    let center = origin
                        + along * (local_anchor.x + local_child.x)
                        + up * (local_anchor.y + local_child.y)
                        + outward * (local_anchor.z + local_child.z);
                    let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
                    let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
                    let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();
                    add_builder_box_mesh(
                        &mut mesh_vertices,
                        &mut mesh_uvs,
                        &mut mesh_indices,
                        center,
                        scaled,
                        anchor.transform.rotation_x + transform.rotation_x,
                        anchor.transform.rotation_y + transform.rotation_y,
                        along,
                        up,
                        outward,
                    );
                    let tile_id = resolve_builder_material_tile_id(
                        host_properties,
                        host_properties.get("source"),
                        material_slot.as_deref(),
                        assets,
                    );
                    vmchunk.add_poly_3d(
                        geo_id,
                        tile_id,
                        mesh_vertices,
                        mesh_uvs,
                        mesh_indices,
                        0,
                        true,
                    );
                }
                BuilderPrimitive::Cylinder {
                    length,
                    radius,
                    transform,
                    material_slot,
                    host_position_normalized: _,
                    host_position_y_normalized,
                    host_scale_y_normalized,
                    host_scale_x_normalized,
                    host_scale_z_normalized: _,
                } => {
                    let scaled_length = if host_scale_x_normalized {
                        length * transform.scale.x * parent_host_dims.x
                    } else {
                        length * transform.scale.y
                    };
                    let scaled_radius = radius
                        * if host_scale_y_normalized {
                            transform.scale.z * parent_host_dims.y
                        } else {
                            transform.scale.z
                        };
                    let anchor_tx = if anchor.host_position_normalized {
                        anchor.transform.translation.x * parent_host_dims.x
                    } else {
                        anchor.transform.translation.x
                    };
                    let anchor_tz = if anchor.host_position_normalized {
                        anchor.transform.translation.z * parent_host_dims.y
                    } else {
                        anchor.transform.translation.z
                    };
                    let local_anchor = Vec3::new(
                        anchor_tx,
                        if anchor.host_position_y_normalized {
                            anchor.transform.translation.y * parent_host_dims.y
                        } else {
                            anchor.transform.translation.y
                        },
                        anchor_tz,
                    );
                    let local_child = rotate_vec3_y(
                        Vec3::new(
                            transform.translation.x,
                            if host_position_y_normalized {
                                transform.translation.y * parent_host_dims.y
                            } else {
                                transform.translation.y
                            } + scaled_length * 0.5,
                            transform.translation.z,
                        ),
                        anchor.transform.rotation_y,
                    );
                    let center = origin
                        + along * (local_anchor.x + local_child.x)
                        + up * (local_anchor.y + local_child.y)
                        + outward * (local_anchor.z + local_child.z);
                    let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
                    let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
                    let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();
                    add_builder_cylinder_mesh(
                        &mut mesh_vertices,
                        &mut mesh_uvs,
                        &mut mesh_indices,
                        center,
                        scaled_length,
                        scaled_radius,
                        anchor.transform.rotation_x + transform.rotation_x,
                        anchor.transform.rotation_y + transform.rotation_y,
                        along,
                        up,
                        outward,
                    );
                    let tile_id = resolve_builder_material_tile_id(
                        host_properties,
                        host_properties.get("source"),
                        material_slot.as_deref(),
                        assets,
                    );
                    vmchunk.add_poly_3d(
                        geo_id,
                        tile_id,
                        mesh_vertices,
                        mesh_uvs,
                        mesh_indices,
                        0,
                        true,
                    );
                }
            }
        }
    }
}

fn emit_builder_linedef_group_meshes(
    graph: &BuilderDocument,
    host_id: u32,
    host_properties: &ValueContainer,
    hosts: &[(Vec3<f32>, Vec3<f32>)],
    assets: &Assets,
    vmchunk: &mut scenevm::Chunk,
) -> bool {
    if hosts.is_empty() {
        return false;
    }
    let spec = graph.output_spec();
    if spec.target != buildergraph::BuilderOutputTarget::Linedef {
        return false;
    }

    let midpoint = |a: Vec3<f32>, b: Vec3<f32>| (a + b) * 0.5;
    let first_mid = midpoint(hosts[0].0, hosts[0].1);
    let last_mid = midpoint(hosts[hosts.len() - 1].0, hosts[hosts.len() - 1].1);
    let origin = (first_mid + last_mid) * 0.5;

    let mut along = Vec3::new(last_mid.x - first_mid.x, 0.0, last_mid.z - first_mid.z);
    let span_length = along.magnitude().max(1e-5);
    if along.magnitude() <= 1e-5 {
        along = Vec3::new(
            hosts[0].1.x - hosts[0].0.x,
            0.0,
            hosts[0].1.z - hosts[0].0.z,
        );
    }
    if along.magnitude() <= 1e-5 {
        along = Vec3::new(1.0, 0.0, 0.0);
    } else {
        along = along.normalized();
    }
    along = builder_linedef_along(along, host_properties);

    let mount = compute_builder_linedef_mount(origin, along, host_properties);
    let outward = mount.outward;
    let up = mount.up;
    let wall_height = mount.wall_height;
    let host_origin = mount.host_origin;
    let Ok(assembly) = graph.evaluate() else {
        return false;
    };
    if assembly.primitives.is_empty() {
        return false;
    }

    for primitive in &assembly.primitives {
        match primitive {
            BuilderPrimitive::Box {
                size,
                transform,
                material_slot,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                ..
            } => {
                let sx = if *host_scale_x_normalized {
                    transform.scale.x * span_length
                } else {
                    transform.scale.x
                };
                let scaled = Vec3::new(
                    size.x * sx,
                    size.y
                        * if *host_scale_y_normalized {
                            transform.scale.y * wall_height
                        } else {
                            transform.scale.y
                        },
                    size.z * transform.scale.z,
                );
                let tx = if *host_position_normalized {
                    transform.translation.x * span_length
                } else {
                    transform.translation.x
                };
                let ty = if *host_position_y_normalized {
                    transform.translation.y * wall_height
                } else {
                    transform.translation.y
                };
                let center = host_origin
                    + along * tx
                    + up * (ty + scaled.y * 0.5)
                    + outward * transform.translation.z;
                let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
                let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
                let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();
                add_builder_box_mesh(
                    &mut mesh_vertices,
                    &mut mesh_uvs,
                    &mut mesh_indices,
                    center,
                    scaled,
                    transform.rotation_x,
                    transform.rotation_y,
                    along,
                    up,
                    outward,
                );
                let tile_id = resolve_builder_material_tile_id(
                    host_properties,
                    host_properties.get("source"),
                    material_slot.as_deref(),
                    assets,
                );
                vmchunk.add_poly_3d(
                    GeoId::Linedef(host_id),
                    tile_id,
                    mesh_vertices,
                    mesh_uvs,
                    mesh_indices,
                    0,
                    true,
                );
            }
            BuilderPrimitive::Cylinder {
                length,
                radius,
                transform,
                material_slot,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                host_scale_z_normalized: _,
            } => {
                let scaled_length = if *host_scale_y_normalized {
                    *length * transform.scale.y * wall_height
                } else {
                    *length * transform.scale.y
                };
                let scaled_radius = *radius
                    * if *host_scale_x_normalized {
                        transform.scale.z * span_length
                    } else {
                        transform.scale.z
                    };
                let tx = if *host_position_normalized {
                    transform.translation.x * span_length
                } else {
                    transform.translation.x
                };
                let ty = if *host_position_y_normalized {
                    transform.translation.y * wall_height
                } else {
                    transform.translation.y
                };
                let center = host_origin
                    + along * tx
                    + up * (ty + scaled_length * 0.5)
                    + outward * transform.translation.z;
                let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
                let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
                let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();
                add_builder_cylinder_mesh(
                    &mut mesh_vertices,
                    &mut mesh_uvs,
                    &mut mesh_indices,
                    center,
                    scaled_length,
                    scaled_radius,
                    transform.rotation_x,
                    transform.rotation_y,
                    along,
                    up,
                    outward,
                );
                let tile_id = resolve_builder_material_tile_id(
                    host_properties,
                    host_properties.get("source"),
                    material_slot.as_deref(),
                    assets,
                );
                vmchunk.add_poly_3d(
                    GeoId::Linedef(host_id),
                    tile_id,
                    mesh_vertices,
                    mesh_uvs,
                    mesh_indices,
                    0,
                    true,
                );
            }
        }
    }

    emit_builder_attached_item_graphs(
        &assembly,
        host_properties,
        Vec2::new(span_length, wall_height),
        host_origin,
        along,
        up,
        outward,
        GeoId::Vertex(host_id),
        assets,
        vmchunk,
    );
    true
}

#[derive(Clone, Copy, Debug)]
struct BuilderLinedefMount {
    outward: Vec3<f32>,
    up: Vec3<f32>,
    wall_height: f32,
    host_origin: Vec3<f32>,
}

fn builder_linedef_outward(along: Vec3<f32>, props: &ValueContainer) -> Vec3<f32> {
    let explicit = Vec3::new(
        props.get_float_default("builder_graph_outward_x", 0.0),
        props.get_float_default("builder_graph_outward_y", 0.0),
        props.get_float_default("builder_graph_outward_z", 0.0),
    );
    if let Some(outward) = explicit.try_normalized() {
        return outward;
    }
    let mut outward = Vec3::new(-along.z, 0.0, along.x);
    let side = props.get_float_default("builder_graph_wall_side", 0.0);
    if side < 0.0 {
        outward = -outward;
    }
    outward
}

fn builder_linedef_along(fallback_along: Vec3<f32>, props: &ValueContainer) -> Vec3<f32> {
    let explicit = Vec3::new(
        props.get_float_default("host_along_x", 0.0),
        props.get_float_default("host_along_y", 0.0),
        props.get_float_default("host_along_z", 0.0),
    );
    if let Some(along) = explicit.try_normalized() {
        return along;
    }
    fallback_along
}

fn compute_builder_linedef_mount(
    origin: Vec3<f32>,
    along: Vec3<f32>,
    host_properties: &ValueContainer,
) -> BuilderLinedefMount {
    let outward = Vec3::new(
        host_properties.get_float_default("host_outward_x", 0.0),
        host_properties.get_float_default("host_outward_y", 0.0),
        host_properties.get_float_default("host_outward_z", 0.0),
    )
    .try_normalized()
    .unwrap_or_else(|| builder_linedef_outward(along, host_properties));
    let up = Vec3::new(0.0, 1.0, 0.0);
    let wall_height = host_properties
        .get_float_default("wall_height", 2.0)
        .max(0.01);
    let wall_epsilon = host_properties
        .get_float_default("profile_wall_epsilon", 0.001)
        .max(0.0);
    let surface_origin = match (
        host_properties.get_float("builder_graph_surface_origin_x"),
        host_properties.get_float("builder_graph_surface_origin_y"),
        host_properties.get_float("builder_graph_surface_origin_z"),
    ) {
        (Some(x), Some(y), Some(z)) => Some(Vec3::new(x, y, z)),
        _ => None,
    };
    let face_offset = host_properties.get_float("builder_graph_face_offset");
    let face_origin = if let Some(face_offset) = face_offset {
        origin + outward * face_offset.max(wall_epsilon)
    } else if let Some(surface_origin) = surface_origin {
        surface_origin
    } else {
        let wall_offset = host_properties
            .get_float_default("wall_width", 0.0)
            .max(0.0)
            * 0.5
            + wall_epsilon;
        origin + outward * wall_offset
    };
    let host_origin = face_origin - up * (wall_height * 0.5) + outward * wall_epsilon;
    BuilderLinedefMount {
        outward,
        up,
        wall_height,
        host_origin,
    }
}

fn generate_builder_linedef_features(
    map: &Map,
    assets: &Assets,
    chunk: &Chunk,
    vmchunk: &mut scenevm::Chunk,
) {
    let mut grouped: FxHashMap<
        Uuid,
        Vec<(u32, Vec3<f32>, Vec3<f32>, ValueContainer, String, i32)>,
    > = FxHashMap::default();

    for linedef in &map.linedefs {
        let builder_graph_data = linedef
            .properties
            .get_str_default("builder_graph_data", String::new());
        if builder_graph_data.trim().is_empty() {
            continue;
        }
        let Ok(graph) = BuilderDocument::from_text(&builder_graph_data) else {
            continue;
        };
        if graph.output_spec().target != buildergraph::BuilderOutputTarget::Linedef {
            continue;
        }
        let Some(v0) = map.get_vertex_3d(linedef.start_vertex) else {
            continue;
        };
        let Some(v1) = map.get_vertex_3d(linedef.end_vertex) else {
            continue;
        };
        let line_mid = Vec2::new((v0.x + v1.x) * 0.5, (v0.z + v1.z) * 0.5);
        if !chunk.bbox.contains(line_mid) {
            continue;
        }
        let group_id = match linedef.properties.get("builder_graph_group_id") {
            Some(Value::Id(id)) => *id,
            _ => match linedef.properties.get("builder_graph_id") {
                Some(Value::Id(id)) => *id,
                _ => Uuid::nil(),
            },
        };
        let group_order = linedef
            .properties
            .get_int_default("builder_graph_group_order", linedef.id as i32);
        grouped.entry(group_id).or_default().push((
            linedef.id,
            v0,
            v1,
            linedef.properties.clone(),
            builder_graph_data,
            group_order,
        ));
    }

    for (_group_id, entries) in grouped {
        if entries.is_empty() {
            continue;
        }
        let Ok(graph) = BuilderDocument::from_text(&entries[0].4) else {
            continue;
        };
        let host_refs = graph.output_spec().host_refs.max(1) as usize;
        if host_refs == 1 {
            for (linedef_id, v0, v1, properties, _graph_data, _) in entries.iter() {
                let _ = emit_builder_linedef_group_meshes(
                    &graph,
                    *linedef_id,
                    properties,
                    &[(*v0, *v1)],
                    assets,
                    vmchunk,
                );
            }
            continue;
        }

        let mut sorted = entries;
        sorted.sort_by_key(|entry| entry.5);
        for group in sorted.chunks(host_refs) {
            if group.len() < 2 {
                continue;
            }
            let host_id = group[0].0;
            let properties = &group[0].3;
            let hosts: Vec<(Vec3<f32>, Vec3<f32>)> =
                group.iter().map(|entry| (entry.1, entry.2)).collect();
            let _ = emit_builder_linedef_group_meshes(
                &graph, host_id, properties, &hosts, assets, vmchunk,
            );
        }
    }
}

fn emit_builder_vertex_meshes(
    graph: &BuilderDocument,
    host_id: u32,
    host_properties: &ValueContainer,
    hosts: &[Vec3<f32>],
    assets: &Assets,
    vmchunk: &mut scenevm::Chunk,
) -> bool {
    if hosts.is_empty() {
        return false;
    }

    let spec = graph.output_spec();
    if spec.target != buildergraph::BuilderOutputTarget::VertexPair {
        return false;
    }

    let origin = if let (Some(x), Some(y), Some(z)) = (
        host_properties.get_float("host_surface_origin_x"),
        host_properties.get_float("host_surface_origin_y"),
        host_properties.get_float("host_surface_origin_z"),
    ) {
        Vec3::new(x, y, z)
    } else if hosts.len() == 1 {
        hosts[0]
    } else {
        (hosts[0] + hosts[hosts.len() - 1]) * 0.5
    };

    let along = if hosts.len() >= 2 {
        let delta = Vec3::new(
            hosts[hosts.len() - 1].x - hosts[0].x,
            0.0,
            hosts[hosts.len() - 1].z - hosts[0].z,
        );
        if delta.magnitude() > 1e-5 {
            delta.normalized()
        } else {
            Vec3::new(1.0, 0.0, 0.0)
        }
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };
    let along = builder_linedef_along(along, host_properties);
    let span_length = if hosts.len() >= 2 {
        Vec3::new(
            hosts[hosts.len() - 1].x - hosts[0].x,
            0.0,
            hosts[hosts.len() - 1].z - hosts[0].z,
        )
        .magnitude()
        .max(1e-5)
    } else {
        1.0
    };
    let outward = Vec3::new(
        host_properties.get_float_default("host_outward_x", 0.0),
        host_properties.get_float_default("host_outward_y", 0.0),
        host_properties.get_float_default("host_outward_z", 0.0),
    )
    .try_normalized()
    .unwrap_or_else(|| builder_linedef_outward(along, host_properties));
    let up = Vec3::new(0.0, 1.0, 0.0);
    let Ok(assembly) = graph.evaluate() else {
        return false;
    };
    if assembly.primitives.is_empty() {
        return false;
    }

    for primitive in &assembly.primitives {
        match primitive {
            BuilderPrimitive::Box {
                size,
                transform,
                material_slot,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                ..
            } => {
                let sx = if *host_scale_x_normalized {
                    transform.scale.x * span_length
                } else {
                    transform.scale.x
                };
                let scaled = Vec3::new(
                    size.x * sx,
                    size.y
                        * if *host_scale_y_normalized {
                            transform.scale.y
                        } else {
                            transform.scale.y
                        },
                    size.z * transform.scale.z,
                );
                let tx = if *host_position_normalized {
                    transform.translation.x * span_length
                } else {
                    transform.translation.x
                };
                let ty = if *host_position_y_normalized {
                    transform.translation.y
                } else {
                    transform.translation.y
                };
                let center = origin
                    + along * tx
                    + up * (ty + scaled.y * 0.5)
                    + outward * transform.translation.z;
                let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
                let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
                let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();
                add_builder_box_mesh(
                    &mut mesh_vertices,
                    &mut mesh_uvs,
                    &mut mesh_indices,
                    center,
                    scaled,
                    transform.rotation_x,
                    transform.rotation_y,
                    along,
                    up,
                    outward,
                );
                let tile_id = resolve_builder_material_tile_id(
                    host_properties,
                    host_properties.get("source"),
                    material_slot.as_deref(),
                    assets,
                );
                vmchunk.add_poly_3d(
                    GeoId::Vertex(host_id),
                    tile_id,
                    mesh_vertices,
                    mesh_uvs,
                    mesh_indices,
                    0,
                    true,
                );
            }
            BuilderPrimitive::Cylinder {
                length,
                radius,
                transform,
                material_slot,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                host_scale_z_normalized: _,
            } => {
                let scaled_length = if *host_scale_y_normalized {
                    *length * transform.scale.y
                } else {
                    *length * transform.scale.y
                };
                let scaled_radius = *radius
                    * if *host_scale_x_normalized {
                        transform.scale.z * span_length
                    } else {
                        transform.scale.z
                    };
                let tx = if *host_position_normalized {
                    transform.translation.x * span_length
                } else {
                    transform.translation.x
                };
                let ty = if *host_position_y_normalized {
                    transform.translation.y
                } else {
                    transform.translation.y
                };
                let center = origin
                    + along * tx
                    + up * (ty + scaled_length * 0.5)
                    + outward * transform.translation.z;
                let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
                let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
                let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();
                add_builder_cylinder_mesh(
                    &mut mesh_vertices,
                    &mut mesh_uvs,
                    &mut mesh_indices,
                    center,
                    scaled_length,
                    scaled_radius,
                    transform.rotation_x,
                    transform.rotation_y,
                    along,
                    up,
                    outward,
                );
                let tile_id = resolve_builder_material_tile_id(
                    host_properties,
                    host_properties.get("source"),
                    material_slot.as_deref(),
                    assets,
                );
                vmchunk.add_poly_3d(
                    GeoId::Vertex(host_id),
                    tile_id,
                    mesh_vertices,
                    mesh_uvs,
                    mesh_indices,
                    0,
                    true,
                );
            }
        }
    }
    emit_builder_attached_item_graphs(
        &assembly,
        host_properties,
        Vec2::new(span_length, 1.0),
        origin,
        along,
        up,
        outward,
        GeoId::Linedef(host_id),
        assets,
        vmchunk,
    );
    true
}

fn generate_vertex_builder_features(
    map: &Map,
    assets: &Assets,
    chunk: &Chunk,
    vmchunk: &mut scenevm::Chunk,
) {
    let mut grouped: FxHashMap<Uuid, Vec<(u32, Vec3<f32>, ValueContainer, String, i32)>> =
        FxHashMap::default();

    for vertex in &map.vertices {
        let builder_graph_data = vertex
            .properties
            .get_str_default("builder_graph_data", String::new());
        if builder_graph_data.trim().is_empty() {
            continue;
        }
        let Ok(graph) = BuilderDocument::from_text(&builder_graph_data) else {
            continue;
        };
        if graph.output_spec().target != buildergraph::BuilderOutputTarget::VertexPair {
            continue;
        }
        let pos = Vec3::new(vertex.x, vertex.z, vertex.y);
        if !chunk.bbox.contains(Vec2::new(pos.x, pos.z)) {
            continue;
        }
        let group_id = match vertex.properties.get("builder_graph_group_id") {
            Some(Value::Id(id)) => *id,
            _ => match vertex.properties.get("builder_graph_id") {
                Some(Value::Id(id)) => *id,
                _ => Uuid::nil(),
            },
        };
        let group_order = vertex
            .properties
            .get_int_default("builder_graph_group_order", vertex.id as i32);
        grouped.entry(group_id).or_default().push((
            vertex.id,
            pos,
            vertex.properties.clone(),
            builder_graph_data,
            group_order,
        ));
    }

    for (_group_id, entries) in grouped {
        if entries.is_empty() {
            continue;
        }
        let Ok(graph) = BuilderDocument::from_text(&entries[0].3) else {
            continue;
        };
        let host_refs = graph.output_spec().host_refs.max(1) as usize;
        if host_refs == 1 {
            for (host_id, pos, properties, _, _) in entries.iter() {
                let _ = emit_builder_vertex_meshes(
                    &graph,
                    *host_id,
                    properties,
                    &[*pos],
                    assets,
                    vmchunk,
                );
            }
        } else {
            let mut sorted = entries;
            sorted.sort_by_key(|entry| entry.4);
            for group in sorted.chunks(host_refs) {
                if group.len() < 2 {
                    continue;
                }
                let host_id = group[0].0;
                let properties = &group[0].2;
                let hosts: Vec<Vec3<f32>> = group.iter().map(|entry| entry.1).collect();
                let _ = emit_builder_vertex_meshes(
                    &graph, host_id, properties, &hosts, assets, vmchunk,
                );
            }
        }
    }
}

fn add_ring(
    mesh_vertices: &mut Vec<[f32; 4]>,
    mesh_uvs: &mut Vec<[f32; 2]>,
    center: Vec3<f32>,
    right: Vec3<f32>,
    forward: Vec3<f32>,
    points: &[(f32, f32)],
) -> Vec<usize> {
    let mut ring = Vec::with_capacity(points.len());
    for &(x, z) in points {
        let p = center + right * x + forward * z;
        ring.push(add_vertex(mesh_vertices, mesh_uvs, p));
    }
    ring
}

fn add_cap_top(mesh_indices: &mut Vec<(usize, usize, usize)>, ring: &[usize]) {
    if ring.len() < 3 {
        return;
    }
    for i in 1..(ring.len() - 1) {
        add_tri(mesh_indices, ring[0], ring[i], ring[i + 1]);
    }
}

fn add_cap_bottom(mesh_indices: &mut Vec<(usize, usize, usize)>, ring: &[usize]) {
    if ring.len() < 3 {
        return;
    }
    for i in 1..(ring.len() - 1) {
        add_tri(mesh_indices, ring[0], ring[i + 1], ring[i]);
    }
}

fn add_prism_stake(
    mesh_vertices: &mut Vec<[f32; 4]>,
    mesh_uvs: &mut Vec<[f32; 2]>,
    mesh_indices: &mut Vec<(usize, usize, usize)>,
    base_center: Vec3<f32>,
    top_center: Vec3<f32>,
    right: Vec3<f32>,
    forward: Vec3<f32>,
    half_w: f32,
    half_d: f32,
    top_mode: i32,
    top_height: f32,
) {
    let base_points = [
        (-half_w, -half_d),
        (half_w, -half_d),
        (half_w, half_d),
        (-half_w, half_d),
    ];
    let bottom_ring = add_ring(
        mesh_vertices,
        mesh_uvs,
        base_center,
        right,
        forward,
        &base_points,
    );
    let top_ring = add_ring(
        mesh_vertices,
        mesh_uvs,
        top_center,
        right,
        forward,
        &base_points,
    );
    for i in 0..4 {
        let ni = (i + 1) % 4;
        add_quad(
            mesh_indices,
            bottom_ring[i],
            bottom_ring[ni],
            top_ring[ni],
            top_ring[i],
        );
    }
    add_cap_bottom(mesh_indices, &bottom_ring);

    match top_mode {
        1 => {
            let apex = add_vertex(
                mesh_vertices,
                mesh_uvs,
                top_center + Vec3::new(0.0, top_height, 0.0),
            );
            for i in 0..4 {
                let ni = (i + 1) % 4;
                add_tri(mesh_indices, top_ring[i], top_ring[ni], apex);
            }
        }
        2 => {
            let scale = 0.45;
            let bevel_points = [
                (-half_w * scale, -half_d * scale),
                (half_w * scale, -half_d * scale),
                (half_w * scale, half_d * scale),
                (-half_w * scale, half_d * scale),
            ];
            let bevel_center = top_center + Vec3::new(0.0, top_height, 0.0);
            let bevel_ring = add_ring(
                mesh_vertices,
                mesh_uvs,
                bevel_center,
                right,
                forward,
                &bevel_points,
            );
            for i in 0..4 {
                let ni = (i + 1) % 4;
                add_quad(
                    mesh_indices,
                    top_ring[i],
                    top_ring[ni],
                    bevel_ring[ni],
                    bevel_ring[i],
                );
            }
            add_cap_top(mesh_indices, &bevel_ring);
        }
        _ => {
            add_cap_top(mesh_indices, &top_ring);
        }
    }
}

fn add_round_stake(
    mesh_vertices: &mut Vec<[f32; 4]>,
    mesh_uvs: &mut Vec<[f32; 2]>,
    mesh_indices: &mut Vec<(usize, usize, usize)>,
    base_center: Vec3<f32>,
    top_center: Vec3<f32>,
    right: Vec3<f32>,
    forward: Vec3<f32>,
    radius: f32,
    segments: usize,
    top_mode: i32,
    top_height: f32,
) {
    if segments < 3 {
        return;
    }
    let mut circle = Vec::with_capacity(segments);
    for i in 0..segments {
        let t = (i as f32) / (segments as f32);
        let a = t * std::f32::consts::TAU;
        circle.push((a.cos() * radius, a.sin() * radius));
    }

    let bottom_ring = add_ring(
        mesh_vertices,
        mesh_uvs,
        base_center,
        right,
        forward,
        &circle,
    );
    let top_ring = add_ring(mesh_vertices, mesh_uvs, top_center, right, forward, &circle);

    for i in 0..segments {
        let ni = (i + 1) % segments;
        add_quad(
            mesh_indices,
            bottom_ring[i],
            bottom_ring[ni],
            top_ring[ni],
            top_ring[i],
        );
    }
    add_cap_bottom(mesh_indices, &bottom_ring);

    match top_mode {
        1 => {
            let apex = add_vertex(
                mesh_vertices,
                mesh_uvs,
                top_center + Vec3::new(0.0, top_height, 0.0),
            );
            for i in 0..segments {
                let ni = (i + 1) % segments;
                add_tri(mesh_indices, top_ring[i], top_ring[ni], apex);
            }
        }
        2 => {
            let bevel_center = top_center + Vec3::new(0.0, top_height, 0.0);
            let mut bevel = Vec::with_capacity(segments);
            for i in 0..segments {
                let t = (i as f32) / (segments as f32);
                let a = t * std::f32::consts::TAU;
                bevel.push((a.cos() * radius * 0.5, a.sin() * radius * 0.5));
            }
            let bevel_ring = add_ring(
                mesh_vertices,
                mesh_uvs,
                bevel_center,
                right,
                forward,
                &bevel,
            );
            for i in 0..segments {
                let ni = (i + 1) % segments;
                add_quad(
                    mesh_indices,
                    top_ring[i],
                    top_ring[ni],
                    bevel_ring[ni],
                    bevel_ring[i],
                );
            }
            add_cap_top(mesh_indices, &bevel_ring);
        }
        _ => {
            add_cap_top(mesh_indices, &top_ring);
        }
    }
}

fn generate_linedef_features(
    map: &Map,
    assets: &Assets,
    chunk: &Chunk,
    vmchunk: &mut scenevm::Chunk,
) {
    let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();

    for linedef in &map.linedefs {
        let feature = linedef
            .properties
            .get_str_default("linedef_feature", "None".to_string());
        if feature != "Palisade" && feature != "Fence" {
            continue;
        }

        let Some(v0) = map.get_vertex_3d(linedef.start_vertex) else {
            continue;
        };
        let Some(v1) = map.get_vertex_3d(linedef.end_vertex) else {
            continue;
        };

        let line_mid = Vec2::new((v0.x + v1.x) * 0.5, (v0.z + v1.z) * 0.5);
        if !chunk.bbox.contains(line_mid) {
            continue;
        }

        let line_flat = Vec3::new(v1.x - v0.x, 0.0, v1.z - v0.z);
        let line_len = line_flat.magnitude();
        if line_len < 1e-5 {
            continue;
        }
        let forward = line_flat / line_len;
        let right = Vec3::new(-forward.z, 0.0, forward.x);
        let up = Vec3::new(0.0, 1.0, 0.0);

        let spacing = linedef
            .properties
            .get_float_default(
                "feature_layout_spacing",
                if feature == "Fence" { 1.5 } else { 1.0 },
            )
            .max(0.1);
        let round_segments = linedef
            .properties
            .get_int_default("feature_round_segments", 8)
            .max(3) as usize;
        let base_height = linedef.properties.get_float_default("feature_height", 2.0);
        if base_height <= 0.0 {
            continue;
        }
        let top_mode = linedef.properties.get_int_default("feature_top_mode", 0);
        let top_height = linedef
            .properties
            .get_float_default("feature_top_height", 0.5)
            .max(0.0);
        let height_variation = linedef
            .properties
            .get_float_default(
                "feature_height_variation",
                if feature == "Fence" { 0.0 } else { 0.35 },
            )
            .max(0.0);
        let lean_amount = linedef
            .properties
            .get_float_default("feature_lean_amount", 0.0)
            .max(0.0);
        let lean_randomness = linedef
            .properties
            .get_float_default("feature_lean_randomness", 1.0)
            .clamp(0.0, 1.0);

        let tile_id = if let Some(Value::Source(ps)) = linedef.properties.get("feature_source") {
            ps.tile_from_tile_list(assets).map(|tile| tile.id)
        } else if let Some(Value::Source(ps)) = linedef.properties.get("row1_source") {
            ps.tile_from_tile_list(assets).map(|tile| tile.id)
        } else {
            None
        }
        .unwrap_or(default_tile_id);

        let count = ((line_len / spacing).floor() as usize).saturating_add(1);
        let count = count.max(1);

        let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
        let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
        let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();
        let mut post_points: Vec<(Vec3<f32>, Vec3<f32>)> = Vec::new();

        for i in 0..count {
            let t = if count <= 1 {
                0.5
            } else {
                (i as f32) / ((count - 1) as f32)
            };

            let base_center = Vec3::new(
                v0.x + (v1.x - v0.x) * t,
                v0.y + (v1.y - v0.y) * t,
                v0.z + (v1.z - v0.z) * t,
            );

            let seed = linedef
                .id
                .wrapping_mul(0x9e3779b1)
                .wrapping_add(i as u32 * 0x85ebca6b);
            let height_rand = hash01(seed) * 2.0 - 1.0;
            let height = (base_height + height_rand * height_variation).max(0.05);

            let lean_x = (hash01(seed ^ 0x68bc21eb) * 2.0 - 1.0) * lean_amount * lean_randomness;
            let lean_z = (hash01(seed ^ 0x2c1b3c6d) * 2.0 - 1.0) * lean_amount * lean_randomness;
            let top_center =
                base_center + Vec3::new(0.0, height, 0.0) + right * lean_x + forward * lean_z;

            let stake_top_mode = if top_mode == 3 {
                (hash01(seed ^ 0xf00d1234) * 3.0).floor() as i32
            } else {
                top_mode
            };

            if feature == "Palisade" {
                let segment_size = linedef
                    .properties
                    .get_float_default("feature_segment_size", 0.75)
                    .max(0.05);
                let shape = linedef.properties.get_int_default("feature_shape", 1);
                let depth = linedef
                    .properties
                    .get_float_default("feature_depth", 0.12)
                    .max(0.02);

                match shape {
                    2 => {
                        let radius = (segment_size * 0.5).max(depth * 0.5);
                        add_round_stake(
                            &mut mesh_vertices,
                            &mut mesh_uvs,
                            &mut mesh_indices,
                            base_center,
                            top_center,
                            right,
                            forward,
                            radius,
                            round_segments,
                            stake_top_mode,
                            top_height,
                        );
                    }
                    1 => {
                        let half = (segment_size * 0.5).max(depth * 0.5);
                        add_prism_stake(
                            &mut mesh_vertices,
                            &mut mesh_uvs,
                            &mut mesh_indices,
                            base_center,
                            top_center,
                            right,
                            forward,
                            half,
                            half,
                            stake_top_mode,
                            top_height,
                        );
                    }
                    _ => {
                        let half_w = segment_size * 0.5;
                        let half_d = (depth * 0.5).max(0.01);
                        add_prism_stake(
                            &mut mesh_vertices,
                            &mut mesh_uvs,
                            &mut mesh_indices,
                            base_center,
                            top_center,
                            right,
                            forward,
                            half_w,
                            half_d,
                            stake_top_mode,
                            top_height,
                        );
                    }
                }
            } else {
                let post_size = linedef
                    .properties
                    .get_float_default("feature_post_size", 0.18)
                    .max(0.02);
                let post_shape = linedef.properties.get_int_default("feature_post_shape", 0);
                let half = post_size * 0.5;

                match post_shape {
                    1 => {
                        add_round_stake(
                            &mut mesh_vertices,
                            &mut mesh_uvs,
                            &mut mesh_indices,
                            base_center,
                            top_center,
                            right,
                            forward,
                            half,
                            round_segments,
                            0,
                            0.0,
                        );
                    }
                    _ => {
                        add_prism_stake(
                            &mut mesh_vertices,
                            &mut mesh_uvs,
                            &mut mesh_indices,
                            base_center,
                            top_center,
                            right,
                            forward,
                            half,
                            half,
                            0,
                            0.0,
                        );
                    }
                }
                post_points.push((base_center, top_center));
            }
        }

        if feature == "Fence" {
            let connector_count = linedef
                .properties
                .get_int_default("feature_connector_count", 2)
                .max(0) as usize;
            let connector_style = linedef
                .properties
                .get_int_default("feature_connector_style", 0);
            let connector_size = linedef
                .properties
                .get_float_default("feature_connector_size", 0.12)
                .max(0.01);
            let connector_drop = linedef
                .properties
                .get_float_default("feature_connector_drop", 1.2)
                .max(0.0);

            if connector_count > 0 && post_points.len() >= 2 {
                for pair in post_points.windows(2) {
                    let (base_a, top_a) = pair[0];
                    let (base_b, top_b) = pair[1];

                    let min_top = top_a.y.min(top_b.y);
                    let min_base = base_a.y.min(base_b.y);
                    let first_y = min_top - connector_size * 0.5 - 0.05;
                    let step = if connector_count > 1 {
                        connector_drop / (connector_count.saturating_sub(1) as f32)
                    } else {
                        0.0
                    };

                    for ci in 0..connector_count {
                        let y = first_y - step * ci as f32;
                        if y <= min_base + connector_size {
                            continue;
                        }

                        let start = Vec3::new(base_a.x, y, base_a.z);
                        let end = Vec3::new(base_b.x, y, base_b.z);

                        match connector_style {
                            2 => {
                                add_round_stake(
                                    &mut mesh_vertices,
                                    &mut mesh_uvs,
                                    &mut mesh_indices,
                                    start,
                                    end,
                                    up,
                                    right,
                                    connector_size * 0.5,
                                    round_segments.max(6),
                                    0,
                                    0.0,
                                );
                            }
                            1 => {
                                add_prism_stake(
                                    &mut mesh_vertices,
                                    &mut mesh_uvs,
                                    &mut mesh_indices,
                                    start,
                                    end,
                                    up,
                                    right,
                                    connector_size * 0.5,
                                    connector_size * 0.5,
                                    0,
                                    0.0,
                                );
                            }
                            _ => {
                                add_prism_stake(
                                    &mut mesh_vertices,
                                    &mut mesh_uvs,
                                    &mut mesh_indices,
                                    start,
                                    end,
                                    up,
                                    right,
                                    connector_size * 0.5,
                                    (connector_size * 0.2).max(0.01),
                                    0,
                                    0.0,
                                );
                            }
                        }
                    }
                }
            }
        }

        if !mesh_indices.is_empty() {
            // Orient UVs to the feature direction instead of world X/Z so texture flow
            // stays continuous along the linedef.
            let mut oriented_uvs: Vec<[f32; 2]> = Vec::with_capacity(mesh_vertices.len());
            for v in &mesh_vertices {
                let p = Vec3::new(v[0], v[1], v[2]);
                oriented_uvs.push([p.dot(forward), p.y]);
            }
            vmchunk.add_poly_3d(
                GeoId::Linedef(linedef.id),
                tile_id,
                mesh_vertices,
                oriented_uvs,
                mesh_indices,
                0,
                true,
            );
        }
    }
}

fn source_to_tile_id(props: &ValueContainer, key: &str, assets: &Assets) -> Option<Uuid> {
    let Value::Source(ps) = props.get(key)? else {
        return None;
    };
    ps.tile_from_tile_list(assets).map(|t| t.id)
}

fn pixel_source_to_tile_id(source: &PixelSource, assets: &Assets) -> Option<Uuid> {
    source.tile_from_tile_list(assets).map(|tile| tile.id)
}

#[derive(Clone, Copy)]
struct OrganicRenderSpan {
    x: i32,
    y: i32,
    bottom: f32,
    top: f32,
    tile_id: Uuid,
}

impl OrganicRenderSpan {
    fn outer(&self) -> f32 {
        if self.top.abs() >= self.bottom.abs() {
            self.top
        } else {
            self.bottom
        }
    }

    fn inner(&self) -> f32 {
        if self.top.abs() >= self.bottom.abs() {
            self.bottom
        } else {
            self.top
        }
    }

    fn outer_sign(&self) -> f32 {
        if self.outer() >= 0.0 { 1.0 } else { -1.0 }
    }
}

fn collect_organic_render_spans(
    columns: &[crate::OrganicColumn],
    tile_for_span: impl Fn(&crate::OrganicSpan) -> Uuid,
) -> Vec<OrganicRenderSpan> {
    let mut spans = Vec::new();
    for column in columns {
        for span in &column.spans {
            spans.push(OrganicRenderSpan {
                x: column.x,
                y: column.y,
                bottom: span.offset,
                top: span.offset + span.height.max(0.01),
                tile_id: tile_for_span(span),
            });
        }
    }
    spans
}

fn organic_overlap_top(
    span_lookup: &FxHashMap<(i32, i32), Vec<OrganicRenderSpan>>,
    x: i32,
    y: i32,
    bottom: f32,
    top: f32,
) -> Option<f32> {
    let outer = if top.abs() >= bottom.abs() {
        top
    } else {
        bottom
    };
    let inner = if top.abs() >= bottom.abs() {
        bottom
    } else {
        top
    };
    span_lookup.get(&(x, y)).and_then(|spans| {
        spans
            .iter()
            .filter(|other| other.outer().abs() > inner.abs() && other.inner().abs() < outer.abs())
            .map(|other| {
                if outer >= 0.0 {
                    other.outer().min(outer)
                } else {
                    other.outer().max(outer)
                }
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    })
}

fn build_organic_corner_lookup(spans: &[OrganicRenderSpan]) -> FxHashMap<(i32, i32), f32> {
    let mut accum: FxHashMap<(i32, i32), (f32, f32)> = FxHashMap::default();
    for span in spans {
        for corner in [
            (span.x, span.y),
            (span.x + 1, span.y),
            (span.x + 1, span.y + 1),
            (span.x, span.y + 1),
        ] {
            let entry = accum.entry((corner.0, corner.1)).or_insert((0.0, 0.0));
            entry.0 += span.outer();
            entry.1 += 1.0;
        }
    }

    let mut result = FxHashMap::default();
    for (key, (sum, count)) in accum {
        if count > 0.0 {
            result.insert(key, sum / count);
        }
    }
    result
}

fn render_organic_shell(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    edge_normals: [Vec3<f32>; 4],
    side_quads: [[Vec3<f32>; 4]; 4],
) {
    let side_uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
    for (edge_index, quad) in side_quads.iter().enumerate() {
        if (quad[3] - quad[0]).magnitude() <= 0.001 && (quad[2] - quad[1]).magnitude() <= 0.001 {
            continue;
        }
        push_quad_with_winding(
            vmchunk,
            geo,
            tile_id,
            vec![
                [quad[0].x, quad[0].y, quad[0].z, 1.0],
                [quad[1].x, quad[1].y, quad[1].z, 1.0],
                [quad[2].x, quad[2].y, quad[2].z, 1.0],
                [quad[3].x, quad[3].y, quad[3].z, 1.0],
            ],
            side_uvs.clone(),
            edge_normals[edge_index],
        );
    }
}

fn is_ribbon_span(
    north_connected: bool,
    east_connected: bool,
    south_connected: bool,
    west_connected: bool,
) -> bool {
    (!north_connected && !south_connected && east_connected && west_connected)
        || (!east_connected && !west_connected && north_connected && south_connected)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RibbonAxis {
    Horizontal,
    Vertical,
}

fn ribbon_axis(
    north_connected: bool,
    east_connected: bool,
    south_connected: bool,
    west_connected: bool,
) -> Option<RibbonAxis> {
    if !north_connected && !south_connected && east_connected && west_connected {
        Some(RibbonAxis::Horizontal)
    } else if !east_connected && !west_connected && north_connected && south_connected {
        Some(RibbonAxis::Vertical)
    } else {
        None
    }
}

fn ribbon_bounds(
    min_a: f32,
    max_a: f32,
    min_b: f32,
    max_b: f32,
    inset: f32,
    north_connected: bool,
    east_connected: bool,
    south_connected: bool,
    west_connected: bool,
) -> (f32, f32, f32, f32) {
    if north_connected && south_connected {
        (min_a + inset, max_a - inset, min_b, max_b)
    } else if east_connected && west_connected {
        (min_a, max_a, min_b + inset, max_b - inset)
    } else {
        (min_a, max_a, min_b, max_b)
    }
}

fn collect_ribbon_runs(
    spans: &[OrganicRenderSpan],
    span_lookup: &FxHashMap<(i32, i32), Vec<OrganicRenderSpan>>,
) -> Vec<(RibbonAxis, Vec<OrganicRenderSpan>)> {
    let mut visited: FxHashSet<(i32, i32, i32)> = FxHashSet::default();
    let mut runs = Vec::new();

    for span in spans {
        let key = (span.x, span.y, span.outer_sign() as i32);
        if visited.contains(&key) {
            continue;
        }
        let inner = span.inner();
        let outer = span.outer();
        let north = organic_overlap_top(&span_lookup, span.x, span.y - 1, inner, outer).is_some();
        let east = organic_overlap_top(&span_lookup, span.x + 1, span.y, inner, outer).is_some();
        let south = organic_overlap_top(&span_lookup, span.x, span.y + 1, inner, outer).is_some();
        let west = organic_overlap_top(&span_lookup, span.x - 1, span.y, inner, outer).is_some();
        let Some(axis) = ribbon_axis(north, east, south, west) else {
            continue;
        };

        let mut run = vec![*span];
        visited.insert(key);

        match axis {
            RibbonAxis::Horizontal => {
                let mut x = span.x - 1;
                loop {
                    let found = spans.iter().find(|other| {
                        other.x == x
                            && other.y == span.y
                            && other.tile_id == span.tile_id
                            && other.outer_sign() as i32 == span.outer_sign() as i32
                            && ribbon_axis(
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y - 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x + 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y + 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x - 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                            ) == Some(RibbonAxis::Horizontal)
                    });
                    let Some(found) = found else { break };
                    visited.insert((found.x, found.y, found.outer_sign() as i32));
                    run.push(*found);
                    x -= 1;
                }
                let mut x = span.x + 1;
                loop {
                    let found = spans.iter().find(|other| {
                        other.x == x
                            && other.y == span.y
                            && other.tile_id == span.tile_id
                            && other.outer_sign() as i32 == span.outer_sign() as i32
                            && ribbon_axis(
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y - 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x + 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y + 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x - 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                            ) == Some(RibbonAxis::Horizontal)
                    });
                    let Some(found) = found else { break };
                    visited.insert((found.x, found.y, found.outer_sign() as i32));
                    run.push(*found);
                    x += 1;
                }
                run.sort_by_key(|s| s.x);
            }
            RibbonAxis::Vertical => {
                let mut y = span.y - 1;
                loop {
                    let found = spans.iter().find(|other| {
                        other.x == span.x
                            && other.y == y
                            && other.tile_id == span.tile_id
                            && other.outer_sign() as i32 == span.outer_sign() as i32
                            && ribbon_axis(
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y - 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x + 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y + 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x - 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                            ) == Some(RibbonAxis::Vertical)
                    });
                    let Some(found) = found else { break };
                    visited.insert((found.x, found.y, found.outer_sign() as i32));
                    run.push(*found);
                    y -= 1;
                }
                let mut y = span.y + 1;
                loop {
                    let found = spans.iter().find(|other| {
                        other.x == span.x
                            && other.y == y
                            && other.tile_id == span.tile_id
                            && other.outer_sign() as i32 == span.outer_sign() as i32
                            && ribbon_axis(
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y - 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x + 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x,
                                    other.y + 1,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                                organic_overlap_top(
                                    &span_lookup,
                                    other.x - 1,
                                    other.y,
                                    other.inner(),
                                    other.outer(),
                                )
                                .is_some(),
                            ) == Some(RibbonAxis::Vertical)
                    });
                    let Some(found) = found else { break };
                    visited.insert((found.x, found.y, found.outer_sign() as i32));
                    run.push(*found);
                    y += 1;
                }
                run.sort_by_key(|s| s.y);
            }
        }

        if run.len() > 1 {
            runs.push((axis, run));
        }
    }

    runs
}

fn push_organic_top_mesh(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    desired_normal: Vec3<f32>,
    vertices: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<(usize, usize, usize)>,
) {
    if vertices.is_empty() || indices.is_empty() {
        return;
    }
    let mut indices = indices;
    fix_winding(&vertices, &mut indices, desired_normal);
    vmchunk.add_poly_3d(geo, tile_id, vertices, uvs, indices, 0, true);
}

fn push_organic_strip_mesh(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    desired_normal: Vec3<f32>,
    pairs: &[(Vec3<f32>, Vec3<f32>)],
    uv_axis_horizontal: bool,
) {
    if pairs.len() < 2 {
        return;
    }
    let mut vertices = Vec::with_capacity(pairs.len() * 2);
    let mut uvs = Vec::with_capacity(pairs.len() * 2);
    let mut indices = Vec::with_capacity((pairs.len() - 1) * 2);
    let mut acc = 0.0f32;
    for (i, pair) in pairs.iter().enumerate() {
        if i > 0 {
            acc += (pairs[i].0 - pairs[i - 1].0).magnitude();
        }
        vertices.push([pair.0.x, pair.0.y, pair.0.z, 1.0]);
        vertices.push([pair.1.x, pair.1.y, pair.1.z, 1.0]);
        if uv_axis_horizontal {
            uvs.push([acc, 1.0]);
            uvs.push([acc, 0.0]);
        } else {
            uvs.push([1.0, acc]);
            uvs.push([0.0, acc]);
        }
    }
    for i in 0..pairs.len() - 1 {
        let a = i * 2;
        let b = a + 1;
        let c = a + 3;
        let d = a + 2;
        indices.push((a, d, c));
        indices.push((a, c, b));
    }
    push_organic_top_mesh(
        vmchunk,
        geo,
        tile_id,
        desired_normal,
        vertices,
        uvs,
        indices,
    );
}

fn collect_vine_runs<'a, F>(
    strokes: &'a [crate::OrganicVineStroke],
    mut tile_for: F,
) -> Vec<(Uuid, Vec<&'a crate::OrganicVineStroke>)>
where
    F: FnMut(&'a crate::OrganicVineStroke) -> Uuid,
{
    let mut by_stroke: FxHashMap<i32, Vec<&'a crate::OrganicVineStroke>> = FxHashMap::default();
    for stroke in strokes {
        by_stroke.entry(stroke.stroke_id).or_default().push(stroke);
    }

    let mut runs = Vec::new();
    for (_stroke_id, mut segs) in by_stroke {
        segs.sort_by_key(|seg| seg.seq);

        let mut current_tile = None;
        let mut current: Vec<&'a crate::OrganicVineStroke> = Vec::new();
        let mut prev_seq = None;

        for seg in segs {
            let tile_id = tile_for(seg);
            let split =
                current_tile != Some(tile_id) || prev_seq.is_some_and(|prev| seg.seq != prev + 1);

            if split && !current.is_empty() {
                runs.push((current_tile.unwrap(), std::mem::take(&mut current)));
            }

            current_tile = Some(tile_id);
            prev_seq = Some(seg.seq);
            current.push(seg);
        }

        if !current.is_empty() {
            runs.push((current_tile.unwrap(), current));
        }
    }

    runs
}

fn build_vine_ribbon_pairs(
    points: &[Vec2<f32>],
    half_width: f32,
    taper_start: bool,
    taper_end: bool,
) -> Vec<(Vec2<f32>, Vec2<f32>)> {
    if points.len() < 2 {
        return Vec::new();
    }

    let mut pairs = Vec::with_capacity(points.len());
    for i in 0..points.len() {
        let prev = if i == 0 {
            points[1] - points[0]
        } else {
            points[i] - points[i - 1]
        };
        let next = if i + 1 == points.len() {
            points[i] - points[i - 1]
        } else {
            points[i + 1] - points[i]
        };
        let prev_n = if prev.magnitude_squared() > 1e-8 {
            prev.normalized()
        } else {
            Vec2::new(1.0, 0.0)
        };
        let next_n = if next.magnitude_squared() > 1e-8 {
            next.normalized()
        } else {
            prev_n
        };
        let tangent = (prev_n + next_n).try_normalized().unwrap_or(next_n);
        let perp = Vec2::new(-tangent.y, tangent.x);
        let center = if i == 0 && !taper_start {
            points[i] - next_n * (half_width * 0.9)
        } else if i + 1 == points.len() && !taper_end {
            points[i] + prev_n * (half_width * 0.9)
        } else {
            points[i]
        };
        let local_half_width = if (i == 0 && taper_start) || (i + 1 == points.len() && taper_end) {
            half_width * 0.12
        } else {
            half_width
        };
        pairs.push((
            center - perp * local_half_width,
            center + perp * local_half_width,
        ));
    }

    pairs
}

fn render_vine_run_top<F>(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    points: &[Vec2<f32>],
    half_width: f32,
    desired_normal: Vec3<f32>,
    taper_start: bool,
    taper_end: bool,
    world_at: F,
) where
    F: Fn(Vec2<f32>) -> Vec3<f32>,
{
    let pairs2 = build_vine_ribbon_pairs(points, half_width, taper_start, taper_end);
    if pairs2.len() < 2 {
        return;
    }
    let mut vertices = Vec::with_capacity(pairs2.len() * 2);
    let mut uvs = Vec::with_capacity(pairs2.len() * 2);
    let mut indices = Vec::new();
    let pair_centers: Vec<Vec2<f32>> = pairs2
        .iter()
        .map(|(left, right)| (*left + *right) * 0.5)
        .collect();
    let mut acc = 0.0;
    for (i, (left, right)) in pairs2.iter().enumerate() {
        if i > 0 {
            let prev_center = pair_centers[i - 1];
            let center = pair_centers[i];
            acc += (center - prev_center).magnitude();
        }
        let l = world_at(*left);
        let r = world_at(*right);
        vertices.push([l.x, l.y, l.z, 1.0]);
        vertices.push([r.x, r.y, r.z, 1.0]);
        uvs.push([acc, 1.0]);
        uvs.push([acc, 0.0]);
    }
    for i in 0..pairs2.len().saturating_sub(1) {
        let a = i * 2;
        let b = a + 1;
        let c = a + 3;
        let d = a + 2;
        indices.push((a, d, c));
        indices.push((a, c, b));
    }

    push_organic_top_mesh(
        vmchunk,
        geo,
        tile_id,
        desired_normal,
        vertices,
        uvs,
        indices,
    );
}

fn emit_oriented_box(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    center: Vec3<f32>,
    right: Vec3<f32>,
    forward: Vec3<f32>,
    up: Vec3<f32>,
    half_right: f32,
    half_forward: f32,
    half_up: f32,
) {
    let p000 = center - right * half_right - forward * half_forward - up * half_up;
    let p001 = center - right * half_right - forward * half_forward + up * half_up;
    let p010 = center - right * half_right + forward * half_forward - up * half_up;
    let p011 = center - right * half_right + forward * half_forward + up * half_up;
    let p100 = center + right * half_right - forward * half_forward - up * half_up;
    let p101 = center + right * half_right - forward * half_forward + up * half_up;
    let p110 = center + right * half_right + forward * half_forward - up * half_up;
    let p111 = center + right * half_right + forward * half_forward + up * half_up;

    let push_face =
        |vmchunk: &mut scenevm::Chunk, a: Vec3<f32>, b: Vec3<f32>, c: Vec3<f32>, d: Vec3<f32>| {
            let vertices = vec![
                [a.x, a.y, a.z, 1.0],
                [b.x, b.y, b.z, 1.0],
                [c.x, c.y, c.z, 1.0],
                [d.x, d.y, d.z, 1.0],
            ];
            let uvs = vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
            let indices = vec![
                (0usize, 1usize, 2usize),
                (0usize, 2usize, 3usize),
                (2usize, 1usize, 0usize),
                (3usize, 2usize, 0usize),
            ];
            vmchunk.add_poly_3d(geo, tile_id, vertices, uvs, indices, 0, true);
        };

    push_face(vmchunk, p001, p101, p111, p011);
    push_face(vmchunk, p000, p010, p110, p100);
    push_face(vmchunk, p000, p001, p011, p010);
    push_face(vmchunk, p100, p110, p111, p101);
    push_face(vmchunk, p010, p011, p111, p110);
    push_face(vmchunk, p000, p100, p101, p001);
}

fn emit_growth_blob(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    center: Vec3<f32>,
    up: Vec3<f32>,
    base_right: Vec3<f32>,
    base_forward: Vec3<f32>,
    radius: f32,
    height: f32,
    layers: i32,
    taper: f32,
    breakup: f32,
) {
    let layer_count = layers.max(3) as usize;
    let voxel_size = (radius * 0.34).clamp(0.05, 0.18);
    let half = voxel_size * 0.5;
    let max_ring = ((radius / voxel_size).ceil() as i32).max(1);
    let breakup_seed = (center.x * 17.13 + center.y * 9.71 + center.z * 5.19).sin();

    for layer_index in 0..layer_count {
        let t = if layer_count <= 1 {
            0.0
        } else {
            layer_index as f32 / (layer_count - 1) as f32
        };
        let width_scale = (1.0 - t * taper * 0.85).max(0.22);
        let layer_radius = radius * width_scale;
        let layer_ring = ((layer_radius / voxel_size).ceil() as i32).clamp(1, max_ring);
        let layer_center = center + up * (height * t * 0.82);
        let edge_hole_bias = breakup.clamp(0.0, 1.0) * 0.55;

        for gx in -layer_ring..=layer_ring {
            for gy in -layer_ring..=layer_ring {
                let fx = gx as f32 / layer_ring as f32;
                let fy = gy as f32 / layer_ring as f32;
                let dist = (fx * fx + fy * fy).sqrt();
                if dist > 1.0 {
                    continue;
                }

                let noise = ((gx as f32 * 1.73)
                    + (gy as f32 * 2.37)
                    + (layer_index as f32 * 0.91)
                    + breakup_seed)
                    .sin()
                    * 0.5
                    + 0.5;
                if dist > 0.55 && noise < edge_hole_bias * dist {
                    continue;
                }

                let column_layers = if dist < 0.35 && layer_index + 1 < layer_count {
                    2
                } else {
                    1
                };
                for lift in 0..column_layers {
                    let c = layer_center
                        + base_right * (gx as f32 * voxel_size)
                        + base_forward * (gy as f32 * voxel_size)
                        + up * (lift as f32 * voxel_size * 0.72);
                    emit_oriented_box(
                        vmchunk,
                        geo,
                        tile_id,
                        c,
                        base_right,
                        base_forward,
                        up,
                        half,
                        half,
                        half,
                    );
                }
            }
        }
    }
}

fn render_bush_cluster(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    center: Vec3<f32>,
    axis: Vec3<f32>,
    right: Vec3<f32>,
    forward: Vec3<f32>,
    radius: f32,
    height: f32,
    layers: i32,
    taper: f32,
    breakup: f32,
    shape: OrganicGrowthShape,
    canopy_lobes: i32,
    canopy_spread: f32,
    trunk_height: f32,
    trunk_radius: f32,
) {
    let up = axis.normalized();
    let base_right = right.normalized();
    let base_forward = forward.normalized();
    match shape {
        OrganicGrowthShape::Bush => {
            emit_growth_blob(
                vmchunk,
                geo,
                tile_id,
                center,
                up,
                base_right,
                base_forward,
                radius,
                height,
                layers,
                taper,
                breakup,
            );
        }
        OrganicGrowthShape::Tree => {
            let trunk_h = trunk_height.max(height * 0.45).max(0.18);
            let trunk_r = trunk_radius.max(radius * 0.12).max(0.03);
            let trunk_segments = ((trunk_h / 0.12).ceil() as i32).clamp(2, 8);
            for i in 0..trunk_segments {
                let t = (i as f32 + 0.5) / trunk_segments as f32;
                let c = center + up * (trunk_h * t);
                let seg_scale = (1.0 - t * 0.22).max(0.72);
                emit_oriented_box(
                    vmchunk,
                    geo,
                    tile_id,
                    c,
                    base_right,
                    base_forward,
                    up,
                    trunk_r * seg_scale,
                    trunk_r * seg_scale,
                    (trunk_h / trunk_segments as f32) * 0.55,
                );
            }

            let crown_center = center + up * trunk_h;
            let crown_radius = (radius * (0.72 + canopy_spread * 0.32)).max(trunk_r * 2.5);
            let crown_height = (height - trunk_h).max(radius * 0.4);
            emit_growth_blob(
                vmchunk,
                geo,
                tile_id,
                crown_center,
                up,
                base_right,
                base_forward,
                crown_radius * 0.72,
                crown_height * 0.85,
                layers.max(3),
                taper * 0.65,
                breakup * 0.7,
            );

            let lobe_count = canopy_lobes.max(3) as usize;
            let ring = crown_radius * (0.22 + canopy_spread * 0.30);
            let lobe_radius = crown_radius * (0.34 - canopy_spread * 0.06).max(0.20);
            for i in 0..lobe_count {
                let a = i as f32 / lobe_count as f32 * std::f32::consts::TAU;
                let lobe_center = crown_center
                    + base_right * (a.cos() * ring)
                    + base_forward * (a.sin() * ring)
                    + up * ((a * 1.7).sin() * crown_height * 0.10 + crown_height * 0.08);
                emit_growth_blob(
                    vmchunk,
                    geo,
                    tile_id,
                    lobe_center,
                    up,
                    base_right,
                    base_forward,
                    lobe_radius,
                    crown_height * 0.52,
                    (layers - 1).max(3),
                    0.35,
                    breakup * 0.9,
                );
            }
        }
    }
}

fn generate_organic_volumes(
    map: &Map,
    assets: &Assets,
    chunk: &Chunk,
    vmchunk: &mut scenevm::Chunk,
) {
    let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();

    for surface in map.surfaces.values() {
        let Some(sector) = map.find_sector(surface.sector_id) else {
            continue;
        };
        let right = surface.frame.right.normalized();
        let normal = surface.normal();
        let forward = (-surface.frame.up).normalized();
        let host_source = sector.properties.get_default_source().cloned();
        let host_tile_id = host_source
            .as_ref()
            .and_then(|source| pixel_source_to_tile_id(source, assets));

        for (tile_id, run) in collect_vine_runs(&surface.organic_vine_strokes, |stroke| {
            stroke
                .source
                .as_ref()
                .and_then(|source| pixel_source_to_tile_id(source, assets))
                .or(host_tile_id)
                .unwrap_or(default_tile_id)
        }) {
            let first = run[0];
            let center_local = (first.start + first.end) * 0.5;
            let center_world = surface.uv_to_world(surface.tile_local_to_uv(center_local, map));
            if !chunk
                .bbox
                .contains(Vec2::new(center_world.x, center_world.z))
            {
                continue;
            }

            let points: Vec<Vec2<f32>> = std::iter::once(run[0].start)
                .chain(run.iter().map(|seg| seg.end))
                .collect();
            let sign = if first.grow_positive { 1.0 } else { -1.0 };
            let outer = if first.grow_positive {
                first.anchor_offset + first.depth
            } else {
                first.anchor_offset - first.depth
            };
            let world_at = |p: Vec2<f32>| -> Vec3<f32> {
                surface.uv_to_world(surface.tile_local_to_uv(p, map)) + normal * outer
            };
            render_vine_run_top(
                vmchunk,
                GeoId::Sector(surface.sector_id),
                tile_id,
                &points,
                first.width * 0.5,
                normal * sign,
                first.cap_start,
                run[run.len() - 1].cap_end,
                world_at,
            );
        }

        for bush in &surface.organic_bush_clusters {
            let tile_id = bush
                .source
                .as_ref()
                .and_then(|source| pixel_source_to_tile_id(source, assets))
                .or(host_tile_id)
                .unwrap_or(default_tile_id);
            let sign = if bush.grow_positive { 1.0 } else { -1.0 };
            let center_world = surface.uv_to_world(surface.tile_local_to_uv(bush.center, map))
                + normal * bush.anchor_offset;
            if !chunk
                .bbox
                .contains(Vec2::new(center_world.x, center_world.z))
            {
                continue;
            }
            render_bush_cluster(
                vmchunk,
                GeoId::Sector(surface.sector_id),
                tile_id,
                center_world,
                normal * sign,
                right,
                forward,
                bush.radius,
                bush.height,
                bush.layers,
                bush.taper,
                bush.breakup,
                bush.shape.clone(),
                bush.canopy_lobes,
                bush.canopy_spread,
                bush.trunk_height,
                bush.trunk_radius,
            );
        }

        for layer in surface.organic_layers.values() {
            let spans = collect_organic_render_spans(&layer.columns, |span| {
                let source = span
                    .source
                    .as_ref()
                    .or_else(|| {
                        layer
                            .binding_for_channel(span.channel)
                            .and_then(|b| b.source.as_ref())
                    })
                    .or(host_source.as_ref());
                source
                    .and_then(|source| pixel_source_to_tile_id(source, assets))
                    .or(host_tile_id)
                    .unwrap_or(default_tile_id)
            });
            let mut span_lookup: FxHashMap<(i32, i32), Vec<OrganicRenderSpan>> =
                FxHashMap::default();
            for span in &spans {
                span_lookup.entry((span.x, span.y)).or_default().push(*span);
            }
            let corner_lookup = build_organic_corner_lookup(&spans);
            let ribbon_runs = collect_ribbon_runs(&spans, &span_lookup);
            let ribbon_keys: FxHashSet<(i32, i32, i32)> = ribbon_runs
                .iter()
                .flat_map(|(_, run)| run.iter().map(|s| (s.x, s.y, s.outer_sign() as i32)))
                .collect();

            for sign in [-1_i32, 1_i32] {
                let group_spans: Vec<OrganicRenderSpan> = spans
                    .iter()
                    .copied()
                    .filter(|span| span.outer_sign() as i32 == sign)
                    .collect();
                if group_spans.is_empty() {
                    continue;
                }

                let filtered_group_spans: Vec<OrganicRenderSpan> = group_spans
                    .iter()
                    .copied()
                    .filter(|span| {
                        let inner = span.inner();
                        let outer = span.outer();
                        let north =
                            organic_overlap_top(&span_lookup, span.x, span.y - 1, inner, outer)
                                .is_some();
                        let east =
                            organic_overlap_top(&span_lookup, span.x + 1, span.y, inner, outer)
                                .is_some();
                        let south =
                            organic_overlap_top(&span_lookup, span.x, span.y + 1, inner, outer)
                                .is_some();
                        let west =
                            organic_overlap_top(&span_lookup, span.x - 1, span.y, inner, outer)
                                .is_some();
                        !is_ribbon_span(north, east, south, west)
                    })
                    .collect();
                if filtered_group_spans.is_empty() {
                    continue;
                }
                let mut top_groups: FxHashMap<Uuid, Vec<OrganicRenderSpan>> = FxHashMap::default();
                for span in &filtered_group_spans {
                    top_groups.entry(span.tile_id).or_default().push(*span);
                }
                for (tile_id, tile_spans) in top_groups {
                    let mut vertex_map: FxHashMap<(i32, i32), usize> = FxHashMap::default();
                    let mut vertices = Vec::new();
                    let mut uvs = Vec::new();
                    let mut indices = Vec::new();

                    let mut ensure_vertex = |cx: i32, cy: i32| -> usize {
                        if let Some(index) = vertex_map.get(&(cx, cy)) {
                            return *index;
                        }
                        let extrude = corner_lookup.get(&(cx, cy)).copied().unwrap_or(0.0);
                        let lx = cx as f32 * layer.cell_size;
                        let ly = cy as f32 * layer.cell_size;
                        let world = surface
                            .uv_to_world(surface.tile_local_to_uv(Vec2::new(lx, ly), map))
                            + normal * extrude;
                        let index = vertices.len();
                        vertices.push([world.x, world.y, world.z, 1.0]);
                        uvs.push([lx, ly]);
                        vertex_map.insert((cx, cy), index);
                        index
                    };

                    for span in &tile_spans {
                        let v00 = ensure_vertex(span.x, span.y);
                        let v10 = ensure_vertex(span.x + 1, span.y);
                        let v11 = ensure_vertex(span.x + 1, span.y + 1);
                        let v01 = ensure_vertex(span.x, span.y + 1);
                        indices.push((v00, v10, v11));
                        indices.push((v00, v11, v01));
                    }

                    let center_span = tile_spans[tile_spans.len() / 2];
                    let center_local = Vec2::new(
                        (center_span.x as f32 + 0.5) * layer.cell_size,
                        (center_span.y as f32 + 0.5) * layer.cell_size,
                    );
                    let center_world =
                        surface.uv_to_world(surface.tile_local_to_uv(center_local, map));
                    if chunk
                        .bbox
                        .contains(Vec2::new(center_world.x, center_world.z))
                    {
                        push_organic_top_mesh(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            tile_id,
                            normal * sign as f32,
                            vertices,
                            uvs,
                            indices,
                        );
                    }
                }
            }

            for (axis, run) in &ribbon_runs {
                let sign = run[0].outer_sign();
                let inset = (layer.cell_size * 0.28).min(layer.cell_size * 0.45);
                let mut vertices = Vec::new();
                let mut uvs = Vec::new();
                let mut indices = Vec::new();

                match axis {
                    RibbonAxis::Horizontal => {
                        let y = run[0].y;
                        let (_, _, shell_y0, shell_y1) = ribbon_bounds(
                            run.first().unwrap().x as f32 * layer.cell_size,
                            (run.last().unwrap().x + 1) as f32 * layer.cell_size,
                            y as f32 * layer.cell_size,
                            (y + 1) as f32 * layer.cell_size,
                            inset,
                            false,
                            true,
                            false,
                            true,
                        );
                        let shell_x0 = run.first().unwrap().x as f32 * layer.cell_size + inset;
                        let shell_x1 = (run.last().unwrap().x + 1) as f32 * layer.cell_size - inset;
                        let world_at = |lx: f32, ly: f32, extrude: f32| -> Vec3<f32> {
                            let base = surface
                                .uv_to_world(surface.tile_local_to_uv(Vec2::new(lx, ly), map));
                            base + normal * extrude
                        };
                        for (i, x) in
                            (run.first().unwrap().x..=run.last().unwrap().x + 1).enumerate()
                        {
                            let lx = if x == run.first().unwrap().x {
                                shell_x0
                            } else if x == run.last().unwrap().x + 1 {
                                shell_x1
                            } else {
                                x as f32 * layer.cell_size
                            };
                            let north_top = run
                                .iter()
                                .find(|s| s.x == x || s.x + 1 == x)
                                .map(|span| {
                                    let clamp_outer = |v: f32| {
                                        if sign >= 0.0 {
                                            v.clamp(span.inner(), span.outer())
                                        } else {
                                            v.clamp(span.outer(), span.inner())
                                        }
                                    };
                                    (
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x, y))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x, y + 1))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                    )
                                })
                                .unwrap_or((run[0].outer(), run[0].outer()));
                            vertices.push([
                                world_at(lx, shell_y0, north_top.0).x,
                                world_at(lx, shell_y0, north_top.0).y,
                                world_at(lx, shell_y0, north_top.0).z,
                                1.0,
                            ]);
                            vertices.push([
                                world_at(lx, shell_y1, north_top.1).x,
                                world_at(lx, shell_y1, north_top.1).y,
                                world_at(lx, shell_y1, north_top.1).z,
                                1.0,
                            ]);
                            uvs.push([i as f32, 1.0]);
                            uvs.push([i as f32, 0.0]);
                        }
                    }
                    RibbonAxis::Vertical => {
                        let x = run[0].x;
                        let (shell_x0, shell_x1, _, _) = ribbon_bounds(
                            x as f32 * layer.cell_size,
                            (x + 1) as f32 * layer.cell_size,
                            run.first().unwrap().y as f32 * layer.cell_size,
                            (run.last().unwrap().y + 1) as f32 * layer.cell_size,
                            inset,
                            true,
                            false,
                            true,
                            false,
                        );
                        let shell_y0 = run.first().unwrap().y as f32 * layer.cell_size + inset;
                        let shell_y1 = (run.last().unwrap().y + 1) as f32 * layer.cell_size - inset;
                        let world_at = |lx: f32, ly: f32, extrude: f32| -> Vec3<f32> {
                            let base = surface
                                .uv_to_world(surface.tile_local_to_uv(Vec2::new(lx, ly), map));
                            base + normal * extrude
                        };
                        for (i, y) in
                            (run.first().unwrap().y..=run.last().unwrap().y + 1).enumerate()
                        {
                            let ly = if y == run.first().unwrap().y {
                                shell_y0
                            } else if y == run.last().unwrap().y + 1 {
                                shell_y1
                            } else {
                                y as f32 * layer.cell_size
                            };
                            let west_east = run
                                .iter()
                                .find(|s| s.y == y || s.y + 1 == y)
                                .map(|span| {
                                    let clamp_outer = |v: f32| {
                                        if sign >= 0.0 {
                                            v.clamp(span.inner(), span.outer())
                                        } else {
                                            v.clamp(span.outer(), span.inner())
                                        }
                                    };
                                    (
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x, y))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x + 1, y))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                    )
                                })
                                .unwrap_or((run[0].outer(), run[0].outer()));
                            vertices.push([
                                world_at(shell_x0, ly, west_east.0).x,
                                world_at(shell_x0, ly, west_east.0).y,
                                world_at(shell_x0, ly, west_east.0).z,
                                1.0,
                            ]);
                            vertices.push([
                                world_at(shell_x1, ly, west_east.1).x,
                                world_at(shell_x1, ly, west_east.1).y,
                                world_at(shell_x1, ly, west_east.1).z,
                                1.0,
                            ]);
                            uvs.push([0.0, i as f32]);
                            uvs.push([1.0, i as f32]);
                        }
                    }
                }

                for i in 0..(vertices.len() / 2).saturating_sub(1) {
                    let a = i * 2;
                    let b = a + 1;
                    let c = a + 3;
                    let d = a + 2;
                    indices.push((a, d, c));
                    indices.push((a, c, b));
                }
                push_organic_top_mesh(
                    vmchunk,
                    GeoId::Sector(surface.sector_id),
                    run[0].tile_id,
                    normal * sign,
                    vertices,
                    uvs,
                    indices,
                );

                let world_at = |lx: f32, ly: f32, extrude: f32| -> Vec3<f32> {
                    let base =
                        surface.uv_to_world(surface.tile_local_to_uv(Vec2::new(lx, ly), map));
                    base + normal * extrude
                };
                match axis {
                    RibbonAxis::Horizontal => {
                        let y = run[0].y;
                        let shell_y0 = y as f32 * layer.cell_size;
                        let shell_y1 = (y + 1) as f32 * layer.cell_size;
                        let shell_x0 = run.first().unwrap().x as f32 * layer.cell_size + inset;
                        let shell_x1 = (run.last().unwrap().x + 1) as f32 * layer.cell_size - inset;
                        let mut north_pairs = Vec::new();
                        let mut south_pairs = Vec::new();
                        for x in run.first().unwrap().x..=run.last().unwrap().x + 1 {
                            let lx = if x == run.first().unwrap().x {
                                shell_x0
                            } else if x == run.last().unwrap().x + 1 {
                                shell_x1
                            } else {
                                x as f32 * layer.cell_size
                            };
                            let span_ref = run
                                .iter()
                                .find(|s| s.x == x || s.x + 1 == x)
                                .unwrap_or(&run[0]);
                            let clamp_outer = |v: f32| {
                                if sign >= 0.0 {
                                    v.clamp(span_ref.inner(), span_ref.outer())
                                } else {
                                    v.clamp(span_ref.outer(), span_ref.inner())
                                }
                            };
                            let nw_top = clamp_outer(
                                corner_lookup
                                    .get(&(x, y))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let sw_top = clamp_outer(
                                corner_lookup
                                    .get(&(x, y + 1))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let north_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x,
                                span_ref.y - 1,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            let south_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x,
                                span_ref.y + 1,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            north_pairs.push((
                                world_at(lx, shell_y0, north_bottom),
                                world_at(lx, shell_y0, nw_top),
                            ));
                            south_pairs.push((
                                world_at(lx, shell_y1, south_bottom),
                                world_at(lx, shell_y1, sw_top),
                            ));
                        }
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            -forward * sign,
                            &north_pairs,
                            true,
                        );
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            forward * sign,
                            &south_pairs,
                            true,
                        );
                        let start_span = run.first().unwrap();
                        let end_span = run.last().unwrap();
                        let start_quad = [
                            world_at(shell_x0, shell_y1, start_span.inner()),
                            world_at(shell_x0, shell_y0, start_span.inner()),
                            world_at(
                                shell_x0,
                                shell_y0,
                                corner_lookup
                                    .get(&(start_span.x, y))
                                    .copied()
                                    .unwrap_or(start_span.outer()),
                            ),
                            world_at(
                                shell_x0,
                                shell_y1,
                                corner_lookup
                                    .get(&(start_span.x, y + 1))
                                    .copied()
                                    .unwrap_or(start_span.outer()),
                            ),
                        ];
                        let end_x = run.last().unwrap().x + 1;
                        let end_quad = [
                            world_at(shell_x1, shell_y0, end_span.inner()),
                            world_at(shell_x1, shell_y1, end_span.inner()),
                            world_at(
                                shell_x1,
                                shell_y1,
                                corner_lookup
                                    .get(&(end_x, y + 1))
                                    .copied()
                                    .unwrap_or(end_span.outer()),
                            ),
                            world_at(
                                shell_x1,
                                shell_y0,
                                corner_lookup
                                    .get(&(end_x, y))
                                    .copied()
                                    .unwrap_or(end_span.outer()),
                            ),
                        ];
                        push_quad_with_winding(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            start_quad.iter().map(|v| [v.x, v.y, v.z, 1.0]).collect(),
                            vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                            -right * sign,
                        );
                        push_quad_with_winding(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            end_quad.iter().map(|v| [v.x, v.y, v.z, 1.0]).collect(),
                            vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                            right * sign,
                        );
                    }
                    RibbonAxis::Vertical => {
                        let x = run[0].x;
                        let shell_x0 = x as f32 * layer.cell_size;
                        let shell_x1 = (x + 1) as f32 * layer.cell_size;
                        let shell_y0 = run.first().unwrap().y as f32 * layer.cell_size + inset;
                        let shell_y1 = (run.last().unwrap().y + 1) as f32 * layer.cell_size - inset;
                        let mut west_pairs = Vec::new();
                        let mut east_pairs = Vec::new();
                        for y in run.first().unwrap().y..=run.last().unwrap().y + 1 {
                            let ly = if y == run.first().unwrap().y {
                                shell_y0
                            } else if y == run.last().unwrap().y + 1 {
                                shell_y1
                            } else {
                                y as f32 * layer.cell_size
                            };
                            let span_ref = run
                                .iter()
                                .find(|s| s.y == y || s.y + 1 == y)
                                .unwrap_or(&run[0]);
                            let clamp_outer = |v: f32| {
                                if sign >= 0.0 {
                                    v.clamp(span_ref.inner(), span_ref.outer())
                                } else {
                                    v.clamp(span_ref.outer(), span_ref.inner())
                                }
                            };
                            let nw_top = clamp_outer(
                                corner_lookup
                                    .get(&(x, y))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let ne_top = clamp_outer(
                                corner_lookup
                                    .get(&(x + 1, y))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let west_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x - 1,
                                span_ref.y,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            let east_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x + 1,
                                span_ref.y,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            west_pairs.push((
                                world_at(shell_x0, ly, west_bottom),
                                world_at(shell_x0, ly, nw_top),
                            ));
                            east_pairs.push((
                                world_at(shell_x1, ly, east_bottom),
                                world_at(shell_x1, ly, ne_top),
                            ));
                        }
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            -right * sign,
                            &west_pairs,
                            false,
                        );
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            right * sign,
                            &east_pairs,
                            false,
                        );
                        let start_span = run.first().unwrap();
                        let end_span = run.last().unwrap();
                        let start_quad = [
                            world_at(shell_x0, shell_y0, start_span.inner()),
                            world_at(shell_x1, shell_y0, start_span.inner()),
                            world_at(
                                shell_x1,
                                shell_y0,
                                corner_lookup
                                    .get(&(x + 1, start_span.y))
                                    .copied()
                                    .unwrap_or(start_span.outer()),
                            ),
                            world_at(
                                shell_x0,
                                shell_y0,
                                corner_lookup
                                    .get(&(x, start_span.y))
                                    .copied()
                                    .unwrap_or(start_span.outer()),
                            ),
                        ];
                        let end_y = run.last().unwrap().y + 1;
                        let end_quad = [
                            world_at(shell_x1, shell_y1, end_span.inner()),
                            world_at(shell_x0, shell_y1, end_span.inner()),
                            world_at(
                                shell_x0,
                                shell_y1,
                                corner_lookup
                                    .get(&(x, end_y))
                                    .copied()
                                    .unwrap_or(end_span.outer()),
                            ),
                            world_at(
                                shell_x1,
                                shell_y1,
                                corner_lookup
                                    .get(&(x + 1, end_y))
                                    .copied()
                                    .unwrap_or(end_span.outer()),
                            ),
                        ];
                        push_quad_with_winding(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            start_quad.iter().map(|v| [v.x, v.y, v.z, 1.0]).collect(),
                            vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                            -forward * sign,
                        );
                        push_quad_with_winding(
                            vmchunk,
                            GeoId::Sector(surface.sector_id),
                            run[0].tile_id,
                            end_quad.iter().map(|v| [v.x, v.y, v.z, 1.0]).collect(),
                            vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
                            forward * sign,
                        );
                    }
                }
            }

            for span in spans {
                if ribbon_keys.contains(&(span.x, span.y, span.outer_sign() as i32)) {
                    continue;
                }
                let local = Vec2::new(
                    (span.x as f32 + 0.5) * layer.cell_size,
                    (span.y as f32 + 0.5) * layer.cell_size,
                );
                let surface_center = surface.uv_to_world(surface.tile_local_to_uv(local, map));
                if !chunk
                    .bbox
                    .contains(Vec2::new(surface_center.x, surface_center.z))
                {
                    continue;
                }
                let outer = span.outer();
                let inner = span.inner();
                let sign = span.outer_sign();
                let clamp_outer = |v: f32| {
                    if sign >= 0.0 {
                        v.clamp(inner, outer)
                    } else {
                        v.clamp(outer, inner)
                    }
                };
                let nw_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x, span.y))
                        .copied()
                        .unwrap_or(outer),
                );
                let ne_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x + 1, span.y))
                        .copied()
                        .unwrap_or(outer),
                );
                let se_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x + 1, span.y + 1))
                        .copied()
                        .unwrap_or(outer),
                );
                let sw_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x, span.y + 1))
                        .copied()
                        .unwrap_or(outer),
                );
                let north_bottom =
                    organic_overlap_top(&span_lookup, span.x, span.y - 1, inner, outer)
                        .unwrap_or(inner);
                let north_connected =
                    organic_overlap_top(&span_lookup, span.x, span.y - 1, inner, outer).is_some();
                let east_bottom =
                    organic_overlap_top(&span_lookup, span.x + 1, span.y, inner, outer)
                        .unwrap_or(inner);
                let east_connected =
                    organic_overlap_top(&span_lookup, span.x + 1, span.y, inner, outer).is_some();
                let south_bottom =
                    organic_overlap_top(&span_lookup, span.x, span.y + 1, inner, outer)
                        .unwrap_or(inner);
                let south_connected =
                    organic_overlap_top(&span_lookup, span.x, span.y + 1, inner, outer).is_some();
                let west_bottom =
                    organic_overlap_top(&span_lookup, span.x - 1, span.y, inner, outer)
                        .unwrap_or(inner);
                let west_connected =
                    organic_overlap_top(&span_lookup, span.x - 1, span.y, inner, outer).is_some();

                let x0 = span.x as f32 * layer.cell_size;
                let x1 = (span.x + 1) as f32 * layer.cell_size;
                let y0 = span.y as f32 * layer.cell_size;
                let y1 = (span.y + 1) as f32 * layer.cell_size;
                let world_at = |lx: f32, ly: f32, extrude: f32| -> Vec3<f32> {
                    let base =
                        surface.uv_to_world(surface.tile_local_to_uv(Vec2::new(lx, ly), map));
                    base + normal * extrude
                };
                let ribbon = is_ribbon_span(
                    north_connected,
                    east_connected,
                    south_connected,
                    west_connected,
                );
                let mut shell_x0 = x0;
                let mut shell_x1 = x1;
                let mut shell_y0 = y0;
                let mut shell_y1 = y1;
                if ribbon {
                    let inset = (layer.cell_size * 0.28).min(layer.cell_size * 0.45);
                    (shell_x0, shell_x1, shell_y0, shell_y1) = ribbon_bounds(
                        x0,
                        x1,
                        y0,
                        y1,
                        inset,
                        north_connected,
                        east_connected,
                        south_connected,
                        west_connected,
                    );
                }
                render_organic_shell(
                    vmchunk,
                    GeoId::Sector(surface.sector_id),
                    span.tile_id,
                    [-forward * sign, right * sign, forward * sign, -right * sign],
                    [
                        [
                            world_at(shell_x0, shell_y0, north_bottom),
                            world_at(shell_x1, shell_y0, north_bottom),
                            world_at(shell_x1, shell_y0, ne_top),
                            world_at(shell_x0, shell_y0, nw_top),
                        ],
                        [
                            world_at(shell_x1, shell_y0, east_bottom),
                            world_at(shell_x1, shell_y1, east_bottom),
                            world_at(shell_x1, shell_y1, se_top),
                            world_at(shell_x1, shell_y0, ne_top),
                        ],
                        [
                            world_at(shell_x1, shell_y1, south_bottom),
                            world_at(shell_x0, shell_y1, south_bottom),
                            world_at(shell_x0, shell_y1, sw_top),
                            world_at(shell_x1, shell_y1, se_top),
                        ],
                        [
                            world_at(shell_x0, shell_y1, west_bottom),
                            world_at(shell_x0, shell_y0, west_bottom),
                            world_at(shell_x0, shell_y0, nw_top),
                            world_at(shell_x0, shell_y1, sw_top),
                        ],
                    ],
                );
            }
        }
    }

    for terrain_chunk in map.terrain.chunks.values() {
        for (tile_id, run) in collect_vine_runs(&terrain_chunk.organic_vine_strokes, |stroke| {
            stroke
                .source
                .as_ref()
                .and_then(|source| pixel_source_to_tile_id(source, assets))
                .unwrap_or(default_tile_id)
        }) {
            let first = run[0];
            let center = (first.start + first.end) * 0.5 + terrain_chunk.origin.map(|v| v as f32);
            if !chunk.bbox.contains(center) {
                continue;
            }

            let points: Vec<Vec2<f32>> = std::iter::once(run[0].start)
                .chain(run.iter().map(|seg| seg.end))
                .collect();
            let sign = if first.grow_positive { 1.0 } else { -1.0 };
            let outer = if first.grow_positive {
                first.anchor_offset + first.depth
            } else {
                first.anchor_offset - first.depth
            };
            let world_at = |p: Vec2<f32>| -> Vec3<f32> {
                let wp = p + terrain_chunk.origin.map(|v| v as f32);
                let h = TerrainGenerator::sample_height_at(map, wp, &TerrainConfig::default());
                Vec3::new(wp.x, h + outer, wp.y)
            };
            render_vine_run_top(
                vmchunk,
                GeoId::Terrain(center.x.floor() as i32, center.y.floor() as i32),
                tile_id,
                &points,
                first.width * 0.5,
                Vec3::unit_y() * sign,
                first.cap_start,
                run[run.len() - 1].cap_end,
                world_at,
            );
        }
        for bush in &terrain_chunk.organic_bush_clusters {
            let tile_id = bush
                .source
                .as_ref()
                .and_then(|source| pixel_source_to_tile_id(source, assets))
                .unwrap_or(default_tile_id);
            let center_world = Vec3::new(
                terrain_chunk.origin.x as f32 + bush.center.x,
                TerrainGenerator::sample_height_at(
                    map,
                    Vec2::new(
                        terrain_chunk.origin.x as f32 + bush.center.x,
                        terrain_chunk.origin.y as f32 + bush.center.y,
                    ),
                    &TerrainConfig::default(),
                ) + bush.anchor_offset,
                terrain_chunk.origin.y as f32 + bush.center.y,
            );
            if !chunk
                .bbox
                .contains(Vec2::new(center_world.x, center_world.z))
            {
                continue;
            }
            render_bush_cluster(
                vmchunk,
                GeoId::Terrain(center_world.x.floor() as i32, center_world.z.floor() as i32),
                tile_id,
                center_world,
                Vec3::unit_y(),
                Vec3::unit_x(),
                Vec3::unit_z(),
                bush.radius,
                bush.height,
                bush.layers,
                bush.taper,
                bush.breakup,
                bush.shape.clone(),
                bush.canopy_lobes,
                bush.canopy_spread,
                bush.trunk_height,
                bush.trunk_radius,
            );
        }
        for layer in terrain_chunk.organic_layers.values() {
            let spans = collect_organic_render_spans(&layer.columns, |span| {
                let source = span.source.as_ref().or_else(|| {
                    layer
                        .binding_for_channel(span.channel)
                        .and_then(|b| b.source.as_ref())
                });
                source
                    .and_then(|source| pixel_source_to_tile_id(source, assets))
                    .unwrap_or(default_tile_id)
            });
            let mut span_lookup: FxHashMap<(i32, i32), Vec<OrganicRenderSpan>> =
                FxHashMap::default();
            for span in &spans {
                span_lookup.entry((span.x, span.y)).or_default().push(*span);
            }
            let corner_lookup = build_organic_corner_lookup(&spans);
            let ribbon_runs = collect_ribbon_runs(&spans, &span_lookup);
            let ribbon_keys: FxHashSet<(i32, i32, i32)> = ribbon_runs
                .iter()
                .flat_map(|(_, run)| run.iter().map(|s| (s.x, s.y, s.outer_sign() as i32)))
                .collect();

            for sign in [-1_i32, 1_i32] {
                let group_spans: Vec<OrganicRenderSpan> = spans
                    .iter()
                    .copied()
                    .filter(|span| span.outer_sign() as i32 == sign)
                    .collect();
                if group_spans.is_empty() {
                    continue;
                }

                let filtered_group_spans: Vec<OrganicRenderSpan> = group_spans
                    .iter()
                    .copied()
                    .filter(|span| {
                        let inner = span.inner();
                        let outer = span.outer();
                        let north =
                            organic_overlap_top(&span_lookup, span.x, span.y - 1, inner, outer)
                                .is_some();
                        let east =
                            organic_overlap_top(&span_lookup, span.x + 1, span.y, inner, outer)
                                .is_some();
                        let south =
                            organic_overlap_top(&span_lookup, span.x, span.y + 1, inner, outer)
                                .is_some();
                        let west =
                            organic_overlap_top(&span_lookup, span.x - 1, span.y, inner, outer)
                                .is_some();
                        !is_ribbon_span(north, east, south, west)
                    })
                    .collect();
                if filtered_group_spans.is_empty() {
                    continue;
                }
                let mut top_groups: FxHashMap<Uuid, Vec<OrganicRenderSpan>> = FxHashMap::default();
                for span in &filtered_group_spans {
                    top_groups.entry(span.tile_id).or_default().push(*span);
                }
                for (tile_id, tile_spans) in top_groups {
                    let mut vertex_map: FxHashMap<(i32, i32), usize> = FxHashMap::default();
                    let mut vertices = Vec::new();
                    let mut uvs = Vec::new();
                    let mut indices = Vec::new();

                    let mut ensure_vertex = |cx: i32, cy: i32| -> usize {
                        if let Some(index) = vertex_map.get(&(cx, cy)) {
                            return *index;
                        }
                        let extrude = corner_lookup.get(&(cx, cy)).copied().unwrap_or(0.0);
                        let wx = terrain_chunk.origin.x as f32 + cx as f32 * layer.cell_size;
                        let wz = terrain_chunk.origin.y as f32 + cy as f32 * layer.cell_size;
                        let base = TerrainGenerator::sample_height_at(
                            map,
                            Vec2::new(wx, wz),
                            &TerrainConfig::default(),
                        );
                        let index = vertices.len();
                        vertices.push([wx, base + extrude, wz, 1.0]);
                        uvs.push([wx, wz]);
                        vertex_map.insert((cx, cy), index);
                        index
                    };

                    for span in &tile_spans {
                        let v00 = ensure_vertex(span.x, span.y);
                        let v10 = ensure_vertex(span.x + 1, span.y);
                        let v11 = ensure_vertex(span.x + 1, span.y + 1);
                        let v01 = ensure_vertex(span.x, span.y + 1);
                        indices.push((v00, v10, v11));
                        indices.push((v00, v11, v01));
                    }

                    let center_span = tile_spans[tile_spans.len() / 2];
                    let center_x = terrain_chunk.origin.x as f32
                        + (center_span.x as f32 + 0.5) * layer.cell_size;
                    let center_z = terrain_chunk.origin.y as f32
                        + (center_span.y as f32 + 0.5) * layer.cell_size;
                    if chunk.bbox.contains(Vec2::new(center_x, center_z)) {
                        push_organic_top_mesh(
                            vmchunk,
                            GeoId::Terrain(center_x.floor() as i32, center_z.floor() as i32),
                            tile_id,
                            Vec3::unit_y() * sign as f32,
                            vertices,
                            uvs,
                            indices,
                        );
                    }
                }
            }

            for (axis, run) in &ribbon_runs {
                let sign = run[0].outer_sign();
                let inset = (layer.cell_size * 0.28).min(layer.cell_size * 0.45);
                let mut vertices = Vec::new();
                let mut uvs = Vec::new();
                let mut indices = Vec::new();

                match axis {
                    RibbonAxis::Horizontal => {
                        let y = run[0].y;
                        let shell_z0 = terrain_chunk.origin.y as f32 + y as f32 * layer.cell_size;
                        let shell_z1 =
                            terrain_chunk.origin.y as f32 + (y + 1) as f32 * layer.cell_size;
                        let shell_x0 = terrain_chunk.origin.x as f32
                            + run.first().unwrap().x as f32 * layer.cell_size
                            + inset;
                        let shell_x1 = terrain_chunk.origin.x as f32
                            + (run.last().unwrap().x + 1) as f32 * layer.cell_size
                            - inset;
                        let world_at = |wx: f32, wz: f32, extrude: f32| -> Vec3<f32> {
                            let base = TerrainGenerator::sample_height_at(
                                map,
                                Vec2::new(wx, wz),
                                &TerrainConfig::default(),
                            );
                            Vec3::new(wx, base + extrude, wz)
                        };
                        for (i, x) in
                            (run.first().unwrap().x..=run.last().unwrap().x + 1).enumerate()
                        {
                            let wx = if x == run.first().unwrap().x {
                                shell_x0
                            } else if x == run.last().unwrap().x + 1 {
                                shell_x1
                            } else {
                                terrain_chunk.origin.x as f32 + x as f32 * layer.cell_size
                            };
                            let north_south = run
                                .iter()
                                .find(|s| s.x == x || s.x + 1 == x)
                                .map(|span| {
                                    let clamp_outer = |v: f32| {
                                        if sign >= 0.0 {
                                            v.clamp(span.inner(), span.outer())
                                        } else {
                                            v.clamp(span.outer(), span.inner())
                                        }
                                    };
                                    (
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x, y))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x, y + 1))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                    )
                                })
                                .unwrap_or((run[0].outer(), run[0].outer()));
                            vertices.push([
                                world_at(wx, shell_z0, north_south.0).x,
                                world_at(wx, shell_z0, north_south.0).y,
                                world_at(wx, shell_z0, north_south.0).z,
                                1.0,
                            ]);
                            vertices.push([
                                world_at(wx, shell_z1, north_south.1).x,
                                world_at(wx, shell_z1, north_south.1).y,
                                world_at(wx, shell_z1, north_south.1).z,
                                1.0,
                            ]);
                            uvs.push([i as f32, 1.0]);
                            uvs.push([i as f32, 0.0]);
                        }
                    }
                    RibbonAxis::Vertical => {
                        let x = run[0].x;
                        let shell_x0 = terrain_chunk.origin.x as f32 + x as f32 * layer.cell_size;
                        let shell_x1 =
                            terrain_chunk.origin.x as f32 + (x + 1) as f32 * layer.cell_size;
                        let shell_z0 = terrain_chunk.origin.y as f32
                            + run.first().unwrap().y as f32 * layer.cell_size
                            + inset;
                        let shell_z1 = terrain_chunk.origin.y as f32
                            + (run.last().unwrap().y + 1) as f32 * layer.cell_size
                            - inset;
                        let world_at = |wx: f32, wz: f32, extrude: f32| -> Vec3<f32> {
                            let base = TerrainGenerator::sample_height_at(
                                map,
                                Vec2::new(wx, wz),
                                &TerrainConfig::default(),
                            );
                            Vec3::new(wx, base + extrude, wz)
                        };
                        for (i, y) in
                            (run.first().unwrap().y..=run.last().unwrap().y + 1).enumerate()
                        {
                            let wz = if y == run.first().unwrap().y {
                                shell_z0
                            } else if y == run.last().unwrap().y + 1 {
                                shell_z1
                            } else {
                                terrain_chunk.origin.y as f32 + y as f32 * layer.cell_size
                            };
                            let west_east = run
                                .iter()
                                .find(|s| s.y == y || s.y + 1 == y)
                                .map(|span| {
                                    let clamp_outer = |v: f32| {
                                        if sign >= 0.0 {
                                            v.clamp(span.inner(), span.outer())
                                        } else {
                                            v.clamp(span.outer(), span.inner())
                                        }
                                    };
                                    (
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x, y))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                        clamp_outer(
                                            corner_lookup
                                                .get(&(x + 1, y))
                                                .copied()
                                                .unwrap_or(span.outer()),
                                        ),
                                    )
                                })
                                .unwrap_or((run[0].outer(), run[0].outer()));
                            vertices.push([
                                world_at(shell_x0, wz, west_east.0).x,
                                world_at(shell_x0, wz, west_east.0).y,
                                world_at(shell_x0, wz, west_east.0).z,
                                1.0,
                            ]);
                            vertices.push([
                                world_at(shell_x1, wz, west_east.1).x,
                                world_at(shell_x1, wz, west_east.1).y,
                                world_at(shell_x1, wz, west_east.1).z,
                                1.0,
                            ]);
                            uvs.push([0.0, i as f32]);
                            uvs.push([1.0, i as f32]);
                        }
                    }
                }

                for i in 0..(vertices.len() / 2).saturating_sub(1) {
                    let a = i * 2;
                    let b = a + 1;
                    let c = a + 3;
                    let d = a + 2;
                    indices.push((a, d, c));
                    indices.push((a, c, b));
                }
                push_organic_top_mesh(
                    vmchunk,
                    GeoId::Terrain(
                        terrain_chunk.origin.x + run[0].x,
                        terrain_chunk.origin.y + run[0].y,
                    ),
                    run[0].tile_id,
                    Vec3::unit_y() * sign,
                    vertices,
                    uvs,
                    indices,
                );

                let world_at = |wx: f32, wz: f32, extrude: f32| -> Vec3<f32> {
                    let base = TerrainGenerator::sample_height_at(
                        map,
                        Vec2::new(wx, wz),
                        &TerrainConfig::default(),
                    );
                    Vec3::new(wx, base + extrude, wz)
                };
                match axis {
                    RibbonAxis::Horizontal => {
                        let y = run[0].y;
                        let shell_z0 = terrain_chunk.origin.y as f32 + y as f32 * layer.cell_size;
                        let shell_z1 =
                            terrain_chunk.origin.y as f32 + (y + 1) as f32 * layer.cell_size;
                        let shell_x0 = terrain_chunk.origin.x as f32
                            + run.first().unwrap().x as f32 * layer.cell_size
                            + inset;
                        let shell_x1 = terrain_chunk.origin.x as f32
                            + (run.last().unwrap().x + 1) as f32 * layer.cell_size
                            - inset;
                        let mut north_pairs = Vec::new();
                        let mut south_pairs = Vec::new();
                        for x in run.first().unwrap().x..=run.last().unwrap().x + 1 {
                            let wx = if x == run.first().unwrap().x {
                                shell_x0
                            } else if x == run.last().unwrap().x + 1 {
                                shell_x1
                            } else {
                                terrain_chunk.origin.x as f32 + x as f32 * layer.cell_size
                            };
                            let span_ref = run
                                .iter()
                                .find(|s| s.x == x || s.x + 1 == x)
                                .unwrap_or(&run[0]);
                            let clamp_outer = |v: f32| {
                                if sign >= 0.0 {
                                    v.clamp(span_ref.inner(), span_ref.outer())
                                } else {
                                    v.clamp(span_ref.outer(), span_ref.inner())
                                }
                            };
                            let north_top = clamp_outer(
                                corner_lookup
                                    .get(&(x, y))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let south_top = clamp_outer(
                                corner_lookup
                                    .get(&(x, y + 1))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let north_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x,
                                span_ref.y - 1,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            let south_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x,
                                span_ref.y + 1,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            north_pairs.push((
                                world_at(wx, shell_z0, north_bottom),
                                world_at(wx, shell_z0, north_top),
                            ));
                            south_pairs.push((
                                world_at(wx, shell_z1, south_bottom),
                                world_at(wx, shell_z1, south_top),
                            ));
                        }
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Terrain(
                                terrain_chunk.origin.x + run[0].x,
                                terrain_chunk.origin.y + run[0].y,
                            ),
                            run[0].tile_id,
                            -Vec3::unit_z() * sign,
                            &north_pairs,
                            true,
                        );
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Terrain(
                                terrain_chunk.origin.x + run[0].x,
                                terrain_chunk.origin.y + run[0].y,
                            ),
                            run[0].tile_id,
                            Vec3::unit_z() * sign,
                            &south_pairs,
                            true,
                        );
                    }
                    RibbonAxis::Vertical => {
                        let x = run[0].x;
                        let shell_x0 = terrain_chunk.origin.x as f32 + x as f32 * layer.cell_size;
                        let shell_x1 =
                            terrain_chunk.origin.x as f32 + (x + 1) as f32 * layer.cell_size;
                        let shell_z0 = terrain_chunk.origin.y as f32
                            + run.first().unwrap().y as f32 * layer.cell_size
                            + inset;
                        let shell_z1 = terrain_chunk.origin.y as f32
                            + (run.last().unwrap().y + 1) as f32 * layer.cell_size
                            - inset;
                        let mut west_pairs = Vec::new();
                        let mut east_pairs = Vec::new();
                        for y in run.first().unwrap().y..=run.last().unwrap().y + 1 {
                            let wz = if y == run.first().unwrap().y {
                                shell_z0
                            } else if y == run.last().unwrap().y + 1 {
                                shell_z1
                            } else {
                                terrain_chunk.origin.y as f32 + y as f32 * layer.cell_size
                            };
                            let span_ref = run
                                .iter()
                                .find(|s| s.y == y || s.y + 1 == y)
                                .unwrap_or(&run[0]);
                            let clamp_outer = |v: f32| {
                                if sign >= 0.0 {
                                    v.clamp(span_ref.inner(), span_ref.outer())
                                } else {
                                    v.clamp(span_ref.outer(), span_ref.inner())
                                }
                            };
                            let west_top = clamp_outer(
                                corner_lookup
                                    .get(&(x, y))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let east_top = clamp_outer(
                                corner_lookup
                                    .get(&(x + 1, y))
                                    .copied()
                                    .unwrap_or(span_ref.outer()),
                            );
                            let west_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x - 1,
                                span_ref.y,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            let east_bottom = organic_overlap_top(
                                &span_lookup,
                                span_ref.x + 1,
                                span_ref.y,
                                span_ref.inner(),
                                span_ref.outer(),
                            )
                            .unwrap_or(span_ref.inner());
                            west_pairs.push((
                                world_at(shell_x0, wz, west_bottom),
                                world_at(shell_x0, wz, west_top),
                            ));
                            east_pairs.push((
                                world_at(shell_x1, wz, east_bottom),
                                world_at(shell_x1, wz, east_top),
                            ));
                        }
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Terrain(
                                terrain_chunk.origin.x + run[0].x,
                                terrain_chunk.origin.y + run[0].y,
                            ),
                            run[0].tile_id,
                            -Vec3::unit_x() * sign,
                            &west_pairs,
                            false,
                        );
                        push_organic_strip_mesh(
                            vmchunk,
                            GeoId::Terrain(
                                terrain_chunk.origin.x + run[0].x,
                                terrain_chunk.origin.y + run[0].y,
                            ),
                            run[0].tile_id,
                            Vec3::unit_x() * sign,
                            &east_pairs,
                            false,
                        );
                    }
                }
            }

            for span in spans {
                if ribbon_keys.contains(&(span.x, span.y, span.outer_sign() as i32)) {
                    continue;
                }
                let world_x =
                    terrain_chunk.origin.x as f32 + (span.x as f32 + 0.5) * layer.cell_size;
                let world_z =
                    terrain_chunk.origin.y as f32 + (span.y as f32 + 0.5) * layer.cell_size;
                if !chunk.bbox.contains(Vec2::new(world_x, world_z)) {
                    continue;
                }
                let outer = span.outer();
                let inner = span.inner();
                let sign = span.outer_sign();
                let clamp_outer = |v: f32| {
                    if sign >= 0.0 {
                        v.clamp(inner, outer)
                    } else {
                        v.clamp(outer, inner)
                    }
                };
                let nw_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x, span.y))
                        .copied()
                        .unwrap_or(outer),
                );
                let ne_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x + 1, span.y))
                        .copied()
                        .unwrap_or(outer),
                );
                let se_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x + 1, span.y + 1))
                        .copied()
                        .unwrap_or(outer),
                );
                let sw_top = clamp_outer(
                    corner_lookup
                        .get(&(span.x, span.y + 1))
                        .copied()
                        .unwrap_or(outer),
                );
                let north_bottom =
                    organic_overlap_top(&span_lookup, span.x, span.y - 1, inner, outer)
                        .unwrap_or(inner);
                let north_connected =
                    organic_overlap_top(&span_lookup, span.x, span.y - 1, inner, outer).is_some();
                let east_bottom =
                    organic_overlap_top(&span_lookup, span.x + 1, span.y, inner, outer)
                        .unwrap_or(inner);
                let east_connected =
                    organic_overlap_top(&span_lookup, span.x + 1, span.y, inner, outer).is_some();
                let south_bottom =
                    organic_overlap_top(&span_lookup, span.x, span.y + 1, inner, outer)
                        .unwrap_or(inner);
                let south_connected =
                    organic_overlap_top(&span_lookup, span.x, span.y + 1, inner, outer).is_some();
                let west_bottom =
                    organic_overlap_top(&span_lookup, span.x - 1, span.y, inner, outer)
                        .unwrap_or(inner);
                let west_connected =
                    organic_overlap_top(&span_lookup, span.x - 1, span.y, inner, outer).is_some();
                let x0 = terrain_chunk.origin.x as f32 + span.x as f32 * layer.cell_size;
                let x1 = terrain_chunk.origin.x as f32 + (span.x + 1) as f32 * layer.cell_size;
                let z0 = terrain_chunk.origin.y as f32 + span.y as f32 * layer.cell_size;
                let z1 = terrain_chunk.origin.y as f32 + (span.y + 1) as f32 * layer.cell_size;
                let world_at = |wx: f32, wz: f32, extrude: f32| -> Vec3<f32> {
                    let base = TerrainGenerator::sample_height_at(
                        map,
                        Vec2::new(wx, wz),
                        &TerrainConfig::default(),
                    );
                    Vec3::new(wx, base + extrude, wz)
                };
                let ribbon = is_ribbon_span(
                    north_connected,
                    east_connected,
                    south_connected,
                    west_connected,
                );
                let mut shell_x0 = x0;
                let mut shell_x1 = x1;
                let mut shell_z0 = z0;
                let mut shell_z1 = z1;
                if ribbon {
                    let inset = (layer.cell_size * 0.28).min(layer.cell_size * 0.45);
                    (shell_x0, shell_x1, shell_z0, shell_z1) = ribbon_bounds(
                        x0,
                        x1,
                        z0,
                        z1,
                        inset,
                        north_connected,
                        east_connected,
                        south_connected,
                        west_connected,
                    );
                }
                render_organic_shell(
                    vmchunk,
                    GeoId::Terrain(world_x.floor() as i32, world_z.floor() as i32),
                    span.tile_id,
                    [
                        -Vec3::unit_z() * sign,
                        Vec3::unit_x() * sign,
                        Vec3::unit_z() * sign,
                        -Vec3::unit_x() * sign,
                    ],
                    [
                        [
                            world_at(shell_x0, shell_z0, north_bottom),
                            world_at(shell_x1, shell_z0, north_bottom),
                            world_at(shell_x1, shell_z0, ne_top),
                            world_at(shell_x0, shell_z0, nw_top),
                        ],
                        [
                            world_at(shell_x1, shell_z0, east_bottom),
                            world_at(shell_x1, shell_z1, east_bottom),
                            world_at(shell_x1, shell_z1, se_top),
                            world_at(shell_x1, shell_z0, ne_top),
                        ],
                        [
                            world_at(shell_x1, shell_z1, south_bottom),
                            world_at(shell_x0, shell_z1, south_bottom),
                            world_at(shell_x0, shell_z1, sw_top),
                            world_at(shell_x1, shell_z1, se_top),
                        ],
                        [
                            world_at(shell_x0, shell_z1, west_bottom),
                            world_at(shell_x0, shell_z0, west_bottom),
                            world_at(shell_x0, shell_z0, nw_top),
                            world_at(shell_x0, shell_z1, sw_top),
                        ],
                    ],
                );
            }
        }
    }
}

fn push_quad_with_winding(
    vmchunk: &mut scenevm::Chunk,
    geo: GeoId,
    tile_id: Uuid,
    mut verts: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    desired_normal: Vec3<f32>,
) {
    let mut inds = vec![(0usize, 1usize, 2usize), (0usize, 2usize, 3usize)];
    fix_winding(&verts, &mut inds, desired_normal);
    vmchunk.add_poly_3d(geo, tile_id, std::mem::take(&mut verts), uvs, inds, 0, true);
}

fn generate_sector_stairs(map: &Map, assets: &Assets, chunk: &Chunk, vmchunk: &mut scenevm::Chunk) {
    let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();

    for sector in &map.sectors {
        let feature = sector
            .properties
            .get_str_default("sector_feature", "None".to_string());
        if feature != "Stairs" {
            continue;
        }

        let bbox = sector.bounding_box(map);
        if !bbox.intersects(&chunk.bbox) {
            continue;
        }

        let steps = sector.properties.get_int_default("stairs_steps", 6).max(1) as usize;
        let total_height = sector
            .properties
            .get_float_default("stairs_total_height", 1.0)
            .max(0.0);
        let fill_sides = sector
            .properties
            .get_bool_default("stairs_fill_sides", true);
        if total_height <= 0.0 {
            continue;
        }
        let dir = sector
            .properties
            .get_int_default("stairs_direction", 0)
            .clamp(0, 3);

        let base_tile = source_to_tile_id(&sector.properties, "stairs_tile_source", assets)
            .or_else(|| source_to_tile_id(&sector.properties, "cap_source", assets))
            .or_else(|| source_to_tile_id(&sector.properties, "source", assets))
            .unwrap_or(default_tile_id);
        let tread_tile = source_to_tile_id(&sector.properties, "stairs_tread_source", assets)
            .unwrap_or(base_tile);
        let riser_tile = source_to_tile_id(&sector.properties, "stairs_riser_source", assets)
            .unwrap_or(base_tile);
        let side_tile = source_to_tile_id(&sector.properties, "stairs_side_source", assets)
            .unwrap_or(base_tile);

        for (_, surface) in &map.surfaces {
            if surface.sector_id != sector.id {
                continue;
            }
            if surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }

            let Some(loop_uv) = surface.sector_loop_uv(map) else {
                continue;
            };
            if loop_uv.len() < 3 {
                continue;
            }

            let mut min_u = f32::INFINITY;
            let mut min_v = f32::INFINITY;
            let mut max_u = f32::NEG_INFINITY;
            let mut max_v = f32::NEG_INFINITY;
            for p in &loop_uv {
                min_u = min_u.min(p.x);
                min_v = min_v.min(p.y);
                max_u = max_u.max(p.x);
                max_v = max_v.max(p.y);
            }
            let tex_scale_x = sector
                .properties
                .get_float_default("texture_scale_x", 1.0)
                .max(1e-4);
            let tex_scale_y = sector
                .properties
                .get_float_default("texture_scale_y", 1.0)
                .max(1e-4);

            let (run_min, run_max, cross_min, cross_max, along_u) = match dir {
                0 => (min_v, max_v, min_u, max_u, false), // north (+V)
                1 => (min_u, max_u, min_v, max_v, true),  // east (+U)
                2 => (min_v, max_v, min_u, max_u, false), // south (-V)
                _ => (min_u, max_u, min_v, max_v, true),  // west (-U)
            };
            let run_len = (run_max - run_min).max(1e-4);
            let normal = {
                let mut n = surface.plane.normal;
                if n.y < 0.0 {
                    n = -n;
                }
                let l = n.magnitude();
                if l > 1e-6 {
                    n / l
                } else {
                    Vec3::new(0.0, 1.0, 0.0)
                }
            };

            for i in 0..steps {
                let t0 = i as f32 / steps as f32;
                let t1 = (i + 1) as f32 / steps as f32;
                let h0 = total_height * t0;
                let h1 = total_height * t1;

                let (r0, r1) = match dir {
                    0 | 1 => (run_min + run_len * t0, run_min + run_len * t1),
                    2 | 3 => (run_max - run_len * t1, run_max - run_len * t0),
                    _ => (run_min + run_len * t0, run_min + run_len * t1),
                };

                let (uv_a, uv_b, uv_c, uv_d) = if along_u {
                    (
                        Vec2::new(r0, cross_min),
                        Vec2::new(r1, cross_min),
                        Vec2::new(r1, cross_max),
                        Vec2::new(r0, cross_max),
                    )
                } else {
                    (
                        Vec2::new(cross_min, r0),
                        Vec2::new(cross_max, r0),
                        Vec2::new(cross_max, r1),
                        Vec2::new(cross_min, r1),
                    )
                };

                let top = vec![
                    {
                        let p = surface.uv_to_world(uv_a) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(uv_b) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(uv_c) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(uv_d) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                ];
                let top_uv = vec![
                    [
                        (uv_a.x - min_u) / tex_scale_x,
                        (uv_a.y - min_v) / tex_scale_y,
                    ],
                    [
                        (uv_b.x - min_u) / tex_scale_x,
                        (uv_b.y - min_v) / tex_scale_y,
                    ],
                    [
                        (uv_c.x - min_u) / tex_scale_x,
                        (uv_c.y - min_v) / tex_scale_y,
                    ],
                    [
                        (uv_d.x - min_u) / tex_scale_x,
                        (uv_d.y - min_v) / tex_scale_y,
                    ],
                ];
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    tread_tile,
                    top,
                    top_uv,
                    normal,
                );

                let front_uv0 = if along_u {
                    Vec2::new(r1, cross_min)
                } else {
                    Vec2::new(cross_min, r1)
                };
                let front_uv1 = if along_u {
                    Vec2::new(r1, cross_max)
                } else {
                    Vec2::new(cross_max, r1)
                };
                let front = vec![
                    {
                        let p = surface.uv_to_world(front_uv0) + normal * h0;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(front_uv1) + normal * h0;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(front_uv1) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(front_uv0) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                ];
                let front_w0 = surface.uv_to_world(front_uv0);
                let front_w1 = surface.uv_to_world(front_uv1);
                let front_u = (front_w1 - front_w0).magnitude() / tex_scale_x;
                let front_v = (h1 - h0).abs() / tex_scale_y;
                let front_uv = vec![
                    [0.0, 0.0],
                    [front_u, 0.0],
                    [front_u, front_v],
                    [0.0, front_v],
                ];
                let mut rise_dir = if along_u {
                    surface.edit_uv.right
                } else {
                    surface.edit_uv.up
                };
                if dir == 2 || dir == 3 {
                    rise_dir = -rise_dir;
                }
                let front_n = {
                    let n = rise_dir.cross(normal);
                    let l = n.magnitude();
                    if l > 1e-6 { n / l } else { n }
                };
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    riser_tile,
                    front,
                    front_uv,
                    front_n,
                );

                let side0_uv0 = if along_u {
                    Vec2::new(r0, cross_min)
                } else {
                    Vec2::new(cross_min, r0)
                };
                let side0_uv1 = if along_u {
                    Vec2::new(r1, cross_min)
                } else {
                    Vec2::new(cross_min, r1)
                };
                let side1_uv0 = if along_u {
                    Vec2::new(r0, cross_max)
                } else {
                    Vec2::new(cross_max, r0)
                };
                let side1_uv1 = if along_u {
                    Vec2::new(r1, cross_max)
                } else {
                    Vec2::new(cross_max, r1)
                };

                let side0 = vec![
                    {
                        let side_bottom_h = if fill_sides { 0.0 } else { h0 };
                        let p = surface.uv_to_world(side0_uv0) + normal * side_bottom_h;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let side_bottom_h = if fill_sides { 0.0 } else { h0 };
                        let p = surface.uv_to_world(side0_uv1) + normal * side_bottom_h;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(side0_uv1) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(side0_uv0) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                ];
                let side1 = vec![
                    {
                        let side_bottom_h = if fill_sides { 0.0 } else { h0 };
                        let p = surface.uv_to_world(side1_uv0) + normal * side_bottom_h;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let side_bottom_h = if fill_sides { 0.0 } else { h0 };
                        let p = surface.uv_to_world(side1_uv1) + normal * side_bottom_h;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(side1_uv1) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                    {
                        let p = surface.uv_to_world(side1_uv0) + normal * h1;
                        [p.x, p.y, p.z, 1.0]
                    },
                ];
                let side0_w0 = surface.uv_to_world(side0_uv0);
                let side0_w1 = surface.uv_to_world(side0_uv1);
                let side_u = (side0_w1 - side0_w0).magnitude() / tex_scale_x;
                let side_v = (h1 - h0).abs() / tex_scale_y;
                let side_uv = vec![[0.0, 0.0], [side_u, 0.0], [side_u, side_v], [0.0, side_v]];
                let side_dir = if along_u {
                    surface.edit_uv.up
                } else {
                    surface.edit_uv.right
                };
                let side_n0 = {
                    let n = side_dir.cross(normal);
                    let l = n.magnitude();
                    if l > 1e-6 { n / l } else { n }
                };
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    side_tile,
                    side0,
                    side_uv.clone(),
                    side_n0,
                );
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    side_tile,
                    side1,
                    side_uv,
                    -side_n0,
                );
            }
        }
    }
}

fn generate_sector_campfires(
    map: &Map,
    assets: &Assets,
    chunk: &Chunk,
    vmchunk: &mut scenevm::Chunk,
) {
    let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();

    for sector in &map.sectors {
        let feature = sector
            .properties
            .get_str_default("sector_feature", "None".to_string());
        if feature != "Campfire" {
            continue;
        }

        let bbox = sector.bounding_box(map);
        if !bbox.intersects(&chunk.bbox) {
            continue;
        }

        let flame_height = sector
            .properties
            .get_float_default("campfire_flame_height", 0.8)
            .max(0.0);
        let flame_width = sector
            .properties
            .get_float_default("campfire_flame_width", 0.45)
            .max(0.05);
        if flame_height <= 0.0 {
            continue;
        }

        let base_tile = source_to_tile_id(&sector.properties, "campfire_base_source", assets)
            .or_else(|| source_to_tile_id(&sector.properties, "source", assets))
            .unwrap_or(default_tile_id);

        let mut best_base_y = f32::NEG_INFINITY;
        let mut center = Vec3::zero();
        let mut found_top_surface = false;
        for surface in map.surfaces.values() {
            if surface.sector_id != sector.id || surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }
            let Some(loop_uv) = surface.sector_loop_uv(map) else {
                continue;
            };
            if loop_uv.len() < 3 {
                continue;
            }
            if surface.plane.origin.y <= best_base_y {
                continue;
            }
            let mut sum = Vec3::zero();
            for uv in &loop_uv {
                sum += surface.uv_to_world(*uv);
            }
            center = sum / loop_uv.len() as f32;
            best_base_y = surface.plane.origin.y;
            found_top_surface = true;
        }

        if !found_top_surface {
            if let Some(c2) = sector.center(map) {
                center = Vec3::new(c2.x, 0.0, c2.y);
                best_base_y = 0.0;
            } else {
                continue;
            }
        }

        let cx = center.x;
        let cz = center.z;
        let base_half = (flame_width * 0.45).max(0.05);
        let log_count = sector
            .properties
            .get_int_default("campfire_log_count", 10)
            .clamp(3, 24) as usize;
        let log_half_len = (sector
            .properties
            .get_float_default("campfire_log_length", 0.55)
            * 0.5)
            .max(0.05);
        let log_half_thick = (sector
            .properties
            .get_float_default("campfire_log_thickness", 0.10)
            * 0.5)
            .max(0.01);
        let log_radius = sector
            .properties
            .get_float_default("campfire_log_radius", 0.55)
            .max(log_half_len + 0.03);
        let log_half_height = (log_half_thick * 0.65).max(0.015);
        let log_center_y = best_base_y + log_half_height + 0.01;

        let uv_unit = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let mut push_oriented_log =
            |center_x: f32, center_z: f32, center_y: f32, inward: Vec3<f32>| {
                let up = Vec3::new(0.0, 1.0, 0.0);
                let mut axis = Vec3::new(inward.x, 0.0, inward.z);
                let axis_len = axis.magnitude();
                if axis_len <= 1e-6 {
                    return;
                }
                axis /= axis_len;
                let mut side = up.cross(axis);
                let side_len = side.magnitude();
                if side_len <= 1e-6 {
                    return;
                }
                side /= side_len;

                let center_v = Vec3::new(center_x, center_y, center_z);

                let a = axis * log_half_len;
                let s = side * log_half_thick;
                let u = up * log_half_height;

                // bottom ring
                let p0 = center_v - a - s - u;
                let p1 = center_v - a + s - u;
                let p2 = center_v + a + s - u;
                let p3 = center_v + a - s - u;
                // top ring
                let p4 = center_v - a - s + u;
                let p5 = center_v - a + s + u;
                let p6 = center_v + a + s + u;
                let p7 = center_v + a - s + u;

                let q = |v: Vec3<f32>| [v.x, v.y, v.z, 1.0];

                // Top / Bottom
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    base_tile,
                    vec![q(p4), q(p5), q(p6), q(p7)],
                    uv_unit.clone(),
                    up,
                );
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    base_tile,
                    vec![q(p0), q(p3), q(p2), q(p1)],
                    uv_unit.clone(),
                    -up,
                );
                // Side faces (+/- side)
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    base_tile,
                    vec![q(p1), q(p2), q(p6), q(p5)],
                    uv_unit.clone(),
                    side,
                );
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    base_tile,
                    vec![q(p0), q(p4), q(p7), q(p3)],
                    uv_unit.clone(),
                    -side,
                );
                // End faces (+/- axis)
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    base_tile,
                    vec![q(p3), q(p7), q(p6), q(p2)],
                    uv_unit.clone(),
                    axis,
                );
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    base_tile,
                    vec![q(p0), q(p1), q(p5), q(p4)],
                    uv_unit.clone(),
                    -axis,
                );
            };

        // Logs in a ring, each pointing toward the center.
        for i in 0..log_count {
            let t = i as f32 / log_count as f32;
            let ang = t * std::f32::consts::TAU;
            let dir_out = Vec3::new(ang.cos(), 0.0, ang.sin());
            let log_cx = cx + dir_out.x * log_radius;
            let log_cz = cz + dir_out.z * log_radius;
            let inward = Vec3::new(cx - log_cx, 0.0, cz - log_cz);
            let y = log_center_y + ((i % 2) as f32) * (log_half_height * 0.35);
            push_oriented_log(log_cx, log_cz, y, inward);
        }

        // Ember/base patch.
        let base = vec![
            [cx - base_half, best_base_y + 0.01, cz - base_half, 1.0],
            [cx + base_half, best_base_y + 0.01, cz - base_half, 1.0],
            [cx + base_half, best_base_y + 0.01, cz + base_half, 1.0],
            [cx - base_half, best_base_y + 0.01, cz + base_half, 1.0],
        ];
        push_quad_with_winding(
            vmchunk,
            GeoId::Sector(sector.id),
            base_tile,
            base,
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            Vec3::new(0.0, 1.0, 0.0),
        );
    }
}

fn generate_sector_builder_features(
    map: &Map,
    assets: &Assets,
    chunk: &mut Chunk,
    vmchunk: &mut scenevm::Chunk,
) {
    for sector in &map.sectors {
        let builder_graph_data = sector
            .properties
            .get_str_default("builder_graph_data", String::new());
        if builder_graph_data.trim().is_empty() {
            continue;
        }
        if sector
            .properties
            .get_str_default("builder_graph_target", "sector".to_string())
            != "sector"
        {
            continue;
        }

        let bbox = sector.bounding_box(map);
        if !bbox.intersects(&chunk.bbox) {
            continue;
        }

        let mut best_surface: Option<crate::Surface> = None;
        let mut best_loop_uv: Option<Vec<Vec2<f32>>> = None;
        let mut best_y = f32::NEG_INFINITY;
        for surface in map.surfaces.values() {
            if surface.sector_id != sector.id || surface.plane.normal.y <= 0.7 {
                continue;
            }
            let Some(loop_uv) = surface.sector_loop_uv(map) else {
                continue;
            };
            if loop_uv.len() < 3 || surface.plane.origin.y <= best_y {
                continue;
            }
            best_y = surface.plane.origin.y;
            best_surface = Some(surface.clone());
            best_loop_uv = Some(loop_uv);
        }

        let (surface, loop_uv) =
            if let (Some(surface), Some(loop_uv)) = (best_surface, best_loop_uv) {
                (surface, loop_uv)
            } else {
                let mut synthetic_surface = crate::Surface::new(sector.id);
                synthetic_surface.calculate_geometry(map);

                if synthetic_surface.plane.normal.y < 0.0 {
                    synthetic_surface.plane.normal = -synthetic_surface.plane.normal;
                    synthetic_surface.frame.normal = -synthetic_surface.frame.normal;
                    synthetic_surface.frame.up = -synthetic_surface.frame.up;
                    synthetic_surface.edit_uv.up = synthetic_surface.frame.up;
                }

                let Some(loop_uv) = synthetic_surface.sector_loop_uv(map) else {
                    continue;
                };

                (synthetic_surface, loop_uv)
            };

        let mut min_uv = loop_uv[0];
        let mut max_uv = loop_uv[0];
        for uv in &loop_uv {
            min_uv.x = min_uv.x.min(uv.x);
            min_uv.y = min_uv.y.min(uv.y);
            max_uv.x = max_uv.x.max(uv.x);
            max_uv.y = max_uv.y.max(uv.y);
        }

        let profile_target = sector.properties.get_int_default("profile_target", 0);
        let base_extrusion = if profile_target == 1 {
            surface.extrusion.depth.abs()
        } else {
            0.0
        };
        // Builder sector assemblies are authored as positive-up features above the host surface.
        // Do not reuse the profile loop recess/relief sign convention here.
        let direction = 1.0;

        let _ = emit_builder_sector_meshes(
            &builder_graph_data,
            min_uv,
            max_uv,
            base_extrusion,
            direction,
            &surface,
            map,
            sector,
            chunk,
            vmchunk,
            assets,
            None,
            profile_target,
        );
    }
}

fn generate_sector_roofs(map: &Map, assets: &Assets, chunk: &Chunk, vmchunk: &mut scenevm::Chunk) {
    let default_tile_id = Uuid::from_str(DEFAULT_TILE_ID).unwrap();

    for sector in &map.sectors {
        let feature = sector
            .properties
            .get_str_default("sector_feature", "None".to_string());
        if feature != "Roof" {
            continue;
        }

        let bbox = sector.bounding_box(map);
        if !bbox.intersects(&chunk.bbox) {
            continue;
        }

        let roof_height = sector
            .properties
            .get_float_default("roof_height", 1.0)
            .max(0.0);
        if roof_height <= 0.0 {
            continue;
        }
        let roof_overhang = sector
            .properties
            .get_float_default("roof_overhang", 0.0)
            .max(0.0);
        let roof_style = sector
            .properties
            .get_int_default("roof_style", 1)
            .clamp(0, 2);

        let roof_tile = source_to_tile_id(&sector.properties, "roof_tile_source", assets)
            .or_else(|| source_to_tile_id(&sector.properties, "cap_source", assets))
            .or_else(|| source_to_tile_id(&sector.properties, "source", assets))
            .unwrap_or(default_tile_id);
        let side_tile =
            source_to_tile_id(&sector.properties, "roof_side_source", assets).unwrap_or(roof_tile);

        // Use the highest horizontal surface of the sector as roof base.
        let mut best_base_y = f32::NEG_INFINITY;
        let mut best_loop_uv: Option<Vec<Vec2<f32>>> = None;
        let mut best_surface: Option<&crate::Surface> = None;

        for (_, surface) in &map.surfaces {
            if surface.sector_id != sector.id {
                continue;
            }
            if surface.plane.normal.y.abs() <= 0.7 {
                continue;
            }
            let Some(loop_uv) = surface.sector_loop_uv(map) else {
                continue;
            };
            if loop_uv.len() < 3 {
                continue;
            }
            if surface.plane.origin.y > best_base_y {
                best_base_y = surface.plane.origin.y;
                best_loop_uv = Some(loop_uv);
                best_surface = Some(surface);
            }
        }

        let Some(loop_uv) = best_loop_uv else {
            continue;
        };
        let Some(surface) = best_surface else {
            continue;
        };

        let mut base_ring: Vec<Vec3<f32>> = Vec::with_capacity(loop_uv.len());
        for uv in &loop_uv {
            let p = surface.uv_to_world(*uv);
            base_ring.push(Vec3::new(p.x, best_base_y, p.z));
        }
        if base_ring.len() < 3 {
            continue;
        }

        let tex_scale_x = sector
            .properties
            .get_float_default("texture_scale_x", 1.0)
            .max(1e-4);
        let tex_scale_y = sector
            .properties
            .get_float_default("texture_scale_y", 1.0)
            .max(1e-4);

        let overhung_base_ring: Vec<Vec3<f32>> = if roof_overhang > 0.0 {
            let base_xz: Vec<Vec2<f32>> = base_ring.iter().map(|p| Vec2::new(p.x, p.z)).collect();
            let expanded_xz = offset_polygon_outward_2d(&base_xz, roof_overhang);
            expanded_xz
                .iter()
                .zip(base_ring.iter())
                .map(|(xz, p)| Vec3::new(xz.x, p.y, xz.y))
                .collect()
        } else {
            base_ring.clone()
        };
        let n_ring = overhung_base_ring.len();
        let polygon_signed_area = {
            let mut a = 0.0f32;
            for i in 0..n_ring {
                let p = overhung_base_ring[i];
                let q = overhung_base_ring[(i + 1) % n_ring];
                a += p.x * q.z - q.x * p.z;
            }
            0.5 * a
        };
        let mut concave_vertex = vec![false; n_ring];
        if n_ring >= 3 {
            for i in 0..n_ring {
                let p0 = overhung_base_ring[(i + n_ring - 1) % n_ring];
                let p1 = overhung_base_ring[i];
                let p2 = overhung_base_ring[(i + 1) % n_ring];
                let e1 = Vec2::new(p1.x - p0.x, p1.z - p0.z);
                let e2 = Vec2::new(p2.x - p1.x, p2.z - p1.z);
                let cross = e1.x * e2.y - e1.y * e2.x;
                let is_concave = if polygon_signed_area >= 0.0 {
                    cross < -1e-5
                } else {
                    cross > 1e-5
                };
                concave_vertex[i] = is_concave;
            }
        }

        // Axis-aligned gable basis:
        // if X span is larger, ridge runs along X and slope samples along Z (and vice versa).
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_z = f32::INFINITY;
        let mut max_z = f32::NEG_INFINITY;
        for p in &overhung_base_ring {
            min_x = min_x.min(p.x);
            max_x = max_x.max(p.x);
            min_z = min_z.min(p.z);
            max_z = max_z.max(p.z);
        }
        let span_x = (max_x - min_x).max(1e-6);
        let span_z = (max_z - min_z).max(1e-6);
        let gable_axis_is_x = span_x >= span_z;
        let along_of = |p: Vec3<f32>| -> f32 { if gable_axis_is_x { p.x } else { p.z } };
        let sample_of = |p: Vec3<f32>| -> f32 { if gable_axis_is_x { p.z } else { p.x } };
        let world_from_along_sample = |along: f32, sample: f32, y: f32| -> [f32; 4] {
            if gable_axis_is_x {
                [along, y, sample, 1.0]
            } else {
                [sample, y, along, 1.0]
            }
        };
        // Keep gable pitch constant when overhang expands the footprint:
        // scale rise by sample-axis span ratio (overhung/original).
        let roof_rise = if roof_style == 2 && roof_overhang > 0.0 {
            let (mut base_min_s, mut base_max_s) = (f32::INFINITY, f32::NEG_INFINITY);
            for p in &base_ring {
                let s = sample_of(*p);
                base_min_s = base_min_s.min(s);
                base_max_s = base_max_s.max(s);
            }
            let base_span_s = (base_max_s - base_min_s).abs();
            let over_span_s = if gable_axis_is_x { span_z } else { span_x };
            if base_span_s > 1e-6 {
                roof_height * (over_span_s / base_span_s)
            } else {
                roof_height
            }
        } else {
            roof_height
        };

        let mut along_values: Vec<f32> = overhung_base_ring.iter().map(|p| along_of(*p)).collect();
        along_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        along_values.dedup_by(|a, b| (*a - *b).abs() < 1e-4);

        let along_min = *along_values.first().unwrap_or(&0.0);
        let along_max = *along_values.last().unwrap_or(&1.0);
        let along_eps = ((along_max - along_min).abs() * 0.001).max(1e-4);

        let cross_segments = |along: f32| -> Vec<(f32, f32)> {
            let mut intersections: Vec<f32> = Vec::new();
            let n = overhung_base_ring.len();
            for i in 0..n {
                let a = overhung_base_ring[i];
                let b = overhung_base_ring[(i + 1) % n];
                let au = along_of(a);
                let bu = along_of(b);
                let av = sample_of(a);
                let bv = sample_of(b);

                if (au - bu).abs() <= 1e-6 {
                    continue;
                }
                if (au <= along && bu > along) || (bu <= along && au > along) {
                    let t = (along - au) / (bu - au);
                    intersections.push(av + t * (bv - av));
                }
            }
            intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mut segs: Vec<(f32, f32)> = Vec::new();
            let mut i = 0usize;
            while i + 1 < intersections.len() {
                let s0 = intersections[i];
                let s1 = intersections[i + 1];
                if s1 - s0 > 1e-6 {
                    segs.push((s0, s1));
                }
                i += 2;
            }
            segs
        };

        let gable_factor = |p: Vec3<f32>| -> f32 {
            let u = along_of(p).clamp(along_min + along_eps, along_max - along_eps);
            let v = sample_of(p);
            let segs = cross_segments(u);
            if segs.is_empty() {
                return 0.0;
            }

            let mut chosen = segs[0];
            for s in &segs {
                if v >= s.0 - 1e-3 && v <= s.1 + 1e-3 {
                    chosen = *s;
                    break;
                }
            }
            let width = (chosen.1 - chosen.0).max(1e-6);
            let t = ((v - chosen.0) / width).clamp(0.0, 1.0);
            1.0 - (2.0 * t - 1.0).abs()
        };
        let top_height = |p: Vec3<f32>| -> f32 {
            match roof_style {
                2 => best_base_y + roof_rise * gable_factor(p),
                _ => best_base_y + roof_height,
            }
        };

        // Gable patch descriptors used by side filler classification:
        // (u0, u1, s0(lo,hi), s1(lo,hi), local_swap)
        let mut gable_patches: Vec<(f32, f32, (f32, f32), (f32, f32), bool)> = Vec::new();

        // --- Top ---
        if roof_style == 1 {
            // Pyramid roof (fan to apex)
            let mut centroid = Vec3::zero();
            for p in &overhung_base_ring {
                centroid += *p;
            }
            centroid /= overhung_base_ring.len() as f32;
            let apex = Vec3::new(centroid.x, best_base_y + roof_height, centroid.z);

            for i in 0..overhung_base_ring.len() {
                let j = (i + 1) % overhung_base_ring.len();
                let a = overhung_base_ring[i];
                let b = overhung_base_ring[j];

                let tri_vertices = vec![
                    [a.x, a.y, a.z, 1.0],
                    [b.x, b.y, b.z, 1.0],
                    [apex.x, apex.y, apex.z, 1.0],
                ];

                let mid = (a + b) * 0.5;
                let outward = Vec3::new(mid.x - centroid.x, 0.2, mid.z - centroid.z);
                let mut tri_indices = vec![(0usize, 1usize, 2usize)];
                fix_winding(&tri_vertices, &mut tri_indices, outward);

                let tri_uvs = vec![
                    [a.x / tex_scale_x, a.z / tex_scale_y],
                    [b.x / tex_scale_x, b.z / tex_scale_y],
                    [apex.x / tex_scale_x, apex.z / tex_scale_y],
                ];

                vmchunk.add_poly_3d(
                    GeoId::Sector(sector.id),
                    roof_tile,
                    tri_vertices,
                    tri_uvs,
                    tri_indices,
                    0,
                    true,
                );
            }
        } else if roof_style == 2 {
            let mut built_strips = 0usize;
            for i in 0..along_values.len().saturating_sub(1) {
                let u0 = along_values[i];
                let u1 = along_values[i + 1];
                if (u1 - u0).abs() <= 1e-4 {
                    continue;
                }
                let u0s = (u0 + along_eps).min(u1 - along_eps);
                let u1s = (u1 - along_eps).max(u0 + along_eps);
                if u1s <= u0s {
                    continue;
                }

                let segs0 = cross_segments(u0s);
                let segs1 = cross_segments(u1s);
                if segs0.is_empty() || segs1.is_empty() {
                    continue;
                }

                let mut used = vec![false; segs1.len()];
                for s0 in &segs0 {
                    let c0 = 0.5 * (s0.0 + s0.1);
                    let mut best_j: Option<usize> = None;
                    let mut best_score = f32::INFINITY;
                    for (j, s1) in segs1.iter().enumerate() {
                        if used[j] {
                            continue;
                        }
                        let c1 = 0.5 * (s1.0 + s1.1);
                        let overlap = (s0.1.min(s1.1) - s0.0.max(s1.0)).max(0.0);
                        let center_dist = (c0 - c1).abs();
                        let score = center_dist - overlap * 0.25;
                        if score < best_score {
                            best_score = score;
                            best_j = Some(j);
                        }
                    }
                    let Some(j) = best_j else {
                        continue;
                    };
                    used[j] = true;
                    let s1 = segs1[j];
                    let along_len = (u1 - u0).abs();
                    let w0 = (s0.1 - s0.0).abs();
                    let w1 = (s1.1 - s1.0).abs();
                    let avg_width = 0.5 * (w0 + w1);
                    let local_swap = avg_width > along_len * 1.05;
                    gable_patches.push((u0, u1, (s0.0, s0.1), (s1.0, s1.1), local_swap));
                    built_strips += 1;
                }
            }
            let mut top_start_override: Vec<Option<(f32, f32)>> = vec![None; gable_patches.len()];
            let mut top_end_override: Vec<Option<(f32, f32)>> = vec![None; gable_patches.len()];
            if gable_patches.len() > 1 {
                #[derive(Clone, Copy)]
                struct JunctionCap {
                    patch: usize,
                    kind: u8, // 0=start, 1=end, 2=low, 3=high
                    a: Vec2<f32>,
                    b: Vec2<f32>,
                    local_swap: bool,
                    ridge_len: f32,
                }
                let to_xz = |u: f32, s: f32| -> Vec2<f32> {
                    if gable_axis_is_x {
                        Vec2::new(u, s)
                    } else {
                        Vec2::new(s, u)
                    }
                };
                let xz_to_us = |p: Vec2<f32>| -> (f32, f32) {
                    if gable_axis_is_x {
                        (p.x, p.y)
                    } else {
                        (p.y, p.x)
                    }
                };
                let mut caps: Vec<JunctionCap> = Vec::new();
                for (idx, (u0, u1, s0, s1, local_swap)) in gable_patches.iter().enumerate() {
                    if !*local_swap {
                        let rl = (*u1 - *u0).abs();
                        caps.push(JunctionCap {
                            patch: idx,
                            kind: 0,
                            a: to_xz(*u0, s0.0),
                            b: to_xz(*u0, s0.1),
                            local_swap: false,
                            ridge_len: rl,
                        });
                        caps.push(JunctionCap {
                            patch: idx,
                            kind: 1,
                            a: to_xz(*u1, s1.0),
                            b: to_xz(*u1, s1.1),
                            local_swap: false,
                            ridge_len: rl,
                        });
                    } else {
                        let rl = 0.5 * ((s0.1 - s0.0).abs() + (s1.1 - s1.0).abs());
                        caps.push(JunctionCap {
                            patch: idx,
                            kind: 2,
                            a: to_xz(*u0, s0.0),
                            b: to_xz(*u1, s1.0),
                            local_swap: true,
                            ridge_len: rl,
                        });
                        caps.push(JunctionCap {
                            patch: idx,
                            kind: 3,
                            a: to_xz(*u0, s0.1),
                            b: to_xz(*u1, s1.1),
                            local_swap: true,
                            ridge_len: rl,
                        });
                    }
                }
                let cap_link_eps = (roof_overhang * 0.5).max(0.12);
                for i in 0..caps.len() {
                    for j in (i + 1)..caps.len() {
                        let ci = caps[i];
                        let cj = caps[j];
                        if ci.patch == cj.patch || ci.local_swap == cj.local_swap {
                            continue;
                        }
                        let d = distance_segment_to_segment_2d(ci.a, ci.b, cj.a, cj.b);
                        if d > cap_link_eps {
                            continue;
                        }
                        let (wrap, anchor) = if ci.ridge_len >= cj.ridge_len {
                            (ci, cj)
                        } else {
                            (cj, ci)
                        };
                        if !wrap.local_swap {
                            // Move only along ridge axis (u). Keep cross axis (s) from the wrapped cap.
                            let (anchor_u, _) = xz_to_us((anchor.a + anchor.b) * 0.5);
                            let (_, wrap_s) = xz_to_us((wrap.a + wrap.b) * 0.5);
                            if wrap.kind == 0 {
                                top_start_override[wrap.patch] = Some((anchor_u, wrap_s));
                            } else if wrap.kind == 1 {
                                top_end_override[wrap.patch] = Some((anchor_u, wrap_s));
                            }
                        }
                    }
                }
            }
            let h = best_base_y + roof_rise;
            for (idx, (u0, u1, s0, s1, local_swap)) in gable_patches.iter().enumerate() {
                if !*local_swap {
                    let sm0 = 0.5 * (s0.0 + s0.1);
                    let sm1 = 0.5 * (s1.0 + s1.1);
                    let ridge0 = top_start_override[idx].unwrap_or((*u0, sm0));
                    let ridge1 = top_end_override[idx].unwrap_or((*u1, sm1));
                    let left = vec![
                        world_from_along_sample(*u0, s0.0, best_base_y),
                        world_from_along_sample(*u1, s1.0, best_base_y),
                        world_from_along_sample(ridge1.0, ridge1.1, h),
                        world_from_along_sample(ridge0.0, ridge0.1, h),
                    ];
                    let right = vec![
                        world_from_along_sample(ridge0.0, ridge0.1, h),
                        world_from_along_sample(ridge1.0, ridge1.1, h),
                        world_from_along_sample(*u1, s1.1, best_base_y),
                        world_from_along_sample(*u0, s0.1, best_base_y),
                    ];
                    let left_uv = vec![
                        [left[0][0] / tex_scale_x, left[0][2] / tex_scale_y],
                        [left[1][0] / tex_scale_x, left[1][2] / tex_scale_y],
                        [left[2][0] / tex_scale_x, left[2][2] / tex_scale_y],
                        [left[3][0] / tex_scale_x, left[3][2] / tex_scale_y],
                    ];
                    let right_uv = vec![
                        [right[0][0] / tex_scale_x, right[0][2] / tex_scale_y],
                        [right[1][0] / tex_scale_x, right[1][2] / tex_scale_y],
                        [right[2][0] / tex_scale_x, right[2][2] / tex_scale_y],
                        [right[3][0] / tex_scale_x, right[3][2] / tex_scale_y],
                    ];
                    push_quad_with_winding(
                        vmchunk,
                        GeoId::Sector(sector.id),
                        roof_tile,
                        left,
                        left_uv,
                        Vec3::new(0.0, 1.0, 0.0),
                    );
                    push_quad_with_winding(
                        vmchunk,
                        GeoId::Sector(sector.id),
                        roof_tile,
                        right,
                        right_uv,
                        Vec3::new(0.0, 1.0, 0.0),
                    );
                } else {
                    let um = 0.5 * (*u0 + *u1);
                    let ml = 0.5 * (s0.0 + s1.0);
                    let mr = 0.5 * (s0.1 + s1.1);
                    let q0 = vec![
                        world_from_along_sample(*u0, s0.0, best_base_y),
                        world_from_along_sample(um, ml, h),
                        world_from_along_sample(um, mr, h),
                        world_from_along_sample(*u0, s0.1, best_base_y),
                    ];
                    let q1 = vec![
                        world_from_along_sample(um, ml, h),
                        world_from_along_sample(*u1, s1.0, best_base_y),
                        world_from_along_sample(*u1, s1.1, best_base_y),
                        world_from_along_sample(um, mr, h),
                    ];
                    let q0_uv = vec![
                        [q0[0][0] / tex_scale_x, q0[0][2] / tex_scale_y],
                        [q0[1][0] / tex_scale_x, q0[1][2] / tex_scale_y],
                        [q0[2][0] / tex_scale_x, q0[2][2] / tex_scale_y],
                        [q0[3][0] / tex_scale_x, q0[3][2] / tex_scale_y],
                    ];
                    let q1_uv = vec![
                        [q1[0][0] / tex_scale_x, q1[0][2] / tex_scale_y],
                        [q1[1][0] / tex_scale_x, q1[1][2] / tex_scale_y],
                        [q1[2][0] / tex_scale_x, q1[2][2] / tex_scale_y],
                        [q1[3][0] / tex_scale_x, q1[3][2] / tex_scale_y],
                    ];
                    push_quad_with_winding(
                        vmchunk,
                        GeoId::Sector(sector.id),
                        roof_tile,
                        q0,
                        q0_uv,
                        Vec3::new(0.0, 1.0, 0.0),
                    );
                    push_quad_with_winding(
                        vmchunk,
                        GeoId::Sector(sector.id),
                        roof_tile,
                        q1,
                        q1_uv,
                        Vec3::new(0.0, 1.0, 0.0),
                    );
                }
            }
            if built_strips == 0 {
                if let Some((_world_vertices, mut top_indices, verts_uv)) =
                    surface.triangulate(sector, map)
                {
                    let mut top_vertices: Vec<[f32; 4]> = Vec::with_capacity(verts_uv.len());
                    let mut top_uvs: Vec<[f32; 2]> = Vec::with_capacity(verts_uv.len());
                    for uv in verts_uv {
                        let wp = surface.uv_to_world(Vec2::new(uv[0], uv[1]));
                        let base = Vec3::new(wp.x, best_base_y, wp.z);
                        let y = top_height(base);
                        top_vertices.push([base.x, y, base.z, 1.0]);
                        top_uvs.push([base.x / tex_scale_x, base.z / tex_scale_y]);
                    }
                    fix_winding(&top_vertices, &mut top_indices, Vec3::new(0.0, 1.0, 0.0));
                    vmchunk.add_poly_3d(
                        GeoId::Sector(sector.id),
                        roof_tile,
                        top_vertices,
                        top_uvs,
                        top_indices,
                        0,
                        true,
                    );
                }
            }
        } else if let Some((_world_vertices, mut top_indices, verts_uv)) =
            surface.triangulate(sector, map)
        {
            let mut top_vertices: Vec<[f32; 4]> = Vec::with_capacity(verts_uv.len());
            let mut top_uvs: Vec<[f32; 2]> = Vec::with_capacity(verts_uv.len());
            for uv in verts_uv {
                let wp = surface.uv_to_world(Vec2::new(uv[0], uv[1]));
                let base = Vec3::new(wp.x, best_base_y, wp.z);
                let y = top_height(base);
                top_vertices.push([base.x, y, base.z, 1.0]);
                top_uvs.push([base.x / tex_scale_x, base.z / tex_scale_y]);
            }
            fix_winding(&top_vertices, &mut top_indices, Vec3::new(0.0, 1.0, 0.0));
            vmchunk.add_poly_3d(
                GeoId::Sector(sector.id),
                roof_tile,
                top_vertices,
                top_uvs,
                top_indices,
                0,
                true,
            );
        }

        // --- Sides ---
        if roof_style == 1 {
            let mut centroid = Vec3::zero();
            for p in &overhung_base_ring {
                centroid += *p;
            }
            centroid /= overhung_base_ring.len() as f32;
            let apex = Vec3::new(centroid.x, best_base_y + roof_height, centroid.z);

            for i in 0..overhung_base_ring.len() {
                let j = (i + 1) % overhung_base_ring.len();
                let a = overhung_base_ring[i];
                let b = overhung_base_ring[j];
                let tri_vertices = vec![
                    [a.x, a.y, a.z, 1.0],
                    [b.x, b.y, b.z, 1.0],
                    [apex.x, apex.y, apex.z, 1.0],
                ];
                let edge_len = (b - a).magnitude().max(1e-4);
                let side_uvs = vec![
                    [0.0, 0.0],
                    [edge_len / tex_scale_x, 0.0],
                    [0.5 * edge_len / tex_scale_x, roof_height / tex_scale_y],
                ];
                let mut tri_indices = vec![(0usize, 1usize, 2usize)];
                let mid = (a + b) * 0.5;
                let outward = Vec3::new(mid.x - centroid.x, 0.2, mid.z - centroid.z);
                fix_winding(&tri_vertices, &mut tri_indices, outward);
                vmchunk.add_poly_3d(
                    GeoId::Sector(sector.id),
                    side_tile,
                    tri_vertices,
                    side_uvs,
                    tri_indices,
                    0,
                    true,
                );
            }
        } else if roof_style == 2 {
            // Build side fillers from explicit gable segments:
            // create end-caps only at open graph ends (start/end of each segment chain),
            // avoiding long-side false positives from boundary-edge heuristics.
            #[derive(Clone, Copy)]
            struct GablePatchSeg {
                u0: f32,
                u1: f32,
                s0: (f32, f32),
                s1: (f32, f32),
                local_swap: bool,
            }

            let patches: Vec<GablePatchSeg> = gable_patches
                .iter()
                .map(|(u0, u1, s0, s1, local_swap)| GablePatchSeg {
                    u0: *u0,
                    u1: *u1,
                    s0: *s0,
                    s1: *s1,
                    local_swap: *local_swap,
                })
                .collect();

            if !patches.is_empty() {
                let mut start_linked = vec![false; patches.len()];
                let mut end_linked = vec![false; patches.len()];
                let link_u_eps = along_eps.max(1e-3);
                let link_overlap_eps = 1e-3;

                for i in 0..patches.len() {
                    for j in 0..patches.len() {
                        if i == j {
                            continue;
                        }
                        if patches[i].local_swap != patches[j].local_swap {
                            continue;
                        }
                        if (patches[i].u1 - patches[j].u0).abs() > link_u_eps {
                            continue;
                        }
                        let overlap = (patches[i].s1.1.min(patches[j].s0.1)
                            - patches[i].s1.0.max(patches[j].s0.0))
                        .max(0.0);
                        if overlap > link_overlap_eps {
                            end_linked[i] = true;
                            start_linked[j] = true;
                        }
                    }
                }

                let ridge_dir = if gable_axis_is_x {
                    Vec3::new(1.0, 0.0, 0.0)
                } else {
                    Vec3::new(0.0, 0.0, 1.0)
                };
                let base_poly_xz: Vec<Vec2<f32>> =
                    base_ring.iter().map(|p| Vec2::new(p.x, p.z)).collect();
                let snap_cap_base_to_footprint = |v: [f32; 4]| -> [f32; 4] {
                    if roof_overhang <= 0.0 {
                        return v;
                    }
                    let snapped =
                        closest_point_on_polygon_edges_2d(Vec2::new(v[0], v[2]), &base_poly_xz);
                    [snapped.x, v[1], snapped.y, v[3]]
                };
                for (idx, p) in patches.iter().enumerate() {
                    if !p.local_swap {
                        if !start_linked[idx] {
                            let lo = p.s0.0.min(p.s0.1);
                            let hi = p.s0.0.max(p.s0.1);
                            let width = (hi - lo).abs();
                            if width > 1e-4 {
                                let b0 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u0,
                                    lo,
                                    best_base_y,
                                ));
                                let b1 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u0,
                                    hi,
                                    best_base_y,
                                ));
                                let apex_xz =
                                    Vec2::new((b0[0] + b1[0]) * 0.5, (b0[2] + b1[2]) * 0.5);
                                let tri_vertices = vec![
                                    b0,
                                    b1,
                                    [apex_xz.x, best_base_y + roof_rise, apex_xz.y, 1.0],
                                ];
                                let tri_uvs = vec![
                                    [0.0, 0.0],
                                    [width / tex_scale_x, 0.0],
                                    [0.5 * width / tex_scale_x, roof_rise / tex_scale_y],
                                ];
                                let mut tri_indices = vec![(0usize, 1usize, 2usize)];
                                fix_winding(&tri_vertices, &mut tri_indices, -ridge_dir);
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    side_tile,
                                    tri_vertices,
                                    tri_uvs,
                                    tri_indices,
                                    0,
                                    true,
                                );
                            }
                        }
                        if !end_linked[idx] {
                            let lo = p.s1.0.min(p.s1.1);
                            let hi = p.s1.0.max(p.s1.1);
                            let width = (hi - lo).abs();
                            if width > 1e-4 {
                                let b0 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u1,
                                    lo,
                                    best_base_y,
                                ));
                                let b1 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u1,
                                    hi,
                                    best_base_y,
                                ));
                                let apex_xz =
                                    Vec2::new((b0[0] + b1[0]) * 0.5, (b0[2] + b1[2]) * 0.5);
                                let tri_vertices = vec![
                                    b0,
                                    b1,
                                    [apex_xz.x, best_base_y + roof_rise, apex_xz.y, 1.0],
                                ];
                                let tri_uvs = vec![
                                    [0.0, 0.0],
                                    [width / tex_scale_x, 0.0],
                                    [0.5 * width / tex_scale_x, roof_rise / tex_scale_y],
                                ];
                                let mut tri_indices = vec![(0usize, 1usize, 2usize)];
                                fix_winding(&tri_vertices, &mut tri_indices, ridge_dir);
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    side_tile,
                                    tri_vertices,
                                    tri_uvs,
                                    tri_indices,
                                    0,
                                    true,
                                );
                            }
                        }
                    } else {
                        // Rotated local segment: close by sample-side caps (not global along-axis caps).
                        let side_dir = if gable_axis_is_x {
                            Vec3::new(0.0, 0.0, 1.0)
                        } else {
                            Vec3::new(1.0, 0.0, 0.0)
                        };
                        let width = (p.u1 - p.u0).abs();
                        if width > 1e-4 {
                            {
                                let b0 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u0,
                                    p.s0.0,
                                    best_base_y,
                                ));
                                let b1 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u1,
                                    p.s1.0,
                                    best_base_y,
                                ));
                                let apex0_xz =
                                    Vec2::new((b0[0] + b1[0]) * 0.5, (b0[2] + b1[2]) * 0.5);
                                let tri0 = vec![
                                    b0,
                                    b1,
                                    [apex0_xz.x, best_base_y + roof_rise, apex0_xz.y, 1.0],
                                ];
                                let uv0 = vec![
                                    [0.0, 0.0],
                                    [width / tex_scale_x, 0.0],
                                    [0.5 * width / tex_scale_x, roof_rise / tex_scale_y],
                                ];
                                let mut ind0 = vec![(0usize, 1usize, 2usize)];
                                fix_winding(&tri0, &mut ind0, -side_dir);
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    side_tile,
                                    tri0,
                                    uv0,
                                    ind0,
                                    0,
                                    true,
                                );
                            }

                            {
                                let b0 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u0,
                                    p.s0.1,
                                    best_base_y,
                                ));
                                let b1 = snap_cap_base_to_footprint(world_from_along_sample(
                                    p.u1,
                                    p.s1.1,
                                    best_base_y,
                                ));
                                let apex1_xz =
                                    Vec2::new((b0[0] + b1[0]) * 0.5, (b0[2] + b1[2]) * 0.5);
                                let tri1 = vec![
                                    b0,
                                    b1,
                                    [apex1_xz.x, best_base_y + roof_rise, apex1_xz.y, 1.0],
                                ];
                                let uv1 = vec![
                                    [0.0, 0.0],
                                    [width / tex_scale_x, 0.0],
                                    [0.5 * width / tex_scale_x, roof_rise / tex_scale_y],
                                ];
                                let mut ind1 = vec![(0usize, 1usize, 2usize)];
                                fix_winding(&tri1, &mut ind1, side_dir);
                                vmchunk.add_poly_3d(
                                    GeoId::Sector(sector.id),
                                    side_tile,
                                    tri1,
                                    uv1,
                                    ind1,
                                    0,
                                    true,
                                );
                            }
                        }
                    }
                }
            }
        } else {
            for i in 0..overhung_base_ring.len() {
                let j = (i + 1) % overhung_base_ring.len();
                let a = overhung_base_ring[i];
                let b = overhung_base_ring[j];
                let ta = Vec3::new(a.x, top_height(a), a.z);
                let tb = Vec3::new(b.x, top_height(b), b.z);
                let edge = b - a;
                let edge_len = edge.magnitude().max(1e-4);
                let outward = Vec3::new(edge.z, 0.0, -edge.x);
                let avg_h = ((ta.y - a.y).abs() + (tb.y - b.y).abs()) * 0.5;
                let h = avg_h.max(1e-4);
                let side_uvs = vec![
                    [0.0, 0.0],
                    [edge_len / tex_scale_x, 0.0],
                    [edge_len / tex_scale_x, h / tex_scale_y],
                    [0.0, h / tex_scale_y],
                ];
                let side_verts = vec![
                    [a.x, a.y, a.z, 1.0],
                    [b.x, b.y, b.z, 1.0],
                    [tb.x, tb.y, tb.z, 1.0],
                    [ta.x, ta.y, ta.z, 1.0],
                ];
                push_quad_with_winding(
                    vmchunk,
                    GeoId::Sector(sector.id),
                    side_tile,
                    side_verts,
                    side_uvs,
                    outward,
                );
            }
        }
    }
}

// --- Terrain Generation ---
fn generate_terrain(
    map: &Map,
    assets: &Assets,
    chunk: &mut Chunk,
    vmchunk: &mut scenevm::Chunk,
    _terrain_id: u32,
) {
    // Check if terrain generation is enabled for this map
    let terrain_enabled = map.properties.get_bool_default("terrain_enabled", false);

    if !terrain_enabled {
        return;
    }

    // Get default terrain tile ID from map properties
    let default_tile_id =
        if let Some(Value::Source(pixel_source)) = map.properties.get("default_terrain_tile") {
            if let Some(tile) = pixel_source.tile_from_tile_list(assets) {
                tile.id
            } else {
                Uuid::from_str(DEFAULT_TILE_ID).unwrap()
            }
        } else {
            Uuid::from_str(DEFAULT_TILE_ID).unwrap()
        };

    // Get tile overrides from map properties (same as surface builder)
    let tile_overrides = map.properties.get("tiles").and_then(|v| {
        if let Value::TileOverrides(map) = v {
            Some(map)
        } else {
            None
        }
    });

    // Get blend tile overrides from map properties
    let blend_overrides = map.properties.get("blend_tiles").and_then(|v| {
        if let Value::BlendOverrides(map) = v {
            Some(map)
        } else {
            None
        }
    });

    // Create terrain generator config (can be upsampled per-ridge via `ridge_subdiv`).
    let mut config = TerrainConfig::default();
    let mut chunk_ridge_subdiv = config.subdivisions.max(1);
    let mut chunk_exclusion_subdiv = config.subdivisions.max(1);
    for sector in &map.sectors {
        match sector.properties.get_int_default("terrain_mode", 0) {
            1 => {
                if sector.bounding_box(map).intersects(&chunk.bbox) {
                    let exclusion_subdiv = sector
                        .properties
                        .get_int_default("terrain_exclusion_subdiv", 4)
                        .clamp(1, 8) as u32;
                    chunk_exclusion_subdiv = chunk_exclusion_subdiv.max(exclusion_subdiv);
                }
            }
            2 => {
                let mut expanded_bbox = sector.bounding_box(map);
                let influence = sector
                    .properties
                    .get_float_default("ridge_plateau_width", 0.0)
                    .max(0.0)
                    + sector
                        .properties
                        .get_float_default("ridge_falloff_distance", 0.0)
                        .max(0.0);
                if influence > 0.0 {
                    expanded_bbox.expand(Vec2::broadcast(influence * 2.0));
                }
                if !expanded_bbox.intersects(&chunk.bbox) {
                    continue;
                }
                let ridge_subdiv = sector
                    .properties
                    .get_int_default("ridge_subdiv", 1)
                    .clamp(1, 8) as u32;
                chunk_ridge_subdiv = chunk_ridge_subdiv.max(ridge_subdiv);
            }
            _ => {}
        }
    }
    config.subdivisions = chunk_ridge_subdiv.max(chunk_exclusion_subdiv);
    let generator = TerrainGenerator::new(config);

    let mut builder_replace_polygons: Vec<(BBox, Vec<Vec2<f32>>)> = Vec::new();
    for sector in &map.sectors {
        if sector
            .properties
            .get_str_default("builder_graph_target", "sector".to_string())
            != "sector"
        {
            continue;
        }
        let builder_replace_surface = sector
            .properties
            .get_str_default("builder_surface_mode", String::new())
            == "replace";
        if !builder_replace_surface {
            continue;
        }
        if sector
            .properties
            .get_str_default("builder_graph_data", String::new())
            .trim()
            .is_empty()
        {
            continue;
        }
        let Some((verts, _)) = sector.generate_geometry(map) else {
            continue;
        };
        let polygon: Vec<Vec2<f32>> = verts.into_iter().map(|v| Vec2::new(v[0], v[1])).collect();
        if polygon.len() < 3 {
            continue;
        }
        builder_replace_polygons.push((sector.bounding_box(map), polygon));
    }

    // Collect road tile definitions from linedefs.
    let mut road_tile_linedefs: Vec<(Vec2<f32>, Vec2<f32>, f32, f32, Uuid, bool)> = Vec::new();
    for linedef in &map.linedefs {
        let Some(Value::Source(PixelSource::TileId(tile_id))) =
            linedef.properties.get("terrain_source")
        else {
            continue;
        };
        let Some(start_vert) = map.vertices.iter().find(|v| v.id == linedef.start_vertex) else {
            continue;
        };
        let Some(end_vert) = map.vertices.iter().find(|v| v.id == linedef.end_vertex) else {
            continue;
        };
        let start = Vec2::new(start_vert.x, start_vert.y);
        let end = Vec2::new(end_vert.x, end_vert.y);
        let width = linedef
            .properties
            .get_float_default("terrain_width", 2.0)
            .max(0.0);
        let falloff = linedef
            .properties
            .get_float_default("terrain_tile_falloff", 1.0)
            .max(0.0);
        let smooth = linedef.properties.get_bool_default("terrain_smooth", false);
        road_tile_linedefs.push((start, end, width, falloff, *tile_id, smooth));
    }

    // Collect ridge tile definitions from sectors.
    let mut ridge_tile_sectors: Vec<(u32, f32, f32, Uuid)> = Vec::new();
    for sector in &map.sectors {
        if sector.properties.get_int_default("terrain_mode", 0) != 2 {
            continue;
        }
        let Some(Value::Source(PixelSource::TileId(tile_id))) =
            sector.properties.get("terrain_source")
        else {
            continue;
        };
        let plateau = sector
            .properties
            .get_float_default("ridge_plateau_width", 0.0)
            .max(0.0);
        let tile_falloff = sector
            .properties
            .get_float_default("terrain_tile_falloff", 1.0)
            .max(0.0);
        ridge_tile_sectors.push((sector.id, plateau, tile_falloff, *tile_id));
    }

    // Collect vertex hill tile definitions.
    let mut vertex_tile_controls: Vec<(Vec2<f32>, f32, f32, Uuid)> = Vec::new();
    for vertex in &map.vertices {
        if !vertex.properties.get_bool_default("terrain_control", false) {
            continue;
        }
        let Some(Value::Source(PixelSource::TileId(tile_id))) =
            vertex.properties.get("terrain_source")
        else {
            continue;
        };
        let smoothness = vertex
            .properties
            .get_float_default("smoothness", 1.0)
            .max(0.0);
        let radius = smoothness * 2.0;
        let tile_falloff = vertex
            .properties
            .get_float_default("terrain_tile_falloff", 1.0)
            .max(0.0);
        vertex_tile_controls.push((
            Vec2::new(vertex.x, vertex.y),
            radius,
            tile_falloff,
            *tile_id,
        ));
    }

    // Generate terrain meshes for this chunk (grouped by tile)
    let generated_meshes = generator.generate(map, chunk, assets, default_tile_id, tile_overrides);
    if let Some(meshes) = generated_meshes.as_ref() {
        // Process each mesh (one per tile)
        for (_mesh_idx, (tile_id, vertices, indices, uvs)) in meshes.iter().enumerate() {
            // Convert vertices from Vec3<f32> to [f32; 4] (homogeneous coordinates)
            let vertices_4d: Vec<[f32; 4]> =
                vertices.iter().map(|v| [v.x, v.y, v.z, 1.0]).collect();

            // Convert indices from Vec<u32> to Vec<(usize, usize, usize)>
            let mut indices_tuples: Vec<(usize, usize, usize)> = indices
                .chunks_exact(3)
                .map(|chunk| (chunk[0] as usize, chunk[1] as usize, chunk[2] as usize))
                .collect();

            // Fix winding order so terrain normals point upward (positive Y)
            let desired_normal = Vec3::new(0.0, 1.0, 0.0);
            mesh_fix_winding(&vertices_4d, &mut indices_tuples, desired_normal);

            // Group triangles by tile coordinates and check for blend overrides
            let mut tile_batches: FxHashMap<(i32, i32), Vec<(usize, usize, usize)>> =
                FxHashMap::default();

            for &triangle in &indices_tuples {
                let i0 = triangle.0;
                let i1 = triangle.1;
                let i2 = triangle.2;

                // Calculate triangle center from UVs (which are in world space)
                let center_u = (uvs[i0][0] + uvs[i1][0] + uvs[i2][0]) / 3.0;
                let center_v = (uvs[i0][1] + uvs[i1][1] + uvs[i2][1]) / 3.0;
                let center = Vec2::new(center_u, center_v);

                if builder_replace_polygons
                    .iter()
                    .any(|entry| entry.0.contains(center) && point_in_polygon_2d(center, &entry.1))
                {
                    continue;
                }

                // Get tile coordinates (can be negative)
                let tile_x = center_u.floor() as i32;
                let tile_z = center_v.floor() as i32;

                tile_batches
                    .entry((tile_x, tile_z))
                    .or_default()
                    .push(triangle);
            }

            // Process each tile batch
            for ((tile_x, tile_z), triangles) in tile_batches {
                let has_manual_tile_override = if let Some(overrides) = tile_overrides {
                    overrides
                        .get(&(tile_x, tile_z))
                        .and_then(|ps| ps.tile_from_tile_list(assets))
                        .is_some()
                } else {
                    false
                };
                let has_manual_blend_override = if let Some(overrides) = blend_overrides {
                    overrides
                        .get(&(tile_x, tile_z))
                        .and_then(|(_, ps)| ps.tile_from_tile_list(assets))
                        .is_some()
                } else {
                    false
                };
                let has_manual_override = has_manual_tile_override || has_manual_blend_override;

                // Road terrain should win over generic map blend overrides.
                let road_tile_id = *tile_id;
                let has_road_tile = road_tile_linedefs
                    .iter()
                    .any(|(_, _, width, _, tid, _)| *tid == road_tile_id && *width > 0.0)
                    && !has_manual_override;
                let has_smooth_road =
                    road_tile_linedefs
                        .iter()
                        .any(|(_, _, width, _, tid, smooth)| {
                            *tid == road_tile_id && *smooth && *width > 0.0
                        })
                        && !has_manual_override;
                let has_ridge_tile = ridge_tile_sectors
                    .iter()
                    .any(|(_, _, _, tid)| *tid == road_tile_id)
                    && !has_manual_override;
                let has_smooth_ridge = ridge_tile_sectors
                    .iter()
                    .any(|(_, _, tile_falloff, tid)| *tid == road_tile_id && *tile_falloff > 0.0)
                    && !has_manual_override;
                let has_vertex_tile = vertex_tile_controls
                    .iter()
                    .any(|(_, _, _, tid)| *tid == road_tile_id)
                    && !has_manual_override;
                let has_smooth_vertex = vertex_tile_controls
                    .iter()
                    .any(|(_, _, tile_falloff, tid)| *tid == road_tile_id && *tile_falloff > 0.0)
                    && !has_manual_override;

                // Automatic distance-based edge blend for smoothed road linedefs.
                if has_smooth_road {
                    let bg_tile = if let Some(overrides) = tile_overrides {
                        if let Some(ps) = overrides.get(&(tile_x, tile_z)) {
                            ps.tile_from_tile_list(assets)
                                .map(|t| t.id)
                                .unwrap_or(default_tile_id)
                        } else {
                            default_tile_id
                        }
                    } else {
                        default_tile_id
                    };

                    if bg_tile != road_tile_id {
                        let mut blend_weights = Vec::new();
                        let mut blended_verts = Vec::new();
                        let mut blended_uvs = Vec::new();
                        let mut blended_indices = Vec::new();

                        for &(i0, i1, i2) in &triangles {
                            let base_idx = blended_verts.len();
                            blended_verts.push(vertices_4d[i0]);
                            blended_verts.push(vertices_4d[i1]);
                            blended_verts.push(vertices_4d[i2]);

                            blended_uvs.push([uvs[i0][0], 1.0 - uvs[i0][1]]);
                            blended_uvs.push([uvs[i1][0], 1.0 - uvs[i1][1]]);
                            blended_uvs.push([uvs[i2][0], 1.0 - uvs[i2][1]]);

                            blended_indices.push((base_idx, base_idx + 1, base_idx + 2));

                            for &vi in &[i0, i1, i2] {
                                let p = Vec2::new(uvs[vi][0], uvs[vi][1]);
                                let mut w = 0.0f32;
                                for &(a, b, width, falloff, tile_id, line_smooth) in
                                    &road_tile_linedefs
                                {
                                    if !line_smooth || tile_id != road_tile_id || width <= 0.0 {
                                        continue;
                                    }
                                    let ab = b - a;
                                    let len_sq = ab.magnitude_squared();
                                    let dist = if len_sq < 1e-8 {
                                        (p - a).magnitude()
                                    } else {
                                        let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
                                        let q = a + ab * t;
                                        (p - q).magnitude()
                                    };

                                    let this_w = if dist <= width {
                                        1.0
                                    } else if falloff > 0.0 && dist <= width + falloff {
                                        1.0 - ((dist - width) / falloff)
                                    } else {
                                        0.0
                                    };
                                    if this_w > w {
                                        w = this_w;
                                    }
                                }
                                blend_weights.push(w.clamp(0.0, 1.0));
                            }
                        }
                        vmchunk.add_poly_3d_blended(
                            GeoId::Terrain(tile_x, tile_z),
                            bg_tile,
                            road_tile_id,
                            blended_verts,
                            blended_uvs,
                            blend_weights,
                            blended_indices,
                            0,
                            true,
                        );
                        continue;
                    }
                }

                // Distance-based edge blend for ridge terrain tiles.
                if has_smooth_ridge {
                    let bg_tile = if let Some(overrides) = tile_overrides {
                        if let Some(ps) = overrides.get(&(tile_x, tile_z)) {
                            ps.tile_from_tile_list(assets)
                                .map(|t| t.id)
                                .unwrap_or(default_tile_id)
                        } else {
                            default_tile_id
                        }
                    } else {
                        default_tile_id
                    };

                    if bg_tile != road_tile_id {
                        let mut blend_weights = Vec::new();
                        let mut blended_verts = Vec::new();
                        let mut blended_uvs = Vec::new();
                        let mut blended_indices = Vec::new();

                        for &(i0, i1, i2) in &triangles {
                            let base_idx = blended_verts.len();
                            blended_verts.push(vertices_4d[i0]);
                            blended_verts.push(vertices_4d[i1]);
                            blended_verts.push(vertices_4d[i2]);

                            blended_uvs.push([uvs[i0][0], 1.0 - uvs[i0][1]]);
                            blended_uvs.push([uvs[i1][0], 1.0 - uvs[i1][1]]);
                            blended_uvs.push([uvs[i2][0], 1.0 - uvs[i2][1]]);

                            blended_indices.push((base_idx, base_idx + 1, base_idx + 2));

                            for &vi in &[i0, i1, i2] {
                                let p = Vec2::new(uvs[vi][0], uvs[vi][1]);
                                let mut w = 0.0f32;
                                for &(sector_id, plateau, tile_falloff, tile_id) in
                                    &ridge_tile_sectors
                                {
                                    if tile_id != road_tile_id {
                                        continue;
                                    }
                                    let Some(sector) = map.find_sector(sector_id) else {
                                        continue;
                                    };
                                    let dist = distance_to_sector_edge_2d(p, sector, map);
                                    let this_w = if dist <= plateau {
                                        1.0
                                    } else if tile_falloff > 0.0 && dist <= plateau + tile_falloff {
                                        1.0 - ((dist - plateau) / tile_falloff)
                                    } else {
                                        0.0
                                    };
                                    if this_w > w {
                                        w = this_w;
                                    }
                                }
                                blend_weights.push(w.clamp(0.0, 1.0));
                            }
                        }

                        vmchunk.add_poly_3d_blended(
                            GeoId::Terrain(tile_x, tile_z),
                            bg_tile,
                            road_tile_id,
                            blended_verts,
                            blended_uvs,
                            blend_weights,
                            blended_indices,
                            0,
                            true,
                        );
                        continue;
                    }
                }

                // Distance-based edge blend for vertex hill terrain tiles.
                if has_smooth_vertex {
                    let bg_tile = if let Some(overrides) = tile_overrides {
                        if let Some(ps) = overrides.get(&(tile_x, tile_z)) {
                            ps.tile_from_tile_list(assets)
                                .map(|t| t.id)
                                .unwrap_or(default_tile_id)
                        } else {
                            default_tile_id
                        }
                    } else {
                        default_tile_id
                    };

                    if bg_tile != road_tile_id {
                        let mut blend_weights = Vec::new();
                        let mut blended_verts = Vec::new();
                        let mut blended_uvs = Vec::new();
                        let mut blended_indices = Vec::new();

                        for &(i0, i1, i2) in &triangles {
                            let base_idx = blended_verts.len();
                            blended_verts.push(vertices_4d[i0]);
                            blended_verts.push(vertices_4d[i1]);
                            blended_verts.push(vertices_4d[i2]);

                            blended_uvs.push([uvs[i0][0], 1.0 - uvs[i0][1]]);
                            blended_uvs.push([uvs[i1][0], 1.0 - uvs[i1][1]]);
                            blended_uvs.push([uvs[i2][0], 1.0 - uvs[i2][1]]);

                            blended_indices.push((base_idx, base_idx + 1, base_idx + 2));

                            for &vi in &[i0, i1, i2] {
                                let p = Vec2::new(uvs[vi][0], uvs[vi][1]);
                                let mut w = 0.0f32;
                                for &(center, radius, tile_falloff, tile_id) in
                                    &vertex_tile_controls
                                {
                                    if tile_id != road_tile_id {
                                        continue;
                                    }
                                    let dist = (p - center).magnitude();
                                    let this_w = if dist <= radius {
                                        1.0
                                    } else if tile_falloff > 0.0 && dist <= radius + tile_falloff {
                                        1.0 - ((dist - radius) / tile_falloff)
                                    } else {
                                        0.0
                                    };
                                    if this_w > w {
                                        w = this_w;
                                    }
                                }
                                blend_weights.push(w.clamp(0.0, 1.0));
                            }
                        }

                        vmchunk.add_poly_3d_blended(
                            GeoId::Terrain(tile_x, tile_z),
                            bg_tile,
                            road_tile_id,
                            blended_verts,
                            blended_uvs,
                            blend_weights,
                            blended_indices,
                            0,
                            true,
                        );
                        continue;
                    }
                }

                // Check if this tile has blend overrides
                if !has_road_tile && !has_ridge_tile && !has_vertex_tile {
                    if let Some(blend_map) = blend_overrides {
                        if let Some((preset, ps)) = blend_map.get(&(tile_x, tile_z)) {
                            if let Some(tile2) = ps.tile_from_tile_list(assets) {
                                let map_v = |v: f32| {
                                    if has_manual_override { v } else { 1.0 - v }
                                };
                                // Build blend weights for each vertex
                                let weights_4 = preset.weights();
                                let mut blend_weights = Vec::new();
                                let mut blended_verts = Vec::new();
                                let mut blended_uvs = Vec::new();
                                let mut blended_indices = Vec::new();

                                for &(i0, i1, i2) in &triangles {
                                    let base_idx = blended_verts.len();

                                    // Add vertices
                                    blended_verts.push(vertices_4d[i0]);
                                    blended_verts.push(vertices_4d[i1]);
                                    blended_verts.push(vertices_4d[i2]);

                                    // Add UVs (flipped V)
                                    blended_uvs.push([uvs[i0][0], map_v(uvs[i0][1])]);
                                    blended_uvs.push([uvs[i1][0], map_v(uvs[i1][1])]);
                                    blended_uvs.push([uvs[i2][0], map_v(uvs[i2][1])]);

                                    // Add triangle indices
                                    blended_indices.push((base_idx, base_idx + 1, base_idx + 2));

                                    // Calculate blend weights for each vertex
                                    for &vi in &[i0, i1, i2] {
                                        let u = (uvs[vi][0] - tile_x as f32).clamp(0.0, 1.0);
                                        let v = (uvs[vi][1] - tile_z as f32).clamp(0.0, 1.0);

                                        // Bilinear interpolation: TL, TR, BR, BL
                                        let top = weights_4[0] * (1.0 - u) + weights_4[1] * u;
                                        let bottom = weights_4[3] * (1.0 - u) + weights_4[2] * u;
                                        let weight = top * (1.0 - v) + bottom * v;

                                        blend_weights.push(weight);
                                    }
                                }

                                // Add blended poly
                                vmchunk.add_poly_3d_blended(
                                    GeoId::Terrain(tile_x, tile_z),
                                    *tile_id,
                                    tile2.id,
                                    blended_verts,
                                    blended_uvs,
                                    blend_weights,
                                    blended_indices,
                                    0,
                                    true,
                                );
                                continue;
                            }
                        }
                    }
                }

                // No blend override - add as regular poly
                let map_v = |v: f32| {
                    if has_manual_override { v } else { 1.0 - v }
                };
                for &triangle in &triangles {
                    let i0 = triangle.0;
                    let i1 = triangle.1;
                    let i2 = triangle.2;

                    let tri_vertices = vec![vertices_4d[i0], vertices_4d[i1], vertices_4d[i2]];
                    let tri_uvs = vec![
                        [uvs[i0][0], map_v(uvs[i0][1])],
                        [uvs[i1][0], map_v(uvs[i1][1])],
                        [uvs[i2][0], map_v(uvs[i2][1])],
                    ];
                    let tri_indices = vec![(0, 1, 2)];

                    vmchunk.add_poly_3d(
                        GeoId::Terrain(tile_x, tile_z),
                        *tile_id,
                        tri_vertices,
                        tri_uvs,
                        tri_indices,
                        0,
                        true,
                    );
                }
            }
        }
    }

    // Optional ridge water surfaces.
    // Build water from generated terrain triangles that lie within the same ridge
    // influence envelope (plateau + falloff) used by terrain height generation.
    for sector in &map.sectors {
        if sector.properties.get_int_default("terrain_mode", 0) != 2 {
            continue;
        }
        if !sector
            .properties
            .get_bool_default("ridge_water_enabled", false)
        {
            continue;
        }

        let Some(Value::Source(PixelSource::TileId(water_tile_id))) =
            sector.properties.get("ridge_water_source")
        else {
            continue;
        };

        let influence = sector
            .properties
            .get_float_default("ridge_plateau_width", 0.0)
            .max(0.0)
            + sector
                .properties
                .get_float_default("ridge_falloff_distance", 0.0)
                .max(0.0);

        let mut expanded_bbox = sector.bounding_box(map);
        if influence > 0.0 {
            expanded_bbox.expand(Vec2::broadcast(influence * 2.0));
        }
        if !expanded_bbox.intersects(&chunk.bbox) {
            continue;
        }

        let water_y = sector.properties.get_float_default("ridge_height", 0.0)
            + sector
                .properties
                .get_float_default("ridge_water_level", 0.0);

        let mut water_vertices: Vec<[f32; 4]> = Vec::new();
        let mut water_uvs: Vec<[f32; 2]> = Vec::new();
        let mut water_indices: Vec<(usize, usize, usize)> = Vec::new();

        if let Some(meshes) = generated_meshes.as_ref() {
            for (_tile_id, mesh_vertices, mesh_indices, _mesh_uvs) in meshes {
                for tri in mesh_indices.chunks_exact(3) {
                    let i0 = tri[0] as usize;
                    let i1 = tri[1] as usize;
                    let i2 = tri[2] as usize;

                    let p0 = Vec2::new(mesh_vertices[i0].x, mesh_vertices[i0].z);
                    let p1 = Vec2::new(mesh_vertices[i1].x, mesh_vertices[i1].z);
                    let p2 = Vec2::new(mesh_vertices[i2].x, mesh_vertices[i2].z);
                    let center = (p0 + p1 + p2) / 3.0;

                    if distance_to_sector_edge_2d(center, sector, map) > influence {
                        continue;
                    }

                    let base = water_vertices.len();
                    for p in [p0, p1, p2] {
                        water_vertices.push([p.x, water_y, p.y, 1.0]);
                        water_uvs.push([p.x, 1.0 - p.y]);
                    }
                    water_indices.push((base, base + 1, base + 2));
                }
            }
        }

        if !water_indices.is_empty() {
            vmchunk.add_poly_3d(
                GeoId::Sector(sector.id),
                *water_tile_id,
                water_vertices,
                water_uvs,
                water_indices,
                0,
                true,
            );
        }
    }
}

// --- Relief/recess pipeline helpers ---
/// Classify profile loops: only true holes (cutouts and through-recesses) are subtracted from the base;
/// shallow recesses and reliefs are handled as feature meshes.
fn split_loops_for_base<'a>(
    surface: &crate::Surface,
    map: &'a Map,
    _outer: &'a ProfileLoop,
    holes: &'a [ProfileLoop],
    extrude_depth_abs: f32,
) -> (Vec<&'a ProfileLoop>, Vec<&'a ProfileLoop>) {
    let mut base_holes = Vec::new();
    let mut feature_loops = Vec::new();
    let eps = 1e-5f32;
    for h in holes {
        match h.op {
            LoopOp::None => {
                let builder_profile_sector = h.origin_profile_sector.and_then(|origin| {
                    surface.profile.and_then(|profile_id| {
                        map.profiles
                            .get(&profile_id)
                            .and_then(|profile_map| profile_map.find_sector(origin))
                    })
                });
                let is_builder_feature = builder_profile_sector
                    .map(|ps| {
                        !ps.properties
                            .get_str_default("builder_graph_data", String::new())
                            .trim()
                            .is_empty()
                            && ps
                                .properties
                                .get_str_default("builder_graph_target", "sector".to_string())
                                == "sector"
                    })
                    .unwrap_or(false);
                let builder_hide_host = builder_profile_sector
                    .map(|ps| ps.properties.get_bool_default("builder_hide_host", false))
                    .unwrap_or(false);
                let builder_replace_surface = builder_profile_sector
                    .map(|ps| {
                        ps.properties
                            .get_str_default("builder_surface_mode", String::new())
                            == "replace"
                    })
                    .unwrap_or(false);
                if is_builder_feature {
                    if builder_replace_surface && !builder_hide_host {
                        base_holes.push(h);
                    }
                    feature_loops.push(h);
                } else {
                    // Pure cutout -> subtract from base; no feature meshes needed
                    base_holes.push(h);
                }
            }
            LoopOp::Recess { .. } => {
                if extrude_depth_abs <= eps {
                    // Zero-thickness surface: we need a visible hole in the base cap
                    // *and* a recessed pocket (cap only, no jamb). Put it in **both** buckets.
                    base_holes.push(h); // subtract from base
                    feature_loops.push(h); // build recess cap
                } else {
                    // On extruded surfaces, always build the recess as a feature (cap + jamb)
                    // The recess creates a visible pocket regardless of depth
                    feature_loops.push(h);
                }
            }
            LoopOp::Relief { .. } => {
                // Relief never subtracts from the base; purely additive feature
                feature_loops.push(h);
            }
            LoopOp::Billboard { .. } => {
                // Billboard cuts a hole in the base and creates a billboard quad
                base_holes.push(h); // subtract from base
                feature_loops.push(h); // build billboard geometry
            }
            LoopOp::Window { .. } => {
                // Window cuts a hole in the base and adds static frame/glass geometry.
                base_holes.push(h);
                feature_loops.push(h);
            }
        }
    }
    (base_holes, feature_loops)
}

/// Read profile loops (outer + holes) for a surface from the profile map, using profile sectors.
fn read_profile_loops(
    surface: &crate::Surface,
    _sector: &Sector,
    map: &Map,
) -> Option<(ProfileLoop, Vec<ProfileLoop>)> {
    // 1) OUTER from the host sector geometry (projected to UV)
    let outer_path = match project_sector_to_uv(surface, _sector, map) {
        Some(p) if p.len() >= 3 => p,
        _ => return None,
    };

    // Read outer-loop op from the host sector if present
    let outer_op_code = _sector.properties.get_int_default("profile_outer_op", 0);
    let outer_op = match outer_op_code {
        1 => LoopOp::Relief {
            height: _sector
                .properties
                .get_float_default("profile_outer_height", 0.0),
        },
        2 => LoopOp::Recess {
            depth: _sector
                .properties
                .get_float_default("profile_outer_depth", 0.0),
        },
        _ => LoopOp::None,
    };
    let outer = ProfileLoop {
        path: outer_path,
        op: outer_op,
        origin_profile_sector: None,
    };

    // 2) HOLES from the profile map for this surface
    let mut holes: Vec<ProfileLoop> = Vec::new();
    if let Some(profile_id) = surface.profile {
        if let Some(profile_map) = map.profiles.get(&profile_id) {
            for ps in profile_map.sectors.iter() {
                // Build UV path from the profile sector boundary (2D profile space).
                // Editor convention: -Y is up → flip Y here.
                // Also collect vertex heights (z-component) for terrain
                let mut uv_path: Vec<vek::Vec2<f32>> = Vec::new();
                let mut heights: Vec<f32> = Vec::new();
                for &ld_id in ps.linedefs.iter() {
                    let ld = match profile_map.find_linedef(ld_id) {
                        Some(x) => x,
                        None => continue,
                    };
                    let v = match profile_map
                        .vertices
                        .iter()
                        .find(|vtx| vtx.id == ld.start_vertex)
                    {
                        Some(x) => x,
                        None => continue,
                    };
                    let uv = vek::Vec2::new(v.x, -v.y);
                    if uv_path.last().map(|p| (p.x, p.y)) != Some((uv.x, uv.y)) {
                        uv_path.push(uv);
                        heights.push(v.z); // Collect z-component as height
                    }
                }
                if uv_path.len() < 3 {
                    continue;
                }
                if (uv_path[0] - *uv_path.last().unwrap()).magnitude_squared() < 1e-8 {
                    uv_path.pop();
                }

                // Op comes from the profile sector itself
                let op_code = ps.properties.get_int_default("profile_op", 0);

                // Read unified property with backward compatibility fallbacks
                // Priority: profile_amount → (profile_height OR profile_depth depending on op) → 0.0
                let amount = ps.properties.get_float_default("profile_amount", f32::NAN);
                let parse_tile_id = |key: &str| -> Option<Uuid> {
                    if let Some(Value::Str(tile_str)) = ps.properties.get(key) {
                        Uuid::from_str(tile_str).ok()
                    } else if let Some(Value::Id(id)) = ps.properties.get(key) {
                        Some(*id)
                    } else {
                        None
                    }
                };
                let parse_tile_from_source = |key: &str| -> Option<Uuid> {
                    if let Some(Value::Source(PixelSource::TileId(id))) = ps.properties.get(key) {
                        Some(*id)
                    } else {
                        None
                    }
                };

                let op = match op_code {
                    1 => {
                        // Relief: prefer profile_amount, fallback to profile_height
                        let height = if amount.is_nan() {
                            ps.properties.get_float_default("profile_height", 0.0)
                        } else {
                            amount
                        };
                        LoopOp::Relief { height }
                    }
                    2 => {
                        // Recess: prefer profile_amount, fallback to profile_depth
                        let depth = if amount.is_nan() {
                            ps.properties.get_float_default("profile_depth", 0.0)
                        } else {
                            amount
                        };
                        LoopOp::Recess { depth }
                    }
                    3 => {
                        // Billboard: gate/door with optional animation
                        let inset = if amount.is_nan() {
                            ps.properties.get_float_default("profile_inset", 0.0)
                        } else {
                            amount
                        };
                        // Read tile_id as UUID string or Id
                        let tile_id = if let Some(Value::Str(tile_str)) =
                            ps.properties.get("billboard_tile_id")
                        {
                            Uuid::from_str(tile_str).ok()
                        } else if let Some(Value::Id(id)) = ps.properties.get("billboard_tile_id") {
                            Some(*id)
                        } else {
                            None
                        };

                        let anim_code = ps.properties.get_int_default("billboard_animation", 0);
                        let animation = match anim_code {
                            1 => BillboardAnimation::OpenUp,
                            2 => BillboardAnimation::OpenRight,
                            3 => BillboardAnimation::OpenDown,
                            4 => BillboardAnimation::OpenLeft,
                            5 => BillboardAnimation::Fade,
                            _ => BillboardAnimation::None,
                        };

                        LoopOp::Billboard {
                            tile_id,
                            animation,
                            inset,
                        }
                    }
                    4 => {
                        // Static window: hole + generated frame/glass meshes.
                        let inset = if amount.is_nan() {
                            ps.properties.get_float_default("profile_inset", 0.0)
                        } else {
                            amount
                        };
                        let frame_tile_id = parse_tile_from_source("window_frame_source")
                            .or_else(|| parse_tile_id("window_frame_tile_id"));
                        let glass_tile_id = parse_tile_from_source("window_glass_source")
                            .or_else(|| parse_tile_id("window_glass_tile_id"));
                        LoopOp::Window {
                            frame_tile_id,
                            glass_tile_id,
                            inset,
                        }
                    }
                    _ => LoopOp::None,
                };

                holes.push(ProfileLoop {
                    path: uv_path,
                    op,
                    origin_profile_sector: Some(ps.id as u32),
                });
            }
        }
    }

    Some((outer, holes))
}

fn ensure_ccw(poly: &mut [vek::Vec2<f32>]) {
    if polygon_area(poly) < 0.0 {
        poly.reverse();
    }
}
fn ensure_cw(poly: &mut [vek::Vec2<f32>]) {
    if polygon_area(poly) > 0.0 {
        poly.reverse();
    }
}

fn point_in_polygon_2d(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let pi = polygon[i];
        let pj = polygon[j];
        let intersects = ((pi.y > point.y) != (pj.y > point.y))
            && (point.x
                < (pj.x - pi.x) * (point.y - pi.y) / ((pj.y - pi.y).abs().max(1e-6)) + pi.x);
        if intersects {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Triangulate an outer polygon with holes in UV space using earcutr.
/// Returns (verts_uv, indices) where verts_uv = [outer..., hole0..., hole1..., ...]
fn earcut_with_holes(
    outer: &mut Vec<vek::Vec2<f32>>,
    holes: &mut [Vec<vek::Vec2<f32>>],
) -> Option<(Vec<[f32; 2]>, Vec<(usize, usize, usize)>)> {
    // Winding for earcut: outer CW, holes CCW (works with our flipped-Y editor space)
    ensure_cw(outer);
    for h in holes.iter_mut() {
        ensure_ccw(h);
    }

    // Flatten vertices: outer then each hole
    let mut verts_uv: Vec<[f32; 2]> = Vec::new();
    let mut holes_idx: Vec<usize> = Vec::new();

    for p in outer.iter() {
        verts_uv.push([p.x, p.y]);
    }
    let mut acc = outer.len();
    for h in holes.iter() {
        holes_idx.push(acc);
        acc += h.len();
        for p in h.iter() {
            verts_uv.push([p.x, p.y]);
        }
    }

    // Validation: check for duplicate vertices
    for i in 0..verts_uv.len() {
        for j in (i + 1)..verts_uv.len() {
            if (verts_uv[i][0] - verts_uv[j][0]).abs() < 0.0001
                && (verts_uv[i][1] - verts_uv[j][1]).abs() < 0.0001
            {
                println!(
                    "[EARCUT WARNING] Duplicate vertices detected at index {} and {}: [{}, {}]",
                    i, j, verts_uv[i][0], verts_uv[i][1]
                );
                return None;
            }
        }
    }

    // Build f64 flat list
    let flattened: Vec<f64> = verts_uv
        .iter()
        .flat_map(|v| [v[0] as f64, v[1] as f64])
        .collect();

    // Run earcut with panic protection
    let idx_result = std::panic::catch_unwind(|| earcutr::earcut(&flattened, &holes_idx, 2));

    let idx = match idx_result {
        Ok(Ok(indices)) => indices,
        Ok(Err(e)) => {
            println!("[EARCUT ERROR] Earcut failed: {:?}", e);
            println!("  outer vertices: {}", outer.len());
            println!("  holes: {}", holes.len());
            println!("  total vertices: {}", verts_uv.len());
            return None;
        }
        Err(_) => {
            println!("[EARCUT PANIC] Earcut panicked!");
            println!("  outer vertices: {}", outer.len());
            println!("  holes: {}", holes.len());
            println!("  total vertices: {}", verts_uv.len());
            for (i, v) in verts_uv.iter().enumerate() {
                println!("    vertex[{}] = [{}, {}]", i, v[0], v[1]);
            }
            return None;
        }
    };

    let indices: Vec<(usize, usize, usize)> =
        idx.chunks_exact(3).map(|c| (c[2], c[1], c[0])).collect();

    Some((verts_uv, indices))
}

fn fix_winding(
    world_vertices: &[[f32; 4]],
    indices: &mut Vec<(usize, usize, usize)>,
    desired_normal: vek::Vec3<f32>,
) {
    if indices.is_empty() {
        return;
    }
    // Average a few triangle normals (robust if the first is degenerate)
    let mut acc = vek::Vec3::zero();
    for (a, b, c) in indices.iter().take(8) {
        let va = vek::Vec3::new(
            world_vertices[*a][0],
            world_vertices[*a][1],
            world_vertices[*a][2],
        );
        let vb = vek::Vec3::new(
            world_vertices[*b][0],
            world_vertices[*b][1],
            world_vertices[*b][2],
        );
        let vc = vek::Vec3::new(
            world_vertices[*c][0],
            world_vertices[*c][1],
            world_vertices[*c][2],
        );
        acc += (vb - va).cross(vc - va);
    }
    let len: f32 = acc.magnitude();
    if len < 1e-8 {
        return;
    }
    let face_n: Vec3<f32> = acc / len;
    if face_n.dot(desired_normal) < 0.0 {
        for tri in indices.iter_mut() {
            core::mem::swap(&mut tri.1, &mut tri.2);
        }
    }
}

fn poly_winding(poly: &[vek::Vec2<f32>]) -> &'static str {
    if polygon_area(poly) > 0.0 {
        "CCW"
    } else {
        "CW"
    }
}

fn dump_poly(label: &str, poly: &[vek::Vec2<f32>]) {
    println!(
        "[DBG] {}: len={}, area={:.6}, winding={}",
        label,
        poly.len(),
        polygon_area(poly).abs(),
        poly_winding(poly)
    );
    for (i, p) in poly.iter().enumerate().take(12) {
        println!("    [{}] ({:.4}, {:.4})", i, p.x, p.y);
    }
    if poly.len() > 12 {
        println!("    ... ({} points total)", poly.len());
    }
}

// --- Profile geometry helpers ---
/// Project a sector boundary (start-vertex ordered) into a surface's UV plane.
fn project_sector_to_uv(
    surface: &crate::Surface,
    sector: &Sector,
    map: &Map,
) -> Option<Vec<vek::Vec2<f32>>> {
    let mut uv: Vec<vek::Vec2<f32>> = Vec::new();
    for &ld_id in sector.linedefs.iter() {
        let ld = map.find_linedef(ld_id)?;
        let v = map.get_vertex_3d(ld.start_vertex)?; // world xyz with Y up
        let p = vek::Vec3::new(v.x, v.y, v.z);
        let q = surface.world_to_uv(p);
        if uv.last().map(|w| (w.x, w.y)) != Some((q.x, q.y)) {
            uv.push(q);
        }
    }
    if uv.len() < 3 {
        return None;
    }
    // drop duplicate last==first
    if (uv[0] - *uv.last().unwrap()).magnitude_squared() < 1e-8 {
        uv.pop();
    }
    Some(uv)
}

fn polygon_area(poly: &[vek::Vec2<f32>]) -> f32 {
    let n = poly.len();
    if n < 3 {
        return 0.0;
    }
    let mut a2 = 0.0f32; // 2*A
    for i in 0..n {
        let p = poly[i];
        let q = poly[(i + 1) % n];
        a2 += p.x * q.y - q.x * p.y;
    }
    0.5 * a2
}

fn feature_pixelsource(
    surface: &crate::Surface,
    map: &Map,
    host_sector: &Sector,
    loop_origin: Option<u32>,
    key: &str,
) -> Option<Value> {
    // Unified property lookup with clean fallback chain
    // Priority: profile sector specific → profile sector generic → host sector specific → host sector fallback → host sector generic

    // 1) Check profile sector first (if this feature came from a profile)
    if let (Some(profile_id), Some(origin_id)) = (surface.profile, loop_origin) {
        if let Some(profile_map) = map.profiles.get(&profile_id) {
            if let Some(ps) = profile_map.find_sector(origin_id) {
                // 1a) Exact key on profile sector (e.g., "cap_source", "jamb_source")
                if let Some(v) = ps.properties.get(key) {
                    return Some(v.clone());
                }
                // 1b) Generic 'source' on profile sector
                if let Some(v) = ps.properties.get("source") {
                    return Some(v.clone());
                }
            }
        }
    }

    // 2) Check host sector
    // 2a) Exact key on host (e.g., "cap_source", "jamb_source")
    if let Some(v) = host_sector.properties.get(key) {
        return Some(v.clone());
    }

    // 2b) Fallback: jamb_source → side_source (for backward compatibility)
    if key == "jamb_source" {
        if let Some(v) = host_sector.properties.get("side_source") {
            return Some(v.clone());
        }
    }

    // 2c) Generic 'source' on host sector
    host_sector.properties.get("source").cloned()
}

fn feature_has_explicit_source(
    surface: &crate::Surface,
    map: &Map,
    host_sector: &Sector,
    loop_origin: Option<u32>,
    key: &str,
) -> bool {
    if let (Some(profile_id), Some(origin_id)) = (surface.profile, loop_origin)
        && let Some(profile_map) = map.profiles.get(&profile_id)
        && let Some(ps) = profile_map.find_sector(origin_id)
        && ps.properties.get(key).is_some()
    {
        return true;
    }
    host_sector.properties.get(key).is_some()
}

fn feature_profile_target(
    surface: &crate::Surface,
    map: &Map,
    sector: &Sector,
    loop_origin: Option<u32>,
) -> i32 {
    if let Some(origin) = loop_origin
        && let Some(profile_id) = surface.profile
        && let Some(profile_map) = map.profiles.get(&profile_id)
        && let Some(ps) = profile_map.find_sector(origin)
    {
        return ps.properties.get_int_default("profile_target", 0);
    }
    sector.properties.get_int_default("profile_target", 0)
}

fn feature_profile_bool(
    surface: &crate::Surface,
    map: &Map,
    sector: &Sector,
    loop_origin: Option<u32>,
    key: &str,
    default: bool,
) -> bool {
    let mut value = sector.properties.get_bool_default(key, default);
    if let Some(origin) = loop_origin
        && let Some(profile_id) = surface.profile
        && let Some(profile_map) = map.profiles.get(&profile_id)
        && let Some(ps) = profile_map.find_sector(origin)
    {
        value = ps.properties.get_bool_default(key, value);
    }
    value
}

fn feature_profile_int(
    surface: &crate::Surface,
    map: &Map,
    sector: &Sector,
    loop_origin: Option<u32>,
    key: &str,
    default: i32,
) -> i32 {
    let mut value = sector.properties.get_int_default(key, default);
    if let Some(origin) = loop_origin
        && let Some(profile_id) = surface.profile
        && let Some(profile_map) = map.profiles.get(&profile_id)
        && let Some(ps) = profile_map.find_sector(origin)
    {
        value = ps.properties.get_int_default(key, value);
    }
    value
}

fn feature_profile_float(
    surface: &crate::Surface,
    map: &Map,
    sector: &Sector,
    loop_origin: Option<u32>,
    key: &str,
    default: f32,
) -> f32 {
    let mut value = sector.properties.get_float_default(key, default);
    if let Some(origin) = loop_origin
        && let Some(profile_id) = surface.profile
        && let Some(profile_map) = map.profiles.get(&profile_id)
        && let Some(ps) = profile_map.find_sector(origin)
    {
        value = ps.properties.get_float_default(key, value);
    }
    value
}

fn emit_feature_meshes(
    surface: &crate::Surface,
    map: &Map,
    sector: &Sector,
    chunk: &mut Chunk,
    vmchunk: &mut scenevm::Chunk,
    assets: &Assets,
    loop_origin: Option<u32>,
    profile_target: i32,
    meshes: &[GeneratedMesh],
    cap_present: bool,
    source_key_override: Option<&str>,
) {
    for (mesh_idx, mesh) in meshes.iter().enumerate() {
        let is_cap = mesh_idx == 0 && cap_present;
        let mut n = surface.plane.normal;
        let ln = n.magnitude();
        if ln > 1e-6 {
            n /= ln;
        } else {
            n = vek::Vec3::unit_y();
        }

        let mut mesh_indices = mesh.indices.clone();
        if is_cap {
            let desired_n = if profile_target == 0 { -n } else { n };
            mesh_fix_winding(&mesh.vertices, &mut mesh_indices, desired_n);
        } else {
            mesh_fix_winding(&mesh.vertices, &mut mesh_indices, n);
        }

        let mut batch = Batch3D::new(
            mesh.vertices.clone(),
            mesh_indices.clone(),
            mesh.uvs.clone(),
        )
        .repeat_mode(RepeatMode::RepeatXY)
        .geometry_source(GeometrySource::Sector(sector.id));

        let source_key = if let Some(override_key) = source_key_override {
            override_key
        } else if is_cap {
            "cap_source"
        } else {
            "jamb_source"
        };
        let mut added = false;
        if let Some(Value::Source(pixelsource)) =
            feature_pixelsource(surface, map, sector, loop_origin, source_key)
        {
            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                vmchunk.add_poly_3d(
                    GeoId::Sector(sector.id),
                    tile.id,
                    mesh.vertices.clone(),
                    mesh.uvs.clone(),
                    mesh_indices.clone(),
                    0,
                    true,
                );
                added = true;
                if let Some(tex) = assets.tile_index(&tile.id) {
                    batch.source = PixelSource::StaticTileIndex(tex);
                }
            }
        }

        if !added {
            if let Ok(default_id) = Uuid::from_str(DEFAULT_TILE_ID)
                && let Some(tex) = assets.tile_index(&default_id)
            {
                batch.source = PixelSource::StaticTileIndex(tex);
            } else {
                // Scene VM always has a checker tile for DEFAULT_TILE_ID, but the software batch
                // path only renders visible output when the batch source is concrete. Keep Builder
                // fallback geometry visible even when the checker tile is not present in assets.
                batch.source = PixelSource::Pixel([180, 180, 180, 255]);
            }
            vmchunk.add_poly_3d(
                GeoId::Sector(sector.id),
                Uuid::from_str(DEFAULT_TILE_ID).unwrap(),
                mesh.vertices.clone(),
                mesh.uvs.clone(),
                mesh_indices,
                0,
                true,
            );
        }

        chunk.batches3d.push(batch);
    }
}

fn emit_builder_sector_meshes(
    graph_text: &str,
    min_uv: Vec2<f32>,
    max_uv: Vec2<f32>,
    base_extrusion: f32,
    _direction: f32,
    surface: &crate::Surface,
    map: &Map,
    sector: &Sector,
    _chunk: &mut Chunk,
    vmchunk: &mut scenevm::Chunk,
    assets: &Assets,
    loop_origin: Option<u32>,
    _profile_target: i32,
) -> bool {
    let builder_geo_id = scenevm::GeoId::Unknown(0xB100_0000u32 ^ sector.id);
    let Ok(graph) = BuilderDocument::from_text(graph_text) else {
        return false;
    };
    let Ok(assembly) = graph.evaluate() else {
        return false;
    };
    if assembly.primitives.is_empty() {
        return false;
    }

    let fallback_source = feature_pixelsource(surface, map, sector, loop_origin, "source");

    let center_uv = Vec2::new((min_uv.x + max_uv.x) * 0.5, (min_uv.y + max_uv.y) * 0.5);
    let sector_size = max_uv - min_uv;
    let along = surface.edit_uv.right.normalized();
    let mut upward = surface.frame.normal.normalized();
    if upward.y < 0.0 {
        upward = -upward;
    }
    let mut outward = along.cross(upward);
    if outward.magnitude() <= 1e-5 {
        outward = surface.edit_uv.up;
    }
    if outward.y < 0.0 {
        outward = -outward;
    }
    outward = outward.normalized();

    let mut mesh_vertices: Vec<[f32; 4]> = Vec::new();
    let mut mesh_uvs: Vec<[f32; 2]> = Vec::new();
    let mut mesh_indices: Vec<(usize, usize, usize)> = Vec::new();

    let host_origin = surface.uv_to_world(center_uv) + upward * base_extrusion;

    for primitive in &assembly.primitives {
        match primitive {
            BuilderPrimitive::Box {
                size,
                transform,
                material_slot,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                host_scale_z_normalized,
            } => {
                let sx = if *host_scale_x_normalized {
                    transform.scale.x * sector_size.x
                } else {
                    transform.scale.x
                };
                let sz = if *host_scale_z_normalized {
                    transform.scale.z * sector_size.y
                } else {
                    transform.scale.z
                };
                let sy = if *host_scale_y_normalized {
                    size.y * transform.scale.y * sector_size.y
                } else {
                    size.y * transform.scale.y
                };
                let scaled = Vec3::new(size.x * sx, sy, size.z * sz);
                let offset_uv = Vec2::new(
                    if *host_position_normalized {
                        transform.translation.x * sector_size.x
                    } else {
                        transform.translation.x
                    },
                    if *host_position_normalized {
                        transform.translation.z * sector_size.y
                    } else {
                        transform.translation.z
                    },
                );
                let base_world =
                    surface.uv_to_world(center_uv + offset_uv) + upward * base_extrusion;
                let center = base_world
                    + upward
                        * ((if *host_position_y_normalized {
                            transform.translation.y * sector_size.y
                        } else {
                            transform.translation.y
                        }) + scaled.y * 0.5);
                add_builder_box_mesh(
                    &mut mesh_vertices,
                    &mut mesh_uvs,
                    &mut mesh_indices,
                    center,
                    scaled,
                    transform.rotation_x,
                    transform.rotation_y,
                    along,
                    upward,
                    outward,
                );
                let tile_id = resolve_builder_material_tile_id(
                    &sector.properties,
                    fallback_source.as_ref(),
                    material_slot.as_deref(),
                    assets,
                );
                vmchunk.add_poly_3d(
                    builder_geo_id,
                    tile_id,
                    mesh_vertices.clone(),
                    mesh_uvs.clone(),
                    mesh_indices.clone(),
                    0,
                    true,
                );
                mesh_vertices.clear();
                mesh_uvs.clear();
                mesh_indices.clear();
            }
            BuilderPrimitive::Cylinder {
                length,
                radius,
                transform,
                material_slot,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                host_scale_z_normalized: _,
            } => {
                let scaled_length = if *host_scale_y_normalized {
                    *length * transform.scale.y * sector_size.y
                } else {
                    *length * transform.scale.y
                };
                let scaled_radius = *radius
                    * if *host_scale_x_normalized {
                        transform.scale.z * sector_size.x
                    } else {
                        transform.scale.z
                    };
                let offset_uv = Vec2::new(
                    if *host_position_normalized {
                        transform.translation.x * sector_size.x
                    } else {
                        transform.translation.x
                    },
                    if *host_position_normalized {
                        transform.translation.z * sector_size.y
                    } else {
                        transform.translation.z
                    },
                );
                let base_world =
                    surface.uv_to_world(center_uv + offset_uv) + upward * base_extrusion;
                let center = base_world
                    + upward
                        * ((if *host_position_y_normalized {
                            transform.translation.y * sector_size.y
                        } else {
                            transform.translation.y
                        }) + scaled_length * 0.5);
                add_builder_cylinder_mesh(
                    &mut mesh_vertices,
                    &mut mesh_uvs,
                    &mut mesh_indices,
                    center,
                    scaled_length,
                    scaled_radius,
                    transform.rotation_x,
                    transform.rotation_y,
                    along,
                    upward,
                    outward,
                );
                let tile_id = resolve_builder_material_tile_id(
                    &sector.properties,
                    fallback_source.as_ref(),
                    material_slot.as_deref(),
                    assets,
                );
                vmchunk.add_poly_3d(
                    builder_geo_id,
                    tile_id,
                    mesh_vertices.clone(),
                    mesh_uvs.clone(),
                    mesh_indices.clone(),
                    0,
                    true,
                );
                mesh_vertices.clear();
                mesh_uvs.clear();
                mesh_indices.clear();
            }
        }
    }
    emit_builder_attached_item_graphs(
        &assembly,
        &sector.properties,
        sector_size,
        host_origin,
        along,
        upward,
        outward,
        builder_geo_id,
        assets,
        vmchunk,
    );
    true
}

/// Process a feature loop using the SurfaceAction trait system
/// Returns meshes (cap and sides) for the feature
fn process_feature_loop_with_action(
    surface: &crate::Surface,
    map: &Map,
    sector: &Sector,
    chunk: &mut Chunk,
    vmchunk: &mut scenevm::Chunk,
    assets: &Assets,
    feature_loop: &ProfileLoop,
) -> Option<()> {
    // Special handling for billboards - use DynamicObject instead of mesh
    if let LoopOp::Billboard {
        tile_id,
        animation,
        inset,
    } = &feature_loop.op
    {
        // Calculate billboard bounds from hole polygon
        let mut min_uv = feature_loop.path[0];
        let mut max_uv = feature_loop.path[0];
        for uv in &feature_loop.path {
            min_uv.x = min_uv.x.min(uv.x);
            min_uv.y = min_uv.y.min(uv.y);
            max_uv.x = max_uv.x.max(uv.x);
            max_uv.y = max_uv.y.max(uv.y);
        }

        // Billboard center is the center of the bounding box
        let center_uv = (min_uv + max_uv) * 0.5;

        // Convert to world space at the inset depth
        let mut center_world = surface.uvw_to_world(center_uv, *inset);

        // Adjust Y position: billboard should be centered at half its height above ground
        let size_uv = max_uv - min_uv;
        let height = size_uv.y * surface.edit_uv.scale;
        center_world.y = height * 0.5;

        let size = size_uv.magnitude() * surface.edit_uv.scale;

        // Get tile using feature_pixelsource (same approach as other features)
        let billboard_tile_id = if let Some(Value::Source(pixelsource)) = feature_pixelsource(
            surface,
            map,
            sector,
            feature_loop.origin_profile_sector,
            "billboard_source",
        ) {
            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                tile.id
            } else {
                Uuid::from_str(DEFAULT_TILE_ID).unwrap()
            }
        } else {
            // Fallback: use tile_id if specified directly
            if let Some(tid) = tile_id {
                if assets.tiles.contains_key(tid) {
                    *tid
                } else {
                    Uuid::from_str(DEFAULT_TILE_ID).unwrap()
                }
            } else {
                Uuid::from_str(DEFAULT_TILE_ID).unwrap()
            }
        };

        // GeoId for the billboard
        let geo_id = if let Some(origin) = feature_loop.origin_profile_sector {
            GeoId::Hole(sector.id, origin)
        } else {
            GeoId::Sector(sector.id)
        };

        // Read repeat mode from profile sector properties
        let repeat_mode = if let Some(origin) = feature_loop.origin_profile_sector {
            if let Some(profile_id) = surface.profile {
                if let Some(profile_map) = map.profiles.get(&profile_id) {
                    if let Some(ps) = profile_map.find_sector(origin) {
                        let mode = ps.properties.get_int_default("billboard_repeat_mode", 1);
                        match mode {
                            1 => scenevm::RepeatMode::Scale,
                            _ => scenevm::RepeatMode::Repeat,
                        }
                    } else {
                        scenevm::RepeatMode::Repeat
                    }
                } else {
                    scenevm::RepeatMode::Repeat
                }
            } else {
                scenevm::RepeatMode::Repeat
            }
        } else {
            scenevm::RepeatMode::Repeat
        };

        // Billboard orientation: for doors/gates, we want world-space up and right perpendicular to surface normal
        let world_up = Vec3::new(0.0, 1.0, 0.0);
        let normal = surface.plane.normal.normalized();

        // Right is perpendicular to both normal and world up
        let billboard_right = normal.cross(world_up).normalized();

        // Up is perpendicular to both normal and right (ensures orthogonal frame)
        let billboard_up = billboard_right.cross(normal).normalized();

        // Store billboard metadata in chunk for transfer to SceneHandler
        // Animation state will be handled dynamically during rendering
        // NOTE: Swapping up/right to fix 90-degree rotation
        chunk.billboards.push(crate::BillboardMetadata {
            geo_id,
            tile_id: billboard_tile_id,
            center: center_world,
            up: billboard_right,
            right: billboard_up,
            size,
            animation: *animation,
            repeat_mode,
        });

        return Some(());
    }

    // Static window: generate frame + glass meshes for a profile hole.
    if let LoopOp::Window { inset, .. } = &feature_loop.op {
        if feature_loop.path.len() < 3 {
            return Some(());
        }

        let mut min_uv = feature_loop.path[0];
        let mut max_uv = feature_loop.path[0];
        for uv in &feature_loop.path {
            min_uv.x = min_uv.x.min(uv.x);
            min_uv.y = min_uv.y.min(uv.y);
            max_uv.x = max_uv.x.max(uv.x);
            max_uv.y = max_uv.y.max(uv.y);
        }

        let sx = (max_uv.x - min_uv.x).abs();
        let sy = (max_uv.y - min_uv.y).abs();
        if sx < 0.03 || sy < 0.03 {
            return Some(());
        }

        let (base_z0, base_z1) = surface.extrusion.span();
        let mut z0 = base_z0 + *inset;
        let mut z1 = base_z1 + *inset;
        if !surface.extrusion.enabled || (z1 - z0).abs() < 1e-4 {
            z0 = *inset - 0.03;
            z1 = *inset + 0.03;
        }

        let frame_w = feature_profile_float(
            surface,
            map,
            sector,
            feature_loop.origin_profile_sector,
            "window_frame_width",
            sx.min(sy) * 0.10,
        )
        .clamp(0.01, sx.min(sy) * 0.45);
        let has_glass_source = feature_has_explicit_source(
            surface,
            map,
            sector,
            feature_loop.origin_profile_sector,
            "window_glass_source",
        );

        let profile_target =
            feature_profile_target(surface, map, sector, feature_loop.origin_profile_sector);
        let mesh_builder = SurfaceMeshBuilder::new(surface);

        let emit_piece = |px0: f32,
                          px1: f32,
                          py0: f32,
                          py1: f32,
                          pz0: f32,
                          pz1: f32,
                          source_key: &str,
                          chunk: &mut Chunk,
                          vmchunk: &mut scenevm::Chunk| {
            if px1 <= px0 || py1 <= py0 || (pz1 - pz0).abs() < 1e-5 {
                return;
            }

            let top_loop = vec![
                ControlPoint {
                    uv: Vec2::new(px0, py0),
                    extrusion: pz1,
                },
                ControlPoint {
                    uv: Vec2::new(px1, py0),
                    extrusion: pz1,
                },
                ControlPoint {
                    uv: Vec2::new(px1, py1),
                    extrusion: pz1,
                },
                ControlPoint {
                    uv: Vec2::new(px0, py1),
                    extrusion: pz1,
                },
            ];
            let bottom_loop = vec![
                ControlPoint {
                    uv: Vec2::new(px0, py0),
                    extrusion: pz0,
                },
                ControlPoint {
                    uv: Vec2::new(px1, py0),
                    extrusion: pz0,
                },
                ControlPoint {
                    uv: Vec2::new(px1, py1),
                    extrusion: pz0,
                },
                ControlPoint {
                    uv: Vec2::new(px0, py1),
                    extrusion: pz0,
                },
            ];

            let top_cap = SectorMeshDescriptor {
                is_hole: false,
                cap: Some(MeshTopology::FilledRegion {
                    outer: top_loop.clone(),
                    holes: vec![],
                }),
                sides: None,
                connection: crate::chunkbuilder::action::ConnectionMode::Hard,
            };
            let top_meshes = mesh_builder.build(&top_cap);
            emit_feature_meshes(
                surface,
                map,
                sector,
                chunk,
                vmchunk,
                assets,
                feature_loop.origin_profile_sector,
                profile_target,
                &top_meshes,
                true,
                Some(source_key),
            );

            let bottom_cap = SectorMeshDescriptor {
                is_hole: false,
                cap: Some(MeshTopology::FilledRegion {
                    outer: bottom_loop.clone(),
                    holes: vec![],
                }),
                sides: None,
                connection: crate::chunkbuilder::action::ConnectionMode::Hard,
            };
            let bottom_meshes = mesh_builder.build(&bottom_cap);
            emit_feature_meshes(
                surface,
                map,
                sector,
                chunk,
                vmchunk,
                assets,
                feature_loop.origin_profile_sector,
                profile_target,
                &bottom_meshes,
                true,
                Some(source_key),
            );

            let sides = SectorMeshDescriptor {
                is_hole: false,
                cap: None,
                sides: Some(MeshTopology::QuadStrip {
                    loop_a: bottom_loop,
                    loop_b: top_loop,
                }),
                connection: crate::chunkbuilder::action::ConnectionMode::Hard,
            };
            let side_meshes = mesh_builder.build(&sides);
            emit_feature_meshes(
                surface,
                map,
                sector,
                chunk,
                vmchunk,
                assets,
                feature_loop.origin_profile_sector,
                profile_target,
                &side_meshes,
                false,
                Some(source_key),
            );
        };

        // Frame (four sides)
        emit_piece(
            min_uv.x,
            min_uv.x + frame_w,
            min_uv.y,
            max_uv.y,
            z0,
            z1,
            "window_frame_source",
            chunk,
            vmchunk,
        );
        emit_piece(
            max_uv.x - frame_w,
            max_uv.x,
            min_uv.y,
            max_uv.y,
            z0,
            z1,
            "window_frame_source",
            chunk,
            vmchunk,
        );
        emit_piece(
            min_uv.x + frame_w,
            max_uv.x - frame_w,
            max_uv.y - frame_w,
            max_uv.y,
            z0,
            z1,
            "window_frame_source",
            chunk,
            vmchunk,
        );
        emit_piece(
            min_uv.x + frame_w,
            max_uv.x - frame_w,
            min_uv.y,
            min_uv.y + frame_w,
            z0,
            z1,
            "window_frame_source",
            chunk,
            vmchunk,
        );

        // Glass pane (slightly inset from frame to avoid overlap)
        let glass_margin = (frame_w * 0.15).clamp(0.005, 0.03);
        let gx0 = min_uv.x + frame_w + glass_margin;
        let gx1 = max_uv.x - frame_w - glass_margin;
        let gy0 = min_uv.y + frame_w + glass_margin;
        let gy1 = max_uv.y - frame_w - glass_margin;
        if gx1 > gx0 && gy1 > gy0 {
            if has_glass_source {
                let mid = (z0 + z1) * 0.5;
                let gt = ((z1 - z0).abs() * 0.12).clamp(0.01, 0.05);
                let gz0 = mid - gt * 0.5;
                let gz1 = mid + gt * 0.5;
                emit_piece(
                    gx0,
                    gx1,
                    gy0,
                    gy1,
                    gz0,
                    gz1,
                    "window_glass_source",
                    chunk,
                    vmchunk,
                );
            }
        }

        return Some(());
    }

    if let Some(origin) = feature_loop.origin_profile_sector
        && let Some(profile_id) = surface.profile
        && let Some(profile_map) = map.profiles.get(&profile_id)
        && let Some(ps) = profile_map.find_sector(origin)
    {
        let builder_graph_data = ps
            .properties
            .get_str_default("builder_graph_data", String::new());
        let builder_target = ps
            .properties
            .get_str_default("builder_graph_target", "sector".to_string());
        if !builder_graph_data.trim().is_empty()
            && builder_target == "sector"
            && feature_loop.path.len() >= 3
        {
            let profile_target =
                feature_profile_target(surface, map, sector, feature_loop.origin_profile_sector);
            let base_extrusion = if profile_target == 1 {
                surface.extrusion.depth.abs()
            } else {
                0.0
            };

            let mut min_uv = feature_loop.path[0];
            let mut max_uv = feature_loop.path[0];
            for uv in &feature_loop.path {
                min_uv.x = min_uv.x.min(uv.x);
                min_uv.y = min_uv.y.min(uv.y);
                max_uv.x = max_uv.x.max(uv.x);
                max_uv.y = max_uv.y.max(uv.y);
            }

            let _ = emit_builder_sector_meshes(
                &builder_graph_data,
                min_uv,
                max_uv,
                base_extrusion,
                1.0,
                surface,
                map,
                sector,
                chunk,
                vmchunk,
                assets,
                feature_loop.origin_profile_sector,
                profile_target,
            );
            return Some(());
        }
    }

    // "Create Props" table mode: build tabletop + legs instead of a full block.
    if matches!(feature_loop.op, LoopOp::Relief { .. })
        && feature_profile_bool(
            surface,
            map,
            sector,
            feature_loop.origin_profile_sector,
            "profile_table",
            false,
        )
    {
        let LoopOp::Relief { height } = feature_loop.op else {
            return Some(());
        };
        if feature_loop.path.len() >= 3 && height > 0.0 {
            let profile_target =
                feature_profile_target(surface, map, sector, feature_loop.origin_profile_sector);

            let base_extrusion = if profile_target == 1 {
                surface.extrusion.depth.abs()
            } else {
                0.0
            };
            let direction = if profile_target == 1 { 1.0 } else { -1.0 };
            let top_extrusion = base_extrusion + direction * height;
            let slab_thickness = (height * 0.15).clamp(0.06, 0.25);
            let underside_extrusion = top_extrusion - direction * slab_thickness;

            let mut min_uv = feature_loop.path[0];
            let mut max_uv = feature_loop.path[0];
            for uv in &feature_loop.path {
                min_uv.x = min_uv.x.min(uv.x);
                min_uv.y = min_uv.y.min(uv.y);
                max_uv.x = max_uv.x.max(uv.x);
                max_uv.y = max_uv.y.max(uv.y);
            }
            let prop_kind = feature_profile_int(
                surface,
                map,
                sector,
                feature_loop.origin_profile_sector,
                "profile_prop_kind",
                0,
            );
            let sx = (max_uv.x - min_uv.x).abs().max(1e-4);
            let sy = (max_uv.y - min_uv.y).abs().max(1e-4);
            let mesh_builder = SurfaceMeshBuilder::new(surface);

            let make_prism = |px0: f32,
                              px1: f32,
                              py0: f32,
                              py1: f32,
                              pz0: f32,
                              pz1: f32|
             -> SectorMeshDescriptor {
                let top = vec![
                    ControlPoint {
                        uv: Vec2::new(px0, py0),
                        extrusion: pz1,
                    },
                    ControlPoint {
                        uv: Vec2::new(px1, py0),
                        extrusion: pz1,
                    },
                    ControlPoint {
                        uv: Vec2::new(px1, py1),
                        extrusion: pz1,
                    },
                    ControlPoint {
                        uv: Vec2::new(px0, py1),
                        extrusion: pz1,
                    },
                ];
                let bottom = vec![
                    ControlPoint {
                        uv: Vec2::new(px0, py0),
                        extrusion: pz0,
                    },
                    ControlPoint {
                        uv: Vec2::new(px1, py0),
                        extrusion: pz0,
                    },
                    ControlPoint {
                        uv: Vec2::new(px1, py1),
                        extrusion: pz0,
                    },
                    ControlPoint {
                        uv: Vec2::new(px0, py1),
                        extrusion: pz0,
                    },
                ];
                SectorMeshDescriptor {
                    is_hole: false,
                    cap: Some(MeshTopology::FilledRegion {
                        outer: top.clone(),
                        holes: vec![],
                    }),
                    sides: Some(MeshTopology::QuadStrip {
                        loop_a: bottom,
                        loop_b: top,
                    }),
                    connection: crate::chunkbuilder::action::ConnectionMode::Hard,
                }
            };

            macro_rules! emit_prism {
                ($px0:expr, $px1:expr, $py0:expr, $py1:expr, $pz0:expr, $pz1:expr) => {{
                    let part = make_prism($px0, $px1, $py0, $py1, $pz0, $pz1);
                    let part_meshes = mesh_builder.build(&part);
                    emit_feature_meshes(
                        surface,
                        map,
                        sector,
                        chunk,
                        vmchunk,
                        assets,
                        feature_loop.origin_profile_sector,
                        profile_target,
                        &part_meshes,
                        part.cap.is_some(),
                        None,
                    );
                }};
            }

            if prop_kind == 1 {
                let panel_t = (sx.min(sy) * 0.08).clamp(0.06, 0.20);
                // Use the full selected sector depth for the bookcase footprint.
                let depth = sy;
                let x0 = min_uv.x;
                let x1 = max_uv.x;
                let y0 = min_uv.y;
                let y1 = min_uv.y + depth.min(sy);
                let z0 = base_extrusion;
                let z1 = top_extrusion;
                let shelves = feature_profile_int(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "bookcase_shelves",
                    4,
                )
                .clamp(1, 12);
                let has_books = feature_profile_bool(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "bookcase_books",
                    true,
                );

                // Carcass without overlapping panel volumes (avoids z-fighting at joints).
                // Back panel
                emit_prism!(x0, x1, y0, y0 + panel_t, z0, z1);
                // Side panels start after back panel depth
                emit_prism!(x0, x0 + panel_t, y0 + panel_t, y1, z0, z1);
                emit_prism!(x1 - panel_t, x1, y0 + panel_t, y1, z0, z1);
                // Bottom/top panels are inset from side thickness and back thickness
                emit_prism!(
                    x0 + panel_t,
                    x1 - panel_t,
                    y0 + panel_t,
                    y1,
                    z0,
                    z0 + direction * panel_t
                );
                emit_prism!(
                    x0 + panel_t,
                    x1 - panel_t,
                    y0 + panel_t,
                    y1,
                    z1 - direction * panel_t,
                    z1
                );

                let inside_z0 = z0 + direction * panel_t * 1.5;
                let inside_z1 = z1 - direction * panel_t * 1.5;
                let shelf_span = (inside_z1 - inside_z0) / (shelves as f32 + 1.0);
                for i in 0..shelves {
                    let sz = inside_z0 + shelf_span * (i as f32 + 1.0);
                    emit_prism!(
                        x0 + panel_t,
                        x1 - panel_t,
                        y0 + panel_t,
                        y1,
                        sz,
                        sz + direction * panel_t * 0.8
                    );

                    if has_books {
                        let available_palette_indices: Vec<u16> = assets
                            .palette
                            .colors
                            .iter()
                            .enumerate()
                            .filter_map(|(idx, c)| c.as_ref().map(|_| idx as u16))
                            .collect();
                        if available_palette_indices.is_empty() {
                            continue;
                        }

                        // Place books near the open/front side so they stay visible.
                        let row_y0 = (y1 - panel_t * 1.6).max(y0 + panel_t * 1.2);
                        let row_y1 = (y1 - panel_t * 0.3).max(row_y0 + 0.05);
                        let mut bx = x0 + panel_t * 1.2;
                        let bx_end = x1 - panel_t * 1.2;
                        let mut b = 0u32;
                        while bx < bx_end - 0.05 {
                            let seed = (sector.id as u32)
                                .wrapping_mul(73856093)
                                .wrapping_add((i as u32) * 19349663)
                                .wrapping_add(b * 83492791);
                            let width_rand = 0.05 + ((seed % 100) as f32 / 100.0) * 0.08;
                            let bw = width_rand.min(bx_end - bx);
                            // Keep a small vertical clearance above shelf top to avoid z-fighting.
                            let shelf_top = sz + direction * panel_t * 0.8;
                            let bz0 = shelf_top + direction * 0.02;
                            // Clamp book height to available space to the next shelf (or top panel)
                            // so books never intersect board geometry.
                            let top_limit = if i + 1 < shelves {
                                sz + shelf_span
                            } else {
                                inside_z1
                            };
                            let available_h = ((top_limit - bz0) * direction - 0.02).max(0.06);
                            let book_h = (height * 0.22).clamp(0.12, 0.80).min(available_h);
                            let bz1 = bz0 + direction * book_h;
                            let book = make_prism(bx, bx + bw, row_y0, row_y1, bz0, bz1);
                            let book_meshes = mesh_builder.build(&book);
                            let palette_pick =
                                ((seed >> 8) as usize) % available_palette_indices.len();
                            let palette_index = available_palette_indices[palette_pick];
                            let pixelsource = PixelSource::PaletteIndex(palette_index);
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                for (mesh_idx, mesh) in book_meshes.iter().enumerate() {
                                    let is_cap = mesh_idx == 0 && book.cap.is_some();
                                    let mut n = surface.plane.normal;
                                    let ln = n.magnitude();
                                    if ln > 1e-6 {
                                        n /= ln;
                                    } else {
                                        n = vek::Vec3::unit_y();
                                    }

                                    let mut mesh_indices = mesh.indices.clone();
                                    if is_cap {
                                        let desired_n = if profile_target == 0 { -n } else { n };
                                        mesh_fix_winding(
                                            &mesh.vertices,
                                            &mut mesh_indices,
                                            desired_n,
                                        );
                                    } else {
                                        mesh_fix_winding(&mesh.vertices, &mut mesh_indices, n);
                                    }

                                    vmchunk.add_poly_3d(
                                        GeoId::Sector(sector.id),
                                        tile.id,
                                        mesh.vertices.clone(),
                                        mesh.uvs.clone(),
                                        mesh_indices.clone(),
                                        0,
                                        true,
                                    );

                                    let mut batch = Batch3D::new(
                                        mesh.vertices.clone(),
                                        mesh_indices,
                                        mesh.uvs.clone(),
                                    )
                                    .repeat_mode(RepeatMode::RepeatXY)
                                    .geometry_source(GeometrySource::Sector(sector.id));
                                    if let Some(tex) = assets.tile_index(&tile.id) {
                                        batch.source = PixelSource::StaticTileIndex(tex);
                                    }
                                    chunk.batches3d.push(batch);
                                }
                            }
                            bx += bw + 0.02;
                            b = b.wrapping_add(1);
                        }
                    }
                }
                return Some(());
            }

            if prop_kind == 2 {
                emit_prism!(
                    min_uv.x,
                    max_uv.x,
                    min_uv.y,
                    max_uv.y,
                    base_extrusion,
                    top_extrusion
                );
                return Some(());
            }

            if prop_kind == 3 {
                let segments = feature_profile_int(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "barrel_segments",
                    12,
                )
                .clamp(6, 32) as usize;
                let bulge = feature_profile_float(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "barrel_bulge",
                    1.12,
                )
                .clamp(1.0, 1.5);

                let cx = (min_uv.x + max_uv.x) * 0.5;
                let cy = (min_uv.y + max_uv.y) * 0.5;
                let rx = sx * 0.5 * 0.92;
                let ry = sy * 0.5 * 0.92;
                let z0 = base_extrusion;
                let z1 = top_extrusion;
                let zm = (z0 + z1) * 0.5;

                let make_ring = |scale: f32, extrusion: f32| -> Vec<ControlPoint> {
                    let mut ring = Vec::with_capacity(segments);
                    for i in 0..segments {
                        let t = i as f32 / segments as f32;
                        let a = t * std::f32::consts::TAU;
                        ring.push(ControlPoint {
                            uv: Vec2::new(cx + a.cos() * rx * scale, cy + a.sin() * ry * scale),
                            extrusion,
                        });
                    }
                    ring
                };

                let ring_bottom = make_ring(1.0, z0);
                let ring_mid = make_ring(bulge, zm);
                let ring_top = make_ring(1.0, z1);

                let top_cap = SectorMeshDescriptor {
                    is_hole: false,
                    cap: Some(MeshTopology::FilledRegion {
                        outer: ring_top.clone(),
                        holes: vec![],
                    }),
                    sides: None,
                    connection: crate::chunkbuilder::action::ConnectionMode::Hard,
                };
                let top_meshes = mesh_builder.build(&top_cap);
                emit_feature_meshes(
                    surface,
                    map,
                    sector,
                    chunk,
                    vmchunk,
                    assets,
                    feature_loop.origin_profile_sector,
                    profile_target,
                    &top_meshes,
                    top_cap.cap.is_some(),
                    None,
                );

                let bottom_cap = SectorMeshDescriptor {
                    is_hole: false,
                    cap: Some(MeshTopology::FilledRegion {
                        outer: ring_bottom.clone(),
                        holes: vec![],
                    }),
                    sides: None,
                    connection: crate::chunkbuilder::action::ConnectionMode::Hard,
                };
                let bottom_meshes = mesh_builder.build(&bottom_cap);
                emit_feature_meshes(
                    surface,
                    map,
                    sector,
                    chunk,
                    vmchunk,
                    assets,
                    feature_loop.origin_profile_sector,
                    profile_target,
                    &bottom_meshes,
                    bottom_cap.cap.is_some(),
                    None,
                );

                let side_lower = SectorMeshDescriptor {
                    is_hole: false,
                    cap: None,
                    sides: Some(MeshTopology::QuadStrip {
                        loop_a: ring_bottom,
                        loop_b: ring_mid.clone(),
                    }),
                    connection: crate::chunkbuilder::action::ConnectionMode::Hard,
                };
                let side_lower_meshes = mesh_builder.build(&side_lower);
                emit_feature_meshes(
                    surface,
                    map,
                    sector,
                    chunk,
                    vmchunk,
                    assets,
                    feature_loop.origin_profile_sector,
                    profile_target,
                    &side_lower_meshes,
                    false,
                    None,
                );

                let side_upper = SectorMeshDescriptor {
                    is_hole: false,
                    cap: None,
                    sides: Some(MeshTopology::QuadStrip {
                        loop_a: ring_mid,
                        loop_b: ring_top,
                    }),
                    connection: crate::chunkbuilder::action::ConnectionMode::Hard,
                };
                let side_upper_meshes = mesh_builder.build(&side_upper);
                emit_feature_meshes(
                    surface,
                    map,
                    sector,
                    chunk,
                    vmchunk,
                    assets,
                    feature_loop.origin_profile_sector,
                    profile_target,
                    &side_upper_meshes,
                    false,
                    None,
                );

                return Some(());
            }

            if prop_kind == 4 {
                let z0 = base_extrusion;
                let z1 = top_extrusion;
                let h = (z1 - z0).abs().max(0.05);
                let mattress_h = h * 0.45;
                let frame_h = h * 0.35;
                let leg_h = h * 0.20;

                // Slight inset so the bed doesn't exactly overlap sector edges.
                let inset = (sx.min(sy) * 0.06).clamp(0.03, 0.20);
                let bx0 = min_uv.x + inset;
                let bx1 = max_uv.x - inset;
                let by0 = min_uv.y + inset;
                let by1 = max_uv.y - inset;

                let direction = if z1 >= z0 { 1.0 } else { -1.0 };
                let frame_top = z0 + direction * (leg_h + frame_h);
                let mattress_top = frame_top + direction * mattress_h;

                // Base frame.
                emit_prism!(bx0, bx1, by0, by1, z0, frame_top);

                // Mattress inset from frame.
                let m_inset = (inset * 0.6).clamp(0.02, 0.12);
                let mattress = make_prism(
                    bx0 + m_inset,
                    bx1 - m_inset,
                    by0 + m_inset,
                    by1 - m_inset,
                    frame_top,
                    mattress_top,
                );
                let mattress_meshes = mesh_builder.build(&mattress);
                emit_feature_meshes(
                    surface,
                    map,
                    sector,
                    chunk,
                    vmchunk,
                    assets,
                    feature_loop.origin_profile_sector,
                    profile_target,
                    &mattress_meshes,
                    mattress.cap.is_some(),
                    Some("bed_mattress_source"),
                );

                // Optional headboard at one short side, aligned to the bed's longer axis.
                let headboard_enabled = feature_profile_bool(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "bed_headboard",
                    true,
                );
                if headboard_enabled {
                    let hb_h = feature_profile_float(
                        surface,
                        map,
                        sector,
                        feature_loop.origin_profile_sector,
                        "bed_headboard_height",
                        0.7,
                    )
                    .clamp(0.2, 2.5);
                    let hb_top = mattress_top + direction * hb_h;
                    let hb_t = (sx.min(sy) * 0.08).clamp(0.04, 0.14);
                    let hb_side = feature_profile_int(
                        surface,
                        map,
                        sector,
                        feature_loop.origin_profile_sector,
                        "bed_headboard_side",
                        0,
                    )
                    .clamp(0, 1);
                    if sx >= sy {
                        // Bed length along x -> headboard on min/max x side.
                        if hb_side == 0 {
                            emit_prism!(bx0, bx0 + hb_t, by0, by1, frame_top, hb_top);
                        } else {
                            emit_prism!(bx1 - hb_t, bx1, by0, by1, frame_top, hb_top);
                        }
                    } else {
                        // Bed length along y -> headboard on min/max y side.
                        if hb_side == 0 {
                            emit_prism!(bx0, bx1, by0, by0 + hb_t, frame_top, hb_top);
                        } else {
                            emit_prism!(bx0, bx1, by1 - hb_t, by1, frame_top, hb_top);
                        }
                    }
                }

                return Some(());
            }
            let leg_half = (sx.min(sy) * 0.10).clamp(0.05, 0.35) * 0.5;
            // Prefer host sector values for Create Props (authored per target sector),
            // and only fall back to profile values when host values are missing.
            let chairs_enabled = match sector.properties.get("table_chairs") {
                Some(Value::Bool(v)) => *v,
                _ => feature_profile_bool(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "table_chairs",
                    false,
                ),
            };
            let chair_count = match sector.properties.get("table_chair_count") {
                Some(Value::Int(v)) => (*v).clamp(0, 8),
                _ => feature_profile_int(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "table_chair_count",
                    4,
                )
                .clamp(0, 8),
            };
            let chair_offset = match sector.properties.get("table_chair_offset") {
                Some(Value::Float(v)) => (*v).max(0.0),
                _ => feature_profile_float(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "table_chair_offset",
                    0.45,
                )
                .max(0.0),
            };
            let chair_width = match sector.properties.get("table_chair_width") {
                Some(Value::Float(v)) => (*v).clamp(0.20, 3.0),
                _ => feature_profile_float(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "table_chair_width",
                    0.85,
                )
                .clamp(0.20, 3.0),
            };
            let chair_back_height = match sector.properties.get("table_chair_back_height") {
                Some(Value::Float(v)) => (*v).clamp(0.25, 3.0),
                _ => feature_profile_float(
                    surface,
                    map,
                    sector,
                    feature_loop.origin_profile_sector,
                    "table_chair_back_height",
                    1.0,
                )
                .clamp(0.25, 3.0),
            };

            let top_loop: Vec<ControlPoint> = feature_loop
                .path
                .iter()
                .map(|&uv| ControlPoint {
                    uv,
                    extrusion: top_extrusion,
                })
                .collect();
            let bottom_loop: Vec<ControlPoint> = feature_loop
                .path
                .iter()
                .map(|&uv| ControlPoint {
                    uv,
                    extrusion: underside_extrusion,
                })
                .collect();

            let tabletop = SectorMeshDescriptor {
                is_hole: false,
                cap: Some(MeshTopology::FilledRegion {
                    outer: top_loop.clone(),
                    holes: vec![],
                }),
                sides: Some(MeshTopology::QuadStrip {
                    loop_a: bottom_loop.clone(),
                    loop_b: top_loop,
                }),
                connection: crate::chunkbuilder::action::ConnectionMode::Hard,
            };
            let tabletop_meshes = mesh_builder.build(&tabletop);
            emit_feature_meshes(
                surface,
                map,
                sector,
                chunk,
                vmchunk,
                assets,
                feature_loop.origin_profile_sector,
                profile_target,
                &tabletop_meshes,
                tabletop.cap.is_some(),
                None,
            );

            let tabletop_underside = SectorMeshDescriptor {
                is_hole: false,
                cap: Some(MeshTopology::FilledRegion {
                    outer: bottom_loop,
                    holes: vec![],
                }),
                sides: None,
                connection: crate::chunkbuilder::action::ConnectionMode::Hard,
            };
            let underside_meshes = mesh_builder.build(&tabletop_underside);
            emit_feature_meshes(
                surface,
                map,
                sector,
                chunk,
                vmchunk,
                assets,
                feature_loop.origin_profile_sector,
                profile_target,
                &underside_meshes,
                tabletop_underside.cap.is_some(),
                None,
            );

            // Keep all legs fully under the tabletop by insetting from bbox corners.
            let inset_x = leg_half.min((sx * 0.5 - 1e-3).max(leg_half));
            let inset_y = leg_half.min((sy * 0.5 - 1e-3).max(leg_half));
            let leg_centers = [
                Vec2::new(min_uv.x + inset_x, min_uv.y + inset_y),
                Vec2::new(max_uv.x - inset_x, min_uv.y + inset_y),
                Vec2::new(max_uv.x - inset_x, max_uv.y - inset_y),
                Vec2::new(min_uv.x + inset_x, max_uv.y - inset_y),
            ];

            for c in leg_centers {
                let leg_loop_uv = vec![
                    Vec2::new(c.x - leg_half, c.y - leg_half),
                    Vec2::new(c.x + leg_half, c.y - leg_half),
                    Vec2::new(c.x + leg_half, c.y + leg_half),
                    Vec2::new(c.x - leg_half, c.y + leg_half),
                ];
                let leg_bottom: Vec<ControlPoint> = leg_loop_uv
                    .iter()
                    .map(|&uv| ControlPoint {
                        uv,
                        extrusion: base_extrusion,
                    })
                    .collect();
                let leg_top: Vec<ControlPoint> = leg_loop_uv
                    .iter()
                    .map(|&uv| ControlPoint {
                        uv,
                        extrusion: underside_extrusion,
                    })
                    .collect();
                let leg = SectorMeshDescriptor {
                    is_hole: false,
                    cap: Some(MeshTopology::FilledRegion {
                        outer: leg_top.clone(),
                        holes: vec![],
                    }),
                    sides: Some(MeshTopology::QuadStrip {
                        loop_a: leg_bottom,
                        loop_b: leg_top,
                    }),
                    connection: crate::chunkbuilder::action::ConnectionMode::Hard,
                };
                let leg_meshes = mesh_builder.build(&leg);
                emit_feature_meshes(
                    surface,
                    map,
                    sector,
                    chunk,
                    vmchunk,
                    assets,
                    feature_loop.origin_profile_sector,
                    profile_target,
                    &leg_meshes,
                    leg.cap.is_some(),
                    None,
                );
            }

            if chairs_enabled && chair_count > 0 {
                let make_prism = |x0: f32,
                                  x1: f32,
                                  y0: f32,
                                  y1: f32,
                                  z0: f32,
                                  z1: f32|
                 -> SectorMeshDescriptor {
                    let top = vec![
                        ControlPoint {
                            uv: Vec2::new(x0, y0),
                            extrusion: z1,
                        },
                        ControlPoint {
                            uv: Vec2::new(x1, y0),
                            extrusion: z1,
                        },
                        ControlPoint {
                            uv: Vec2::new(x1, y1),
                            extrusion: z1,
                        },
                        ControlPoint {
                            uv: Vec2::new(x0, y1),
                            extrusion: z1,
                        },
                    ];
                    let bottom = vec![
                        ControlPoint {
                            uv: Vec2::new(x0, y0),
                            extrusion: z0,
                        },
                        ControlPoint {
                            uv: Vec2::new(x1, y0),
                            extrusion: z0,
                        },
                        ControlPoint {
                            uv: Vec2::new(x1, y1),
                            extrusion: z0,
                        },
                        ControlPoint {
                            uv: Vec2::new(x0, y1),
                            extrusion: z0,
                        },
                    ];
                    SectorMeshDescriptor {
                        is_hole: false,
                        cap: Some(MeshTopology::FilledRegion {
                            outer: top.clone(),
                            holes: vec![],
                        }),
                        sides: Some(MeshTopology::QuadStrip {
                            loop_a: bottom,
                            loop_b: top,
                        }),
                        connection: crate::chunkbuilder::action::ConnectionMode::Hard,
                    }
                };

                let csize = chair_width.clamp(0.20, 3.0);
                let chalf = csize * 0.5;
                let seat_h = height * 0.45;
                let seat_t = (height * 0.12).clamp(0.05, 0.18);
                let leg_w = (csize * 0.16).clamp(0.05, 0.16);
                let lhalf = leg_w * 0.5;
                // Make the backrest clearly extend above tabletop so chairs remain visible.
                let back_h = (height * 0.95 * chair_back_height).clamp(0.25, 3.0);
                let back_t = (csize * 0.10).clamp(0.04, 0.12);

                let chair_z0 = base_extrusion;
                let chair_seat_bottom = base_extrusion + direction * (seat_h - seat_t);
                let chair_seat_top = base_extrusion + direction * seat_h;
                let chair_back_top = base_extrusion + direction * (seat_h + back_h);

                let cx = (min_uv.x + max_uv.x) * 0.5;
                let cy = (min_uv.y + max_uv.y) * 0.5;
                let mut centers: Vec<(Vec2<f32>, i32)> = Vec::new(); // dir: 0=north 1=south 2=west 3=east
                if chair_count >= 1 {
                    centers.push((Vec2::new(cx, max_uv.y + chair_offset), 0));
                }
                if chair_count >= 2 {
                    centers.push((Vec2::new(cx, min_uv.y - chair_offset), 1));
                }
                if chair_count >= 3 {
                    centers.push((Vec2::new(min_uv.x - chair_offset, cy), 2));
                }
                if chair_count >= 4 {
                    centers.push((Vec2::new(max_uv.x + chair_offset, cy), 3));
                }
                if chair_count >= 5 {
                    centers.push((
                        Vec2::new(min_uv.x - chair_offset, max_uv.y + chair_offset),
                        2,
                    ));
                }
                if chair_count >= 6 {
                    centers.push((
                        Vec2::new(max_uv.x + chair_offset, max_uv.y + chair_offset),
                        3,
                    ));
                }
                if chair_count >= 7 {
                    centers.push((
                        Vec2::new(min_uv.x - chair_offset, min_uv.y - chair_offset),
                        2,
                    ));
                }
                if chair_count >= 8 {
                    centers.push((
                        Vec2::new(max_uv.x + chair_offset, min_uv.y - chair_offset),
                        3,
                    ));
                }

                for (cc, dir_idx) in centers {
                    // Seat
                    let seat = make_prism(
                        cc.x - chalf,
                        cc.x + chalf,
                        cc.y - chalf,
                        cc.y + chalf,
                        chair_seat_bottom,
                        chair_seat_top,
                    );
                    let seat_meshes = mesh_builder.build(&seat);
                    emit_feature_meshes(
                        surface,
                        map,
                        sector,
                        chunk,
                        vmchunk,
                        assets,
                        feature_loop.origin_profile_sector,
                        profile_target,
                        &seat_meshes,
                        seat.cap.is_some(),
                        Some("chair_source"),
                    );

                    // Chair legs
                    let leg_centers = [
                        Vec2::new(cc.x - chalf + lhalf, cc.y - chalf + lhalf),
                        Vec2::new(cc.x + chalf - lhalf, cc.y - chalf + lhalf),
                        Vec2::new(cc.x + chalf - lhalf, cc.y + chalf - lhalf),
                        Vec2::new(cc.x - chalf + lhalf, cc.y + chalf - lhalf),
                    ];
                    for lc in leg_centers {
                        let cleg = make_prism(
                            lc.x - lhalf,
                            lc.x + lhalf,
                            lc.y - lhalf,
                            lc.y + lhalf,
                            chair_z0,
                            chair_seat_bottom,
                        );
                        let cleg_meshes = mesh_builder.build(&cleg);
                        emit_feature_meshes(
                            surface,
                            map,
                            sector,
                            chunk,
                            vmchunk,
                            assets,
                            feature_loop.origin_profile_sector,
                            profile_target,
                            &cleg_meshes,
                            cleg.cap.is_some(),
                            Some("chair_source"),
                        );
                    }

                    // Backrest slab (faces toward table center)
                    let (bx0, bx1, by0, by1) = match dir_idx {
                        0 => (
                            cc.x - chalf,
                            cc.x + chalf,
                            cc.y + chalf - back_t,
                            cc.y + chalf,
                        ),
                        1 => (
                            cc.x - chalf,
                            cc.x + chalf,
                            cc.y - chalf,
                            cc.y - chalf + back_t,
                        ),
                        2 => (
                            cc.x - chalf,
                            cc.x - chalf + back_t,
                            cc.y - chalf,
                            cc.y + chalf,
                        ),
                        _ => (
                            cc.x + chalf - back_t,
                            cc.x + chalf,
                            cc.y - chalf,
                            cc.y + chalf,
                        ),
                    };
                    let back = make_prism(bx0, bx1, by0, by1, chair_seat_top, chair_back_top);
                    let back_meshes = mesh_builder.build(&back);
                    emit_feature_meshes(
                        surface,
                        map,
                        sector,
                        chunk,
                        vmchunk,
                        assets,
                        feature_loop.origin_profile_sector,
                        profile_target,
                        &back_meshes,
                        back.cap.is_some(),
                        Some("chair_source"),
                    );
                }
            }
        }
        return Some(());
    }

    // Get the action for this loop operation
    let action = feature_loop.op.get_action()?;

    // Get profile_target to determine which side to attach to
    let profile_target = if let Some(origin) = feature_loop.origin_profile_sector {
        if let Some(profile_id) = surface.profile {
            if let Some(profile_map) = map.profiles.get(&profile_id) {
                if let Some(ps) = profile_map.find_sector(origin) {
                    ps.properties.get_int_default("profile_target", 0)
                } else {
                    sector.properties.get_int_default("profile_target", 0)
                }
            } else {
                sector.properties.get_int_default("profile_target", 0)
            }
        } else {
            sector.properties.get_int_default("profile_target", 0)
        }
    } else {
        sector.properties.get_int_default("profile_target", 0)
    };

    // Create action properties
    let mut properties = feature_loop.op.to_action_properties(profile_target);

    // Read connection_mode from properties if set
    let connection_mode = if let Some(origin) = feature_loop.origin_profile_sector {
        if let Some(profile_id) = surface.profile {
            if let Some(profile_map) = map.profiles.get(&profile_id) {
                if let Some(ps) = profile_map.find_sector(origin) {
                    ps.properties.get_int_default("connection_mode", -1)
                } else {
                    -1
                }
            } else {
                -1
            }
        } else {
            -1
        }
    } else {
        -1
    };

    // Apply connection mode if specified
    if connection_mode >= 0 {
        use crate::chunkbuilder::action::ConnectionMode;
        properties.connection_override = match connection_mode {
            0 => Some(ConnectionMode::Hard),
            1 => Some(ConnectionMode::Smooth),
            2 => {
                // Bevel mode - read additional parameters
                let segments = if let Some(origin) = feature_loop.origin_profile_sector {
                    if let Some(profile_id) = surface.profile {
                        if let Some(profile_map) = map.profiles.get(&profile_id) {
                            if let Some(ps) = profile_map.find_sector(origin) {
                                ps.properties.get_int_default("bevel_segments", 4) as u8
                            } else {
                                4
                            }
                        } else {
                            4
                        }
                    } else {
                        4
                    }
                } else {
                    4
                };

                let radius = if let Some(origin) = feature_loop.origin_profile_sector {
                    if let Some(profile_id) = surface.profile {
                        if let Some(profile_map) = map.profiles.get(&profile_id) {
                            if let Some(ps) = profile_map.find_sector(origin) {
                                ps.properties.get_float_default("bevel_radius", 0.5)
                            } else {
                                0.5
                            }
                        } else {
                            0.5
                        }
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };

                Some(ConnectionMode::Bevel { segments, radius })
            }
            _ => None,
        };
    }

    // Get mesh descriptor from the action
    let descriptor = action.describe_mesh(
        &feature_loop.path,
        surface.extrusion.depth.abs(),
        &properties,
    )?;

    // Build the meshes using the unified builder
    let mesh_builder = SurfaceMeshBuilder::new(surface);
    let meshes = mesh_builder.build(&descriptor);

    // Process each generated mesh
    for (mesh_idx, mesh) in meshes.iter().enumerate() {
        let is_cap = mesh_idx == 0 && descriptor.cap.is_some();
        let is_side = !is_cap;

        // Determine normal direction for winding
        let mut n = surface.plane.normal;
        let ln = n.magnitude();
        if ln > 1e-6 {
            n /= ln;
        } else {
            n = vek::Vec3::unit_y();
        }

        // For caps, determine which direction they should face based on target
        let mut mesh_indices = mesh.indices.clone();
        if is_cap {
            let desired_n = if profile_target == 0 { -n } else { n };
            mesh_fix_winding(&mesh.vertices, &mut mesh_indices, desired_n);
        } else if is_side {
            mesh_fix_winding(&mesh.vertices, &mut mesh_indices, n);
        }

        // Create batch
        let mut batch = Batch3D::new(
            mesh.vertices.clone(),
            mesh_indices.clone(),
            mesh.uvs.clone(),
        )
        .repeat_mode(RepeatMode::RepeatXY)
        .geometry_source(GeometrySource::Sector(sector.id));

        // Determine material source key based on mesh type
        // Use unified property names that work for all actions
        let source_key = if is_cap {
            "cap_source" // Unified: all caps use cap_source
        } else {
            "jamb_source" // Unified: all sides/walls use jamb_source
        };

        // Apply material
        let mut added = false;
        if let Some(Value::Source(pixelsource)) = feature_pixelsource(
            surface,
            map,
            sector,
            feature_loop.origin_profile_sector,
            source_key,
        ) {
            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                vmchunk.add_poly_3d(
                    GeoId::Sector(sector.id),
                    tile.id,
                    mesh.vertices.clone(),
                    mesh.uvs.clone(),
                    mesh_indices.clone(),
                    0,
                    true,
                );
                added = true;

                if let Some(tex) = assets.tile_index(&tile.id) {
                    batch.source = PixelSource::StaticTileIndex(tex);
                }
            }
        }

        if !added {
            vmchunk.add_poly_3d(
                GeoId::Sector(sector.id),
                Uuid::from_str(DEFAULT_TILE_ID).unwrap(),
                mesh.vertices.clone(),
                mesh.uvs.clone(),
                mesh_indices,
                0,
                true,
            );
        }

        chunk.batches3d.push(batch);
    }

    Some(())
}
