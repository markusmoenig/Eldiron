//! Recipe-authored collision/support contours, with mesh bounds as a fallback.
//! Sector IDs and anchors stay stable; custom polygons keep their authored geometry.
use super::*;
use rusterix::{Map, PixelSource, Tile, Value};

pub fn anchor(map: &Map, sector: &rusterix::Sector) -> Option<Vec2<f32>> {
    let center = sector.center(map)?;
    Some(
        sector
            .properties
            .get_vec2("iso_anchor_offset")
            .map(|o| center + Vec2::from(o))
            .or_else(|| sector.properties.get_vec2("iso_anchor").map(Vec2::from))
            .unwrap_or(center),
    )
}
/// Cached local X/Z rectangles, in the same direction order as the sprite frames.
pub(super) fn mesh_bounds(mesh: &Mesh, yaw: f32) -> Option<[[f32; 2]; 2]> {
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    for v in mesh.parts.iter().flat_map(|p| &p.vertices) {
        let p = rotate(v.position, yaw);
        for (k, axis) in [0, 2].into_iter().enumerate() {
            lo[k] = lo[k].min(p[axis]);
            hi[k] = hi[k].max(p[axis]);
        }
    }
    (lo.iter().chain(&hi).all(|v| v.is_finite()) && (0..2).all(|k| hi[k] - lo[k] > 0.0001))
        .then_some([lo, hi])
}
/// Apply cached bounds without evaluating or baking any recipes. Returns changed maps.
pub fn synchronize(project: &mut Project) -> Vec<Uuid> {
    let mut changed = vec![];
    for region in &mut project.regions {
        if update_map(&mut region.map, &project.tiles) {
            changed.push(region.map.id);
        }
    }
    changed
}
pub fn update_map(map: &mut Map, tiles: &indexmap::IndexMap<Uuid, Tile>) -> bool {
    let snapped = super::snapping::update_map(map, tiles);
    use denrim_materials::geometry::footprint::{self, MAX_POINTS, Shape};
    use std::collections::{HashMap, HashSet};
    if !map.sectors.iter().any(|s| {
        s.properties
            .get_bool_default("iso_follow_mesh_bounds", false)
    }) {
        return false;
    }
    // Index once: footprint changes and brush strokes must remain linear in map size.
    let edges: HashMap<_, _> = map.linedefs.iter().map(|e| (e.id, e)).collect();
    let vertices: HashMap<_, _> = map
        .vertices
        .iter()
        .enumerate()
        .map(|(i, v)| (v.id, i))
        .collect();
    let mut degree = HashMap::<u32, usize>::new();
    for e in &map.linedefs {
        *degree.entry(e.start_vertex).or_default() += 1;
        *degree.entry(e.end_vertex).or_default() += 1;
    }
    let updates = map
        .sectors
        .iter()
        .filter_map(|sector| {
            if !sector
                .properties
                .get_bool_default("iso_follow_mesh_bounds", false)
                || !(3..=MAX_POINTS).contains(&sector.linedefs.len())
            {
                return None;
            }
            let source = match sector.properties.get_default_source()? {
                PixelSource::TileId(id) => *id,
                _ => return None,
            };
            let layout = tiles.get(&source)?.isometric.as_ref()?;
            if layout.ground {
                return None;
            }
            let yaw = sector
                .properties
                .get_float_default("iso_yaw", 0.)
                .to_radians();
            if !yaw.is_finite() {
                return None;
            }
            let direction = layout.frame(Vec2::new(yaw.sin(), yaw.cos()), false, 0.)
                / layout.frames_per_direction.max(1);
            let local = if let Some(shape) = layout.placement_shapes.get(direction) {
                shape.clone()
            } else if let Some(contour) = layout.placement_contours.get(direction) {
                Shape::single(contour.clone())
            } else {
                let [lo, hi] = *layout.placement_bounds.get(direction)?;
                Shape::single(vec![
                    [lo[0], lo[1]],
                    [hi[0], lo[1]],
                    [hi[0], hi[1]],
                    [lo[0], hi[1]],
                ])
            };
            footprint::validate(&local).ok()?;
            let old_topology = sector.footprint_rings()?;
            if old_topology.points().copied().collect::<Vec<_>>() != sector.linedefs {
                return None;
            }
            let ring = sector
                .linedefs
                .iter()
                .map(|id| edges.get(id).copied())
                .collect::<Option<Vec<_>>>()?;
            // Never rewire shared map topology or move a neighboring placement.
            if ring.iter().any(|e| {
                e.sector_ids.iter().any(|id| *id != sector.id)
                    || degree.get(&e.start_vertex) != Some(&2)
            }) {
                return None;
            }
            let ids: Vec<_> = ring.iter().map(|e| e.start_vertex).collect();
            if ids.iter().copied().collect::<HashSet<_>>().len() != ids.len() {
                return None;
            }
            for region in &old_topology.regions {
                for loop_ids in std::iter::once(&region.outer).chain(region.holes.iter()) {
                    if !(3..=64).contains(&loop_ids.len())
                        || loop_ids.iter().enumerate().any(|(i, id)| {
                            edges[id].end_vertex
                                != edges[&loop_ids[(i + 1) % loop_ids.len()]].start_vertex
                        })
                    {
                        return None;
                    }
                }
            }
            let current = ids
                .iter()
                .map(|id| vertices.get(id).map(|i| map.vertices[*i].as_vec2()))
                .collect::<Option<Vec<_>>>()?;
            let center =
                current.iter().copied().fold(Vec2::zero(), |a, b| a + b) / current.len() as f32;
            let at = sector
                .properties
                .get_vec2("iso_anchor_offset")
                .map(|o| center + Vec2::from(o))
                .or_else(|| sector.properties.get_vec2("iso_anchor").map(Vec2::from))
                .unwrap_or(center);
            let points = local
                .points()
                .map(|p| at + Vec2::from(*p))
                .collect::<Vec<_>>();
            let mut index = 0usize;
            let topology = local.map(|_| {
                let i = index;
                index += 1;
                i
            });
            if sector
                .properties
                .get_bool_default("iso_precise_footprint", false)
                && old_topology.layout() == topology.layout()
                && current.len() == points.len()
                && current
                    .iter()
                    .zip(&points)
                    .all(|(v, p)| (*v - *p).magnitude_squared() < 1e-10)
            {
                return None;
            }
            Some((
                sector.id,
                sector.linedefs.clone(),
                ids,
                points,
                at,
                topology,
            ))
        })
        .collect::<Vec<_>>();
    if updates.is_empty() {
        return snapped;
    }
    let extra = updates
        .iter()
        .map(|(_, _, ids, p, _, _)| p.len().saturating_sub(ids.len()))
        .sum::<usize>();
    let mut next_vertex = map
        .vertices
        .iter()
        .map(|v| v.id as u64)
        .max()
        .map_or(0, |v| v + 1);
    let mut next_edge = map
        .linedefs
        .iter()
        .map(|v| v.id as u64)
        .max()
        .map_or(0, |v| v + 1);
    if next_vertex + extra as u64 > u32::MAX as u64 || next_edge + extra as u64 > u32::MAX as u64 {
        return false;
    }
    drop(edges);
    let mut vertex_indices = vertices;
    let mut edge_indices: HashMap<_, _> = map
        .linedefs
        .iter()
        .enumerate()
        .map(|(i, e)| (e.id, i))
        .collect();
    let sectors: HashMap<_, _> = map
        .sectors
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id, i))
        .collect();
    let mut remove_vertices = HashSet::new();
    let mut remove_edges = HashSet::new();
    for (id, mut edge_ids, mut ids, points, at, topology) in updates {
        let vertex_template = map.vertices[vertex_indices[&ids[0]]].clone();
        let edge_template = map.linedefs[edge_indices[&edge_ids[0]]].clone();
        while ids.len() < points.len() {
            let mut vertex = vertex_template.clone();
            vertex.id = next_vertex as u32;
            next_vertex += 1;
            vertex_indices.insert(vertex.id, map.vertices.len());
            ids.push(vertex.id);
            map.vertices.push(vertex);
            let mut edge = edge_template.clone();
            edge.id = next_edge as u32;
            next_edge += 1;
            edge.creator_id = Uuid::new_v4();
            edge.sector_ids = vec![id];
            edge_indices.insert(edge.id, map.linedefs.len());
            edge_ids.push(edge.id);
            map.linedefs.push(edge);
        }
        remove_vertices.extend(ids.drain(points.len()..));
        remove_edges.extend(edge_ids.drain(points.len()..));
        for (i, p) in points.iter().enumerate() {
            let v = &mut map.vertices[vertex_indices[&ids[i]]];
            v.x = p.x;
            v.y = p.y;
        }
        for region in &topology.regions {
            for ring in std::iter::once(&region.outer).chain(region.holes.iter()) {
                for (j, i) in ring.iter().enumerate() {
                    let edge = &mut map.linedefs[edge_indices[&edge_ids[*i]]];
                    edge.start_vertex = ids[*i];
                    edge.end_vertex = ids[ring[(j + 1) % ring.len()]];
                }
            }
        }
        let center = points.iter().copied().fold(Vec2::zero(), |a, b| a + b) / points.len() as f32;
        let sector = &mut map.sectors[sectors[&id]];
        sector.footprint_topology = Some(topology.map(|i| *i as u32));
        sector.linedefs = edge_ids;
        sector
            .properties
            .set("iso_precise_footprint", Value::Bool(true));
        sector.properties.set(
            "iso_anchor_offset",
            Value::Vec2([at.x - center.x, at.y - center.y]),
        );
        sector.properties.remove("iso_anchor");
    }
    map.vertices.retain(|v| !remove_vertices.contains(&v.id));
    map.linedefs.retain(|e| !remove_edges.contains(&e.id));
    map.selected_vertices
        .retain(|id| !remove_vertices.contains(id));
    map.selected_linedefs
        .retain(|id| !remove_edges.contains(id));
    map.changed = map.changed.wrapping_add(1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn place(map: &mut Map, tile: Uuid, at: Vec2<f32>, follow: bool, yaw: f32) -> u32 {
        map.possible_polygon.clear();
        let vertices = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]].map(|p| {
            let id = map.find_free_vertex_id().unwrap();
            map.vertices
                .push(rusterix::Vertex::new(id, at.x + p[0], at.y + p[1]));
            id
        });
        for i in 0..4 {
            map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        let s = map.find_sector_mut(id).unwrap();
        s.layer = Some(1);
        s.properties.set("rect", Value::Bool(true));
        s.properties
            .set("source", Value::Source(PixelSource::TileId(tile)));
        s.properties
            .set("iso_follow_mesh_bounds", Value::Bool(follow));
        s.properties.set("iso_yaw", Value::Float(yaw));
        id
    }
    fn fixture() -> (Project, Uuid, u32, u32) {
        let mut p = Project::default();
        let id = Uuid::new_v4();
        let source =
            include_str!("../../../../creator/embedded/meshes/recipes/fence.mzgeometry.json");
        p.procedural_appearances.insert(id, source.into());
        p.isometric_bakes.insert(
            id,
            Bake {
                recipe: id,
                ground: false,
                yaw: 0.,
                directions: 4,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
        let map = &mut p.regions[0].map;
        map.camera = rusterix::MapCamera::TwoDIso;
        let follow = place(map, id, Vec2::new(10., 20.), true, 0.);
        let manual = place(map, id, Vec2::new(10., 30.), false, 0.);
        assert!(super::super::synchronize(&mut p).is_empty());
        p.tiles.get_mut(&id).unwrap().blocking = true;
        (p, id, follow, manual)
    }
    fn edit(p: &mut Project, id: Uuid, count: f32, spacing: f32) {
        let mut d = Document::load(&p.procedural_appearances[&id]).unwrap();
        let denrim_materials::geometry::Operation::Group { controls, .. } =
            &mut d.rows[0].tiles[0].op
        else {
            panic!()
        };
        for c in controls {
            if c.id == "count" {
                c.value = count;
            }
            if c.id == "spacing" {
                c.value = spacing;
            }
        }
        p.procedural_appearances.insert(id, d.save().unwrap());
        assert!(super::super::synchronize(p).is_empty());
    }
    fn bounds(p: &Project, id: u32) -> ([f32; 2], [f32; 2]) {
        let map = &p.regions[0].map;
        let b = map.find_sector(id).unwrap().bounding_box(map);
        ([b.min.x, b.min.y], [b.max.x, b.max.y])
    }
    fn near(a: [f32; 2], b: [f32; 2]) {
        assert!(
            (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4,
            "{a:?} != {b:?}"
        );
    }
    #[test]
    fn recipe_dimensions_update_collision_without_moving_anchors_or_manual_footprints() {
        let (mut p, id, follow, manual) = fixture();
        near(bounds(&p, follow).0, [10., 19.94]);
        near(bounds(&p, follow).1, [12.1, 20.06]);
        let source = p.procedural_appearances[&id].clone();
        let geometry = p.regions[0]
            .map
            .find_sector(follow)
            .unwrap()
            .linedefs
            .clone();
        edit(&mut p, id, 8., 0.45);
        near(bounds(&p, follow).1, [13.6, 20.06]);
        near(bounds(&p, manual).0, [9.5, 29.5]);
        near(bounds(&p, manual).1, [10.5, 30.5]);
        let map = &p.regions[0].map;
        let a = anchor(map, map.find_sector(follow).unwrap()).unwrap();
        near([a.x, a.y], [10., 20.]);
        assert_eq!(map.find_sector(follow).unwrap().linedefs, geometry);
        let mini = map.as_mini(&[id].into_iter().collect());
        let (_, blocked) = mini.move_distance(Vec2::new(13., 19.), Vec2::new(0., 2.), 0.1);
        assert!(blocked);
        let (_, blocked) = mini.move_distance(Vec2::new(14., 19.), Vec2::new(0., 2.), 0.1);
        assert!(!blocked);
        // Restoring the source (undo) restores the derived collision.
        p.procedural_appearances.insert(id, source);
        assert!(super::super::synchronize(&mut p).is_empty());
        near(bounds(&p, follow).1, [12.1, 20.06]);
        let changed = p.regions[0].map.changed;
        assert!(synchronize(&mut p).is_empty());
        assert_eq!(p.regions[0].map.changed, changed);
    }
    #[test]
    fn rotation_translation_save_and_reload_keep_bounds_aligned_to_the_sprite() {
        let (mut p, id, follow, _) = fixture();
        p.regions[0]
            .map
            .find_sector_mut(follow)
            .unwrap()
            .properties
            .set("iso_yaw", Value::Float(90.));
        assert!(!synchronize(&mut p).is_empty());
        near(bounds(&p, follow).0, [9.94, 17.9]);
        near(bounds(&p, follow).1, [10.06, 20.]);
        let vertices = p.regions[0]
            .map
            .find_sector(follow)
            .unwrap()
            .linedefs
            .iter()
            .map(|id| p.regions[0].map.find_linedef(*id).unwrap().start_vertex)
            .collect::<Vec<_>>();
        for id in vertices {
            let v = p.regions[0].map.find_vertex_mut(id).unwrap();
            v.x += 3.;
            v.y += 4.;
        }
        edit(&mut p, id, 8., 0.45);
        near(bounds(&p, follow).0, [12.94, 20.4]);
        near(bounds(&p, follow).1, [13.06, 24.]);
        let bytes = crate::project_io::encode_project(&p).unwrap();
        let reopened = crate::project_io::decode_project(&bytes).unwrap();
        assert_eq!(bounds(&p, follow), bounds(&reopened, follow));
        assert_eq!(p.tiles[&id].isometric, reopened.tiles[&id].isometric);
        let source = p.procedural_appearances[&id].clone();
        p.procedural_appearances.insert(id, "invalid recipe".into());
        assert!(!super::super::synchronize(&mut p).is_empty());
        assert_eq!(bounds(&p, follow), bounds(&reopened, follow));
        p.procedural_appearances.insert(id, source);
    }
    #[test]
    fn cached_rotation_bounds_are_optional_and_old_layouts_still_load() {
        let layout: rusterix::isometric::SpriteLayout=serde_json::from_str(r#"{"anchor":[0,0],"pixels_per_unit":32,"ground":false,"directions":1,"frames_per_direction":1,"walk_frames":0,"frames_per_second":0}"#).unwrap();
        assert!(layout.placement_bounds.is_empty());
    }
    #[test]
    fn shared_map_vertices_and_manual_freeze_are_preserved() {
        let (mut p, id, follow, _) = fixture();
        p.regions[0]
            .map
            .find_sector_mut(follow)
            .unwrap()
            .properties
            .set("iso_follow_mesh_bounds", Value::Bool(false));
        edit(&mut p, id, 8., 0.45);
        near(bounds(&p, follow).1, [12.1, 20.06]);
        p.regions[0]
            .map
            .find_sector_mut(follow)
            .unwrap()
            .properties
            .set("iso_follow_mesh_bounds", Value::Bool(true));
        let map = &mut p.regions[0].map;
        let edge = map.find_sector(follow).unwrap().linedefs[0];
        let v = map.find_linedef(edge).unwrap().start_vertex;
        let other = map.find_free_vertex_id().unwrap();
        map.vertices.push(rusterix::Vertex::new(other, 9., 19.));
        let edge = map.find_free_linedef_id().unwrap();
        map.linedefs.push(rusterix::Linedef::new(edge, v, other));
        let original = map.vertices.clone();
        assert!(synchronize(&mut p).is_empty());
        assert_eq!(p.regions[0].map.vertices, original);
    }
    #[test]
    fn concave_corners_and_trunks_replace_rectangles_without_leaks_or_anchor_drift() {
        let (mut p, id, follow, _) = fixture();
        let old_edges = p.regions[0]
            .map
            .find_sector(follow)
            .unwrap()
            .linedefs
            .clone();
        let creator = p.regions[0].map.find_sector(follow).unwrap().creator_id;
        let corner =
            include_str!("../../../../creator/embedded/meshes/recipes/wall-corner.mzgeometry.json");
        p.procedural_appearances.insert(id, corner.into());
        assert!(super::super::synchronize(&mut p).is_empty());
        let map = &p.regions[0].map;
        let sector = map.find_sector(follow).unwrap();
        assert_eq!(sector.linedefs.len(), 6);
        assert_eq!(&sector.linedefs[..4], old_edges.as_slice());
        assert_eq!(sector.creator_id, creator);
        assert_eq!(map.vertices.len(), 10);
        assert_eq!(map.linedefs.len(), 10);
        near(
            [
                anchor(map, sector).unwrap().x,
                anchor(map, sector).unwrap().y,
            ],
            [10., 20.],
        );
        assert!(!sector.is_inside(map, Vec2::new(10.7, 20.7)));
        assert!(sector.is_inside(map, Vec2::new(10.7, 20.1)));
        let mini = map.as_mini(&[id].into_iter().collect());
        assert!(
            !mini
                .move_distance(Vec2::new(10.7, 20.7), Vec2::new(0., -0.2), 0.1)
                .1,
            "Corner notch must stay walkable"
        );
        assert!(
            mini.move_distance(Vec2::new(10.7, 20.7), Vec2::new(0., -0.6), 0.1)
                .1,
            "Actual wall arm must block"
        );
        assert!(
            !mini
                .move_distance(Vec2::new(11.3, 20.7), Vec2::new(-0.6, 0.), 0.1)
                .1,
            "Entering the open notch from outside must not hit a whole-cell block"
        );
        assert!(mini.is_walkable_position(Vec2::new(10.7, 20.7), 0.1));
        assert!(!mini.is_walkable_position(Vec2::new(10.7, 20.1), 0.01));
        p.regions[0]
            .map
            .find_sector_mut(follow)
            .unwrap()
            .properties
            .set("iso_follow_mesh_bounds", Value::Bool(false));
        assert!(
            p.regions[0]
                .map
                .as_mini(&[id].into_iter().collect())
                .is_walkable_position(Vec2::new(10.7, 20.7), 0.1),
            "Frozen contours must keep precise collision"
        );
        p.regions[0]
            .map
            .find_sector_mut(follow)
            .unwrap()
            .properties
            .set("iso_follow_mesh_bounds", Value::Bool(true));
        let tree = include_str!("../../../../creator/embedded/meshes/recipes/tree.mzgeometry.json");
        p.procedural_appearances.insert(id, tree.into());
        assert!(super::super::synchronize(&mut p).is_empty());
        let map = &p.regions[0].map;
        let sector = map.find_sector(follow).unwrap();
        assert_eq!(sector.linedefs.len(), 8);
        assert_eq!(map.vertices.len(), 12);
        assert_eq!(map.linedefs.len(), 12);
        let at = anchor(map, sector).unwrap();
        near([at.x, at.y], [10., 20.]);
        let mini = map.as_mini(&[id].into_iter().collect());
        assert!(
            !mini
                .move_distance(Vec2::new(10.55, 19.5), Vec2::new(0., 1.), 0.05)
                .1,
            "Canopy must not block"
        );
        let center = sector.bounding_box(map).center();
        assert!(
            mini.move_distance(center - Vec2::new(0., 0.5), Vec2::new(0., 1.), 0.05)
                .1,
            "Trunk must block"
        );
        let bytes = crate::project_io::encode_project(&p).unwrap();
        let restored = crate::project_io::decode_project(&bytes).unwrap();
        assert_eq!(
            restored.regions[0]
                .map
                .find_sector(follow)
                .unwrap()
                .linedefs,
            sector.linedefs
        );
        assert_eq!(restored.tiles[&id].isometric, p.tiles[&id].isometric);
        // Going back to a legacy bounds-only recipe cleans up surplus edges/vertices.
        p.procedural_appearances.insert(
            id,
            include_str!("../../../../creator/embedded/meshes/recipes/fence.mzgeometry.json")
                .into(),
        );
        assert!(super::super::synchronize(&mut p).is_empty());
        let map = &p.regions[0].map;
        assert_eq!(map.find_sector(follow).unwrap().linedefs, old_edges);
        assert_eq!(map.vertices.len(), 8);
        assert_eq!(map.linedefs.len(), 8);
        near(
            [
                anchor(map, map.find_sector(follow).unwrap()).unwrap().x,
                anchor(map, map.find_sector(follow).unwrap()).unwrap().y,
            ],
            [10., 20.],
        );
    }

    #[test]
    fn rotated_recipe_polygon_keeps_concavity_and_invalid_edits_keep_good_collision() {
        let (mut p, id, follow, _) = fixture();
        p.procedural_appearances.insert(
            id,
            include_str!("../../../../creator/embedded/meshes/recipes/wall-corner.mzgeometry.json")
                .into(),
        );
        p.regions[0]
            .map
            .find_sector_mut(follow)
            .unwrap()
            .properties
            .set("iso_yaw", Value::Float(90.));
        assert!(super::super::synchronize(&mut p).is_empty());
        let sector = p.regions[0].map.find_sector(follow).unwrap();
        assert!(!sector.is_inside(&p.regions[0].map, Vec2::new(10.7, 19.3)));
        assert!(sector.is_inside(&p.regions[0].map, Vec2::new(10.1, 19.3)));
        let vertices = p.regions[0].map.vertices.clone();
        p.procedural_appearances
            .insert(id, "invalid footprint recipe".into());
        assert!(!super::super::synchronize(&mut p).is_empty());
        assert_eq!(p.regions[0].map.vertices, vertices);
    }
    #[test]
    fn compound_rings_keep_gaps_holes_and_walkable_support_through_rebakes() {
        let (mut p, id, follow, _) = fixture();
        let original_edges = p.regions[0]
            .map
            .find_sector(follow)
            .unwrap()
            .linedefs
            .clone();
        p.procedural_appearances.insert(
            id,
            include_str!("../../../../creator/embedded/meshes/recipes/doorway.mzgeometry.json")
                .into(),
        );
        assert!(super::super::synchronize(&mut p).is_empty());
        let map = &p.regions[0].map;
        let sector = map.find_sector(follow).unwrap();
        assert_eq!(sector.footprint_topology.as_ref().unwrap().regions.len(), 2);
        assert_eq!(sector.linedefs.len(), 8);
        assert!(!sector.is_inside(map, Vec2::new(10., 20.)));
        let mini = map.as_mini(&[id].into_iter().collect());
        assert!(
            !mini
                .move_distance(Vec2::new(10., 19.), Vec2::new(0., 2.), 0.22)
                .1,
            "Doorway must stay open under its lintel"
        );
        assert!(!mini.is_walkable_position(Vec2::new(10.6, 20.), 0.01));
        p.procedural_appearances.insert(
            id,
            include_str!(
                "../../../../creator/embedded/meshes/recipes/ring-walkway.mzgeometry.json"
            )
            .into(),
        );
        assert!(super::super::synchronize(&mut p).is_empty());
        let map = &mut p.regions[0].map;
        let sector = map.find_sector_mut(follow).unwrap();
        assert_eq!(
            sector.footprint_topology.as_ref().unwrap().regions[0]
                .holes
                .len(),
            1
        );
        sector.properties.set("iso_walkable", Value::Bool(true));
        sector.properties.set("floor_height", Value::Float(0.25));
        p.tiles.get_mut(&id).unwrap().blocking = false;
        let map = &p.regions[0].map;
        let sector = map.find_sector(follow).unwrap();
        let (vertices, triangles) = sector.generate_geometry(map).unwrap();
        let area = triangles
            .iter()
            .map(|&(a, b, c)| {
                let a = vertices[a];
                let b = vertices[b];
                let c = vertices[c];
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
            })
            .sum::<f32>();
        assert!(
            (area - 6.75).abs() < 1e-4,
            "Hole must be excluded from triangulation: {area}"
        );
        let mini = map.as_mini(&Default::default());
        assert!((mini.elevation.height_at(Vec2::new(11.2, 20.)) - 0.25).abs() < 1e-4);
        assert!(
            mini.elevation.height_at(Vec2::new(10., 20.)).abs() < 1e-4,
            "Hole must not become a walkable cap"
        );
        let frozen_topology = sector.footprint_topology.clone();
        let map = &mut p.regions[0].map;
        map.selected_sectors = vec![follow];
        let clipboard = map.copy_selected(false);
        assert!(!clipboard.sectors[0].is_inside(&clipboard, Vec2::new(1.5, 1.5)));
        assert!(clipboard.sectors[0].is_inside(&clipboard, Vec2::new(0.3, 1.5)));
        let mut pasted = Map::default();
        pasted.paste_at_position(&clipboard, Vec2::new(30., 40.));
        assert!(!pasted.sectors[0].is_inside(&pasted, Vec2::new(31.5, 41.5)));
        assert!(pasted.sectors[0].is_inside(&pasted, Vec2::new(30.3, 41.5)));

        let bytes = crate::project_io::encode_project(&p).unwrap();
        let restored = crate::project_io::decode_project(&bytes).unwrap();
        assert_eq!(
            restored.regions[0]
                .map
                .find_sector(follow)
                .unwrap()
                .footprint_topology,
            frozen_topology
        );
        p.procedural_appearances.insert(
            id,
            include_str!(
                "../../../../creator/embedded/meshes/recipes/repeated-posts.mzgeometry.json"
            )
            .into(),
        );
        assert!(super::super::synchronize(&mut p).is_empty());
        let map = &p.regions[0].map;
        let sector = map.find_sector(follow).unwrap();
        assert_eq!(sector.footprint_topology.as_ref().unwrap().regions.len(), 5);
        assert_eq!(map.vertices.len(), 24);
        assert!(
            !sector.is_inside(map, Vec2::new(10.175, 20.)),
            "Space between posts must remain empty"
        );
        p.procedural_appearances.insert(
            id,
            include_str!("../../../../creator/embedded/meshes/recipes/fence.mzgeometry.json")
                .into(),
        );
        assert!(super::super::synchronize(&mut p).is_empty());
        let map = &p.regions[0].map;
        assert_eq!(map.vertices.len(), 8);
        assert_eq!(map.linedefs.len(), 8);
        assert_eq!(map.find_sector(follow).unwrap().linedefs, original_edges);
        let a = anchor(map, map.find_sector(follow).unwrap()).unwrap();
        near([a.x, a.y], [10., 20.]);
    }
}
