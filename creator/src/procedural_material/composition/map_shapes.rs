//! Lower existing geometry-driven demo layers to self-contained MZ mixers.
//! Rect Compose only authors tile shapes; geometry layers have no binding UI.
use super::*;
use shared::material_composition::CompositionGeometry;
use std::fmt::Write;

type Edge = (Vec2<f32>, Vec2<f32>);
pub(super) const MAX_EDGES: usize = 256;

fn reduce(code: &mut String, mut terms: Vec<String>, operation: &str, prefix: &str) -> String {
    if terms.is_empty() {
        return "1000".into();
    }
    let mut group = 0;
    while terms.len() > 1 {
        terms = terms
            .chunks(6)
            .map(|chunk| {
                let mut iter = chunk.iter();
                let mut value = iter.next().unwrap().clone();
                for next in iter {
                    value = if operation == "sum" {
                        format!("({value} + {next})")
                    } else {
                        format!("{operation}({value}, {next})")
                    };
                }
                let name = format!("{prefix}_{group}");
                group += 1;
                writeln!(code, "let {name} = {value}").unwrap();
                name
            })
            .collect();
    }
    terms.pop().unwrap()
}
fn segment(code: &mut String, a: Vec2<f32>, b: Vec2<f32>, id: usize) -> String {
    let name = format!("segment_{id}");
    writeln!(code, "let delta_{id} = vec2({}, {})", b.x - a.x, b.y - a.y).unwrap();
    writeln!(code,"let t_{id} = clamp(dot(uv - vec2({}, {}), delta_{id}) / max(dot(delta_{id}, delta_{id}), 0.000001), 0, 1)",a.x,a.y).unwrap();
    writeln!(
        code,
        "let {name} = length(uv - vec2({}, {}) - t_{id} * delta_{id})",
        a.x, a.y
    )
    .unwrap();
    name
}
fn owning_map<'a>(project: &'a Project, geometry: &CompositionGeometry) -> Result<&'a Map, String> {
    project
        .regions
        .iter()
        .map(|r| &r.map)
        .chain(project.screens.values().map(|s| &s.map))
        .find(|m| m.id == geometry.map)
        .ok_or_else(|| fl!("mz_shape_geometry_missing"))
}
fn path_edges(
    map: &Map,
    geometry: &CompositionGeometry,
    width: f32,
) -> Result<Vec<(Edge, f32)>, String> {
    geometry
        .linedefs
        .iter()
        .map(|id| {
            let ends = || {
                let line = map.find_linedef(*id)?;
                let segment_width = geometry.path_width_source.map_or(width, |source| {
                    let (key, default) = source.property();
                    line.properties
                        .get_float_default(key, default)
                        .clamp(0.1, 64.)
                });
                Some((
                    (
                        map.get_vertex(line.start_vertex)?,
                        map.get_vertex(line.end_vertex)?,
                    ),
                    segment_width * 0.5,
                ))
            };
            ends().ok_or_else(|| fl!("mz_shape_geometry_missing"))
        })
        .collect()
}
fn stroke_distance(code: &mut String, edges: &[(Edge, f32)]) -> String {
    let distances = edges
        .iter()
        .enumerate()
        .map(|(index, &((a, b), half_width))| {
            let nearest = segment(code, a, b, index);
            let name = format!("stroke_{index}");
            writeln!(code, "let {name} = {half_width} - {nearest}").unwrap();
            name
        })
        .collect();
    reduce(code, distances, "max", "path")
}
fn polygon_edges(map: &Map, id: u32) -> Result<Vec<Edge>, String> {
    let points = map
        .find_sector(id)
        .and_then(|s| s.vertices_world(map))
        .ok_or_else(|| fl!("mz_shape_geometry_missing"))?;
    if points.len() < 3 {
        return Err(fl!("mz_shape_closed_area"));
    }
    Ok(points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| (Vec2::new(a.x, a.z), Vec2::new(b.x, b.z)))
        .collect())
}
fn crossing(code: &mut String, a: Vec2<f32>, b: Vec2<f32>, id: usize) -> String {
    let name = format!("crossing_{id}");
    writeln!(code,"let {name} = select(logical_and(different(greater({}, uv.y), greater({}, uv.y)), less(uv.x, {} + (uv.y - {}) * {} / nonzero({}))), 1, 0)",a.y,b.y,a.x,a.y,b.x-a.x,b.y-a.y).unwrap();
    name
}
fn distance(
    project: &Project,
    layer: &CompositionLayer,
) -> Result<Option<(String, String)>, String> {
    let Some(geometry) = &layer.geometry else {
        return Ok(Some((String::new(), "-1000".into())));
    };
    let map = owning_map(project, geometry)?;
    let mut code = String::new();
    let mut count = 0;
    if layer.shape == CompositionShape::MapPath {
        let edges = path_edges(map, geometry, layer.position)?;
        if edges.is_empty() {
            return Err(fl!("mz_shape_geometry_missing"));
        }
        if edges.len() > MAX_EDGES {
            return Err(fl!("mz_shape_limit"));
        }
        if geometry.path_width_source.is_some() {
            let distance = stroke_distance(&mut code, &edges);
            return Ok(Some((code, distance)));
        }
        let mut distances = Vec::new();
        for ((a, b), _) in edges {
            distances.push(segment(&mut code, a, b, count));
            count += 1;
        }
        let nearest = reduce(&mut code, distances, "min", "path");
        Ok(Some((
            code,
            format!("{} - {nearest}", layer.position * 0.5),
        )))
    } else {
        let mut areas = Vec::new();
        for (polygon, id) in geometry.sectors.iter().enumerate() {
            let mut distances = Vec::new();
            let mut crossings = Vec::new();
            for (a, b) in polygon_edges(map, *id)? {
                distances.push(segment(&mut code, a, b, count));
                crossings.push(crossing(&mut code, a, b, count));
                count += 1;
            }
            if count > MAX_EDGES {
                return Err(fl!("mz_shape_limit"));
            }
            let nearest = reduce(
                &mut code,
                distances,
                "min",
                &format!("area_distance_{polygon}"),
            );
            let inside = reduce(
                &mut code,
                crossings,
                "sum",
                &format!("area_crossings_{polygon}"),
            );
            let name = format!("area_{polygon}");
            writeln!(
                code,
                "let {name} = (mod({inside}, 2) * 2 - 1) * {nearest} - {}",
                layer.position
            )
            .unwrap();
            areas.push(name);
        }
        if areas.is_empty() {
            return Err(fl!("mz_shape_geometry_missing"));
        }
        let area = reduce(&mut code, areas, "max", "areas");
        Ok(Some((code, area)))
    }
}

/// Keep large shapes within TileLang's per-tile instruction budget. A row
/// carries unsigned distance and crossing count until the polygon is complete.
/// Feathering and edge noise are applied once, after taking the shape union.
pub(super) fn append_mask(
    recipe: &mut String,
    project: &Project,
    layer: &CompositionLayer,
    tile: &str,
) -> Result<(), String> {
    let expression = shape_code_with_distance(layer, distance(project, layer)?);
    if expression.lines().count() <= 128 {
        return append_mask_row(recipe, "shape", tile, &expression);
    }
    let geometry = layer
        .geometry
        .as_ref()
        .ok_or_else(|| fl!("mz_shape_geometry_missing"))?;
    let map = owning_map(project, geometry)?;
    let mut rows = MaskRows {
        recipe,
        tile,
        next: 0,
    };
    let (input, distance) = if layer.shape == CompositionShape::MapPath {
        let input = rows.stroke(&path_edges(map, geometry, layer.position)?)?;
        (input, "color.x".into())
    } else {
        let mut areas = Vec::new();
        for id in &geometry.sectors {
            let edges = rows.polygon(&polygon_edges(map, *id)?)?;
            let signed = rows.push(
                Kernel::Color,
                &[("color", &edges)],
                "let signed = (mod(color.y, 2) * 2 - 1) * color.x\nimage = vec4(signed, 0, 0, 1)",
            )?;
            areas.push(signed);
        }
        let input = rows.combine(areas, "max")?;
        (input, format!("color.x - {}", layer.position))
    };
    rows.recipe
        .push_str("  mixing mask from row \"shape\" xyz\n");
    append_algorithm_row(
        rows.recipe,
        "shape",
        tile,
        Kernel::Color,
        &[("color", &input)],
        &shape_code_with_distance(layer, Some((String::new(), distance))),
    )
}

struct MaskRows<'a> {
    recipe: &'a mut String,
    tile: &'a str,
    next: usize,
}

impl MaskRows<'_> {
    fn push(
        &mut self,
        kernel: Kernel,
        inputs: &[(&str, &str)],
        code: &str,
    ) -> Result<String, String> {
        let row = format!("{}-{}", self.tile, self.next);
        self.next += 1;
        append_algorithm_row(self.recipe, &row, &row, kernel, inputs, code)?;
        Ok(row)
    }
    fn stroke(&mut self, edges: &[(Edge, f32)]) -> Result<String, String> {
        let mut groups = Vec::new();
        for group in edges.chunks(16) {
            let mut code = String::new();
            let distance = stroke_distance(&mut code, group);
            writeln!(code, "image = vec4({distance}, 0, 0, 1)").unwrap();
            groups.push(self.push(Kernel::Color, &[], &code)?);
        }
        self.combine(groups, "max")
    }
    fn polygon(&mut self, edges: &[Edge]) -> Result<String, String> {
        let mut groups = Vec::new();
        for group in edges.chunks(16) {
            let mut code = String::new();
            let mut distances = Vec::with_capacity(group.len());
            let mut crossings = Vec::with_capacity(group.len());
            for (index, &(a, b)) in group.iter().enumerate() {
                distances.push(segment(&mut code, a, b, index));
                crossings.push(crossing(&mut code, a, b, index));
            }
            let nearest = reduce(&mut code, distances, "min", "distance");
            let count = reduce(&mut code, crossings, "sum", "crossings");
            writeln!(code, "image = vec4({nearest}, {count}, 0, 1)").unwrap();
            groups.push(self.push(Kernel::Color, &[], &code)?);
        }
        self.combine(groups, "min")
    }
    fn combine(&mut self, mut rows: Vec<String>, operation: &str) -> Result<String, String> {
        let code =
            format!("image = vec4({operation}(color1.x, color2.x), color1.y + color2.y, 0, 1)");
        while rows.len() > 1 {
            let mut pairs = rows.into_iter();
            rows = Vec::new();
            while let Some(a) = pairs.next() {
                rows.push(match pairs.next() {
                    Some(b) => {
                        self.push(Kernel::ColorBlend, &[("color1", &a), ("color2", &b)], &code)?
                    }
                    None => a,
                });
            }
        }
        rows.pop().ok_or_else(|| fl!("mz_shape_geometry_missing"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use denrim_materials::{cpu::Evaluator, ir::Channel};
    use shared::material_composition::CompositionGeometry;
    fn program(project: &Project, layer: &CompositionLayer) -> denrim_materials::ir::Program {
        let mut source =
            String::from("project 1\nmaterial \"surface\" called \"Shape\"\n  mode pbr\n");
        append_mask(&mut source, project, layer, "shape").unwrap();
        source = source.replace("  mixing mask from row \"shape\" xyz\n", "");
        source.push_str("  output base color from row \"shape\" xyz\n");
        denrim_materials::compile(&source).unwrap()
    }
    #[test]
    fn map_paths_join_across_cells_and_follow_geometry_edits() {
        let mut p = Project::default();
        p.regions = vec![shared::region::Region::new()];
        let map = &mut p.regions[0].map;
        let a = map.add_vertex_at(0., 0.);
        let b = map.add_vertex_at(4., 0.);
        let c = map.add_vertex_at(4., 4.);
        let first = map.create_linedef_manual(a, b);
        map.clear_temp();
        let second = map.create_linedef_manual(b, c);
        map.clear_temp();
        let mut layer = CompositionLayer::split(Uuid::new_v4());
        layer.shape = CompositionShape::MapPath;
        layer.position = 1.35;
        layer.feather = 0.45;
        layer.wobble = 0.13;
        layer.geometry = Some(CompositionGeometry {
            map: map.id,
            linedefs: vec![first, second],
            sectors: vec![],
            path_width_source: None,
        });
        let code = program(&p, &layer);
        let eval = Evaluator::new(&code);
        for i in 0..=80 {
            let t = i as f32 / 20.;
            for uv in [[t, 0.], [4., t]] {
                assert!(eval.sample("surface", Channel::BaseColor, uv).unwrap()[0] > 0.99);
            }
        }
        for boundary in [1., 2., 3.] {
            let a = eval
                .sample("surface", Channel::BaseColor, [boundary - 0.00001, 0.65])
                .unwrap()[0];
            let b = eval
                .sample("surface", Channel::BaseColor, [boundary + 0.00001, 0.65])
                .unwrap()[0];
            assert!((a - b).abs() < 0.001);
        }
        assert_eq!(
            eval.sample("surface", Channel::BaseColor, [2., 2.])
                .unwrap()[0],
            0.
        );
        p.regions[0]
            .map
            .vertices
            .iter_mut()
            .find(|v| v.id == c)
            .unwrap()
            .x = 7.;
        let moved = program(&p, &layer);
        let eval = Evaluator::new(&moved);
        assert!(
            eval.sample("surface", Channel::BaseColor, [7., 4.])
                .unwrap()[0]
                > 0.99
        );
        assert_eq!(
            eval.sample("surface", Channel::BaseColor, [4., 4.])
                .unwrap()[0],
            0.
        );
    }
    #[test]
    fn map_areas_fill_disconnected_sectors_without_custom_masks() {
        let mut p = Project::default();
        p.regions = vec![shared::region::Region::new()];
        let map = &mut p.regions[0].map;
        let mut sectors = Vec::new();
        for offset in [0., 8.] {
            map.clear_temp();
            let vertices = [
                [offset, 0.],
                [offset + 4., 0.],
                [offset + 4., 4.],
                [offset, 4.],
            ]
            .map(|p| map.add_vertex_at(p[0], p[1]));
            for i in 0..4 {
                map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
            }
            sectors.push(map.close_polygon_manual().unwrap());
        }
        let mut layer = CompositionLayer::split(Uuid::new_v4());
        layer.shape = CompositionShape::MapArea;
        layer.position = 0.3;
        layer.feather = 0.6;
        layer.wobble = 0.1;
        layer.geometry = Some(CompositionGeometry {
            map: map.id,
            linedefs: vec![],
            sectors,
            path_width_source: None,
        });
        assert!(layer.mask.is_none());
        let code = program(&p, &layer);
        let eval = Evaluator::new(&code);
        for uv in [[2., 2.], [10., 2.]] {
            assert!(eval.sample("surface", Channel::BaseColor, uv).unwrap()[0] > 0.99);
        }
        for uv in [[6., 2.], [-1., 2.], [2., 5.]] {
            assert_eq!(
                eval.sample("surface", Channel::BaseColor, uv).unwrap()[0],
                0.
            );
        }
    }

    #[test]
    fn large_terrain_masks_keep_distance_and_polygon_parity_across_rows() {
        let mut project = Project::default();
        project.regions = vec![shared::region::Region::new()];
        let map = &mut project.regions[0].map;
        let vertices: Vec<_> = (0..48)
            .map(|index| {
                let angle = index as f32 * std::f32::consts::TAU / 48.;
                let radius = if index % 2 == 0 { 4. } else { 3. };
                map.add_vertex_at(radius * angle.cos(), radius * angle.sin())
            })
            .collect();
        let edges: Vec<_> = (0..vertices.len())
            .map(|index| {
                map.create_linedef_manual(vertices[index], vertices[(index + 1) % vertices.len()])
            })
            .collect();
        let area = map.close_polygon_manual().unwrap();
        let mut layer = CompositionLayer::split(Uuid::new_v4());
        layer.position = 0.1;
        layer.feather = 0.2;
        layer.wobble = 0.0;
        layer.geometry = Some(CompositionGeometry {
            map: map.id,
            sectors: vec![area],
            linedefs: edges,
            path_width_source: None,
        });
        for (shape, inside, outside) in [
            (CompositionShape::MapArea, [0., 0.], [5., 0.]),
            (CompositionShape::MapPath, [4., 0.], [0., 0.]),
        ] {
            layer.shape = shape;
            if shape == CompositionShape::MapPath {
                layer.position = 0.8;
            }
            let code = program(&project, &layer);
            let eval = Evaluator::new(&code);
            let mask = |point| eval.sample("surface", Channel::BaseColor, point).unwrap()[0];
            assert!(mask(inside) > 0.99);
            assert!(mask(outside) < 0.01);
        }
    }
}
