//! Borrowed content checks for an immutable static overview publication.
use super::*;

impl Map {
    /// Compare overview source data without cloning/serializing a complete map.
    /// Root viewport, selection and occupants are deliberately excluded. Profile
    /// maps retain their authored serialized inputs. IDs/revisions alone cannot
    /// establish equality: undo and public in-place edits can reuse both.
    pub fn same_overview_source(&self, other: &Self) -> bool {
        same_map(self, other, true)
    }
}

fn same_map(map: &Map, other: &Map, overview: bool) -> bool {
    // Exhaustive destructuring makes new Map fields an explicit source-policy
    // decision. The lookup is derived; temporary fields are not serialized.
    let Map {
        geometry_lookup: _,
        id,
        name,
        offset: _,
        grid_size: _,
        subdivisions,
        possible_polygon: _,
        curr_grid_pos: _,
        curr_grid_pos_3d: _,
        curr_mouse_pos: _,
        curr_rectangle: _,
        vertices,
        linedefs,
        sectors,
        geometry_objects,
        face_emissions,
        face_particle_emissions,
        block_prop_instances,
        block_prop_surface_placements,
        camera,
        lights,
        entities: _,
        items: _,
        selected_vertices: _,
        selected_linedefs: _,
        selected_sectors: _,
        selected_geometry_objects: _,
        selected_block_prop_instances: _,
        selected_geometry_vertices: _,
        selected_geometry_faces: _,
        selected_geometry_surface_points: _,
        selected_geometry_surface_segments: _,
        geometry_selection_mode: _,
        selected_entity_item: _,
        properties,
        surfaces,
        profiles,
        orthographic_bake,
        changed: _,
    } = map;
    id == &other.id
        && name == &other.name
        && subdivisions == &other.subdivisions
        && camera == &other.camera
        && vertices == &other.vertices
        && linedefs.len() == other.linedefs.len()
        && linedefs
            .iter()
            .zip(&other.linedefs)
            .all(|(a, b)| linedef_source(a) == linedef_source(b))
        && sectors.len() == other.sectors.len()
        && sectors
            .iter()
            .zip(&other.sectors)
            .all(|(a, b)| sector_source(a) == sector_source(b))
        && geometry_objects == &other.geometry_objects
        && face_emissions == &other.face_emissions
        && face_particle_emissions == &other.face_particle_emissions
        && block_prop_instances == &other.block_prop_instances
        && block_prop_surface_placements == &other.block_prop_surface_placements
        && lights == &other.lights
        && properties.same_serialized_source(&other.properties)
        && orthographic_bake == &other.orthographic_bake
        && surfaces.len() == other.surfaces.len()
        && surfaces
            .iter()
            .zip(other.surfaces.iter())
            .all(|((id, a), (other_id, b))| {
                id == other_id && surface_source(a) == surface_source(b)
            })
        && profiles.len() == other.profiles.len()
        && profiles.iter().all(|(id, profile)| {
            other
                .profiles
                .get(id)
                .is_some_and(|other| same_map(profile, other, false))
        })
        && (overview || same_profile_state(map, other))
}

// Existing Linedef/Sector equality is intentionally geometric (undirected
// endpoints / unordered boundary membership). Source identity also includes
// direction, draw order, IDs and authored properties; do not change that API.
struct Properties<'a>(&'a ValueContainer);
impl PartialEq for Properties<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_serialized_source(other.0)
    }
}
fn linedef_source(line: &Linedef) -> impl PartialEq + '_ {
    let Linedef {
        id,
        creator_id,
        name,
        start_vertex,
        end_vertex,
        sector_ids,
        properties,
    } = line;
    (
        id,
        creator_id,
        name,
        start_vertex,
        end_vertex,
        sector_ids,
        Properties(properties),
    )
}
fn sector_source(sector: &Sector) -> impl PartialEq + '_ {
    let Sector {
        id,
        creator_id,
        name,
        linedefs,
        footprint_topology,
        properties,
        layer,
    } = sector;
    (
        id,
        creator_id,
        name,
        linedefs,
        footprint_topology,
        Properties(properties),
        layer,
    )
}

fn surface_source(surface: &Surface) -> impl PartialEq + '_ {
    let Surface {
        id,
        sector_id,
        plane,
        frame,
        edit_uv,
        extrusion,
        profile,
        world_vertices: _,
    } = surface;
    (id, sector_id, plane, frame, edit_uv, extrusion, profile)
}

fn same_profile_state(a: &Map, b: &Map) -> bool {
    a.offset == b.offset && a.grid_size == b.grid_size && a.changed == b.changed
        && a.selected_vertices == b.selected_vertices
        && a.selected_linedefs == b.selected_linedefs
        && a.selected_sectors == b.selected_sectors
        && a.selected_geometry_objects == b.selected_geometry_objects
        && a.selected_entity_item == b.selected_entity_item
        // Profiles normally have no occupants. Retain the complete existing
        // serialized policy for the uncommon populated profile without making
        // all entities/items/maps acquire general equality implementations.
        && same_occupants(&a.entities, &b.entities)
        && same_occupants(&a.items, &b.items)
}
fn same_occupants<T: Serialize>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len()
        && (a.is_empty()
            || serde_json::to_vec(a).expect("serializable profile occupants")
                == serde_json::to_vec(b).expect("serializable profile occupants"))
}
