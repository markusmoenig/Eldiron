use crate::chunkbuilder::d3chunkbuilder::DEFAULT_TILE_ID;
use crate::chunkbuilder::terrain_generator::{TerrainConfig, TerrainGenerator};
use crate::{Assets, Batch2D, Chunk, ChunkBuilder, Map, PixelSource, Value};
use rustc_hash::FxHashMap;
use scenevm::GeoId;
use std::str::FromStr;
use uuid::Uuid;
use vek::Vec2;

pub struct D2ChunkBuilder {}

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

fn road_noise_hash(value: f32) -> f32 {
    (value.sin() * 43_758.547).fract().abs()
}

fn road_noise1(value: f32, seed: f32) -> f32 {
    let x = value.floor();
    let t = value - x;
    let a = road_noise_hash(x * 12.9898 + seed * 78.233);
    let b = road_noise_hash((x + 1.0) * 12.9898 + seed * 78.233);
    let u = t * t * (3.0 - 2.0 * t);
    a + (b - a) * u
}

fn road_noise2(point: Vec2<f32>, seed: f32) -> f32 {
    let ix = point.x.floor();
    let iy = point.y.floor();
    let tx = point.x - ix;
    let ty = point.y - iy;
    let h = |x: f32, y: f32| road_noise_hash(x * 12.9898 + y * 78.233 + seed * 37.719);
    let a = h(ix, iy);
    let b = h(ix + 1.0, iy);
    let c = h(ix, iy + 1.0);
    let d = h(ix + 1.0, iy + 1.0);
    let ux = tx * tx * (3.0 - 2.0 * tx);
    let uy = ty * ty * (3.0 - 2.0 * ty);
    let x0 = a + (b - a) * ux;
    let x1 = c + (d - c) * ux;
    x0 + (x1 - x0) * uy
}

fn organic_road_weight_2d(
    point: Vec2<f32>,
    start: Vec2<f32>,
    end: Vec2<f32>,
    width: f32,
    falloff: f32,
    line_id: u32,
    organic: f32,
) -> f32 {
    if width <= 0.0 {
        return 0.0;
    }
    let ab = end - start;
    let len_sq = ab.magnitude_squared();
    let (t, mut closest, normal) = if len_sq < 1e-8 {
        (0.0, start, Vec2::new(0.0, 1.0))
    } else {
        let t = ((point - start).dot(ab) / len_sq).clamp(0.0, 1.0);
        let dir = ab.normalized();
        (t, start + ab * t, Vec2::new(-dir.y, dir.x))
    };

    let organic = organic.clamp(0.0, 1.0);
    let seed = line_id as f32 * 0.173 + 11.0;
    if organic > 0.0 {
        let taper = (t * (1.0 - t) * 4.0).clamp(0.0, 1.0);
        let center_wobble = (road_noise1(t * 11.0, seed) * 2.0 - 1.0) * width * 0.18 * organic;
        closest += normal * center_wobble * taper;
    }

    let side = if (point - closest).dot(normal) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let side_seed = seed + if side > 0.0 { 19.0 } else { 53.0 };
    let width_variation = (road_noise1(t * 8.0 + 13.7, seed) * 2.0 - 1.0) * width * 0.24 * organic;
    let side_edge = (road_noise1(t * 18.0 + 3.1, side_seed) * 2.0 - 1.0) * width * 0.30 * organic;
    let local_edge =
        (road_noise2(point * 2.2, side_seed + 7.0) * 2.0 - 1.0) * width * 0.12 * organic;
    let effective_width = (width + width_variation).max(width * 0.35);
    let side_width = (effective_width + side_edge + local_edge).max(width * 0.28);
    let dist = (point - closest).magnitude();

    let mut weight = if dist <= effective_width {
        1.0
    } else if falloff > 0.0 && dist <= side_width + falloff {
        let x = ((dist - side_width) / falloff).clamp(0.0, 1.0);
        let smooth = x * x * (3.0 - 2.0 * x);
        1.0 - smooth
    } else {
        0.0
    };

    if organic > 0.0 && weight > 0.0 {
        let breakup = road_noise2(point * 3.4, seed + 41.0);
        weight *= 1.0 - organic * 0.26 * (1.0 - breakup);
    }

    weight.clamp(0.0, 1.0)
}

fn distance_to_sector_edge_2d(point: Vec2<f32>, sector: &crate::Sector, map: &Map) -> f32 {
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

impl Clone for D2ChunkBuilder {
    fn clone(&self) -> Self {
        D2ChunkBuilder {}
    }
}

impl ChunkBuilder for D2ChunkBuilder {
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
        let sectors = map.sorted_sectors_by_area();
        for sector in &sectors {
            if !sector.intersects_vertical_slice(map, 0.0, 1.0) {
                continue;
            }

            let bbox = sector.bounding_box(map);

            // Collect occluded sectors and store them in the chunk
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut occl_bbox = bbox.clone();
                occl_bbox.expand(Vec2::new(0.1, 0.1));
                chunk.occluded_sectors.push((occl_bbox, occlusion));
            }

            if bbox.intersects(&chunk.bbox) && chunk.bbox.contains(bbox.center()) {
                if let Some(geo) = sector.generate_geometry(map) {
                    let mut vertices: Vec<[f32; 2]> = vec![];
                    let mut uvs: Vec<[f32; 2]> = vec![];

                    let mut repeat = true;
                    if sector.properties.get_int_default("tile_mode", 1) == 0 {
                        repeat = false;
                    }

                    let source = sector.properties.get_default_source();
                    for vertex in &geo.0 {
                        let local = Vec2::new(vertex[0], vertex[1]);

                        if !repeat {
                            let uv = [
                                (vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x),
                                (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y),
                            ];
                            uvs.push(uv);
                        } else {
                            let texture_scale = 1.0;
                            let uv = [
                                (vertex[0] - bbox.min.x) / texture_scale,
                                (vertex[1] - bbox.min.y) / texture_scale,
                            ];
                            uvs.push(uv);
                        }
                        vertices.push([local.x, local.y]);
                    }

                    if let Some(pixelsource) = source {
                        if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                            vmchunk.add_poly_2d(
                                GeoId::Sector(sector.id),
                                tile.id,
                                vertices.clone(),
                                uvs.clone(),
                                geo.1.clone(),
                                0,
                                true,
                            );
                        }
                    }
                }
            }
        }

        // Walls
        for sector in &sectors {
            let bbox = sector.bounding_box(map);
            if bbox.intersects(&chunk.bbox) && chunk.bbox.contains(bbox.center()) {
                if let Some(hash) = sector.generate_wall_geometry_by_linedef(map) {
                    for (linedef_id, geo) in hash.iter() {
                        let mut source = None;

                        if let Some(linedef) = map.find_linedef(*linedef_id) {
                            if let Some(Value::Source(pixelsource)) =
                                linedef.properties.get("row1_source")
                            {
                                source = Some(pixelsource);
                            }
                        }

                        let mut vertices: Vec<[f32; 2]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];
                        let bbox = sector.bounding_box(map);

                        let repeat = true;

                        if let Some(pixelsource) = source {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                for vertex in &geo.0 {
                                    let local = Vec2::new(vertex[0], vertex[1]);

                                    if !repeat {
                                        let uv = [
                                            (vertex[0] - bbox.min.x) / (bbox.max.x - bbox.min.x),
                                            (vertex[1] - bbox.min.y) / (bbox.max.y - bbox.min.y),
                                        ];
                                        uvs.push(uv);
                                    } else {
                                        let texture_scale = 1.0;
                                        let uv = [
                                            (vertex[0] - bbox.min.x) / texture_scale,
                                            (vertex[1] - bbox.min.y) / texture_scale,
                                        ];
                                        uvs.push(uv);
                                    }
                                    vertices.push([local.x, local.y]);
                                }

                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    let batch = Batch2D::new(vertices, geo.1.clone(), uvs)
                                        .repeat_mode(if repeat {
                                            crate::RepeatMode::RepeatXY
                                        } else {
                                            crate::RepeatMode::ClampXY
                                        })
                                        .source(PixelSource::StaticTileIndex(texture_index));
                                    chunk.batches2d.push(batch);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Add standalone walls
        for linedef in &map.linedefs {
            let bbox = linedef.bounding_box(map);
            if bbox.intersects(&chunk.bbox)
                && chunk.bbox.contains(bbox.center())
                && linedef.sector_ids.is_empty()
                && linedef.properties.get_float_default("wall_width", 0.0) > 0.0
            {
                if let Some(hash) =
                    crate::map::geometry::generate_line_segments_d2(map, &[linedef.id])
                {
                    for (_linedef_id, geo) in hash.iter() {
                        let mut source = None;

                        if let Some(Value::Source(pixelsource)) =
                            linedef.properties.get("row1_source")
                        {
                            source = Some(pixelsource);
                        }

                        let mut vertices: Vec<[f32; 2]> = vec![];
                        let mut uvs: Vec<[f32; 2]> = vec![];

                        if let Some(pixelsource) = source {
                            if let Some(tile) = pixelsource.tile_from_tile_list(assets) {
                                if let Some(texture_index) = assets.tile_index(&tile.id) {
                                    for vertex in &geo.0 {
                                        let local = Vec2::new(vertex[0], vertex[1]);

                                        let texture_scale = 1.0;
                                        let uv = [
                                            (vertex[0]) / texture_scale,
                                            (vertex[1]) / texture_scale,
                                        ];
                                        uvs.push(uv);
                                        vertices.push([local.x, local.y]);
                                    }

                                    let batch = Batch2D::new(vertices, geo.1.clone(), uvs)
                                        .repeat_mode(crate::RepeatMode::RepeatXY)
                                        .source(PixelSource::StaticTileIndex(texture_index));
                                    chunk.batches2d.push(batch);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Procedural terrain in 2D (mirrors 3D terrain source, including vertex-based blend overrides).
        if map.properties.get_bool_default("terrain_enabled", false)
            && let Ok(fallback_tile_id) = Uuid::from_str(DEFAULT_TILE_ID)
        {
            let default_tile_id = if let Some(Value::Source(pixel_source)) =
                map.properties.get("default_terrain_tile")
            {
                pixel_source
                    .tile_from_tile_list(assets)
                    .map(|tile| tile.id)
                    .unwrap_or(fallback_tile_id)
            } else {
                fallback_tile_id
            };

            let tile_overrides = map.properties.get("tiles").and_then(|v| {
                if let Value::TileOverrides(map) = v {
                    Some(map)
                } else {
                    None
                }
            });
            let blend_overrides = map.properties.get("blend_tiles").and_then(|v| {
                if let Value::BlendOverrides(map) = v {
                    Some(map)
                } else {
                    None
                }
            });

            // Collect road tile definitions from linedefs (same source as D3 terrain generation).
            let mut road_tile_linedefs: Vec<(
                Vec2<f32>,
                Vec2<f32>,
                f32,
                f32,
                Uuid,
                bool,
                u32,
                f32,
            )> = Vec::new();
            for linedef in &map.linedefs {
                let Some(Value::Source(PixelSource::TileId(tile_id))) =
                    linedef.properties.get("terrain_source")
                else {
                    continue;
                };
                let Some(start_vert) = map.vertices.iter().find(|v| v.id == linedef.start_vertex)
                else {
                    continue;
                };
                let Some(end_vert) = map.vertices.iter().find(|v| v.id == linedef.end_vertex)
                else {
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
                let organic = linedef
                    .properties
                    .get_float_default("terrain_road_organic", 0.0)
                    .clamp(0.0, 1.0);
                road_tile_linedefs.push((
                    start, end, width, falloff, *tile_id, smooth, linedef.id, organic,
                ));
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

            let mut config = TerrainConfig::default();
            let mut chunk_ridge_subdiv = config.subdivisions.max(1);
            for sector in &map.sectors {
                if sector.properties.get_int_default("terrain_mode", 0) != 2 {
                    continue;
                }
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
            config.subdivisions = chunk_ridge_subdiv;
            let generator = TerrainGenerator::new(config);
            if let Some(meshes) =
                generator.generate(map, chunk, assets, default_tile_id, tile_overrides)
            {
                for (tile_id, vertices, indices, uvs) in meshes {
                    let mut tile_batches: FxHashMap<(i32, i32), Vec<(usize, usize, usize)>> =
                        FxHashMap::default();
                    for tri in indices.chunks_exact(3) {
                        let i0 = tri[0] as usize;
                        let i1 = tri[1] as usize;
                        let i2 = tri[2] as usize;

                        let center_u = (uvs[i0][0] + uvs[i1][0] + uvs[i2][0]) / 3.0;
                        let center_v = (uvs[i0][1] + uvs[i1][1] + uvs[i2][1]) / 3.0;
                        let tile_x = center_u.floor() as i32;
                        let tile_z = center_v.floor() as i32;

                        tile_batches
                            .entry((tile_x, tile_z))
                            .or_default()
                            .push((i0, i1, i2));
                    }

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
                        let has_manual_override =
                            has_manual_tile_override || has_manual_blend_override;

                        let road_tile_id = tile_id;
                        let has_road_tile =
                            road_tile_linedefs
                                .iter()
                                .any(|(_, _, width, _, tid, _, _, _)| {
                                    *tid == road_tile_id && *width > 0.0
                                })
                                && !has_manual_override;
                        let has_smooth_road =
                            road_tile_linedefs
                                .iter()
                                .any(|(_, _, width, _, tid, smooth, _, _)| {
                                    *tid == road_tile_id && *smooth && *width > 0.0
                                })
                                && !has_manual_override;
                        let has_ridge_tile = ridge_tile_sectors
                            .iter()
                            .any(|(_, _, _, tid)| *tid == road_tile_id)
                            && !has_manual_override;
                        let has_smooth_ridge =
                            ridge_tile_sectors.iter().any(|(_, _, tile_falloff, tid)| {
                                *tid == road_tile_id && *tile_falloff > 0.0
                            }) && !has_manual_override;
                        let has_vertex_tile = vertex_tile_controls
                            .iter()
                            .any(|(_, _, _, tid)| *tid == road_tile_id)
                            && !has_manual_override;
                        let has_smooth_vertex =
                            vertex_tile_controls
                                .iter()
                                .any(|(_, _, tile_falloff, tid)| {
                                    *tid == road_tile_id && *tile_falloff > 0.0
                                })
                                && !has_manual_override;

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

                        let mut primary_tile_id = tile_id;
                        let mut blend_tile_id = None;
                        let mut blend_preset = None;
                        enum BlendKind {
                            Road,
                            Ridge,
                            Vertex,
                            Preset,
                        }
                        let mut blend_kind = None;
                        if has_smooth_road && bg_tile != road_tile_id {
                            primary_tile_id = bg_tile;
                            blend_tile_id = Some(road_tile_id);
                            blend_kind = Some(BlendKind::Road);
                        } else if has_smooth_ridge && bg_tile != road_tile_id {
                            primary_tile_id = bg_tile;
                            blend_tile_id = Some(road_tile_id);
                            blend_kind = Some(BlendKind::Ridge);
                        } else if has_smooth_vertex && bg_tile != road_tile_id {
                            primary_tile_id = bg_tile;
                            blend_tile_id = Some(road_tile_id);
                            blend_kind = Some(BlendKind::Vertex);
                        } else if !has_road_tile && !has_ridge_tile && !has_vertex_tile {
                            blend_tile_id = blend_overrides
                                .and_then(|m| m.get(&(tile_x, tile_z)))
                                .and_then(|(_, ps)| ps.tile_from_tile_list(assets))
                                .map(|tile| tile.id);
                            blend_preset =
                                blend_overrides.and_then(|m| m.get(&(tile_x, tile_z)).map(|v| v.0));
                            if blend_tile_id.is_some() {
                                blend_kind = Some(BlendKind::Preset);
                            }
                        }

                        let mut out_vertices = Vec::with_capacity(triangles.len() * 3);
                        let mut out_uvs = Vec::with_capacity(triangles.len() * 3);
                        let mut out_indices = Vec::with_capacity(triangles.len());
                        let mut out_blend_weights = Vec::with_capacity(triangles.len() * 3);

                        for (i0, i1, i2) in triangles {
                            let base = out_vertices.len();
                            let v0 = vertices[i0];
                            let v1 = vertices[i1];
                            let v2 = vertices[i2];

                            out_vertices.push([v0.x, v0.z]);
                            out_vertices.push([v1.x, v1.z]);
                            out_vertices.push([v2.x, v2.z]);

                            out_uvs.push(uvs[i0]);
                            out_uvs.push(uvs[i1]);
                            out_uvs.push(uvs[i2]);
                            out_indices.push((base, base + 1, base + 2));

                            for &vi in &[i0, i1, i2] {
                                let uv = uvs[vi];
                                let p = Vec2::new(uv[0], uv[1]);
                                let weight = match blend_kind {
                                    Some(BlendKind::Road) => {
                                        let mut w = 0.0f32;
                                        for &(
                                            a,
                                            b,
                                            width,
                                            falloff,
                                            tid,
                                            line_smooth,
                                            line_id,
                                            organic,
                                        ) in &road_tile_linedefs
                                        {
                                            if !line_smooth || tid != road_tile_id || width <= 0.0 {
                                                continue;
                                            }
                                            let this_w = organic_road_weight_2d(
                                                p, a, b, width, falloff, line_id, organic,
                                            );
                                            if this_w > w {
                                                w = this_w;
                                            }
                                        }
                                        w
                                    }
                                    Some(BlendKind::Ridge) => {
                                        let mut w = 0.0f32;
                                        for &(sector_id, plateau, tile_falloff, tid) in
                                            &ridge_tile_sectors
                                        {
                                            if tid != road_tile_id {
                                                continue;
                                            }
                                            let Some(sector) = map.find_sector(sector_id) else {
                                                continue;
                                            };
                                            let dist = distance_to_sector_edge_2d(p, sector, map);
                                            let this_w = if dist <= plateau {
                                                1.0
                                            } else if tile_falloff > 0.0
                                                && dist <= plateau + tile_falloff
                                            {
                                                1.0 - ((dist - plateau) / tile_falloff)
                                            } else {
                                                0.0
                                            };
                                            if this_w > w {
                                                w = this_w;
                                            }
                                        }
                                        w
                                    }
                                    Some(BlendKind::Vertex) => {
                                        let mut w = 0.0f32;
                                        for &(center, radius, tile_falloff, tid) in
                                            &vertex_tile_controls
                                        {
                                            if tid != road_tile_id {
                                                continue;
                                            }
                                            let dist = (p - center).magnitude();
                                            let this_w = if dist <= radius {
                                                1.0
                                            } else if tile_falloff > 0.0
                                                && dist <= radius + tile_falloff
                                            {
                                                1.0 - ((dist - radius) / tile_falloff)
                                            } else {
                                                0.0
                                            };
                                            if this_w > w {
                                                w = this_w;
                                            }
                                        }
                                        w
                                    }
                                    Some(BlendKind::Preset) => {
                                        if let Some(preset) = blend_preset {
                                            let weights = preset.weights();
                                            let u = (uv[0] - tile_x as f32).clamp(0.0, 1.0);
                                            let v = (uv[1] - tile_z as f32).clamp(0.0, 1.0);
                                            let top = weights[0] * (1.0 - u) + weights[1] * u;
                                            let bottom = weights[3] * (1.0 - u) + weights[2] * u;
                                            top * (1.0 - v) + bottom * v
                                        } else {
                                            0.0
                                        }
                                    }
                                    None => 0.0,
                                };
                                out_blend_weights.push(weight.clamp(0.0, 1.0));
                            }
                        }

                        if let Some(tile_id2) = blend_tile_id {
                            vmchunk.add_poly_2d_blended(
                                GeoId::Terrain(tile_x, tile_z),
                                primary_tile_id,
                                tile_id2,
                                out_vertices,
                                out_uvs,
                                out_blend_weights,
                                out_indices,
                                0,
                                true,
                            );
                        } else {
                            vmchunk.add_poly_2d(
                                GeoId::Terrain(tile_x, tile_z),
                                primary_tile_id,
                                out_vertices,
                                out_uvs,
                                out_indices,
                                0,
                                true,
                            );
                        }
                    }
                }
            }
        }
    }
}
