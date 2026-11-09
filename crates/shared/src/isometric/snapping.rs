//! Recipe connector placement using the same directional cache as the sprite art.
//! Links form a parent forest; cycles and stale connectors freeze affected parts.
use super::*;
use denrim_materials::geometry::connectors::Connector;
use rusterix::{Map, Sector, Tile, Value};
use std::collections::{HashMap, HashSet};
type Tiles = indexmap::IndexMap<Uuid, Tile>;
const TARGET: &str = "iso_snap_target";
#[derive(Clone, Copy)]
struct Pose {
    at: Vec2<f32>,
    height: f32,
    yaw: f32,
}
struct Entry {
    pose: Pose,
    vertices: Vec<u32>,
    isolated: bool,
}
struct Index<'a> {
    sectors: HashMap<u32, &'a Sector>,
    entries: HashMap<u32, Entry>,
}
impl<'a> Index<'a> {
    fn new(map: &'a Map, tiles: &Tiles) -> Self {
        let edges: HashMap<_, _> = map.linedefs.iter().map(|e| (e.id, e)).collect();
        let vertices: HashMap<_, _> = map.vertices.iter().map(|v| (v.id, v.as_vec2())).collect();
        let mut degree = HashMap::<u32, usize>::new();
        for e in &map.linedefs {
            *degree.entry(e.start_vertex).or_default() += 1;
            *degree.entry(e.end_vertex).or_default() += 1;
        }
        let terrain = rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(map);
        let entries = map
            .sectors
            .iter()
            .filter_map(|s| {
                if !layout(s, tiles)
                    .is_some_and(|l| l.placement_connectors.iter().any(|c| !c.is_empty()))
                {
                    return None;
                }
                let ring = s
                    .linedefs
                    .iter()
                    .map(|id| edges.get(id).copied())
                    .collect::<Option<Vec<_>>>()?;
                if ring.is_empty() {
                    return None;
                }
                let mut sum = Vec2::zero();
                for e in &ring {
                    sum += *vertices.get(&e.start_vertex)?;
                    sum += *vertices.get(&e.end_vertex)?;
                }
                let center = sum / (ring.len() * 2) as f32;
                let at = s
                    .properties
                    .get_vec2("iso_anchor_offset")
                    .map(|o| center + Vec2::from(o))
                    .or_else(|| s.properties.get_vec2("iso_anchor").map(Vec2::from))
                    .unwrap_or(center);
                let mut height = s.properties.get_float_default("floor_height", 0.);
                if height.abs() < 0.001
                    && !s.properties.get_bool_default("iso_walkable", false)
                    && s.properties.get_int(TARGET).is_none()
                {
                    height = terrain
                        .as_ref()
                        .and_then(|t| t.height_at(at))
                        .unwrap_or(height);
                }
                let yaw = s.properties.get_float_default("iso_yaw", 0.);
                if !at.x.is_finite() || !at.y.is_finite() || !height.is_finite() || !yaw.is_finite()
                {
                    return None;
                }
                let ids = ring
                    .iter()
                    .flat_map(|e| [e.start_vertex, e.end_vertex])
                    .collect::<HashSet<_>>();
                let isolated = ring
                    .iter()
                    .all(|e| e.sector_ids.iter().all(|id| *id == s.id))
                    && ids.iter().all(|id| degree.get(id) == Some(&2));
                Some((
                    s.id,
                    Entry {
                        pose: Pose { at, height, yaw },
                        vertices: ids.into_iter().collect(),
                        isolated,
                    },
                ))
            })
            .collect();
        Self {
            sectors: map.sectors.iter().map(|s| (s.id, s)).collect(),
            entries,
        }
    }
}
fn layout<'a>(sector: &Sector, tiles: &'a Tiles) -> Option<&'a rusterix::isometric::SpriteLayout> {
    let rusterix::PixelSource::TileId(id) = sector.properties.get_default_source()? else {
        return None;
    };
    let layout = tiles.get(id)?.isometric.as_ref()?;
    (!layout.ground).then_some(layout)
}
fn connectors(layout: &rusterix::isometric::SpriteLayout, yaw: f32) -> &[Connector] {
    let a = yaw.to_radians();
    let i =
        layout.frame(Vec2::new(a.sin(), a.cos()), false, 0.) / layout.frames_per_direction.max(1);
    layout
        .placement_connectors
        .get(i)
        .map_or(&[], Vec::as_slice)
}
fn opposite(a: &Connector, b: &Connector) -> bool {
    a.family == b.family
        && a.direction
            .iter()
            .zip(b.direction)
            .map(|(a, b)| a * b)
            .sum::<f32>()
            < -0.999
}
fn joined(parent: Pose, local: &Connector, target: &Connector, yaw: f32) -> Option<Pose> {
    let p = Pose {
        at: parent.at
            + Vec2::new(
                target.position[0] - local.position[0],
                target.position[2] - local.position[2],
            ),
        height: parent.height + target.position[1] - local.position[1],
        yaw,
    };
    (p.at.x.is_finite() && p.at.y.is_finite() && p.height.is_finite() && yaw.is_finite())
        .then_some(p)
}
/// Resolve iteratively: arbitrary sector order and deep forests need no recursion.
fn resolved(index: &Index<'_>, tiles: &Tiles) -> HashMap<u32, Option<Pose>> {
    let mut cache = HashMap::new();
    for &id in index.entries.keys() {
        let mut chain = vec![];
        let mut seen = HashSet::new();
        let mut at = id;
        while !cache.contains_key(&at) {
            if !seen.insert(at) {
                cache.insert(at, None);
                break;
            }
            let Some(entry) = index.entries.get(&at) else {
                cache.insert(at, None);
                break;
            };
            let sector = index.sectors[&at];
            if let Some(parent) = sector.properties.get_int(TARGET) {
                chain.push(at);
                at = parent as u32;
            } else {
                cache.insert(at, Some(entry.pose));
                break;
            }
        }
        for id in chain.into_iter().rev() {
            let result = (|| {
                let entry = index.entries.get(&id)?;
                let sector = index.sectors[&id];
                if !entry.isolated
                    || !sector
                        .properties
                        .get_bool_default("iso_follow_mesh_bounds", false)
                {
                    return None;
                }
                let parent = index
                    .sectors
                    .get(&(sector.properties.get_int(TARGET)? as u32))?;
                let parent_pose = cache.get(&parent.id).copied().flatten()?;
                let yaw = parent_pose.yaw
                    + sector
                        .properties
                        .get_float_default("iso_snap_yaw_delta", 0.);
                let local = connectors(layout(sector, tiles)?, yaw)
                    .iter()
                    .find(|c| Some(c.id.as_str()) == sector.properties.get_str("iso_snap_local"))?;
                let remote = connectors(layout(parent, tiles)?, parent_pose.yaw)
                    .iter()
                    .find(|c| {
                        Some(c.id.as_str()) == sector.properties.get_str("iso_snap_remote")
                    })?;
                if !opposite(local, remote) {
                    return None;
                }
                joined(parent_pose, local, remote, yaw)
            })();
            cache.insert(id, result);
        }
    }
    cache
}
/// Detaching freezes the placement without changing its footprint mode.
pub fn detach(sector: &mut Sector) {
    for key in [
        TARGET,
        "iso_snap_local",
        "iso_snap_remote",
        "iso_snap_yaw_delta",
    ] {
        sector.properties.remove(key);
    }
}
struct Move {
    id: u32,
    from: Vec2<f32>,
    to: Pose,
    vertices: Vec<u32>,
}
fn movement(index: &Index<'_>, id: u32, to: Pose) -> Option<Move> {
    let entry = index.entries.get(&id)?;
    entry.isolated.then(|| Move {
        id,
        from: entry.pose.at,
        to,
        vertices: entry.vertices.clone(),
    })
}
fn apply_moves(map: &mut Map, moves: Vec<Move>) -> bool {
    if moves.is_empty() {
        return false;
    }
    let mut deltas = HashMap::new();
    let mut poses = HashMap::new();
    for m in moves {
        for id in m.vertices {
            deltas.insert(id, m.to.at - m.from);
        }
        poses.insert(m.id, m.to);
    }
    for v in &mut map.vertices {
        if let Some(d) = deltas.get(&v.id) {
            v.x += d.x;
            v.y += d.y;
        }
    }
    for sector in &mut map.sectors {
        if let Some(dest) = poses.get(&sector.id) {
            // Offset anchors move with their vertices; legacy absolute anchors move explicitly.
            if sector.properties.get_vec2("iso_anchor_offset").is_none() {
                sector
                    .properties
                    .set("iso_anchor", Value::Vec2([dest.at.x, dest.at.y]));
            }
            sector
                .properties
                .set("floor_height", Value::Float(dest.height));
            sector
                .properties
                .set("iso_yaw", Value::Float(dest.yaw.rem_euclid(360.)));
        }
    }
    true
}
/// Find the closest compatible unoccupied pair within 0.8 world units. Rotation
/// is restricted to baked views, keeping collision, connectors and art aligned.
pub fn snap(map: &mut Map, tiles: &Tiles, id: u32) -> bool {
    let index = Index::new(map, tiles);
    let Some(&sector) = index.sectors.get(&id) else {
        return false;
    };
    if !sector
        .properties
        .get_bool_default("iso_follow_mesh_bounds", false)
    {
        return false;
    }
    let Some(local_layout) = layout(sector, tiles) else {
        return false;
    };
    let Some(original) = index.entries.get(&id).map(|e| e.pose) else {
        return false;
    };
    let poses = resolved(&index, tiles);
    let occupied: HashSet<_> =
        map.sectors
            .iter()
            .filter(|s| s.id != id)
            .filter_map(|s| {
                Some((
                    s.properties.get_int(TARGET)? as u32,
                    s.properties.get_str("iso_snap_remote")?.to_owned(),
                ))
            })
            .chain(
                map.sectors.iter().filter(|s| s.id != id).filter_map(|s| {
                    Some((s.id, s.properties.get_str("iso_snap_local")?.to_owned()))
                }),
            )
            .collect();
    // Exclude all descendants without walking each candidate's ancestor chain.
    let mut children = HashMap::<u32, Vec<u32>>::new();
    for s in &map.sectors {
        if let Some(parent) = s.properties.get_int(TARGET) {
            children.entry(parent as u32).or_default().push(s.id);
        }
    }
    let mut excluded = HashSet::new();
    let mut pending = vec![id];
    while let Some(n) = pending.pop() {
        if excluded.insert(n) {
            if let Some(ids) = children.get(&n) {
                pending.extend(ids);
            }
        }
    }
    let mut best: Option<(f32, u32, String, String, Pose, f32)> = None;
    // Stable map order gives deterministic ties, independent of the hash indexes.
    for parent in &map.sectors {
        if excluded.contains(&parent.id)
            || !parent
                .properties
                .get_bool_default("iso_follow_mesh_bounds", false)
        {
            continue;
        }
        let Some(remote_layout) = layout(parent, tiles) else {
            continue;
        };
        let Some(parent_pose) = poses.get(&parent.id).copied().flatten() else {
            continue;
        };
        for remote in connectors(remote_layout, parent_pose.yaw) {
            if occupied.contains(&(parent.id, remote.id.clone())) {
                continue;
            }
            for i in 0..local_layout.directions.clamp(1, 8) {
                let yaw = i as f32 * 360. / local_layout.directions.clamp(1, 8) as f32;
                for local in connectors(local_layout, yaw) {
                    if occupied.contains(&(id, local.id.clone())) || !opposite(local, remote) {
                        continue;
                    }
                    let Some(result) = joined(parent_pose, local, remote, yaw) else {
                        continue;
                    };
                    let d = (result.at - original.at).magnitude_squared()
                        + (result.height - original.height).powi(2);
                    let turn = ((yaw - original.yaw + 180.).rem_euclid(360.) - 180.).abs();
                    let score = d + turn * 0.00001;
                    if d <= 0.64 && best.as_ref().is_none_or(|b| score < b.0) {
                        best = Some((
                            score,
                            parent.id,
                            local.id.clone(),
                            remote.id.clone(),
                            result,
                            parent_pose.yaw,
                        ));
                    }
                }
            }
        }
    }
    let Some((_, target, local, remote, result, parent_yaw)) = best else {
        return false;
    };
    let Some(m) = movement(&index, id, result) else {
        return false;
    };
    drop(index);
    apply_moves(map, vec![m]);
    let sector = map.find_sector_mut(id).unwrap();
    sector.properties.set(TARGET, Value::Int(target as i32));
    sector.properties.set("iso_snap_local", Value::Str(local));
    sector.properties.set("iso_snap_remote", Value::Str(remote));
    sector
        .properties
        .set("iso_snap_yaw_delta", Value::Float(result.yaw - parent_yaw));
    map.changed = map.changed.wrapping_add(1);
    true
}
/// Recompute the forest before the footprint pass. Index geometry once and apply
/// all translations together, keeping map editing linear in vertices and links.
pub fn update_map(map: &mut Map, tiles: &Tiles) -> bool {
    if !map
        .sectors
        .iter()
        .any(|s| s.properties.get_int(TARGET).is_some())
    {
        return false;
    }
    let index = Index::new(map, tiles);
    let poses = resolved(&index, tiles);
    let moves = map
        .sectors
        .iter()
        .filter(|s| s.properties.get_int(TARGET).is_some())
        .filter_map(|s| {
            let to = poses.get(&s.id).copied().flatten()?;
            let before = index.entries.get(&s.id)?.pose;
            if (to.at - before.at).magnitude_squared() <= 1e-10
                && (to.height - before.height).abs() <= 1e-5
                && ((to.yaw - before.yaw + 180.).rem_euclid(360.) - 180.).abs() <= 1e-4
            {
                return None;
            }
            movement(&index, s.id, to)
        })
        .collect();
    drop(index);
    let changed = apply_moves(map, moves);
    if changed {
        map.changed = map.changed.wrapping_add(1);
    }
    changed
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tile(tiles: &mut Tiles, source: &str) -> Uuid {
        let mesh = Document::load(source).unwrap().evaluate().unwrap().mesh;
        let id = Uuid::new_v4();
        let mut tile = Tile::default();
        tile.id = id;
        tile.isometric = Some(rusterix::isometric::SpriteLayout {
            directions: 4,
            frames_per_direction: 1,
            placement_bounds: (0..4)
                .map(|i| placement::mesh_bounds(&mesh, i as f32 * 90.).unwrap())
                .collect(),
            placement_connectors: (0..4)
                .map(|i| {
                    mesh.connectors
                        .iter()
                        .cloned()
                        .map(|mut c| {
                            c.position = rotate(c.position, i as f32 * 90.);
                            c.direction = rotate(c.direction, i as f32 * 90.);
                            c
                        })
                        .collect()
                })
                .collect(),
            ..Default::default()
        });
        tiles.insert(id, tile);
        id
    }
    fn place(map: &mut Map, tile: Uuid, x: f32, z: f32) -> u32 {
        map.possible_polygon.clear();
        let ids = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]].map(|p| {
            let id = map.find_free_vertex_id().unwrap();
            map.vertices
                .push(rusterix::Vertex::new(id, x + p[0], z + p[1]));
            id
        });
        for i in 0..4 {
            map.create_linedef_manual(ids[i], ids[(i + 1) % 4]);
        }
        let id = map.close_polygon_manual().unwrap();
        let s = map.find_sector_mut(id).unwrap();
        s.layer = Some(1);
        s.properties
            .set("source", Value::Source(rusterix::PixelSource::TileId(tile)));
        s.properties
            .set("iso_follow_mesh_bounds", Value::Bool(true));
        id
    }
    const WALL: &str =
        include_str!("../../../../creator/embedded/meshes/recipes/wall-segment.mzgeometry.json");
    #[test]
    fn joins_are_exact_and_linked_descendants_follow_dimensions_roundtrip_and_detach() {
        let mut tiles = Tiles::new();
        let id = tile(&mut tiles, WALL);
        let mut map = Map::default();
        let a = place(&mut map, id, 5., 5.);
        let b = place(&mut map, id, 6.3, 5.1);
        assert!(snap(&mut map, &tiles, b));
        let c = place(&mut map, id, 7.3, 5.1);
        assert!(snap(&mut map, &tiles, c));
        placement::update_map(&mut map, &tiles);
        assert!(
            (placement::anchor(&map, map.find_sector(b).unwrap())
                .unwrap()
                .x
                - 6.)
                .abs()
                < 1e-5
        );
        for view in &mut tiles[&id].isometric.as_mut().unwrap().placement_connectors {
            for c in view {
                if c.family == "wall" {
                    c.position = c.position.map(|v| v * 2.);
                }
            }
        }
        assert!(placement::update_map(&mut map, &tiles));
        assert!(
            (placement::anchor(&map, map.find_sector(c).unwrap())
                .unwrap()
                .x
                - 9.)
                .abs()
                < 1e-5
        );
        map.find_sector_mut(a)
            .unwrap()
            .properties
            .set("iso_yaw", Value::Float(90.));
        assert!(placement::update_map(&mut map, &tiles));
        let end = placement::anchor(&map, map.find_sector(c).unwrap()).unwrap();
        assert!((end - Vec2::new(5., 1.)).magnitude() < 1e-4);
        let json = serde_json::to_string(&map).unwrap();
        let mut map: Map = serde_json::from_str(&json).unwrap();
        assert!(!update_map(&mut map, &tiles));
        detach(map.find_sector_mut(b).unwrap());
        map.find_sector_mut(a)
            .unwrap()
            .properties
            .set("iso_yaw", Value::Float(180.));
        assert!(!update_map(&mut map, &tiles));
        assert_eq!(
            map.find_sector(c).unwrap().properties.get_int(TARGET),
            Some(b as i32)
        );
    }
    #[test]
    fn incompatible_occupied_and_cyclic_connections_do_not_move_placements() {
        let mut tiles = Tiles::new();
        let id = tile(&mut tiles, WALL);
        let mut map = Map::default();
        let a = place(&mut map, id, 5., 5.);
        let b = place(&mut map, id, 6.3, 5.1);
        assert!(snap(&mut map, &tiles, b));
        assert!(!snap(&mut map, &tiles, a));
        let c = place(&mut map, id, 6.05, 5.1);
        assert!(!snap(&mut map, &tiles, c));
        let bprops = map.find_sector(b).unwrap().properties.clone();
        let sa = map.find_sector_mut(a).unwrap();
        sa.properties.set(TARGET, Value::Int(b as i32));
        sa.properties.set(
            "iso_snap_local",
            Value::Str(bprops.get_str("iso_snap_remote").unwrap().into()),
        );
        sa.properties.set(
            "iso_snap_remote",
            Value::Str(bprops.get_str("iso_snap_local").unwrap().into()),
        );
        assert!(!update_map(&mut map, &tiles));
        for view in &mut tiles[&id].isometric.as_mut().unwrap().placement_connectors {
            for c in view {
                c.family = "other".into();
            }
        }
        // Existing stale connector IDs are harmless after recipe removal.
        tiles[&id]
            .isometric
            .as_mut()
            .unwrap()
            .placement_connectors
            .clear();
        assert!(!update_map(&mut map, &tiles));
    }
    #[test]
    fn copying_a_connected_assembly_remaps_links_and_copying_one_child_detaches() {
        let mut tiles = Tiles::new();
        let id = tile(&mut tiles, WALL);
        let mut map = Map::default();
        let a = place(&mut map, id, 5., 5.);
        let b = place(&mut map, id, 6.3, 5.1);
        assert!(snap(&mut map, &tiles, b));
        placement::update_map(&mut map, &tiles);
        map.find_sector_mut(a).unwrap().name = "Parent".into();
        map.find_sector_mut(b).unwrap().name = "Child".into();
        map.selected_sectors = vec![a, b];
        let clipboard = map.copy_selected(false);
        map.paste_at_position(&clipboard, Vec2::new(20., 20.));
        let pasted = &map.selected_sectors;
        let new_parent = map
            .sectors
            .iter()
            .find(|s| pasted.contains(&s.id) && s.name == "Parent")
            .unwrap()
            .id;
        let new_child = map
            .sectors
            .iter()
            .find(|s| pasted.contains(&s.id) && s.name == "Child")
            .unwrap()
            .id;
        assert_eq!(
            map.find_sector(new_child)
                .unwrap()
                .properties
                .get_int(TARGET),
            Some(new_parent as i32)
        );
        assert!(!update_map(&mut map, &tiles));
        assert!(
            placement::anchor(&map, map.find_sector(new_child).unwrap())
                .unwrap()
                .x
                > 20.
        );
        map.selected_sectors = vec![b];
        let clipboard = map.copy_selected(false);
        assert!(clipboard.sectors[0].properties.get_int(TARGET).is_none());
        map.paste_at_position(&clipboard, Vec2::new(30., 30.));
        assert!(!update_map(&mut map, &tiles));
        assert!(
            map.find_sector(map.selected_sectors[0])
                .unwrap()
                .properties
                .get_int(TARGET)
                .is_none()
        );
        map.delete_elements(&[], &[], &[a]);
        assert!(
            map.find_sector(b)
                .unwrap()
                .properties
                .get_int(TARGET)
                .is_none()
        );
        let replacement = place(&mut map, id, 50., 50.);
        assert_eq!(replacement, a);
        assert!(!update_map(&mut map, &tiles));
    }

    #[test]
    fn deep_reversed_forests_update_without_recursive_resolution() {
        let mut tiles = Tiles::new();
        let tile = tile(&mut tiles, WALL);
        let mut map = Map::default();
        let root = place(&mut map, tile, 0., 0.);
        let mut parent = root;
        for i in 1..=300 {
            let id = place(&mut map, tile, i as f32, 0.);
            let sector = map.find_sector_mut(id).unwrap();
            sector.properties.set(TARGET, Value::Int(parent as i32));
            sector
                .properties
                .set("iso_snap_local", Value::Str("join-start".into()));
            sector
                .properties
                .set("iso_snap_remote", Value::Str("join-end".into()));
            parent = id;
        }
        map.find_sector_mut(root)
            .unwrap()
            .properties
            .set("floor_height", Value::Float(2.));
        map.sectors.reverse();
        assert!(update_map(&mut map, &tiles));
        assert!(
            map.sectors
                .iter()
                .all(|s| s.properties.get_float_default("floor_height", 0.) == 2.)
        );
        assert!(!update_map(&mut map, &tiles));
    }

    #[test]
    fn roof_snaps_to_wall_height_and_follows_raised_wall() {
        let mut tiles = Tiles::new();
        let wall = tile(&mut tiles, WALL);
        let roof = tile(
            &mut tiles,
            include_str!("../../../../creator/embedded/meshes/recipes/gable-roof.mzgeometry.json"),
        );
        let mut map = Map::default();
        let a = place(&mut map, wall, 5., 5.);
        let b = place(&mut map, roof, 2.5, 5.);
        map.find_sector_mut(b)
            .unwrap()
            .properties
            .set("floor_height", Value::Float(2.4));
        assert!(snap(&mut map, &tiles, b));
        map.find_sector_mut(a)
            .unwrap()
            .properties
            .set("floor_height", Value::Float(1.));
        assert!(update_map(&mut map, &tiles));
        assert!(
            (map.find_sector(b)
                .unwrap()
                .properties
                .get_float_default("floor_height", 0.)
                - 3.4)
                .abs()
                < 1e-5
        );
    }
}
