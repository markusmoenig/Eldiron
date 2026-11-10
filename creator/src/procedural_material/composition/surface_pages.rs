//! World-coordinate coverage of whole surfaces and reusable map materials.
use super::*;
use rusterix::{PixelSource, ValueContainer};
use shared::material_composition::{CompositionSurface as Surface, MapMaterialBinding};
pub(super) const PAGES: &str = "eldiron-surface-pages";
pub(super) const AXES: &str = "eldiron-surface-axes";
const FLOOR: [[f32; 3]; 2] = [[1., 0., 0.], [0., 0., 1.]];
fn point(p: Vec3<f32>, axes: [[f32; 3]; 2]) -> [f32; 2] {
    axes.map(|a| a[0] * p.x + a[1] * p.y + a[2] * p.z)
}
fn axes(points: &[Vec3<f32>]) -> [[f32; 3]; 2] {
    if points.len() < 3 {
        return FLOOR;
    }
    let n = (points[1] - points[0]).cross(points[2] - points[0]);
    if n.y.abs() >= n.x.abs().max(n.z.abs()) {
        return FLOOR;
    }
    let mut u = Vec3::new(n.z, 0., -n.x);
    if u.magnitude_squared() < 0.00001 {
        return FLOOR;
    }
    u = u.normalized();
    if u.x < -0.00001 || (u.x.abs() < 0.00001 && u.z < 0.) {
        u = -u;
    }
    [[u.x, 0., u.z], [0., -1., 0.]]
}
fn layout(points: Vec<[f32; 2]>, axes: [[f32; 3]; 2]) -> Option<rusterix::TileUvPages> {
    let mut min = [f32::INFINITY; 2];
    let mut max = [f32::NEG_INFINITY; 2];
    for p in points {
        if p.iter().all(|v| v.is_finite()) {
            for c in 0..2 {
                min[c] = min[c].min(p[c]);
                max[c] = max[c].max(p[c]);
            }
        }
    }
    if !min.iter().all(|v| v.is_finite()) {
        return None;
    }
    let mut units = 1_u32;
    loop {
        let origin = min.map(|v| (v / units as f32).floor() as i32);
        let size: [u32; 2] = std::array::from_fn(|c| {
            ((max[c] / units as f32).ceil() - origin[c] as f32).max(1.) as u32
        });
        if u64::from(size[0]) * u64::from(size[1]) <= 4096 {
            return Some(rusterix::TileUvPages {
                animation_frames: 1,
                repeat_period: [0.; 2],
                origin,
                size,
                axes,
                units_per_page: units,
            });
        }
        units = units.checked_mul(2)?;
    }
}
fn face_points(object: &rusterix::GeometryObject, face: &rusterix::GeometryFace) -> Vec<Vec3<f32>> {
    face.indices
        .iter()
        .filter_map(|i| object.vertices.get(*i))
        .map(|p| object.transform_point(*p))
        .collect()
}
/// Include caps and extruded sides: a sector can represent a horizontal floor
/// or a vertical wall, and its source may also cover its extruded side faces.
fn sector_uv_points(map: &Map, id: u32) -> Vec<[f32; 2]> {
    let Some(vertices) = map.find_sector(id).and_then(|s| s.vertices_world(map)) else {
        return Vec::new();
    };
    let basis = axes(&vertices);
    let mut points: Vec<_> = vertices.iter().map(|p| point(*p, basis)).collect();
    for surface in map.surfaces.values().filter(|s| s.sector_id == id) {
        if !surface.extrusion.enabled {
            continue;
        }
        let (front, back) = surface.extrusion.offsets();
        let normal = surface.plane.normal;
        for offset in [front, back] {
            points.extend(vertices.iter().map(|p| point(*p + normal * offset, basis)));
        }
        for i in 0..vertices.len() {
            let a = vertices[i];
            let b = vertices[(i + 1) % vertices.len()];
            let face = [
                a + normal * front,
                b + normal * front,
                b + normal * back,
                a + normal * back,
            ];
            let basis = axes(&face);
            points.extend(face.into_iter().map(|p| point(p, basis)));
        }
    }
    points
}
pub(super) fn for_surface(
    project: &Project,
    binding: &MapMaterialBinding,
) -> Option<rusterix::TileUvPages> {
    let map = project
        .regions
        .iter()
        .map(|r| &r.map)
        .chain(project.screens.values().map(|s| &s.map))
        .find(|map| map.id == binding.map)?;
    let points;
    let basis;
    match binding.surface {
        Surface::Sector { id, .. } => {
            return layout(sector_uv_points(map, id), [[0.; 3]; 2]);
        }
        Surface::GeometryFace { object, face } => {
            let object = map.geometry_objects.iter().find(|o| o.id == object)?;
            points = face_points(object, object.faces.get(face)?);
            basis = axes(&points);
        }
        Surface::Linedef { id, .. } => {
            let line = map.find_linedef(id)?;
            let a = map.find_vertex(line.start_vertex)?;
            let b = map.find_vertex(line.end_vertex)?;
            let direction = Vec3::new(b.x - a.x, 0., b.y - a.y);
            let length = direction.magnitude();
            if length < 0.00001 {
                return None;
            }
            let mut u = direction / length;
            if u.x < -0.00001 || (u.x.abs() < 0.00001 && u.z < 0.) {
                u = -u;
            }
            basis = [[u.x, 0., u.z], [0., -1., 0.]];
            let mut low = 0_f32;
            let mut high = line.properties.get_float_default("wall_height", 3.).max(3.);
            for surface in map
                .surfaces
                .values()
                .filter(|s| line.sector_ids.contains(&s.sector_id))
            {
                if let Some(vertices) = map
                    .find_sector(surface.sector_id)
                    .and_then(|s| s.vertices_world(map))
                {
                    let offsets = if surface.extrusion.enabled {
                        surface.extrusion.offsets()
                    } else {
                        (0., 0.)
                    };
                    for p in vertices {
                        for offset in [offsets.0, offsets.1] {
                            let y = (p + surface.plane.normal * offset).y;
                            low = low.min(y);
                            high = high.max(y);
                        }
                    }
                }
            }
            points = vec![Vec3::new(a.x, low, a.y), Vec3::new(b.x, high, b.y)];
        }

        _ => return None,
    }
    layout(points.into_iter().map(|p| point(p, basis)).collect(), basis)
}
fn is_source(project: &Project, source: Option<&PixelSource>, id: Uuid) -> bool {
    match source {
        Some(PixelSource::TileId(tile)) => *tile == id
            || project.tiles.get(tile).and_then(|t|t.isometric.as_ref()).is_some_and(|l|l.top_material==Some(id))
            || matches!(shared::isometric::tile_recipe(project,*tile),Some(shared::isometric::TileRecipe::Material(recipe)) if recipe==id),
        Some(PixelSource::Color(color)) => PixelSource::color_tile_uuid(color) == id,
        Some(PixelSource::PaletteIndex(index)) => PixelSource::palette_tile_uuid(*index) == id,
        _ => false,
    }
}
pub(super) fn for_template(project: &Project, id: Uuid) -> Option<rusterix::TileUvPages> {
    if project.tiles.get(&id).is_some_and(|t| t.paint.is_some()) {
        return Some(rusterix::TileUvPages { animation_frames:4, repeat_period:[1.;2], origin:[0;2], size:[1;2], axes:[[0.;3];2], units_per_page:1 });
    }
    let mut points = Vec::new();
    for map in project
        .regions
        .iter()
        .map(|r| &r.map)
        .chain(project.screens.values().map(|s| &s.map))
    {
        let outputs = rusterix::chunkbuilder::terrain_material::outputs(map);
        // Derived outputs cover the same world domain as their painted background.
        let id = outputs.iter().find_map(|(base, output)| (*output == id).then_some(*base)).unwrap_or(id);
        for sector in &map.sectors {
            if is_source(project, sector.properties.get_source("source"), id)
                || is_source(project, sector.properties.get_source("ceiling_source"), id)
            {
                points.extend(sector_uv_points(map, sector.id));
            }
            add_overrides(project, &sector.properties, id, &mut points);
        }
        add_overrides(project, &map.properties, id, &mut points);
        if is_source(
            project,
            map.properties.get_source("default_terrain_tile"),
            id,
        ) {
            let bounds = rusterix::chunkbuilder::terrain_surface::TerrainSurface::cached(map)
                .map_or_else(|| map.bbox(), |t| t.bounds);
            points.extend([[bounds.min.x, bounds.min.y], [bounds.max.x, bounds.max.y]]);
        }
        for object in &map.geometry_objects {
            for face in &object.faces {
                if is_source(project, face.tile.as_ref(), id)
                    || face.tiles.values().any(|s| is_source(project, Some(s), id))
                {
                    let p = face_points(object, face);
                    let a = axes(&p);
                    points.extend(p.into_iter().map(|p| point(p, a)));
                }
            }
        }
        for line in &map.linedefs {
            for row in 1..=4 {
                if is_source(
                    project,
                    line.properties.get_source(&format!("row{row}_source")),
                    id,
                ) {
                    let binding = MapMaterialBinding {
                        map: map.id,
                        surface: Surface::Linedef { id: line.id, row },
                    };
                    if let Some(pages) = for_surface(project, &binding) {
                        add_extent(&pages, &mut points);
                    }
                }
            }
        }
    }
    layout(points, [[0.; 3]; 2])
}
fn add_extent(pages: &rusterix::TileUvPages, points: &mut Vec<[f32; 2]>) {
    points.push(pages.origin.map(|v| v as f32 * pages.units_per_page as f32));
    points.push(std::array::from_fn(|i| {
        (pages.origin[i] as f32 + pages.size[i] as f32) * pages.units_per_page as f32
    }));
}
fn add_overrides(
    project: &Project,
    properties: &ValueContainer,
    id: Uuid,
    points: &mut Vec<[f32; 2]>,
) {
    for value in properties.values() {
        match value {
            Value::TileOverrides(tiles) => {
                for ((x, y), source) in tiles {
                    if is_source(project, Some(source), id) {
                        points.extend([[*x as f32, *y as f32], [*x as f32 + 1., *y as f32 + 1.]]);
                    }
                }
            }
            Value::BlendOverrides(tiles) => {
                for ((x, y), (_, source)) in tiles {
                    if is_source(project, Some(source), id) {
                        points.extend([[*x as f32, *y as f32], [*x as f32 + 1., *y as f32 + 1.]]);
                    }
                }
            }
            _ => {}
        }
    }
}
pub(in crate::procedural_material) fn apply(
    asset: &mut EditableMaterialAsset,
    pages: Option<&rusterix::TileUvPages>,
) {
    let repeating = asset
        .root
        .project
        .materials
        .iter()
        .find(|m| m.id == asset.selected_material)
        .is_some_and(|m| m.tiling.mode != denrim_materials::periodic::Mode::Raw);
    let pages = if repeating { None } else { pages };
    asset
        .instances
        .retain(|r| r.id != PAGES && r.id != AXES && !(repeating && r.id == super::SURFACE_DOMAIN));
    if let Some(pages) = pages {
        let mut domain = InstanceRecord::new(PAGES);
        domain.uv_transform = [
            pages.size[0] as f32,
            pages.units_per_page as f32,
            0.,
            pages.size[1] as f32,
            pages.origin[0] as f32,
            pages.origin[1] as f32,
        ];
        let mut axes = InstanceRecord::new(AXES);
        axes.uv_transform = [
            pages.axes[0][0],
            pages.axes[0][1],
            pages.axes[0][2],
            pages.axes[1][0],
            pages.axes[1][1],
            pages.axes[1][2],
        ];
        asset.instances.extend([domain, axes]);
    }
}

pub(crate) fn decode(
    asset: &EditableMaterialAsset,
) -> Result<Option<rusterix::TileUvPages>, String> {
    let Some(record) = asset.instances.iter().find(|r| r.id == PAGES) else {
        return Ok(None);
    };
    let m = record.uv_transform;
    let size = [m[0] as u32, m[3] as u32];
    let units = (m[1] as u32).max(1);
    if size.contains(&0) || u64::from(size[0]) * u64::from(size[1]) > 4096 {
        return Err("Invalid surface page dimensions".into());
    }
    let a = asset
        .instances
        .iter()
        .find(|r| r.id == AXES)
        .map_or([0.; 6], |r| r.uv_transform);
    Ok(Some(rusterix::TileUvPages {
        animation_frames: 1,
        repeat_period: [0.; 2],
        origin: [m[4] as i32, m[5] as i32],
        size,
        axes: [[a[0], a[1], a[2]], [a[3], a[4], a[5]]],
        units_per_page: units,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_material_coverage_follows_projected_iso_output_placements() {
        let mut p = Project::default();
        let source = Uuid::new_v4();
        let output = Uuid::new_v4();
        p.procedural_materials.insert(source, "source".into());
        p.isometric_bakes.insert(
            output,
            shared::isometric::Bake {
                recipe: source,
                ground: true,
                yaw: 0.,
                directions: 1,
                motion: None,
                frames: 1,
                signature: 0,
            },
        );
        let map = &mut p.regions[0].map;
        *map = Map::default();
        map.camera = rusterix::MapCamera::TwoDIso;
        super::super::tests::add_floor(map, 1, output, [-2., -1.], [3., 4.]);
        let first = for_template(&p, source)
            .expect("Iso placements must give their source a world UV extent");
        assert_eq!(first.origin, [-2, -1]);
        assert_eq!(first.size, [5, 5]);
        for v in &mut p.regions[0].map.vertices {
            v.x += 7.;
        }
        let moved = for_template(&p, source).unwrap();
        assert_eq!(moved.origin, [5, -1]);
        assert_eq!(moved.size, [5, 5]);
    }
    #[test]
    fn screen_only_material_bakes_cover_authored_grid_and_follow_edits() {
        let mut project = Project::default();
        let tile = Uuid::new_v4();
        let mut screen = shared::screen::Screen::new();
        let v = [[-3., -2.], [1., -2.], [1., 2.], [-3., 2.]]
            .map(|p| screen.map.add_vertex_at(p[0], p[1]));
        for i in 0..4 {
            screen.map.create_linedef(v[i], v[(i + 1) % 4]);
        }
        screen.map.sectors[0]
            .properties
            .set("source", rusterix::Value::Source(PixelSource::TileId(tile)));
        let id = screen.id;
        project.screens.insert(id, screen);
        let pages = for_template(&project, tile).unwrap();
        assert_eq!(pages.origin, [-3, -2]);
        assert_eq!(pages.size, [4, 4]);
        let binding = MapMaterialBinding {
            map: project.screens[&id].map.id,
            surface: Surface::Sector {
                id: project.screens[&id].map.sectors[0].id,
                ceiling: false,
            },
        };
        assert_eq!(for_surface(&project, &binding).unwrap(), pages);
        project.screens[&id].map.vertices[0].x = -5.;
        let pages = for_template(&project, tile).unwrap();
        assert_eq!(pages.origin, [-5, -2]);
        assert_eq!(pages.size, [6, 4]);
    }
    #[test]
    fn page_coverage_keeps_negative_world_coordinates_and_bounds_large_surfaces() {
        let small = layout(vec![[-2.25, -1.], [3.5, 2.]], FLOOR).unwrap();
        assert_eq!(small.origin, [-3, -1]);
        assert_eq!(small.size, [7, 3]);
        assert_eq!(small.units_per_page, 1);
        let huge = layout(vec![[-1000., -500.], [1000., 500.]], FLOOR).unwrap();
        assert!(u64::from(huge.size[0]) * u64::from(huge.size[1]) <= 4096);
        assert!(huge.units_per_page > 1);
        for i in 0..2 {
            assert!(huge.origin[i] as f32 * huge.units_per_page as f32 <= [-1000., -500.][i]);
            assert!(
                (huge.origin[i] as f32 + huge.size[i] as f32) * huge.units_per_page as f32
                    >= [1000., 500.][i]
            );
        }
    }
    #[test]
    fn vertical_sector_bounds_include_height_without_cached_world_vertices() {
        let mut map = Map::default();
        let vertices = [[-3., 7., 2.], [1., 7., 2.], [1., 7., 5.], [-3., 7., 5.]]
            .map(|p| map.add_vertex_at_3d(p[0], p[1], p[2], false));
        let lines = (0..4)
            .map(|i| map.create_linedef_manual(vertices[i], vertices[(i + 1) % 4]))
            .collect();
        map.sectors.push(rusterix::Sector::new(17, lines));
        let pages = layout(sector_uv_points(&map, 17), [[0.; 3]; 2]).unwrap();
        assert_eq!(pages.origin, [-3, -5]);
        assert_eq!(pages.size, [4, 3]);
        let mut surface = rusterix::Surface::new(17);
        surface.calculate_geometry(&map);
        surface.extrusion.enabled = true;
        surface.extrusion.depth = 1.;
        surface.world_vertices.clear();
        map.surfaces.insert(surface.id, surface);
        let extruded = layout(sector_uv_points(&map, 17), [[0.; 3]; 2]).unwrap();
        assert!(extruded.origin[1] <= -5);
        assert!(extruded.origin[1] as i64 + extruded.size[1] as i64 >= 8);
    }
    #[test]
    fn floor_and_rotated_wall_projections_use_world_distances() {
        assert_eq!(
            axes(&[
                Vec3::new(0., 0., 0.),
                Vec3::new(2., 0., 0.),
                Vec3::new(2., 0., 3.)
            ]),
            FLOOR
        );
        let wall = [
            Vec3::new(-1., 0., -1.),
            Vec3::new(1., 0., 1.),
            Vec3::new(1., 2., 1.),
        ];
        let basis = axes(&wall);
        let a = point(wall[0], basis);
        let b = point(wall[1], basis);
        assert!((b[0] - a[0] - 8_f32.sqrt()).abs() < 0.0001);
        assert_eq!(point(wall[2], basis)[1], -2.);
    }
}
