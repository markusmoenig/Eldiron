use crate::{BBox, PixelSource, ValueContainer};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_noise: Option<GeometrySurfaceNoise>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GeometrySurfaceNoise {
    #[serde(default = "default_surface_noise_scale")]
    pub scale: f32,
    #[serde(default = "default_surface_noise_amount")]
    pub amount: f32,
    #[serde(default)]
    pub seed: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PixelSource>,
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
        surface_noise: None,
    }
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

fn default_surface_noise_scale() -> f32 {
    1.0
}

fn default_surface_noise_amount() -> f32 {
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
