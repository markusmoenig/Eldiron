use crate::{BBox, ParticleEmissionShape, ParticleEmitterDef, PixelSource, ValueContainer};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use theframework::prelude::FxHashMap;
use uuid::Uuid;
use vek::{Vec2, Vec3};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryObjectKind {
    Brush,
    Prop,
    Generated,
}

/// A persistent light source authored from a geometry face. The map stores these by the face's
/// effective paint-surface ID so generated faces can be rebuilt without losing their emission.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FaceEmission {
    #[serde(default = "default_face_emission_color")]
    pub color: [f32; 3],
    #[serde(default = "default_face_emission_intensity")]
    pub intensity: f32,
    #[serde(default = "default_face_emission_start_distance")]
    pub start_distance: f32,
    #[serde(default = "default_face_emission_end_distance")]
    pub end_distance: f32,
    #[serde(default = "default_face_emission_offset")]
    pub offset: f32,
    #[serde(default)]
    pub flicker: f32,
}

impl Default for FaceEmission {
    fn default() -> Self {
        Self {
            color: default_face_emission_color(),
            intensity: default_face_emission_intensity(),
            start_distance: default_face_emission_start_distance(),
            end_distance: default_face_emission_end_distance(),
            offset: default_face_emission_offset(),
            flicker: 0.0,
        }
    }
}

fn default_face_emission_color() -> [f32; 3] {
    [1.0, 0.28, 0.05]
}

fn default_face_emission_intensity() -> f32 {
    2.0
}

fn default_face_emission_start_distance() -> f32 {
    0.25
}

fn default_face_emission_end_distance() -> f32 {
    4.0
}

fn default_face_emission_offset() -> f32 {
    0.04
}

/// Persistent particles authored on a geometry face. The containing map keys this by effective
/// paint-surface ID, just like [`FaceEmission`], so generated Wall faces keep the effect when the
/// procedural mesh is rebuilt.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FaceParticleEmission {
    #[serde(default)]
    pub emitter: ParticleEmitterDef,
    #[serde(default = "default_face_particle_offset")]
    pub offset: f32,
    /// Optional project-palette references for the four lifetime ramp colors. A missing entry
    /// keeps using the literal color stored in `emitter.color_ramp`, which preserves authored
    /// colors when a project has no matching palette entry.
    #[serde(default)]
    pub palette_indices: [Option<u16>; 4],
}

impl Default for FaceParticleEmission {
    fn default() -> Self {
        let mut emitter = ParticleEmitterDef::default();
        emitter.direction = Vec3::unit_y();
        emitter.spread = 0.24;
        emitter.rate = 12.0;
        emitter.color = [64, 62, 58, 120];
        emitter.color_ramp = Some([
            [48, 46, 43, 118],
            [76, 74, 70, 96],
            [108, 106, 102, 52],
            [138, 138, 138, 0],
        ]);
        emitter.color_variation = 10;
        emitter.lifetime_range = (1.4, 2.8);
        emitter.radius_range = (0.03, 0.09);
        emitter.speed_range = (0.08, 0.22);
        emitter.spawn_area = [0.0; 3];
        emitter.emission_shape = ParticleEmissionShape::Surface;
        emitter.flame_base = false;
        emitter.size_curve = [0.55, 0.9, 1.25, 1.55];
        emitter.opacity_curve = [0.12, 0.82, 0.42, 0.0];
        emitter.gravity = [0.0, 0.06, 0.0];
        emitter.turbulence = 0.1;
        Self {
            emitter,
            offset: default_face_particle_offset(),
            palette_indices: [None; 4],
        }
    }
}

fn default_face_particle_offset() -> f32 {
    0.02
}

impl Default for GeometryObjectKind {
    fn default() -> Self {
        Self::Brush
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryFace {
    /// Persistent identity used by systems, such as 3D Paint, that must survive face reordering
    /// and object transforms.
    #[serde(default = "default_geometry_face_id")]
    pub id: Uuid,
    /// Logical painted surface shared by faces produced from a topology split. When absent, the
    /// mesh face ID is also the paint surface ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paint_surface_id: Option<Uuid>,
    pub indices: Vec<usize>,
    #[serde(default)]
    pub uvs: Vec<Vec2<f32>>,
    /// Dedicated object-local coordinates for 3D Paint. These never inherit material UV tiling,
    /// offsets, or rotation.
    #[serde(default)]
    pub paint_uvs: Vec<Vec2<f32>>,
    #[serde(default = "default_auto_uv")]
    pub auto_uv: bool,
    #[serde(default = "default_texture_offset")]
    pub texture_offset: Vec2<f32>,
    #[serde(default = "default_texture_scale")]
    pub texture_scale: Vec2<f32>,
    #[serde(default)]
    pub texture_rotation: f32,
    #[serde(default)]
    pub tile: Option<PixelSource>,
    #[serde(default, with = "geometry_face_tiles")]
    pub tiles: FxHashMap<(i32, i32), PixelSource>,
    #[serde(default)]
    pub surface_points: Vec<GeometrySurfacePoint>,
    #[serde(default)]
    pub surface_segments: Vec<GeometrySurfaceSegment>,
    /// Faces in the same non-zero group share vertex normals. Group zero keeps the face flat,
    /// preserving the appearance of geometry authored before smoothing groups were introduced.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub smoothing_group: u32,
}

/// Triangulate a planar 3D polygon while preserving its original winding and corner indices.
///
/// Geometry faces retain n-gons for editing, UVs, and stable paint identity. Rendering needs
/// triangles, though, and a triangle fan is only valid for convex polygons. This helper projects
/// the polygon along its dominant normal axis and uses earcut so concave authored faces are safe.
pub fn triangulate_geometry_polygon(points: &[Vec3<f32>]) -> Option<Vec<(usize, usize, usize)>> {
    if points.len() < 3
        || points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite())
    {
        return None;
    }

    // Earcut accepts collinear vertices, but consecutive duplicates can make its linked-list
    // topology ambiguous. Keep a map back to the face's original corners so UV and paint arrays
    // remain aligned with the editable n-gon.
    let mut cleaned = Vec::with_capacity(points.len());
    for (index, point) in points.iter().copied().enumerate() {
        if cleaned
            .last()
            .is_some_and(|(_, previous): &(usize, Vec3<f32>)| {
                (*previous - point).magnitude_squared() <= 1e-12
            })
        {
            continue;
        }
        cleaned.push((index, point));
    }
    if cleaned.len() >= 2
        && (cleaned[0].1 - cleaned[cleaned.len() - 1].1).magnitude_squared() <= 1e-12
    {
        cleaned.pop();
    }
    if cleaned.len() < 3 {
        return None;
    }

    // Newell's method is stable for convex and concave planar polygons and preserves winding.
    let mut polygon_normal = Vec3::<f32>::zero();
    for index in 0..cleaned.len() {
        let current = cleaned[index].1;
        let next = cleaned[(index + 1) % cleaned.len()].1;
        polygon_normal.x += (current.y - next.y) * (current.z + next.z);
        polygon_normal.y += (current.z - next.z) * (current.x + next.x);
        polygon_normal.z += (current.x - next.x) * (current.y + next.y);
    }
    if polygon_normal.magnitude_squared() <= 1e-12 {
        return None;
    }

    let abs_normal = polygon_normal.map(f32::abs);
    let project = |point: Vec3<f32>| {
        if abs_normal.x >= abs_normal.y && abs_normal.x >= abs_normal.z {
            Vec2::new(point.y, point.z)
        } else if abs_normal.y >= abs_normal.z {
            Vec2::new(point.x, point.z)
        } else {
            Vec2::new(point.x, point.y)
        }
    };
    let flattened = cleaned
        .iter()
        .flat_map(|(_, point)| {
            let projected = project(*point);
            [projected.x as f64, projected.y as f64]
        })
        .collect::<Vec<_>>();
    let earcut_indices = std::panic::catch_unwind(|| earcutr::earcut(&flattened, &[], 2))
        .ok()?
        .ok()?;

    let mut triangles = Vec::with_capacity(cleaned.len().saturating_sub(2));
    for triangle in earcut_indices.chunks_exact(3) {
        let a = cleaned.get(triangle[0])?.0;
        let mut b = cleaned.get(triangle[1])?.0;
        let mut c = cleaned.get(triangle[2])?.0;
        let triangle_normal = (points[b] - points[a]).cross(points[c] - points[a]);
        if triangle_normal.magnitude_squared() <= 1e-12 {
            continue;
        }
        if triangle_normal.dot(polygon_normal) < 0.0 {
            std::mem::swap(&mut b, &mut c);
        }
        triangles.push((a, b, c));
    }

    (!triangles.is_empty()).then_some(triangles)
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometrySurfacePointMode {
    Corner,
}

impl Default for GeometrySurfacePointMode {
    fn default() -> Self {
        Self::Corner
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometrySurfaceSegmentMode {
    Line,
    Arc,
}

impl Default for GeometrySurfaceSegmentMode {
    fn default() -> Self {
        Self::Line
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometrySurfacePoint {
    pub position: Vec3<f32>,
    #[serde(default)]
    pub mode: GeometrySurfacePointMode,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometrySurfaceSegment {
    pub start: usize,
    pub end: usize,
    #[serde(default)]
    pub mode: GeometrySurfaceSegmentMode,
    #[serde(default = "default_surface_curve_amount")]
    pub curve_amount: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometryObject {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub kind: GeometryObjectKind,
    #[serde(default)]
    pub vertices: Vec<Vec3<f32>>,
    #[serde(default)]
    pub faces: Vec<GeometryFace>,
    #[serde(default = "identity_transform")]
    pub transform: [[f32; 4]; 4],
    #[serde(default = "default_geometry_object_visible")]
    pub visible: bool,
    #[serde(default = "default_geometry_object_solid")]
    pub solid: bool,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub properties: ValueContainer,
}

impl GeometryObject {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind: GeometryObjectKind::Brush,
            vertices: Vec::new(),
            faces: Vec::new(),
            transform: identity_transform(),
            visible: true,
            solid: true,
            group: String::new(),
            tags: Vec::new(),
            properties: ValueContainer::default(),
        }
    }

    pub fn box_(name: impl Into<String>, center: Vec3<f32>, size: Vec3<f32>) -> Self {
        let half = size * 0.5;
        let p = |x: f32, y: f32, z: f32| center + Vec3::new(x * half.x, y * half.y, z * half.z);
        let mut object = Self::new(name);
        object.vertices = vec![
            p(-1.0, -1.0, -1.0),
            p(1.0, -1.0, -1.0),
            p(1.0, 1.0, -1.0),
            p(-1.0, 1.0, -1.0),
            p(-1.0, -1.0, 1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(-1.0, 1.0, 1.0),
        ];

        object.faces = vec![
            face(vec![0, 1, 2, 3]), // front
            face(vec![5, 4, 7, 6]), // back
            face(vec![4, 0, 3, 7]), // left
            face(vec![1, 5, 6, 2]), // right
            face(vec![3, 2, 6, 7]), // top
            face(vec![4, 5, 1, 0]), // bottom
        ];
        object.ensure_face_paint_data();
        object
    }

    pub fn box_from_bounds(name: impl Into<String>, min: Vec3<f32>, max: Vec3<f32>) -> Self {
        let mut object = Self::new(name);
        object.vertices = vec![
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ];

        object.faces = vec![
            face(vec![0, 1, 2, 3]), // front
            face(vec![5, 4, 7, 6]), // back
            face(vec![4, 0, 3, 7]), // left
            face(vec![1, 5, 6, 2]), // right
            face(vec![3, 2, 6, 7]), // top
            face(vec![4, 5, 1, 0]), // bottom
        ];
        object.ensure_face_paint_data();
        object
    }

    /// Create a cuboid with a true geometric corner radius. The surface is
    /// built from shared vertices so smoothing groups remain continuous across
    /// planar faces, cylindrical edges, and spherical corners.
    pub fn rounded_box_from_bounds(
        name: impl Into<String>,
        min: Vec3<f32>,
        max: Vec3<f32>,
        radius: f32,
        segments: usize,
        smooth: bool,
    ) -> Self {
        let center = (min + max) * 0.5;
        let half = (max - min).map(f32::abs) * 0.5;
        let radius = radius.max(0.0).min(half.x.min(half.y).min(half.z));
        if radius <= 1e-5 {
            return Self::box_from_bounds(name, center - half, center + half);
        }

        let inner = (half - Vec3::broadcast(radius)).map(|value| value.max(0.0));
        let samples = [
            rounded_axis_samples(half.x, inner.x, segments),
            rounded_axis_samples(half.y, inner.y, segments),
            rounded_axis_samples(half.z, inner.z, segments),
        ];
        let mut object = Self::new(name);
        let mut vertex_map = FxHashMap::<[i32; 3], usize>::default();

        for (fixed_axis, u_axis, v_axis) in [(0usize, 1usize, 2usize), (1, 2, 0), (2, 0, 1)] {
            for sign in [-1.0f32, 1.0] {
                let expected = component_axis(fixed_axis) * sign;
                let mut grid = vec![vec![0usize; samples[u_axis].len()]; samples[v_axis].len()];
                for (v_index, v) in samples[v_axis].iter().copied().enumerate() {
                    for (u_index, u) in samples[u_axis].iter().copied().enumerate() {
                        let mut source = Vec3::zero();
                        set_component(&mut source, fixed_axis, component(half, fixed_axis) * sign);
                        set_component(&mut source, u_axis, u);
                        set_component(&mut source, v_axis, v);
                        let local = rounded_box_surface_point(source, inner, radius);
                        let world = center + local;
                        let key = [
                            (world.x * 1_000_000.0).round() as i32,
                            (world.y * 1_000_000.0).round() as i32,
                            (world.z * 1_000_000.0).round() as i32,
                        ];
                        let vertex_index = *vertex_map.entry(key).or_insert_with(|| {
                            let index = object.vertices.len();
                            object.vertices.push(world);
                            index
                        });
                        grid[v_index][u_index] = vertex_index;
                    }
                }
                for v_index in 0..grid.len().saturating_sub(1) {
                    for u_index in 0..grid[v_index].len().saturating_sub(1) {
                        let mut indices = vec![
                            grid[v_index][u_index],
                            grid[v_index][u_index + 1],
                            grid[v_index + 1][u_index + 1],
                            grid[v_index + 1][u_index],
                        ];
                        let normal = (object.vertices[indices[1]] - object.vertices[indices[0]])
                            .cross(object.vertices[indices[2]] - object.vertices[indices[0]]);
                        if normal.dot(expected) < 0.0 {
                            indices.reverse();
                        }
                        let mut surface = face(indices);
                        surface.smoothing_group = if smooth { 1 } else { 0 };
                        object.faces.push(surface);
                    }
                }
            }
        }
        object.ensure_face_paint_data();
        object
    }

    /// Create a vertical circular or elliptical cylinder bounded by `min` and
    /// `max`. Caps remain flat while side faces can share smooth normals.
    pub fn cylinder_from_bounds(
        name: impl Into<String>,
        min: Vec3<f32>,
        max: Vec3<f32>,
        segments: usize,
        smooth: bool,
    ) -> Self {
        let segments = segments.max(3);
        let center = (min + max) * 0.5;
        let radius_x = ((max.x - min.x).abs() * 0.5).max(0.0001);
        let radius_z = ((max.z - min.z).abs() * 0.5).max(0.0001);
        let bottom = min.y.min(max.y);
        let top = min.y.max(max.y);
        let mut object = Self::new(name);
        object.vertices.reserve(segments * 2);
        for y in [bottom, top] {
            for segment in 0..segments {
                let angle = std::f32::consts::TAU * segment as f32 / segments as f32;
                object.vertices.push(Vec3::new(
                    center.x + radius_x * angle.cos(),
                    y,
                    center.z + radius_z * angle.sin(),
                ));
            }
        }
        for segment in 0..segments {
            let next = (segment + 1) % segments;
            let mut surface = face(vec![segment, next, next + segments, segment + segments]);
            surface.smoothing_group = if smooth { 1 } else { 0 };
            object.faces.push(surface);
        }
        object
            .faces
            .push(face((0..segments).rev().collect::<Vec<_>>()));
        object
            .faces
            .push(face((segments..segments * 2).collect::<Vec<_>>()));
        object.ensure_face_paint_data();
        object
    }

    /// Replace only this object's mesh while retaining the closest authored
    /// face source and automatic texture transform for every generated face. This lets
    /// parametric edits change topology without unexpectedly returning a
    /// painted object to the checker material.
    pub fn replace_mesh_preserving_face_sources(&mut self, mut replacement: Self) {
        let source_faces = self
            .faces
            .iter()
            .filter_map(|face| {
                geometry_face_normal(&self.vertices, face).map(|normal| {
                    (
                        normal,
                        face.tile.clone(),
                        face.texture_offset,
                        face.texture_scale,
                        face.texture_rotation,
                    )
                })
            })
            .collect::<Vec<_>>();

        for face in &mut replacement.faces {
            let Some(normal) = geometry_face_normal(&replacement.vertices, face) else {
                continue;
            };
            let Some((_, tile, texture_offset, texture_scale, texture_rotation)) = source_faces
                .iter()
                .max_by(|left, right| left.0.dot(normal).total_cmp(&right.0.dot(normal)))
            else {
                continue;
            };
            face.tile = tile.clone();
            face.texture_offset = *texture_offset;
            face.texture_scale = *texture_scale;
            face.texture_rotation = *texture_rotation;
        }

        self.vertices = replacement.vertices;
        self.faces = replacement.faces;
        self.ensure_face_paint_data();
    }

    pub fn bbox(&self) -> Option<BBox> {
        let mut min = Vec2::new(f32::INFINITY, f32::INFINITY);
        let mut max = Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        let mut found = false;

        for vertex in &self.vertices {
            let world = self.transform_point(*vertex);
            if !world.x.is_finite() || !world.z.is_finite() {
                continue;
            }
            min.x = min.x.min(world.x);
            min.y = min.y.min(world.z);
            max.x = max.x.max(world.x);
            max.y = max.y.max(world.z);
            found = true;
        }

        found.then(|| BBox::new(min, max))
    }

    pub fn transform_point(&self, point: Vec3<f32>) -> Vec3<f32> {
        let m = &self.transform;
        Vec3::new(
            point.x * m[0][0] + point.y * m[1][0] + point.z * m[2][0] + m[3][0],
            point.x * m[0][1] + point.y * m[1][1] + point.z * m[2][1] + m[3][1],
            point.x * m[0][2] + point.y * m[1][2] + point.z * m[2][2] + m[3][2],
        )
    }

    /// Ensure every face has persistent identity and object-local paint coordinates.
    ///
    /// This is intentionally explicit instead of being part of rendering: once a face can be
    /// painted, its coordinates must be serialized and remain unchanged by object transforms.
    pub fn ensure_face_paint_data(&mut self) -> bool {
        let vertices = &self.vertices;
        let mut changed = false;
        let mut face_ids = HashSet::with_capacity(self.faces.len());
        for face in &mut self.faces {
            if face.id.is_nil() || !face_ids.insert(face.id) {
                face.id = Uuid::new_v4();
                face_ids.insert(face.id);
                changed = true;
            }
            if face.paint_uvs.len() != face.indices.len() {
                let local_points = face
                    .indices
                    .iter()
                    .filter_map(|index| vertices.get(*index).copied())
                    .collect::<Vec<_>>();
                if local_points.len() == face.indices.len() {
                    face.paint_uvs = geometry_face_paint_uvs(&local_points);
                    changed = true;
                }
            }
        }
        changed
    }
}

/// Generate a stable, material-independent projection in object-local space.
pub fn geometry_face_paint_uvs(points: &[Vec3<f32>]) -> Vec<Vec2<f32>> {
    if points.len() < 3 {
        return vec![Vec2::zero(); points.len()];
    }
    let mut normal = Vec3::<f32>::zero();
    for index in 1..points.len() - 1 {
        normal += (points[index] - points[0]).cross(points[index + 1] - points[0]);
    }
    let abs = Vec3::new(normal.x.abs(), normal.y.abs(), normal.z.abs());
    points
        .iter()
        .map(|point| {
            if abs.y >= abs.x && abs.y >= abs.z {
                Vec2::new(point.x, point.z)
            } else if abs.x >= abs.z {
                Vec2::new(point.z, point.y)
            } else {
                Vec2::new(point.x, point.y)
            }
        })
        .collect()
}

pub fn geometry_face_effective_paint_surface_id(face: &GeometryFace) -> Uuid {
    face.paint_surface_id.unwrap_or(face.id)
}

/// Transfer a source face's paint coordinates to replacement vertices. The barycentric lookup
/// intentionally ignores displacement along the source normal, so an extruded cap retains the
/// exact coordinates of the face it continues.
pub fn remap_geometry_face_paint_uvs(
    vertices: &[Vec3<f32>],
    source: &GeometryFace,
    replacement_indices: &[usize],
) -> Vec<Vec2<f32>> {
    let source_points = source
        .indices
        .iter()
        .filter_map(|index| vertices.get(*index).copied())
        .collect::<Vec<_>>();
    let replacement_points = replacement_indices
        .iter()
        .filter_map(|index| vertices.get(*index).copied())
        .collect::<Vec<_>>();
    if source_points.len() != source.indices.len()
        || replacement_points.len() != replacement_indices.len()
        || source_points.len() < 3
    {
        return geometry_face_paint_uvs(&replacement_points);
    }
    let source_uvs = if source.paint_uvs.len() == source_points.len() {
        source.paint_uvs.clone()
    } else {
        geometry_face_paint_uvs(&source_points)
    };
    let fallback_uvs = geometry_face_paint_uvs(&replacement_points);

    replacement_points
        .iter()
        .enumerate()
        .map(|(replacement_index, point)| {
            for index in 1..source_points.len() - 1 {
                let a = source_points[0];
                let b = source_points[index];
                let c = source_points[index + 1];
                let v0 = b - a;
                let v1 = c - a;
                let v2 = *point - a;
                let d00 = v0.dot(v0);
                let d01 = v0.dot(v1);
                let d11 = v1.dot(v1);
                let d20 = v2.dot(v0);
                let d21 = v2.dot(v1);
                let denominator = d00 * d11 - d01 * d01;
                if denominator.abs() <= 1e-8 {
                    continue;
                }
                let v = (d11 * d20 - d01 * d21) / denominator;
                let w = (d00 * d21 - d01 * d20) / denominator;
                let u = 1.0 - v - w;
                if u >= -1e-4 && v >= -1e-4 && w >= -1e-4 {
                    return source_uvs[0] * u + source_uvs[index] * v + source_uvs[index + 1] * w;
                }
            }
            fallback_uvs
                .get(replacement_index)
                .copied()
                .unwrap_or_else(Vec2::zero)
        })
        .collect()
}

fn component(value: Vec3<f32>, axis: usize) -> f32 {
    match axis {
        0 => value.x,
        1 => value.y,
        _ => value.z,
    }
}

fn set_component(value: &mut Vec3<f32>, axis: usize, component: f32) {
    match axis {
        0 => value.x = component,
        1 => value.y = component,
        _ => value.z = component,
    }
}

fn component_axis(axis: usize) -> Vec3<f32> {
    match axis {
        0 => Vec3::unit_x(),
        1 => Vec3::unit_y(),
        _ => Vec3::unit_z(),
    }
}

fn rounded_axis_samples(half: f32, inner: f32, segments: usize) -> Vec<f32> {
    let segments = segments.max(1);
    let radius = (half - inner).max(0.0);
    let mut samples = Vec::with_capacity(segments * 2 + 2);
    for index in 0..=segments {
        samples.push(-half + radius * index as f32 / segments as f32);
    }
    if inner > 1e-5 {
        samples.push(inner);
    }
    for index in 1..=segments {
        samples.push(inner + radius * index as f32 / segments as f32);
    }
    samples.dedup_by(|a, b| (*a - *b).abs() <= 1e-6);
    samples
}

fn rounded_box_surface_point(source: Vec3<f32>, inner: Vec3<f32>, radius: f32) -> Vec3<f32> {
    let core = Vec3::new(
        source.x.clamp(-inner.x, inner.x),
        source.y.clamp(-inner.y, inner.y),
        source.z.clamp(-inner.z, inner.z),
    );
    let offset = source - core;
    core + offset.try_normalized().unwrap_or_else(Vec3::unit_y) * radius
}

fn geometry_face_normal(vertices: &[Vec3<f32>], face: &GeometryFace) -> Option<Vec3<f32>> {
    let first = *vertices.get(*face.indices.first()?)?;
    let mut normal = Vec3::zero();
    for index in 1..face.indices.len().saturating_sub(1) {
        normal += (*vertices.get(face.indices[index])? - first)
            .cross(*vertices.get(face.indices[index + 1])? - first);
    }
    normal.try_normalized()
}

pub fn identity_transform() -> [[f32; 4]; 4] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

fn face(indices: Vec<usize>) -> GeometryFace {
    GeometryFace {
        id: Uuid::new_v4(),
        paint_surface_id: None,
        indices,
        uvs: vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ],
        paint_uvs: Vec::new(),
        auto_uv: true,
        texture_offset: default_texture_offset(),
        texture_scale: default_texture_scale(),
        texture_rotation: 0.0,
        tile: None,
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
        smoothing_group: 0,
    }
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn default_geometry_face_id() -> Uuid {
    Uuid::new_v4()
}

fn default_auto_uv() -> bool {
    true
}

fn default_texture_offset() -> Vec2<f32> {
    Vec2::zero()
}

fn default_texture_scale() -> Vec2<f32> {
    Vec2::broadcast(1.0)
}

fn default_geometry_object_visible() -> bool {
    true
}

fn default_geometry_object_solid() -> bool {
    true
}

fn default_surface_curve_amount() -> f32 {
    0.35
}

mod geometry_face_tiles {
    use super::*;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(
        tiles: &FxHashMap<(i32, i32), PixelSource>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        vectorize::serialize(tiles, serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<FxHashMap<(i32, i32), PixelSource>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum TileCells {
            Vector(Vec<((i32, i32), PixelSource)>),
            LegacyMap(FxHashMap<String, PixelSource>),
        }

        match TileCells::deserialize(deserializer)? {
            TileCells::Vector(entries) => Ok(entries.into_iter().collect()),
            TileCells::LegacyMap(entries) => {
                let mut tiles = FxHashMap::default();
                for (key, source) in entries {
                    let Some(coord) = parse_legacy_key(&key) else {
                        return Err(D::Error::custom(format!(
                            "invalid geometry face tile cell key `{key}`"
                        )));
                    };
                    tiles.insert(coord, source);
                }
                Ok(tiles)
            }
        }
    }

    fn parse_legacy_key(key: &str) -> Option<(i32, i32)> {
        let trimmed = key
            .trim()
            .trim_start_matches('(')
            .trim_start_matches('[')
            .trim_end_matches(')')
            .trim_end_matches(']');
        let (x, y) = trimmed.split_once(',')?;
        Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle_area_xz(points: &[Vec3<f32>], triangle: (usize, usize, usize)) -> f32 {
        let a = points[triangle.0];
        let b = points[triangle.1];
        let c = points[triangle.2];
        ((b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x)).abs() * 0.5
    }

    #[test]
    fn concave_geometry_polygon_triangulates_without_filling_the_notch() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 0.0),
            Vec3::new(3.0, 0.0, 3.0),
            Vec3::new(2.0, 0.0, 3.0),
            Vec3::new(2.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(1.0, 0.0, 3.0),
            Vec3::new(0.0, 0.0, 3.0),
        ];

        let triangles = triangulate_geometry_polygon(&points).expect("concave face triangulates");
        let area = triangles
            .iter()
            .copied()
            .map(|triangle| triangle_area_xz(&points, triangle))
            .sum::<f32>();

        assert_eq!(triangles.len(), points.len() - 2);
        assert!(
            (area - 7.0).abs() < 1e-5,
            "triangles cover only the U-shaped polygon"
        );
    }

    #[test]
    fn geometry_polygon_triangulation_preserves_winding() {
        let clockwise = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        let counter_clockwise = clockwise.iter().copied().rev().collect::<Vec<_>>();

        for points in [clockwise, counter_clockwise] {
            let expected = (points[1] - points[0]).cross(points[2] - points[0]);
            for (a, b, c) in triangulate_geometry_polygon(&points).unwrap() {
                let actual = (points[b] - points[a]).cross(points[c] - points[a]);
                assert!(actual.dot(expected) > 0.0);
            }
        }
    }

    #[test]
    fn geometry_polygon_triangulation_maps_past_duplicate_corners() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(0.0, 0.0, 2.0),
        ];

        let triangles = triangulate_geometry_polygon(&points).unwrap();
        assert_eq!(triangles.len(), 2);
        assert!(
            triangles
                .iter()
                .all(|(a, b, c)| { [*a, *b, *c].into_iter().all(|index| index < points.len()) })
        );
    }

    #[test]
    fn degenerate_geometry_polygon_is_rejected() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
        ];
        assert!(triangulate_geometry_polygon(&points).is_none());
    }

    #[test]
    fn redundant_collinear_corner_does_not_reject_a_valid_polygon() {
        let points = vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 2.0),
            Vec3::new(0.0, 0.0, 2.0),
        ];
        let triangles = triangulate_geometry_polygon(&points).unwrap();
        let area = triangles
            .iter()
            .copied()
            .map(|triangle| triangle_area_xz(&points, triangle))
            .sum::<f32>();
        assert!((area - 4.0).abs() < 1e-5);
    }

    #[test]
    fn geometry_face_tile_cells_serialize_to_toml() {
        let tile_id = Uuid::new_v4();
        let mut face = face(vec![0, 1, 2, 3]);
        face.tiles.insert((2, -1), PixelSource::TileId(tile_id));

        let serialized = toml::to_string(&face).expect("face tile overrides serialize");
        let restored: GeometryFace =
            toml::from_str(&serialized).expect("face tile overrides deserialize");

        assert_eq!(
            restored.tiles.get(&(2, -1)),
            Some(&PixelSource::TileId(tile_id))
        );
    }

    #[test]
    fn geometry_face_legacy_empty_tile_cells_deserialize_from_json() {
        let json = r#"{
            "indices": [0, 1, 2, 3],
            "uvs": [],
            "auto_uv": true,
            "tile": null,
            "tiles": {},
            "surface_points": [],
            "surface_segments": []
        }"#;

        let restored: GeometryFace =
            serde_json::from_str(json).expect("legacy empty tile map deserializes");

        assert!(restored.tiles.is_empty());
        assert!(!restored.id.is_nil());
        assert!(restored.paint_uvs.is_empty());
    }

    #[test]
    fn geometry_face_legacy_string_tile_cells_deserialize_from_json() {
        let tile_id = Uuid::new_v4();
        let json = format!(
            r#"{{
                "indices": [0, 1, 2, 3],
                "uvs": [],
                "auto_uv": true,
                "tile": null,
                "tiles": {{
                    "(2, -1)": {{ "TileId": "{tile_id}" }}
                }},
                "surface_points": [],
                "surface_segments": []
            }}"#
        );

        let restored: GeometryFace =
            serde_json::from_str(&json).expect("legacy string tile map deserializes");

        assert_eq!(
            restored.tiles.get(&(2, -1)),
            Some(&PixelSource::TileId(tile_id))
        );
    }

    #[test]
    fn face_paint_data_is_object_local_and_survives_object_transform() {
        let mut object = GeometryObject::box_from_bounds(
            "Painted box",
            Vec3::new(-1.0, 0.0, -2.0),
            Vec3::new(2.0, 3.0, 4.0),
        );
        let face_ids = object.faces.iter().map(|face| face.id).collect::<Vec<_>>();
        let paint_uvs = object
            .faces
            .iter()
            .map(|face| face.paint_uvs.clone())
            .collect::<Vec<_>>();

        object.transform[0][0] = 0.0;
        object.transform[0][2] = -1.0;
        object.transform[2][0] = 1.0;
        object.transform[2][2] = 0.0;
        object.transform[3][0] = 7.0;
        object.transform[3][1] = 2.0;
        object.transform[3][2] = -3.0;

        assert!(!object.ensure_face_paint_data());
        assert_eq!(
            object.faces.iter().map(|face| face.id).collect::<Vec<_>>(),
            face_ids
        );
        assert_eq!(
            object
                .faces
                .iter()
                .map(|face| face.paint_uvs.clone())
                .collect::<Vec<_>>(),
            paint_uvs
        );
    }

    #[test]
    fn duplicate_face_ids_are_repaired_before_painting() {
        let mut object = GeometryObject::box_("Box", Vec3::zero(), Vec3::one());
        object.faces[1].id = object.faces[0].id;

        assert!(object.ensure_face_paint_data());
        let unique = object
            .faces
            .iter()
            .map(|face| face.id)
            .collect::<HashSet<_>>();
        assert_eq!(unique.len(), object.faces.len());
    }
}
