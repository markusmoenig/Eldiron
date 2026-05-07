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
    #[serde(default)]
    pub tile: Option<PixelSource>,
    #[serde(default)]
    pub tiles: FxHashMap<(i32, i32), PixelSource>,
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
        tile: None,
        tiles: FxHashMap::default(),
    }
}

fn default_auto_uv() -> bool {
    true
}
