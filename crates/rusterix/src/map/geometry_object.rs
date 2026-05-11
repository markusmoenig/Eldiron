use crate::{BBox, PixelSource};
use serde::{Deserialize, Serialize};
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
    pub indices: Vec<usize>,
    #[serde(default)]
    pub uvs: Vec<Vec2<f32>>,
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
        indices,
        uvs: vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ],
        auto_uv: true,
        texture_offset: default_texture_offset(),
        texture_scale: default_texture_scale(),
        texture_rotation: 0.0,
        tile: None,
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
    }
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
}
