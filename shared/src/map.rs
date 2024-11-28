use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Map {
    pub offset: Vec2f,
    pub grid_size: f32,

    // For temporary line previews
    pub curr_grid_pos: Option<Vec2f>,
    pub curr_mouse_pos: Option<Vec2f>,

    pub vertices: Vec<Vertex>,
    pub linedefs: Vec<Linedef>,
    pub sectors: Vec<Sector>,
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

            curr_grid_pos: None,
            curr_mouse_pos: None,

            vertices: vec![],
            linedefs: vec![],
            sectors: vec![],
        }
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

    pub fn create_linedef(&mut self, start_vertex: u32, end_vertex: u32) -> u32 {
        let id = self.linedefs.len() as u32;

        let linedef = Linedef::new(id, start_vertex, end_vertex);
        self.linedefs.push(linedef);

        id
    }

    pub fn add_sector(&mut self, sector: Sector) {
        self.sectors.push(sector);
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
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Sector {
    pub id: u32,
    pub floor_height: f32,
    pub ceiling_height: f32,
    pub floor_texture: Option<Uuid>,
    pub ceiling_texture: Option<Uuid>,
    pub neighbours: Vec<u32>,
}

impl Sector {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            floor_height: 0.0,
            ceiling_height: 0.0,
            floor_texture: None,
            ceiling_texture: None,
            neighbours: vec![],
        }
    }
}
