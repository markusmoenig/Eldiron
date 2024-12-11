use crate::prelude::Map;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct Linedef {
    pub id: u32,
    pub start_vertex: u32,
    pub end_vertex: u32,
    pub front_sector: Option<u32>,
    pub back_sector: Option<u32>,
    pub texture: Option<Uuid>,
    pub material: Option<u8>,
    pub wall_width: f32,
    pub wall_height: f32,
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
            wall_width: 0.0,
            wall_height: 0.0,
        }
    }

    // Generate geometry with UVs for the wall defined by this linedef.
    /*
    #[allow(clippy::type_complexity)]
    pub fn generate_geometry(&self, map: &Map) -> Option<(Vec<[f32; 3]>, Vec<Vec2f>, Vec<u32>)> {
        // Retrieve the start and end vertices for the linedef
        let start_vertex = map.vertices.get(self.start_vertex as usize)?;
        let end_vertex = map.vertices.get(self.end_vertex as usize)?;

        // Calculate the vector along the linedef and its perpendicular
        let dx = end_vertex.x - start_vertex.x;
        let dy = end_vertex.y - start_vertex.y;
        let length = (dx * dx + dy * dy).sqrt();

        // Normalize the direction vector
        let dir_x = dx / length;
        let dir_y = dy / length;

        // Perpendicular vector (for wall width offset)
        let perp_x = -dir_y * self.wall_width * 0.5;
        let perp_y = dir_x * self.wall_width * 0.5;

        // Compute the four corners of the wall quad
        let bottom_left = [start_vertex.x + perp_x, start_vertex.y + perp_y, 0.0];
        let top_left = [
            start_vertex.x + perp_x,
            start_vertex.y + perp_y,
            self.wall_height,
        ];
        let bottom_right = [end_vertex.x + perp_x, end_vertex.y + perp_y, 0.0];
        let top_right = [
            end_vertex.x + perp_x,
            end_vertex.y + perp_y,
            self.wall_height,
        ];

        let inner_bottom_left = [start_vertex.x - perp_x, start_vertex.y - perp_y, 0.0];
        let inner_top_left = [
            start_vertex.x - perp_x,
            start_vertex.y - perp_y,
            self.wall_height,
        ];
        let inner_bottom_right = [end_vertex.x - perp_x, end_vertex.y - perp_y, 0.0];
        let inner_top_right = [
            end_vertex.x - perp_x,
            end_vertex.y - perp_y,
            self.wall_height,
        ];

        // Combine vertices for the wall (outer face and inner face)
        let vertices = vec![
            bottom_left,
            top_left,
            bottom_right,
            top_right,
            inner_bottom_left,
            inner_top_left,
            inner_bottom_right,
            inner_top_right,
        ];

        // Generate UVs based on the wall's dimensions
        let uvs: Vec<Vec2f> = vec![
            // Outer face
            vec2f(0.0, 0.0),    // bottom_left
            vec2f(0.0, 1.0),    // top_left
            vec2f(length, 0.0), // bottom_right
            vec2f(length, 1.0), // top_right
            // Inner face
            vec2f(0.0, 0.0),    // inner_bottom_left
            vec2f(0.0, 1.0),    // inner_top_left
            vec2f(length, 0.0), // inner_bottom_right
            vec2f(length, 1.0), // inner_top_right
        ];

        // Generate indices for the wall's faces (using triangles)
        let indices = vec![
            // Outer face
            0, 1, 2, 2, 1, 3, // Inner face
            4, 5, 6, 6, 5, 7, // Top face
            1, 5, 3, 3, 5, 7, // Bottom face
            0, 4, 2, 2, 4, 6, // Left face
            0, 1, 4, 4, 1, 5, // Right face
            2, 3, 6, 6, 3, 7,
        ];

        Some((vertices, uvs, indices))
    }*/

    #[allow(clippy::type_complexity)]
    pub fn generate_geometry(&self, map: &Map) -> Option<(Vec<[f32; 3]>, Vec<Vec2f>, Vec<u32>)> {
        // Retrieve the start and end vertices for the linedef
        let start_vertex = map.vertices.get(self.start_vertex as usize)?;
        let end_vertex = map.vertices.get(self.end_vertex as usize)?;

        // Calculate the vector along the linedef and its perpendicular
        let dx = end_vertex.x - start_vertex.x;
        let dy = end_vertex.y - start_vertex.y;
        let length = (dx * dx + dy * dy).sqrt();

        // Normalize the direction vector
        let dir_x = dx / length;
        let dir_y = dy / length;

        // Perpendicular vector (for wall width offset)
        let perp_x = -dir_y * self.wall_width * 0.5;
        let perp_y = dir_x * self.wall_width * 0.5;

        // Compute the four corners of the 2D wall rectangle
        let bottom_left = [start_vertex.x + perp_x, start_vertex.y + perp_y, 0.0];
        let top_left = [start_vertex.x - perp_x, start_vertex.y - perp_y, 0.0];
        let bottom_right = [end_vertex.x + perp_x, end_vertex.y + perp_y, 0.0];
        let top_right = [end_vertex.x - perp_x, end_vertex.y - perp_y, 0.0];

        // Combine vertices for the wall
        let vertices = vec![bottom_left, top_left, bottom_right, top_right];

        // Generate UVs for texture mapping (proportional to the wall dimensions)
        let uvs: Vec<Vec2f> = vec![
            vec2f(0.0, 0.0),                // bottom_left
            vec2f(0.0, self.wall_width),    // top_left
            vec2f(length, 0.0),             // bottom_right
            vec2f(length, self.wall_width), // top_right
        ];

        // Indices for the two triangles forming the rectangle
        let mut indices = vec![0, 1, 2, 2, 1, 3];
        indices.reverse();

        Some((vertices, uvs, indices))
    }
}
/*
fn adjust_shared_vertex(
    map: &Map,
    linedef_a: &Linedef,
    linedef_b: &Linedef,
    shared_vertex: &Vertex,
    half_width: f32,
) -> ([f32; 3], [f32; 3]) {
    let start_a = map.vertices.get(linedef_a.start_vertex as usize).unwrap();
    let end_a = map.vertices.get(linedef_a.end_vertex as usize).unwrap();
    let start_b = map.vertices.get(linedef_b.start_vertex as usize).unwrap();
    let end_b = map.vertices.get(linedef_b.end_vertex as usize).unwrap();

    // Directions and perpendiculars for both linedefs
    let dir_a = vec2f(end_a.x - start_a.x, end_a.y - start_a.y).normalize();
    let dir_b = vec2f(end_b.x - start_b.x, end_b.y - start_b.y).normalize();

    let perp_a = vec2f(-dir_a.y, dir_a.x) * half_width;
    let perp_b = vec2f(-dir_b.y, dir_b.x) * half_width;

    // Calculate the intersection point of the offsets
    let offset_outer = intersect_lines(
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() + perp_a,
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() + perp_b,
    );

    let offset_inner = intersect_lines(
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() - perp_a,
        shared_vertex.as_vec2f(),
        shared_vertex.as_vec2f() - perp_b,
    );

    (
        [offset_outer.x, offset_outer.y, 0.0],
        [offset_inner.x, offset_inner.y, 0.0],
    )
}

fn intersect_lines(
    p1: Vec2f,
    p2: Vec2f,
    q1: Vec2f,
    q2: Vec2f,
) -> Vec2f {
    let a1 = p2.y - p1.y;
    let b1 = p1.x - p2.x;
    let c1 = a1 * p1.x + b1 * p1.y;

    let a2 = q2.y - q1.y;
    let b2 = q1.x - q2.x;
    let c2 = a2 * q1.x + b2 * q1.y;

    let det = a1 * b2 - a2 * b1;

    if det.abs() < f32::EPSILON {
        // Lines are parallel; return one of the endpoints
        return p1;
    }

    Vec2f::new(
        (b2 * c1 - b1 * c2) / det,
        (a1 * c2 - a2 * c1) / det,
    )
}
*/
