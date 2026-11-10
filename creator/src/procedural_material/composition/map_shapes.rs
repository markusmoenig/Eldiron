//! Lower existing geometry-driven demo layers to self-contained MZ mixers.
//! Rect Compose only authors tile shapes; geometry layers have no binding UI.
use super::*;
use std::fmt::Write;

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
pub(super) fn distance(
    project: &Project,
    layer: &CompositionLayer,
) -> Result<Option<(String, String)>, String> {
    let Some(geometry) = &layer.geometry else {
        return Ok(Some((String::new(), "-1000".into())));
    };
    let map = project
        .regions
        .iter()
        .map(|r| &r.map)
        .chain(project.screens.values().map(|s| &s.map))
        .find(|m| m.id == geometry.map)
        .ok_or_else(|| fl!("mz_shape_geometry_missing"))?;
    let mut code = String::new();
    let mut count = 0;
    if layer.shape == CompositionShape::MapPath {
        let mut distances = Vec::new();
        for id in &geometry.linedefs {
            let line = map
                .find_linedef(*id)
                .ok_or_else(|| fl!("mz_shape_geometry_missing"))?;
            let a = map
                .get_vertex(line.start_vertex)
                .ok_or_else(|| fl!("mz_shape_geometry_missing"))?;
            let b = map
                .get_vertex(line.end_vertex)
                .ok_or_else(|| fl!("mz_shape_geometry_missing"))?;
            distances.push(segment(&mut code, a, b, count));
            count += 1;
        }
        if distances.is_empty() {
            return Err(fl!("mz_shape_geometry_missing"));
        }
        if count > 256 {
            return Err(fl!("mz_shape_limit"));
        }
        let nearest = reduce(&mut code, distances, "min", "path");
        Ok(Some((
            code,
            format!("{} - {nearest}", layer.position * 0.5),
        )))
    } else {
        let mut areas = Vec::new();
        for (polygon, id) in geometry.sectors.iter().enumerate() {
            let points = map
                .find_sector(*id)
                .and_then(|s| s.vertices_world(map))
                .ok_or_else(|| fl!("mz_shape_geometry_missing"))?;
            if points.len() < 3 {
                return Err(fl!("mz_shape_closed_area"));
            }
            let mut distances = Vec::new();
            let mut crossings = Vec::new();
            for (a, b) in points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .take(points.len())
            {
                let a = Vec2::new(a.x, a.z);
                let b = Vec2::new(b.x, b.z);
                distances.push(segment(&mut code, a, b, count));
                let crossing = format!("crossing_{count}");
                writeln!(code,"let {crossing} = select(logical_and(different(greater({}, uv.y), greater({}, uv.y)), less(uv.x, {} + (uv.y - {}) * {} / nonzero({}))), 1, 0)",a.y,b.y,a.x,a.y,b.x-a.x,b.y-a.y).unwrap();
                crossings.push(crossing);
                count += 1;
            }
            if count > 256 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use denrim_materials::{cpu::Evaluator, ir::Channel};
    use shared::material_composition::CompositionGeometry;
    fn program(project: &Project, layer: &CompositionLayer) -> denrim_materials::ir::Program {
        let mut source =
            String::from("project 1\nmaterial \"surface\" called \"Shape\"\n  mode pbr\n");
        append_mask_row(
            &mut source,
            "shape",
            "shape",
            &shape_code_with_distance(layer, distance(project, layer).unwrap()),
        )
        .unwrap();
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
}
