use crate::prelude::Map;
use earcutr::earcut;
use theframework::prelude::*;

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

    /// Sets the wall height for all linedefs in the sector.
    pub fn set_wall_height(&self, map: &mut Map, height: f32) {
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.linedefs.iter_mut().find(|l| l.id == linedef_id) {
                linedef.wall_height = height;
            }
        }
    }

    /// Generate geometry (vertices and indices) for the polygon using earcutr
    pub fn generate_geometry(&self, map: &Map) -> Option<(Vec<[f32; 2]>, Vec<u32>)> {
        // Collect unique vertices from the Linedefs in order
        let mut vertices = Vec::new();
        for &linedef_id in self.linedefs.iter() {
            let linedef = map.linedefs.get(linedef_id as usize)?;
            let start_vertex = map.vertices.get(linedef.start_vertex as usize)?;
            let vertex = [start_vertex.x, start_vertex.y];

            // Add the vertex to the list if it isn't already there
            // if vertices.last() != Some(&vertex) {
            //     vertices.push(vertex);
            // }
            //
            if !vertices.contains(&vertex) {
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
