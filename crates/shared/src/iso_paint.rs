use crate::prelude::*;
use theframework::prelude::{Uuid, Vec2, Vec3};

fn default_visible() -> bool {
    true
}

fn default_chunk_size() -> i32 {
    512
}

fn default_operation() -> String {
    "draw".to_string()
}

fn default_brush() -> String {
    "material".to_string()
}

fn default_material() -> String {
    "default".to_string()
}

fn default_finish() -> String {
    "natural".to_string()
}

fn default_clip() -> String {
    "object".to_string()
}

fn default_size() -> f32 {
    1.0
}

fn default_opacity() -> f32 {
    1.0
}

fn default_color() -> [u8; 4] {
    [132, 132, 128, 255]
}

fn default_pattern_kind() -> String {
    "brick".to_string()
}

fn default_pattern_scale() -> f32 {
    1.0
}

fn default_pattern_mortar() -> f32 {
    0.08
}

fn default_pattern_detail() -> f32 {
    0.65
}

fn default_pattern_variation() -> f32 {
    0.6
}

fn default_revision() -> u64 {
    0
}

/// Stable reference to the scene element under an Iso Paint point.
///
/// The paint remains authored in fixed isometric screen space. This optional
/// metadata is only for later sorting, masking, picking, and scene-aware tools.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum IsoPaintOwner {
    Unknown(u32),
    Vertex(u32),
    Linedef(u32),
    Sector(u32),
    Character(u32),
    Item(u32),
    Light(u32),
    ItemLight(u32),
    Triangle(u32),
    Terrain { x: i32, z: i32 },
    GeometryObject(Uuid),
    Hole { sector_id: u32, hole_id: u32 },
    Gizmo(u32),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintPoint {
    pub screen: [i32; 2],
    pub world: Option<[f32; 3]>,
    #[serde(default)]
    pub surface_uv: Option<[f32; 2]>,
    #[serde(default)]
    pub surface_normal: Option<[f32; 3]>,
    #[serde(default)]
    pub camera_scale: Option<f32>,
    pub owner: Option<IsoPaintOwner>,
}

impl IsoPaintPoint {
    pub fn new(screen: [i32; 2], world: Option<Vec3<f32>>, owner: Option<IsoPaintOwner>) -> Self {
        Self {
            screen,
            world: world.map(|p| [p.x, p.y, p.z]),
            surface_uv: None,
            surface_normal: None,
            camera_scale: None,
            owner,
        }
    }

    pub fn with_surface_uv(mut self, surface_uv: Option<Vec2<f32>>) -> Self {
        self.surface_uv = surface_uv.map(|uv| [uv.x, uv.y]);
        self
    }

    pub fn with_surface_normal(mut self, surface_normal: Option<Vec3<f32>>) -> Self {
        self.surface_normal = surface_normal.map(|normal| [normal.x, normal.y, normal.z]);
        self
    }

    pub fn with_camera_scale(mut self, camera_scale: Option<f32>) -> Self {
        self.camera_scale = camera_scale;
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintStroke {
    pub id: Uuid,
    pub operation: String,
    pub brush: String,
    pub material: String,
    pub finish: String,
    #[serde(default = "default_clip")]
    pub clip: String,
    #[serde(default = "default_color")]
    pub color: [u8; 4],
    #[serde(default = "default_pattern_kind")]
    pub pattern_kind: String,
    #[serde(default = "default_pattern_scale")]
    pub pattern_scale: f32,
    #[serde(default = "default_pattern_mortar")]
    pub pattern_mortar: f32,
    #[serde(default = "default_pattern_detail")]
    pub pattern_detail: f32,
    #[serde(default = "default_pattern_variation")]
    pub pattern_variation: f32,
    pub size: f32,
    pub opacity: f32,
    pub points: Vec<IsoPaintPoint>,
    pub screen_bounds: [i32; 4],
}

impl IsoPaintStroke {
    pub fn new(
        operation: String,
        brush: String,
        material: String,
        finish: String,
        clip: String,
        color: [u8; 4],
        pattern_kind: String,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
        size: f32,
        opacity: f32,
        first_point: IsoPaintPoint,
    ) -> Self {
        let screen = first_point.screen;
        Self {
            id: Uuid::new_v4(),
            operation,
            brush,
            material,
            finish,
            clip,
            color,
            pattern_kind,
            pattern_scale: pattern_scale.clamp(0.25, 4.0),
            pattern_mortar: pattern_mortar.clamp(0.0, 0.4),
            pattern_detail: pattern_detail.clamp(0.0, 1.0),
            pattern_variation: pattern_variation.clamp(0.0, 1.0),
            size: size.max(0.01),
            opacity: opacity.clamp(0.0, 1.0),
            points: vec![first_point],
            screen_bounds: [screen[0], screen[1], screen[0], screen[1]],
        }
    }

    pub fn append_point(&mut self, point: IsoPaintPoint) {
        self.screen_bounds[0] = self.screen_bounds[0].min(point.screen[0]);
        self.screen_bounds[1] = self.screen_bounds[1].min(point.screen[1]);
        self.screen_bounds[2] = self.screen_bounds[2].max(point.screen[0]);
        self.screen_bounds[3] = self.screen_bounds[3].max(point.screen[1]);
        self.points.push(point);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintChunk {
    pub origin: [i32; 2],
    #[serde(default = "default_revision")]
    pub revision: u64,
    pub strokes: Vec<IsoPaintStroke>,
}

impl IsoPaintChunk {
    pub fn new(origin: [i32; 2]) -> Self {
        Self {
            origin,
            revision: 0,
            strokes: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintLayer {
    #[serde(default = "default_visible")]
    pub visible: bool,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: i32,
    #[serde(default)]
    pub chunks: IndexMap<String, IsoPaintChunk>,
    #[serde(default = "default_operation")]
    pub active_operation: String,
    #[serde(default = "default_brush")]
    pub active_brush: String,
    #[serde(default = "default_material")]
    pub active_material: String,
    #[serde(default = "default_finish")]
    pub active_finish: String,
    #[serde(default = "default_clip")]
    pub active_clip: String,
    #[serde(default = "default_color")]
    pub active_color: [u8; 4],
    #[serde(default = "default_pattern_kind")]
    pub active_pattern_kind: String,
    #[serde(default = "default_pattern_scale")]
    pub active_pattern_scale: f32,
    #[serde(default = "default_pattern_mortar")]
    pub active_pattern_mortar: f32,
    #[serde(default = "default_pattern_detail")]
    pub active_pattern_detail: f32,
    #[serde(default = "default_pattern_variation")]
    pub active_pattern_variation: f32,
    #[serde(default = "default_size")]
    pub active_size: f32,
    #[serde(default = "default_opacity")]
    pub active_opacity: f32,
}

impl Default for IsoPaintLayer {
    fn default() -> Self {
        Self {
            visible: true,
            chunk_size: default_chunk_size(),
            chunks: IndexMap::default(),
            active_operation: default_operation(),
            active_brush: default_brush(),
            active_material: default_material(),
            active_finish: default_finish(),
            active_clip: default_clip(),
            active_color: default_color(),
            active_pattern_kind: default_pattern_kind(),
            active_pattern_scale: default_pattern_scale(),
            active_pattern_mortar: default_pattern_mortar(),
            active_pattern_detail: default_pattern_detail(),
            active_pattern_variation: default_pattern_variation(),
            active_size: default_size(),
            active_opacity: default_opacity(),
        }
    }
}

impl IsoPaintLayer {
    pub fn stroke_first_owner(&self, stroke_id: Uuid) -> Option<IsoPaintOwner> {
        self.chunks
            .values()
            .flat_map(|chunk| &chunk.strokes)
            .find(|stroke| stroke.id == stroke_id)
            .and_then(|stroke| stroke.points.first())
            .and_then(|point| point.owner.clone())
    }

    pub fn set_active_settings(
        &mut self,
        operation: impl Into<String>,
        brush: impl Into<String>,
        material: impl Into<String>,
        finish: impl Into<String>,
        clip: impl Into<String>,
        color: [u8; 4],
        pattern_kind: impl Into<String>,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
        size: f32,
        opacity: f32,
    ) {
        self.active_operation = operation.into();
        self.active_brush = brush.into();
        self.active_material = material.into();
        self.active_finish = finish.into();
        self.active_clip = clip.into();
        self.active_color = color;
        self.active_pattern_kind = pattern_kind.into();
        self.active_pattern_scale = pattern_scale.clamp(0.25, 4.0);
        self.active_pattern_mortar = pattern_mortar.clamp(0.0, 0.4);
        self.active_pattern_detail = pattern_detail.clamp(0.0, 1.0);
        self.active_pattern_variation = pattern_variation.clamp(0.0, 1.0);
        self.active_size = size.max(0.01);
        self.active_opacity = opacity.clamp(0.0, 1.0);
    }

    pub fn chunk_origin_for_screen(&self, screen: [i32; 2]) -> [i32; 2] {
        let size = self.chunk_size.max(1);
        [
            screen[0].div_euclid(size) * size,
            screen[1].div_euclid(size) * size,
        ]
    }

    pub fn chunk_key(origin: [i32; 2]) -> String {
        format!("{},{}", origin[0], origin[1])
    }

    pub fn begin_stroke(&mut self, first_point: IsoPaintPoint) -> Uuid {
        let origin = self.chunk_origin_for_screen(first_point.screen);
        let key = Self::chunk_key(origin);
        let stroke = IsoPaintStroke::new(
            self.active_operation.clone(),
            self.active_brush.clone(),
            self.active_material.clone(),
            self.active_finish.clone(),
            self.active_clip.clone(),
            self.active_color,
            self.active_pattern_kind.clone(),
            self.active_pattern_scale,
            self.active_pattern_mortar,
            self.active_pattern_detail,
            self.active_pattern_variation,
            self.active_size,
            self.active_opacity,
            first_point,
        );
        let id = stroke.id;
        let chunk = self
            .chunks
            .entry(key)
            .or_insert_with(|| IsoPaintChunk::new(origin));
        chunk.revision = chunk.revision.wrapping_add(1);
        chunk.strokes.push(stroke);
        id
    }

    pub fn append_point(&mut self, stroke_id: Uuid, point: IsoPaintPoint) -> bool {
        for chunk in self.chunks.values_mut() {
            if let Some(stroke) = chunk
                .strokes
                .iter_mut()
                .find(|stroke| stroke.id == stroke_id)
            {
                if stroke
                    .points
                    .last()
                    .is_some_and(|last| last.screen == point.screen)
                {
                    return false;
                }
                stroke.append_point(point);
                chunk.revision = chunk.revision.wrapping_add(1);
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_origin_uses_floor_division_for_negative_screen_coords() {
        let layer = IsoPaintLayer::default();
        assert_eq!(layer.chunk_origin_for_screen([10, 20]), [0, 0]);
        assert_eq!(layer.chunk_origin_for_screen([-1, -1]), [-512, -512]);
    }

    #[test]
    fn stroke_bounds_expand_when_points_are_appended() {
        let mut layer = IsoPaintLayer::default();
        let id = layer.begin_stroke(IsoPaintPoint::new([10, 12], None, None));
        let initial_revision = layer.chunks.values().next().unwrap().revision;
        assert!(layer.append_point(id, IsoPaintPoint::new([20, 4], None, None)));
        let chunk = layer.chunks.values().next().unwrap();
        assert_eq!(chunk.strokes[0].screen_bounds, [10, 4, 20, 12]);
        assert!(chunk.revision > initial_revision);
    }
}
