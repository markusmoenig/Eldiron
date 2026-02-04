use super::pixelsource::PixelSource;
use crate::{BBox, Map, Value, ValueContainer};
use earcutr::earcut;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Sector {
    pub id: u32,

    // For editors
    pub creator_id: Uuid,

    pub name: String,
    pub linedefs: Vec<u32>,

    pub properties: ValueContainer,

    #[serde(default)]
    pub shader: Option<Uuid>,

    /// The rect tool layer for this sector (if created by the rect tool).
    #[serde(default)]
    pub layer: Option<u8>,
}

impl Sector {
    pub fn new(id: u32, linedefs: Vec<u32>) -> Self {
        let mut properties = ValueContainer::default();
        properties.set("source", Value::Source(PixelSource::Off));

        Self {
            id,
            creator_id: Uuid::new_v4(),
            name: String::new(),
            linedefs,
            properties,

            shader: None,
            layer: None,
        }
    }

    /// Returns the sector's vertices in world space as Vec<Vec3<f32>>.
    pub fn vertices_world(&self, map: &Map) -> Option<Vec<Vec3<f32>>> {
        let mut verts = Vec::new();
        for &linedef_id in &self.linedefs {
            let ld = map.find_linedef(linedef_id)?;
            let v = map.find_vertex(ld.start_vertex)?;
            verts.push(Vec3::new(v.x, v.z, v.y));
        }
        verts.dedup_by(|a, b| (a.x == b.x) && (a.y == b.y) && (a.z == b.z));
        if verts.len() < 3 {
            return None;
        }
        Some(verts)
    }

    /// Returns the vertical span (min_y, max_y) of this sector in world space.
    pub fn y_span(&self, map: &Map) -> Option<(f32, f32)> {
        let verts = self.vertices_world(map)?;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for p in verts {
            min_y = min_y.min(p.y);
            max_y = max_y.max(p.y);
        }
        if min_y.is_finite() && max_y.is_finite() {
            Some((min_y, max_y))
        } else {
            None
        }
    }

    /// Checks whether this sector intersects a vertical slice centered at `slice_y` with thickness `thickness`.
    pub fn intersects_vertical_slice(&self, map: &Map, slice_y: f32, thickness: f32) -> bool {
        if thickness <= 0.0 {
            return false;
        }
        if let Some((min_y, max_y)) = self.y_span(map) {
            let half = thickness * 0.5;
            let y0 = slice_y - half;
            let y1 = slice_y + half;
            max_y >= y0 && min_y <= y1
        } else {
            false
        }
    }

    // Generate a bounding box for the sector
    pub fn bounding_box(&self, map: &Map) -> BBox {
        // Collect all vertices for the sector
        let mut vertices = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    vertices.push(Vec2::new(start_vertex.x, start_vertex.y));
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        vertices.push(Vec2::new(end_vertex.x, end_vertex.y));
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

        BBox::new(Vec2::new(min_x, min_y), Vec2::new(max_x, max_y))
    }

    /// Calculate the center of the sector
    pub fn center(&self, map: &Map) -> Option<Vec2<f32>> {
        // Collect all vertices for the sector
        let mut vertices = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    vertices.push(Vec2::new(start_vertex.x, start_vertex.y));
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        vertices.push(Vec2::new(end_vertex.x, end_vertex.y));
                    }
                }
            }
        }

        // Ensure we have vertices to calculate the center
        if vertices.is_empty() {
            return None;
        }

        // Calculate the average x and y coordinates
        let sum = vertices.iter().fold(Vec2::new(0.0, 0.0), |acc, v| acc + *v);
        let count = vertices.len() as f32;
        Some(sum / count)
    }

    /// Calculate the center of the sector using 3D coords.
    pub fn center_3d(&self, map: &Map) -> Option<Vec3<f32>> {
        // Collect all vertices for the sector
        let mut vertices = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    vertices.push(Vec2::new(start_vertex.x, start_vertex.y));
                    if let Some(end_vertex) = map.find_vertex(linedef.end_vertex) {
                        vertices.push(Vec2::new(end_vertex.x, end_vertex.y));
                    }
                }
            }
        }

        // Ensure we have vertices to calculate the center
        if vertices.is_empty() {
            return None;
        }

        // Calculate the average x and y coordinates
        let sum = vertices.iter().fold(Vec3::zero(), |acc, v| acc + *v);
        let count = vertices.len() as f32;
        Some(sum / count)
    }

    /// Calculate the area of the sector (for sorting).
    pub fn area(&self, map: &Map) -> f32 {
        // Generate geometry for the sector
        if let Some((vertices, indices)) = self.generate_geometry(map) {
            // Calculate the area by summing up the areas of individual triangles
            indices.iter().fold(0.0, |acc, &(i1, i2, i3)| {
                let v1 = vertices[i1];
                let v2 = vertices[i2];
                let v3 = vertices[i3];

                // Calculate the area of the triangle using the shoelace formula
                acc + 0.5
                    * ((v1[0] * v2[1] + v2[0] * v3[1] + v3[0] * v1[1])
                        - (v1[1] * v2[0] + v2[1] * v3[0] + v3[1] * v1[0]))
                        .abs()
            })
        } else {
            0.0 // Return 0 if the geometry couldn't be generated
        }
    }

    /// Generate geometry (vertices and indices) for the polygon using earcutr
    #[allow(clippy::type_complexity)]
    pub fn generate_geometry(
        &self,
        map: &Map,
    ) -> Option<(Vec<[f32; 2]>, Vec<(usize, usize, usize)>)> {
        // Collect unique vertices from the Linedefs in order
        let mut vertices = Vec::new();
        for &linedef_id in self.linedefs.iter() {
            let linedef = map.find_linedef(linedef_id)?;
            let start_vertex = map.get_vertex(linedef.start_vertex)?;
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
            let indices: Vec<(usize, usize, usize)> = indices
                .chunks_exact(3)
                .map(|chunk| (chunk[2], chunk[1], chunk[0]))
                .collect();
            Some((vertices, indices))
        } else {
            None
        }
    }

    // Returns a random position inside the sector.
    // pub fn get_random_position(&self, map: &Map) -> Option<Vec2<f32>> {
    //     // Generate geometry for the sector
    //     if let Some((vertices, indices)) = self.generate_geometry(map) {
    //         // Create a random number generator
    //         let mut rng = rand::rng();

    //         // Randomly select a triangle from the indices
    //         if let Some(&(i1, i2, i3)) = indices.choose(&mut rng) {
    //             let v1 = vertices[i1];
    //             let v2 = vertices[i2];
    //             let v3 = vertices[i3];

    //             // Generate random barycentric coordinates
    //             let r1: f32 = rng.random();
    //             let r2: f32 = rng.random();

    //             // Ensure they are constrained to the triangle
    //             let sqrt_r1 = r1.sqrt();
    //             let u = 1.0 - sqrt_r1;
    //             let v = r2 * sqrt_r1;

    //             // Compute the random position as a weighted sum of the triangle's vertices
    //             let x = u * v1[0] + v * v2[0] + (1.0 - u - v) * v3[0];
    //             let y = u * v1[1] + v * v2[1] + (1.0 - u - v) * v3[1];

    //             Some(Vec2::new(x, y))
    //         } else {
    //             None // Return None if no triangles are available
    //         }
    //     } else {
    //         None // Return None if geometry couldn't be generated
    //     }
    // }

    /// Checks if a point is inside the sector polygon using the ray-casting algorithm.
    pub fn is_inside(&self, map: &Map, point: Vec2<f32>) -> bool {
        // Collect the polygon vertices
        let mut polygon = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(start_vertex) = map.get_vertex(linedef.start_vertex) {
                    polygon.push(Vec2::new(start_vertex.x, start_vertex.y));
                }
            }
        }

        // Early exit if the polygon is invalid
        if polygon.len() < 3 {
            return false; // A polygon must have at least 3 vertices
        }

        // Ray-casting algorithm
        let mut inside = false;
        let mut j = polygon.len() - 1;

        for i in 0..polygon.len() {
            if (polygon[i].y > point.y) != (polygon[j].y > point.y)
                && point.x
                    < (polygon[j].x - polygon[i].x) * (point.y - polygon[i].y)
                        / (polygon[j].y - polygon[i].y)
                        + polygon[i].x
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    /// Generate the signed distance to the
    pub fn signed_distance(&self, map: &Map, point: Vec2<f32>) -> Option<f32> {
        let mut min_dist = f32::MAX;

        // Distance to nearest edge
        for &linedef_id in &self.linedefs {
            if let Some(ld) = map.find_linedef(linedef_id) {
                let v0 = map.get_vertex(ld.start_vertex)?;
                let v1 = map.get_vertex(ld.end_vertex)?;
                let edge = v1 - v0;
                let to_point = point - v0;

                let t = to_point.dot(edge) / edge.dot(edge);
                let t_clamped = t.clamp(0.0, 1.0);
                let closest = v0 + edge * t_clamped;

                let dist = (point - closest).magnitude();
                min_dist = min_dist.min(dist);
            }
        }

        // Check if point is inside
        let inside = self.is_inside(map, point);

        // Return signed distance
        Some(if inside { -min_dist } else { min_dist })
    }

    /// Generate 2D walls with uniform thickness for the sector using mitered joins.
    #[allow(clippy::type_complexity)]
    pub fn generate_wall_geometry(
        &self,
        map: &Map,
        thickness: f32,
    ) -> Option<(Vec<[f32; 2]>, Vec<(usize, usize, usize)>)> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // Collect ordered unique vertices of the sector
        let mut polygon = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    let v_start = Vec2::new(start_vertex.x, start_vertex.y);
                    polygon.push(v_start);
                }
            }
        }

        // Ensure the polygon is closed
        if let (Some(&first), Some(&last)) = (polygon.first(), polygon.last()) {
            if first != last {
                polygon.push(first);
            }
        }

        // Remove the artificially duplicated last point
        polygon.pop();

        let len = polygon.len();
        if len < 3 {
            return None;
        }

        // We'll store our "inner" and "outer" offset points
        let mut outer_points = Vec::with_capacity(len);
        let mut inner_points = Vec::with_capacity(len);

        for i in 0..len {
            let prev = polygon[(i + len - 1) % len];
            let curr = polygon[i];
            let next = polygon[(i + 1) % len];

            // Compute segment direction and normal for edges (prev->curr) and (curr->next)
            let dir1 = (curr - prev).normalized();
            let dir2 = (next - curr).normalized();

            let normal1 = Vec2::new(-dir1.y, dir1.x);
            let normal2 = Vec2::new(-dir2.y, dir2.x);

            // Compute the bisector and how much to offset
            let bisector = (normal1 + normal2).normalized();
            let angle = dir1.angle_between(dir2) / 2.0;

            // Prevent near-zero or negative cosines from blowing up offset_length
            let offset_length = thickness / (2.0 * angle.cos()).max(0.1);

            let outer = curr + bisector * offset_length;
            let inner = curr - bisector * offset_length;

            outer_points.push(outer);
            inner_points.push(inner);
        }

        // Convert all offset points into 'vertices' in one big buffer
        let mut outer_indices = Vec::with_capacity(len);
        let mut inner_indices = Vec::with_capacity(len);

        for &pt in &outer_points {
            outer_indices.push(vertices.len());
            vertices.push([pt.x, pt.y]);
        }

        for &pt in &inner_points {
            inner_indices.push(vertices.len());
            vertices.push([pt.x, pt.y]);
        }

        // Now generate triangles connecting outer/inner rings
        for i in 0..len {
            let next = (i + 1) % len;

            // Outer ring indices
            let o1 = outer_indices[i];
            let o2 = outer_indices[next];
            // Inner ring indices
            let i1 = inner_indices[i];
            let i2 = inner_indices[next];

            // Two triangles per quad:
            //  1) outer[i], outer[i+1], inner[i]
            //  2) outer[i+1], inner[i+1], inner[i]
            indices.push((o1, o2, i1));
            indices.push((o2, i2, i1));
        }

        Some((vertices, indices))
    }

    /// For each linedef, we produce its own (vertices, indices).
    #[allow(clippy::type_complexity)]
    pub fn generate_wall_geometry_by_linedef(
        &self,
        map: &Map,
        // thickness: f32,
    ) -> Option<FxHashMap<u32, (Vec<[f32; 2]>, Vec<(usize, usize, usize)>)>> {
        let mut result: FxHashMap<u32, (Vec<[f32; 2]>, Vec<(usize, usize, usize)>)> =
            FxHashMap::default();

        let mut all_walls_have_no_width = true;
        let mut polygon = Vec::new();
        for &linedef_id in &self.linedefs {
            if let Some(linedef) = map.find_linedef(linedef_id) {
                if let Some(start_vertex) = map.find_vertex(linedef.start_vertex) {
                    let v_start = Vec2::new(start_vertex.x, start_vertex.y);
                    polygon.push(v_start);
                    if linedef.properties.get_float_default("wall_width", 0.0) > 0.0 {
                        all_walls_have_no_width = false;
                    }
                }
            }
        }

        // If all walls have no width, return None
        if all_walls_have_no_width {
            return None;
        }

        // Ensure the polygon is closed
        if let (Some(&first), Some(&last)) = (polygon.first(), polygon.last()) {
            if first != last {
                polygon.push(first);
            }
        }
        // Remove the artificially duplicated last point (avoid zero-length edge)
        polygon.pop();

        let len = polygon.len();
        if len < 3 {
            return None;
        }

        let mut outer_points = Vec::with_capacity(len);
        let mut inner_points = Vec::with_capacity(len);

        /*
        for i in 0..len {
            let prev = polygon[(i + len - 1) % len];
            let curr = polygon[i];
            let next = polygon[(i + 1) % len];

            // Directions and normals
            let dir1 = (curr - prev).normalized();
            let dir2 = (next - curr).normalized();

            let normal1 = Vec2::new(-dir1.y, dir1.x);
            let normal2 = Vec2::new(-dir2.y, dir2.x);

            // Bisector for the corner
            let bisector = (normal1 + normal2).normalized();

            // Half the angle between edges
            let angle = dir1.angle_between(dir2) / 2.0;
            // Avoid blow-ups for near-180° angles
            let offset_length = thickness / (2.0 * angle.cos()).max(0.1);

            let outer = curr + bisector * offset_length;
            let inner = curr - bisector * offset_length;

            outer_points.push(outer);
            inner_points.push(inner);
        }*/

        // For each corner i in [0..len)
        for i in 0..len {
            let prev = polygon[(i + len - 1) % len];
            let curr = polygon[i];
            let next = polygon[(i + 1) % len];

            // Directions, normals, angle as before...
            let dir1 = (curr - prev).normalized();
            let dir2 = (next - curr).normalized();
            let normal1 = Vec2::new(-dir1.y, dir1.x);
            let normal2 = Vec2::new(-dir2.y, dir2.x);
            let bisector = (normal1 + normal2).normalized();
            let angle = dir1.angle_between(dir2) / 2.0;

            // Look up each linedef's thickness:
            let prev_line_id = self.linedefs[(i + len - 1) % len];
            let curr_line_id = self.linedefs[i];

            let mut t_prev = 0.0;
            let mut t_curr = 0.0;

            if let Some(linedef) = map.find_linedef(prev_line_id) {
                t_prev = linedef.properties.get_float_default("wall_width", 0.0);
            }

            if let Some(linedef) = map.find_linedef(curr_line_id) {
                t_curr = linedef.properties.get_float_default("wall_width", 0.0);
            }

            // For a "smooth" approach, average them. (Or pick whichever logic you prefer.)
            let corner_thickness = (t_prev + t_curr) * 0.5;

            // The final offset distance at corner i
            let offset_length = corner_thickness / (2.0 * angle.cos()).max(0.1);

            let outer = curr + bisector * offset_length;
            let inner = curr - bisector * offset_length;

            outer_points.push(outer);
            inner_points.push(inner);
        }

        // The "wall" between corner i and i+1 is a quad -> 2 triangles
        // We'll store them in the HashMap keyed by linedef_id.
        for i in 0..len {
            let next = (i + 1) % len;
            let linedef_id = self.linedefs[i];

            // 4 local, duplicated vertices for this linedef
            let o1 = outer_points[i];
            let o2 = outer_points[next];
            let i1 = inner_points[i];
            let i2 = inner_points[next];

            // We'll store them in order: [o1, o2, i2, i1]
            // so the winding of the first triangle is (0->1->3), etc.
            let local_verts = vec![
                [o1.x, o1.y], // index 0
                [o2.x, o2.y], // index 1
                [i2.x, i2.y], // index 2
                [i1.x, i1.y], // index 3
            ];

            let local_inds = vec![(0, 1, 3), (1, 2, 3)];
            result.insert(linedef_id, (local_verts, local_inds));
        }

        Some(result)
    }
}

impl PartialEq for Sector {
    fn eq(&self, other: &Self) -> bool {
        // Compare sectors by their geometry: same set of linedef IDs,
        // ignoring order (and ignoring duplicate IDs if any).
        let mut a = self.linedefs.clone();
        let mut b = other.linedefs.clone();
        a.sort_unstable();
        a.dedup();
        b.sort_unstable();
        b.dedup();
        a == b
    }
}

impl Eq for Sector {}
