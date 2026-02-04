use crate::chunkbuilder::surface_mesh_builder::{
    SurfaceMeshBuilder, fix_winding as mesh_fix_winding,
};
use crate::chunkbuilder::terrain_generator::{TerrainConfig, TerrainGenerator};
use crate::collision_world::{BlockingVolume, DynamicOpening, OpeningType, WalkableFloor};
use crate::{
    Assets, Batch3D, Chunk, ChunkBuilder, Item, Map, PixelSource, Value, VertexBlendPreset,
};
use crate::{BillboardAnimation, GeometrySource, LoopOp, ProfileLoop, RepeatMode, Sector};
use rustc_hash::{FxHashMap, FxHashSet};
use scenevm::GeoId;
use std::str::FromStr;
use uuid::Uuid;
use vek::{Vec2, Vec3};

/// Default tile UUID for untextured/fallback meshes
pub const DEFAULT_TILE_ID: &str = "27826750-a9e7-4346-994b-fb318b238452";

pub struct D3ChunkBuilder {}

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

fn build_surface_uvs(verts_uv: &[[f32; 2]], sector: &Sector) -> Vec<[f32; 2]> {
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
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(verts_uv.len());
    if tile_mode == 0 {
        for v in verts_uv {
            uvs.push([(v[0] - minx) / sx, (v[1] - miny) / sy]);
        }
    } else {
        let tex_scale_x = sector.properties.get_float_default("texture_scale_x", 1.0);
        let tex_scale_y = sector.properties.get_float_default("texture_scale_y", 1.0);
        for v in verts_uv {
            uvs.push([(v[0] - minx) / tex_scale_x, (v[1] - miny) / tex_scale_y]);
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
        subdivide_triangles_into_tiles(indices, uvs, surface);

    // Build a per-vertex UV set that is local to each tile (0..1), used for overrides
    let mut uvs_local = tiled_uvs.clone();
    for (i, uv) in uvs_local.iter_mut().enumerate() {
        let (tx, ty) = vertex_cells[i];
        uv[0] -= tx as f32;
        uv[1] -= ty as f32;
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
        .map(|uv| (uv[0].floor() as i32, uv[1].floor() as i32))
        .collect();
    let mut tiled_indices = Vec::new();

    for &(a, b, c) in indices {
        let pa = vek::Vec2::new(verts_uv[a][0], verts_uv[a][1]);
        let pb = vek::Vec2::new(verts_uv[b][0], verts_uv[b][1]);
        let pc = vek::Vec2::new(verts_uv[c][0], verts_uv[c][1]);
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
                    new_uvs.push([p.x, p.y]);
                    let w = surface.uv_to_world(*p);
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

            // Skip sectors in ridge mode - they only contribute height to terrain, not surfaces
            let terrain_mode = sector.properties.get_int_default("terrain_mode", 0);
            if terrain_mode == 2 {
                continue;
            }

            // Keep track of hidden sectors so that we can set them as not visible later
            let visible = sector.properties.get_bool_default("visible", true);
            if !visible {
                hidden.insert(GeoId::Sector(sector.id));
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
                    split_loops_for_base(&outer_loop, &hole_loops, extrude_abs);
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
                    let world_vertices_for_fix = build_world_vertices(&verts_uv, surface);

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
                    let (
                        verts_uv,
                        world_vertices,
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
                    );

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

                    let uvs = build_surface_uvs(&verts_uv, sector);
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
                                         depth: f32|
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
                            let p = surface.uv_to_world(loop_uv[i]);
                            front_ws.push(p);
                        }
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
                        let depth_abs = depth.abs().max(1e-6);

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
                            let a_world = surface.uv_to_world(a_uv);
                            let b_world = surface.uv_to_world(b_uv);
                            let a_back = a_world + n * depth;
                            let b_back = b_world + n * depth;

                            // Skip edge if both vertices are at nearly the same low height (e.g., door bottom on floor)
                            // Check if edge is horizontal and at the minimum Y position
                            const MIN_HEIGHT_THRESHOLD: f32 = 0.2;
                            let edge_is_horizontal = (a_world.y - b_world.y).abs() < 0.01;
                            let edge_is_low = a_world.y.min(b_world.y) < MIN_HEIGHT_THRESHOLD;

                            if edge_is_horizontal && edge_is_low {
                                continue; // Skip horizontal edges at floor level (door bottoms)
                            }

                            let base = verts.len();
                            verts.push([a_world.x, a_world.y, a_world.z, 1.0]);
                            verts.push([b_world.x, b_world.y, b_world.z, 1.0]);
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
                    if surface.extrusion.enabled && surface.extrusion.depth.abs() > 1e-6 {
                        let depth = surface.extrusion.depth;
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
                                            + n * depth;
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
                                );

                                let mut back_world_vertices = back_world_vertices;
                                for v in back_world_vertices.iter_mut() {
                                    let p = vek::Vec3::new(v[0], v[1], v[2]) + n * depth;
                                    v[0] = p.x;
                                    v[1] = p.y;
                                    v[2] = p.z;
                                }

                                let back_uvs = build_surface_uvs(&back_verts_uv, sector);

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
                            let (ring_v, mut ring_i, ring_uv) = build_jamb_uv(loop_uv, depth);
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
                    let world_vertices_for_fix = build_world_vertices(&verts_uv, surface);
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
                    let (
                        verts_uv,
                        world_vertices,
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
                    );

                    let uvs = build_surface_uvs(&verts_uv, sector);
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

        // Set all hidden geometry as not visible
        for hidden in hidden {
            if let Some(poly) = vmchunk.polys3d_map.get_mut(&hidden) {
                for p in poly {
                    p.visible = false;
                }
            }
        }

        // Generate terrain for this chunk
        let terrain_counter = chunk.bbox.min.x as u32 * 10000 + chunk.bbox.min.y as u32;
        generate_terrain(map, assets, chunk, vmchunk, terrain_counter);
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

            let bbox = sector.bounding_box(map);
            // Cull with the sector bbox: only check intersection
            // Don't require center to be in chunk - surfaces can span multiple chunks!
            if !bbox.intersects(&chunk_bbox) {
                continue;
            }

            // Get profile loops (same as rendering)
            if let Some((outer_loop, hole_loops)) = read_profile_loops(surface, sector, map) {
                let extrude_abs = surface.extrusion.depth.abs();

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

                // Only add blocking volumes for VERTICAL surfaces (walls)
                // Horizontal surfaces (floors/ceilings) should not block movement
                if !is_horizontal {
                    // Add blocking volume for vertical surfaces (both extruded and non-extruded)
                    if surface.extrusion.enabled && extrude_abs > 1e-6 {
                        // Extruded surface - full volume
                        let top_y = base_y + surface.extrusion.depth;
                        let (min_y, max_y) = if surface.extrusion.depth > 0.0 {
                            (base_y, top_y)
                        } else {
                            (top_y, base_y)
                        };

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

                        collision.static_volumes.push(BlockingVolume {
                            geo_id: GeoId::Sector(sector.id),
                            min: wall_min,
                            max: wall_max,
                        });

                        // Add walkable floor at base level
                        let floor_polygon: Vec<Vec2<f32>> = outer_loop
                            .path
                            .iter()
                            .map(|uv| {
                                let world_pos = surface.uv_to_world(*uv);
                                Vec2::new(world_pos.x, world_pos.z)
                            })
                            .collect();

                        collision.walkable_floors.push(WalkableFloor {
                            geo_id: GeoId::Sector(sector.id),
                            height: base_y,
                            polygon_2d: floor_polygon,
                        });
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

                        collision.static_volumes.push(BlockingVolume {
                            geo_id: GeoId::Sector(sector.id),
                            min: wall_min,
                            max: wall_max,
                        });
                    }
                } else {
                    // Horizontal surface (floor/ceiling) - only add as walkable floor if facing up
                    if normal.y > 0.7 {
                        let floor_polygon: Vec<Vec2<f32>> = outer_loop
                            .path
                            .iter()
                            .map(|uv| {
                                let world_pos = surface.uv_to_world(*uv);
                                Vec2::new(world_pos.x, world_pos.z)
                            })
                            .collect();

                        collision.walkable_floors.push(WalkableFloor {
                            geo_id: GeoId::Sector(sector.id),
                            height: base_y,
                            polygon_2d: floor_polygon,
                        });
                    }
                }

                // Process holes/doors/windows as dynamic openings
                for h in &hole_loops {
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

                    if !is_horizontal {
                        // Vertical wall - add blocking volume
                        let extrude_abs = surface.extrusion.depth.abs();
                        const MIN_THICKNESS: f32 = 0.1;

                        if surface.extrusion.enabled && extrude_abs > 1e-6 {
                            let top_y = base_y + surface.extrusion.depth;
                            let (min_y, max_y) = if surface.extrusion.depth > 0.0 {
                                (base_y, top_y)
                            } else {
                                (top_y, base_y)
                            };

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

                            collision.static_volumes.push(BlockingVolume {
                                geo_id: GeoId::Sector(sector.id),
                                min: wall_min,
                                max: wall_max,
                            });
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

                            collision.static_volumes.push(BlockingVolume {
                                geo_id: GeoId::Sector(sector.id),
                                min: wall_min,
                                max: wall_max,
                            });
                        }
                    } else if normalized_y > 0.7 {
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

        collision
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

    // Create terrain generator with default config
    let config = TerrainConfig::default();
    let generator = TerrainGenerator::new(config);

    // Generate terrain meshes for this chunk (grouped by tile)
    if let Some(meshes) = generator.generate(map, chunk, assets, default_tile_id, tile_overrides) {
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
                // Check if this tile has blend overrides
                if let Some(blend_map) = blend_overrides {
                    if let Some((preset, ps)) = blend_map.get(&(tile_x, tile_z)) {
                        if let Some(tile2) = ps.tile_from_tile_list(assets) {
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
                                blended_uvs.push([uvs[i0][0], 1.0 - uvs[i0][1]]);
                                blended_uvs.push([uvs[i1][0], 1.0 - uvs[i1][1]]);
                                blended_uvs.push([uvs[i2][0], 1.0 - uvs[i2][1]]);

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

                // No blend override - add as regular poly
                for &triangle in &triangles {
                    let i0 = triangle.0;
                    let i1 = triangle.1;
                    let i2 = triangle.2;

                    let tri_vertices = vec![vertices_4d[i0], vertices_4d[i1], vertices_4d[i2]];
                    let tri_uvs = vec![
                        [uvs[i0][0], 1.0 - uvs[i0][1]],
                        [uvs[i1][0], 1.0 - uvs[i1][1]],
                        [uvs[i2][0], 1.0 - uvs[i2][1]],
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
}

// --- Relief/recess pipeline helpers ---
/// Classify profile loops: only true holes (cutouts and through-recesses) are subtracted from the base;
/// shallow recesses and reliefs are handled as feature meshes.
fn split_loops_for_base<'a>(
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
                // Pure cutout → subtract from base; no feature meshes needed
                base_holes.push(h);
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
