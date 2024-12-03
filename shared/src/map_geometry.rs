use crate::map::Map;
use theframework::prelude::*;

#[derive(Debug)]
pub struct Geometry {
    pub vertices: Vec<Vec3f>,
    pub indices: Vec<u32>,
    pub uvs: Vec<Vec2f>,
}

impl Default for Geometry {
    fn default() -> Self {
        Self::new()
    }
}

impl Geometry {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            uvs: Vec::new(),
        }
    }

    /// Appends another `Geometry` into this one, updating indices to match the new vertex positions.
    pub fn extend(&mut self, other: Geometry) {
        let vertex_offset = self.vertices.len() as u32;
        self.vertices.extend(other.vertices);
        self.indices
            .extend(other.indices.iter().map(|&i| i + vertex_offset));
        self.uvs.extend(other.uvs);
    }
}

pub fn generate_map_geometry(
    map: &Map,
    atlas_size: f32,
    atlas_elements: &FxHashMap<Uuid, Vec<Vec4i>>,
) -> Geometry {
    let mut geometry: Geometry = Geometry::new();

    for sector in &map.sectors {
        if let Some(floor_geo) = sector.generate_geometry(map) {
            let bbox = sector.bounding_box(map);

            // Generate floor geometry
            if let Some(floor_texture_id) = &sector.floor_texture {
                if let Some(el) = atlas_elements.get(floor_texture_id) {
                    let floor_vertices = floor_geo
                        .0
                        .iter()
                        .map(|&v| vec3f(v[0], sector.floor_height, v[1]))
                        .collect::<Vec<Vec3f>>();

                    let floor_uvs = floor_geo
                        .0
                        .iter()
                        .map(|&v| {
                            let uv = vec2f(
                                el[0].x as f32
                                    + ((v[0] - bbox.0.x) / (bbox.1.x - bbox.0.x) * el[0].z as f32),
                                el[0].y as f32
                                    + ((v[1] - bbox.0.y) / (bbox.1.y - bbox.0.y) * el[0].w as f32),
                            );
                            uv / atlas_size
                        })
                        .collect::<Vec<Vec2f>>();

                    geometry.extend(Geometry {
                        vertices: floor_vertices,
                        indices: floor_geo.1.clone(),
                        uvs: floor_uvs,
                    });
                }
            }

            // Generate ceiling geometry
            if let Some(ceiling_texture_id) = &sector.ceiling_texture {
                if let Some(el) = atlas_elements.get(ceiling_texture_id) {
                    let ceiling_vertices = floor_geo
                        .0
                        .iter()
                        .map(|&v| vec3f(v[0], sector.ceiling_height, v[1]))
                        .collect::<Vec<Vec3f>>();

                    let ceiling_uvs = floor_geo
                        .0
                        .iter()
                        .map(|&v| {
                            let uv = vec2f(
                                el[0].x as f32
                                    + ((v[0] - bbox.0.x) / (bbox.1.x - bbox.0.x) * el[0].z as f32),
                                el[0].y as f32
                                    + ((v[1] - bbox.0.y) / (bbox.1.y - bbox.0.y) * el[0].w as f32),
                            );
                            uv / atlas_size
                        })
                        .collect::<Vec<Vec2f>>();

                    geometry.extend(Geometry {
                        vertices: ceiling_vertices,
                        indices: floor_geo.1.clone(),
                        uvs: ceiling_uvs,
                    });
                }
            }

            // Generate wall geometry
            for &linedef_id in &sector.linedefs {
                if let Some(linedef) = map.linedefs.get(linedef_id as usize) {
                    let start_vertex = map.find_vertex(linedef.start_vertex).unwrap();
                    let end_vertex = map.find_vertex(linedef.end_vertex).unwrap();

                    let wall_vertices = vec![
                        vec3f(start_vertex.x, 0.0, start_vertex.y),
                        vec3f(start_vertex.x, linedef.wall_height, start_vertex.y),
                        vec3f(end_vertex.x, linedef.wall_height, end_vertex.y),
                        vec3f(end_vertex.x, 0.0, end_vertex.y),
                    ];

                    if let Some(texture_id) = &linedef.texture {
                        if let Some(el) = atlas_elements.get(texture_id) {
                            let wall_uvs = vec![
                                vec2f(el[0].x as f32, el[0].y as f32) / atlas_size,
                                vec2f(el[0].x as f32, el[0].y as f32 + el[0].w as f32) / atlas_size,
                                vec2f(
                                    el[0].x as f32 + el[0].z as f32,
                                    el[0].y as f32 + el[0].w as f32,
                                ) / atlas_size,
                                vec2f(el[0].x as f32 + el[0].z as f32, el[0].y as f32) / atlas_size,
                            ];

                            let wall_indices = vec![0, 1, 2, 0, 2, 3];

                            geometry.extend(Geometry {
                                vertices: wall_vertices,
                                indices: wall_indices,
                                uvs: wall_uvs,
                            });
                        }
                    }
                }
            }
        }
    }

    // Generate wall geometry for walls not inside a sector
    for linedef in &map.linedefs {
        if linedef.front_sector.is_none() && linedef.back_sector.is_none() {
            let start_vertex = map.find_vertex(linedef.start_vertex).unwrap();
            let end_vertex = map.find_vertex(linedef.end_vertex).unwrap();

            let wall_vertices = vec![
                vec3f(start_vertex.x, 0.0, start_vertex.y),
                vec3f(start_vertex.x, linedef.wall_height, start_vertex.y),
                vec3f(end_vertex.x, linedef.wall_height, end_vertex.y),
                vec3f(end_vertex.x, 0.0, end_vertex.y),
            ];

            if let Some(texture_id) = &linedef.texture {
                if let Some(el) = atlas_elements.get(texture_id) {
                    let wall_uvs = vec![
                        vec2f(el[0].x as f32, el[0].y as f32) / atlas_size,
                        vec2f(el[0].x as f32, el[0].y as f32 + el[0].w as f32) / atlas_size,
                        vec2f(
                            el[0].x as f32 + el[0].z as f32,
                            el[0].y as f32 + el[0].w as f32,
                        ) / atlas_size,
                        vec2f(el[0].x as f32 + el[0].z as f32, el[0].y as f32) / atlas_size,
                    ];

                    let wall_indices = vec![0, 1, 2, 0, 2, 3];

                    geometry.extend(Geometry {
                        vertices: wall_vertices,
                        indices: wall_indices,
                        uvs: wall_uvs,
                    });
                }
            }
        }
    }

    geometry
}
