use earcutr::earcut;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum MapCamera {
    TwoD,
    ThreeDIso,
    ThreeDFirstPerson,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Map {
    pub offset: Vec2f,
    pub grid_size: f32,
    pub subdivisions: f32,

    // When adding linedefs we keep track of them to check if we have a closed polygon
    #[serde(skip)]
    pub possible_polygon: Vec<u32>,

    // For temporary line previews
    #[serde(skip)]
    pub curr_grid_pos: Option<Vec2f>,
    #[serde(skip)]
    pub curr_mouse_pos: Option<Vec2f>,
    #[serde(skip)]
    pub curr_rectangle: Option<(Vec2f, Vec2f)>,

    pub vertices: Vec<Vertex>,
    pub linedefs: Vec<Linedef>,
    pub sectors: Vec<Sector>,

    // Camera Mode
    pub camera: MapCamera,

    // Selection
    pub selected_vertices: Vec<u32>,
    pub selected_linedefs: Vec<u32>,
    pub selected_sectors: Vec<u32>,
}

impl Default for Map {
    fn default() -> Self {
        Self::new()
    }
}

impl Map {
    pub fn new() -> Self {
        Self {
            offset: Vec2f::zero(),
            grid_size: 30.0,
            subdivisions: 1.0,

            possible_polygon: vec![],
            curr_grid_pos: None,
            curr_mouse_pos: None,
            curr_rectangle: None,

            vertices: vec![],
            linedefs: vec![],
            sectors: vec![],

            camera: MapCamera::TwoD,

            selected_vertices: vec![],
            selected_linedefs: vec![],
            selected_sectors: vec![],
        }
    }

    // Clear temporary data
    pub fn clear_temp(&mut self) {
        self.possible_polygon = vec![];
        self.curr_grid_pos = None;
        self.curr_mouse_pos = None;
        self.curr_rectangle = None;
    }

    //
    pub fn add_vertex_at(&mut self, x: f32, y: f32) -> u32 {
        // Check if the vertex already exists
        if let Some(id) = self.find_vertex_at(x, y) {
            return id;
        }

        let id = self.vertices.len() as u32;

        let vertex = Vertex::new(id, x, y);
        self.vertices.push(vertex);

        id
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

    // Create a new (or use an existing) linedef for the given vertices.
    pub fn create_linedef(&mut self, start_vertex: u32, end_vertex: u32) -> (u32, Option<u32>) {
        let id = self.linedefs.len() as u32;
        let mut sector_id: Option<u32> = None;

        let linedef = Linedef::new(id, start_vertex, end_vertex);
        self.linedefs.push(linedef);
        self.possible_polygon.push(id);

        if let Some(sid) = self.create_sector_from_polygon() {
            sector_id = Some(sid);
        }
        (id, sector_id)
    }

    /// Check if the `possible_polygon` forms a closed loop
    pub fn test_for_closed_polygon(&self) -> bool {
        if self.possible_polygon.len() < 3 {
            return false; // A polygon needs at least 3 edges
        }

        let first_linedef = &self.linedefs[self.possible_polygon[0] as usize];
        let last_linedef =
            &self.linedefs[self.possible_polygon[self.possible_polygon.len() - 1] as usize];

        // Check if the last linedef's end_vertex matches the first linedef's start_vertex
        last_linedef.end_vertex == first_linedef.start_vertex
    }

    /// Tries to create a polyon from the tracked vertices in possible_polygon
    pub fn create_sector_from_polygon(&mut self) -> Option<u32> {
        if !self.test_for_closed_polygon() {
            //println!("Polygon is not closed. Cannot create sector.");
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
        let sector_id = self.sectors.len() as u32;
        // println!(
        //     "Created sector ID: {} with linedefs: {:?}",
        //     sector_id, self.possible_polygon
        // );

        for &id in &self.possible_polygon {
            if let Some(linedef) = self.linedefs.iter_mut().find(|l| l.id == id) {
                // Assign the sector ID to the front or back
                if linedef.front_sector.is_none() {
                    linedef.front_sector = Some(sector_id);
                } else if linedef.back_sector.is_none() {
                    linedef.back_sector = Some(sector_id);
                } else {
                    println!(
                        "Warning: Linedef {} already has both front and back sectors assigned.",
                        linedef.id
                    );
                }
            }
        }

        let sector = Sector::new(sector_id, self.possible_polygon.clone());
        self.sectors.push(sector);

        self.possible_polygon.clear(); // Reset after creating the sector
        Some(sector_id)
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

    /// Deletes the specified vertices, linedefs, and sectors, along with their associated geometry.
    pub fn delete_elements(&mut self, vertex_ids: &[u32], linedef_ids: &[u32], sector_ids: &[u32]) {
        // 1. Delete specified vertices
        if !vertex_ids.is_empty() {
            // Remove vertices
            self.vertices
                .retain(|vertex| !vertex_ids.contains(&vertex.id));

            // Remove any linedefs that depend on the deleted vertices
            self.linedefs.retain(|linedef| {
                !vertex_ids.contains(&linedef.start_vertex)
                    && !vertex_ids.contains(&linedef.end_vertex)
            });

            // Remove references to these linedefs in sectors
            self.cleanup_sectors();
        }

        // 2. Delete specified linedefs
        if !linedef_ids.is_empty() {
            // Remove linedefs
            self.linedefs
                .retain(|linedef| !linedef_ids.contains(&linedef.id));

            // Remove references to these linedefs in sectors
            self.cleanup_sectors();
        }

        // 3. Delete specified sectors
        if !sector_ids.is_empty() {
            // Remove sectors
            self.sectors
                .retain(|sector| !sector_ids.contains(&sector.id));

            // Remove references to these sectors in linedefs
            for linedef in &mut self.linedefs {
                if let Some(front_sector) = linedef.front_sector {
                    if sector_ids.contains(&front_sector) {
                        linedef.front_sector = None;
                    }
                }
                if let Some(back_sector) = linedef.back_sector {
                    if sector_ids.contains(&back_sector) {
                        linedef.back_sector = None;
                    }
                }
            }
        }
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
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Vertex {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

impl Vertex {
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self { id, x, y }
    }

    pub fn as_vec2f(&self) -> Vec2f {
        vec2f(self.x, self.y)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Linedef {
    pub id: u32,
    pub start_vertex: u32,
    pub end_vertex: u32,
    pub front_sector: Option<u32>,
    pub back_sector: Option<u32>,
    pub texture: Option<Uuid>,
    pub material: Option<u8>,
}

impl Linedef {
    pub fn new(id: u32, start_vertex: u32, end_vertex: u32) -> Self {
        Self {
            id,
            start_vertex,
            end_vertex,
            front_sector: None,
            back_sector: None,
            texture: None,
            material: None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Sector {
    pub id: u32,
    pub linedefs: Vec<u32>,
    pub floor_height: f32,
    pub ceiling_height: f32,
    pub floor_texture: Option<Uuid>,
    pub ceiling_texture: Option<Uuid>,
    pub floor_material: Option<u8>,
    pub ceiling_material: Option<u8>,
    pub neighbours: Vec<u32>,
}

impl Sector {
    pub fn new(id: u32, linedefs: Vec<u32>) -> Self {
        Self {
            id,
            linedefs,
            floor_height: 0.0,
            ceiling_height: 0.0,
            floor_texture: None,
            ceiling_texture: None,
            floor_material: None,
            ceiling_material: None,
            neighbours: vec![],
        }
    }

    // Generate a bounding box for the sector
    pub fn bounding_box(&self, map: &Map) -> (Vec2f, Vec2f) {
        // Collect all vertices for the sector
        let mut vertices = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                if let Some(start_vertex) = map.vertices.get(linedef.start_vertex as usize) {
                    vertices.push(Vec2f::new(start_vertex.x, start_vertex.y));
                    if let Some(end_vertex) = map.vertices.get(linedef.end_vertex as usize) {
                        vertices.push(Vec2f::new(end_vertex.x, end_vertex.y));
                    }
                }
            }
        }

        // Find min and max coordinates
        let min_x = vertices.iter().map(|v| v.x).fold(f32::INFINITY, f32::min);
        let max_x = vertices
            .iter()
            .map(|v| v.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min);
        let max_y = vertices
            .iter()
            .map(|v| v.y)
            .fold(f32::NEG_INFINITY, f32::max);

        // Return the bounding box corners
        (Vec2f::new(min_x, min_y), Vec2f::new(max_x, max_y))
    }

    /// Generate geometry (vertices and indices) for the polygon using earcutr
    pub fn generate_geometry(&self, map: &Map) -> Option<(Vec<[f32; 2]>, Vec<u32>)> {
        // Collect unique vertices from the Linedefs in order
        let mut vertices = Vec::new();
        for &linedef_id in &self.linedefs {
            let linedef = map.linedefs.get(linedef_id as usize)?;
            let start_vertex = map.vertices.get(linedef.start_vertex as usize)?;
            let vertex = [start_vertex.x, start_vertex.y];

            // Add the vertex to the list if it isn't already there
            if vertices.last() != Some(&vertex) {
                vertices.push(vertex);
            }
        }

        // Flatten the vertices for earcutr
        let flattened_vertices: Vec<f64> = vertices
            .iter()
            .flat_map(|v| vec![v[0] as f64, v[1] as f64])
            .collect();

        // No holes in this example, so pass an empty holes array
        let holes: Vec<usize> = Vec::new();

        // Perform triangulation
        if let Ok(indices) = earcut(&flattened_vertices, &holes, 2) {
            let indices: Vec<u32> = indices.iter().rev().map(|&i| i as u32).collect();
            Some((vertices, indices))
        } else {
            None
        }
    }
}
