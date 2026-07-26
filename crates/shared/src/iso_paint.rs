use crate::iso_paint_brush::{self, IsoPaintBrushSample};
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

fn default_brush_shape() -> String {
    "solid".to_string()
}

fn default_material() -> String {
    "default".to_string()
}

fn default_finish() -> String {
    "natural".to_string()
}

fn default_material_id() -> u8 {
    0
}

fn default_material_mode() -> String {
    "coat".to_string()
}

fn default_clip() -> String {
    "surface".to_string()
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

fn default_palette_indices() -> Vec<u16> {
    Vec::new()
}

fn default_palette_colors() -> Vec<[u8; 4]> {
    Vec::new()
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

fn default_stamp_density() -> f32 {
    0.6
}

fn default_stamp_size_jitter() -> f32 {
    0.25
}

fn default_stamp_rotation_jitter() -> f32 {
    1.0
}

fn default_stamp_variant() -> String {
    "wildflowers".to_string()
}

fn default_revision() -> u64 {
    0
}

fn default_order() -> u64 {
    0
}

pub const ISO_PAINT_BAKED_CHUNK_SIZE: i32 = 64;
/// Paint coordinates are generated from stable surface space, independent of a material's UVs.
pub const ISO_PAINT_BAKED_PIXELS_PER_UV: f32 = 32.0;
pub const ISO_PAINT_NO_SURFACE_DEPTH: f32 = -1.0;
pub const ISO_PAINT_BAKE_VERSION: u8 = 23;
/// UI brush size is measured in the old painter's diameter units. Two paint texels per size
/// unit matches the visible cursor diameter at the current surface-coordinate density.
const ISO_PAINT_UV_BRUSH_TEXELS_PER_SIZE: f32 = 2.0;

fn validated_brush_transform(transform: Option<[f32; 4]>) -> Option<[f32; 4]> {
    let transform = transform?;
    if transform.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let determinant = transform[0] * transform[3] - transform[1] * transform[2];
    if !determinant.is_finite() || determinant.abs() <= 1e-6 {
        return None;
    }
    Some(transform)
}

fn brush_local_offset(transform: [f32; 4], offset: [f32; 2]) -> [f32; 2] {
    let determinant = transform[0] * transform[3] - transform[1] * transform[2];
    [
        (transform[3] * offset[0] - transform[1] * offset[1]) / determinant,
        (-transform[2] * offset[0] + transform[0] * offset[1]) / determinant,
    ]
}

fn interpolated_brush_transform(
    a: Option<[f32; 4]>,
    b: Option<[f32; 4]>,
    t: f32,
) -> Option<[f32; 4]> {
    match (validated_brush_transform(a), validated_brush_transform(b)) {
        (Some(a), Some(b)) => validated_brush_transform(Some([
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ])),
        (Some(transform), None) | (None, Some(transform)) => Some(transform),
        (None, None) => None,
    }
}

fn deserialize_surface_depth<'de, D>(deserializer: D) -> Result<Vec<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = <Option<Vec<Option<f32>>> as serde::Deserialize>::deserialize(deserializer)?;
    Ok(values
        .unwrap_or_default()
        .into_iter()
        .map(|depth| match depth {
            Some(depth) if depth.is_finite() && depth >= 0.0 => depth,
            _ => ISO_PAINT_NO_SURFACE_DEPTH,
        })
        .collect())
}

/// Stable reference to the rendered 3D surface under a paint point.
///
/// Combined with the surface UV, this anchors paint to durable scene geometry rather than a
/// particular camera projection.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
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

impl IsoPaintOwner {
    pub fn same_paint_object(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::GeometryObject(a), Self::GeometryObject(b)) => a == b,
            (Self::Sector(a), Self::Sector(b)) => a == b,
            (Self::Terrain { .. }, Self::Terrain { .. }) => true,
            (Self::Character(a), Self::Character(b)) => a == b,
            (Self::Item(a), Self::Item(b)) => a == b,
            (Self::Triangle(a), Self::Triangle(b)) => a == b,
            _ => self == other,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintPoint {
    pub screen: [i32; 2],
    pub world: Option<[f32; 3]>,
    #[serde(default)]
    pub surface_uv: Option<[f32; 2]>,
    #[serde(default)]
    pub paint_geo: Option<[u32; 4]>,
    #[serde(default)]
    pub surface_normal: Option<[f32; 3]>,
    #[serde(default)]
    pub camera_scale: Option<f32>,
    #[serde(default)]
    pub viewport_size: Option<[i32; 2]>,
    /// Transform from a screen-round brush into stable surface coordinates.
    /// Stored per point so rebaking keeps the footprint authored from that camera angle.
    #[serde(default)]
    pub brush_transform: Option<[f32; 4]>,
    pub owner: Option<IsoPaintOwner>,
}

impl IsoPaintPoint {
    pub fn new(screen: [i32; 2], world: Option<Vec3<f32>>, owner: Option<IsoPaintOwner>) -> Self {
        Self {
            screen,
            world: world.map(|p| [p.x, p.y, p.z]),
            surface_uv: None,
            paint_geo: None,
            surface_normal: None,
            camera_scale: None,
            viewport_size: None,
            brush_transform: None,
            owner,
        }
    }

    pub fn with_surface_uv(mut self, surface_uv: Option<Vec2<f32>>) -> Self {
        self.surface_uv = surface_uv.map(|uv| [uv.x, uv.y]);
        self
    }

    pub fn with_paint_geo(mut self, paint_geo: Option<[u32; 4]>) -> Self {
        self.paint_geo = paint_geo;
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

    pub fn with_viewport_size(mut self, viewport_size: Option<[i32; 2]>) -> Self {
        self.viewport_size = viewport_size;
        self
    }

    pub fn with_brush_transform(mut self, brush_transform: Option<[f32; 4]>) -> Self {
        self.brush_transform = brush_transform;
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintStamp {
    pub id: Uuid,
    #[serde(default = "default_order")]
    pub order: u64,
    pub kind: String,
    pub screen: [i32; 2],
    pub world: Option<[f32; 3]>,
    #[serde(default)]
    pub surface_uv: Option<[f32; 2]>,
    #[serde(default)]
    pub paint_geo: Option<[u32; 4]>,
    #[serde(default)]
    pub surface_normal: Option<[f32; 3]>,
    pub owner: Option<IsoPaintOwner>,
    pub sort_depth: f32,
    pub size: f32,
    #[serde(default)]
    pub camera_scale: Option<f32>,
    #[serde(default)]
    pub viewport_size: Option<[i32; 2]>,
    #[serde(default = "default_stamp_variant")]
    pub variant: String,
    pub rotation: f32,
    pub variation: u32,
    #[serde(default = "default_material_id")]
    pub material_id: u8,
    pub color: [u8; 4],
    #[serde(default = "default_palette_indices")]
    pub palette_indices: Vec<u16>,
    #[serde(default = "default_palette_colors")]
    pub palette_colors: Vec<[u8; 4]>,
    pub opacity: f32,
    pub screen_bounds: [i32; 4],
}

impl IsoPaintStamp {
    pub fn new(
        kind: String,
        point: IsoPaintPoint,
        color: [u8; 4],
        palette_indices: Vec<u16>,
        palette_colors: Vec<[u8; 4]>,
        material_id: u8,
        size: f32,
        opacity: f32,
        variant: String,
        size_jitter: f32,
        rotation_jitter: f32,
    ) -> Self {
        let variation = Self::variation_seed(point.screen, point.world);
        let size_jitter = size_jitter.clamp(0.0, 1.0);
        let size_noise = ((variation >> 16) & 0xff) as f32 / 255.0;
        let size_scale = 1.0 + (size_noise - 0.5) * 2.0 * size_jitter;
        let size = (size * size_scale).max(0.01);
        let radius = (size * 5.0).round().max(5.0) as i32;
        let height = (size * 13.0).round().max(13.0) as i32;
        let rotation = ((variation & 0xffff) as f32 / 65535.0 - 0.5)
            * std::f32::consts::TAU
            * rotation_jitter.clamp(0.0, 1.0);
        Self {
            id: Uuid::new_v4(),
            order: 0,
            kind,
            screen: point.screen,
            world: point.world,
            surface_uv: point.surface_uv,
            paint_geo: point.paint_geo,
            surface_normal: point.surface_normal,
            owner: point.owner,
            sort_depth: point.screen[1] as f32,
            size,
            camera_scale: point.camera_scale,
            viewport_size: point.viewport_size,
            variant: if variant.is_empty() {
                default_stamp_variant()
            } else {
                variant
            },
            rotation,
            variation,
            material_id,
            color,
            palette_indices,
            palette_colors,
            opacity: opacity.clamp(0.0, 1.0),
            screen_bounds: [
                point.screen[0] - radius,
                point.screen[1] - height,
                point.screen[0] + radius,
                point.screen[1] + radius,
            ],
        }
    }

    fn variation_seed(screen: [i32; 2], world: Option<[f32; 3]>) -> u32 {
        let mut value = (screen[0] as u32).wrapping_mul(0x9e37_79b9)
            ^ (screen[1] as u32).wrapping_mul(0x85eb_ca6b);
        if let Some(world) = world {
            for component in world {
                value ^= component.to_bits().wrapping_mul(0xc2b2_ae35);
                value = value.rotate_left(13);
            }
        }
        value ^= value >> 16;
        value = value.wrapping_mul(0x7feb_352d);
        value ^ (value >> 15)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintStroke {
    pub id: Uuid,
    #[serde(default = "default_order")]
    pub order: u64,
    pub operation: String,
    pub brush: String,
    #[serde(default = "default_brush_shape")]
    pub brush_shape: String,
    pub material: String,
    pub finish: String,
    #[serde(default = "default_material_id")]
    pub material_id: u8,
    #[serde(default = "default_material_mode")]
    pub material_mode: String,
    #[serde(default = "default_clip")]
    pub clip: String,
    #[serde(default = "default_color")]
    pub color: [u8; 4],
    #[serde(default = "default_palette_indices")]
    pub palette_indices: Vec<u16>,
    #[serde(default = "default_palette_colors")]
    pub palette_colors: Vec<[u8; 4]>,
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
        brush_shape: String,
        material: String,
        finish: String,
        material_id: u8,
        material_mode: String,
        clip: String,
        color: [u8; 4],
        palette_indices: Vec<u16>,
        palette_colors: Vec<[u8; 4]>,
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
            order: 0,
            operation,
            brush,
            brush_shape,
            material,
            finish,
            material_id,
            material_mode,
            clip,
            color,
            palette_indices,
            palette_colors,
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
    #[serde(default)]
    pub stamp_revision: u64,
    pub strokes: Vec<IsoPaintStroke>,
    #[serde(default)]
    pub stamps: Vec<IsoPaintStamp>,
}

impl IsoPaintChunk {
    pub fn new(origin: [i32; 2]) -> Self {
        Self {
            origin,
            revision: 0,
            stamp_revision: 0,
            strokes: Vec::new(),
            stamps: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintBakedChunk {
    pub owner: IsoPaintOwner,
    pub paint_geo: [u32; 4],
    pub origin: [i32; 2],
    #[serde(default = "default_revision")]
    pub revision: u64,
    pub color_rgba: Vec<u8>,
    pub material_rgba: Vec<u8>,
}

impl IsoPaintBakedChunk {
    pub fn new(owner: IsoPaintOwner, paint_geo: [u32; 4], origin: [i32; 2]) -> Self {
        let len = ISO_PAINT_BAKED_CHUNK_SIZE as usize * ISO_PAINT_BAKED_CHUNK_SIZE as usize * 4;
        let mut material_rgba = vec![0_u8; len];
        for pixel in material_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[254, 0, 0, 0]);
        }
        Self {
            owner,
            paint_geo,
            origin,
            revision: 0,
            color_rgba: vec![0_u8; len],
            material_rgba,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsoPaintScreenChunk {
    pub origin: [i32; 2],
    /// The last paint operation incorporated into this baked chunk. Screen chunks created from
    /// different camera states can overlap after reprojection, so this keeps their composition
    /// deterministic once live strokes have been discarded.
    #[serde(default)]
    pub paint_order: u64,
    #[serde(default)]
    pub screen_anchor: Option<[i32; 2]>,
    #[serde(default)]
    pub world_anchor: Option<[f32; 3]>,
    #[serde(default)]
    pub camera_scale: Option<f32>,
    #[serde(default)]
    pub viewport_size: Option<[i32; 2]>,
    #[serde(default)]
    pub source_camera_key: Option<u64>,
    #[serde(default)]
    pub clip_owner: Option<IsoPaintOwner>,
    #[serde(default)]
    pub replace_color: bool,
    #[serde(default = "default_revision")]
    pub revision: u64,
    pub color_rgba: Vec<u8>,
    pub material_rgba: Vec<u8>,
    #[serde(default, deserialize_with = "deserialize_surface_depth")]
    pub surface_depth: Vec<f32>,
    #[serde(default)]
    pub surface_anchor_depth: Option<f32>,
}

impl IsoPaintScreenChunk {
    pub fn new(origin: [i32; 2], size: i32) -> Self {
        let size = size.max(1) as usize;
        let len = size * size * 4;
        let mut material_rgba = vec![0_u8; len];
        for pixel in material_rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[254, 0, 0, 0]);
        }
        Self {
            origin,
            paint_order: 0,
            screen_anchor: None,
            world_anchor: None,
            camera_scale: None,
            viewport_size: None,
            source_camera_key: None,
            clip_owner: None,
            replace_color: false,
            revision: 0,
            color_rgba: vec![0_u8; len],
            material_rgba,
            surface_depth: vec![ISO_PAINT_NO_SURFACE_DEPTH; size * size],
            surface_anchor_depth: None,
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
    /// Legacy camera-space chunks are intentionally not loaded or saved by 3D Paint.
    #[serde(skip)]
    pub screen_chunks: IndexMap<String, IsoPaintScreenChunk>,
    #[serde(default)]
    pub baked_chunks: IndexMap<String, IsoPaintBakedChunk>,
    /// Bumped when the UV bake algorithm changes. Earlier experimental bakes are discarded;
    /// screen-space paint has intentionally no migration path.
    #[serde(default)]
    pub baked_version: u8,
    #[serde(skip)]
    pub surface_commit_strokes: Vec<Uuid>,
    #[serde(default = "default_operation")]
    pub active_operation: String,
    #[serde(default = "default_brush")]
    pub active_brush: String,
    #[serde(default = "default_brush_shape")]
    pub active_brush_shape: String,
    #[serde(default = "default_material")]
    pub active_material: String,
    #[serde(default = "default_finish")]
    pub active_finish: String,
    #[serde(default = "default_material_id")]
    pub active_material_id: u8,
    #[serde(default = "default_material_mode")]
    pub active_material_mode: String,
    #[serde(default = "default_clip")]
    pub active_clip: String,
    #[serde(default = "default_color")]
    pub active_color: [u8; 4],
    #[serde(default = "default_palette_indices")]
    pub active_palette_indices: Vec<u16>,
    #[serde(default = "default_palette_colors")]
    pub active_palette_colors: Vec<[u8; 4]>,
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
    #[serde(default = "default_stamp_density")]
    pub active_stamp_density: f32,
    #[serde(default = "default_stamp_size_jitter")]
    pub active_stamp_size_jitter: f32,
    #[serde(default = "default_stamp_rotation_jitter")]
    pub active_stamp_rotation_jitter: f32,
    #[serde(default = "default_stamp_variant")]
    pub active_stamp_variant: String,
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
            screen_chunks: IndexMap::default(),
            baked_chunks: IndexMap::default(),
            baked_version: ISO_PAINT_BAKE_VERSION,
            surface_commit_strokes: Vec::new(),
            active_operation: default_operation(),
            active_brush: default_brush(),
            active_brush_shape: default_brush_shape(),
            active_material: default_material(),
            active_finish: default_finish(),
            active_material_id: default_material_id(),
            active_material_mode: default_material_mode(),
            active_clip: default_clip(),
            active_color: default_color(),
            active_palette_indices: default_palette_indices(),
            active_palette_colors: default_palette_colors(),
            active_pattern_kind: default_pattern_kind(),
            active_pattern_scale: default_pattern_scale(),
            active_pattern_mortar: default_pattern_mortar(),
            active_pattern_detail: default_pattern_detail(),
            active_pattern_variation: default_pattern_variation(),
            active_stamp_density: default_stamp_density(),
            active_stamp_size_jitter: default_stamp_size_jitter(),
            active_stamp_rotation_jitter: default_stamp_rotation_jitter(),
            active_stamp_variant: default_stamp_variant(),
            active_size: default_size(),
            active_opacity: default_opacity(),
        }
    }
}

impl IsoPaintLayer {
    fn point_brush_transform(&self, point: &IsoPaintPoint) -> [f32; 4] {
        if self.active_brush == "material" {
            [1.0, 0.0, 0.0, 1.0]
        } else {
            validated_brush_transform(point.brush_transform).unwrap_or([1.0, 0.0, 0.0, 1.0])
        }
    }

    /// `object` was the persisted key before the UI correctly called this Surface. Keep it as a
    /// surface clip while rebuilding recently-authored direct-paint strokes.
    fn is_surface_clip(clip: &str) -> bool {
        matches!(clip, "surface" | "object")
    }

    fn baked_chunk_key(paint_geo: [u32; 4], origin: [i32; 2]) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            paint_geo[0], paint_geo[1], paint_geo[2], paint_geo[3], origin[0], origin[1]
        )
    }

    fn baked_chunk_origin_for_uv_pixel(pixel: [i32; 2]) -> [i32; 2] {
        [
            pixel[0].div_euclid(ISO_PAINT_BAKED_CHUNK_SIZE) * ISO_PAINT_BAKED_CHUNK_SIZE,
            pixel[1].div_euclid(ISO_PAINT_BAKED_CHUNK_SIZE) * ISO_PAINT_BAKED_CHUNK_SIZE,
        ]
    }

    fn baked_material_pixel(&self, coverage: u8) -> [u8; 4] {
        let mode = if self.active_material_mode == "replace" {
            // Zero means Coat. Replace stores the selected opacity plus one, leaving the A
            // channel for spatial brush coverage. The renderer can therefore route a globally
            // translucent Replace through the alpha pass without confusing it with the soft edge
            // of an otherwise opaque stroke.
            let replace_opacity =
                ((self.active_opacity.clamp(0.0, 1.0) * 254.0).round() as u8).min(254);
            replace_opacity.saturating_add(1).max(1)
        } else {
            0
        };
        [254, self.active_material_id, mode, coverage]
    }

    /// The original paint compositor treats Coat as a layer, not repeated normal-alpha draws.
    /// A continuous stroke produces many overlapping dabs; regular "over" compositing makes a
    /// 20% coat silently approach 100% opacity. Keep the strongest coverage and only blend its
    /// colour within that fixed layer, matching the former screen-space painter.
    fn coat_baked_color_pixel(dst: &mut [u8], src: [u8; 4]) {
        let src_a = src[3] as u16;
        if src_a == 0 {
            return;
        }
        let dst_a = dst[3] as u16;
        if dst_a == 0 || src_a >= dst_a {
            dst.copy_from_slice(&src);
            return;
        }
        let keep_a = dst_a - src_a;
        for channel in 0..3 {
            dst[channel] =
                ((src[channel] as u16 * src_a + dst[channel] as u16 * keep_a) / dst_a) as u8;
        }
        // Deliberately do not add alpha here: re-sampling a coat must not make it denser.
    }

    fn write_baked_pixel(
        &mut self,
        owner: &IsoPaintOwner,
        paint_geo: [u32; 4],
        uv_pixel: [i32; 2],
        mut color: [u8; 4],
        coverage: u8,
    ) {
        let origin = Self::baked_chunk_origin_for_uv_pixel(uv_pixel);
        let key = Self::baked_chunk_key(paint_geo, origin);
        let local_x = uv_pixel[0] - origin[0];
        let local_y = uv_pixel[1] - origin[1];
        if local_x < 0
            || local_y < 0
            || local_x >= ISO_PAINT_BAKED_CHUNK_SIZE
            || local_y >= ISO_PAINT_BAKED_CHUNK_SIZE
        {
            return;
        }
        let index = (local_y as usize * ISO_PAINT_BAKED_CHUNK_SIZE as usize + local_x as usize) * 4;
        let opacity = self.active_opacity.clamp(0.0, 1.0);
        let alpha = ((coverage as f32 / 255.0) * opacity * 255.0).round() as u8;
        // Replace needs a spatial mask even at opacity zero so it can make the surface fully
        // transparent. Coat continues to weight material coverage by its selected opacity.
        let material_coverage = if self.active_material_mode == "replace" {
            coverage
        } else {
            alpha
        };
        let material_pixel = self.baked_material_pixel(material_coverage);
        color[3] = alpha;
        let erase = self.active_operation == "erase";
        let chunk = self
            .baked_chunks
            .entry(key)
            .or_insert_with(|| IsoPaintBakedChunk::new(owner.clone(), paint_geo, origin));
        if index + 3 >= chunk.color_rgba.len() || index + 3 >= chunk.material_rgba.len() {
            return;
        }
        if erase {
            let clear = alpha;
            chunk.color_rgba[index + 3] = chunk.color_rgba[index + 3].saturating_sub(clear);
            chunk.material_rgba[index + 3] = chunk.material_rgba[index + 3].saturating_sub(clear);
        } else {
            if self.active_material_mode == "replace" {
                if alpha == 0 {
                    // A zero-opacity Replace is an intentional transparent cut, including when it
                    // is painted over an older non-zero Replace.
                    chunk.color_rgba[index..index + 4]
                        .copy_from_slice(&[color[0], color[1], color[2], 0]);
                } else {
                    // Repeated dabs within a continuous stroke must not build selected opacity
                    // toward one. Keep the strongest coverage while allowing an equally strong
                    // new dab to replace RGB, matching the stable layer behavior used by Coat.
                    Self::coat_baked_color_pixel(&mut chunk.color_rgba[index..index + 4], color);
                }
            } else {
                Self::coat_baked_color_pixel(&mut chunk.color_rgba[index..index + 4], color);
            }
            let existing = chunk.material_rgba[index + 3] as u16;
            let src = material_pixel[3] as u16;
            let out_alpha = if self.active_material_mode == "replace" {
                (src + (existing * (255 - src)) / 255).min(255) as u8
            } else {
                existing.max(src) as u8
            };
            chunk.material_rgba[index..index + 4].copy_from_slice(&[
                material_pixel[0],
                material_pixel[1],
                material_pixel[2],
                out_alpha,
            ]);
        }
        chunk.revision = chunk.revision.wrapping_add(1);
    }

    fn paint_baked_pattern_color(&self, x: i32, y: i32) -> [u8; 4] {
        let fallback = self.active_color;
        let scale = self.active_pattern_scale.clamp(0.25, 4.0);
        // Pattern dimensions are independent of surface-coordinate density. Keep the raw
        // preview cell size so world-space paint does not turn each brick into a wall.
        let brick_w = if self.active_pattern_kind == "tile" {
            12.0
        } else {
            17.0
        } * scale;
        let brick_h = if self.active_pattern_kind == "tile" {
            12.0
        } else {
            8.5
        } * scale;
        let mortar = (brick_w.min(brick_h) * self.active_pattern_mortar.clamp(0.0, 0.4))
            .min(brick_w.min(brick_h) * 0.45);
        // Pattern dimensions are expressed in painter texels. This keeps the visible brick
        // density aligned with the brush preview instead of stretching one brick across most
        // of a UV island.
        let px = x as f32;
        let py = y as f32;
        let row = (py / brick_h).floor() as i32;
        let offset = if self.active_pattern_kind != "tile" && row & 1 != 0 {
            brick_w * 0.5
        } else {
            0.0
        };
        let local_x = (px + offset).rem_euclid(brick_w);
        let local_y = py.rem_euclid(brick_h);
        if local_x < mortar || local_y < mortar {
            return [fallback[0], fallback[1], fallback[2], 0];
        }
        let col = ((px + offset) / brick_w).floor() as i32;
        let hash = |nx: i32, ny: i32, salt: i32| -> f32 {
            let mut n = nx
                .wrapping_mul(374_761_393)
                .wrapping_add(ny.wrapping_mul(668_265_263))
                .wrapping_add(salt.wrapping_mul(2_147_483_647));
            n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
            ((n ^ (n >> 16)) & 0xffff) as f32 / 65_535.0
        };

        // Keep the original brick treatment: the selected primary color defines the whole wall,
        // while deterministic tonal variation gives individual bricks character. Choosing a
        // different palette slot for every cell made the baked 3D pattern much more patchy than
        // the former screen-space painter.
        let edge_distance = local_x
            .min(local_y)
            .min(brick_w - local_x)
            .min(brick_h - local_y);
        let detail = self.active_pattern_detail.clamp(0.0, 1.0);
        let edge_wear = if edge_distance < mortar + 1.6 {
            1.0 - 0.12 * detail + hash(col, row, 31) * 0.06 * detail
        } else {
            1.0
        };
        let brick_variation =
            1.0 + (hash(col, row, 11) - 0.5) * 0.44 * self.active_pattern_variation.clamp(0.0, 1.0);
        let grain =
            1.0 + (hash(x, y, col.wrapping_mul(19) ^ row.wrapping_mul(23)) - 0.5) * 0.20 * detail;
        let hairline = if (local_y - mortar).abs() < 1.0 || (local_x - mortar).abs() < 0.8 {
            1.0 - 0.07 * detail
        } else {
            1.0
        };
        let shade = brick_variation * grain * edge_wear * hairline;
        [
            (fallback[0] as f32 * shade).clamp(0.0, 255.0) as u8,
            (fallback[1] as f32 * shade).clamp(0.0, 255.0) as u8,
            (fallback[2] as f32 * shade).clamp(0.0, 255.0) as u8,
            fallback[3],
        ]
    }

    fn paint_baked_at_point(&mut self, point: &IsoPaintPoint, clip_geo: Option<[u32; 4]>) {
        let (Some(owner), Some(uv), Some(paint_geo)) =
            (point.owner.as_ref(), point.surface_uv, point.paint_geo)
        else {
            return;
        };
        if Self::is_surface_clip(&self.active_clip)
            && clip_geo.is_some_and(|clip_geo| clip_geo != paint_geo)
        {
            return;
        }
        let center = [
            (uv[0] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
            (uv[1] * ISO_PAINT_BAKED_PIXELS_PER_UV).round() as i32,
        ];
        let radius = (self.active_size * ISO_PAINT_UV_BRUSH_TEXELS_PER_SIZE)
            .round()
            .clamp(1.0, 96.0) as i32;
        let transform = self.point_brush_transform(point);
        let extent_x = (radius as f32
            * (transform[0] * transform[0] + transform[1] * transform[1]).sqrt())
        .ceil() as i32;
        let extent_y = (radius as f32
            * (transform[2] * transform[2] + transform[3] * transform[3]).sqrt())
        .ceil() as i32;
        let mut seed = paint_geo[0] ^ paint_geo[1].rotate_left(7) ^ paint_geo[2].rotate_left(13);
        seed ^= paint_geo[3].rotate_left(19);
        seed ^= (center[0] as u32).wrapping_mul(0x9e37_79b9);
        seed ^= (center[1] as u32).wrapping_mul(0x85eb_ca6b);
        let brush = self.active_brush.clone();
        let shape = self.active_brush_shape.clone();
        let color = self.active_color;
        let palette = self.active_palette_colors.clone();
        let sample = IsoPaintBrushSample {
            brush: &brush,
            shape: &shape,
            color,
            palette: &palette,
            opacity: 1.0,
            radius,
            seed,
        };
        for y in center[1] - extent_y..=center[1] + extent_y {
            for x in center[0] - extent_x..=center[0] + extent_x {
                let local =
                    brush_local_offset(transform, [(x - center[0]) as f32, (y - center[1]) as f32]);
                let Some(mut color) = iso_paint_brush::sample_pixel(
                    &sample,
                    local[0].round() as i32,
                    local[1].round() as i32,
                ) else {
                    continue;
                };
                if brush == "brick" {
                    let pattern_color = self.paint_baked_pattern_color(x, y);
                    if pattern_color[3] == 0 {
                        continue;
                    }
                    color[0..3].copy_from_slice(&pattern_color[0..3]);
                }
                self.write_baked_pixel(owner, paint_geo, [x, y], color, color[3]);
            }
        }
    }

    fn paint_baked_segment(
        &mut self,
        a: &IsoPaintPoint,
        b: &IsoPaintPoint,
        clip_geo: Option<[u32; 4]>,
    ) {
        let (Some(uv_a), Some(uv_b), Some(paint_geo_a), Some(paint_geo_b)) =
            (a.surface_uv, b.surface_uv, a.paint_geo, b.paint_geo)
        else {
            return;
        };
        if paint_geo_a != paint_geo_b {
            self.paint_baked_at_point(b, clip_geo);
            return;
        }
        let ax = uv_a[0] * ISO_PAINT_BAKED_PIXELS_PER_UV;
        let ay = uv_a[1] * ISO_PAINT_BAKED_PIXELS_PER_UV;
        let bx = uv_b[0] * ISO_PAINT_BAKED_PIXELS_PER_UV;
        let by = uv_b[1] * ISO_PAINT_BAKED_PIXELS_PER_UV;
        let spacing_transform = if self.active_brush == "material" {
            [1.0, 0.0, 0.0, 1.0]
        } else {
            interpolated_brush_transform(a.brush_transform, b.brush_transform, 0.5)
                .unwrap_or([1.0, 0.0, 0.0, 1.0])
        };
        let local_delta = brush_local_offset(spacing_transform, [bx - ax, by - ay]);
        let distance = (local_delta[0].powi(2) + local_delta[1].powi(2)).sqrt();
        let radius = (self.active_size * ISO_PAINT_UV_BRUSH_TEXELS_PER_SIZE)
            .round()
            .max(1.0);
        let steps = (distance / (radius * 0.35).clamp(1.0, 10.0))
            .ceil()
            .max(1.0) as usize;
        for step in 0..=steps {
            let t = step as f32 / steps.max(1) as f32;
            let mut point = b.clone();
            point.surface_uv = Some([
                uv_a[0] + (uv_b[0] - uv_a[0]) * t,
                uv_a[1] + (uv_b[1] - uv_a[1]) * t,
            ]);
            point.paint_geo = Some(paint_geo_a);
            point.brush_transform =
                interpolated_brush_transform(a.brush_transform, b.brush_transform, t);
            self.paint_baked_at_point(&point, clip_geo);
        }
    }

    fn bake_stroke(&mut self, stroke: &IsoPaintStroke) {
        let saved = (
            self.active_operation.clone(),
            self.active_brush.clone(),
            self.active_brush_shape.clone(),
            self.active_material_id,
            self.active_material_mode.clone(),
            self.active_clip.clone(),
            self.active_color,
            self.active_palette_colors.clone(),
            self.active_pattern_kind.clone(),
            self.active_pattern_scale,
            self.active_pattern_mortar,
            self.active_pattern_detail,
            self.active_pattern_variation,
            self.active_size,
            self.active_opacity,
        );
        self.active_operation = stroke.operation.clone();
        self.active_brush = stroke.brush.clone();
        self.active_brush_shape = stroke.brush_shape.clone();
        self.active_material_id = stroke.material_id;
        self.active_material_mode = stroke.material_mode.clone();
        self.active_clip = if Self::is_surface_clip(&stroke.clip) {
            "surface".to_string()
        } else {
            "none".to_string()
        };
        self.active_color = stroke.color;
        self.active_palette_colors = stroke.palette_colors.clone();
        self.active_pattern_kind = stroke.pattern_kind.clone();
        self.active_pattern_scale = stroke.pattern_scale;
        self.active_pattern_mortar = stroke.pattern_mortar;
        self.active_pattern_detail = stroke.pattern_detail;
        self.active_pattern_variation = stroke.pattern_variation;
        self.active_size = stroke.size;
        self.active_opacity = stroke.opacity;

        let clip_geo = Self::is_surface_clip(&stroke.clip)
            .then(|| stroke.points.first().and_then(|point| point.paint_geo))
            .flatten();
        if let Some(first) = stroke.points.first() {
            self.paint_baked_at_point(first, clip_geo);
        }
        for points in stroke.points.windows(2) {
            self.paint_baked_segment(&points[0], &points[1], clip_geo);
        }

        (
            self.active_operation,
            self.active_brush,
            self.active_brush_shape,
            self.active_material_id,
            self.active_material_mode,
            self.active_clip,
            self.active_color,
            self.active_palette_colors,
            self.active_pattern_kind,
            self.active_pattern_scale,
            self.active_pattern_mortar,
            self.active_pattern_detail,
            self.active_pattern_variation,
            self.active_size,
            self.active_opacity,
        ) = saved;
    }

    /// Rebuild the transient surface-space bake from persistent surface strokes. Stamps are
    /// rendered separately, resolving their persistent surface coordinates against current
    /// geometry before billboard creation.
    /// Points from the discarded screen-space experiment have no surface coordinates and are
    /// therefore ignored rather than being projected with the wrong semantics.
    pub fn rebuild_baked_paint(&mut self) {
        let strokes: Vec<_> = self
            .chunks
            .values()
            .flat_map(|chunk| chunk.strokes.iter().cloned())
            .collect();
        self.baked_chunks.clear();
        for stroke in &strokes {
            self.bake_stroke(stroke);
        }
    }

    pub fn stroke_first_owner(&self, stroke_id: Uuid) -> Option<IsoPaintOwner> {
        self.chunks
            .values()
            .flat_map(|chunk| &chunk.strokes)
            .find(|stroke| stroke.id == stroke_id)
            .and_then(|stroke| stroke.points.first())
            .and_then(|point| point.owner.clone())
    }

    pub fn stroke_first_paint_geo(&self, stroke_id: Uuid) -> Option<[u32; 4]> {
        self.chunks
            .values()
            .flat_map(|chunk| &chunk.strokes)
            .find(|stroke| stroke.id == stroke_id)
            .and_then(|stroke| stroke.points.first())
            .and_then(|point| point.paint_geo)
    }

    pub fn set_active_settings(
        &mut self,
        operation: impl Into<String>,
        brush: impl Into<String>,
        brush_shape: impl Into<String>,
        material: impl Into<String>,
        finish: impl Into<String>,
        material_id: u8,
        material_mode: impl Into<String>,
        clip: impl Into<String>,
        color: [u8; 4],
        palette_indices: Vec<u16>,
        palette_colors: Vec<[u8; 4]>,
        pattern_kind: impl Into<String>,
        pattern_scale: f32,
        pattern_mortar: f32,
        pattern_detail: f32,
        pattern_variation: f32,
        stamp_density: f32,
        stamp_size_jitter: f32,
        stamp_rotation_jitter: f32,
        stamp_variant: impl Into<String>,
        size: f32,
        opacity: f32,
    ) {
        self.active_operation = operation.into();
        self.active_brush = brush.into();
        self.active_brush_shape = brush_shape.into();
        self.active_material = material.into();
        self.active_finish = finish.into();
        self.active_material_id = material_id;
        let material_mode = material_mode.into();
        self.active_material_mode = match material_mode.as_str() {
            "replace" => "replace".to_string(),
            "stamp" => "stamp".to_string(),
            _ => "coat".to_string(),
        };
        self.active_clip = clip.into();
        self.active_color = color;
        self.active_palette_indices = palette_indices;
        self.active_palette_colors = palette_colors;
        self.active_pattern_kind = pattern_kind.into();
        self.active_pattern_scale = pattern_scale.clamp(0.25, 4.0);
        self.active_pattern_mortar = pattern_mortar.clamp(0.0, 0.4);
        self.active_pattern_detail = pattern_detail.clamp(0.0, 1.0);
        self.active_pattern_variation = pattern_variation.clamp(0.0, 1.0);
        self.active_stamp_density = stamp_density.clamp(0.0, 1.0);
        self.active_stamp_size_jitter = stamp_size_jitter.clamp(0.0, 1.0);
        self.active_stamp_rotation_jitter = stamp_rotation_jitter.clamp(0.0, 1.0);
        let stamp_variant = stamp_variant.into();
        self.active_stamp_variant = if stamp_variant.is_empty() {
            default_stamp_variant()
        } else {
            stamp_variant
        };
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

    pub fn ensure_screen_chunk(&mut self, origin: [i32; 2]) -> &mut IsoPaintScreenChunk {
        let key = Self::chunk_key(origin);
        self.screen_chunks
            .entry(key)
            .or_insert_with(|| IsoPaintScreenChunk::new(origin, self.chunk_size))
    }

    fn next_paint_order(&self) -> u64 {
        self.chunks
            .values()
            .flat_map(|chunk| {
                chunk
                    .strokes
                    .iter()
                    .map(|stroke| stroke.order)
                    .chain(chunk.stamps.iter().map(|stamp| stamp.order))
            })
            .chain(self.screen_chunks.values().map(|chunk| chunk.paint_order))
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }

    pub fn begin_stroke(&mut self, first_point: IsoPaintPoint) -> Uuid {
        self.paint_baked_at_point(&first_point, first_point.paint_geo);
        let origin = self.chunk_origin_for_screen(first_point.screen);
        let key = Self::chunk_key(origin);
        let mut stroke = IsoPaintStroke::new(
            self.active_operation.clone(),
            self.active_brush.clone(),
            self.active_brush_shape.clone(),
            self.active_material.clone(),
            self.active_finish.clone(),
            self.active_material_id,
            self.active_material_mode.clone(),
            self.active_clip.clone(),
            self.active_color,
            self.active_palette_indices.clone(),
            self.active_palette_colors.clone(),
            self.active_pattern_kind.clone(),
            self.active_pattern_scale,
            self.active_pattern_mortar,
            self.active_pattern_detail,
            self.active_pattern_variation,
            self.active_size,
            self.active_opacity,
            first_point,
        );
        stroke.order = self.next_paint_order();
        let id = stroke.id;
        let chunk = self
            .chunks
            .entry(key)
            .or_insert_with(|| IsoPaintChunk::new(origin));
        chunk.revision = chunk.revision.wrapping_add(1);
        chunk.strokes.push(stroke);
        id
    }

    pub fn add_stamp(&mut self, point: IsoPaintPoint) -> Uuid {
        let origin = self.chunk_origin_for_screen(point.screen);
        let key = Self::chunk_key(origin);
        let mut stamp = IsoPaintStamp::new(
            self.active_brush.clone(),
            point,
            self.active_color,
            self.active_palette_indices.clone(),
            self.active_palette_colors.clone(),
            self.active_material_id,
            self.active_size,
            self.active_opacity,
            self.active_stamp_variant.clone(),
            self.active_stamp_size_jitter,
            self.active_stamp_rotation_jitter,
        );
        stamp.order = self.next_paint_order();
        let id = stamp.id;
        let chunk = self
            .chunks
            .entry(key)
            .or_insert_with(|| IsoPaintChunk::new(origin));
        chunk.stamp_revision = chunk.stamp_revision.wrapping_add(1);
        chunk.stamps.push(stamp);
        id
    }

    pub fn erase_stamps_near(&mut self, screen: [i32; 2], radius: f32) -> bool {
        self.erase_stamps_near_owner(screen, radius, None)
    }

    pub fn erase_stamps_near_owner(
        &mut self,
        screen: [i32; 2],
        radius: f32,
        owner_filter: Option<&IsoPaintOwner>,
    ) -> bool {
        self.erase_stamps_near_owner_kind(screen, radius, owner_filter, None)
    }

    pub fn erase_stamps_near_owner_kind(
        &mut self,
        screen: [i32; 2],
        radius: f32,
        owner_filter: Option<&IsoPaintOwner>,
        kind_filter: Option<&str>,
    ) -> bool {
        let radius = (radius * 3.0).round().max(3.0) as i32;
        let radius_sq = radius * radius;
        let mut changed = false;
        for chunk in self.chunks.values_mut() {
            let before = chunk.stamps.len();
            chunk.stamps.retain(|stamp| {
                let dx = stamp.screen[0] - screen[0];
                let dy = stamp.screen[1] - screen[1];
                let near = dx * dx + dy * dy <= radius_sq;
                let owner_matches = owner_filter.is_none_or(|owner_filter| {
                    stamp
                        .owner
                        .as_ref()
                        .is_some_and(|owner| owner_filter.same_paint_object(owner))
                });
                let kind_matches = kind_filter.is_none_or(|kind_filter| {
                    stamp.kind == kind_filter
                        || (kind_filter == "grass" && stamp.kind == "grass_stamp")
                });
                !(near && owner_matches && kind_matches)
            });
            if chunk.stamps.len() != before {
                chunk.stamp_revision = chunk.stamp_revision.wrapping_add(1);
                changed = true;
            }
        }
        if changed {
            self.rebuild_baked_paint();
        }
        changed
    }

    pub fn append_point(&mut self, stroke_id: Uuid, point: IsoPaintPoint) -> bool {
        let previous_point = self
            .chunks
            .values()
            .flat_map(|chunk| &chunk.strokes)
            .find(|stroke| stroke.id == stroke_id)
            .and_then(|stroke| stroke.points.last())
            .cloned();
        let mut baked_segment = None;
        for chunk in self.chunks.values_mut() {
            if let Some(stroke) = chunk
                .strokes
                .iter_mut()
                .find(|stroke| stroke.id == stroke_id)
            {
                if let Some(last) = stroke.points.last() {
                    let min_spacing = (stroke.size * 0.75).round().clamp(1.0, 12.0) as i32;
                    let dx = point.screen[0] - last.screen[0];
                    let dy = point.screen[1] - last.screen[1];
                    if dx * dx + dy * dy < min_spacing * min_spacing {
                        return false;
                    }
                }
                stroke.append_point(point);
                chunk.revision = chunk.revision.wrapping_add(1);
                baked_segment = previous_point.and_then(|previous_point| {
                    stroke
                        .points
                        .last()
                        .cloned()
                        .map(|current_point| (previous_point, current_point))
                });
                break;
            }
        }
        if let Some((previous_point, current_point)) = baked_segment {
            let clip_geo = self.stroke_first_paint_geo(stroke_id);
            self.paint_baked_segment(&previous_point, &current_point, clip_geo);
            return true;
        }
        false
    }

    pub fn take_stroke(&mut self, stroke_id: Uuid) -> Option<IsoPaintStroke> {
        for chunk in self.chunks.values_mut() {
            if let Some(index) = chunk
                .strokes
                .iter()
                .position(|stroke| stroke.id == stroke_id)
            {
                chunk.revision = chunk.revision.wrapping_add(1);
                return Some(chunk.strokes.remove(index));
            }
        }
        None
    }

    pub fn mark_stroke_for_screen_commit(&mut self, stroke_id: Uuid) {
        let _ = stroke_id;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn former_object_clip_key_remains_a_surface_clip() {
        assert!(IsoPaintLayer::is_surface_clip("object"));
        assert!(IsoPaintLayer::is_surface_clip("surface"));
        assert!(!IsoPaintLayer::is_surface_clip("none"));
    }

    #[test]
    fn chunk_origin_uses_floor_division_for_negative_screen_coords() {
        let layer = IsoPaintLayer::default();
        assert_eq!(layer.chunk_origin_for_screen([10, 20]), [0, 0]);
        assert_eq!(layer.chunk_origin_for_screen([-1, -1]), [-512, -512]);
    }

    #[test]
    fn screen_chunks_store_color_and_material_pixels() {
        let mut layer = IsoPaintLayer::default();
        let chunk = layer.ensure_screen_chunk([0, 0]);
        assert_eq!(chunk.color_rgba.len(), 512 * 512 * 4);
        assert_eq!(chunk.material_rgba.len(), 512 * 512 * 4);
        assert_eq!(chunk.surface_depth.len(), 512 * 512);
        assert_eq!(&chunk.color_rgba[0..4], &[0, 0, 0, 0]);
        assert_eq!(&chunk.material_rgba[0..4], &[254, 0, 0, 0]);
    }

    #[test]
    fn baked_bricks_use_primary_color_instead_of_random_palette_slots() {
        let mut layer = IsoPaintLayer::default();
        layer.active_color = [144, 96, 64, 255];
        layer.active_palette_colors = vec![
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 0, 255, 255],
        ];

        let with_palette = layer.paint_baked_pattern_color(4, 4);
        layer.active_palette_colors.clear();
        let without_palette = layer.paint_baked_pattern_color(4, 4);

        assert_eq!(with_palette, without_palette);
        assert!(with_palette[0] > with_palette[1]);
        assert!(with_palette[1] > with_palette[2]);
        assert_eq!(with_palette[3], 255);
    }

    #[test]
    fn baked_brush_transform_compensates_for_projected_surface_axes() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "crack".to_string();
        layer.active_brush_shape = "scratch".to_string();
        layer.active_size = 4.0;
        let point = IsoPaintPoint::new([32, 32], None, Some(IsoPaintOwner::Sector(7)))
            .with_surface_uv(Some(Vec2::new(1.0, 1.0)))
            .with_paint_geo(Some([7, 0, 0, 1]))
            .with_brush_transform(Some([2.0, 0.0, 0.0, 0.5]));

        layer.begin_stroke(point);

        let chunk = layer.baked_chunks.values().next().unwrap();
        let mut bounds = [i32::MAX, i32::MAX, i32::MIN, i32::MIN];
        for y in 0..ISO_PAINT_BAKED_CHUNK_SIZE {
            for x in 0..ISO_PAINT_BAKED_CHUNK_SIZE {
                let index = (y * ISO_PAINT_BAKED_CHUNK_SIZE + x) as usize * 4;
                if chunk.color_rgba[index + 3] == 0 {
                    continue;
                }
                bounds[0] = bounds[0].min(x);
                bounds[1] = bounds[1].min(y);
                bounds[2] = bounds[2].max(x);
                bounds[3] = bounds[3].max(y);
            }
        }
        let width = bounds[2] - bounds[0] + 1;
        let height = bounds[3] - bounds[1] + 1;
        assert!(
            width > height * 2,
            "expected wide ellipse, got {width}x{height}"
        );
    }

    #[test]
    fn material_brush_keeps_legacy_round_footprint() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "material".to_string();
        let point = IsoPaintPoint::new([32, 32], None, Some(IsoPaintOwner::Sector(7)))
            .with_brush_transform(Some([3.0, 0.0, 0.0, 1.0]));

        assert_eq!(layer.point_brush_transform(&point), [1.0, 0.0, 0.0, 1.0]);
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

    #[test]
    fn stamps_are_stored_separately_from_strokes() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "grass".to_string();
        layer.active_material_mode = "stamp".to_string();
        layer.active_color = [35, 120, 45, 255];
        let id = layer.add_stamp(IsoPaintPoint::new(
            [20, 24],
            Some(Vec3::new(1.0, 2.0, 3.0)),
            None,
        ));
        let chunk = layer.chunks.values().next().unwrap();
        assert!(chunk.strokes.is_empty());
        assert_eq!(chunk.stamps.len(), 1);
        assert_eq!(chunk.stamps[0].id, id);
        assert_eq!(chunk.stamps[0].kind, "grass");
        assert_eq!(chunk.stamps[0].world, Some([1.0, 2.0, 3.0]));
        assert!(chunk.stamp_revision > 0);
        assert!(layer.baked_chunks.is_empty());
    }

    #[test]
    fn coat_opacity_does_not_accumulate_across_overlapping_dabs() {
        let mut layer = IsoPaintLayer::default();
        layer.active_material_mode = "coat".to_string();
        layer.active_brush = "material".to_string();
        layer.active_brush_shape = "solid".to_string();
        layer.active_opacity = 0.2;
        let point = IsoPaintPoint::new([10, 12], None, Some(IsoPaintOwner::Sector(7)))
            .with_surface_uv(Some(Vec2::new(0.25, 0.25)))
            .with_paint_geo(Some([7, 0, 0, 1]));

        layer.begin_stroke(point.clone());
        let first_alpha = layer
            .baked_chunks
            .values()
            .flat_map(|chunk| chunk.color_rgba.chunks_exact(4))
            .map(|pixel| pixel[3])
            .max()
            .unwrap_or(0);
        layer.begin_stroke(point);
        let second_alpha = layer
            .baked_chunks
            .values()
            .flat_map(|chunk| chunk.color_rgba.chunks_exact(4))
            .map(|pixel| pixel[3])
            .max()
            .unwrap_or(0);

        assert!((49..=52).contains(&first_alpha));
        assert_eq!(second_alpha, first_alpha);
    }

    #[test]
    fn rubble_stamps_keep_their_brush_kind() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "rubble".to_string();
        layer.active_material_mode = "stamp".to_string();
        layer.active_color = [96, 90, 76, 255];
        layer.add_stamp(IsoPaintPoint::new([42, 48], None, None));
        let chunk = layer.chunks.values().next().unwrap();
        assert!(chunk.strokes.is_empty());
        assert_eq!(chunk.stamps.len(), 1);
        assert_eq!(chunk.stamps[0].kind, "rubble");
        assert_eq!(chunk.stamps[0].color, [96, 90, 76, 255]);
    }

    #[test]
    fn stamps_keep_their_material_id() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "leaves".to_string();
        layer.active_material_mode = "stamp".to_string();
        layer.active_material_id = 44;
        layer.add_stamp(IsoPaintPoint::new([42, 48], None, None));
        let chunk = layer.chunks.values().next().unwrap();
        assert_eq!(chunk.stamps.len(), 1);
        assert_eq!(chunk.stamps[0].material_id, 44);
    }

    #[test]
    fn flowers_stamps_keep_their_brush_kind() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "flowers".to_string();
        layer.active_material_mode = "stamp".to_string();
        layer.active_stamp_variant = "poppies".to_string();
        layer.active_color = [75, 119, 57, 255];
        layer.active_palette_indices = vec![37, 32, 46, 47];
        layer.active_palette_colors = vec![
            [30, 80, 40, 255],
            [220, 40, 50, 255],
            [180, 28, 42, 255],
            [40, 22, 18, 255],
        ];
        layer.add_stamp(IsoPaintPoint::new([50, 56], None, None));
        let chunk = layer.chunks.values().next().unwrap();
        assert_eq!(chunk.stamps.len(), 1);
        assert_eq!(chunk.stamps[0].kind, "flowers");
        assert_eq!(chunk.stamps[0].variant, "poppies");
        assert_eq!(chunk.stamps[0].color, [75, 119, 57, 255]);
        assert_eq!(chunk.stamps[0].palette_indices, vec![37, 32, 46, 47]);
        assert_eq!(chunk.stamps[0].palette_colors[1], [220, 40, 50, 255]);
    }

    #[test]
    fn stamp_erase_can_be_filtered_to_one_owner() {
        let mut layer = IsoPaintLayer::default();
        let owner_a = IsoPaintOwner::Sector(1);
        let owner_b = IsoPaintOwner::Sector(2);
        layer.add_stamp(IsoPaintPoint::new([20, 24], None, Some(owner_a.clone())));
        layer.add_stamp(IsoPaintPoint::new([21, 25], None, Some(owner_b)));

        assert!(layer.erase_stamps_near_owner([20, 24], 1.0, Some(&owner_a)));
        let stamps: Vec<_> = layer
            .chunks
            .values()
            .flat_map(|chunk| &chunk.stamps)
            .collect();
        assert_eq!(stamps.len(), 1);
        assert_eq!(stamps[0].owner, Some(IsoPaintOwner::Sector(2)));
    }

    #[test]
    fn stamp_erase_can_be_filtered_to_one_kind() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "rubble".to_string();
        layer.add_stamp(IsoPaintPoint::new([20, 24], None, None));
        layer.active_brush = "leaves".to_string();
        layer.add_stamp(IsoPaintPoint::new([21, 25], None, None));

        assert!(layer.erase_stamps_near_owner_kind([20, 24], 1.0, None, Some("rubble")));
        let stamps: Vec<_> = layer
            .chunks
            .values()
            .flat_map(|chunk| &chunk.stamps)
            .collect();
        assert_eq!(stamps.len(), 1);
        assert_eq!(stamps[0].kind, "leaves");
    }

    #[test]
    fn stamp_jitter_settings_affect_new_stamps() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "footprints".to_string();
        layer.active_size = 4.0;
        layer.active_stamp_size_jitter = 0.0;
        layer.active_stamp_rotation_jitter = 0.0;
        layer.add_stamp(IsoPaintPoint::new([42, 48], None, None));
        let stamp = layer
            .chunks
            .values()
            .next()
            .unwrap()
            .stamps
            .first()
            .unwrap();
        assert_eq!(stamp.size, 4.0);
        assert_eq!(stamp.rotation, 0.0);
    }

    #[test]
    fn stamps_remember_placement_camera_scale() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "flowers".to_string();
        layer.active_material_mode = "stamp".to_string();
        layer.add_stamp(IsoPaintPoint::new([42, 48], None, None).with_camera_scale(Some(6.0)));
        let stamp = layer
            .chunks
            .values()
            .next()
            .unwrap()
            .stamps
            .first()
            .unwrap();
        assert_eq!(stamp.camera_scale, Some(6.0));
    }

    #[test]
    fn stamp_erase_only_removes_nearby_stamps() {
        let mut layer = IsoPaintLayer::default();
        layer.active_brush = "grass".to_string();
        layer.active_material_mode = "stamp".to_string();
        layer.add_stamp(IsoPaintPoint::new([20, 24], None, None));
        layer.add_stamp(IsoPaintPoint::new([200, 240], None, None));
        assert!(layer.erase_stamps_near([21, 25], 1.0));
        let stamps: Vec<_> = layer
            .chunks
            .values()
            .flat_map(|chunk| &chunk.stamps)
            .collect();
        assert_eq!(stamps.len(), 1);
        assert_eq!(stamps[0].screen, [200, 240]);
    }

    #[test]
    fn direct_uv_paint_is_baked_per_rendered_face() {
        let mut layer = IsoPaintLayer::default();
        let owner = IsoPaintOwner::Sector(7);
        let first = IsoPaintPoint::new([10, 12], None, Some(owner.clone()))
            .with_surface_uv(Some(Vec2::new(0.25, 0.25)))
            .with_paint_geo(Some([7, 0, 0, 1]));
        let second = IsoPaintPoint::new([20, 12], None, Some(owner))
            .with_surface_uv(Some(Vec2::new(0.25, 0.25)))
            .with_paint_geo(Some([7, 0, 0, 2]));

        layer.begin_stroke(first);
        layer.begin_stroke(second);

        assert_eq!(layer.baked_chunks.len(), 2);
        assert!(
            layer
                .baked_chunks
                .values()
                .all(|chunk| chunk.color_rgba.chunks_exact(4).any(|pixel| pixel[3] > 0))
        );
    }

    #[test]
    fn object_clip_rejects_points_on_another_owner() {
        let mut layer = IsoPaintLayer::default();
        layer.active_clip = "surface".to_string();
        let first = IsoPaintPoint::new([10, 12], None, Some(IsoPaintOwner::Sector(1)))
            .with_surface_uv(Some(Vec2::new(0.25, 0.25)))
            .with_paint_geo(Some([1, 0, 0, 1]));
        let second = IsoPaintPoint::new([30, 12], None, Some(IsoPaintOwner::Sector(2)))
            .with_surface_uv(Some(Vec2::new(0.25, 0.25)))
            .with_paint_geo(Some([2, 0, 0, 1]));

        let stroke = layer.begin_stroke(first);
        assert!(layer.append_point(stroke, second));
        assert!(
            layer
                .baked_chunks
                .values()
                .all(|chunk| chunk.paint_geo == [1, 0, 0, 1])
        );
    }

    #[test]
    fn object_clip_rejects_another_surface_of_the_same_owner() {
        let mut layer = IsoPaintLayer::default();
        layer.active_clip = "surface".to_string();
        let first = IsoPaintPoint::new([10, 12], None, Some(IsoPaintOwner::Sector(1)))
            .with_surface_uv(Some(Vec2::new(0.25, 0.25)))
            .with_paint_geo(Some([1, 0, 0, 1]));
        let second = IsoPaintPoint::new([30, 12], None, Some(IsoPaintOwner::Sector(1)))
            .with_surface_uv(Some(Vec2::new(0.25, 0.25)))
            .with_paint_geo(Some([1, 0, 0, 2]));

        let stroke = layer.begin_stroke(first);
        assert!(layer.append_point(stroke, second));
        assert!(
            layer
                .baked_chunks
                .values()
                .all(|chunk| chunk.paint_geo == [1, 0, 0, 1])
        );
    }
}
