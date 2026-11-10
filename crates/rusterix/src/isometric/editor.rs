//! Authoring overlays for the sprite iso view. Picking shares the same geometry filter.
use super::{Projection, line, terrain_rectangle_guide};
use crate::scene_handler::{EDITOR_CURSOR_COLOR, EDITOR_GEOMETRY_COLOR, EDITOR_SELECTION_COLOR};
use crate::{Map, MapToolType};
use rustc_hash::{FxHashMap, FxHashSet};
use theframework::prelude::*;

#[derive(Clone, Copy)]
pub struct EditorGuides {
    pub grid: bool,
    pub hover: Option<Vec2<f32>>,
    pub geometry_hover: (Option<u32>, Option<u32>, Option<u32>),
    pub tool: MapToolType,
    pub hide_rect_geometry: bool,
    pub editing_slice: f32,
    pub editing_slice_height: f32,
}

/// Geometry on the active iso construction level, excluding hidden tile footprints.
/// The same IDs drive both guides and screen-space picking in the creator.
pub fn editing_geometry(map: &Map, hide_rect_geometry: bool) -> (FxHashSet<u32>, FxHashSet<u32>) {
    let height = map.properties.get_float_default("iso_edit_height", 0.);
    let hidden: FxHashSet<_> = if hide_rect_geometry {
        map.sectors
            .iter()
            .filter(|s| s.layer.is_some())
            .flat_map(|s| s.linedefs.iter().copied())
            .collect()
    } else {
        FxHashSet::default()
    };
    let mut lines: FxHashSet<_> = map
        .sectors
        .iter()
        .filter(|s| (s.properties.get_float_default("floor_height", 0.) - height).abs() < 0.001)
        .flat_map(|s| s.linedefs.iter().copied())
        .filter(|id| !hidden.contains(id))
        .collect();
    if height.abs() < 0.001 {
        lines.extend(
            map.linedefs
                .iter()
                .filter(|l| l.sector_ids.is_empty() && !hidden.contains(&l.id))
                .map(|l| l.id),
        );
    }
    let mut vertices: FxHashSet<_> = map
        .linedefs
        .iter()
        .filter(|l| lines.contains(&l.id))
        .flat_map(|l| [l.start_vertex, l.end_vertex])
        .collect();
    if height.abs() < 0.001 {
        let used: FxHashSet<_> = map
            .linedefs
            .iter()
            .flat_map(|l| [l.start_vertex, l.end_vertex])
            .collect();
        vertices.extend(
            map.vertices
                .iter()
                .filter(|v| !used.contains(&v.id))
                .map(|v| v.id),
        );
    }
    vertices.extend(
        map.vertices
            .iter()
            .filter(|v| v.properties.get_bool_default("terrain_control", false))
            .map(|v| v.id),
    );
    (vertices, lines)
}

/// Project an authored edge along the same terrain used for cursor picking.
/// `projection` includes the active construction-plane offset.
pub fn projected_segment(
    map: &Map,
    projection: Projection,
    a: Vec2<f32>,
    b: Vec2<f32>,
) -> Vec<Vec2<f32>> {
    projected_segment_at_height(
        map,
        projection,
        a,
        b,
        map.properties.get_float_default("iso_edit_height", 0.),
    )
}

fn projected_segment_at_height(
    map: &Map,
    projection: Projection,
    a: Vec2<f32>,
    b: Vec2<f32>,
    height: f32,
) -> Vec<Vec2<f32>> {
    let surface = (height.abs() < 0.001)
        .then(|| crate::chunkbuilder::terrain_surface::TerrainSurface::cached(map))
        .flatten();
    let steps = if surface.is_some() {
        ((b - a).magnitude() * 8.).ceil().clamp(1., 512.) as usize
    } else {
        1
    };
    (0..=steps)
        .map(|i| {
            let q = a + (b - a) * (i as f32 / steps as f32);
            projection.screen(
                q,
                surface.as_ref().and_then(|t| t.height_at(q)).unwrap_or(0.),
            )
        })
        .collect()
}

/// Guides sit on terrain or the active construction plane, over the rendered scene.
pub(super) fn draw_guides(
    map: &Map,
    buffer: &mut TheRGBABuffer,
    p: Projection,
    guides: EditorGuides,
) {
    let surface = if map
        .properties
        .get_float_default("iso_edit_height", 0.)
        .abs()
        < 0.001
    {
        crate::chunkbuilder::terrain_surface::TerrainSurface::cached(map)
    } else {
        None
    };
    let screen = |q: Vec2<f32>| {
        p.screen(
            q,
            surface.as_ref().and_then(|t| t.height_at(q)).unwrap_or(0.),
        )
    };
    if guides.grid {
        let w = buffer.dim().width as f32;
        let h = buffer.dim().height as f32;
        let corners = [
            Vec2::zero(),
            Vec2::new(w, 0.),
            Vec2::new(w, h),
            Vec2::new(0., h),
        ]
        .map(|q| p.ground(q));
        let min = corners
            .iter()
            .fold(Vec2::new(f32::INFINITY, f32::INFINITY), |a, b| {
                Vec2::new(a.x.min(b.x), a.y.min(b.y))
            })
            .map(|v| v.floor());
        let max = corners
            .iter()
            .fold(Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY), |a, b| {
                Vec2::new(a.x.max(b.x), a.y.max(b.y))
            })
            .map(|v| v.ceil());
        let subdivisions = map.subdivisions.clamp(1., 8.);
        for axis in 0..2 {
            let start = (min[axis] * subdivisions) as i32;
            let end = (max[axis] * subdivisions) as i32;
            for i in start..=end.min(start + 2048) {
                let v = i as f32 / subdivisions;
                let color = if v.fract() == 0. {
                    [165, 193, 174, 50]
                } else {
                    [145, 174, 155, 20]
                };
                let (a, b) = if axis == 0 {
                    (Vec2::new(v, min.y), Vec2::new(v, max.y))
                } else {
                    (Vec2::new(min.x, v), Vec2::new(max.x, v))
                };
                let steps = if surface.is_some() {
                    ((b - a).magnitude() * 4.).ceil().min(2048.) as usize
                } else {
                    1
                };
                let mut previous = screen(a);
                for k in 1..=steps.max(1) {
                    let next = screen(a + (b - a) * (k as f32 / steps.max(1) as f32));
                    line(buffer, previous, next, color);
                    previous = next;
                }
            }
        }
    }
    let topology = matches!(
        guides.tool,
        MapToolType::Vertex | MapToolType::Linedef | MapToolType::Sector
    );
    let (vertices, lines) = if topology {
        editing_geometry(map, guides.hide_rect_geometry)
    } else {
        (FxHashSet::default(), FxHashSet::default())
    };
    let edit_height = map.properties.get_float_default("iso_edit_height", 0.);
    let selected_sector_lines: FxHashMap<_, _> = map
        .sectors
        .iter()
        .filter(|s| map.selected_sectors.contains(&s.id) || guides.geometry_hover.2 == Some(s.id))
        .flat_map(|s| {
            s.linedefs
                .iter()
                .map(move |id| (*id, s.properties.get_float_default("floor_height", 0.)))
        })
        .collect();
    let vertex_heights: FxHashMap<_, _> = map
        .linedefs
        .iter()
        .filter_map(|l| selected_sector_lines.get(&l.id).map(|h| (l, *h)))
        .flat_map(|(l, h)| [(l.start_vertex, h), (l.end_vertex, h)])
        .collect();
    let selected_lines: FxHashSet<_> = map
        .selected_linedefs
        .iter()
        .copied()
        .chain(selected_sector_lines.keys().copied())
        .chain(guides.geometry_hover.1)
        .collect();
    let selected_vertices: FxHashSet<_> = map
        .linedefs
        .iter()
        .filter(|l| selected_lines.contains(&l.id))
        .flat_map(|l| [l.start_vertex, l.end_vertex])
        .chain(map.selected_vertices.iter().copied())
        .chain(guides.geometry_hover.0)
        .collect();
    for l in &map.linedefs {
        let selected = selected_lines.contains(&l.id);
        let pending = map.possible_polygon.contains(&l.id);
        if !(selected || pending || topology && lines.contains(&l.id)) {
            continue;
        }
        if !l.intersects_vertical_slice(map, guides.editing_slice, guides.editing_slice_height) {
            continue;
        }
        if let (Some(a), Some(b)) = (map.get_vertex(l.start_vertex), map.get_vertex(l.end_vertex)) {
            let color = if selected {
                EDITOR_SELECTION_COLOR
            } else if pending {
                [255, 255, 255, 255]
            } else {
                EDITOR_GEOMETRY_COLOR
            };
            let height = selected_sector_lines
                .get(&l.id)
                .copied()
                .unwrap_or(edit_height);
            let projection = p.at_height(height - edit_height);
            for segment in projected_segment_at_height(map, projection, a, b, height).windows(2) {
                line(buffer, segment[0], segment[1], color);
            }
        }
    }
    for v in &map.vertices {
        let selected = selected_vertices.contains(&v.id);
        let terrain_control = v.properties.get_bool_default("terrain_control", false);
        if !(selected || terrain_control || topology && vertices.contains(&v.id)) {
            continue;
        }
        if !terrain_control
            && !v.intersects_vertical_slice(guides.editing_slice, guides.editing_slice_height)
        {
            continue;
        }
        let q = if let Some(height) = vertex_heights.get(&v.id) {
            let projection = p.at_height(*height - edit_height);
            projected_segment_at_height(map, projection, v.as_vec2(), v.as_vec2(), *height)[0]
        } else {
            screen(v.as_vec2())
        };
        let color =
            if map.selected_vertices.contains(&v.id) || guides.geometry_hover.0 == Some(v.id) {
                EDITOR_SELECTION_COLOR
            } else {
                EDITOR_GEOMETRY_COLOR
            };
        let radius = if selected || terrain_control { 3. } else { 2. };
        line(
            buffer,
            q - Vec2::new(radius, 0.),
            q + Vec2::new(radius, 0.),
            color,
        );
        line(
            buffer,
            q - Vec2::new(0., radius),
            q + Vec2::new(0., radius),
            color,
        );
    }
    if topology {
        if let (Some(a), Some(b)) = (map.curr_grid_pos, map.curr_mouse_pos) {
            for segment in projected_segment(map, p, a, b).windows(2) {
                line(buffer, segment[0], segment[1], [255, 255, 255, 255]);
            }
        }
        if let Some((a, b)) = map.curr_rectangle {
            // Iso geometry tools store the actual screen rectangle, matching picking.
            let min = Vec2::new(a.x.min(b.x), a.y.min(b.y));
            let max = Vec2::new(a.x.max(b.x), a.y.max(b.y));
            let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
            for i in 0..4 {
                line(
                    buffer,
                    corners[i],
                    corners[(i + 1) % 4],
                    [255, 255, 255, 255],
                );
            }
        }
    }
    if let Some(hover) = guides.hover {
        if topology {
            let q = screen(hover);
            line(
                buffer,
                q - Vec2::new(4., 0.),
                q + Vec2::new(4., 0.),
                EDITOR_CURSOR_COLOR,
            );
            line(
                buffer,
                q - Vec2::new(0., 4.),
                q + Vec2::new(0., 4.),
                EDITOR_CURSOR_COLOR,
            );
        } else {
            let a = hover.map(|v| v.floor());
            terrain_rectangle_guide(buffer, map, p, a, a + 1., [196, 211, 179, 150]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    fn guides(tool: MapToolType) -> EditorGuides {
        EditorGuides {
            grid: false,
            hover: None,
            geometry_hover: (None, None, None),
            tool,
            hide_rect_geometry: true,
            editing_slice: 0.,
            editing_slice_height: 2.,
        }
    }

    fn room(map: &mut Map, at: Vec2<f32>) -> u32 {
        let v = [at, at + Vec2::unit_x(), at + 1., at + Vec2::unit_y()]
            .map(|q| map.add_vertex_at(q.x, q.y));
        for i in 0..4 {
            map.create_linedef_manual(v[i], v[(i + 1) % 4]);
        }
        map.close_polygon_manual().unwrap()
    }

    fn pixel(buffer: &TheRGBABuffer, point: Vec2<f32>) -> &[u8] {
        let at =
            (point.y.round() as usize * buffer.dim().width as usize + point.x.round() as usize) * 4;
        &buffer.pixels()[at..at + 4]
    }

    #[test]
    fn topology_is_visible_only_in_authoring_tools_and_highlights_hover() {
        let mut map = Map::default();
        let id = room(&mut map, Vec2::zero());
        let projection = Projection {
            origin: Vec2::new(60., 60.),
            unit: 32.,
        };
        let edge = projection.screen(Vec2::new(0.5, 0.), 0.);
        let vertex = projection.screen(Vec2::zero(), 0.);
        for tool in [
            MapToolType::Vertex,
            MapToolType::Linedef,
            MapToolType::Sector,
        ] {
            let mut buffer = TheRGBABuffer::new(TheDim::sized(180, 140));
            draw_guides(&map, &mut buffer, projection, guides(tool));
            assert_ne!(pixel(&buffer, edge), &[0, 0, 0, 0]);
            assert_ne!(pixel(&buffer, vertex), &[0, 0, 0, 0]);
        }
        let mut buffer = TheRGBABuffer::new(TheDim::sized(180, 140));
        draw_guides(&map, &mut buffer, projection, guides(MapToolType::Rect));
        assert_eq!(pixel(&buffer, edge), &[0, 0, 0, 0]);
        let mut hovered = guides(MapToolType::Sector);
        hovered.geometry_hover.2 = Some(id);
        draw_guides(&map, &mut buffer, projection, hovered);
        assert_eq!(pixel(&buffer, edge), &EDITOR_SELECTION_COLOR);
    }

    #[test]
    fn raised_selected_sector_keeps_its_outline_without_moving_the_editor_plane() {
        let mut map = Map::default();
        let raised = room(&mut map, Vec2::zero());
        room(&mut map, Vec2::new(2., 0.));
        map.find_sector_mut(raised)
            .unwrap()
            .properties
            .set("floor_height", crate::Value::Float(1.));
        map.selected_sectors = vec![raised];
        let projection = Projection {
            origin: Vec2::new(30., 140.),
            unit: 32.,
        };
        let mut buffer = TheRGBABuffer::new(TheDim::sized(220, 200));
        draw_guides(&map, &mut buffer, projection, guides(MapToolType::Sector));
        assert_eq!(
            pixel(&buffer, projection.screen(Vec2::new(0.5, 0.), 1.)),
            &EDITOR_SELECTION_COLOR
        );
        assert_eq!(
            pixel(&buffer, projection.screen(Vec2::new(2.5, 0.), 0.)),
            &EDITOR_GEOMETRY_COLOR
        );
        assert_eq!(
            pixel(&buffer, projection.screen(Vec2::new(0.5, 0.), 0.)),
            &[0, 0, 0, 0]
        );
    }

    #[test]
    fn pending_line_and_box_use_world_and_screen_coordinates_respectively() {
        let mut map = Map::default();
        map.curr_grid_pos = Some(Vec2::zero());
        map.curr_mouse_pos = Some(Vec2::new(2., 0.));
        map.curr_rectangle = Some((Vec2::new(12., 15.), Vec2::new(42., 32.)));
        let projection = Projection {
            origin: Vec2::new(60., 90.),
            unit: 32.,
        };
        let mut buffer = TheRGBABuffer::new(TheDim::sized(180, 140));
        draw_guides(&map, &mut buffer, projection, guides(MapToolType::Linedef));
        assert_eq!(
            pixel(&buffer, projection.screen(Vec2::new(1., 0.), 0.)),
            &[255; 4]
        );
        assert_eq!(pixel(&buffer, Vec2::new(20., 15.)), &[255; 4]);
        assert_eq!(pixel(&buffer, Vec2::new(42., 25.)), &[255; 4]);
    }

    #[test]
    fn hidden_tile_footprints_do_not_hide_authored_regions_or_open_lines() {
        let mut map = Map::default();
        let region = room(&mut map, Vec2::zero());
        let tile = room(&mut map, Vec2::new(3., 0.));
        map.find_sector_mut(tile).unwrap().layer = Some(0);
        let start = map.add_vertex_at(6., 0.);
        let end = map.add_vertex_at(7., 0.);
        let open = map.create_linedef_manual(start, end);
        let (vertices, lines) = editing_geometry(&map, true);
        assert!(lines.contains(&open));
        assert!(vertices.contains(&start));
        for id in &map.find_sector(region).unwrap().linedefs {
            assert!(lines.contains(id));
        }
        for id in &map.find_sector(tile).unwrap().linedefs {
            assert!(!lines.contains(id));
            assert!(!vertices.contains(&map.find_linedef(*id).unwrap().start_vertex));
        }
        map.properties.set("iso_edit_height", Value::Float(2.));
        map.find_sector_mut(region)
            .unwrap()
            .properties
            .set("floor_height", Value::Float(2.));
        let (_, lines) = editing_geometry(&map, true);
        assert!(!lines.contains(&open));
        assert!(lines.contains(&map.find_sector(region).unwrap().linedefs[0]));
    }

    #[test]
    fn guides_follow_shared_terrain_and_the_active_construction_plane() {
        let mut map = Map::default();
        map.properties.set("terrain_enabled", Value::Bool(true));
        map.properties.set("terrain_edge_falloff", Value::Float(0.));
        map.add_vertex_at(-6., -6.);
        map.add_vertex_at(6., 6.);
        let id = map.add_vertex_at(0., 0.);
        let peak = map.find_vertex_mut(id).unwrap();
        peak.z = 1.;
        peak.properties.set("terrain_control", Value::Bool(true));
        peak.properties.set("smoothness", Value::Float(1.));
        let projection = Projection {
            origin: Vec2::new(100., 100.),
            unit: 32.,
        };
        let mut buffer = TheRGBABuffer::new(TheDim::sized(200, 200));
        draw_guides(&map, &mut buffer, projection, guides(MapToolType::Vertex));
        assert_ne!(
            pixel(&buffer, projection.screen(Vec2::zero(), 1.)),
            &[0, 0, 0, 0]
        );
        buffer.pixels_mut().fill(0);
        map.properties.set("iso_edit_height", Value::Float(2.));
        let projection = projection.at_height(2.);
        draw_guides(&map, &mut buffer, projection, guides(MapToolType::Vertex));
        assert_ne!(
            pixel(&buffer, projection.screen(Vec2::zero(), 0.)),
            &[0, 0, 0, 0]
        );
    }
}
