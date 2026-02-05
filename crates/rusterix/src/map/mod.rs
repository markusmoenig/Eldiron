pub mod bbox;
pub mod geometry;
pub mod light;
pub mod linedef;
pub mod meta;
pub mod mini;
pub mod particle;
pub mod pixelsource;
pub mod sector;
pub mod softrig;
pub mod surface;
pub mod tile;
pub mod vertex;

use crate::{
    BBox, Keyform, MapMini, PixelSource, ShapeFXGraph, SoftRig, SoftRigAnimator, Surface, Terrain,
    Value, ValueContainer,
};
use codegridfx::Module;
use indexmap::IndexMap;
use std::collections::VecDeque;
use theframework::prelude::{FxHashMap, FxHashSet};

use linedef::*;
use sector::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::{Vec2, Vec3, Vec4};
use vertex::*;

use crate::{Entity, Item, Light};

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Copy)]
pub enum MapCamera {
    TwoD,
    ThreeDIso,
    ThreeDFirstPerson,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Copy)]
pub enum MapToolType {
    General,
    Selection,
    Vertex,
    Linedef,
    Sector,
    Effects,
    Rect,
    Game,
    MiniMap,
    World,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Map {
    #[serde(default)]
    pub id: Uuid,
    pub name: String,

    pub offset: Vec2<f32>,
    pub grid_size: f32,
    pub subdivisions: f32,

    #[serde(default)]
    pub terrain: Terrain,

    // When adding linedefs we keep track of them to check if we have a closed polygon
    #[serde(skip)]
    pub possible_polygon: Vec<u32>,

    // For temporary line previews
    #[serde(skip)]
    pub curr_grid_pos: Option<Vec2<f32>>,
    #[serde(skip)]
    pub curr_mouse_pos: Option<Vec2<f32>>,
    #[serde(skip)]
    pub curr_rectangle: Option<(Vec2<f32>, Vec2<f32>)>,

    pub vertices: Vec<Vertex>,
    pub linedefs: Vec<Linedef>,
    pub sectors: Vec<Sector>,

    #[serde(default)]
    pub shapefx_graphs: IndexMap<Uuid, ShapeFXGraph>,

    pub sky_texture: Option<Uuid>,

    // Camera Mode
    pub camera: MapCamera,
    #[serde(skip)]
    pub camera_xz: Option<Vec2<f32>>,
    #[serde(skip)]
    pub look_at_xz: Option<Vec2<f32>>,

    // Lights
    pub lights: Vec<Light>,

    // Entities
    pub entities: Vec<Entity>,

    // Items
    pub items: Vec<Item>,

    // Selection
    pub selected_vertices: Vec<u32>,
    pub selected_linedefs: Vec<u32>,
    pub selected_sectors: Vec<u32>,

    pub selected_entity_item: Option<Uuid>,

    // Meta Data
    #[serde(default)]
    pub properties: ValueContainer,

    /// All SoftRigs in the map, each defining vertex-based keyforms
    #[serde(default)]
    pub softrigs: IndexMap<Uuid, SoftRig>,

    /// Currently edited SoftRig, or None for base geometry
    #[serde(skip)]
    pub editing_rig: Option<Uuid>,

    /// Vertex animation
    #[serde(skip)]
    pub soft_animator: Option<SoftRigAnimator>,

    /// The surfaces of the 3D meshes.
    #[serde(default)]
    pub surfaces: IndexMap<Uuid, Surface>,

    /// The optional profile of surfaces.
    #[serde(default)]
    pub profiles: FxHashMap<Uuid, Map>,

    /// The shaders used in the map.
    #[serde(default)]
    pub shaders: IndexMap<Uuid, Module>,

    // Change counter, right now only used for materials
    // to indicate when to refresh live updates
    #[serde(default)]
    pub changed: u32,
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl Map {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New Map".to_string(),

            offset: Vec2::zero(),
            grid_size: 30.0,
            subdivisions: 1.0,

            terrain: Terrain::default(),

            possible_polygon: vec![],
            curr_grid_pos: None,
            curr_mouse_pos: None,
            curr_rectangle: None,

            vertices: vec![],
            linedefs: vec![],
            sectors: vec![],

            shapefx_graphs: IndexMap::default(),
            sky_texture: None,

            camera: MapCamera::TwoD,
            camera_xz: None,
            look_at_xz: None,

            lights: vec![],
            entities: vec![],
            items: vec![],

            selected_vertices: vec![],
            selected_linedefs: vec![],
            selected_sectors: vec![],

            selected_entity_item: None,

            properties: ValueContainer::default(),
            softrigs: IndexMap::default(),
            editing_rig: None,
            soft_animator: None,

            surfaces: IndexMap::default(),
            profiles: FxHashMap::default(),
            shaders: IndexMap::default(),

            changed: 0,
        }
    }

    /// Clear temporary data
    pub fn clear_temp(&mut self) {
        self.possible_polygon = vec![];
        self.curr_grid_pos = None;
        self.curr_rectangle = None;
    }

    /// Clear the selection
    pub fn clear_selection(&mut self) {
        self.selected_vertices = vec![];
        self.selected_linedefs = vec![];
        self.selected_sectors = vec![];
        self.selected_entity_item = None;
    }

    /// Returns the surface for the given sector_id
    pub fn get_surface_for_sector_id(&self, sector_id: u32) -> Option<&Surface> {
        self.surfaces
            .values()
            .find(|surface| surface.sector_id == sector_id)
    }

    /// Returns the mutable surface for the given sector_id
    pub fn get_surface_for_sector_id_mut(&mut self, sector_id: u32) -> Option<&mut Surface> {
        self.surfaces
            .values_mut()
            .find(|surface| surface.sector_id == sector_id)
    }

    /// Updates the geometry of all surfaces
    pub fn update_surfaces(&mut self) {
        let mut surfaces = std::mem::take(&mut self.surfaces);
        for (_id, surface) in surfaces.iter_mut() {
            surface.calculate_geometry(self);
        }
        self.surfaces = surfaces;
    }

    /// Return the Map as MapMini
    pub fn as_mini(&self, blocking_tiles: &FxHashSet<Uuid>) -> MapMini {
        let mut linedefs: Vec<CompiledLinedef> = vec![];
        let mut occluded_sectors: Vec<(BBox, f32)> = vec![];

        let mut blocked_tiles = FxHashSet::default();

        for sector in self.sectors.iter() {
            let mut add_it = false;

            // We collect occluded sectors
            let occlusion = sector.properties.get_float_default("occlusion", 1.0);
            if occlusion < 1.0 {
                let mut bbox = sector.bounding_box(self);
                bbox.expand(Vec2::new(0.1, 0.1));
                occluded_sectors.push((bbox, occlusion));
            }

            if sector.layer.is_some() {
                let render_mode = sector.properties.contains("rect");
                if render_mode {
                    add_it = false;
                }
                // If the tile is explicitly set to blocking we have to add the geometry
                match sector.properties.get_default_source() {
                    Some(PixelSource::TileId(id)) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                            if let Some(center) = sector.center(self) {
                                blocked_tiles.insert(center.map(|c| (c.floor()) as i32));
                            }
                        }
                    }
                    Some(PixelSource::MaterialId(id)) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                        }
                    }
                    _ => {}
                }
            }

            if add_it {
                for linedef_id in sector.linedefs.iter() {
                    if let Some(linedef) = self.find_linedef(*linedef_id) {
                        if let Some(start) = self.find_vertex(linedef.start_vertex) {
                            if let Some(end) = self.find_vertex(linedef.end_vertex) {
                                let sy = start.as_vec3_world().y;
                                let ey = end.as_vec3_world().y;
                                if sy == 0.0 && ey == 0.0 {
                                    let cl = CompiledLinedef::new(
                                        start.as_vec2(),
                                        end.as_vec2(),
                                        linedef.properties.get_float_default("wall_width", 0.0),
                                        linedef.properties.get_float_default("wall_height", 0.0),
                                    );
                                    linedefs.push(cl);
                                }
                            }
                        }
                    }
                }
            }
        }

        for l in self.linedefs.iter() {
            if l.sector_ids.is_empty() {
                let wall_height = l.properties.get_float_default("wall_height", 0.0);
                let mut add_it = false;

                // If the tile is explicitly set to blocking we have to add the geometry
                match l.properties.get("source") {
                    Some(Value::Source(PixelSource::TileId(id))) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                        }
                    }
                    Some(Value::Source(PixelSource::MaterialId(id))) => {
                        if blocking_tiles.contains(id) {
                            add_it = true;
                        }
                    }
                    _ => {}
                }

                if add_it {
                    if let Some(start) = self.find_vertex(l.start_vertex) {
                        if let Some(end) = self.find_vertex(l.end_vertex) {
                            let sy = start.as_vec3_world().y;
                            let ey = end.as_vec3_world().y;
                            if sy == 0.0 && ey == 0.0 {
                                let cl = CompiledLinedef::new(
                                    start.as_vec2(),
                                    end.as_vec2(),
                                    l.properties.get_float_default("wall_width", 0.0),
                                    wall_height,
                                );
                                linedefs.push(cl);
                            }
                        }
                    }
                }
            }
        }

        let mut mini = MapMini::new(self.offset, self.grid_size, linedefs, occluded_sectors);
        mini.blocked_tiles = blocked_tiles;
        mini
    }

    /// Generate a bounding box for all vertices in the map
    pub fn bbox(&self) -> BBox {
        // Find min and max coordinates among all vertices
        let min_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);
        BBox {
            min: Vec2::new(min_x, min_y),
            max: Vec2::new(max_x, max_y),
        }
    }

    /// Generate a bounding box for all vertices in the map
    pub fn bounding_box(&self) -> Option<Vec4<f32>> {
        if self.vertices.is_empty() {
            return None; // No vertices in the map
        }

        // Find min and max coordinates among all vertices
        let min_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = self
            .vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::INFINITY, f32::min);
        let max_y = self
            .vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);

        // Calculate width and height
        let width = max_x - min_x;
        let height = max_y - min_y;

        // Return the bounding box as Vec4f (x, y, width, height)
        Some(Vec4::new(min_x, min_y, width, height))
    }

    /// Tick the soft animator.
    pub fn tick(&mut self, delta_time: f32) {
        if let Some(anim) = &mut self.soft_animator {
            anim.tick(delta_time);
        }
    }

    /// Get the current position of a vertex, using any keyform override in the current SoftRig.
    pub fn get_vertex(&self, vertex_id: u32) -> Option<Vec2<f32>> {
        // Base vertex lookup
        let base = self.vertices.iter().find(|v| v.id == vertex_id)?;
        let base_pos = Vec2::new(base.x, base.y);

        // 1. Try runtime animation
        if let Some(animator) = &self.soft_animator {
            if let Some(rig) = animator.get_blended_rig(self) {
                if let Some((_, pos)) = rig
                    .keyforms
                    .first()
                    .and_then(|key| key.vertex_positions.iter().find(|(id, _)| *id == vertex_id))
                {
                    return Some(*pos);
                }
            }
        }

        // 2. Try editing override (if not currently animating)
        if self.soft_animator.is_none() {
            if let Some(rig_id) = self.editing_rig {
                if let Some(rig) = self.softrigs.get(&rig_id) {
                    for keyform in &rig.keyforms {
                        if let Some((_, pos)) = keyform
                            .vertex_positions
                            .iter()
                            .find(|(id, _)| *id == vertex_id)
                        {
                            return Some(*pos);
                        }
                    }
                }
            }
        }

        // 3. Fallback to base
        Some(base_pos)
    }

    /// Get the current position of a vertex, using any keyform override in the current SoftRig.
    pub fn get_vertex_3d(&self, vertex_id: u32) -> Option<Vec3<f32>> {
        // Base vertex lookup
        let base = self.vertices.iter().find(|v| v.id == vertex_id)?;
        let base_pos = Vec3::new(base.x, base.z, base.y);

        // 1. Try runtime animation
        // if let Some(animator) = &self.soft_animator {
        //     if let Some(rig) = animator.get_blended_rig(self) {
        //         if let Some((_, pos)) = rig
        //             .keyforms
        //             .first()
        //             .and_then(|key| key.vertex_positions.iter().find(|(id, _)| *id == vertex_id))
        //         {
        //             return Some(*pos);
        //         }
        //     }
        // }

        // 2. Try editing override (if not currently animating)
        // if self.soft_animator.is_none() {
        //     if let Some(rig_id) = self.editing_rig {
        //         if let Some(rig) = self.softrigs.get(&rig_id) {
        //             for keyform in &rig.keyforms {
        //                 if let Some((_, pos)) = keyform
        //                     .vertex_positions
        //                     .iter()
        //                     .find(|(id, _)| *id == vertex_id)
        //                 {
        //                     return Some(*pos);
        //                 }
        //             }
        //         }
        //     }
        // }

        // 3. Fallback to base
        Some(base_pos)
    }

    /// Update the vertex position. If a keyform in the selected rig contains this vertex, update it.
    /// Otherwise, create a new keyform for this single vertex.
    pub fn update_vertex(&mut self, vertex_id: u32, new_position: Vec2<f32>) {
        // Update in active SoftRig
        if let Some(rig_id) = self.editing_rig {
            if let Some(rig) = self.softrigs.get_mut(&rig_id) {
                // Try to find a keyform that already contains this vertex
                for keyform in &mut rig.keyforms {
                    if let Some(entry) = keyform
                        .vertex_positions
                        .iter_mut()
                        .find(|(id, _)| *id == vertex_id)
                    {
                        entry.1 = new_position;
                        return;
                    }
                }

                // No existing keyform contains this vertex → create new keyform
                let new_keyform = Keyform {
                    vertex_positions: vec![(vertex_id, new_position)],
                };

                rig.keyforms.push(new_keyform);
                return;
            }
        }

        // Otherwise update base geometry
        if let Some(v) = self.vertices.iter_mut().find(|v| v.id == vertex_id) {
            v.x = new_position.x;
            v.y = new_position.y;
        }
    }

    // Add the vertex (and snap it to the subdivsion grid)
    pub fn add_vertex_at(&mut self, mut x: f32, mut y: f32) -> u32 {
        let subdivisions = 1.0 / self.subdivisions;

        x = (x / subdivisions).round() * subdivisions;
        y = (y / subdivisions).round() * subdivisions;

        // Check if the vertex already exists
        if let Some(id) = self.find_vertex_at(x, y) {
            return id;
        }

        if let Some(id) = self.find_free_vertex_id() {
            let vertex = Vertex::new(id, x, y);
            self.vertices.push(vertex);
            id
        } else {
            println!("No free vertex ID available");
            0
        }
    }

    /// Add a 3D vertex (x,y on the 2D grid; z is up).
    pub fn add_vertex_at_3d(&mut self, mut x: f32, mut y: f32, mut z: f32, snap: bool) -> u32 {
        // Snap X/Y using the same 2D grid/subdivision logic as add_vertex_at
        if snap {
            let subdivisions = 1.0 / self.subdivisions;
            x = (x / subdivisions).round() * subdivisions;
            y = (y / subdivisions).round() * subdivisions;
            z = (z / subdivisions).round() * subdivisions;
        }

        // Check if a vertex at (x,y,z) already exists
        if let Some(id) = self.find_vertex_at_3d(x, y, z) {
            return id;
        }

        // Allocate a new vertex id and insert
        if let Some(id) = self.find_free_vertex_id() {
            let vertex = Vertex::new_3d(id, x, y, z);
            self.vertices.push(vertex);
            id
        } else {
            println!("No free vertex ID available");
            0
        }
    }

    /// Finds a vertex exactly at (x,y,z) and returns its ID if it exists
    pub fn find_vertex_at_3d(&self, x: f32, y: f32, z: f32) -> Option<u32> {
        self.vertices
            .iter()
            .find(|v| v.x == x && v.y == y && v.z == z)
            .map(|v| v.id)
    }

    /// Finds a vertex at the given position and returns its ID if it exists
    pub fn find_vertex_at(&self, x: f32, y: f32) -> Option<u32> {
        self.vertices
            .iter()
            .find(|v| v.x == x && v.y == y)
            .map(|v| v.id)
    }

    /// Finds a reference to a vertex by its ID
    pub fn find_vertex(&self, id: u32) -> Option<&Vertex> {
        self.vertices.iter().find(|vertex| vertex.id == id)
    }

    /// Finds a mutable reference to a vertex by its ID
    pub fn find_vertex_mut(&mut self, id: u32) -> Option<&mut Vertex> {
        self.vertices.iter_mut().find(|vertex| vertex.id == id)
    }

    /// Finds a reference to a linedef by its ID
    pub fn find_linedef(&self, id: u32) -> Option<&Linedef> {
        self.linedefs.iter().find(|linedef| linedef.id == id)
    }

    /// Finds a reference to a linedef by its ID
    pub fn find_linedef_mut(&mut self, id: u32) -> Option<&mut Linedef> {
        self.linedefs.iter_mut().find(|linedef| linedef.id == id)
    }

    /// Finds a mutable reference to a sector by its ID
    pub fn find_sector(&self, id: u32) -> Option<&Sector> {
        self.sectors.iter().find(|sector| sector.id == id)
    }

    /// Finds a mutable reference to a sector by its ID
    pub fn find_sector_mut(&mut self, id: u32) -> Option<&mut Sector> {
        self.sectors.iter_mut().find(|sector| sector.id == id)
    }

    // Create a new (or use an existing) linedef for the given vertices and closes a polygon sector if it detects a loop.
    pub fn create_linedef(&mut self, start_vertex: u32, end_vertex: u32) -> (u32, Option<u32>) {
        let mut sector_id: Option<u32> = None;

        // Reuse an existing linedef only if it matches the requested winding direction exactly.
        if let Some(existing) = self
            .linedefs
            .iter()
            .find(|l| l.start_vertex == start_vertex && l.end_vertex == end_vertex)
        {
            let id = existing.id;
            if let Some(polygon) = self.find_directed_cycle_from_edge(id) {
                self.possible_polygon = polygon;
                sector_id = self.create_sector_from_polygon();
            }
            return (id, sector_id);
        }

        // Create a new linedef as before and try to close a sector.
        if let Some(id) = self.find_free_linedef_id() {
            let linedef = Linedef::new(id, start_vertex, end_vertex);
            self.linedefs.push(linedef);

            if let Some(polygon) = self.find_directed_cycle_from_edge(id) {
                self.possible_polygon = polygon;

                if let Some(sid) = self.create_sector_from_polygon() {
                    if let Some(linedef) = self.find_linedef_mut(id) {
                        // Assign the new sector on the freshly created edge if possible; the full
                        // assignment across the ring is handled inside create_sector_from_polygon.
                        if !linedef.sector_ids.contains(&sid) {
                            linedef.sector_ids.push(sid);
                        }
                    }
                    sector_id = Some(sid);
                }
            }
            (id, sector_id)
        } else {
            println!("No free linedef ID available");
            (0, None)
        }
    }
    // Create a new (or use an existing) linedef for the given vertices WITHOUT auto-creating sectors.
    // This is useful for manual polygon creation where the user is drawing a sequence of lines.
    // The linedef ID is added to possible_polygon for later manual sector creation.
    pub fn create_linedef_manual(&mut self, start_vertex: u32, end_vertex: u32) -> u32 {
        // Reuse an existing linedef if it matches the requested winding direction exactly.
        if let Some(existing) = self
            .linedefs
            .iter()
            .find(|l| l.start_vertex == start_vertex && l.end_vertex == end_vertex)
        {
            let id = existing.id;
            // Add to possible_polygon for manual tracking
            if !self.possible_polygon.contains(&id) {
                self.possible_polygon.push(id);
            }
            return id;
        }

        // Create a new linedef
        if let Some(id) = self.find_free_linedef_id() {
            let linedef = Linedef::new(id, start_vertex, end_vertex);
            self.linedefs.push(linedef);

            // Add to possible_polygon for manual tracking
            self.possible_polygon.push(id);
            id
        } else {
            println!("No free linedef ID available");
            0
        }
    }

    // Manually close the current polygon tracked in possible_polygon if it forms a closed loop.
    // Returns the sector ID if a sector was created, None otherwise.
    pub fn close_polygon_manual(&mut self) -> Option<u32> {
        if self.test_for_closed_polygon() {
            self.create_sector_from_polygon()
        } else {
            None
        }
    }

    // Check if a vertex is used by any sector with the "rect" property
    pub fn is_vertex_in_rect_sector(&self, vertex_id: u32) -> bool {
        self.sectors.iter().any(|sector| {
            if sector.properties.contains("rect") {
                sector.linedefs.iter().any(|&line_id| {
                    if let Some(line) = self.find_linedef(line_id) {
                        line.start_vertex == vertex_id || line.end_vertex == vertex_id
                    } else {
                        false
                    }
                })
            } else {
                false
            }
        })
    }

    // Duplicate a vertex at the same position and return the new vertex ID
    pub fn duplicate_vertex(&mut self, vertex_id: u32) -> Option<u32> {
        if let Some(vertex) = self.find_vertex(vertex_id) {
            let new_id = self.find_free_vertex_id()?;
            let mut new_vertex = vertex.clone();
            new_vertex.id = new_id;
            self.vertices.push(new_vertex);
            Some(new_id)
        } else {
            None
        }
    }

    // Replace a vertex in a sector's linedefs with a new vertex
    pub fn replace_vertex_in_sector(
        &mut self,
        sector_id: u32,
        old_vertex_id: u32,
        new_vertex_id: u32,
    ) {
        if let Some(sector) = self.find_sector(sector_id) {
            let linedef_ids: Vec<u32> = sector.linedefs.clone();
            for linedef_id in linedef_ids {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    // Check if this linedef is shared with other sectors
                    let is_shared = linedef.sector_ids.len() > 1
                        || self
                            .sectors
                            .iter()
                            .any(|s| s.id != sector_id && s.linedefs.contains(&linedef_id));

                    let needs_vertex_change = linedef.start_vertex == old_vertex_id
                        || linedef.end_vertex == old_vertex_id;

                    if is_shared && needs_vertex_change {
                        // Linedef is shared - create a new linedef for this sector
                        let new_start = if linedef.start_vertex == old_vertex_id {
                            new_vertex_id
                        } else {
                            linedef.start_vertex
                        };
                        let new_end = if linedef.end_vertex == old_vertex_id {
                            new_vertex_id
                        } else {
                            linedef.end_vertex
                        };

                        if let Some(new_linedef_id) = self.find_free_linedef_id() {
                            let mut new_linedef = linedef.clone();
                            new_linedef.id = new_linedef_id;
                            new_linedef.start_vertex = new_start;
                            new_linedef.end_vertex = new_end;
                            new_linedef.sector_ids = vec![sector_id];
                            self.linedefs.push(new_linedef);

                            // Remove sector_id from old linedef's sector_ids
                            if let Some(old_linedef) = self.find_linedef_mut(linedef_id) {
                                old_linedef.sector_ids.retain(|&sid| sid != sector_id);
                            }

                            // Replace the linedef in sector's linedefs list
                            if let Some(sector) = self.find_sector_mut(sector_id) {
                                if let Some(pos) =
                                    sector.linedefs.iter().position(|&id| id == linedef_id)
                                {
                                    sector.linedefs[pos] = new_linedef_id;
                                }
                            }
                        }
                    } else if needs_vertex_change {
                        // Linedef is not shared - modify it directly
                        if let Some(linedef) = self.find_linedef_mut(linedef_id) {
                            if linedef.start_vertex == old_vertex_id {
                                linedef.start_vertex = new_vertex_id;
                            }
                            if linedef.end_vertex == old_vertex_id {
                                linedef.end_vertex = new_vertex_id;
                            }
                        }
                    }
                }
            }
        }
    }
    /// Attempts to find a closed directed cycle that uses the provided linedef ID.
    /// The traversal walks forward along linedef winding to keep sector orientation deterministic.
    fn find_directed_cycle_from_edge(&self, edge_id: u32) -> Option<Vec<u32>> {
        let edge = self.find_linedef(edge_id)?;

        // We look for a directed path from the end of the new edge back to its start.
        let path = self.find_directed_path(edge.end_vertex, edge.start_vertex, edge_id)?;

        // A polygon needs at least three edges: path (>=1) + the new edge.
        if path.len() + 1 < 3 {
            return None;
        }

        let mut cycle = path;
        cycle.push(edge_id);
        Some(cycle)
    }

    /// Breadth-first search for a directed path from `from` to `to`, following linedef winding.
    /// The search is iterative (no recursion) and skips the provided `skip_edge` ID.
    fn find_directed_path(&self, from: u32, to: u32, skip_edge: u32) -> Option<Vec<u32>> {
        let mut queue = VecDeque::new();
        let mut visited = FxHashSet::default();
        let mut parent: FxHashMap<u32, (u32, u32)> = FxHashMap::default(); // vertex -> (prev_vertex, edge_id)

        queue.push_back(from);
        visited.insert(from);

        while let Some(v) = queue.pop_front() {
            // Collect all outgoing edges that respect winding (start == v)
            for edge in self.linedefs.iter().filter(|e| e.start_vertex == v) {
                if edge.id == skip_edge {
                    continue;
                }
                let next = edge.end_vertex;

                if visited.contains(&next) {
                    continue;
                }

                parent.insert(next, (v, edge.id));

                if next == to {
                    // Reconstruct edge list from `from` -> ... -> `to`
                    let mut path = Vec::new();
                    let mut current = to;
                    while let Some((prev_vertex, edge_id)) = parent.get(&current) {
                        path.push(*edge_id);
                        if *prev_vertex == from {
                            break;
                        }
                        current = *prev_vertex;
                    }
                    path.reverse();
                    return Some(path);
                }

                visited.insert(next);
                queue.push_back(next);
            }
        }

        None
    }

    /// Check if the `possible_polygon` forms a closed loop
    pub fn test_for_closed_polygon(&self) -> bool {
        if self.possible_polygon.len() < 3 {
            return false; // A polygon needs at least 3 edges
        }

        if let Some(first_linedef) = self.find_linedef(self.possible_polygon[0]) {
            if let Some(last_linedef) =
                self.find_linedef(self.possible_polygon[self.possible_polygon.len() - 1])
            {
                // Check if the last linedef's end_vertex matches the first linedef's start_vertex
                return last_linedef.end_vertex == first_linedef.start_vertex;
            }
        }
        false
    }

    /// Tries to create a polyon from the tracked vertices in possible_polygon
    pub fn create_sector_from_polygon(&mut self) -> Option<u32> {
        if !self.test_for_closed_polygon() {
            // println!("Polygon is not closed. Cannot create sector.");
            return None;
        }

        // Check for duplicate sector
        if self
            .find_sector_by_linedefs(&self.possible_polygon)
            .is_some()
        {
            // println!(
            //     "Polygon already exists",
            // );
            self.possible_polygon.clear();
            return None;
        }

        // Create a new sector
        if let Some(sector_id) = self.find_free_sector_id() {
            for &id in &self.possible_polygon {
                if let Some(linedef) = self.linedefs.iter_mut().find(|l| l.id == id) {
                    // Add sector to the sector_ids list
                    if !linedef.sector_ids.contains(&sector_id) {
                        linedef.sector_ids.push(sector_id);
                    }
                }
            }

            let sector = Sector::new(sector_id, self.possible_polygon.clone());
            self.sectors.push(sector);

            self.possible_polygon.clear();
            Some(sector_id)
        } else {
            None
        }
    }

    /// Check if a set of linedefs matches any existing sector
    fn find_sector_by_linedefs(&self, linedefs: &[u32]) -> Option<u32> {
        for sector in &self.sectors {
            if sector.linedefs.len() == linedefs.len()
                && sector.linedefs.iter().all(|id| linedefs.contains(id))
            {
                return Some(sector.id);
            }
        }
        None
    }

    /// Deletes the specified vertices, linedefs, and sectors.
    pub fn delete_elements(&mut self, vertex_ids: &[u32], linedef_ids: &[u32], sector_ids: &[u32]) {
        let mut all_linedef_ids = linedef_ids.to_vec();
        let all_vertex_ids = vertex_ids.to_vec();

        // Step 1: Collect linedefs (and eventually vertices) from sectors to delete
        if !sector_ids.is_empty() {
            for sector in &self.sectors {
                if sector_ids.contains(&sector.id) {
                    for &linedef_id in &sector.linedefs {
                        // Check if this linedef is used by other sectors (that are not also being deleted)
                        // We check both: sector.linedefs lists AND linedef.sector_ids
                        let used_in_sector_linedefs = self.sectors.iter().any(|s| {
                            s.id != sector.id
                                && !sector_ids.contains(&s.id)
                                && s.linedefs.contains(&linedef_id)
                        });

                        let used_in_linedef_sector_ids =
                            if let Some(linedef) = self.find_linedef(linedef_id) {
                                linedef
                                    .sector_ids
                                    .iter()
                                    .any(|&sid| sid != sector.id && !sector_ids.contains(&sid))
                            } else {
                                false
                            };

                        let used_elsewhere = used_in_sector_linedefs || used_in_linedef_sector_ids;

                        if !used_elsewhere && !all_linedef_ids.contains(&linedef_id) {
                            all_linedef_ids.push(linedef_id);
                        }
                    }
                }
            }
        }
        /*
        // Do not delete vertices from deleted linedefs / sectors, only the selected ones.
        // Step 2: Collect vertices used *only* by linedefs being deleted
        for &linedef_id in &all_linedef_ids {
            if let Some(linedef) = self.find_linedef(linedef_id) {
                for &vertex_id in &[linedef.start_vertex, linedef.end_vertex] {
                    let used_elsewhere = self.linedefs.iter().any(|l| {
                        l.id != linedef_id
                            && (l.start_vertex == vertex_id || l.end_vertex == vertex_id)
                            && !all_linedef_ids.contains(&l.id)
                    });

                    if !used_elsewhere && !all_vertex_ids.contains(&vertex_id) {
                        all_vertex_ids.push(vertex_id);
                    }
                }
            }
        }*/

        // Step 3: Delete sectors
        if !sector_ids.is_empty() {
            self.sectors
                .retain(|sector| !sector_ids.contains(&sector.id));

            // Collect surfaces and remove them
            let mut surfaces_to_remove: Vec<uuid::Uuid> = Vec::new();
            for (surf_id, surf) in self.surfaces.iter() {
                if sector_ids.contains(&surf.sector_id) {
                    if let Some(profile_id) = surf.profile {
                        self.profiles.remove(&profile_id);
                    }
                    surfaces_to_remove.push(*surf_id);
                }
            }

            for sid in surfaces_to_remove {
                let _ = self.surfaces.shift_remove(&sid);
            }

            // --

            for linedef in &mut self.linedefs {
                // Remove deleted sectors from sector_ids list
                linedef.sector_ids.retain(|sid| !sector_ids.contains(sid));
            }
        }

        // Step 3.5: Before deleting vertices, collect linedefs that reference them
        for &vertex_id in &all_vertex_ids {
            for linedef in &self.linedefs {
                if (linedef.start_vertex == vertex_id || linedef.end_vertex == vertex_id)
                    && !all_linedef_ids.contains(&linedef.id)
                {
                    all_linedef_ids.push(linedef.id);
                }
            }
        }

        // Step 4: Delete linedefs
        if !all_linedef_ids.is_empty() {
            self.linedefs
                .retain(|linedef| !all_linedef_ids.contains(&linedef.id));
        }

        self.cleanup_sectors();

        // Step 5: Delete vertices (only if explicitly requested or now truly orphaned)
        if !all_vertex_ids.is_empty() {
            self.vertices
                .retain(|vertex| !all_vertex_ids.contains(&vertex.id));
        }
        // Step 6: Sanitize to ensure consistency (rebuild surfaces, clean up any remaining issues)
        self.sanitize();
    }

    /// Cleans up sectors to ensure no references to deleted linedefs remain.
    fn cleanup_sectors(&mut self) {
        let valid_linedef_ids: std::collections::HashSet<u32> =
            self.linedefs.iter().map(|l| l.id).collect();

        for sector in &mut self.sectors {
            sector
                .linedefs
                .retain(|linedef_id| valid_linedef_ids.contains(linedef_id));
        }

        // Remove empty sectors
        self.sectors.retain(|sector| !sector.linedefs.is_empty());
    }

    /// Check if a given linedef ID is part of any sector.
    pub fn is_linedef_in_closed_polygon(&self, linedef_id: u32) -> bool {
        self.sectors
            .iter()
            .any(|sector| sector.linedefs.contains(&linedef_id))
    }

    /// Add the given geometry to the selection.
    pub fn add_to_selection(&mut self, vertices: Vec<u32>, linedefs: Vec<u32>, sectors: Vec<u32>) {
        for v in &vertices {
            if !self.selected_vertices.contains(v) {
                self.selected_vertices.push(*v);
            }
        }
        for l in &linedefs {
            if !self.selected_linedefs.contains(l) {
                self.selected_linedefs.push(*l);
            }
        }
        for s in &sectors {
            if !self.selected_sectors.contains(s) {
                self.selected_sectors.push(*s);
            }
        }
    }

    /// Remove the given geometry from the selection.
    pub fn remove_from_selection(
        &mut self,
        vertices: Vec<u32>,
        linedefs: Vec<u32>,
        sectors: Vec<u32>,
    ) {
        for v in &vertices {
            self.selected_vertices.retain(|&selected| selected != *v);
        }
        for l in &linedefs {
            self.selected_linedefs.retain(|&selected| selected != *l);
        }
        for s in &sectors {
            self.selected_sectors.retain(|&selected| selected != *s);
        }
    }

    /// Returns the sectors sorted from largest to smallest by area
    pub fn sorted_sectors_by_area(&self) -> Vec<&Sector> {
        let mut sectors_with_areas: Vec<(&Sector, f32)> = self
            .sectors
            .iter()
            .map(|sector| (sector, sector.area(self))) // Calculate the area for each sector
            .collect();

        // Sort by area in descending order
        sectors_with_areas
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return the sorted sectors
        sectors_with_areas
            .into_iter()
            .map(|(sector, _)| sector)
            .collect()
    }

    /// Adds a midpoint to a specified linedef, updates the geometry, and returns the new vertex ID.
    pub fn add_midpoint(&mut self, linedef_id: u32) -> Option<u32> {
        // Step 1: Find the linedef
        let linedef = self.find_linedef(linedef_id)?.clone(); // Clone to avoid borrow issues
        let start_vertex = self.find_vertex(linedef.start_vertex)?.clone();
        let end_vertex = self.find_vertex(linedef.end_vertex)?.clone();

        // Step 2: Calculate the midpoint
        let midpoint = Vec3::new(
            (start_vertex.x + end_vertex.x) / 2.0,
            (start_vertex.y + end_vertex.y) / 2.0,
            (start_vertex.z + end_vertex.z) / 2.0,
        );

        // Step 3: Add the midpoint as a new vertex
        let new_vertex_id = self.add_vertex_at_3d(midpoint.x, midpoint.y, midpoint.z, false);

        // Step 4: Create new linedefs
        let mut new_linedef_1 = Linedef::new(
            linedef_id, // Use the same ID as the old linedef for the first new linedef
            linedef.start_vertex,
            new_vertex_id,
        );
        let mut new_linedef_2 = Linedef::new(
            self.linedefs.len() as u32, // New unique ID for the second linedef
            new_vertex_id,
            linedef.end_vertex,
        );

        // Assign the old properties of the linedef to the two new ones.
        new_linedef_1.properties = linedef.properties.clone();
        new_linedef_2.properties = linedef.properties.clone();

        // Step 5: Replace the old linedef in all sectors
        for sector in self.sectors.iter_mut() {
            if let Some(position) = sector.linedefs.iter().position(|&id| id == linedef_id) {
                // Replace the old linedef with the new ones in the correct order
                sector.linedefs.splice(
                    position..=position, // Replace the single linedef
                    [new_linedef_1.id, new_linedef_2.id].iter().cloned(), // Insert the new linedefs
                );
            }
        }

        // Step 6: Update the global linedef list
        if let Some(index) = self.linedefs.iter().position(|l| l.id == linedef_id) {
            self.linedefs[index] = new_linedef_1; // Replace the old linedef with the first new one
        }
        self.linedefs.push(new_linedef_2); // Add the second new linedef at the end

        // Return the ID of the new vertex
        Some(new_vertex_id)
    }

    /// Find sectors which consist of exactly the same 4 vertices and return them.
    /// This is used for stacking tiles / layering via the RECT tool.
    pub fn find_sectors_with_vertex_indices(&self, vertex_indices: &[u32; 4]) -> Vec<u32> {
        let mut matching_sectors = Vec::new();

        let mut new_vertex_set = vertex_indices.to_vec();
        new_vertex_set.sort();

        for sector in &self.sectors {
            let mut sector_vertex_indices = Vec::new();

            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    sector_vertex_indices.push(linedef.start_vertex);
                    sector_vertex_indices.push(linedef.end_vertex);
                }
            }

            // Deduplicate and sort for consistent comparison
            sector_vertex_indices.sort();
            sector_vertex_indices.dedup();

            // If the sector contains exactly these 4 vertices, it's a match
            if sector_vertex_indices == new_vertex_set {
                matching_sectors.push(sector.id);
            }
        }

        matching_sectors
    }

    /// Returns the sector at the given position (if any).
    pub fn find_sector_at(&self, position: Vec2<f32>) -> Option<&Sector> {
        self.sectors
            .iter()
            .find(|s| s.is_inside(self, position) && s.layer.is_none())
    }

    /// Debug: Print all vertices with their current animated positions
    pub fn debug_print_vertices(&self) {
        for vertex in &self.vertices {
            let current_position = self
                .get_vertex(vertex.id)
                .unwrap_or(Vec2::new(vertex.x, vertex.y));
            println!(
                "Vertex ID: {}, Base: ({}, {}), Animated: ({}, {})",
                vertex.id, vertex.x, vertex.y, current_position.x, current_position.y
            );
        }
    }

    /// Returns information about the Map
    pub fn info(&self) -> String {
        format!(
            "V {}, L {}, S {}",
            self.vertices.len(),
            self.linedefs.len(),
            self.sectors.len()
        )
    }

    /// Sanitizes and associates linedefs with their sectors by populating the sector_ids vector.
    /// Removes orphaned linedefs that reference non-existent vertices.
    /// This should be called after loading a map or when sectors are modified.
    pub fn sanitize(&mut self) {
        // First, sanitize: remove linedefs that reference non-existent vertices
        let valid_vertex_ids: std::collections::HashSet<u32> =
            self.vertices.iter().map(|v| v.id).collect();

        let mut orphaned_linedef_ids = Vec::new();

        for linedef in &self.linedefs {
            if !valid_vertex_ids.contains(&linedef.start_vertex)
                || !valid_vertex_ids.contains(&linedef.end_vertex)
            {
                println!(
                    "Sanitizing: removing orphaned linedef {} (references vertices {} -> {})",
                    linedef.id, linedef.start_vertex, linedef.end_vertex
                );
                orphaned_linedef_ids.push(linedef.id);
            }
        }

        if !orphaned_linedef_ids.is_empty() {
            self.linedefs
                .retain(|linedef| !orphaned_linedef_ids.contains(&linedef.id));

            // Collect sectors before cleanup to remove their surfaces
            let sectors_before: std::collections::HashSet<u32> =
                self.sectors.iter().map(|s| s.id).collect();

            // Clean up sectors that reference these linedefs (removes invalid refs and empty sectors)
            self.cleanup_sectors();

            // Find which sectors were removed
            let sectors_after: std::collections::HashSet<u32> =
                self.sectors.iter().map(|s| s.id).collect();
            let removed_sector_ids: Vec<u32> =
                sectors_before.difference(&sectors_after).copied().collect();

            // Remove surfaces for deleted sectors
            if !removed_sector_ids.is_empty() {
                let mut surfaces_to_remove: Vec<uuid::Uuid> = Vec::new();
                for (surf_id, surf) in self.surfaces.iter() {
                    if removed_sector_ids.contains(&surf.sector_id) {
                        if let Some(profile_id) = surf.profile {
                            self.profiles.remove(&profile_id);
                        }
                        surfaces_to_remove.push(*surf_id);
                    }
                }

                for sid in surfaces_to_remove {
                    let _ = self.surfaces.shift_remove(&sid);
                }

                println!(
                    "Sanitized: removed {} sector(s) and their surfaces",
                    removed_sector_ids.len()
                );
            }

            println!(
                "Sanitized: removed {} orphaned linedef(s)",
                orphaned_linedef_ids.len()
            );
        }

        // Additional validation: check for sectors with invalid geometry
        let mut invalid_sectors = Vec::new();
        for sector in &self.sectors {
            // Check if sector has enough linedefs to form a polygon
            if sector.linedefs.len() < 3 {
                println!(
                    "Sanitizing: sector {} has only {} linedef(s), need at least 3",
                    sector.id,
                    sector.linedefs.len()
                );
                invalid_sectors.push(sector.id);
                continue;
            }

            // Check if all linedefs in the sector reference valid vertices
            let mut has_invalid_linedef = false;
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    if self.find_vertex(linedef.start_vertex).is_none()
                        || self.find_vertex(linedef.end_vertex).is_none()
                    {
                        println!(
                            "Sanitizing: sector {} has linedef {} with invalid vertices",
                            sector.id, linedef_id
                        );
                        has_invalid_linedef = true;
                        break;
                    }
                } else {
                    println!(
                        "Sanitizing: sector {} references non-existent linedef {}",
                        sector.id, linedef_id
                    );
                    has_invalid_linedef = true;
                    break;
                }
            }
            if has_invalid_linedef {
                invalid_sectors.push(sector.id);
                continue;
            }

            // Check if the sector forms a closed loop
            if let Some(first_linedef) = self.find_linedef(sector.linedefs[0]) {
                if let Some(last_linedef) =
                    self.find_linedef(sector.linedefs[sector.linedefs.len() - 1])
                {
                    if last_linedef.end_vertex != first_linedef.start_vertex {
                        println!(
                            "Sanitizing: sector {} does not form a closed loop (last vertex {} != first vertex {})",
                            sector.id, last_linedef.end_vertex, first_linedef.start_vertex
                        );
                        invalid_sectors.push(sector.id);
                        continue;
                    }
                }
            }

            // Check for consecutive duplicate vertices (zero-length edges)
            let mut has_zero_length = false;
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    if linedef.start_vertex == linedef.end_vertex {
                        println!(
                            "Sanitizing: sector {} has linedef {} with same start and end vertex {}",
                            sector.id, linedef_id, linedef.start_vertex
                        );
                        has_zero_length = true;
                        break;
                    }
                }
            }
            if has_zero_length {
                invalid_sectors.push(sector.id);
                continue;
            }

            // Check for NaN or infinite vertex coordinates
            let mut has_invalid_coords = false;
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    if let Some(start_v) = self.find_vertex(linedef.start_vertex) {
                        if !start_v.x.is_finite()
                            || !start_v.y.is_finite()
                            || !start_v.z.is_finite()
                        {
                            println!(
                                "Sanitizing: sector {} has vertex {} with invalid coordinates: ({}, {}, {})",
                                sector.id, linedef.start_vertex, start_v.x, start_v.y, start_v.z
                            );
                            has_invalid_coords = true;
                            break;
                        }
                    }
                    if let Some(end_v) = self.find_vertex(linedef.end_vertex) {
                        if !end_v.x.is_finite() || !end_v.y.is_finite() || !end_v.z.is_finite() {
                            println!(
                                "Sanitizing: sector {} has vertex {} with invalid coordinates: ({}, {}, {})",
                                sector.id, linedef.end_vertex, end_v.x, end_v.y, end_v.z
                            );
                            has_invalid_coords = true;
                            break;
                        }
                    }
                }
            }
            if has_invalid_coords {
                invalid_sectors.push(sector.id);
            }
        }

        // Remove invalid sectors
        if !invalid_sectors.is_empty() {
            // Remove surfaces for these sectors
            let mut surfaces_to_remove: Vec<uuid::Uuid> = Vec::new();
            for (surf_id, surf) in self.surfaces.iter() {
                if invalid_sectors.contains(&surf.sector_id) {
                    if let Some(profile_id) = surf.profile {
                        self.profiles.remove(&profile_id);
                    }
                    surfaces_to_remove.push(*surf_id);
                }
            }

            for sid in surfaces_to_remove {
                let _ = self.surfaces.shift_remove(&sid);
            }

            self.sectors.retain(|s| !invalid_sectors.contains(&s.id));

            println!(
                "Sanitized: removed {} invalid sector(s)",
                invalid_sectors.len()
            );
        }

        // Rebuild surfaces for remaining sectors and check for invalid transforms
        self.update_surfaces();

        let mut invalid_surface_sectors = Vec::new();
        for (_surface_id, surface) in &self.surfaces {
            if !surface.is_valid() {
                println!(
                    "Sanitizing: sector {} has surface with invalid transform (NaN/Inf)",
                    surface.sector_id
                );
                invalid_surface_sectors.push(surface.sector_id);
            }
        }

        // Remove sectors with invalid surfaces
        if !invalid_surface_sectors.is_empty() {
            // Remove the invalid surfaces and profiles
            let mut surfaces_to_remove: Vec<uuid::Uuid> = Vec::new();
            for (surf_id, surf) in self.surfaces.iter() {
                if invalid_surface_sectors.contains(&surf.sector_id) {
                    if let Some(profile_id) = surf.profile {
                        self.profiles.remove(&profile_id);
                    }
                    surfaces_to_remove.push(*surf_id);
                }
            }

            for sid in surfaces_to_remove {
                let _ = self.surfaces.shift_remove(&sid);
            }

            // Remove the sectors themselves
            self.sectors
                .retain(|s| !invalid_surface_sectors.contains(&s.id));

            println!(
                "Sanitized: removed {} sector(s) with invalid surfaces",
                invalid_surface_sectors.len()
            );
        }

        // Now, clear all existing sector_ids
        for linedef in &mut self.linedefs {
            linedef.sector_ids.clear();
        }

        // Collect all linedef-to-sector associations
        let mut associations: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
        for sector in &self.sectors {
            let sector_id = sector.id;
            for &linedef_id in &sector.linedefs {
                associations
                    .entry(linedef_id)
                    .or_insert_with(Vec::new)
                    .push(sector_id);
            }
        }

        // Apply the associations to linedefs
        for (linedef_id, sector_ids) in associations {
            if let Some(linedef) = self.linedefs.iter_mut().find(|l| l.id == linedef_id) {
                linedef.sector_ids = sector_ids;
            }
        }
    }

    /// Alias for sanitize() to maintain backward compatibility.
    pub fn associate_linedefs_with_sectors(&mut self) {
        self.sanitize();
    }

    // /// Returns true if the given vertex is part of a sector with rect rendering enabled.
    // pub fn is_vertex_in_rect(&self, vertex_id: u32) -> bool {
    //     for sector in &self.sectors {
    //         if sector.layer.is_none() {
    //             continue;
    //         }
    //         for linedef_id in sector.linedefs.iter() {
    //             if let Some(linedef) = self.find_linedef(*linedef_id) {
    //                 if linedef.start_vertex == vertex_id || linedef.end_vertex == vertex_id {
    //                     return true;
    //                 }
    //             }
    //         }
    //     }
    //     false
    // }

    /// Returns true if the given linedef is part of a sector with rect rendering enabled.
    pub fn is_linedef_in_rect(&self, linedef_id: u32) -> bool {
        for sector in &self.sectors {
            if sector.layer.is_none() {
                continue;
            }

            if sector.linedefs.contains(&linedef_id) {
                return true;
            }
        }
        false
    }

    /// Finds a free vertex ID that can be used for creating a new vertex.
    pub fn find_free_vertex_id(&self) -> Option<u32> {
        (0..).find(|&id| !self.vertices.iter().any(|v| v.id == id))
    }

    /// Finds a free linedef ID that can be used for creating a new linedef.
    pub fn find_free_linedef_id(&self) -> Option<u32> {
        (0..).find(|&id| !self.linedefs.iter().any(|l| l.id == id))
    }

    /// Finds a free sector ID that can be used for creating a new sector.
    pub fn find_free_sector_id(&self) -> Option<u32> {
        (0..).find(|&id| !self.sectors.iter().any(|s| s.id == id))
    }

    /// Check if the map has selected geometry.
    pub fn has_selection(&self) -> bool {
        !self.selected_vertices.is_empty()
            || !self.selected_linedefs.is_empty()
            || !self.selected_sectors.is_empty()
    }

    /// Check if the map is empty.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.linedefs.is_empty() && self.sectors.is_empty()
    }

    /// Copy selected geometry into a new map
    pub fn copy_selected(&mut self, cut: bool) -> Map {
        let mut clipboard = Map::new();

        let mut old_to_new_vertex: FxHashMap<u32, u32> = FxHashMap::default();
        let mut old_to_new_linedef: FxHashMap<u32, u32> = FxHashMap::default();
        // let mut old_to_new_sector: FxHashMap<u32, u32> = FxHashMap::default();

        let mut vertex_ids: FxHashSet<u32> = FxHashSet::default();
        let mut linedef_ids: FxHashSet<u32> = self.selected_linedefs.iter().copied().collect();
        let sector_ids: FxHashSet<u32> = self.selected_sectors.iter().copied().collect();

        // Add linedefs from selected sectors
        for sid in &sector_ids {
            if let Some(sector) = self.find_sector(*sid) {
                for &lid in &sector.linedefs {
                    linedef_ids.insert(lid);
                }
            }
        }

        // Add vertices from selected linedefs
        for lid in &linedef_ids {
            if let Some(ld) = self.find_linedef(*lid) {
                vertex_ids.insert(ld.start_vertex);
                vertex_ids.insert(ld.end_vertex);
            }
        }

        // Add standalone selected vertices
        for &vid in &self.selected_vertices {
            vertex_ids.insert(vid);
        }

        // Normalize vertex positions
        let copied_vertices: Vec<Vertex> = vertex_ids
            .iter()
            .filter_map(|id| self.find_vertex(*id).cloned())
            .collect();

        if copied_vertices.is_empty() {
            return clipboard;
        }

        let min_x = copied_vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::INFINITY, f32::min);
        let min_y = copied_vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::INFINITY, f32::min);
        let offset = Vec2::new(min_x, min_y);

        // Remap and store vertices
        for old in copied_vertices {
            if let Some(new_id) = clipboard.find_free_vertex_id() {
                let mut new_v = old.clone();
                new_v.id = new_id;
                new_v.x -= offset.x;
                new_v.y -= offset.y;
                old_to_new_vertex.insert(old.id, new_id);
                clipboard.vertices.push(new_v);
            }
        }

        // Remap and store linedefs
        for old_id in &linedef_ids {
            if let Some(ld) = self.find_linedef(*old_id).cloned() {
                if let Some(new_id) = clipboard.find_free_linedef_id() {
                    let mut new_ld = ld.clone();
                    new_ld.id = new_id;
                    new_ld.start_vertex = *old_to_new_vertex.get(&ld.start_vertex).unwrap();
                    new_ld.end_vertex = *old_to_new_vertex.get(&ld.end_vertex).unwrap();
                    new_ld.sector_ids.clear();
                    old_to_new_linedef.insert(ld.id, new_id);
                    clipboard.linedefs.push(new_ld);
                }
            }
        }

        // Remap and store sectors (only those whose linedefs were copied)
        for sid in &sector_ids {
            if let Some(s) = self.find_sector(*sid).cloned() {
                if s.linedefs.iter().all(|id| linedef_ids.contains(id)) {
                    if let Some(new_id) = clipboard.find_free_sector_id() {
                        let mut new_s = s.clone();
                        new_s.id = new_id;
                        new_s.linedefs = s
                            .linedefs
                            .iter()
                            .map(|id| *old_to_new_linedef.get(id).unwrap())
                            .collect();

                        // Update sector_ids in the linedefs
                        for &old_lid in &s.linedefs {
                            if let Some(&new_lid) = old_to_new_linedef.get(&old_lid) {
                                if let Some(ld) =
                                    clipboard.linedefs.iter_mut().find(|l| l.id == new_lid)
                                {
                                    if !ld.sector_ids.contains(&new_id) {
                                        ld.sector_ids.push(new_id);
                                    }
                                }
                            }
                        }

                        clipboard.sectors.push(new_s);
                    }
                }
            }
        }

        // Delete source geometry if cutting
        if cut {
            self.delete_elements(
                &vertex_ids.iter().copied().collect::<Vec<_>>(),
                &linedef_ids.iter().copied().collect::<Vec<_>>(),
                &sector_ids.iter().copied().collect::<Vec<_>>(),
            );
            self.clear_selection();
        }

        clipboard
    }

    /// Inserts the given map at the given position.
    pub fn paste_at_position(&mut self, local_map: &Map, position: Vec2<f32>) {
        let mut vertex_map = FxHashMap::default();
        let mut linedef_map = FxHashMap::default();

        self.clear_selection();

        // Vertices
        for v in &local_map.vertices {
            if let Some(new_id) = self.find_free_vertex_id() {
                let mut new_v = v.clone();
                new_v.id = new_id;
                new_v.x += position.x;
                new_v.y += position.y;
                self.vertices.push(new_v);
                self.selected_vertices.push(new_id);
                vertex_map.insert(v.id, new_id);
            }
        }

        // Linedefs
        for l in &local_map.linedefs {
            if let Some(new_id) = self.find_free_linedef_id() {
                let mut new_l = l.clone();
                new_l.id = new_id;
                new_l.start_vertex = *vertex_map.get(&l.start_vertex).unwrap();
                new_l.end_vertex = *vertex_map.get(&l.end_vertex).unwrap();
                // Reset front/back sector and sector_ids
                new_l.sector_ids.clear();
                self.linedefs.push(new_l);
                self.selected_linedefs.push(new_id);
                linedef_map.insert(l.id, new_id);
            }
        }

        // Sectors
        for s in &local_map.sectors {
            if let Some(new_id) = self.find_free_sector_id() {
                let mut new_s = s.clone();
                new_s.id = new_id;
                new_s.linedefs = s
                    .linedefs
                    .iter()
                    .map(|id| *linedef_map.get(id).unwrap())
                    .collect();

                // Assign sector to each of its linedefs
                for old_lid in &s.linedefs {
                    if let Some(&new_lid) = linedef_map.get(old_lid) {
                        if let Some(ld) = self.linedefs.iter_mut().find(|l| l.id == new_lid) {
                            // Add sector to sector_ids list
                            if !ld.sector_ids.contains(&new_id) {
                                ld.sector_ids.push(new_id);
                            }
                        }
                    }
                }

                self.sectors.push(new_s);
                self.selected_sectors.push(new_id);
            }
        }
    }

    /// Creates a geometry_clone clone of the map containing only vertices, linedefs, and sectors.
    pub fn geometry_clone(&self) -> Map {
        Map {
            id: Uuid::new_v4(),
            name: format!("{} (geometry_clone)", self.name),

            offset: self.offset,
            grid_size: self.grid_size,
            subdivisions: self.subdivisions,

            terrain: Terrain::default(),

            possible_polygon: vec![],
            curr_grid_pos: None,
            curr_mouse_pos: None,
            curr_rectangle: None,

            vertices: self.vertices.clone(),
            linedefs: self.linedefs.clone(),
            sectors: self.sectors.clone(),

            shapefx_graphs: self.shapefx_graphs.clone(),
            sky_texture: None,

            camera: self.camera,
            camera_xz: None,
            look_at_xz: None,

            lights: vec![],
            entities: vec![],
            items: vec![],

            selected_vertices: vec![],
            selected_linedefs: vec![],
            selected_sectors: vec![],

            selected_entity_item: None,

            properties: ValueContainer::default(),
            softrigs: IndexMap::default(),
            editing_rig: None,
            soft_animator: None,

            surfaces: IndexMap::default(),
            profiles: FxHashMap::default(),
            shaders: IndexMap::default(),

            changed: 0,
        }
    }

    /// Extracts all geometry into a new Map which intersects with the given chunk bbox.
    pub fn extract_chunk_geometry(&self, bbox: BBox) -> Map {
        let mut result = Map::new();

        let mut vertex_map: FxHashMap<u32, u32> = FxHashMap::default();
        let mut linedef_map: FxHashMap<u32, u32> = FxHashMap::default();

        // Step 1: Find all linedefs that intersect the BBox
        for l in &self.linedefs {
            if let (Some(start), Some(end)) = (
                self.get_vertex(l.start_vertex),
                self.get_vertex(l.end_vertex),
            ) {
                // Check if either endpoint is inside or the segment intersects bbox
                if bbox.contains(start) || bbox.contains(end) || bbox.line_intersects(start, end) {
                    let new_id = result.find_free_linedef_id().unwrap_or(l.id);
                    let mut l_clone = l.clone();
                    l_clone.id = new_id;
                    l_clone.sector_ids.clear();
                    result.linedefs.push(l_clone);
                    linedef_map.insert(l.id, new_id);

                    // Ensure both vertices are marked for inclusion
                    for vid in &[l.start_vertex, l.end_vertex] {
                        if !vertex_map.contains_key(vid) {
                            if let Some(v) = self.find_vertex(*vid) {
                                let new_vid = result.find_free_vertex_id().unwrap_or(v.id);
                                let mut v_clone = v.clone();
                                v_clone.id = new_vid;
                                result.vertices.push(v_clone);
                                vertex_map.insert(*vid, new_vid);
                            }
                        }
                    }

                    // Reassign the vertex IDs
                    if let Some(ld) = result.linedefs.last_mut() {
                        ld.start_vertex = vertex_map[&l.start_vertex];
                        ld.end_vertex = vertex_map[&l.end_vertex];
                    }
                }
            }
        }

        // Step 2: Add sectors that reference any included linedef
        for s in &self.sectors {
            if s.linedefs.iter().any(|lid| linedef_map.contains_key(lid)) {
                let new_id = result.find_free_sector_id().unwrap_or(s.id);
                let mut s_clone = s.clone();
                s_clone.id = new_id;
                s_clone.linedefs = s
                    .linedefs
                    .iter()
                    .filter_map(|lid| linedef_map.get(lid).copied())
                    .collect();

                // Re-link sector ID into included linedefs
                for lid in &s.linedefs {
                    if let Some(&new_lid) = linedef_map.get(lid) {
                        if let Some(ld) = result.linedefs.iter_mut().find(|l| l.id == new_lid) {
                            // Add sector to sector_ids list
                            if !ld.sector_ids.contains(&new_id) {
                                ld.sector_ids.push(new_id);
                            }
                        }
                    }
                }

                result.sectors.push(s_clone);
            }
        }

        result
    }

    // Check if a point is inside a sector (using ray casting algorithm)
    fn is_point_in_sector(&self, point: Vec2<f32>, sector_id: u32) -> bool {
        if let Some(sector) = self.find_sector(sector_id) {
            let mut vertices = Vec::new();
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    if let Some(vertex) = self.find_vertex(linedef.start_vertex) {
                        vertices.push(Vec2::new(vertex.x, vertex.y));
                    }
                }
            }

            // Ray casting algorithm
            let mut inside = false;
            let mut j = vertices.len() - 1;
            for i in 0..vertices.len() {
                if ((vertices[i].y > point.y) != (vertices[j].y > point.y))
                    && (point.x
                        < (vertices[j].x - vertices[i].x) * (point.y - vertices[i].y)
                            / (vertices[j].y - vertices[i].y)
                            + vertices[i].x)
                {
                    inside = !inside;
                }
                j = i;
            }
            inside
        } else {
            false
        }
    }

    // Find all sectors that are completely embedded within a given sector
    pub fn find_embedded_sectors(&self, container_sector_id: u32) -> Vec<u32> {
        let mut embedded = Vec::new();

        for sector in &self.sectors {
            if sector.id == container_sector_id {
                continue; // Skip the container itself
            }

            if sector.linedefs.is_empty() {
                continue;
            }

            // Collect all unique vertices from the sector's linedefs
            let mut vertices = Vec::new();
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = self.find_linedef(linedef_id) {
                    if let Some(vertex) = self.find_vertex(linedef.start_vertex) {
                        vertices.push(Vec2::new(vertex.x, vertex.y));
                    }
                    if let Some(vertex) = self.find_vertex(linedef.end_vertex) {
                        vertices.push(Vec2::new(vertex.x, vertex.y));
                    }
                }
            }

            if vertices.is_empty() {
                continue;
            }

            // Calculate the centroid (center point) of the sector
            let mut centroid = Vec2::new(0.0, 0.0);
            for vertex in &vertices {
                centroid.x += vertex.x;
                centroid.y += vertex.y;
            }
            centroid.x /= vertices.len() as f32;
            centroid.y /= vertices.len() as f32;

            // Check if the centroid is inside the container sector
            if self.is_point_in_sector(centroid, container_sector_id) {
                embedded.push(sector.id);
            }
        }

        embedded
    }
}
