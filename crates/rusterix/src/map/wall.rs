use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::Vec3;

use super::Map;
use crate::{
    BlockPropHostAttachment, BlockPropInstance, GeometryFace, GeometryObject, GeometryObjectKind,
    PixelSource, Value, identity_block_prop_transform,
};
use theframework::prelude::{FxHashMap, TheColor};

const GENERATED_WALL_TAG: &str = "eldiron_generated_wall";
const GENERATED_WALL_ID_MASK: u128 = 0x5741_4C4C_0000_0000_0000_0000_0000_0000;
const GENERATED_WALL_JUNCTION_ID_MASK: u128 = 0x4A55_4E43_5449_4F4E_0000_0000_0000_0000;
const GENERATED_WALL_FLOOR_ID_MASK: u128 = 0x464C_4F4F_525F_0000_0000_0000_0000_0000;
const GENERATED_WALL_SURFACE_ID_MASK: u128 = 0x5355_5246_4143_455F_0000_0000_0000_0000;
const WALL_SPLIT_NODE_ID_MASK: u128 = 0x5350_4C49_545F_4E4F_4445_0000_0000_0000;
const WALL_SPLIT_SPAN_ID_MASK: u128 = 0x5350_4C49_545F_5350_414E_0000_0000_0000;

/// Shared construction defaults for every span in a connected wall assembly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallStyle {
    pub height: f32,
    pub thickness: f32,
    pub brick_width: f32,
    pub brick_height: f32,
    pub mortar_gap: f32,
    pub alternating_course_offset: f32,
    pub variation_seed: u64,
    #[serde(default)]
    pub masonry: WallMasonryPattern,
    #[serde(default = "default_wall_bevel")]
    pub bevel: f32,
    #[serde(default = "default_wall_irregularity")]
    pub irregularity: f32,
    #[serde(default = "default_wall_damage")]
    pub damage: f32,
    #[serde(default = "default_wall_stone_variation")]
    pub stone_variation: f32,
    #[serde(default = "default_wall_frame_width")]
    pub frame_width: f32,
    #[serde(default = "default_wall_frame_depth")]
    pub frame_depth: f32,
    #[serde(default = "default_wall_arch_stones")]
    pub arch_stones: u16,
    #[serde(
        default = "default_wall_stone_source",
        skip_serializing_if = "Option::is_none"
    )]
    pub stone_source: Option<PixelSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stone_variants: Vec<PixelSource>,
    #[serde(
        default = "default_wall_mortar_source",
        skip_serializing_if = "Option::is_none"
    )]
    pub mortar_source: Option<PixelSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_source: Option<PixelSource>,
}

impl Default for WallStyle {
    fn default() -> Self {
        Self {
            height: 3.0,
            thickness: 0.35,
            brick_width: 0.5,
            brick_height: 0.25,
            mortar_gap: 0.0125,
            alternating_course_offset: 0.5,
            variation_seed: 0,
            masonry: WallMasonryPattern::Brick,
            bevel: default_wall_bevel(),
            irregularity: default_wall_irregularity(),
            damage: default_wall_damage(),
            stone_variation: default_wall_stone_variation(),
            frame_width: default_wall_frame_width(),
            frame_depth: default_wall_frame_depth(),
            arch_stones: default_wall_arch_stones(),
            stone_source: default_wall_stone_source(),
            stone_variants: default_wall_stone_variants(),
            mortar_source: default_wall_mortar_source(),
            frame_source: None,
        }
    }
}

/// Structural coursing, not a texture preset. Every choice changes the generated stone units,
/// their addressable removal cells, and the mortar joints between them.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WallMasonryPattern {
    #[default]
    Brick,
    StoneBlocks,
    Rubble,
}

impl WallMasonryPattern {
    pub fn label(self) -> &'static str {
        match self {
            Self::Brick => "Brick",
            Self::StoneBlocks => "Blocks",
            Self::Rubble => "Rubble",
        }
    }

    pub fn offset(self, delta: i32) -> Self {
        let index = match self {
            Self::Brick => 0,
            Self::StoneBlocks => 1,
            Self::Rubble => 2,
        };
        match (index + delta).rem_euclid(3) {
            0 => Self::Brick,
            1 => Self::StoneBlocks,
            _ => Self::Rubble,
        }
    }
}

impl WallStyle {
    pub fn primary_stone_source(&self) -> PixelSource {
        self.stone_source
            .clone()
            .or_else(default_wall_stone_source)
            .unwrap()
    }

    pub fn mortar_pixel_source(&self) -> PixelSource {
        self.mortar_source
            .clone()
            .or_else(default_wall_mortar_source)
            .unwrap()
    }

    pub fn frame_pixel_source(&self) -> PixelSource {
        self.frame_source
            .clone()
            .unwrap_or_else(|| self.primary_stone_source())
    }

    fn stone_source_for_key(&self, key: WallBrickKey) -> PixelSource {
        if self.stone_variants.is_empty()
            || wall_stone_noise(self, key, 37) > self.stone_variation.clamp(0.0, 1.0)
        {
            return self.primary_stone_source();
        }
        let index = (wall_stone_noise(self, key, 41) * self.stone_variants.len() as f32)
            .floor()
            .min(self.stone_variants.len().saturating_sub(1) as f32) as usize;
        self.stone_variants[index].clone()
    }
}

fn default_wall_bevel() -> f32 {
    0.025
}

fn default_wall_irregularity() -> f32 {
    0.12
}

fn default_wall_damage() -> f32 {
    0.04
}

fn default_wall_stone_variation() -> f32 {
    0.6
}

fn default_wall_frame_width() -> f32 {
    0.18
}

fn default_wall_frame_depth() -> f32 {
    0.025
}

fn default_wall_arch_stones() -> u16 {
    9
}

fn wall_color(r: u8, g: u8, b: u8) -> PixelSource {
    PixelSource::Color(TheColor::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        1.0,
    ))
}

fn default_wall_stone_source() -> Option<PixelSource> {
    Some(wall_color(104, 94, 80))
}

fn default_wall_stone_variants() -> Vec<PixelSource> {
    vec![wall_color(123, 108, 86), wall_color(83, 82, 78)]
}

fn default_wall_mortar_source() -> Option<PixelSource> {
    Some(wall_color(52, 49, 44))
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallNode {
    pub id: Uuid,
    pub position: Vec3<f32>,
}

impl WallNode {
    pub fn new(position: Vec3<f32>) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
        }
    }
}

/// One edge in a wall graph. Both endpoints refer to nodes owned by the same assembly.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallSpan {
    pub id: Uuid,
    pub start_node: Uuid,
    pub end_node: Uuid,
    /// Signed perpendicular displacement of the quadratic path control point. Zero is straight.
    #[serde(default)]
    pub curve_offset: f32,
    #[serde(default = "default_wall_curve_segments")]
    pub curve_segments: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_override: Option<WallStyle>,
    #[serde(default)]
    pub openings: Vec<WallOpening>,
    #[serde(default)]
    pub removed_bricks: Vec<WallBrickKey>,
}

/// One directed wall span in the boundary of a fitted area surface.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct WallSurfaceEdge {
    pub span_id: Uuid,
    pub forward: bool,
}

/// A horizontal surface fitted to one bounded face of the connected wall graph.
/// Keeping the directed span boundary instead of baked vertices lets the surface follow later
/// wall-node moves and curve edits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallAreaSurface {
    pub id: Uuid,
    pub boundary: Vec<WallSurfaceEdge>,
    pub elevation: f32,
    pub thickness: f32,
    pub clearance: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PixelSource>,
}

impl WallAreaSurface {
    pub fn new(boundary: Vec<WallSurfaceEdge>) -> Self {
        Self {
            id: Uuid::new_v4(),
            boundary,
            elevation: 0.25,
            thickness: 0.08,
            clearance: 0.015,
            source: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WallAreaSurfacePreview {
    pub assembly_id: Uuid,
    pub surface: WallAreaSurface,
    /// A transient, topologically welded copy used when visually touching wall runs have not yet
    /// been joined at endpoint-on-span contacts. Committing the preview promotes this graph.
    pub wall_assemblies: Option<Vec<WallAssembly>>,
    /// Host attachments must be promoted together with a split wall graph. Otherwise an attachment
    /// on the second half of a split span keeps the old distance and jumps to the new span end.
    pub block_prop_instances: Option<Vec<BlockPropInstance>>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WallOpeningShape {
    #[default]
    Rectangular,
    Arch,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallOpening {
    pub id: Uuid,
    /// Distance along the span from its start node.
    pub center: f32,
    pub bottom: f32,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub shape: WallOpeningShape,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch_radius: Option<f32>,
    #[serde(default)]
    pub frame: WallOpeningFrame,
}

impl WallOpening {
    pub fn effective_arch_radius(&self) -> f32 {
        self.arch_radius
            .unwrap_or(self.width * 0.5)
            .clamp(0.001, (self.width * 0.5).min(self.height).max(0.001))
    }
}

/// Per-opening framing overrides. Empty overrides inherit the span's wall style, allowing a door,
/// window, and arch in the same wall to use different fitted surrounds without duplicating the
/// complete wall style.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallOpeningFrame {
    #[serde(default = "default_wall_opening_frame_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub surround: WallOpeningSurround,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch_stones: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PixelSource>,
}

impl Default for WallOpeningFrame {
    fn default() -> Self {
        Self {
            enabled: true,
            surround: WallOpeningSurround::Blocks,
            width: None,
            depth: None,
            arch_stones: None,
            source: None,
        }
    }
}

impl WallOpeningFrame {
    pub fn width(&self, style: &WallStyle) -> f32 {
        if self.enabled && self.surround != WallOpeningSurround::None {
            self.width.unwrap_or(style.frame_width).max(0.0)
        } else {
            0.0
        }
    }

    pub fn depth(&self, style: &WallStyle) -> f32 {
        self.depth.unwrap_or(style.frame_depth).max(0.0)
    }

    pub fn arch_stones(&self, style: &WallStyle) -> u16 {
        self.arch_stones.unwrap_or(style.arch_stones).clamp(3, 32)
    }

    pub fn pixel_source(&self, style: &WallStyle) -> PixelSource {
        self.source
            .clone()
            .unwrap_or_else(|| style.frame_pixel_source())
    }
}

/// The hole profile and its construction surround are intentionally independent. This permits a
/// plain arched cut, light architectural trim, or a structural row of dungeon masonry around the
/// same rectangular or arched opening.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum WallOpeningSurround {
    None,
    Trim,
    #[default]
    Blocks,
}

impl WallOpeningSurround {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Trim => "Trim",
            Self::Blocks => "Blocks",
        }
    }
}

/// Ordered ownership of a point on the wall construction plane. Geometry generators ask this
/// compositor before emitting faces, so masonry, surrounds, and voids never occupy the same region.
/// Future recess, grille, and decorative-cutout layers can extend this contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WallGeometryLayer {
    Masonry,
    OpeningSurround(Uuid),
    Void(Uuid),
}

fn default_wall_opening_frame_enabled() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct WallBrickKey {
    pub course: i32,
    pub index: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallBrickPreview {
    pub assembly_id: Uuid,
    pub span_id: Uuid,
    pub key: WallBrickKey,
    pub remove: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WallOpeningPreview {
    pub assembly_id: Uuid,
    pub span_id: Uuid,
    pub start: vek::Vec2<f32>,
    pub end: vek::Vec2<f32>,
    pub shape: WallOpeningShape,
    pub surround: WallOpeningSurround,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WallJunctionKind {
    End,
    Continuation,
    Corner,
    Tee,
    Crossing,
    MultiWay,
}

fn default_wall_curve_segments() -> u16 {
    12
}

#[derive(Clone, Debug)]
struct WallPath {
    points: Vec<Vec3<f32>>,
    cumulative: Vec<f32>,
    length: f32,
}

impl WallPath {
    fn new(start: Vec3<f32>, end: Vec3<f32>, span: &WallSpan) -> Option<Self> {
        let chord = Vec3::new(end.x - start.x, 0.0, end.z - start.z);
        let chord_length = chord.magnitude();
        if chord_length <= 1e-5 {
            return None;
        }
        let segments = if span.curve_offset.abs() <= 1e-5 {
            1
        } else {
            span.curve_segments.clamp(2, 64) as usize
        };
        let perpendicular = Vec3::new(-chord.z / chord_length, 0.0, chord.x / chord_length);
        let control = (start + end) * 0.5 + perpendicular * span.curve_offset;
        let points = (0..=segments)
            .map(|index| {
                let t = index as f32 / segments as f32;
                if segments == 1 {
                    start + (end - start) * t
                } else {
                    start * (1.0 - t).powi(2) + control * (2.0 * (1.0 - t) * t) + end * t.powi(2)
                }
            })
            .collect::<Vec<_>>();
        let mut cumulative = Vec::with_capacity(points.len());
        cumulative.push(0.0);
        for pair in points.windows(2) {
            let delta = Vec3::new(pair[1].x - pair[0].x, 0.0, pair[1].z - pair[0].z);
            cumulative.push(cumulative.last().copied().unwrap_or(0.0) + delta.magnitude());
        }
        let length = *cumulative.last()?;
        (length > 1e-5).then_some(Self {
            points,
            cumulative,
            length,
        })
    }

    fn point_at(&self, distance: f32) -> Vec3<f32> {
        let distance = distance.clamp(0.0, self.length);
        let index = self
            .cumulative
            .partition_point(|value| *value < distance)
            .saturating_sub(1)
            .min(self.points.len().saturating_sub(2));
        let start_distance = self.cumulative[index];
        let segment_length = self.cumulative[index + 1] - start_distance;
        let t = if segment_length <= 1e-6 {
            0.0
        } else {
            (distance - start_distance) / segment_length
        };
        self.points[index] + (self.points[index + 1] - self.points[index]) * t
    }

    fn project(&self, point: Vec3<f32>) -> Option<(f32, Vec3<f32>)> {
        let point_2d = vek::Vec2::new(point.x, point.z);
        self.points
            .windows(2)
            .enumerate()
            .filter_map(|(index, pair)| {
                let start = vek::Vec2::new(pair[0].x, pair[0].z);
                let end = vek::Vec2::new(pair[1].x, pair[1].z);
                let direction = end - start;
                let length_squared = direction.magnitude_squared();
                if length_squared <= 1e-10 {
                    return None;
                }
                let t = ((point_2d - start).dot(direction) / length_squared).clamp(0.0, 1.0);
                let projected_2d = start + direction * t;
                let projected = pair[0] + (pair[1] - pair[0]) * t;
                let along = self.cumulative[index]
                    + (self.cumulative[index + 1] - self.cumulative[index]) * t;
                Some((
                    (point_2d - projected_2d).magnitude_squared(),
                    along,
                    projected,
                ))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, along, projected)| (along, projected))
    }
}

/// Editable source for a connected wall network. Generated meshes are derived output and do not
/// replace this graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallAssembly {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<WallNode>,
    #[serde(default)]
    pub spans: Vec<WallSpan>,
    #[serde(default)]
    pub style: WallStyle,
    #[serde(default)]
    pub auto_floor: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_source: Option<PixelSource>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub area_surfaces: Vec<WallAreaSurface>,
}

impl WallAssembly {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            nodes: Vec::new(),
            spans: Vec::new(),
            style: WallStyle::default(),
            auto_floor: false,
            floor_source: None,
            area_surfaces: Vec::new(),
        }
    }

    pub fn floor_pixel_source(&self) -> PixelSource {
        self.floor_source
            .clone()
            .unwrap_or_else(|| self.style.primary_stone_source())
    }

    pub fn node(&self, node_id: Uuid) -> Option<&WallNode> {
        self.nodes.iter().find(|node| node.id == node_id)
    }

    pub fn node_mut(&mut self, node_id: Uuid) -> Option<&mut WallNode> {
        self.nodes.iter_mut().find(|node| node.id == node_id)
    }

    /// Moves one graph node while keeping every connected opening valid for its new span length.
    /// All incident spans derive their geometry from the shared node, so corners and tees update as
    /// one operation rather than leaving detached wall ends.
    pub fn set_node_position(&mut self, node_id: Uuid, position: Vec3<f32>) -> Result<(), String> {
        let connected = self
            .connected_spans(node_id)
            .map(|span| {
                let other = if span.start_node == node_id {
                    span.end_node
                } else {
                    span.start_node
                };
                (span.id, other)
            })
            .collect::<Vec<_>>();
        if connected.iter().any(|(_, other)| {
            self.node(*other).is_some_and(|node| {
                (node.position - position).magnitude_squared() <= 0.05_f32.powi(2)
            })
        }) {
            return Err("A wall node cannot be moved onto its connected endpoint.".to_string());
        }
        self.node_mut(node_id)
            .ok_or_else(|| "The wall node no longer exists.".to_string())?
            .position = position;

        for (span_id, _) in connected {
            let Some(length) = self.span_length(span_id) else {
                continue;
            };
            let Some(span) = self.span_mut(span_id) else {
                continue;
            };
            for opening in &mut span.openings {
                opening.width = opening.width.min(length).max(0.05);
                let half_width = opening.width * 0.5;
                opening.center = opening.center.clamp(half_width, length - half_width);
            }
        }
        Ok(())
    }

    pub fn span(&self, span_id: Uuid) -> Option<&WallSpan> {
        self.spans.iter().find(|span| span.id == span_id)
    }

    pub fn span_mut(&mut self, span_id: Uuid) -> Option<&mut WallSpan> {
        self.spans.iter_mut().find(|span| span.id == span_id)
    }

    pub fn opening(&self, span_id: Uuid, opening_id: Uuid) -> Option<&WallOpening> {
        self.span(span_id)?
            .openings
            .iter()
            .find(|opening| opening.id == opening_id)
    }

    pub fn opening_mut(&mut self, span_id: Uuid, opening_id: Uuid) -> Option<&mut WallOpening> {
        self.span_mut(span_id)?
            .openings
            .iter_mut()
            .find(|opening| opening.id == opening_id)
    }

    pub fn area_surface(&self, surface_id: Uuid) -> Option<&WallAreaSurface> {
        self.area_surfaces
            .iter()
            .find(|surface| surface.id == surface_id)
    }

    pub fn area_surface_mut(&mut self, surface_id: Uuid) -> Option<&mut WallAreaSurface> {
        self.area_surfaces
            .iter_mut()
            .find(|surface| surface.id == surface_id)
    }

    pub fn remove_area_surface(&mut self, surface_id: Uuid) -> bool {
        let previous_len = self.area_surfaces.len();
        self.area_surfaces
            .retain(|surface| surface.id != surface_id);
        self.area_surfaces.len() != previous_len
    }

    pub fn opening_at(&self, span_id: Uuid, coordinates: vek::Vec2<f32>) -> Option<Uuid> {
        match self.span_geometry_layer_at(span_id, coordinates)? {
            WallGeometryLayer::OpeningSurround(opening_id)
            | WallGeometryLayer::Void(opening_id) => Some(opening_id),
            WallGeometryLayer::Masonry => None,
        }
    }

    pub fn remove_opening(&mut self, span_id: Uuid, opening_id: Uuid) -> bool {
        let Some(span) = self.span_mut(span_id) else {
            return false;
        };
        let previous_len = span.openings.len();
        span.openings.retain(|opening| opening.id != opening_id);
        span.openings.len() != previous_len
    }

    fn span_path(&self, span: &WallSpan) -> Option<WallPath> {
        WallPath::new(
            self.node(span.start_node)?.position,
            self.node(span.end_node)?.position,
            span,
        )
    }

    pub fn span_length(&self, span_id: Uuid) -> Option<f32> {
        let span = self.span(span_id)?;
        let start = self.node(span.start_node)?.position;
        let end = self.node(span.end_node)?.position;
        Some(WallPath::new(start, end, span)?.length)
    }

    /// Converts a world point on the wall into distance-along-span and height-above-base.
    pub fn span_coordinates(&self, span_id: Uuid, point: Vec3<f32>) -> Option<vek::Vec2<f32>> {
        let span = self.span(span_id)?;
        let start = self.node(span.start_node)?.position;
        let end = self.node(span.end_node)?.position;
        let path = WallPath::new(start, end, span)?;
        let (along, base) = path.project(point)?;
        Some(vek::Vec2::new(along, point.y - base.y))
    }

    /// Converts span-local distance and height back to the wall center plane.
    pub fn span_point(&self, span_id: Uuid, coordinates: vek::Vec2<f32>) -> Option<Vec3<f32>> {
        let span = self.span(span_id)?;
        let start = self.node(span.start_node)?.position;
        let end = self.node(span.end_node)?.position;
        let path = WallPath::new(start, end, span)?;
        let base = path.point_at(coordinates.x);
        Some(base + Vec3::unit_y() * coordinates.y)
    }

    pub fn span_geometry_layer_at(
        &self,
        span_id: Uuid,
        coordinates: vek::Vec2<f32>,
    ) -> Option<WallGeometryLayer> {
        let span = self.span(span_id)?;
        let style = span.style_override.as_ref().unwrap_or(&self.style);
        Some(wall_geometry_layer_at(span, style, coordinates))
    }

    pub fn add_rectangular_opening(
        &mut self,
        span_id: Uuid,
        first: vek::Vec2<f32>,
        second: vek::Vec2<f32>,
    ) -> Result<Uuid, String> {
        self.add_opening(span_id, first, second, WallOpeningShape::Rectangular)
    }

    pub fn add_opening(
        &mut self,
        span_id: Uuid,
        first: vek::Vec2<f32>,
        second: vek::Vec2<f32>,
        shape: WallOpeningShape,
    ) -> Result<Uuid, String> {
        let length = self
            .span_length(span_id)
            .ok_or_else(|| "The selected wall span does not exist.".to_string())?;
        let style = self
            .span(span_id)
            .and_then(|span| span.style_override.as_ref())
            .unwrap_or(&self.style);
        let left = first.x.min(second.x).clamp(0.0, length);
        let right = first.x.max(second.x).clamp(0.0, length);
        let bottom = first.y.min(second.y).clamp(0.0, style.height);
        let top = first.y.max(second.y).clamp(0.0, style.height);
        if right - left <= 0.05 || top - bottom <= 0.05 {
            return Err("A wall opening needs visible width and height.".to_string());
        }
        let id = Uuid::new_v4();
        self.span_mut(span_id)
            .ok_or_else(|| "The selected wall span does not exist.".to_string())?
            .openings
            .push(WallOpening {
                id,
                center: (left + right) * 0.5,
                bottom,
                width: right - left,
                height: top - bottom,
                shape,
                arch_radius: None,
                frame: WallOpeningFrame::default(),
            });
        Ok(id)
    }

    pub fn brick_at(&self, span_id: Uuid, coordinates: vek::Vec2<f32>) -> Option<WallBrickKey> {
        let span = self.span(span_id)?;
        let style = span.style_override.as_ref().unwrap_or(&self.style);
        let length = self.span_length(span_id)?;
        if coordinates.x < 0.0
            || coordinates.x > length
            || coordinates.y < 0.0
            || coordinates.y > style.height
            || wall_geometry_layer_at(span, style, coordinates) != WallGeometryLayer::Masonry
        {
            return None;
        }
        let layout = wall_masonry_layout(style);
        let gap = style
            .mortar_gap
            .max(0.0)
            .min(layout.stone_width.min(layout.course_height) * 0.45);
        let pitch_y = layout.course_height + gap;
        let nominal_course = (coordinates.y / pitch_y).floor() as i32;
        let pitch_x = layout.stone_width + gap;
        for course in nominal_course.saturating_sub(1)..=nominal_course.saturating_add(1) {
            if course < 0 {
                continue;
            }
            let offset = if course.rem_euclid(2) == 1 {
                layout.course_offset * layout.stone_width
            } else {
                0.0
            };
            let nominal_index = ((coordinates.x - offset) / pitch_x).floor() as i32;
            for index in nominal_index.saturating_sub(1)..=nominal_index.saturating_add(1) {
                let key = WallBrickKey { course, index };
                let Some((left, right, bottom, top)) = wall_brick_rect(style, key, length) else {
                    continue;
                };
                if coordinates.x >= left
                    && coordinates.x <= right
                    && coordinates.y >= bottom
                    && coordinates.y <= top
                {
                    return Some(key);
                }
            }
        }
        None
    }

    pub fn brick_rect(&self, span_id: Uuid, key: WallBrickKey) -> Option<(f32, f32, f32, f32)> {
        let span = self.span(span_id)?;
        let style = span.style_override.as_ref().unwrap_or(&self.style);
        wall_brick_rect(style, key, self.span_length(span_id)?)
    }

    pub fn set_brick_removed(
        &mut self,
        span_id: Uuid,
        key: WallBrickKey,
        removed: bool,
    ) -> Result<(), String> {
        let span = self
            .span_mut(span_id)
            .ok_or_else(|| "The selected wall span does not exist.".to_string())?;
        if removed {
            if !span.removed_bricks.contains(&key) {
                span.removed_bricks.push(key);
            }
        } else {
            span.removed_bricks.retain(|candidate| *candidate != key);
        }
        Ok(())
    }

    pub fn add_node(&mut self, position: Vec3<f32>) -> Uuid {
        let node = WallNode::new(position);
        let id = node.id;
        self.nodes.push(node);
        id
    }

    pub fn nearest_node(&self, position: Vec3<f32>, maximum_distance: f32) -> Option<Uuid> {
        let maximum_distance_squared = maximum_distance.max(0.0).powi(2);
        self.nodes
            .iter()
            .filter_map(|node| {
                let distance_squared = (node.position - position).magnitude_squared();
                (distance_squared <= maximum_distance_squared)
                    .then_some((node.id, distance_squared))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(id, _)| id)
    }

    pub fn add_span(&mut self, start_node: Uuid, end_node: Uuid) -> Result<Uuid, String> {
        if start_node == end_node {
            return Err("A wall span needs two different nodes.".to_string());
        }
        if self.node(start_node).is_none() || self.node(end_node).is_none() {
            return Err("A wall span endpoint is not part of the assembly.".to_string());
        }
        if self.spans.iter().any(|span| {
            (span.start_node == start_node && span.end_node == end_node)
                || (span.start_node == end_node && span.end_node == start_node)
        }) {
            return Err("The wall nodes are already connected.".to_string());
        }
        let id = Uuid::new_v4();
        self.spans.push(WallSpan {
            id,
            start_node,
            end_node,
            curve_offset: 0.0,
            curve_segments: default_wall_curve_segments(),
            style_override: None,
            openings: Vec::new(),
            removed_bricks: Vec::new(),
        });
        Ok(id)
    }

    /// Splits a straight span at an interior point while retaining its authored settings. Curved
    /// spans are deliberately left intact: endpoint contacts normally land on the straight host
    /// wall, while the contacting wall is free to remain curved.
    fn split_straight_span_at(
        &mut self,
        span_id: Uuid,
        point: Vec3<f32>,
    ) -> Option<(Uuid, Uuid, f32)> {
        let span = self.span(span_id)?.clone();
        if span.curve_offset.abs() > 1e-5 {
            return None;
        }
        let length = self.span_length(span_id)?;
        let coordinates = self.span_coordinates(span_id, point)?;
        let split = coordinates.x;
        if split <= 1e-4 || split >= length - 1e-4 {
            return None;
        }
        // Do not silently damage authored features that cross the requested junction.
        if !span.removed_bricks.is_empty()
            || span.openings.iter().any(|opening| {
                opening.center - opening.width * 0.5 < split
                    && opening.center + opening.width * 0.5 > split
            })
        {
            return None;
        }

        let projected = self.span_point(span_id, vek::Vec2::new(split, 0.0))?;
        let split_key = (split * 10_000.0).round().max(0.0) as u128;
        let node_id = Uuid::from_u128(
            span_id.as_u128() ^ WALL_SPLIT_NODE_ID_MASK ^ split_key.rotate_left(37),
        );
        let second_id = Uuid::from_u128(
            span_id.as_u128() ^ WALL_SPLIT_SPAN_ID_MASK ^ split_key.rotate_left(71),
        );
        self.nodes.push(WallNode {
            id: node_id,
            position: projected,
        });
        let mut first = span.clone();
        first.end_node = node_id;
        first.openings = span
            .openings
            .iter()
            .filter(|opening| opening.center <= split)
            .cloned()
            .collect();
        let mut second = span;
        second.id = second_id;
        second.start_node = node_id;
        second.openings = second
            .openings
            .into_iter()
            .filter_map(|mut opening| {
                (opening.center > split).then(|| {
                    opening.center -= split;
                    opening
                })
            })
            .collect();
        let span_index = self
            .spans
            .iter()
            .position(|candidate| candidate.id == span_id)?;
        self.spans[span_index] = first;
        self.spans.insert(span_index + 1, second);
        for surface in &mut self.area_surfaces {
            let mut boundary = Vec::with_capacity(surface.boundary.len() + 1);
            for edge in surface.boundary.iter().copied() {
                if edge.span_id != span_id {
                    boundary.push(edge);
                } else if edge.forward {
                    boundary.push(edge);
                    boundary.push(WallSurfaceEdge {
                        span_id: second_id,
                        forward: true,
                    });
                } else {
                    boundary.push(WallSurfaceEdge {
                        span_id: second_id,
                        forward: false,
                    });
                    boundary.push(edge);
                }
            }
            surface.boundary = boundary;
        }
        Some((node_id, second_id, split))
    }

    fn weld_node_into(&mut self, node_id: Uuid, junction_id: Uuid) -> bool {
        if node_id == junction_id
            || self.node(node_id).is_none()
            || self.node(junction_id).is_none()
        {
            return node_id == junction_id;
        }
        for span in &mut self.spans {
            if span.start_node == node_id {
                span.start_node = junction_id;
            }
            if span.end_node == node_id {
                span.end_node = junction_id;
            }
        }
        self.nodes.retain(|node| node.id != node_id);
        self.spans.retain(|span| span.start_node != span.end_node);
        true
    }

    pub fn connected_spans(&self, node_id: Uuid) -> impl Iterator<Item = &WallSpan> {
        self.spans
            .iter()
            .filter(move |span| span.start_node == node_id || span.end_node == node_id)
    }

    pub fn junction_kind(&self, node_id: Uuid) -> Option<WallJunctionKind> {
        self.node(node_id)?;
        Some(match self.connected_spans(node_id).count() {
            0 | 1 => WallJunctionKind::End,
            2 => {
                let mut directions = self
                    .connected_spans(node_id)
                    .filter_map(|span| self.span_direction_from_node(span, node_id));
                match (directions.next(), directions.next()) {
                    (Some(a), Some(b)) if a.dot(b).abs() > 0.999 => WallJunctionKind::Continuation,
                    _ => WallJunctionKind::Corner,
                }
            }
            3 => WallJunctionKind::Tee,
            4 => WallJunctionKind::Crossing,
            _ => WallJunctionKind::MultiWay,
        })
    }

    fn span_direction_from_node(&self, span: &WallSpan, node_id: Uuid) -> Option<Vec3<f32>> {
        let path = self.span_path(span)?;
        let direction = if span.start_node == node_id {
            path.points[1] - path.points[0]
        } else if span.end_node == node_id {
            path.points[path.points.len() - 2] - path.points[path.points.len() - 1]
        } else {
            return None;
        };
        Vec3::new(direction.x, 0.0, direction.z).try_normalized()
    }

    fn junction_inset_for_course(
        &self,
        span: &WallSpan,
        node_id: Uuid,
        course: i32,
        style: &WallStyle,
    ) -> f32 {
        let directions = self
            .connected_spans(node_id)
            .filter_map(|incident| self.span_direction_from_node(incident, node_id))
            .collect::<Vec<_>>();
        if directions.len() < 2 {
            return 0.0;
        }
        let Some(span_direction) = self.span_direction_from_node(span, node_id) else {
            return 0.0;
        };
        let bond_direction = directions[course.rem_euclid(directions.len() as i32) as usize];
        let layout = wall_masonry_layout(style);
        let half_length = layout.stone_width.max(style.thickness * 1.35).max(0.05) * 0.5;
        let half_depth = style.thickness * (0.54 + style.irregularity.clamp(0.0, 1.0) * 0.06);
        let bond_perpendicular = Vec3::new(-bond_direction.z, 0.0, bond_direction.x);
        // Project the bonded corner stone's oriented bounds onto this span. Every incident span
        // stops outside that owned volume, so the corner is interlocked without duplicate blocks.
        span_direction.dot(bond_direction).abs() * half_length
            + span_direction.dot(bond_perpendicular).abs() * half_depth
            + style.mortar_gap.max(0.0) * 0.5
    }

    fn closed_floor_outline(&self) -> Option<Vec<Vec3<f32>>> {
        let simple_loop = self.nodes.len() >= 3
            && self.spans.len() == self.nodes.len()
            && self
                .nodes
                .iter()
                .all(|node| self.connected_spans(node.id).count() == 2);
        if simple_loop {
            let start_node = self.nodes.first()?.id;
            let mut current_node = start_node;
            let mut previous_span = None;
            let mut outline = Vec::new();
            for _ in 0..self.spans.len() {
                let span = self
                    .connected_spans(current_node)
                    .find(|span| Some(span.id) != previous_span)?;
                let path = self.span_path(span)?;
                let (points, next_node) = if span.start_node == current_node {
                    (path.points, span.end_node)
                } else {
                    (path.points.into_iter().rev().collect(), span.start_node)
                };
                outline.extend(points.into_iter().take_while(|point| {
                    self.node(next_node).map_or(true, |next| {
                        (*point - next.position).magnitude_squared() > 1e-10
                    })
                }));
                previous_span = Some(span.id);
                current_node = next_node;
            }
            if current_node == start_node && outline.len() >= 3 {
                return Self::prepare_floor_outline(outline);
            }
        }

        // Internal walls turn an otherwise closed room into a branched planar graph. Its outside
        // face still traces the original room perimeter; using that face avoids the convex-hull
        // fallback filling concave corners after a fitted enclosure is joined to the room.
        let (_, boundary) = self
            .wall_surface_faces()?
            .into_iter()
            .filter(|(area, _)| *area < -1e-5)
            .min_by(|left, right| left.0.total_cmp(&right.0))?;
        Self::prepare_floor_outline(self.wall_surface_boundary_outline(&boundary)?)
    }

    fn prepare_floor_outline(mut outline: Vec<Vec3<f32>>) -> Option<Vec<Vec3<f32>>> {
        let floor_y = outline.iter().map(|point| point.y).fold(f32::MAX, f32::min) - 0.002;
        for point in &mut outline {
            point.y = floor_y;
        }
        let signed_area = outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .map(|(current, next)| current.x * next.z - next.x * current.z)
            .sum::<f32>();
        if signed_area.abs() <= 1e-5 {
            return None;
        }
        // Clockwise X/Z winding produces an upward-facing Y normal.
        if signed_area > 0.0 {
            outline.reverse();
        }
        Some(outline)
    }

    fn floor_area_outline(&self) -> Option<Vec<Vec3<f32>>> {
        // Treat the authored wall network as an area boundary rather than requiring an exact
        // graph loop. Each path segment contributes its full wall footprint; the outer hull then
        // gives open U/L-shaped rooms a useful floor and gives a single wall a narrow floor strip.
        let mut points = Vec::new();
        for span in &self.spans {
            let path = self.span_path(span)?;
            let style = span.style_override.as_ref().unwrap_or(&self.style);
            let half_width = (style.thickness * 0.5).max(0.01);
            for pair in path.points.windows(2) {
                let Some(tangent) =
                    Vec3::new(pair[1].x - pair[0].x, 0.0, pair[1].z - pair[0].z).try_normalized()
                else {
                    continue;
                };
                let side = Vec3::new(-tangent.z, 0.0, tangent.x) * half_width;
                for endpoint in pair {
                    points.push(Vec3::new(
                        endpoint.x + side.x,
                        endpoint.y,
                        endpoint.z + side.z,
                    ));
                    points.push(Vec3::new(
                        endpoint.x - side.x,
                        endpoint.y,
                        endpoint.z - side.z,
                    ));
                }
            }
        }
        if points.len() < 3 {
            return None;
        }

        points.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.z.total_cmp(&b.z)));
        points.dedup_by(|a, b| (a.x - b.x).abs() <= 1e-5 && (a.z - b.z).abs() <= 1e-5);
        if points.len() < 3 {
            return None;
        }

        let cross = |a: Vec3<f32>, b: Vec3<f32>, c: Vec3<f32>| {
            (b.x - a.x) * (c.z - a.z) - (b.z - a.z) * (c.x - a.x)
        };
        let mut lower = Vec::new();
        for point in points.iter().copied() {
            while lower.len() >= 2
                && cross(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 1e-6
            {
                lower.pop();
            }
            lower.push(point);
        }
        let mut upper = Vec::new();
        for point in points.iter().rev().copied() {
            while upper.len() >= 2
                && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 1e-6
            {
                upper.pop();
            }
            upper.push(point);
        }
        lower.pop();
        upper.pop();
        let mut outline = lower;
        outline.extend(upper);
        if outline.len() < 3 {
            return None;
        }

        Self::prepare_floor_outline(outline)
    }

    fn wall_surface_edge_nodes(&self, edge: WallSurfaceEdge) -> Option<(Uuid, Uuid)> {
        let span = self.span(edge.span_id)?;
        Some(if edge.forward {
            (span.start_node, span.end_node)
        } else {
            (span.end_node, span.start_node)
        })
    }

    fn wall_surface_edge_points(&self, edge: WallSurfaceEdge) -> Option<Vec<Vec3<f32>>> {
        let span = self.span(edge.span_id)?;
        let mut points = self.span_path(span)?.points;
        if !edge.forward {
            points.reverse();
        }
        Some(points)
    }

    fn wall_surface_edge_direction(&self, edge: WallSurfaceEdge) -> Option<vek::Vec2<f32>> {
        let points = self.wall_surface_edge_points(edge)?;
        let direction = *points.get(1)? - *points.first()?;
        vek::Vec2::new(direction.x, direction.z).try_normalized()
    }

    fn wall_surface_boundary_outline(
        &self,
        boundary: &[WallSurfaceEdge],
    ) -> Option<Vec<Vec3<f32>>> {
        if boundary.len() < 3 {
            return None;
        }
        let mut outline = Vec::new();
        for edge in boundary {
            let points = self.wall_surface_edge_points(*edge)?;
            for point in points {
                if outline
                    .last()
                    .is_none_or(|last: &Vec3<f32>| (*last - point).magnitude_squared() > 1e-10)
                {
                    outline.push(point);
                }
            }
        }
        if outline.len() >= 2
            && (outline[0] - outline[outline.len() - 1]).magnitude_squared() <= 1e-10
        {
            outline.pop();
        }
        (outline.len() >= 3).then_some(outline)
    }

    fn wall_surface_polygon_area(outline: &[Vec3<f32>]) -> f32 {
        outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .map(|(current, next)| current.x * next.z - next.x * current.z)
            .sum::<f32>()
            * 0.5
    }

    fn wall_surface_polygon_contains(outline: &[Vec3<f32>], point: vek::Vec2<f32>) -> bool {
        if outline.len() < 3 {
            return false;
        }
        let mut inside = false;
        let mut previous = outline.len() - 1;
        for current in 0..outline.len() {
            let a = vek::Vec2::new(outline[current].x, outline[current].z);
            let b = vek::Vec2::new(outline[previous].x, outline[previous].z);
            if (a.y > point.y) != (b.y > point.y)
                && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
            {
                inside = !inside;
            }
            previous = current;
        }
        inside
    }

    /// Finds the smallest bounded graph face containing the supplied X/Z point. Each directed
    /// half-edge walks the face on its left, using the actual curve tangent at junctions.
    pub fn wall_surface_region_at(&self, point: vek::Vec2<f32>) -> Option<Vec<WallSurfaceEdge>> {
        self.wall_surface_faces()?
            .into_iter()
            .filter(|(area, boundary)| {
                *area > 1e-5
                    && self
                        .wall_surface_boundary_outline(boundary)
                        .is_some_and(|outline| Self::wall_surface_polygon_contains(&outline, point))
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, boundary)| boundary)
    }

    fn wall_surface_faces(&self) -> Option<Vec<(f32, Vec<WallSurfaceEdge>)>> {
        let half_edges = self
            .spans
            .iter()
            .flat_map(|span| {
                [
                    WallSurfaceEdge {
                        span_id: span.id,
                        forward: true,
                    },
                    WallSurfaceEdge {
                        span_id: span.id,
                        forward: false,
                    },
                ]
            })
            .collect::<Vec<_>>();
        let mut visited = std::collections::HashSet::<WallSurfaceEdge>::new();
        let mut faces = Vec::<(f32, Vec<WallSurfaceEdge>)>::new();

        for seed in half_edges.iter().copied() {
            if visited.contains(&seed) {
                continue;
            }
            let mut boundary = Vec::new();
            let mut current = seed;
            for _ in 0..half_edges.len().saturating_add(1) {
                if !visited.insert(current) && current != seed {
                    boundary.clear();
                    break;
                }
                boundary.push(current);
                let (_, end_node) = self.wall_surface_edge_nodes(current)?;
                let twin = WallSurfaceEdge {
                    span_id: current.span_id,
                    forward: !current.forward,
                };
                let mut outgoing = half_edges
                    .iter()
                    .copied()
                    .filter(|edge| {
                        self.wall_surface_edge_nodes(*edge)
                            .is_some_and(|(start, _)| start == end_node)
                    })
                    .filter_map(|edge| {
                        let direction = self.wall_surface_edge_direction(edge)?;
                        Some((edge, direction.y.atan2(direction.x)))
                    })
                    .collect::<Vec<_>>();
                outgoing.sort_by(|left, right| left.1.total_cmp(&right.1));
                let twin_index = outgoing.iter().position(|(edge, _)| *edge == twin)?;
                current = outgoing[(twin_index + outgoing.len() - 1) % outgoing.len()].0;
                if current == seed {
                    break;
                }
            }
            if boundary.len() < 3 || current != seed {
                continue;
            }
            let Some(outline) = self.wall_surface_boundary_outline(&boundary) else {
                continue;
            };
            let area = Self::wall_surface_polygon_area(&outline);
            if area.abs() > 1e-5 {
                faces.push((area, boundary));
            }
        }
        Some(faces)
    }

    fn inset_wall_surface_outline(outline: &[Vec3<f32>], clearance: f32) -> Option<Vec<Vec3<f32>>> {
        if clearance <= 1e-6 {
            return Some(outline.to_vec());
        }
        let y = outline.first()?.y;
        let points = outline
            .iter()
            .map(|point| vek::Vec2::new(point.x, point.z))
            .collect::<Vec<_>>();
        let cross = |a: vek::Vec2<f32>, b: vek::Vec2<f32>| a.x * b.y - a.y * b.x;
        let mut inset = Vec::with_capacity(points.len());
        for index in 0..points.len() {
            let previous = points[(index + points.len() - 1) % points.len()];
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            let before = (current - previous).try_normalized()?;
            let after = (next - current).try_normalized()?;
            let before_normal = vek::Vec2::new(-before.y, before.x);
            let after_normal = vek::Vec2::new(-after.y, after.x);
            let line_a = previous + before_normal * clearance;
            let line_b = current + after_normal * clearance;
            let denominator = cross(before, after);
            let candidate = if denominator.abs() > 1e-5 {
                line_a + before * (cross(line_b - line_a, after) / denominator)
            } else {
                current
                    + (before_normal + after_normal)
                        .try_normalized()
                        .unwrap_or(before_normal)
                        * clearance
            };
            // Avoid extreme mitres at very acute wall junctions.
            let offset = candidate - current;
            let limited = if offset.magnitude() > clearance * 6.0 {
                current + offset.normalized() * clearance * 6.0
            } else {
                candidate
            };
            inset.push(Vec3::new(limited.x, y, limited.y));
        }
        (Self::wall_surface_polygon_area(&inset) > 1e-5).then_some(inset)
    }

    fn area_surface_geometry(&self, surface: &WallAreaSurface) -> Option<GeometryObject> {
        let mut outline = self.wall_surface_boundary_outline(&surface.boundary)?;
        if Self::wall_surface_polygon_area(&outline) < 0.0 {
            outline.reverse();
        }
        outline = Self::inset_wall_surface_outline(&outline, surface.clearance.max(0.0))?;
        let base_y = surface
            .boundary
            .iter()
            .filter_map(|edge| self.wall_surface_edge_nodes(*edge))
            .filter_map(|(node, _)| self.node(node).map(|node| node.position.y))
            .fold(f32::INFINITY, f32::min);
        if !base_y.is_finite() {
            return None;
        }
        let top_y = base_y + surface.elevation;
        let thickness = surface.thickness.max(0.005);
        for point in &mut outline {
            point.y = top_y;
        }
        // Clockwise X/Z winding produces an upward-facing Y normal.
        outline.reverse();

        let source = surface
            .source
            .clone()
            .unwrap_or_else(|| self.floor_pixel_source());
        let count = outline.len();
        let mut object = GeometryObject::new(format!("{} / Area Surface", self.name));
        object.id = generated_wall_surface_object_id(surface.id);
        object.kind = GeometryObjectKind::Generated;
        object.vertices.extend(outline.iter().copied());
        object.vertices.extend(
            outline
                .iter()
                .map(|point| *point - Vec3::unit_y() * thickness),
        );
        object.faces.push(wall_face(
            surface.id,
            0,
            (0..count).collect(),
            Some(&source),
        ));
        object.faces.push(wall_face(
            surface.id,
            1,
            (count..count * 2).rev().collect(),
            Some(&source),
        ));
        for index in 0..count {
            let next = (index + 1) % count;
            object.faces.push(wall_face(
                surface.id,
                index + 2,
                vec![index, count + index, count + next, next],
                Some(&source),
            ));
        }
        object.tags.push(GENERATED_WALL_TAG.to_string());
        object
            .properties
            .set("wall_assembly_id", Value::Id(self.id));
        object
            .properties
            .set("wall_area_surface_id", Value::Id(surface.id));
        object
            .properties
            .set("wall_area_surface", Value::Bool(true));
        object.ensure_face_paint_data();
        Some(object)
    }

    fn structural_floor_geometry(&self) -> Option<GeometryObject> {
        let outline = self
            .closed_floor_outline()
            .or_else(|| self.floor_area_outline())?;
        let primary_span = self.spans.first()?;
        let source = self.floor_pixel_source();
        let mut object = GeometryObject::new(format!("{} / Auto Floor", self.name));
        object.id = generated_wall_floor_object_id(self.id);
        object.kind = GeometryObjectKind::Generated;
        object.vertices = outline;
        object.faces.push(wall_face(
            primary_span.id,
            0,
            (0..object.vertices.len()).collect(),
            Some(&source),
        ));
        object.tags.push(GENERATED_WALL_TAG.to_string());
        object
            .properties
            .set("wall_assembly_id", Value::Id(self.id));
        object
            .properties
            .set("wall_span_id", Value::Id(primary_span.id));
        object.properties.set("wall_auto_floor", Value::Bool(true));
        object.ensure_face_paint_data();
        Some(object)
    }

    /// Builds full-depth masonry blocks for every span and bonded masonry at graph junctions.
    /// Mortar is emitted only as joint geometry; no backing shell sits behind removable blocks.
    pub fn structural_geometry(&self) -> Vec<GeometryObject> {
        self.spans
            .iter()
            .filter_map(|span| self.structural_span_geometry(span))
            .chain(
                self.nodes
                    .iter()
                    .filter_map(|node| self.structural_junction_geometry(node)),
            )
            .chain(
                self.auto_floor
                    .then(|| self.structural_floor_geometry())
                    .flatten(),
            )
            .chain(
                self.area_surfaces
                    .iter()
                    .filter_map(|surface| self.area_surface_geometry(surface)),
            )
            .collect()
    }

    /// Junction bonds make adjacent spans read as one masonry run. Each course selects an incident
    /// direction, producing an interlocking quoin at corners and hiding the capped-span seam at
    /// straight continuations.
    fn structural_junction_geometry(&self, node: &WallNode) -> Option<GeometryObject> {
        if self.junction_kind(node.id)? == WallJunctionKind::End {
            return None;
        }
        let incident = self.connected_spans(node.id).collect::<Vec<_>>();
        let primary_span = *incident.first()?;
        let style = primary_span.style_override.as_ref().unwrap_or(&self.style);
        if style.height <= 0.0 || style.thickness <= 0.0 {
            return None;
        }
        let directions = incident
            .iter()
            .filter_map(|span| self.span_direction_from_node(span, node.id))
            .collect::<Vec<_>>();
        if directions.len() < 2 {
            return None;
        }

        let mut object = GeometryObject::new(format!("{} / Wall Junction", self.name));
        object.id = generated_wall_junction_object_id(node.id);
        object.kind = GeometryObjectKind::Generated;
        let layout = wall_masonry_layout(style);
        let brick_height = layout.course_height;
        let gap = style
            .mortar_gap
            .max(0.0)
            .min(layout.stone_width.min(brick_height) * 0.45);
        let pitch_y = brick_height + gap;
        let course_count = (style.height / pitch_y).ceil() as i32;
        let vertical_amplitude = layout.vertical_amplitude;
        for course in 0..course_count {
            let direction = directions[course as usize % directions.len()];
            let perpendicular = Vec3::new(-direction.z, 0.0, direction.x);
            let stone_length = layout.stone_width.max(style.thickness * 1.35).max(0.05);
            let start = node.position - direction * (stone_length * 0.5);
            let end = node.position + direction * (stone_length * 0.5);
            let key = WallBrickKey {
                course,
                index: (node.id.as_u128() as u64 as i32),
            };
            let bottom = (course as f32 * pitch_y
                + wall_boundary_shift(style, course, 0, 3, vertical_amplitude))
            .max(0.0);
            let top = (course as f32 * pitch_y
                + brick_height
                + wall_boundary_shift(style, course.saturating_add(1), 0, 3, vertical_amplitude))
            .min(style.height);
            let depth_noise = wall_stone_noise(style, key, 29) * 2.0 - 1.0;
            let half_depth = (style.thickness * 0.54
                + depth_noise * style.irregularity.clamp(0.0, 1.0) * style.thickness * 0.06)
                .max(style.thickness * 0.5);
            let erosion = (0.08 + wall_stone_noise(style, key, 31) * 0.34)
                * style.damage.clamp(0.0, 1.0)
                * stone_length.min(brick_height);
            let stone_source = style.stone_source_for_key(key);
            append_wall_prism(
                &mut object,
                node.id,
                start,
                end,
                stone_length,
                perpendicular,
                0.0,
                stone_length,
                bottom,
                top,
                half_depth,
                style.bevel.max(0.0) + erosion,
                Some(&stone_source),
            );
            let bed_shift =
                wall_boundary_shift(style, course.saturating_add(1), 0, 3, vertical_amplitude);
            let bed_bottom = course as f32 * pitch_y + brick_height + bed_shift + 0.0005;
            let bed_top = (course as f32 * pitch_y + brick_height + gap + bed_shift - 0.0005)
                .min(style.height);
            if bed_top - bed_bottom > 1e-5 {
                let joint_size = style.thickness * 1.06;
                let joint_start = node.position - direction * (joint_size * 0.5);
                let joint_end = node.position + direction * (joint_size * 0.5);
                let mortar_source = style.mortar_pixel_source();
                append_wall_prism(
                    &mut object,
                    node.id,
                    joint_start,
                    joint_end,
                    joint_size,
                    perpendicular,
                    0.0,
                    joint_size,
                    bed_bottom,
                    bed_top,
                    joint_size * 0.47,
                    0.0,
                    Some(&mortar_source),
                );
            }
        }
        if object.faces.is_empty() {
            return None;
        }
        object.tags.push(GENERATED_WALL_TAG.to_string());
        object
            .properties
            .set("wall_assembly_id", Value::Id(self.id));
        object
            .properties
            .set("wall_span_id", Value::Id(primary_span.id));
        object.properties.set("wall_node_id", Value::Id(node.id));
        object
            .properties
            .set("paint_group_object_id", Value::Id(self.id));
        object.ensure_face_paint_data();
        Some(object)
    }

    fn structural_span_geometry(&self, span: &WallSpan) -> Option<GeometryObject> {
        let style = span.style_override.as_ref().unwrap_or(&self.style);
        let path = self.span_path(span)?;
        let length = path.length;
        if length <= 1e-5 || style.height <= 0.0 || style.thickness <= 0.0 {
            return None;
        }
        let mut object = GeometryObject::new(format!("{} / Wall Span", self.name));
        object.id = generated_wall_object_id(span.id);
        object.kind = GeometryObjectKind::Generated;

        let layout = wall_masonry_layout(style);
        let brick_width = layout.stone_width;
        let brick_height = layout.course_height;
        let gap = style
            .mortar_gap
            .max(0.0)
            .min(brick_width.min(brick_height) * 0.45);
        let pitch_x = brick_width + gap;
        let pitch_y = brick_height + gap;
        let mortar_half_depth = style.thickness * 0.47;
        let mortar_source = style.mortar_pixel_source();
        let course_count = (style.height / pitch_y).ceil() as i32 + 1;
        let horizontal_amplitude = layout.horizontal_amplitude;
        let vertical_amplitude = layout.vertical_amplitude;
        let opening_bounds = span
            .openings
            .iter()
            .map(|opening| wall_opening_outer_bounds(opening, style))
            .collect::<Vec<_>>();
        let opening_profiles = span
            .openings
            .iter()
            .map(|opening| wall_opening_outer_profile(opening, style))
            .collect::<Vec<_>>();

        // Mortar is physical joint geometry only. There is deliberately no continuous backing
        // shell, so removing a stone exposes a full-depth cavity.
        if gap > 1e-5 {
            for course in 0..course_count {
                let bed_shift =
                    wall_boundary_shift(style, course.saturating_add(1), 0, 3, vertical_amplitude);
                let bed_bottom = course as f32 * pitch_y + brick_height + bed_shift + 0.0005;
                let bed_top = (course as f32 * pitch_y + brick_height + gap + bed_shift - 0.0005)
                    .min(style.height);
                if bed_top - bed_bottom > 1e-5 {
                    let subject = vec![
                        vek::Vec2::new(0.0, bed_bottom),
                        vek::Vec2::new(length, bed_bottom),
                        vek::Vec2::new(length, bed_top),
                        vek::Vec2::new(0.0, bed_top),
                    ];
                    for profile in subtract_wall_opening_profiles(subject, &opening_profiles) {
                        append_wall_path_profile_prism(
                            &mut object,
                            span.id,
                            &path,
                            &profile,
                            mortar_half_depth,
                            Some(&mortar_source),
                        );
                    }
                }

                let offset = if course.rem_euclid(2) == 1 {
                    layout.course_offset * brick_width
                } else {
                    0.0
                };
                let first_index = ((-offset - brick_width) / pitch_x).floor() as i32;
                let last_index = ((length - offset) / pitch_x).ceil() as i32;
                let joint_bottom = (course as f32 * pitch_y
                    + wall_boundary_shift(style, course, 0, 3, vertical_amplitude)
                    + 0.0005)
                    .max(0.0);
                let joint_top = (course as f32 * pitch_y
                    + brick_height
                    + wall_boundary_shift(
                        style,
                        course.saturating_add(1),
                        0,
                        3,
                        vertical_amplitude,
                    )
                    - 0.0005)
                    .min(style.height);
                for index in first_index..=last_index {
                    let head_shift = wall_boundary_shift(
                        style,
                        course,
                        index.saturating_add(1),
                        1,
                        horizontal_amplitude,
                    );
                    let raw_joint_left = offset + index as f32 * pitch_x + brick_width + head_shift;
                    let raw_joint_right = raw_joint_left + gap;
                    if raw_joint_right <= 0.0 || raw_joint_left >= length {
                        continue;
                    }
                    let joint_left = (raw_joint_left + 0.0005).max(0.0);
                    let joint_right = (raw_joint_right - 0.0005).min(length);
                    if joint_right - joint_left <= 1e-5 || joint_top - joint_bottom <= 1e-5 {
                        continue;
                    }
                    let subject = vec![
                        vek::Vec2::new(joint_left, joint_bottom),
                        vek::Vec2::new(joint_right, joint_bottom),
                        vek::Vec2::new(joint_right, joint_top),
                        vek::Vec2::new(joint_left, joint_top),
                    ];
                    for profile in subtract_wall_opening_profiles(subject, &opening_profiles) {
                        append_wall_path_profile_prism(
                            &mut object,
                            span.id,
                            &path,
                            &profile,
                            mortar_half_depth,
                            Some(&mortar_source),
                        );
                    }
                }
            }
        }
        for course in 0..course_count {
            let start_inset = self.junction_inset_for_course(span, span.start_node, course, style);
            let end_inset = self.junction_inset_for_course(span, span.end_node, course, style);
            let offset = if course.rem_euclid(2) == 1 {
                layout.course_offset * brick_width
            } else {
                0.0
            };
            let first_index = ((-offset - brick_width) / pitch_x).floor() as i32;
            let last_index = ((length - offset) / pitch_x).ceil() as i32;
            for index in first_index..=last_index {
                let key = WallBrickKey { course, index };
                if span.removed_bricks.contains(&key) {
                    continue;
                }
                let Some((mut brick_left, mut brick_right, brick_bottom, brick_top)) =
                    wall_brick_rect(style, key, length)
                else {
                    continue;
                };
                brick_left = brick_left.max(start_inset);
                brick_right = brick_right.min(length - end_inset);
                if brick_right - brick_left <= 1e-5 {
                    continue;
                }
                let brick_is_clipped = opening_bounds.iter().any(
                    |(outer_left, outer_right, outer_bottom, outer_top)| {
                        ranges_overlap(brick_left, brick_right, *outer_left, *outer_right)
                            && ranges_overlap(brick_bottom, brick_top, *outer_bottom, *outer_top)
                    },
                );
                let depth_noise = wall_stone_noise(style, key, 7) * 2.0 - 1.0;
                let stone_source = style.stone_source_for_key(key);
                if brick_is_clipped {
                    let subject = vec![
                        vek::Vec2::new(brick_left, brick_bottom),
                        vek::Vec2::new(brick_right, brick_bottom),
                        vek::Vec2::new(brick_right, brick_top),
                        vek::Vec2::new(brick_left, brick_top),
                    ];
                    for profile in subtract_wall_opening_profiles(subject, &opening_profiles) {
                        append_wall_path_profile_prism(
                            &mut object,
                            span.id,
                            &path,
                            &profile,
                            style.thickness * 0.5,
                            Some(&stone_source),
                        );
                    }
                } else {
                    let half_depth = (style.thickness * 0.5
                        + depth_noise
                            * style.irregularity.clamp(0.0, 1.0)
                            * style.thickness
                            * 0.08)
                        .max(style.thickness * 0.5);
                    let erosion = (0.08 + wall_stone_noise(style, key, 11) * 0.34)
                        * style.damage.clamp(0.0, 1.0)
                        * (brick_right - brick_left).min(brick_top - brick_bottom);
                    append_wall_path_prism(
                        &mut object,
                        span.id,
                        &path,
                        brick_left,
                        brick_right,
                        brick_bottom,
                        brick_top,
                        half_depth,
                        style.bevel.max(0.0) + erosion,
                        Some(&stone_source),
                    );
                }
            }
        }
        for opening in &span.openings {
            append_wall_opening_frame(&mut object, span.id, &path, style, opening);
        }
        if object.faces.is_empty() {
            return None;
        }
        object.tags.push(GENERATED_WALL_TAG.to_string());
        object
            .properties
            .set("wall_assembly_id", Value::Id(self.id));
        object.properties.set("wall_span_id", Value::Id(span.id));
        object
            .properties
            .set("paint_group_object_id", Value::Id(self.id));
        object.ensure_face_paint_data();
        Some(object)
    }
}

fn append_wall_path_profile_prism(
    object: &mut GeometryObject,
    span_id: Uuid,
    path: &WallPath,
    profile: &[vek::Vec2<f32>],
    half_depth: f32,
    source: Option<&PixelSource>,
) {
    if profile.len() < 3 || half_depth <= 1e-5 {
        return;
    }
    let vertex_offset = object.vertices.len();
    let tangent_epsilon = (path.length * 0.0001).max(0.0001);
    for depth in [-half_depth, half_depth] {
        for point in profile {
            let before = path.point_at(point.x - tangent_epsilon);
            let after = path.point_at(point.x + tangent_epsilon);
            let Some(direction) =
                Vec3::new(after.x - before.x, 0.0, after.z - before.z).try_normalized()
            else {
                return;
            };
            let perpendicular = Vec3::new(-direction.z, 0.0, direction.x);
            object
                .vertices
                .push(path.point_at(point.x) + Vec3::unit_y() * point.y + perpendicular * depth);
        }
    }
    let profile_len = profile.len();
    let mut push_face = |indices: Vec<usize>| {
        object.faces.push(wall_face(
            span_id,
            object.faces.len(),
            indices
                .into_iter()
                .map(|index| vertex_offset + index)
                .collect(),
            source,
        ));
    };
    push_face((0..profile_len).rev().collect());
    push_face((profile_len..profile_len * 2).collect());
    for index in 0..profile_len {
        let next = (index + 1) % profile_len;
        push_face(vec![index, next, profile_len + next, profile_len + index]);
    }
}

fn append_wall_frame_rect(
    object: &mut GeometryObject,
    span_id: Uuid,
    path: &WallPath,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    half_depth: f32,
    source: &PixelSource,
) {
    let left = left.clamp(0.0, path.length);
    let right = right.clamp(0.0, path.length);
    if right - left <= 1e-5 || top - bottom <= 1e-5 {
        return;
    }
    append_wall_path_profile_prism(
        object,
        span_id,
        path,
        &[
            vek::Vec2::new(left, bottom),
            vek::Vec2::new(right, bottom),
            vek::Vec2::new(right, top),
            vek::Vec2::new(left, top),
        ],
        half_depth,
        Some(source),
    );
}

#[allow(clippy::too_many_arguments)]
fn append_wall_frame_horizontal_blocks(
    object: &mut GeometryObject,
    span_id: Uuid,
    path: &WallPath,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    target_width: f32,
    gap: f32,
    stone_depth: f32,
    mortar_depth: f32,
    bevel: f32,
    stone_source: &PixelSource,
    mortar_source: &PixelSource,
) {
    let length = (right - left).max(0.0);
    let count = (length / target_width.max(0.05)).round().max(1.0) as usize;
    let segment = length / count as f32;
    let half_gap = gap.min(segment * 0.4) * 0.5;
    for index in 0..count {
        let boundary_left = left + segment * index as f32;
        let boundary_right = left + segment * (index + 1) as f32;
        append_wall_path_prism(
            object,
            span_id,
            path,
            boundary_left + if index == 0 { 0.0 } else { half_gap },
            boundary_right - if index + 1 == count { 0.0 } else { half_gap },
            bottom,
            top,
            stone_depth,
            bevel,
            Some(stone_source),
        );
        if index + 1 < count && half_gap > 1e-5 {
            append_wall_frame_rect(
                object,
                span_id,
                path,
                boundary_right - half_gap,
                boundary_right + half_gap,
                bottom,
                top,
                mortar_depth,
                mortar_source,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_wall_frame_vertical_blocks(
    object: &mut GeometryObject,
    span_id: Uuid,
    path: &WallPath,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    target_height: f32,
    gap: f32,
    stone_depth: f32,
    mortar_depth: f32,
    bevel: f32,
    stone_source: &PixelSource,
    mortar_source: &PixelSource,
) {
    let height = (top - bottom).max(0.0);
    let count = (height / target_height.max(0.05)).round().max(1.0) as usize;
    let segment = height / count as f32;
    let half_gap = gap.min(segment * 0.4) * 0.5;
    for index in 0..count {
        let boundary_bottom = bottom + segment * index as f32;
        let boundary_top = bottom + segment * (index + 1) as f32;
        append_wall_path_prism(
            object,
            span_id,
            path,
            left,
            right,
            boundary_bottom + if index == 0 { 0.0 } else { half_gap },
            boundary_top - if index + 1 == count { 0.0 } else { half_gap },
            stone_depth,
            bevel,
            Some(stone_source),
        );
        if index + 1 < count && half_gap > 1e-5 {
            append_wall_frame_rect(
                object,
                span_id,
                path,
                left,
                right,
                boundary_top - half_gap,
                boundary_top + half_gap,
                mortar_depth,
                mortar_source,
            );
        }
    }
}

fn append_wall_opening_frame(
    object: &mut GeometryObject,
    span_id: Uuid,
    path: &WallPath,
    style: &WallStyle,
    opening: &WallOpening,
) {
    let frame_width = opening.frame.width(style);
    if frame_width <= 1e-5 {
        return;
    }
    let left = opening.center - opening.width * 0.5;
    let right = opening.center + opening.width * 0.5;
    let top = (opening.bottom + opening.height).min(style.height);
    let horizontal_radius = opening.width * 0.5;
    let vertical_radius = opening.effective_arch_radius();
    let frame_top = match opening.shape {
        WallOpeningShape::Rectangular => top,
        WallOpeningShape::Arch => top - vertical_radius,
    };
    let source = opening.frame.pixel_source(style);
    let mortar_source = style.mortar_pixel_source();
    let half_depth = style.thickness * 0.5 + opening.frame.depth(style);
    let mortar_depth = (style.thickness * 0.47).min(half_depth);
    let layout = wall_masonry_layout(style);
    let unit_height = layout.course_height.max(0.05);
    let unit_width = layout.stone_width.max(0.05);
    let gap = style
        .mortar_gap
        .max(0.0)
        .min(frame_width.min(unit_height) * 0.35);
    let blocks = opening.frame.surround == WallOpeningSurround::Blocks;
    if blocks {
        for (jamb_left, jamb_right) in [(left - frame_width, left), (right, right + frame_width)] {
            append_wall_frame_vertical_blocks(
                object,
                span_id,
                path,
                jamb_left,
                jamb_right,
                opening.bottom,
                frame_top,
                unit_height,
                gap,
                half_depth,
                mortar_depth,
                style.bevel,
                &source,
                &mortar_source,
            );
        }
    } else {
        for (jamb_left, jamb_right) in [(left - frame_width, left), (right, right + frame_width)] {
            append_wall_frame_rect(
                object,
                span_id,
                path,
                jamb_left,
                jamb_right,
                opening.bottom,
                frame_top,
                half_depth,
                &source,
            );
        }
    }

    if opening.bottom > frame_width * 0.5 {
        let sill_bottom = (opening.bottom - frame_width).max(0.0);
        if blocks {
            append_wall_frame_horizontal_blocks(
                object,
                span_id,
                path,
                left - frame_width,
                right + frame_width,
                sill_bottom,
                opening.bottom,
                unit_width,
                gap,
                half_depth,
                mortar_depth,
                style.bevel,
                &source,
                &mortar_source,
            );
        } else {
            append_wall_frame_rect(
                object,
                span_id,
                path,
                left - frame_width,
                right + frame_width,
                sill_bottom,
                opening.bottom,
                half_depth,
                &source,
            );
        }
    }

    match opening.shape {
        WallOpeningShape::Rectangular => {
            let lintel_top = (top + frame_width).min(style.height);
            if blocks {
                append_wall_frame_horizontal_blocks(
                    object,
                    span_id,
                    path,
                    left - frame_width,
                    right + frame_width,
                    top,
                    lintel_top,
                    unit_width,
                    gap,
                    half_depth,
                    mortar_depth,
                    style.bevel,
                    &source,
                    &mortar_source,
                );
            } else {
                append_wall_frame_rect(
                    object,
                    span_id,
                    path,
                    left - frame_width,
                    right + frame_width,
                    top,
                    lintel_top,
                    half_depth,
                    &source,
                );
            }
        }
        WallOpeningShape::Arch => {
            let spring = top - vertical_radius;
            let outer_radius_x = horizontal_radius + frame_width;
            let outer_radius_y = vertical_radius + frame_width;
            let stones = if blocks {
                opening.frame.arch_stones(style) as usize
            } else {
                (opening.frame.arch_stones(style) as usize * 4).max(24)
            };
            let angle_step = std::f32::consts::PI / stones as f32;
            let half_angle_gap = if blocks {
                (gap / horizontal_radius.min(vertical_radius).max(0.05)).min(angle_step * 0.3) * 0.5
            } else {
                0.0
            };
            let frame_point = |angle: f32, radius_x: f32, radius_y: f32| {
                vek::Vec2::new(
                    (opening.center + angle.cos() * radius_x).clamp(0.0, path.length),
                    (spring + angle.sin() * radius_y).clamp(0.0, style.height),
                )
            };
            for stone in 0..stones {
                let angle0 = std::f32::consts::PI * stone as f32 / stones as f32;
                let angle1 = std::f32::consts::PI * (stone + 1) as f32 / stones as f32;
                let stone_angle0 = angle0 + if stone == 0 { 0.0 } else { half_angle_gap };
                let stone_angle1 = angle1
                    - if stone + 1 == stones {
                        0.0
                    } else {
                        half_angle_gap
                    };
                let profile = [
                    frame_point(stone_angle0, horizontal_radius, vertical_radius),
                    frame_point(stone_angle1, horizontal_radius, vertical_radius),
                    frame_point(stone_angle1, outer_radius_x, outer_radius_y),
                    frame_point(stone_angle0, outer_radius_x, outer_radius_y),
                ];
                append_wall_path_profile_prism(
                    object,
                    span_id,
                    path,
                    &profile,
                    half_depth,
                    Some(&source),
                );
                if blocks && stone + 1 < stones && half_angle_gap > 1e-5 {
                    let mortar_profile = [
                        frame_point(angle1 - half_angle_gap, horizontal_radius, vertical_radius),
                        frame_point(angle1 + half_angle_gap, horizontal_radius, vertical_radius),
                        frame_point(angle1 + half_angle_gap, outer_radius_x, outer_radius_y),
                        frame_point(angle1 - half_angle_gap, outer_radius_x, outer_radius_y),
                    ];
                    append_wall_path_profile_prism(
                        object,
                        span_id,
                        path,
                        &mortar_profile,
                        mortar_depth,
                        Some(&mortar_source),
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn append_wall_path_prism(
    object: &mut GeometryObject,
    span_id: Uuid,
    path: &WallPath,
    u0: f32,
    u1: f32,
    v0: f32,
    v1: f32,
    half_depth: f32,
    bevel: f32,
    source: Option<&PixelSource>,
) {
    if u1 - u0 <= 1e-5 || v1 - v0 <= 1e-5 || half_depth <= 1e-5 {
        return;
    }
    let mut stations = std::iter::once(u0)
        .chain(
            path.cumulative
                .iter()
                .copied()
                .filter(|distance| *distance > u0 && *distance < u1),
        )
        .chain(std::iter::once(u1))
        .collect::<Vec<_>>();
    stations.sort_by(f32::total_cmp);
    stations.dedup_by(|left, right| (*left - *right).abs() <= 1e-5);
    if stations.len() == 2 {
        let start = path.point_at(u0);
        let end = path.point_at(u1);
        let horizontal = Vec3::new(end.x - start.x, 0.0, end.z - start.z);
        let length = horizontal.magnitude();
        if length <= 1e-5 {
            return;
        }
        let perpendicular = Vec3::new(-horizontal.z / length, 0.0, horizontal.x / length);
        append_wall_prism(
            object,
            span_id,
            start,
            end,
            length,
            perpendicular,
            0.0,
            length,
            v0,
            v1,
            half_depth,
            bevel,
            source,
        );
        return;
    }

    let bevel = bevel
        .max(0.0)
        .min((u1 - u0).min(v1 - v0).min(half_depth * 2.0) * 0.24);
    let cross_section = if bevel <= 1e-5 {
        vec![
            (-half_depth, v0),
            (-half_depth, v1),
            (half_depth, v1),
            (half_depth, v0),
        ]
    } else {
        let corner =
            |index| bevel * (0.45 + wall_prism_corner_noise(span_id, u0, u1, v0, v1, index) * 0.85);
        let back_bottom = corner(0);
        let back_top = corner(1);
        let front_top = corner(2);
        let front_bottom = corner(3);
        vec![
            (-half_depth + back_bottom, v0),
            (-half_depth, v0 + back_bottom),
            (-half_depth, v1 - back_top),
            (-half_depth + back_top, v1),
            (half_depth - front_top, v1),
            (half_depth, v1 - front_top),
            (half_depth, v0 + front_bottom),
            (half_depth - front_bottom, v0),
        ]
    };
    let ring_size = cross_section.len();
    let vertex_offset = object.vertices.len();
    let tangent_epsilon = (path.length * 0.0001).max(0.0001);
    for distance in &stations {
        let before = path.point_at(*distance - tangent_epsilon);
        let after = path.point_at(*distance + tangent_epsilon);
        let horizontal = Vec3::new(after.x - before.x, 0.0, after.z - before.z);
        let Some(direction) = horizontal.try_normalized() else {
            return;
        };
        let perpendicular = Vec3::new(-direction.z, 0.0, direction.x);
        let base = path.point_at(*distance);
        object.vertices.extend(
            cross_section
                .iter()
                .map(|(depth, height)| base + perpendicular * *depth + Vec3::unit_y() * *height),
        );
    }
    let mut push_face = |indices: Vec<usize>| {
        object.faces.push(wall_face(
            span_id,
            object.faces.len(),
            indices
                .into_iter()
                .map(|index| vertex_offset + index)
                .collect(),
            source,
        ));
    };
    push_face((0..ring_size).rev().collect());
    for station in 0..stations.len() - 1 {
        let current = station * ring_size;
        let next = (station + 1) * ring_size;
        for side in 0..ring_size {
            let following = (side + 1) % ring_size;
            push_face(vec![
                current + side,
                current + following,
                next + following,
                next + side,
            ]);
        }
    }
    let last = (stations.len() - 1) * ring_size;
    push_face((0..ring_size).map(|index| last + index).collect());
}

#[allow(clippy::too_many_arguments)]
fn append_wall_prism(
    object: &mut GeometryObject,
    span_id: Uuid,
    start: Vec3<f32>,
    end: Vec3<f32>,
    length: f32,
    perpendicular_direction: Vec3<f32>,
    u0: f32,
    u1: f32,
    v0: f32,
    v1: f32,
    half_depth: f32,
    bevel: f32,
    source: Option<&PixelSource>,
) {
    if u1 - u0 <= 1e-5 || v1 - v0 <= 1e-5 || half_depth <= 1e-5 {
        return;
    }
    let base_at = |u: f32| start + (end - start) * (u / length);
    let point_at = |u: f32, v: f32, depth: f32| {
        base_at(u) + Vec3::unit_y() * v + perpendicular_direction * depth
    };
    let bevel = bevel
        .max(0.0)
        .min((u1 - u0).min(v1 - v0).min(half_depth * 2.0) * 0.24);
    let vertex_offset = object.vertices.len();
    let local_faces = if bevel <= 1e-5 {
        object.vertices.extend([
            point_at(u0, v0, -half_depth),
            point_at(u1, v0, -half_depth),
            point_at(u1, v1, -half_depth),
            point_at(u0, v1, -half_depth),
            point_at(u0, v0, half_depth),
            point_at(u1, v0, half_depth),
            point_at(u1, v1, half_depth),
            point_at(u0, v1, half_depth),
        ]);
        vec![
            vec![0, 1, 2, 3],
            vec![5, 4, 7, 6],
            vec![4, 0, 3, 7],
            vec![1, 5, 6, 2],
            vec![3, 2, 6, 7],
            vec![4, 5, 1, 0],
        ]
    } else {
        // Damage increases `bevel` at the caller. Unequal corner cuts turn that amount into
        // localized chips instead of uniformly shrinking the stone and widening its joint.
        let corner_bevel = |corner| {
            bevel * (0.45 + wall_prism_corner_noise(span_id, u0, u1, v0, v1, corner) * 0.85)
        };
        let bottom_left = corner_bevel(0);
        let bottom_right = corner_bevel(1);
        let top_right = corner_bevel(2);
        let top_left = corner_bevel(3);
        let bottom_left_depth = (half_depth - bottom_left).max(0.0);
        let bottom_right_depth = (half_depth - bottom_right).max(0.0);
        let top_right_depth = (half_depth - top_right).max(0.0);
        let top_left_depth = (half_depth - top_left).max(0.0);
        object.vertices.extend([
            point_at(u0 + bottom_left, v0 + bottom_left, -half_depth),
            point_at(u1 - bottom_right, v0 + bottom_right, -half_depth),
            point_at(u1 - top_right, v1 - top_right, -half_depth),
            point_at(u0 + top_left, v1 - top_left, -half_depth),
            point_at(u0, v0, -bottom_left_depth),
            point_at(u1, v0, -bottom_right_depth),
            point_at(u1, v1, -top_right_depth),
            point_at(u0, v1, -top_left_depth),
            point_at(u0, v0, bottom_left_depth),
            point_at(u1, v0, bottom_right_depth),
            point_at(u1, v1, top_right_depth),
            point_at(u0, v1, top_left_depth),
            point_at(u0 + bottom_left, v0 + bottom_left, half_depth),
            point_at(u1 - bottom_right, v0 + bottom_right, half_depth),
            point_at(u1 - top_right, v1 - top_right, half_depth),
            point_at(u0 + top_left, v1 - top_left, half_depth),
        ]);
        vec![
            vec![0, 1, 2, 3],
            vec![4, 5, 1, 0],
            vec![5, 6, 2, 1],
            vec![6, 7, 3, 2],
            vec![7, 4, 0, 3],
            vec![8, 4, 7, 11],
            vec![5, 9, 10, 6],
            vec![7, 6, 10, 11],
            vec![8, 9, 5, 4],
            vec![12, 8, 11, 15],
            vec![9, 13, 14, 10],
            vec![15, 11, 10, 14],
            vec![12, 13, 9, 8],
            vec![13, 12, 15, 14],
        ]
    };
    for indices in local_faces {
        let indices = indices
            .into_iter()
            .map(|index| vertex_offset + index)
            .collect();
        object
            .faces
            .push(wall_face(span_id, object.faces.len(), indices, source));
    }
}

fn wall_stone_noise(style: &WallStyle, key: WallBrickKey, salt: u64) -> f32 {
    let mut value = style.variation_seed
        ^ (key.course as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (key.index as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ salt.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as u32) as f32 / u32::MAX as f32
}

fn wall_prism_corner_noise(span_id: Uuid, u0: f32, u1: f32, v0: f32, v1: f32, corner: u64) -> f32 {
    let mut value = span_id.as_u128() as u64
        ^ (span_id.as_u128() >> 64) as u64
        ^ (u0.to_bits() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (u1.to_bits() as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (v0.to_bits() as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
        ^ (v1.to_bits() as u64).rotate_left(29)
        ^ corner.wrapping_mul(0xD6E8_FEB8_6659_FD93);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    (value as u32) as f32 / u32::MAX as f32
}

fn wall_boundary_shift(
    style: &WallStyle,
    course: i32,
    index: i32,
    salt: u64,
    amplitude: f32,
) -> f32 {
    (wall_stone_noise(style, WallBrickKey { course, index }, salt) * 2.0 - 1.0) * amplitude
}

#[derive(Clone, Copy, Debug)]
struct WallMasonryLayout {
    stone_width: f32,
    course_height: f32,
    course_offset: f32,
    horizontal_amplitude: f32,
    vertical_amplitude: f32,
}

fn wall_masonry_layout(style: &WallStyle) -> WallMasonryLayout {
    let base_width = style.brick_width.max(0.02);
    let base_height = style.brick_height.max(0.02);
    let irregularity = style.irregularity.clamp(0.0, 1.0);
    let (width_scale, height_scale, course_offset, horizontal_variation, vertical_variation) =
        match style.masonry {
            WallMasonryPattern::Brick => (
                1.0,
                1.0,
                style.alternating_course_offset,
                irregularity * 0.03,
                irregularity * 0.025,
            ),
            // Broad cut blocks with restrained but visible differences between neighboring units.
            WallMasonryPattern::StoneBlocks => (
                1.65,
                1.6,
                0.5,
                0.06 + irregularity * 0.08,
                0.02 + irregularity * 0.04,
            ),
            // Coursed rubble keeps the stones editable while deliberately breaking the regular
            // vertical and horizontal rhythm. A later free-rubble pattern can build on the same
            // source model without pretending these are texture cut-ins.
            WallMasonryPattern::Rubble => (
                1.35,
                1.25,
                0.35,
                0.12 + irregularity * 0.16,
                0.06 + irregularity * 0.10,
            ),
        };
    let stone_width = base_width * width_scale;
    let course_height = base_height * height_scale;
    WallMasonryLayout {
        stone_width,
        course_height,
        course_offset,
        horizontal_amplitude: stone_width * horizontal_variation,
        vertical_amplitude: course_height * vertical_variation,
    }
}

fn wall_brick_rect(
    style: &WallStyle,
    key: WallBrickKey,
    length: f32,
) -> Option<(f32, f32, f32, f32)> {
    if key.course < 0 || length <= 0.0 || style.height <= 0.0 {
        return None;
    }
    let layout = wall_masonry_layout(style);
    let brick_width = layout.stone_width;
    let brick_height = layout.course_height;
    let gap = style
        .mortar_gap
        .max(0.0)
        .min(brick_width.min(brick_height) * 0.45);
    let offset = if key.course.rem_euclid(2) == 1 {
        layout.course_offset * brick_width
    } else {
        0.0
    };
    let raw_left = offset + key.index as f32 * (brick_width + gap);
    let raw_right = raw_left + brick_width;
    let raw_bottom = key.course as f32 * (brick_height + gap);
    let raw_top = raw_bottom + brick_height;
    let horizontal_amplitude = layout.horizontal_amplitude;
    let vertical_amplitude = layout.vertical_amplitude;
    // Adjacent blocks use the same boundary displacement, keeping the authored mortar width
    // stable while their individual widths and course heights vary.
    let left = (raw_left
        + wall_boundary_shift(style, key.course, key.index, 1, horizontal_amplitude))
    .max(0.0);
    let right = (raw_right
        + wall_boundary_shift(
            style,
            key.course,
            key.index.saturating_add(1),
            1,
            horizontal_amplitude,
        ))
    .min(length);
    let bottom =
        (raw_bottom + wall_boundary_shift(style, key.course, 0, 3, vertical_amplitude)).max(0.0);
    let top = (raw_top
        + wall_boundary_shift(
            style,
            key.course.saturating_add(1),
            0,
            3,
            vertical_amplitude,
        ))
    .min(style.height);
    (right - left > 1e-5 && top - bottom > 1e-5).then_some((left, right, bottom, top))
}

fn wall_opening_contains(opening: &WallOpening, point: vek::Vec2<f32>) -> bool {
    let left = opening.center - opening.width * 0.5;
    let right = opening.center + opening.width * 0.5;
    let top = opening.bottom + opening.height;
    if point.x <= left || point.x >= right || point.y <= opening.bottom || point.y >= top {
        return false;
    }
    match opening.shape {
        WallOpeningShape::Rectangular => true,
        WallOpeningShape::Arch => {
            let radius_x = (opening.width * 0.5).max(0.001);
            let radius_y = opening.effective_arch_radius();
            let spring = top - radius_y;
            point.y <= spring
                || ((point.x - opening.center) / radius_x).powi(2)
                    + ((point.y - spring) / radius_y).powi(2)
                    < 1.0
        }
    }
}

fn wall_opening_frame_outer_contains(
    opening: &WallOpening,
    style: &WallStyle,
    point: vek::Vec2<f32>,
) -> bool {
    let frame_width = opening.frame.width(style);
    if frame_width <= 1e-5 {
        return false;
    }
    let left = opening.center - opening.width * 0.5;
    let right = opening.center + opening.width * 0.5;
    let top = opening.bottom + opening.height;
    let outer_bottom = if opening.bottom > frame_width * 0.5 {
        opening.bottom - frame_width
    } else {
        opening.bottom
    };
    match opening.shape {
        WallOpeningShape::Rectangular => {
            point.x > left - frame_width
                && point.x < right + frame_width
                && point.y > outer_bottom
                && point.y < top + frame_width
        }
        WallOpeningShape::Arch => {
            let radius_x = opening.width * 0.5;
            let radius_y = opening.effective_arch_radius();
            let spring = top - radius_y;
            let outer_radius_x = radius_x + frame_width;
            let outer_radius_y = radius_y + frame_width;
            if point.y <= spring {
                point.x > left - frame_width
                    && point.x < right + frame_width
                    && point.y > outer_bottom
            } else {
                ((point.x - opening.center) / outer_radius_x.max(0.001)).powi(2)
                    + ((point.y - spring) / outer_radius_y.max(0.001)).powi(2)
                    < 1.0
                    && point.y < spring + outer_radius_y
            }
        }
    }
}

fn wall_opening_outer_bounds(opening: &WallOpening, style: &WallStyle) -> (f32, f32, f32, f32) {
    let frame_width = opening.frame.width(style);
    let half_width = opening.width * 0.5;
    let left = opening.center - half_width - frame_width;
    let right = opening.center + half_width + frame_width;
    let bottom = if opening.bottom > frame_width * 0.5 {
        opening.bottom - frame_width
    } else {
        opening.bottom
    };
    let top = opening.bottom + opening.height + frame_width;
    (left, right, bottom, top)
}

fn wall_opening_arch_profile_angles(opening: &WallOpening, style: &WallStyle) -> Vec<f32> {
    let frame_width = opening.frame.width(style);
    let blocks = frame_width > 1e-5 && opening.frame.surround == WallOpeningSurround::Blocks;
    let stones = if blocks {
        opening.frame.arch_stones(style) as usize
    } else if frame_width > 1e-5 {
        (opening.frame.arch_stones(style) as usize * 4).max(24)
    } else {
        32
    };
    let horizontal_radius = opening.width * 0.5;
    let vertical_radius = opening.effective_arch_radius();
    let layout = wall_masonry_layout(style);
    let gap = style
        .mortar_gap
        .max(0.0)
        .min(frame_width.min(layout.course_height.max(0.05)) * 0.35);
    let angle_step = std::f32::consts::PI / stones as f32;
    let half_angle_gap = if blocks {
        (gap / horizontal_radius.min(vertical_radius).max(0.05)).min(angle_step * 0.3) * 0.5
    } else {
        0.0
    };
    let mut angles = vec![0.0, std::f32::consts::PI];
    for stone in 0..stones {
        let angle0 = angle_step * stone as f32;
        let angle1 = angle_step * (stone + 1) as f32;
        angles.push(angle0 + if stone == 0 { 0.0 } else { half_angle_gap });
        angles.push(
            angle1
                - if stone + 1 == stones {
                    0.0
                } else {
                    half_angle_gap
                },
        );
        if blocks && stone + 1 < stones && half_angle_gap > 1e-5 {
            angles.extend([angle1 - half_angle_gap, angle1 + half_angle_gap]);
        }
    }
    angles.sort_by(f32::total_cmp);
    angles.dedup_by(|left, right| (*left - *right).abs() <= 1e-6);
    angles
}

fn wall_opening_outer_profile(opening: &WallOpening, style: &WallStyle) -> Vec<vek::Vec2<f32>> {
    let frame_width = opening.frame.width(style);
    let half_width = opening.width * 0.5;
    let left = opening.center - half_width - frame_width;
    let right = opening.center + half_width + frame_width;
    let bottom = if opening.bottom > frame_width * 0.5 {
        opening.bottom - frame_width
    } else {
        opening.bottom
    };
    let top = opening.bottom + opening.height;
    match opening.shape {
        WallOpeningShape::Rectangular => vec![
            vek::Vec2::new(left, bottom),
            vek::Vec2::new(right, bottom),
            vek::Vec2::new(right, top + frame_width),
            vek::Vec2::new(left, top + frame_width),
        ],
        WallOpeningShape::Arch => {
            let radius_x = half_width + frame_width;
            let radius_y = opening.effective_arch_radius() + frame_width;
            let spring = top - opening.effective_arch_radius();
            let mut profile = vec![vek::Vec2::new(left, bottom), vek::Vec2::new(right, bottom)];
            profile.extend(
                wall_opening_arch_profile_angles(opening, style)
                    .into_iter()
                    .map(|angle| {
                        vek::Vec2::new(
                            opening.center + angle.cos() * radius_x,
                            spring + angle.sin() * radius_y,
                        )
                    }),
            );
            profile
        }
    }
}

fn wall_polygon_area(profile: &[vek::Vec2<f32>]) -> f32 {
    if profile.len() < 3 {
        return 0.0;
    }
    profile
        .iter()
        .zip(profile.iter().cycle().skip(1))
        .take(profile.len())
        .map(|(a, b)| a.x * b.y - b.x * a.y)
        .sum::<f32>()
        * 0.5
}

fn clean_wall_polygon(mut profile: Vec<vek::Vec2<f32>>) -> Vec<vek::Vec2<f32>> {
    profile.dedup_by(|left, right| (*left - *right).magnitude_squared() <= 1e-10);
    if profile.len() > 1 && (profile[0] - profile[profile.len() - 1]).magnitude_squared() <= 1e-10 {
        profile.pop();
    }
    if profile.len() < 3 || wall_polygon_area(&profile).abs() <= 1e-7 {
        Vec::new()
    } else {
        profile
    }
}

fn split_wall_polygon_by_edge(
    profile: &[vek::Vec2<f32>],
    edge_start: vek::Vec2<f32>,
    edge_end: vek::Vec2<f32>,
) -> (Vec<vek::Vec2<f32>>, Vec<vek::Vec2<f32>>) {
    let edge = edge_end - edge_start;
    let side = |point: vek::Vec2<f32>| {
        let relative = point - edge_start;
        edge.x * relative.y - edge.y * relative.x
    };
    let mut inside = Vec::new();
    let mut outside = Vec::new();
    for index in 0..profile.len() {
        let current = profile[index];
        let next = profile[(index + 1) % profile.len()];
        let current_side = side(current);
        let next_side = side(next);
        let current_inside = current_side >= -1e-6;
        let next_inside = next_side >= -1e-6;
        if current_inside {
            inside.push(current);
        } else {
            outside.push(current);
        }
        if current_inside != next_inside {
            let denominator = current_side - next_side;
            if denominator.abs() > 1e-8 {
                let intersection = current + (next - current) * (current_side / denominator);
                inside.push(intersection);
                outside.push(intersection);
            }
        }
    }
    (clean_wall_polygon(inside), clean_wall_polygon(outside))
}

fn subtract_convex_wall_profile(
    subject: Vec<vek::Vec2<f32>>,
    clip: &[vek::Vec2<f32>],
) -> Vec<Vec<vek::Vec2<f32>>> {
    if subject.len() < 3 || clip.len() < 3 {
        return vec![subject];
    }
    let mut oriented_clip = clip.to_vec();
    if wall_polygon_area(&oriented_clip) < 0.0 {
        oriented_clip.reverse();
    }
    let mut candidate = subject;
    let mut outside_pieces = Vec::new();
    for index in 0..oriented_clip.len() {
        if candidate.len() < 3 {
            break;
        }
        let (inside, outside) = split_wall_polygon_by_edge(
            &candidate,
            oriented_clip[index],
            oriented_clip[(index + 1) % oriented_clip.len()],
        );
        if outside.len() >= 3 {
            outside_pieces.push(outside);
        }
        candidate = inside;
    }
    outside_pieces
}

fn subtract_wall_opening_profiles(
    subject: Vec<vek::Vec2<f32>>,
    openings: &[Vec<vek::Vec2<f32>>],
) -> Vec<Vec<vek::Vec2<f32>>> {
    let mut pieces = vec![subject];
    for opening in openings {
        pieces = pieces
            .into_iter()
            .flat_map(|piece| subtract_convex_wall_profile(piece, opening))
            .collect();
        if pieces.is_empty() {
            break;
        }
    }
    pieces
}

fn ranges_overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> bool {
    a1 > b0 + 1e-5 && b1 > a0 + 1e-5
}

#[cfg(test)]
fn wall_opening_exclusion_intersects_rect(
    opening: &WallOpening,
    style: &WallStyle,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
) -> bool {
    if right - left <= 1e-5 || top - bottom <= 1e-5 {
        return false;
    }
    let frame_width = opening.frame.width(style);
    let half_width = opening.width * 0.5;
    let outer_left = opening.center - half_width - frame_width;
    let outer_right = opening.center + half_width + frame_width;
    let outer_bottom = if opening.bottom > frame_width * 0.5 {
        opening.bottom - frame_width
    } else {
        opening.bottom
    };
    match opening.shape {
        WallOpeningShape::Rectangular => {
            let outer_top = opening.bottom + opening.height + frame_width;
            ranges_overlap(left, right, outer_left, outer_right)
                && ranges_overlap(bottom, top, outer_bottom, outer_top)
        }
        WallOpeningShape::Arch => {
            let radius_y = opening.effective_arch_radius();
            let spring = opening.bottom + opening.height - radius_y;
            let lower_intersects = ranges_overlap(left, right, outer_left, outer_right)
                && ranges_overlap(bottom, top, outer_bottom, spring);
            if lower_intersects || top <= spring + 1e-5 {
                return lower_intersects;
            }
            let radius_x = (half_width + frame_width).max(0.001);
            let radius_y = (radius_y + frame_width).max(0.001);
            let upper_bottom = bottom.max(spring);
            let closest_x = opening.center.clamp(left, right);
            let closest_y = spring.clamp(upper_bottom, top);
            ((closest_x - opening.center) / radius_x).powi(2)
                + ((closest_y - spring) / radius_y).powi(2)
                < 1.0 - 1e-5
        }
    }
}

fn wall_geometry_layer_at(
    span: &WallSpan,
    style: &WallStyle,
    point: vek::Vec2<f32>,
) -> WallGeometryLayer {
    for opening in &span.openings {
        if wall_opening_contains(opening, point) {
            return WallGeometryLayer::Void(opening.id);
        }
    }
    for opening in &span.openings {
        if wall_opening_frame_outer_contains(opening, style, point) {
            return WallGeometryLayer::OpeningSurround(opening.id);
        }
    }
    WallGeometryLayer::Masonry
}

#[cfg(test)]
fn wall_cell_is_masonry(
    span: &WallSpan,
    style: &WallStyle,
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
) -> bool {
    !span.openings.iter().any(|opening| {
        wall_opening_exclusion_intersects_rect(opening, style, left, right, bottom, top)
    })
}

#[cfg(test)]
fn wall_span_void_contains(
    span: &WallSpan,
    style: &WallStyle,
    length: f32,
    point: vek::Vec2<f32>,
) -> bool {
    span.openings
        .iter()
        .any(|opening| wall_opening_contains(opening, point))
        || span.removed_bricks.iter().any(|key| {
            wall_brick_rect(style, *key, length).is_some_and(|(left, right, bottom, top)| {
                point.x > left && point.x < right && point.y > bottom && point.y < top
            })
        })
}

fn generated_wall_object_id(span_id: Uuid) -> Uuid {
    Uuid::from_u128(span_id.as_u128() ^ GENERATED_WALL_ID_MASK)
}

fn generated_wall_junction_object_id(node_id: Uuid) -> Uuid {
    Uuid::from_u128(node_id.as_u128() ^ GENERATED_WALL_JUNCTION_ID_MASK)
}

fn generated_wall_floor_object_id(assembly_id: Uuid) -> Uuid {
    Uuid::from_u128(assembly_id.as_u128() ^ GENERATED_WALL_FLOOR_ID_MASK)
}

fn generated_wall_surface_object_id(surface_id: Uuid) -> Uuid {
    Uuid::from_u128(surface_id.as_u128() ^ GENERATED_WALL_SURFACE_ID_MASK)
}

fn wall_face(
    span_id: Uuid,
    face_index: usize,
    indices: Vec<usize>,
    source: Option<&PixelSource>,
) -> GeometryFace {
    GeometryFace {
        id: Uuid::from_u128(
            span_id.as_u128() ^ GENERATED_WALL_ID_MASK.rotate_left(17) ^ face_index as u128,
        ),
        paint_surface_id: None,
        uvs: Vec::new(),
        indices,
        paint_uvs: Vec::new(),
        auto_uv: true,
        texture_offset: vek::Vec2::zero(),
        texture_scale: vek::Vec2::broadcast(1.0),
        texture_rotation: 0.0,
        tile: source.cloned(),
        tiles: FxHashMap::default(),
        surface_points: Vec::new(),
        surface_segments: Vec::new(),
        smoothing_group: 0,
    }
}

impl Map {
    pub fn wall_assembly(&self, assembly_id: Uuid) -> Option<&WallAssembly> {
        self.wall_assemblies
            .iter()
            .find(|assembly| assembly.id == assembly_id)
    }

    pub fn wall_assembly_mut(&mut self, assembly_id: Uuid) -> Option<&mut WallAssembly> {
        self.wall_assemblies
            .iter_mut()
            .find(|assembly| assembly.id == assembly_id)
    }

    pub fn wall_source_for_geometry_object(&self, object_id: Uuid) -> Option<(Uuid, Uuid)> {
        let object = self
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        let assembly_id = object.properties.get_id("wall_assembly_id")?;
        let span_id = object.properties.get_id("wall_span_id")?;
        self.wall_assembly(assembly_id)?.span(span_id)?;
        Some((assembly_id, span_id))
    }

    pub fn wall_area_surface_for_geometry_object(&self, object_id: Uuid) -> Option<(Uuid, Uuid)> {
        let object = self
            .geometry_objects
            .iter()
            .find(|object| object.id == object_id)?;
        let assembly_id = object.properties.get_id("wall_assembly_id")?;
        let surface_id = object.properties.get_id("wall_area_surface_id")?;
        (self
            .wall_assembly(assembly_id)
            .is_some_and(|assembly| assembly.area_surface(surface_id).is_some())
            || self.wall_surface_preview.as_ref().is_some_and(|preview| {
                preview.assembly_id == assembly_id && preview.surface.id == surface_id
            }))
        .then_some((assembly_id, surface_id))
    }

    pub fn wall_surface_region_at(
        &self,
        position: Vec3<f32>,
    ) -> Option<(Uuid, Vec<WallSurfaceEdge>)> {
        let point = vek::Vec2::new(position.x, position.z);
        self.wall_assemblies
            .iter()
            .filter_map(|assembly| {
                let boundary = assembly.wall_surface_region_at(point)?;
                let outline = assembly.wall_surface_boundary_outline(&boundary)?;
                Some((
                    assembly.id,
                    boundary,
                    WallAssembly::wall_surface_polygon_area(&outline).abs(),
                ))
            })
            .min_by(|left, right| left.2.total_cmp(&right.2))
            .map(|(assembly_id, boundary, _)| (assembly_id, boundary))
    }

    /// Resolve a hit on generated masonry back to the smooth parent wall plane. Generated bricks
    /// have bevelled and damaged faces whose individual normals are unsuitable for mounted props.
    pub fn wall_surface_frame_for_geometry_object(
        &self,
        object_id: Uuid,
        hit: Vec3<f32>,
        fallback_normal: Option<Vec3<f32>>,
    ) -> Option<(Vec3<f32>, Vec3<f32>)> {
        let (assembly_id, span_id) = self.wall_source_for_geometry_object(object_id)?;
        let assembly = self.wall_assembly(assembly_id)?;
        let span = assembly.span(span_id)?;
        let style = span.style_override.as_ref().unwrap_or(&assembly.style);
        let coordinates = assembly.span_coordinates(span_id, hit)?;
        let length = assembly.span_length(span_id)?;
        let along = coordinates.x.clamp(0.0, length);
        let center = assembly.span_point(span_id, vek::Vec2::new(along, coordinates.y))?;
        let epsilon = (length * 0.01).clamp(0.005, 0.05);
        let before = assembly.span_point(
            span_id,
            vek::Vec2::new((along - epsilon).max(0.0), coordinates.y),
        )?;
        let after = assembly.span_point(
            span_id,
            vek::Vec2::new((along + epsilon).min(length), coordinates.y),
        )?;
        let tangent = Vec3::new(after.x - before.x, 0.0, after.z - before.z).try_normalized()?;
        let canonical = Vec3::new(-tangent.z, 0.0, tangent.x);
        let signed_distance = (hit - center).dot(canonical);
        let side = if signed_distance.abs() > (style.thickness * 0.05).max(0.002) {
            signed_distance.signum()
        } else {
            let fallback = fallback_normal
                .map(|normal| Vec3::new(normal.x, 0.0, normal.z))
                .and_then(|normal| normal.try_normalized())
                .unwrap_or(canonical);
            if canonical.dot(fallback) < 0.0 {
                -1.0
            } else {
                1.0
            }
        };
        let outward = canonical * side;
        Some((center + outward * style.thickness * 0.5, outward))
    }

    /// Recompute mounted Prefab transforms from stable wall/span coordinates.
    /// The last transform is retained if a referenced wall was deleted.
    pub fn sync_wall_hosted_block_props(&mut self) -> usize {
        let updates = self
            .block_prop_instances
            .iter()
            .filter_map(|instance| {
                let BlockPropHostAttachment::WallSpan {
                    assembly_id,
                    span_id,
                    along,
                    height,
                    side,
                    offset,
                    rotation_quarters,
                } = instance.host_attachment.as_ref()?;
                let assembly = self.wall_assembly(*assembly_id)?;
                let span = assembly.span(*span_id)?;
                let style = span.style_override.as_ref().unwrap_or(&assembly.style);
                let length = assembly.span_length(*span_id)?;
                let along = along.clamp(0.0, length);
                let point = assembly.span_point(*span_id, vek::Vec2::new(along, *height))?;
                let epsilon = (length * 0.01).clamp(0.005, 0.05);
                let before = assembly.span_point(
                    *span_id,
                    vek::Vec2::new((along - epsilon).max(0.0), *height),
                )?;
                let after = assembly.span_point(
                    *span_id,
                    vek::Vec2::new((along + epsilon).min(length), *height),
                )?;
                let tangent =
                    Vec3::new(after.x - before.x, 0.0, after.z - before.z).try_normalized()?;
                let side = if *side < 0.0 { -1.0 } else { 1.0 };
                let outward = Vec3::new(-tangent.z, 0.0, tangent.x) * side;
                let mut up = Vec3::unit_y();
                let mut right = up.cross(outward).try_normalized()?;
                let angle = rotation_quarters.rem_euclid(4) as f32 * std::f32::consts::FRAC_PI_2;
                let (sin, cos) = angle.sin_cos();
                let rotated_right = right * cos + up * sin;
                let rotated_up = up * cos - right * sin;
                right = rotated_right;
                up = rotated_up;
                let origin = point + outward * (style.thickness * 0.5 + offset.max(0.0));
                let mut transform = identity_block_prop_transform();
                transform[0][0] = right.x;
                transform[0][1] = right.y;
                transform[0][2] = right.z;
                transform[1][0] = up.x;
                transform[1][1] = up.y;
                transform[1][2] = up.z;
                transform[2][0] = outward.x;
                transform[2][1] = outward.y;
                transform[2][2] = outward.z;
                transform[3][0] = origin.x;
                transform[3][1] = origin.y;
                transform[3][2] = origin.z;
                Some((instance.id, transform))
            })
            .collect::<Vec<_>>();
        for (instance_id, transform) in &updates {
            if let Some(instance) = self
                .block_prop_instances
                .iter_mut()
                .find(|instance| instance.id == *instance_id)
            {
                instance.world_transform = *transform;
            }
        }
        updates.len()
    }

    /// Re-project a moved mounted instance onto its existing wall span and
    /// update the semantic host coordinates. This lets the ordinary object
    /// move gizmo edit along, height, side, and standoff without detaching.
    pub fn refresh_wall_host_from_instance_transform(&mut self, instance_id: Uuid) -> bool {
        let Some(instance) = self
            .block_prop_instances
            .iter()
            .find(|instance| instance.id == instance_id)
        else {
            return false;
        };
        let Some(BlockPropHostAttachment::WallSpan {
            assembly_id,
            span_id,
            rotation_quarters,
            ..
        }) = instance.host_attachment.as_ref().cloned()
        else {
            return false;
        };
        let origin = Vec3::new(
            instance.world_transform[3][0],
            instance.world_transform[3][1],
            instance.world_transform[3][2],
        );
        let Some(assembly) = self.wall_assembly(assembly_id) else {
            return false;
        };
        let Some(coordinates) = assembly.span_coordinates(span_id, origin) else {
            return false;
        };
        let Some(length) = assembly.span_length(span_id) else {
            return false;
        };
        let along = coordinates.x.clamp(0.0, length);
        let Some(center) = assembly.span_point(span_id, vek::Vec2::new(along, coordinates.y))
        else {
            return false;
        };
        let epsilon = (length * 0.01).clamp(0.005, 0.05);
        let Some(before) = assembly.span_point(
            span_id,
            vek::Vec2::new((along - epsilon).max(0.0), coordinates.y),
        ) else {
            return false;
        };
        let Some(after) = assembly.span_point(
            span_id,
            vek::Vec2::new((along + epsilon).min(length), coordinates.y),
        ) else {
            return false;
        };
        let Some(tangent) = Vec3::new(after.x - before.x, 0.0, after.z - before.z).try_normalized()
        else {
            return false;
        };
        let canonical = Vec3::new(-tangent.z, 0.0, tangent.x);
        let signed_distance = (origin - center).dot(canonical);
        let side = if signed_distance < 0.0 { -1.0 } else { 1.0 };
        let thickness = assembly
            .span(span_id)
            .and_then(|span| span.style_override.as_ref())
            .unwrap_or(&assembly.style)
            .thickness;
        let offset = (signed_distance.abs() - thickness * 0.5).max(0.0);
        let Some(instance) = self
            .block_prop_instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)
        else {
            return false;
        };
        instance.host_attachment = Some(BlockPropHostAttachment::WallSpan {
            assembly_id,
            span_id,
            along,
            height: coordinates.y,
            side,
            offset,
            rotation_quarters,
        });
        true
    }

    pub fn flip_wall_hosted_block_prop(&mut self, instance_id: Uuid) -> bool {
        let Some(instance) = self
            .block_prop_instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)
        else {
            return false;
        };
        let Some(BlockPropHostAttachment::WallSpan { side, .. }) =
            instance.host_attachment.as_mut()
        else {
            return false;
        };
        *side *= -1.0;
        self.sync_wall_hosted_block_props();
        true
    }

    pub fn rotate_wall_hosted_block_prop(&mut self, instance_id: Uuid, turns: i32) -> bool {
        let Some(instance) = self
            .block_prop_instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)
        else {
            return false;
        };
        let Some(BlockPropHostAttachment::WallSpan {
            rotation_quarters, ..
        }) = instance.host_attachment.as_mut()
        else {
            return false;
        };
        *rotation_quarters = (*rotation_quarters + turns).rem_euclid(4);
        self.sync_wall_hosted_block_props();
        true
    }

    /// Rotate a placed Prefab. Mounted instances roll around the surface
    /// normal; free/ground instances yaw around world Y.
    pub fn rotate_block_prop_placement(&mut self, instance_id: Uuid, turns: i32) -> bool {
        if self.rotate_wall_hosted_block_prop(instance_id, turns) {
            return true;
        }
        let Some(instance) = self
            .block_prop_instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)
        else {
            return false;
        };
        let angle = turns as f32 * std::f32::consts::FRAC_PI_2;
        let (sin, cos) = angle.sin_cos();
        for column in [0usize, 2] {
            let x = instance.world_transform[column][0];
            let z = instance.world_transform[column][2];
            instance.world_transform[column][0] = x * cos + z * sin;
            instance.world_transform[column][2] = -x * sin + z * cos;
        }
        true
    }

    pub fn detach_block_prop(&mut self, instance_id: Uuid) -> bool {
        let Some(instance) = self
            .block_prop_instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)
        else {
            return false;
        };
        instance.host_attachment.take().is_some()
    }

    /// Attach an existing instance to the exact generated wall surface under
    /// the cursor. Geometry IDs are resolved back to stable assembly/span IDs.
    pub fn attach_block_prop_to_wall_surface(
        &mut self,
        instance_id: Uuid,
        object_id: Uuid,
        hit: Vec3<f32>,
        normal: Vec3<f32>,
        offset: f32,
    ) -> bool {
        let Some((assembly_id, span_id)) = self.wall_source_for_geometry_object(object_id) else {
            return false;
        };
        let Some(assembly) = self.wall_assembly(assembly_id) else {
            return false;
        };
        let Some(coordinates) = assembly.span_coordinates(span_id, hit) else {
            return false;
        };
        let Some(length) = assembly.span_length(span_id) else {
            return false;
        };
        let along = coordinates.x.clamp(0.0, length);
        let epsilon = (length * 0.01).clamp(0.005, 0.05);
        let Some(before) = assembly.span_point(
            span_id,
            vek::Vec2::new((along - epsilon).max(0.0), coordinates.y),
        ) else {
            return false;
        };
        let Some(after) = assembly.span_point(
            span_id,
            vek::Vec2::new((along + epsilon).min(length), coordinates.y),
        ) else {
            return false;
        };
        let Some(tangent) = Vec3::new(after.x - before.x, 0.0, after.z - before.z).try_normalized()
        else {
            return false;
        };
        let canonical = Vec3::new(-tangent.z, 0.0, tangent.x);
        let center = assembly
            .span_point(span_id, vek::Vec2::new(along, coordinates.y))
            .unwrap_or(hit);
        let signed_distance = (hit - center).dot(canonical);
        let side = if signed_distance.abs() > 0.002 {
            signed_distance.signum()
        } else {
            let normal = Vec3::new(normal.x, 0.0, normal.z)
                .try_normalized()
                .unwrap_or(canonical);
            if canonical.dot(normal) < 0.0 {
                -1.0
            } else {
                1.0
            }
        };
        let Some(instance) = self
            .block_prop_instances
            .iter_mut()
            .find(|instance| instance.id == instance_id)
        else {
            return false;
        };
        let rotation_quarters = match instance.host_attachment.as_ref() {
            Some(BlockPropHostAttachment::WallSpan {
                rotation_quarters, ..
            }) => *rotation_quarters,
            None => 0,
        };
        instance.host_attachment = Some(BlockPropHostAttachment::WallSpan {
            assembly_id,
            span_id,
            along,
            height: coordinates.y,
            side,
            offset: offset.max(0.0),
            rotation_quarters,
        });
        self.sync_wall_hosted_block_props();
        true
    }

    pub fn nearest_wall_span(
        &self,
        position: Vec3<f32>,
        maximum_distance: f32,
    ) -> Option<(Uuid, Uuid)> {
        let point = vek::Vec2::new(position.x, position.z);
        let maximum_distance_squared = maximum_distance.max(0.0).powi(2);
        self.wall_assemblies
            .iter()
            .flat_map(|assembly| {
                assembly.spans.iter().filter_map(move |span| {
                    let path = assembly.span_path(span)?;
                    let distance_squared = path
                        .points
                        .windows(2)
                        .filter_map(|pair| {
                            let a = vek::Vec2::new(pair[0].x, pair[0].z);
                            let b = vek::Vec2::new(pair[1].x, pair[1].z);
                            let direction = b - a;
                            let length_squared = direction.magnitude_squared();
                            if length_squared <= 1e-10 {
                                return None;
                            }
                            let t = ((point - a).dot(direction) / length_squared).clamp(0.0, 1.0);
                            Some((point - (a + direction * t)).magnitude_squared())
                        })
                        .min_by(f32::total_cmp)?;
                    (distance_squared <= maximum_distance_squared).then_some((
                        assembly.id,
                        span.id,
                        distance_squared,
                    ))
                })
            })
            .min_by(|left, right| left.2.total_cmp(&right.2))
            .map(|(assembly_id, span_id, _)| (assembly_id, span_id))
    }

    /// Finds the closest shared endpoint across all wall assemblies. The placement tool uses this
    /// to continue an existing run instead of producing overlapping disconnected walls.
    pub fn nearest_wall_node(
        &self,
        position: Vec3<f32>,
        maximum_distance: f32,
    ) -> Option<(Uuid, Uuid)> {
        self.wall_assemblies
            .iter()
            .filter_map(|assembly| {
                let node_id = assembly.nearest_node(position, maximum_distance)?;
                let distance_squared =
                    (assembly.node(node_id)?.position - position).magnitude_squared();
                Some((assembly.id, node_id, distance_squared))
            })
            .min_by(|left, right| left.2.total_cmp(&right.2))
            .map(|(assembly_id, node_id, _)| (assembly_id, node_id))
    }

    /// Turns visually touching open wall endpoints into real graph junctions. This is primarily
    /// used by fitted area surfaces: an independently drawn curved forge front may terminate in
    /// the middle of two room-wall spans and still needs to divide the room into two bounded faces.
    pub fn resolve_wall_endpoint_contacts(&mut self, tolerance: f32) -> usize {
        let tolerance = tolerance.clamp(0.001, 0.1);
        let mut resolved = 0;
        for _ in 0..128 {
            let endpoints = self
                .wall_assemblies
                .iter()
                .flat_map(|assembly| {
                    assembly.nodes.iter().filter_map(|node| {
                        (assembly.connected_spans(node.id).count() == 1).then_some((
                            assembly.id,
                            node.id,
                            node.position,
                        ))
                    })
                })
                .collect::<Vec<_>>();
            let mut contact = None;
            for (source_assembly, source_node, position) in endpoints {
                let candidate = self
                    .wall_assemblies
                    .iter()
                    .flat_map(|assembly| {
                        assembly.spans.iter().filter_map(move |span| {
                            if assembly.id == source_assembly
                                && (span.start_node == source_node || span.end_node == source_node)
                            {
                                return None;
                            }
                            if span.curve_offset.abs() > 1e-5 || !span.removed_bricks.is_empty() {
                                return None;
                            }
                            let length = assembly.span_length(span.id)?;
                            let coordinates = assembly.span_coordinates(span.id, position)?;
                            if coordinates.x <= tolerance || coordinates.x >= length - tolerance {
                                return None;
                            }
                            let projected =
                                assembly.span_point(span.id, vek::Vec2::new(coordinates.x, 0.0))?;
                            let distance =
                                Vec3::new(projected.x - position.x, 0.0, projected.z - position.z)
                                    .magnitude();
                            ((projected.y - position.y).abs() <= tolerance && distance <= tolerance)
                                .then_some((assembly.id, span.id, coordinates.x, distance))
                        })
                    })
                    .min_by(|left, right| left.3.total_cmp(&right.3));
                if let Some((target_assembly, target_span, _, _)) = candidate {
                    contact = Some((
                        source_assembly,
                        source_node,
                        target_assembly,
                        target_span,
                        position,
                    ));
                    break;
                }
            }
            let Some((source_assembly, source_node, target_assembly, target_span, position)) =
                contact
            else {
                break;
            };
            let Some((junction_node, second_span, split)) = self
                .wall_assembly_mut(target_assembly)
                .and_then(|assembly| assembly.split_straight_span_at(target_span, position))
            else {
                break;
            };

            for instance in &mut self.block_prop_instances {
                let Some(BlockPropHostAttachment::WallSpan {
                    assembly_id,
                    span_id,
                    along,
                    ..
                }) = instance.host_attachment.as_mut()
                else {
                    continue;
                };
                if *assembly_id == target_assembly && *span_id == target_span && *along > split {
                    *span_id = second_span;
                    *along -= split;
                }
            }

            if target_assembly != source_assembly {
                if self
                    .merge_wall_assemblies(target_assembly, source_assembly)
                    .is_err()
                {
                    break;
                }
                for instance in &mut self.block_prop_instances {
                    if let Some(BlockPropHostAttachment::WallSpan { assembly_id, .. }) =
                        instance.host_attachment.as_mut()
                        && *assembly_id == source_assembly
                    {
                        *assembly_id = target_assembly;
                    }
                }
            }
            if let Some(assembly) = self.wall_assembly_mut(target_assembly) {
                assembly.weld_node_into(source_node, junction_node);
            }
            for node in &mut self.selected_wall_nodes {
                if *node == source_node {
                    *node = junction_node;
                }
            }
            resolved += 1;
        }
        resolved
    }

    /// Combines two previously separate networks when a newly placed span connects them.
    pub fn merge_wall_assemblies(
        &mut self,
        target_id: Uuid,
        source_id: Uuid,
    ) -> Result<(), String> {
        if target_id == source_id {
            return Ok(());
        }
        let target_index = self
            .wall_assemblies
            .iter()
            .position(|assembly| assembly.id == target_id)
            .ok_or_else(|| "Target wall assembly does not exist.".to_string())?;
        let source_index = self
            .wall_assemblies
            .iter()
            .position(|assembly| assembly.id == source_id)
            .ok_or_else(|| "Source wall assembly does not exist.".to_string())?;
        let target_style = self.wall_assemblies[target_index].style.clone();
        let mut source = self.wall_assemblies.remove(source_index);
        if source.style != target_style {
            let source_style = source.style.clone();
            for span in &mut source.spans {
                if span.style_override.is_none() {
                    span.style_override = Some(source_style.clone());
                }
            }
        }
        let adjusted_target_index = if source_index < target_index {
            target_index - 1
        } else {
            target_index
        };
        let target = &mut self.wall_assemblies[adjusted_target_index];
        target.auto_floor |= source.auto_floor;
        if target.floor_source.is_none() {
            target.floor_source = source.floor_source.take();
        }
        target.nodes.extend(source.nodes);
        target.spans.extend(source.spans);
        target.area_surfaces.extend(source.area_surfaces);
        if self.selected_wall_assembly == Some(source_id) {
            self.selected_wall_assembly = Some(target_id);
        }
        Ok(())
    }

    /// Adds a span while reusing nearby endpoints and merging networks when the new span bridges
    /// two assemblies. Returns the assembly, span, and endpoint identities.
    pub fn connect_wall_points(
        &mut self,
        start: Vec3<f32>,
        end: Vec3<f32>,
        snap_distance: f32,
    ) -> Result<(Uuid, Uuid, Uuid, Uuid), String> {
        if Vec3::new(end.x - start.x, 0.0, end.z - start.z).magnitude_squared() <= 1e-10 {
            return Err("A wall span needs horizontal length.".to_string());
        }

        let start_anchor = self.nearest_wall_node(start, snap_distance);
        let end_anchor = self.nearest_wall_node(end, snap_distance);
        let assembly_id = match (start_anchor, end_anchor) {
            (Some((start_assembly, _)), Some((end_assembly, _)))
                if start_assembly != end_assembly =>
            {
                self.merge_wall_assemblies(start_assembly, end_assembly)?;
                start_assembly
            }
            (Some((assembly_id, _)), _) | (_, Some((assembly_id, _))) => assembly_id,
            (None, None) => {
                let assembly = WallAssembly::new(format!(
                    "Wall {}",
                    self.wall_assemblies.len().saturating_add(1)
                ));
                let assembly_id = assembly.id;
                self.wall_assemblies.push(assembly);
                assembly_id
            }
        };

        let start_node = start_anchor
            .map(|(_, node_id)| node_id)
            .unwrap_or_else(|| self.wall_assembly_mut(assembly_id).unwrap().add_node(start));
        let end_node = end_anchor
            .map(|(_, node_id)| node_id)
            .unwrap_or_else(|| self.wall_assembly_mut(assembly_id).unwrap().add_node(end));
        let span_id = self
            .wall_assembly_mut(assembly_id)
            .ok_or_else(|| "Wall assembly disappeared while connecting its span.".to_string())?
            .add_span(start_node, end_node)?;
        Ok((assembly_id, span_id, start_node, end_node))
    }

    /// Replaces only wall-owned generated objects. The connected wall graph remains the editable
    /// source of truth and unrelated authored geometry is left untouched.
    pub fn rebuild_wall_geometry(&mut self) {
        self.geometry_objects.retain(|object| {
            !object
                .tags
                .iter()
                .any(|tag| tag.as_str() == GENERATED_WALL_TAG)
        });
        self.geometry_objects.extend(
            self.wall_assemblies
                .iter()
                .flat_map(WallAssembly::structural_geometry),
        );
        self.sync_wall_hosted_block_props();
    }

    /// Rebuilds generated walls with the transient opening drag applied. The source graph is not
    /// changed; cancelling the interaction and calling `rebuild_wall_geometry` restores it.
    pub fn rebuild_wall_geometry_with_opening_preview(&mut self) {
        let mut assemblies = self.wall_assemblies.clone();
        if let Some(preview) = self.wall_opening_preview
            && let Some(assembly) = assemblies
                .iter_mut()
                .find(|assembly| assembly.id == preview.assembly_id)
        {
            if let Ok(opening_id) =
                assembly.add_opening(preview.span_id, preview.start, preview.end, preview.shape)
                && let Some(opening) = assembly.opening_mut(preview.span_id, opening_id)
            {
                opening.frame.surround = preview.surround;
            }
        }
        let generated = assemblies
            .iter()
            .flat_map(WallAssembly::structural_geometry)
            .collect::<Vec<_>>();
        self.geometry_objects.retain(|object| {
            !object
                .tags
                .iter()
                .any(|tag| tag.as_str() == GENERATED_WALL_TAG)
        });
        self.geometry_objects.extend(generated);
    }

    pub fn rebuild_wall_geometry_with_brick_preview(&mut self) {
        let mut assemblies = self.wall_assemblies.clone();
        if let Some(preview) = self.wall_brick_preview
            && let Some(assembly) = assemblies
                .iter_mut()
                .find(|assembly| assembly.id == preview.assembly_id)
        {
            let _ = assembly.set_brick_removed(preview.span_id, preview.key, preview.remove);
        }
        let generated = assemblies
            .iter()
            .flat_map(WallAssembly::structural_geometry)
            .collect::<Vec<_>>();
        self.geometry_objects.retain(|object| {
            !object
                .tags
                .iter()
                .any(|tag| tag.as_str() == GENERATED_WALL_TAG)
        });
        self.geometry_objects.extend(generated);
    }

    pub fn rebuild_wall_geometry_with_surface_preview(&mut self) {
        let mut assemblies = self
            .wall_surface_preview
            .as_ref()
            .and_then(|preview| preview.wall_assemblies.clone())
            .unwrap_or_else(|| self.wall_assemblies.clone());
        if let Some(preview) = self.wall_surface_preview.clone()
            && let Some(assembly) = assemblies
                .iter_mut()
                .find(|assembly| assembly.id == preview.assembly_id)
        {
            assembly.area_surfaces.push(preview.surface);
        }
        let generated = assemblies
            .iter()
            .flat_map(WallAssembly::structural_geometry)
            .collect::<Vec<_>>();
        self.geometry_objects.retain(|object| {
            !object
                .tags
                .iter()
                .any(|tag| tag.as_str() == GENERATED_WALL_TAG)
        });
        self.geometry_objects.extend(generated);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_node_forms_a_corner_and_then_a_tee() {
        let mut wall = WallAssembly::new("Dungeon wall");
        let center = wall.add_node(Vec3::zero());
        let east = wall.add_node(Vec3::new(1.0, 0.0, 0.0));
        let north = wall.add_node(Vec3::new(0.0, 0.0, 1.0));
        let west = wall.add_node(Vec3::new(-1.0, 0.0, 0.0));
        wall.add_span(center, east).unwrap();
        wall.add_span(center, north).unwrap();
        assert_eq!(wall.junction_kind(center), Some(WallJunctionKind::Corner));
        wall.add_span(center, west).unwrap();
        assert_eq!(wall.junction_kind(center), Some(WallJunctionKind::Tee));
    }

    #[test]
    fn corner_span_insets_clear_the_owned_bond_stone() {
        let mut wall = WallAssembly::new("Bonded corner");
        let center = wall.add_node(Vec3::zero());
        let east = wall.add_node(Vec3::unit_x());
        let north = wall.add_node(Vec3::unit_z());
        let east_span = wall.add_span(center, east).unwrap();
        let north_span = wall.add_span(center, north).unwrap();
        let east_span = wall.span(east_span).unwrap();
        let north_span = wall.span(north_span).unwrap();

        assert!(
            wall.junction_inset_for_course(east_span, center, 1, &wall.style)
                > wall.style.thickness * 0.5
        );
        assert!(
            wall.junction_inset_for_course(north_span, center, 0, &wall.style)
                > wall.style.thickness * 0.5
        );
    }

    #[test]
    fn structural_masonry_shares_one_paint_group_without_grouping_the_floor() {
        let mut wall = WallAssembly::new("Paintable room corner");
        wall.auto_floor = true;
        let southwest = wall.add_node(Vec3::zero());
        let southeast = wall.add_node(Vec3::unit_x());
        let northeast = wall.add_node(Vec3::new(1.0, 0.0, 1.0));
        let northwest = wall.add_node(Vec3::unit_z());
        wall.add_span(southwest, southeast).unwrap();
        wall.add_span(southeast, northeast).unwrap();
        wall.add_span(northeast, northwest).unwrap();
        wall.add_span(northwest, southwest).unwrap();

        let geometry = wall.structural_geometry();
        let masonry = geometry
            .iter()
            .filter(|object| {
                object.properties.get_id("wall_span_id").is_some()
                    && object.properties.get_bool("wall_auto_floor") != Some(true)
            })
            .collect::<Vec<_>>();
        assert!(masonry.len() >= 4);
        assert!(
            masonry
                .iter()
                .all(|object| object.properties.get_id("paint_group_object_id") == Some(wall.id))
        );
        assert!(geometry.iter().any(|object| {
            object.properties.get_bool("wall_auto_floor") == Some(true)
                && object.properties.get_id("paint_group_object_id").is_none()
        }));
    }

    #[test]
    fn auto_floor_uses_the_authored_area_without_requiring_a_closed_loop() {
        let mut wall = WallAssembly::new("Room");
        wall.auto_floor = true;
        let a = wall.add_node(Vec3::new(0.0, 0.0, 0.0));
        let b = wall.add_node(Vec3::new(3.0, 0.0, 0.0));
        let c = wall.add_node(Vec3::new(3.0, 0.0, 2.0));
        let d = wall.add_node(Vec3::new(0.0, 0.0, 2.0));
        wall.add_span(a, b).unwrap();
        wall.add_span(b, c).unwrap();
        wall.add_span(c, d).unwrap();

        let floor = wall.structural_floor_geometry().unwrap();
        assert!(floor.vertices.len() >= 4);
        assert_eq!(floor.faces.len(), 1);
        assert_eq!(floor.properties.get_bool("wall_auto_floor"), Some(true));

        wall.add_span(d, a).unwrap();
        assert!(wall.structural_floor_geometry().is_some());
    }

    #[test]
    fn closed_concave_floor_preserves_the_wall_path_instead_of_using_its_hull() {
        let mut wall = WallAssembly::new("Concave room");
        let nodes = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 3.0),
            Vec3::new(2.0, 0.0, 3.0),
            Vec3::new(2.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, 1.0),
        ]
        .into_iter()
        .map(|position| wall.add_node(position))
        .collect::<Vec<_>>();
        for index in 0..nodes.len() {
            wall.add_span(nodes[index], nodes[(index + 1) % nodes.len()])
                .unwrap();
        }

        let outline = wall.closed_floor_outline().unwrap();
        assert_eq!(outline.len(), nodes.len());
        assert!(
            outline
                .iter()
                .any(|point| (point.x - 2.0).abs() < 1e-5 && (point.z - 1.0).abs() < 1e-5)
        );
    }

    #[test]
    fn duplicate_and_self_spans_are_rejected() {
        let mut wall = WallAssembly::new("Wall");
        let a = wall.add_node(Vec3::zero());
        let b = wall.add_node(Vec3::unit_x());
        wall.add_span(a, b).unwrap();
        assert!(wall.add_span(b, a).is_err());
        assert!(wall.add_span(a, a).is_err());
    }

    #[test]
    fn map_finds_shared_nodes_and_merges_connected_networks() {
        let mut map = Map::default();
        let mut first = WallAssembly::new("First");
        let shared = first.add_node(Vec3::new(1.0, 0.0, 0.0));
        let first_id = first.id;
        let mut second = WallAssembly::new("Second");
        second.add_node(Vec3::new(2.0, 0.0, 0.0));
        let second_id = second.id;
        map.wall_assemblies.extend([first, second]);

        assert_eq!(
            map.nearest_wall_node(Vec3::new(1.01, 0.0, 0.0), 0.1),
            Some((first_id, shared))
        );
        map.merge_wall_assemblies(first_id, second_id).unwrap();
        assert_eq!(map.wall_assemblies.len(), 1);
        assert_eq!(map.wall_assemblies[0].nodes.len(), 2);
    }

    #[test]
    fn moving_a_shared_node_reshapes_every_incident_span() {
        let mut wall = WallAssembly::new("Editable graph");
        let west = wall.add_node(Vec3::zero());
        let shared = wall.add_node(Vec3::new(2.0, 0.0, 0.0));
        let north = wall.add_node(Vec3::new(2.0, 0.0, 2.0));
        let horizontal = wall.add_span(west, shared).unwrap();
        let vertical = wall.add_span(shared, north).unwrap();

        wall.set_node_position(shared, Vec3::new(3.0, 0.0, 0.0))
            .unwrap();

        assert_eq!(
            wall.node(shared).unwrap().position,
            Vec3::new(3.0, 0.0, 0.0)
        );
        assert!((wall.span_length(horizontal).unwrap() - 3.0).abs() < 1e-5);
        assert!((wall.span_length(vertical).unwrap() - 5.0_f32.sqrt()).abs() < 1e-5);
        assert_eq!(wall.junction_kind(shared), Some(WallJunctionKind::Corner));
    }

    #[test]
    fn wall_hosted_prefab_follows_span_edits() {
        let mut map = Map::default();
        let mut wall = WallAssembly::new("Host wall");
        wall.style.thickness = 0.4;
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span_id = wall.add_span(start, end).unwrap();
        let assembly_id = wall.id;
        map.wall_assemblies.push(wall);

        let mut instance = crate::BlockPropInstance::new(Uuid::new_v4());
        instance.host_attachment = Some(BlockPropHostAttachment::WallSpan {
            assembly_id,
            span_id,
            along: 2.0,
            height: 1.0,
            side: 1.0,
            offset: 0.1,
            rotation_quarters: 0,
        });
        map.block_prop_instances.push(instance);
        assert_eq!(map.sync_wall_hosted_block_props(), 1);
        let first = map.block_prop_instances[0].world_transform[3];
        assert!((first[0] - 2.0).abs() < 1e-4);
        assert!((first[1] - 1.0).abs() < 1e-4);
        assert!((first[2] - 0.3).abs() < 1e-4);

        map.wall_assembly_mut(assembly_id)
            .unwrap()
            .set_node_position(end, Vec3::new(0.0, 0.0, 4.0))
            .unwrap();
        map.rebuild_wall_geometry();
        let moved = map.block_prop_instances[0].world_transform[3];
        assert!((moved[0] + 0.3).abs() < 1e-4);
        assert!((moved[1] - 1.0).abs() < 1e-4);
        assert!((moved[2] - 2.0).abs() < 1e-4);
    }

    #[test]
    fn mounted_prefab_move_flip_rotate_and_detach_update_host_state() {
        let mut map = Map::default();
        let mut wall = WallAssembly::new("Editable host");
        wall.style.thickness = 0.4;
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span_id = wall.add_span(start, end).unwrap();
        let assembly_id = wall.id;
        map.wall_assemblies.push(wall);
        map.rebuild_wall_geometry();

        let mut instance = crate::BlockPropInstance::new(Uuid::new_v4());
        let instance_id = instance.id;
        instance.host_attachment = Some(BlockPropHostAttachment::WallSpan {
            assembly_id,
            span_id,
            along: 1.0,
            height: 1.0,
            side: 1.0,
            offset: 0.1,
            rotation_quarters: 0,
        });
        map.block_prop_instances.push(instance);
        map.sync_wall_hosted_block_props();

        map.block_prop_instances[0].world_transform[3][0] = 2.25;
        map.block_prop_instances[0].world_transform[3][1] = 1.75;
        map.block_prop_instances[0].world_transform[3][2] = 0.55;
        assert!(map.refresh_wall_host_from_instance_transform(instance_id));
        map.sync_wall_hosted_block_props();
        let Some(BlockPropHostAttachment::WallSpan {
            along,
            height,
            offset,
            ..
        }) = map.block_prop_instances[0].host_attachment.as_ref()
        else {
            panic!("mounted host missing");
        };
        assert!((*along - 2.25).abs() < 1e-4);
        assert!((*height - 1.75).abs() < 1e-4);
        assert!((*offset - 0.35).abs() < 1e-4);

        assert!(map.flip_wall_hosted_block_prop(instance_id));
        assert!(map.block_prop_instances[0].world_transform[3][2] < 0.0);
        assert!(map.rotate_block_prop_placement(instance_id, 1));
        let Some(BlockPropHostAttachment::WallSpan {
            rotation_quarters, ..
        }) = map.block_prop_instances[0].host_attachment.as_ref()
        else {
            panic!("mounted host missing");
        };
        assert_eq!(*rotation_quarters, 1);
        assert!(map.detach_block_prop(instance_id));
        assert!(map.block_prop_instances[0].host_attachment.is_none());
    }

    #[test]
    fn connecting_points_reuses_nodes_and_builds_structural_geometry() {
        let mut map = Map::default();
        let (assembly_id, first_span, _, shared_node) = map
            .connect_wall_points(Vec3::zero(), Vec3::unit_x(), 0.1)
            .unwrap();
        let (continued_assembly, second_span, reused_node, _) = map
            .connect_wall_points(Vec3::new(1.02, 0.0, 0.0), Vec3::unit_z(), 0.1)
            .unwrap();

        assert_eq!(continued_assembly, assembly_id);
        assert_eq!(reused_node, shared_node);
        assert_ne!(first_span, second_span);
        map.rebuild_wall_geometry();
        assert_eq!(map.geometry_objects.len(), 3);
        assert!(
            map.geometry_objects
                .iter()
                .all(|object| object.kind == GeometryObjectKind::Generated)
        );
        assert_eq!(
            map.wall_source_for_geometry_object(map.geometry_objects[0].id),
            Some((assembly_id, first_span))
        );
        assert!(
            map.geometry_objects
                .iter()
                .any(|object| object.properties.get_id("wall_node_id") == Some(shared_node))
        );
    }

    #[test]
    fn bevel_and_variation_are_deterministic() {
        let mut wall = WallAssembly::new("Weathered wall");
        wall.style.bevel = 0.04;
        wall.style.irregularity = 0.35;
        wall.style.damage = 0.2;
        wall.style.variation_seed = 42;
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(2.0, 0.0, 0.0));
        wall.add_span(start, end).unwrap();

        let first = wall.structural_geometry();
        let second = wall.structural_geometry();
        assert_eq!(first, second);
        assert!(first[0].vertices.len() > first[0].faces.len());
    }

    #[test]
    fn masonry_patterns_change_real_unit_layout_without_changing_the_authored_gap() {
        let mut style = WallStyle::default();
        let brick = wall_masonry_layout(&style);
        style.masonry = WallMasonryPattern::StoneBlocks;
        let blocks = wall_masonry_layout(&style);
        style.masonry = WallMasonryPattern::Rubble;
        let rubble = wall_masonry_layout(&style);

        assert!(blocks.stone_width > brick.stone_width);
        assert!(blocks.course_height > brick.course_height);
        assert!(rubble.horizontal_amplitude > blocks.horizontal_amplitude);
        assert!(rubble.vertical_amplitude > blocks.vertical_amplitude);
        assert_eq!(style.mortar_gap, WallStyle::default().mortar_gap);
    }

    #[test]
    fn rectangular_opening_is_a_real_void_in_the_generated_wall() {
        let mut wall = WallAssembly::new("Opening wall");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();

        wall.add_rectangular_opening(span, vek::Vec2::new(1.0, 0.0), vek::Vec2::new(3.0, 2.0))
            .unwrap();
        let object = wall.structural_geometry().pop().unwrap();

        assert_eq!(wall.span(span).unwrap().openings.len(), 1);
        assert!(object.vertices.len() > 8);
        let source = wall.span(span).unwrap();
        assert!(wall_span_void_contains(
            source,
            &wall.style,
            4.0,
            vek::Vec2::new(2.0, 1.0)
        ));
    }

    #[test]
    fn masonry_courses_are_addressable_and_removable() {
        let mut wall = WallAssembly::new("Masonry");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(3.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let key = wall.brick_at(span, vek::Vec2::new(0.25, 0.1)).unwrap();
        assert_eq!(
            key,
            WallBrickKey {
                course: 0,
                index: 0
            }
        );
        assert!(wall.brick_at(span, vek::Vec2::new(0.50625, 0.1)).is_none());

        wall.set_brick_removed(span, key, true).unwrap();
        let after = wall.structural_geometry().pop().unwrap();
        assert!(wall.span(span).unwrap().removed_bricks.contains(&key));
        assert!(!after.vertices.is_empty());
        assert!(wall_span_void_contains(
            wall.span(span).unwrap(),
            &wall.style,
            3.0,
            vek::Vec2::new(0.25, 0.1)
        ));
    }

    #[test]
    fn masonry_keeps_distinct_stone_and_mortar_sources() {
        let mut wall = WallAssembly::new("Colored masonry");
        wall.style.stone_source = Some(PixelSource::PaletteIndex(7));
        wall.style.stone_variants.clear();
        wall.style.mortar_source = Some(PixelSource::PaletteIndex(2));
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(2.0, 0.0, 0.0));
        wall.add_span(start, end).unwrap();
        let object = wall.structural_geometry().pop().unwrap();

        assert!(
            object
                .faces
                .iter()
                .any(|face| face.tile == wall.style.stone_source)
        );
        assert!(
            object
                .faces
                .iter()
                .any(|face| face.tile == wall.style.mortar_source)
        );
    }

    #[test]
    fn damage_and_irregularity_do_not_inflate_mortar_or_create_a_backing_shell() {
        let mut wall = WallAssembly::new("Full block masonry");
        wall.style.irregularity = 1.0;
        wall.style.damage = 1.0;
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(2.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let key = wall.brick_at(span, vek::Vec2::new(0.25, 0.1)).unwrap();
        wall.set_brick_removed(span, key, true).unwrap();
        let object = wall.structural_geometry().pop().unwrap();
        let mortar_source = wall.style.mortar_pixel_source();
        let maximum_joint_width = wall.style.mortar_gap + 1e-4;
        let maximum_bed_height = wall.style.mortar_gap + 1e-4;

        assert!(object.faces.iter().all(|face| face.tile.is_some()));
        for face in object
            .faces
            .iter()
            .filter(|face| face.tile.as_ref() == Some(&mortar_source))
        {
            let vertices = face
                .indices
                .iter()
                .map(|index| object.vertices[*index])
                .collect::<Vec<_>>();
            let x_min = vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::MAX, f32::min);
            let x_max = vertices
                .iter()
                .map(|vertex| vertex.x)
                .fold(f32::MIN, f32::max);
            let y_min = vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::MAX, f32::min);
            let y_max = vertices
                .iter()
                .map(|vertex| vertex.y)
                .fold(f32::MIN, f32::max);
            assert!(x_max - x_min <= maximum_joint_width || y_max - y_min <= maximum_joint_width);
            if x_min.abs() <= 1e-5 && x_max <= maximum_joint_width {
                assert!(y_max - y_min <= maximum_bed_height);
            }
        }
    }

    #[test]
    fn arched_opening_preserves_its_shoulders_and_clears_its_crown() {
        let mut wall = WallAssembly::new("Arch");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let opening_id = wall
            .add_opening(
                span,
                vek::Vec2::new(1.0, 0.0),
                vek::Vec2::new(3.0, 2.5),
                WallOpeningShape::Arch,
            )
            .unwrap();
        let opening = &wall.span(span).unwrap().openings[0];
        assert!(wall_opening_contains(opening, vek::Vec2::new(2.0, 2.3)));
        assert!(!wall_opening_contains(opening, vek::Vec2::new(1.1, 2.3)));
        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(2.0, 2.3)),
            Some(WallGeometryLayer::Void(opening_id))
        );
        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(2.0, 2.58)),
            Some(WallGeometryLayer::OpeningSurround(opening_id))
        );
        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(0.5, 2.3)),
            Some(WallGeometryLayer::Masonry)
        );
        assert!(wall.brick_at(span, vek::Vec2::new(2.0, 2.58)).is_none());
    }

    #[test]
    fn opening_frame_overrides_generate_fitted_material_geometry() {
        let mut wall = WallAssembly::new("Special arch frame");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let opening_id = wall
            .add_opening(
                span,
                vek::Vec2::new(1.0, 0.0),
                vek::Vec2::new(3.0, 2.5),
                WallOpeningShape::Arch,
            )
            .unwrap();
        let frame_source = PixelSource::PaletteIndex(13);
        let opening = wall.opening_mut(span, opening_id).unwrap();
        opening.frame.width = Some(0.3);
        opening.frame.depth = Some(0.06);
        opening.frame.arch_stones = Some(7);
        opening.frame.source = Some(frame_source.clone());

        let object = wall.structural_geometry().pop().unwrap();
        let frame_faces = object
            .faces
            .iter()
            .filter(|face| face.tile.as_ref() == Some(&frame_source))
            .count();
        assert!(frame_faces >= 7 * 6);
        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(2.0, 2.7)),
            Some(WallGeometryLayer::OpeningSurround(opening_id))
        );
    }

    #[test]
    fn arch_hole_and_block_arch_surround_are_independent() {
        let mut wall = WallAssembly::new("Dungeon doorway");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let opening_id = wall
            .add_opening(
                span,
                vek::Vec2::new(1.0, 0.0),
                vek::Vec2::new(3.0, 2.5),
                WallOpeningShape::Arch,
            )
            .unwrap();
        let arch_source = PixelSource::PaletteIndex(14);
        let opening = wall.opening_mut(span, opening_id).unwrap();
        opening.frame.source = Some(arch_source.clone());
        opening.frame.surround = WallOpeningSurround::None;

        let plain_hole = wall.structural_geometry().pop().unwrap();
        assert!(
            plain_hole
                .faces
                .iter()
                .all(|face| face.tile.as_ref() != Some(&arch_source))
        );
        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(2.0, 2.58)),
            Some(WallGeometryLayer::Masonry)
        );

        wall.opening_mut(span, opening_id).unwrap().frame.surround = WallOpeningSurround::Blocks;
        let block_arch = wall.structural_geometry().pop().unwrap();
        assert!(
            block_arch
                .faces
                .iter()
                .any(|face| face.tile.as_ref() == Some(&arch_source))
        );
        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(2.0, 2.58)),
            Some(WallGeometryLayer::OpeningSurround(opening_id))
        );
    }

    #[test]
    fn openings_are_addressable_and_arch_radius_is_independently_editable() {
        let mut wall = WallAssembly::new("Editable doorway");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let opening_id = wall
            .add_opening(
                span,
                vek::Vec2::new(1.0, 0.0),
                vek::Vec2::new(3.0, 2.5),
                WallOpeningShape::Arch,
            )
            .unwrap();

        assert_eq!(
            wall.opening_at(span, vek::Vec2::new(2.0, 1.0)),
            Some(opening_id)
        );
        assert!(wall_opening_contains(
            wall.opening(span, opening_id).unwrap(),
            vek::Vec2::new(2.6, 2.2)
        ));
        assert!(!wall_opening_contains(
            wall.opening(span, opening_id).unwrap(),
            vek::Vec2::new(2.8, 2.2)
        ));

        let opening = wall.opening_mut(span, opening_id).unwrap();
        opening.arch_radius = Some(0.4);
        opening.frame.surround = WallOpeningSurround::None;
        // Changing the rise keeps the arch connected to both jambs; it creates a flatter
        // elliptical crown instead of narrowing the hole into a disconnected small circle.
        assert!(wall_opening_contains(
            wall.opening(span, opening_id).unwrap(),
            vek::Vec2::new(2.8, 2.2)
        ));

        assert!(wall.remove_opening(span, opening_id));
        assert!(wall.opening(span, opening_id).is_none());
        assert_eq!(wall.opening_at(span, vek::Vec2::new(2.0, 1.0)), None);
    }

    #[test]
    fn arch_partition_cells_cannot_overlap_the_curved_surround() {
        let mut wall = WallAssembly::new("Clean arch compositor");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span_id = wall.add_span(start, end).unwrap();
        wall.add_opening(
            span_id,
            vek::Vec2::new(1.0, 0.0),
            vek::Vec2::new(3.0, 2.5),
            WallOpeningShape::Arch,
        )
        .unwrap();
        let span = wall.span(span_id).unwrap();

        // This cell only clips the crown between its sampled corners and center. The old
        // five-point test kept it as masonry, leaving a triangular bevel under the arch blocks.
        assert!(!wall_cell_is_masonry(
            span,
            &wall.style,
            1.5,
            2.5,
            2.67,
            2.69,
        ));
        assert!(wall_cell_is_masonry(
            span,
            &wall.style,
            0.0,
            0.5,
            2.67,
            2.69,
        ));
    }

    #[test]
    fn convex_opening_subtraction_preserves_only_the_exterior_profile() {
        let subject = vec![
            vek::Vec2::new(0.0, 0.0),
            vek::Vec2::new(2.0, 0.0),
            vek::Vec2::new(2.0, 1.0),
            vek::Vec2::new(0.0, 1.0),
        ];
        let clip = vec![
            vek::Vec2::new(1.0, -1.0),
            vek::Vec2::new(3.0, -1.0),
            vek::Vec2::new(3.0, 2.0),
            vek::Vec2::new(1.0, 2.0),
        ];
        let pieces = subtract_convex_wall_profile(subject, &clip);
        let remaining_area = pieces
            .iter()
            .map(|piece| wall_polygon_area(piece).abs())
            .sum::<f32>();

        assert!((remaining_area - 1.0).abs() < 1e-5);
        assert!(pieces.iter().flatten().all(|point| point.x <= 1.0 + 1e-5));
    }

    #[test]
    fn arch_subtraction_emits_the_shared_sloped_boundary_not_stair_steps() {
        let mut wall = WallAssembly::new("Polygonal arch cut");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let opening_id = wall
            .add_opening(
                span,
                vek::Vec2::new(1.0, 0.0),
                vek::Vec2::new(3.0, 2.5),
                WallOpeningShape::Arch,
            )
            .unwrap();
        let opening = wall.opening(span, opening_id).unwrap();
        let arch = wall_opening_outer_profile(opening, &wall.style);
        let subject = vec![
            vek::Vec2::new(1.0, 2.2),
            vek::Vec2::new(3.0, 2.2),
            vek::Vec2::new(3.0, 2.8),
            vek::Vec2::new(1.0, 2.8),
        ];
        let pieces = subtract_convex_wall_profile(subject, &arch);

        assert!(!pieces.is_empty());
        assert!(pieces.iter().any(|piece| {
            piece
                .iter()
                .zip(piece.iter().cycle().skip(1))
                .take(piece.len())
                .any(|(a, b)| (a.x - b.x).abs() > 1e-4 && (a.y - b.y).abs() > 1e-4)
        }));
    }

    #[test]
    fn disabled_opening_frame_returns_its_surround_to_masonry() {
        let mut wall = WallAssembly::new("Unframed opening");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span = wall.add_span(start, end).unwrap();
        let opening_id = wall
            .add_rectangular_opening(span, vek::Vec2::new(1.0, 0.0), vek::Vec2::new(3.0, 2.0))
            .unwrap();
        wall.opening_mut(span, opening_id).unwrap().frame.enabled = false;

        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(0.9, 1.0)),
            Some(WallGeometryLayer::Masonry)
        );
        assert_eq!(
            wall.span_geometry_layer_at(span, vek::Vec2::new(2.0, 1.0)),
            Some(WallGeometryLayer::Void(opening_id))
        );
    }

    #[test]
    fn span_coordinates_round_trip_between_wall_and_world_space() {
        let mut wall = WallAssembly::new("Coordinates");
        let start = wall.add_node(Vec3::new(2.0, 0.5, 4.0));
        let end = wall.add_node(Vec3::new(2.0, 0.5, 9.0));
        let span = wall.add_span(start, end).unwrap();
        let local = vek::Vec2::new(1.75, 1.2);
        let world = wall.span_point(span, local).unwrap();

        let round_trip = wall.span_coordinates(span, world).unwrap();
        assert!((round_trip - local).magnitude() < 1e-5);
    }

    #[test]
    fn curved_span_keeps_distance_coordinates_and_generates_masonry() {
        let mut wall = WallAssembly::new("Curved fireplace wall");
        let start = wall.add_node(Vec3::zero());
        let end = wall.add_node(Vec3::new(4.0, 0.0, 0.0));
        let span_id = wall.add_span(start, end).unwrap();
        let span = wall.span_mut(span_id).unwrap();
        span.curve_offset = 1.25;
        span.curve_segments = 16;

        let length = wall.span_length(span_id).unwrap();
        assert!(length > 4.0);
        let midpoint = wall
            .span_point(span_id, vek::Vec2::new(length * 0.5, 0.0))
            .unwrap();
        assert!(midpoint.z > 0.5);
        let local = wall.span_coordinates(span_id, midpoint).unwrap();
        assert!((local.x - length * 0.5).abs() < 0.05);
        assert!(!wall.structural_geometry()[0].faces.is_empty());
    }

    #[test]
    fn corner_bond_alternates_which_wall_runs_through() {
        let mut wall = WallAssembly::new("Bonded corner");
        let center = wall.add_node(Vec3::zero());
        let east = wall.add_node(Vec3::unit_x());
        let north = wall.add_node(Vec3::unit_z());
        let east_span_id = wall.add_span(center, east).unwrap();
        let north_span_id = wall.add_span(center, north).unwrap();
        let east_span = wall.span(east_span_id).unwrap();
        let north_span = wall.span(north_span_id).unwrap();

        let east_first = wall.junction_inset_for_course(east_span, center, 0, &wall.style);
        let north_first = wall.junction_inset_for_course(north_span, center, 0, &wall.style);
        let east_second = wall.junction_inset_for_course(east_span, center, 1, &wall.style);
        let north_second = wall.junction_inset_for_course(north_span, center, 1, &wall.style);
        assert!(east_first > north_first);
        assert!(east_second < north_second);
    }

    #[test]
    fn merging_assemblies_preserves_the_source_spans_style() {
        let mut map = Map::default();
        let mut target = WallAssembly::new("Thick");
        target.style.thickness = 0.75;
        let target_a = target.add_node(Vec3::zero());
        let target_b = target.add_node(Vec3::unit_x());
        target.add_span(target_a, target_b).unwrap();
        let target_id = target.id;

        let mut source = WallAssembly::new("Thin");
        source.style.thickness = 0.2;
        let source_a = source.add_node(Vec3::unit_z());
        let source_b = source.add_node(Vec3::new(2.0, 0.0, 1.0));
        let source_span = source.add_span(source_a, source_b).unwrap();
        let source_id = source.id;
        map.wall_assemblies.extend([target, source]);

        map.merge_wall_assemblies(target_id, source_id).unwrap();
        let merged = map.wall_assembly(target_id).unwrap();
        assert_eq!(
            merged
                .spans
                .iter()
                .find(|span| span.id == source_span)
                .and_then(|span| span.style_override.as_ref())
                .map(|style| style.thickness),
            Some(0.2)
        );
    }
}
