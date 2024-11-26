use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Map {
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
            vertices: vec![],
            linedefs: vec![],
            sectors: vec![],
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    pub fn add_linedef(&mut self, linedef: Linedef) {
        self.linedefs.push(linedef);
    }

    pub fn add_sector(&mut self, sector: Sector) {
        self.sectors.push(sector);
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Vertex {
    pub id: i32,
    pub x: f32,
    pub y: f32,
}

impl Vertex {
    pub fn new(id: i32, x: f32, y: f32) -> Self {
        Self { id, x, y }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Linedef {
    pub id: i32,
    pub start_vertex: u32,
    pub end_vertex: u32,
    pub front_sector: Option<u32>,
    pub back_sector: Option<u32>,
    pub texture: Option<Uuid>,
}

impl Linedef {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            start_vertex: 0,
            end_vertex: 0,
            front_sector: None,
            back_sector: None,
            texture: None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Sector {
    pub id: i32,
    pub floor_height: f32,
    pub ceiling_height: f32,
    pub floor_texture: Option<Uuid>,
    pub ceiling_texture: Option<Uuid>,
    pub neighbours: Vec<u32>,
}

impl Sector {
    pub fn new(id: i32) -> Self {
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
