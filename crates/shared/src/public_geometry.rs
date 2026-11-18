//! Public region layout shared by network content and client releases.
use rusterix::{
    GeometryObject, Map, Surface, Value, ValueContainer,
    map::{MapCamera, linedef::Linedef, sector::Sector, vertex::Vertex},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use theframework_core::Uuid;
const VISUAL: &[&str] = &[
    "source",
    "floor_source",
    "ceiling_source",
    "wall_source",
    "floor_height",
    "ceiling_height",
    "wall_height",
    "wall_width",
    "source_size",
    "visible",
    "blocking",
    "tile_mode",
    "rect",
    "tiles",
    "floor_tiles",
    "ceiling_tiles",
    "layer",
    "texture_scale_x",
    "texture_scale_y",
    "texture_offset_x",
    "texture_offset_y",
    "texture_rotation",
    "material",
    "is_door",
    "is_window",
    "profile_target",
    "profile_op",
    "profile_amount",
    "profile_depth",
    "profile_height",
    "profile_inset",
    "profile_outer_op",
    "profile_outer_depth",
    "profile_outer_height",
    "generated_profile",
    "generated_profile_host_linedef",
    "side_texture_scale_x",
    "side_texture_scale_y",
    "side_tile_mode",
    "rect_rendering",
    "underlay_source",
    "occlusion",
    "billboard_animation",
    "billboard_repeat_mode",
    "bevel_radius",
    "bevel_segments",
    "smoothness",
    "sector_feature",
    "linedef_feature",
    "stairs_direction",
    "stairs_fill_sides",
    "stairs_generated",
    "stairs_part",
    "stairs_source_a",
    "stairs_source_b",
    "stairs_steps",
    "stairs_total_height",
    "terrain_control",
    "terrain_enabled",
    "terrain_mode",
    "terrain_edge_falloff",
    "terrain_end_height",
    "terrain_exclusion_subdiv",
    "terrain_falloff_distance",
    "terrain_falloff_steepness",
    "terrain_road_organic",
    "terrain_shape",
    "terrain_shape_blend",
    "terrain_shape_height",
    "terrain_shape_mode",
    "terrain_shape_width",
    "terrain_smooth",
    "terrain_start_height",
    "terrain_subdivisions",
    "terrain_tile_falloff",
    "terrain_width",
    "default_terrain_tile",
    "ridge_height",
    "ridge_falloff_distance",
    "ridge_falloff_steepness",
    "ridge_plateau_width",
    "ridge_subdiv",
    "ridge_water_enabled",
    "ridge_water_level",
    "iso_anchor",
    "iso_anchor_offset",
    "iso_cutaway",
    "iso_edit_height",
    "iso_interior",
    "iso_snap_target",
    "iso_walkable",
    "iso_yaw",
    "roof_height",
    "roof_overhang",
    "roof_style",
    "feature_connector_count",
    "feature_connector_drop",
    "feature_connector_size",
    "feature_connector_style",
    "feature_depth",
    "feature_height",
    "feature_height_variation",
    "feature_layout_spacing",
    "feature_lean_amount",
    "feature_lean_randomness",
    "feature_post_shape",
    "feature_post_size",
    "feature_round_segments",
    "feature_segment_size",
    "feature_shape",
    "feature_top_height",
    "feature_top_mode",
    "tile_size",
    "no_rect_geo",
];
fn allowed(value: &Value) -> bool {
    matches!(
        value,
        Value::Bool(_)
            | Value::Int(_)
            | Value::UInt(_)
            | Value::Float(_)
            | Value::Vec2(_)
            | Value::Vec3(_)
            | Value::Vec4(_)
            | Value::Id(_)
            | Value::Source(_)
            | Value::Color(_)
            | Value::HeightPoints(_)
            | Value::TileOverrides(_)
            | Value::BlendOverrides(_)
    )
}
fn properties(source: &ValueContainer) -> ValueContainer {
    let mut out = ValueContainer::default();
    for (key, value) in source.iter() {
        if VISUAL.contains(&key.as_str()) && allowed(value) {
            out.set(key, value.clone());
        }
    }
    out
}
fn merge(base: &mut ValueContainer, public: &ValueContainer) {
    // Keep release-local rendering extensions, but allow removal of published
    // properties. This never exports a local/source property back to the host.
    for key in VISUAL {
        base.remove(key);
    }
    for (key, value) in public.iter() {
        base.set(key, value.clone());
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub struct RegionGeometry {
    pub id: Uuid,
    pub revision: u32,
    pub camera: MapCamera,
    pub grid_size: f32,
    pub subdivisions: f32,
    pub vertices: Vec<Vertex>,
    pub linedefs: Vec<Linedef>,
    pub sectors: Vec<Sector>,
    pub objects: Vec<GeometryObject>,
    pub surfaces: Vec<(Uuid, Surface)>,
    pub profiles: Vec<(Uuid, RegionGeometry)>,
    pub properties: ValueContainer,
}
impl RegionGeometry {
    pub fn capture(map: &Map) -> Result<Self, String> {
        Self::capture_at(map, 0)
    }
    fn capture_at(map: &Map, depth: usize) -> Result<Self, String> {
        if depth > 8 {
            return Err("Region profiles nested too deeply".into());
        }
        let mut vertices = map.vertices.clone();
        for v in &mut vertices {
            v.name.clear();
            v.properties = properties(&v.properties);
        }
        let mut linedefs = map.linedefs.clone();
        for l in &mut linedefs {
            l.name.clear();
            l.properties = properties(&l.properties);
        }
        let mut sectors = map.sectors.clone();
        for s in &mut sectors {
            s.name.clear();
            s.properties = properties(&s.properties);
        }
        let mut objects = map.geometry_objects.clone();
        for o in &mut objects {
            o.name.clear();
            o.group.clear();
            o.tags.clear();
            o.properties = properties(&o.properties);
        }
        let mut profiles = map
            .profiles
            .iter()
            .map(|(id, map)| Ok((*id, Self::capture_at(map, depth + 1)?)))
            .collect::<Result<Vec<_>, String>>()?;
        profiles.sort_by_key(|(id, _)| *id);
        Ok(Self {
            id: map.id,
            revision: map.changed,
            camera: map.camera,
            grid_size: map.grid_size,
            subdivisions: map.subdivisions,
            vertices,
            linedefs,
            sectors,
            objects,
            surfaces: map
                .surfaces
                .iter()
                .map(|(id, s)| (*id, s.clone()))
                .collect(),
            profiles,
            properties: properties(&map.properties),
        })
    }
    /// Stricter release policy in addition to transport topology validation.
    pub fn validate_public(&self) -> Result<(), String> {
        self.validate()?;
        fn clean(p: &ValueContainer) -> bool {
            p.iter()
                .all(|(k, v)| VISUAL.contains(&k.as_str()) && allowed(v))
        }
        if !clean(&self.properties)
            || self
                .vertices
                .iter()
                .any(|v| !v.name.is_empty() || !clean(&v.properties))
            || self
                .linedefs
                .iter()
                .any(|v| !v.name.is_empty() || !clean(&v.properties))
            || self
                .sectors
                .iter()
                .any(|v| !v.name.is_empty() || !clean(&v.properties))
            || self.objects.iter().any(|v| {
                !v.name.is_empty()
                    || !v.group.is_empty()
                    || !v.tags.is_empty()
                    || !clean(&v.properties)
            })
        {
            return Err("Private authoring metadata in public geometry".into());
        }
        for (_, profile) in &self.profiles {
            profile.validate_public()?;
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut count = 0;
        self.validate_at(0, &mut count)
    }
    fn validate_at(&self, depth: usize, count: &mut usize) -> Result<(), String> {
        *count += self.vertices.len()
            + self.linedefs.len()
            + self.sectors.len()
            + self.objects.len()
            + self.surfaces.len();
        if depth > 8
            || *count > 100_000
            || !self.grid_size.is_finite()
            || !self.subdivisions.is_finite()
        {
            return Err("Invalid region geometry bounds".into());
        }
        let vertices: HashSet<_> = self.vertices.iter().map(|v| v.id).collect();
        let lines: HashSet<_> = self.linedefs.iter().map(|l| l.id).collect();
        let sectors: HashSet<_> = self.sectors.iter().map(|s| s.id).collect();
        if vertices.len() != self.vertices.len()
            || lines.len() != self.linedefs.len()
            || sectors.len() != self.sectors.len()
            || self
                .vertices
                .iter()
                .any(|v| ![v.x, v.y, v.z].iter().all(|v| v.is_finite()))
            || self
                .linedefs
                .iter()
                .any(|l| !vertices.contains(&l.start_vertex) || !vertices.contains(&l.end_vertex))
            || self
                .sectors
                .iter()
                .any(|s| s.linedefs.iter().any(|l| !lines.contains(l)))
        {
            return Err("Invalid region topology".into());
        }
        for object in &self.objects {
            *count += object.vertices.len() + object.faces.len();
            if *count > 100_000
                || object
                    .vertices
                    .iter()
                    .any(|p| ![p.x, p.y, p.z].iter().all(|v| v.is_finite()))
                || object.transform.iter().flatten().any(|v| !v.is_finite())
                || object
                    .faces
                    .iter()
                    .any(|f| f.indices.iter().any(|i| *i >= object.vertices.len()))
            {
                return Err("Invalid geometry mesh".into());
            }
        }
        for (_, profile) in &self.profiles {
            profile.validate_at(depth + 1, count)?;
        }
        Ok(())
    }
    /// Replace public layout while retaining release-local art/bakes and prop
    /// materials. Occupants arrive only through private gameplay replication.
    pub fn apply(&self, map: &mut Map) {
        let mut vertices = self.vertices.clone();
        for v in &mut vertices {
            if let Some(old) = map.vertices.iter().find(|o| o.id == v.id) {
                v.name.clone_from(&old.name);
                let mut p = old.properties.clone();
                merge(&mut p, &v.properties);
                v.properties = p;
            }
        }
        let mut lines = self.linedefs.clone();
        for l in &mut lines {
            if let Some(old) = map.linedefs.iter().find(|o| o.id == l.id) {
                l.name.clone_from(&old.name);
                let mut p = old.properties.clone();
                merge(&mut p, &l.properties);
                l.properties = p;
            }
        }
        let mut sectors = self.sectors.clone();
        for s in &mut sectors {
            if let Some(old) = map.sectors.iter().find(|o| o.id == s.id) {
                s.name.clone_from(&old.name);
                let mut p = old.properties.clone();
                merge(&mut p, &s.properties);
                s.properties = p;
            }
        }
        let mut objects = self.objects.clone();
        for o in &mut objects {
            if let Some(old) = map.geometry_objects.iter().find(|v| v.id == o.id) {
                o.name.clone_from(&old.name);
                o.group.clone_from(&old.group);
                o.tags.clone_from(&old.tags);
                let mut p = old.properties.clone();
                merge(&mut p, &o.properties);
                o.properties = p;
            }
        }
        let old_profiles = std::mem::take(&mut map.profiles);
        map.profiles = self
            .profiles
            .iter()
            .map(|(id, profile)| {
                let mut map = old_profiles.get(id).cloned().unwrap_or_default();
                profile.apply(&mut map);
                (*id, map)
            })
            .collect();
        map.id = self.id;
        map.changed = self.revision;
        map.camera = self.camera;
        map.grid_size = self.grid_size;
        map.subdivisions = self.subdivisions;
        map.vertices = vertices;
        map.linedefs = lines;
        map.sectors = sectors;
        map.geometry_objects = objects;
        map.surfaces = self.surfaces.iter().cloned().collect();
        merge(&mut map.properties, &self.properties);
        map.entities.clear();
        map.items.clear();
    }
}
