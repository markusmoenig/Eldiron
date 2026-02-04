pub mod material;
pub mod shape;
pub mod shapecontext;
pub mod shapefx;
pub mod shapefxgraph;
pub mod tilebuilder;

use crate::{Assets, BBox, Map, PixelSource, Sector, ShapeContext, ShapeFXGraph, Texture, Value};
use rayon::prelude::*;
use theframework::prelude::*;
use vek::{Vec2, Vec4};

pub struct ShapeStack {
    area_min: Vec2<f32>,
    area_max: Vec2<f32>,
}

impl ShapeStack {
    pub fn new(area_min: Vec2<f32>, area_max: Vec2<f32>) -> Self {
        Self { area_min, area_max }
    }

    /// Render the geometry into a material texture
    pub fn render_geometry(
        &mut self,
        buffer: &mut Texture,
        map: &Map,
        assets: &Assets,
        material_mode: bool,
        sector_overrides: &FxHashMap<u32, Vec4<f32>>,
    ) {
        let width = buffer.width;
        let height = buffer.height;
        let area_size = self.area_max - self.area_min;

        let sector_graph_name = if material_mode {
            "source"
        } else {
            "shape_graph"
        };

        let linedef_graph_name = if material_mode {
            "row1_source"
        } else {
            "shape_graph"
        };

        let grid_offsets: [Vec2<i32>; 9] = [
            Vec2::new(-1, -1),
            Vec2::new(0, -1),
            Vec2::new(1, -1),
            Vec2::new(-1, 0),
            Vec2::new(0, 0),
            Vec2::new(1, 0),
            Vec2::new(-1, 1),
            Vec2::new(0, 1),
            Vec2::new(1, 1),
        ];

        let offsets: &[Vec2<i32>] = if material_mode {
            &grid_offsets
        } else {
            &grid_offsets[4..5] // only center
        };

        let px = area_size.x / width as f32;

        struct ResolvedSector<'a> {
            sector: &'a Sector,
            bbox: BBox,
            graph: &'a ShapeFXGraph,
            rounding: f32,
            aa: f32,
            edges: Vec<(Vec2<f32>, Vec2<f32>)>,
        }

        let mut map = map.clone();
        for graph in map.shapefx_graphs.values_mut() {
            for node in graph.nodes.iter_mut() {
                node.render_setup(0.0);
            }
        }

        let resolved_sectors: Vec<ResolvedSector> = map
            .sorted_sectors_by_area()
            .into_iter()
            .filter_map(|sector| {
                let bbox = sector.bounding_box(&map);
                let mut edges = Vec::new();

                for &linedef_id in &sector.linedefs {
                    if let Some(ld) = map.find_linedef(linedef_id) {
                        if let (Some(v0), Some(v1)) = (
                            map.get_vertex(ld.start_vertex),
                            map.get_vertex(ld.end_vertex),
                        ) {
                            edges.push((v0, v1));
                        }
                    }
                }

                if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                    sector.properties.get(sector_graph_name)
                {
                    if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                        let rounding = graph.nodes[0].values.get_float_default("rounding", 0.0);
                        let aa = sector.properties.get_float_default("material_a_a", 1.0);

                        return Some(ResolvedSector {
                            sector,
                            bbox,
                            graph,
                            rounding,
                            aa,
                            edges,
                        });
                    }
                }

                None
            })
            .collect();

        buffer
            .data
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as f32;
                    let y = (i / width) as f32;

                    let uv = Vec2::new(x / width as f32, 1.0 - y / height as f32);
                    let world = self.area_min + uv * area_size;

                    let mut color = Vec4::new(
                        pixel[0] as f32 / 255.0,
                        pixel[1] as f32 / 255.0,
                        pixel[2] as f32 / 255.0,
                        pixel[3] as f32 / 255.0,
                    );

                    // Do the sectors
                    for resolved in &resolved_sectors {
                        let bbox = resolved.bbox;

                        let mut best_ctx = None;
                        let mut min_sdf = f32::MAX;

                        for &offset_i in offsets {
                            let offset = Vec2::new(
                                offset_i.x as f32 * area_size.x,
                                offset_i.y as f32 * area_size.y,
                            );

                            let shifted_point = world - offset;

                            let uv = Vec2::new(
                                (shifted_point.x - bbox.min.x) / (bbox.max.x - bbox.min.x),
                                (shifted_point.y - bbox.min.y) / (bbox.max.y - bbox.min.y),
                            );

                            // Get the distance using the precomputed sector edges
                            let mut min_dist = f32::MAX;
                            for &(v0, v1) in &resolved.edges {
                                let edge = v1 - v0;
                                let to_point = shifted_point - v0;

                                let t = to_point.dot(edge) / edge.dot(edge);
                                let t_clamped = t.clamp(0.0, 1.0);
                                let closest = v0 + edge * t_clamped;

                                let dist = (shifted_point - closest).magnitude();
                                min_dist = min_dist.min(dist);
                            }

                            let inside = resolved.sector.is_inside(&map, shifted_point);
                            let distance = if inside { -min_dist } else { min_dist };
                            let sdf = distance / px - resolved.rounding;

                            if sdf < min_sdf {
                                min_sdf = sdf;
                                best_ctx = Some(ShapeContext {
                                    point_world: shifted_point,
                                    point: shifted_point / px,
                                    uv,
                                    distance_world: distance,
                                    distance: sdf,
                                    shape_id: resolved.sector.id,
                                    px,
                                    anti_aliasing: resolved.aa,
                                    t: None,
                                    line_dir: None,
                                    override_color: None,
                                });
                            }
                        }

                        if let Some(mut ctx) = best_ctx {
                            if let Some(color) = sector_overrides.get(&ctx.shape_id) {
                                ctx.override_color = Some(*color);
                            }

                            if let Some(col) = resolved.graph.evaluate_material(&ctx, color, assets)
                            {
                                color = Vec4::lerp(color, col, col.w);
                            }
                        }
                    }

                    // And now the standalone linedefs
                    for linedef in &map.linedefs {
                        if linedef.sector_ids.is_empty() {
                            if let Some(Value::Source(PixelSource::ShapeFXGraphId(graph_id))) =
                                linedef.properties.get(linedef_graph_name)
                            {
                                if let Some(graph) = map.shapefx_graphs.get(graph_id) {
                                    let line_width_px =
                                        graph.nodes[0].values.get_float_default("line_width", 1.0);

                                    if let Some(start) = map.find_vertex(linedef.start_vertex) {
                                        if let Some(end) = map.find_vertex(linedef.end_vertex) {
                                            let a = start.as_vec2();
                                            let b = end.as_vec2();

                                            let tile_size = Vec2::new(10.0, 10.0); // or store in graph
                                            let px = tile_size.x / width as f32;

                                            let ab = b - a;
                                            let ab_len = ab.magnitude();
                                            let ab_dir = ab / ab_len;
                                            // let normal = Vec2::new(-ab_dir.y, ab_dir.x);

                                            let mut min_sdf = f32::MAX;
                                            let mut final_t = 0.0;
                                            let mut final_dir = Vec2::zero();

                                            for &offset_i in offsets {
                                                let offset = Vec2::new(
                                                    offset_i.x as f32 * tile_size.x,
                                                    offset_i.y as f32 * tile_size.y,
                                                );

                                                let shifted_point = world - offset;
                                                let ap = shifted_point - a;

                                                let t = ap.dot(ab_dir) / ab_len;
                                                let t_clamped = t.clamp(0.0, 1.0);
                                                let closest = a + ab_dir * (t_clamped * ab_len);

                                                let sdf_px = (shifted_point - closest).magnitude()
                                                    / px
                                                    - line_width_px * 0.5;

                                                if sdf_px < min_sdf {
                                                    min_sdf = sdf_px;
                                                    final_t = t;
                                                    final_dir = ab_dir;
                                                }
                                            }

                                            let ctx = ShapeContext {
                                                point_world: world,
                                                point: world / px,
                                                uv: Vec2::new(final_t.fract(), 0.5 + min_sdf), // optional, depends on effect
                                                distance_world: min_sdf * px,
                                                distance: min_sdf,
                                                shape_id: 0,
                                                px,
                                                anti_aliasing: linedef
                                                    .properties
                                                    .get_float_default("material_a_a", 1.0),
                                                t: Some(final_t),
                                                line_dir: Some(final_dir),
                                                override_color: None,
                                            };

                                            if let Some(col) =
                                                graph.evaluate_material(&ctx, color, assets)
                                            {
                                                color = Vec4::lerp(color, col, col.w);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });
    }
}
