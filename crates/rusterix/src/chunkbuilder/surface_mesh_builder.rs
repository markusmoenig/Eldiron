use super::action::{ControlPoint, MeshTopology, SectorMeshDescriptor};
use crate::Surface;
use vek::{Vec2, Vec3};

/// Result of mesh generation for a surface action
#[derive(Debug, Clone)]
pub struct GeneratedMesh {
    /// World-space vertices [x, y, z, w]
    pub vertices: Vec<[f32; 4]>,
    /// Triangle indices
    pub indices: Vec<(usize, usize, usize)>,
    /// UV coordinates for texturing
    pub uvs: Vec<[f32; 2]>,
    /// Whether this mesh should cut holes in the base surface
    pub cuts_base_surface: bool,
}

/// Unified mesh builder that processes SectorMeshDescriptors
pub struct SurfaceMeshBuilder<'a> {
    surface: &'a Surface,
}

impl<'a> SurfaceMeshBuilder<'a> {
    pub fn new(surface: &'a Surface) -> Self {
        Self { surface }
    }

    /// Build mesh geometry from a descriptor
    pub fn build(&self, descriptor: &SectorMeshDescriptor) -> Vec<GeneratedMesh> {
        let mut meshes = Vec::new();

        // Build cap if present
        if let Some(cap_topology) = &descriptor.cap {
            if let Some(cap_mesh) = self.build_topology(cap_topology, false) {
                meshes.push(cap_mesh);
            }
        }

        // Build sides if present
        if let Some(side_topology) = &descriptor.sides {
            if let Some(side_mesh) = self.build_topology(side_topology, true) {
                meshes.push(side_mesh);
            }
        }

        meshes
    }

    /// Build mesh from a topology description
    fn build_topology(&self, topology: &MeshTopology, is_side: bool) -> Option<GeneratedMesh> {
        match topology {
            MeshTopology::Loop(points) => self.build_loop(points, is_side),
            MeshTopology::FilledRegion { outer, holes } => {
                self.build_filled_region(outer, holes, is_side)
            }
            MeshTopology::QuadStrip { loop_a, loop_b } => self.build_quad_strip(loop_a, loop_b),
        }
    }

    /// Build a simple loop (currently just returns empty, as loops need context)
    fn build_loop(&self, _points: &[ControlPoint], _is_side: bool) -> Option<GeneratedMesh> {
        // Loops by themselves don't create mesh, they define boundaries
        // This could be extended to create line geometry for debugging
        None
    }

    /// Build a filled region (triangulated cap)
    fn build_filled_region(
        &self,
        outer: &[ControlPoint],
        holes: &[Vec<ControlPoint>],
        _is_side: bool,
    ) -> Option<GeneratedMesh> {
        if outer.len() < 3 {
            return None;
        }

        // Convert control points to world space vertices
        let mut all_vertices = Vec::new();
        let mut all_uvs = Vec::new();

        // Process outer loop
        for cp in outer {
            let world_pos = self.control_point_to_world(cp);
            all_vertices.push([world_pos.x, world_pos.y, world_pos.z, 1.0]);
            all_uvs.push([cp.uv.x, cp.uv.y]);
        }

        // Track where holes start for earcut
        let mut hole_indices = Vec::new();

        // Process holes
        for hole in holes {
            hole_indices.push(all_vertices.len());
            for cp in hole {
                let world_pos = self.control_point_to_world(cp);
                all_vertices.push([world_pos.x, world_pos.y, world_pos.z, 1.0]);
                all_uvs.push([cp.uv.x, cp.uv.y]);
            }
        }

        // Triangulate using earcut
        let flat_coords: Vec<f64> = all_uvs
            .iter()
            .flat_map(|uv| [uv[0] as f64, uv[1] as f64])
            .collect();

        let triangle_indices = earcutr::earcut(&flat_coords, &hole_indices, 2).ok()?;

        // Convert to triangle tuples with reversed winding
        let indices: Vec<(usize, usize, usize)> = triangle_indices
            .chunks_exact(3)
            .map(|chunk| (chunk[2], chunk[1], chunk[0]))
            .collect();

        Some(GeneratedMesh {
            vertices: all_vertices,
            indices,
            uvs: all_uvs,
            cuts_base_surface: false,
        })
    }

    /// Build a quad strip connecting two loops (used for sides/walls)
    fn build_quad_strip(
        &self,
        loop_a: &[ControlPoint],
        loop_b: &[ControlPoint],
    ) -> Option<GeneratedMesh> {
        if loop_a.len() < 2 || loop_b.len() != loop_a.len() {
            return None;
        }

        let n = loop_a.len();
        let mut vertices = Vec::with_capacity(n * 2);
        let mut uvs = Vec::with_capacity(n * 2);
        let mut indices = Vec::with_capacity((n - 1) * 2);

        // Calculate perimeter distances for UV mapping
        let mut perimeter_dists = vec![0.0f32];
        let mut total_dist = 0.0f32;

        for i in 0..n {
            let curr_a = self.control_point_to_world(&loop_a[i]);
            let next_a = self.control_point_to_world(&loop_a[(i + 1) % n]);
            total_dist += (next_a - curr_a).magnitude();
            perimeter_dists.push(total_dist);
        }

        // Build vertices and UVs
        for i in 0..n {
            let pos_a = self.control_point_to_world(&loop_a[i]);
            let pos_b = self.control_point_to_world(&loop_b[i]);

            vertices.push([pos_a.x, pos_a.y, pos_a.z, 1.0]);
            vertices.push([pos_b.x, pos_b.y, pos_b.z, 1.0]);

            // U coordinate: normalized distance along perimeter
            let u = if total_dist > 1e-6 {
                perimeter_dists[i] / total_dist
            } else {
                i as f32 / n as f32
            };

            // V coordinate: 0 at loop_a, 1 at loop_b
            uvs.push([u, 0.0]);
            uvs.push([u, 1.0]);
        }

        // Build quad faces (two triangles per quad)
        for i in 0..n {
            let curr_a = i * 2;
            let curr_b = i * 2 + 1;
            let next_a = ((i + 1) % n) * 2;
            let next_b = ((i + 1) % n) * 2 + 1;

            // First triangle
            indices.push((curr_a, next_a, next_b));
            // Second triangle
            indices.push((curr_a, next_b, curr_b));
        }

        Some(GeneratedMesh {
            vertices,
            indices,
            uvs,
            cuts_base_surface: false,
        })
    }

    /// Convert a control point (UV + extrusion) to world space coordinates
    fn control_point_to_world(&self, cp: &ControlPoint) -> Vec3<f32> {
        // Use the surface's uvw_to_world method
        // UV gives position on surface plane, extrusion is the W component
        let world_pos = self.surface.uvw_to_world(cp.uv, cp.extrusion);
        world_pos
    }
}

/// Helper function to fix triangle winding based on desired normal
pub fn fix_winding(
    vertices: &[[f32; 4]],
    indices: &mut Vec<(usize, usize, usize)>,
    desired_normal: Vec3<f32>,
) {
    if indices.is_empty() || vertices.len() < 3 {
        return;
    }

    // Sample a few triangles to determine the average normal
    let mut avg_normal = Vec3::zero();
    let sample_count = indices.len().min(8);

    for &(a, b, c) in indices.iter().take(sample_count) {
        if a >= vertices.len() || b >= vertices.len() || c >= vertices.len() {
            continue;
        }

        let va = Vec3::new(vertices[a][0], vertices[a][1], vertices[a][2]);
        let vb = Vec3::new(vertices[b][0], vertices[b][1], vertices[b][2]);
        let vc = Vec3::new(vertices[c][0], vertices[c][1], vertices[c][2]);

        avg_normal += (vb - va).cross(vc - va);
    }

    let mag = avg_normal.magnitude();
    if mag < 1e-8 {
        return; // Degenerate triangles
    }

    avg_normal /= mag;

    // If the normals point in opposite directions, flip winding
    if avg_normal.dot(desired_normal) < 0.0 {
        for tri in indices.iter_mut() {
            std::mem::swap(&mut tri.1, &mut tri.2);
        }
    }
}

/// Calculate UV coordinates for a set of vertices
/// Supports both fit (normalize to 0..1) and tile (world-space) modes
pub fn calculate_uvs(
    vertices_2d: &[[f32; 2]],
    tile_mode: bool,
    texture_scale: Vec2<f32>,
) -> Vec<[f32; 2]> {
    if vertices_2d.is_empty() {
        return vec![];
    }

    // Find bounds
    let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
    let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);

    for &[x, y] in vertices_2d {
        min.x = min.x.min(x);
        min.y = min.y.min(y);
        max.x = max.x.max(x);
        max.y = max.y.max(y);
    }

    let size = max - min;
    let size = Vec2::new(size.x.max(1e-6), size.y.max(1e-6));

    if tile_mode {
        // Tile mode: scale by texture_scale in world units
        vertices_2d
            .iter()
            .map(|&[x, y]| {
                [
                    (x - min.x) / texture_scale.x.max(1e-6),
                    (y - min.y) / texture_scale.y.max(1e-6),
                ]
            })
            .collect()
    } else {
        // Fit mode: normalize to 0..1
        vertices_2d
            .iter()
            .map(|&[x, y]| [(x - min.x) / size.x, (y - min.y) / size.y])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_uvs_fit_mode() {
        let vertices = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let uvs = calculate_uvs(&vertices, false, Vec2::new(1.0, 1.0));

        assert_eq!(uvs.len(), 4);
        assert!((uvs[0][0] - 0.0).abs() < 1e-5);
        assert!((uvs[0][1] - 0.0).abs() < 1e-5);
        assert!((uvs[2][0] - 1.0).abs() < 1e-5);
        assert!((uvs[2][1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_calculate_uvs_tile_mode() {
        let vertices = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let uvs = calculate_uvs(&vertices, true, Vec2::new(2.0, 2.0));

        assert_eq!(uvs.len(), 4);
        assert!((uvs[2][0] - 5.0).abs() < 1e-5); // 10 / 2 = 5
        assert!((uvs[2][1] - 5.0).abs() < 1e-5); // 10 / 2 = 5
    }
}
