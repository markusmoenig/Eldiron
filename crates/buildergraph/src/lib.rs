use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use vek::{Vec2, Vec3};

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderPreview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderGraph {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "default_graph_name")]
    pub name: String,
    #[serde(default)]
    pub connections: Vec<(u16, u8, u16, u8)>,
    #[serde(default = "default_builder_nodes")]
    pub nodes: Vec<BuilderNode>,
    #[serde(default)]
    pub selected_node: Option<usize>,
    #[serde(default)]
    pub scroll_offset: Vec2<i32>,
    #[serde(default)]
    pub preview_host: Option<BuilderPreviewHost>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderPreviewHost {
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub depth: f32,
    #[serde(default)]
    pub height: f32,
    #[serde(default)]
    pub surface: BuilderPreviewSurface,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuilderPreviewSurface {
    #[default]
    Floor,
    Wall,
}

impl Default for BuilderPreviewHost {
    fn default() -> Self {
        Self {
            width: 1.0,
            depth: 1.0,
            height: 1.0,
            surface: BuilderPreviewSurface::Floor,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderScript {
    pub name: String,
    pub host: BuilderScriptHost,
    pub preview_host: BuilderPreviewHost,
    pub params: Vec<BuilderScriptParam>,
    pub parts: Vec<BuilderScriptPart>,
    pub cuts: Vec<BuilderScriptCut>,
    pub details: Vec<BuilderScriptSurfaceDetail>,
    pub slots: Vec<BuilderScriptSlot>,
    pub output: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderDocument {
    Script(BuilderScript),
    Graph(BuilderGraph),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuilderScriptHost {
    Line,
    Sector,
    Vertex,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderScriptParam {
    pub name: String,
    pub value: BuilderScriptParamValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptParamValue {
    Scalar(BuilderScriptScalarExpr),
    Ident(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptParameterValue {
    Number(f32),
    Ident(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptPlacementExpr {
    Literal(BuilderDetailPlacement),
    Param(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderScriptPart {
    pub name: String,
    pub kind: BuilderScriptPartKind,
    pub attach: BuilderScriptPointExpr,
    pub parent: Option<BuilderScriptRef>,
    pub material: Option<String>,
    pub axis: Option<BuilderScriptRef>,
    pub rotate_x: Option<BuilderScriptScalarExpr>,
    pub rotate_y: Option<BuilderScriptScalarExpr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptPartKind {
    Box {
        size: [BuilderScriptScalarExpr; 3],
    },
    Cylinder {
        length: BuilderScriptScalarExpr,
        radius: BuilderScriptScalarExpr,
    },
    Planks {
        size: [BuilderScriptScalarExpr; 3],
        count: BuilderScriptScalarExpr,
        direction: BuilderPlankDirection,
        jitter: Option<BuilderScriptScalarExpr>,
        alignment_jitter: Option<BuilderScriptScalarExpr>,
        missing_chance: Option<BuilderScriptScalarExpr>,
        seed: Option<BuilderScriptScalarExpr>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptCut {
    Rect {
        min: [BuilderScriptScalarExpr; 2],
        max: [BuilderScriptScalarExpr; 2],
        mode: BuilderCutMode,
        offset: Option<BuilderScriptScalarExpr>,
        inset: Option<BuilderScriptScalarExpr>,
        shape: BuilderCutShape,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptSurfaceDetail {
    Rect {
        min: [BuilderScriptScalarExpr; 2],
        max: [BuilderScriptScalarExpr; 2],
        offset: Option<BuilderScriptScalarExpr>,
        inset: Option<BuilderScriptScalarExpr>,
        shape: BuilderCutShape,
        material: Option<String>,
        tile_alias: Option<String>,
    },
    Masonry {
        min: [BuilderScriptScalarExpr; 2],
        max: [BuilderScriptScalarExpr; 2],
        block: [BuilderScriptScalarExpr; 2],
        mortar: Option<BuilderScriptScalarExpr>,
        offset: Option<BuilderScriptScalarExpr>,
        pattern: BuilderMasonryPattern,
        material: Option<String>,
        tile_alias: Option<String>,
    },
    Planks {
        min: [BuilderScriptScalarExpr; 2],
        max: [BuilderScriptScalarExpr; 2],
        count: BuilderScriptScalarExpr,
        direction: BuilderPlankDirection,
        jitter: Option<BuilderScriptScalarExpr>,
        alignment_jitter: Option<BuilderScriptScalarExpr>,
        missing_chance: Option<BuilderScriptScalarExpr>,
        seed: Option<BuilderScriptScalarExpr>,
        offset: Option<BuilderScriptScalarExpr>,
        material: Option<String>,
        tile_alias: Option<String>,
    },
    Column {
        center: [BuilderScriptScalarExpr; 2],
        height: BuilderScriptScalarExpr,
        radius: BuilderScriptScalarExpr,
        offset: Option<BuilderScriptScalarExpr>,
        base_height: Option<BuilderScriptScalarExpr>,
        cap_height: Option<BuilderScriptScalarExpr>,
        transition_height: Option<BuilderScriptScalarExpr>,
        segments: Option<BuilderScriptScalarExpr>,
        placement: BuilderScriptPlacementExpr,
        cut_footprint: bool,
        material: Option<String>,
        rect_material: Option<String>,
        cyl_material: Option<String>,
        tile_alias: Option<String>,
    },
    ColumnSeries {
        start: BuilderScriptScalarExpr,
        end: BuilderScriptScalarExpr,
        y: BuilderScriptScalarExpr,
        spacing: BuilderScriptScalarExpr,
        height: BuilderScriptScalarExpr,
        radius: BuilderScriptScalarExpr,
        broken_chance: Option<BuilderScriptScalarExpr>,
        broken_min_height: Option<BuilderScriptScalarExpr>,
        seed: Option<BuilderScriptScalarExpr>,
        offset: Option<BuilderScriptScalarExpr>,
        base_height: Option<BuilderScriptScalarExpr>,
        cap_height: Option<BuilderScriptScalarExpr>,
        transition_height: Option<BuilderScriptScalarExpr>,
        segments: Option<BuilderScriptScalarExpr>,
        placement: BuilderScriptPlacementExpr,
        cut_footprint: bool,
        material: Option<String>,
        rect_material: Option<String>,
        cyl_material: Option<String>,
        tile_alias: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderScriptSlot {
    pub name: String,
    pub kind: BuilderAttachmentKind,
    pub source: BuilderScriptRef,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderScriptPointExpr {
    pub terms: Vec<(f32, BuilderScriptVecExpr)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptVecExpr {
    Ref(BuilderScriptRef),
    ScaledRef(BuilderScriptRef, BuilderScriptScalarExpr),
    Literal([BuilderScriptScalarExpr; 3]),
}

#[derive(Clone, Debug, PartialEq)]
pub enum BuilderScriptScalarExpr {
    Constant(f32),
    Ref(BuilderScriptRef),
    Add(Box<BuilderScriptScalarExpr>, Box<BuilderScriptScalarExpr>),
    Sub(Box<BuilderScriptScalarExpr>, Box<BuilderScriptScalarExpr>),
    Mul(Box<BuilderScriptScalarExpr>, Box<BuilderScriptScalarExpr>),
    Div(Box<BuilderScriptScalarExpr>, Box<BuilderScriptScalarExpr>),
    Neg(Box<BuilderScriptScalarExpr>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BuilderScriptRef {
    Host(String),
    Param(String),
    Part(String, String),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderNode {
    pub id: u16,
    #[serde(default)]
    pub name: String,
    pub kind: BuilderNodeKind,
    #[serde(default)]
    pub pos: Vec2<i32>,
    #[serde(default)]
    pub preview_collapsed: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuilderNodeKind {
    SectorSurface,
    LinedefSurface,
    VertexPoint,
    Offset {
        #[serde(default = "default_translate")]
        translate: Vec3<f32>,
    },
    CornerLayout {
        #[serde(default = "default_corner_inset")]
        inset_x: f32,
        #[serde(default = "default_corner_inset")]
        inset_z: f32,
    },
    Box {
        width: f32,
        depth: f32,
        height: f32,
    },
    Cylinder {
        length: f32,
        radius: f32,
    },
    SectorCorners {
        #[serde(default = "default_corner_inset")]
        inset_x: f32,
        #[serde(default = "default_corner_inset")]
        inset_z: f32,
        #[serde(default)]
        elevation: f32,
    },
    SectorGrid {
        #[serde(default = "default_grid_count")]
        columns: u16,
        #[serde(default = "default_grid_count")]
        rows: u16,
        #[serde(default)]
        inset_x: f32,
        #[serde(default)]
        inset_z: f32,
        #[serde(default)]
        elevation: f32,
    },
    SectorEdges {
        #[serde(default = "default_true")]
        north: bool,
        #[serde(default = "default_true")]
        south: bool,
        #[serde(default = "default_true")]
        east: bool,
        #[serde(default = "default_true")]
        west: bool,
        #[serde(default)]
        inset: f32,
        #[serde(default)]
        elevation: f32,
    },
    LinedefRow {
        #[serde(default = "default_grid_count")]
        count: u16,
        #[serde(default)]
        inset: f32,
        #[serde(default)]
        elevation: f32,
    },
    LinedefSpan {
        #[serde(default)]
        inset: f32,
        #[serde(default)]
        elevation: f32,
    },
    ItemAnchor {
        #[serde(default = "default_item_slot_name")]
        name: String,
    },
    ItemSurface {
        #[serde(default = "default_item_slot_name")]
        name: String,
    },
    MaterialAnchor {
        #[serde(default = "default_material_slot_name")]
        name: String,
    },
    ItemSlot {
        #[serde(default = "default_item_slot_name")]
        name: String,
        #[serde(default = "default_translate")]
        position: Vec3<f32>,
    },
    MaterialSlot {
        #[serde(default = "default_material_slot_name")]
        name: String,
        #[serde(default = "default_translate")]
        position: Vec3<f32>,
    },
    Join,
    GeometryOutput {
        #[serde(default)]
        target: BuilderOutputTarget,
        #[serde(default = "default_output_host_refs")]
        host_refs: u8,
    },
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuilderOutputTarget {
    #[default]
    Sector,
    VertexPair,
    Linedef,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuilderOutputSpec {
    pub target: BuilderOutputTarget,
    pub host_refs: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BuilderHost {
    Object(BuilderObjectHost),
    Sector(BuilderSectorHost),
    Linedef(BuilderLinedefHost),
    Vertex(BuilderVertexHost),
    Surface(BuilderSurfaceHost),
    Terrain(BuilderTerrainHost),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderObjectHost {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_unit")]
    pub width: f32,
    #[serde(default = "default_unit")]
    pub depth: f32,
    #[serde(default = "default_unit")]
    pub height: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderSectorHost {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_unit")]
    pub width: f32,
    #[serde(default = "default_unit")]
    pub depth: f32,
    #[serde(default = "default_unit")]
    pub height: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderLinedefHost {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_unit")]
    pub length: f32,
    #[serde(default = "default_unit")]
    pub height: f32,
    #[serde(default)]
    pub width: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderVertexHost {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_unit")]
    pub width: f32,
    #[serde(default = "default_unit")]
    pub depth: f32,
    #[serde(default = "default_unit")]
    pub height: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderSurfaceHost {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_unit")]
    pub width: f32,
    #[serde(default = "default_unit")]
    pub height: f32,
    #[serde(default)]
    pub thickness: f32,
    #[serde(default)]
    pub side: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderTerrainHost {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_terrain_size")]
    pub width: f32,
    #[serde(default = "default_terrain_size")]
    pub depth: f32,
    #[serde(default)]
    pub height: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq)]
pub struct BuilderAssembly {
    #[serde(default)]
    pub primitives: Vec<BuilderPrimitive>,
    #[serde(default)]
    pub anchors: Vec<BuilderAnchor>,
    #[serde(default)]
    pub cuts: Vec<BuilderCutMask>,
    #[serde(default)]
    pub surface_details: Vec<BuilderSurfaceDetail>,
    #[serde(default)]
    pub static_billboards: Vec<BuilderStaticBillboardBatch>,
    #[serde(default)]
    pub warnings: Vec<BuilderWarning>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuilderPrimitive {
    Box {
        size: Vec3<f32>,
        transform: BuilderTransform,
        material_slot: Option<String>,
        host_position_normalized: bool,
        host_position_y_normalized: bool,
        host_scale_y_normalized: bool,
        host_scale_x_normalized: bool,
        host_scale_z_normalized: bool,
    },
    Cylinder {
        length: f32,
        radius: f32,
        transform: BuilderTransform,
        material_slot: Option<String>,
        host_position_normalized: bool,
        host_position_y_normalized: bool,
        host_scale_y_normalized: bool,
        host_scale_x_normalized: bool,
        host_scale_z_normalized: bool,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderAnchor {
    pub name: String,
    pub kind: BuilderAttachmentKind,
    pub transform: BuilderTransform,
    pub host_position_normalized: bool,
    pub host_position_y_normalized: bool,
    pub surface_extent: Vec2<f32>,
    pub surface_extent_normalized: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuilderCutMask {
    Rect {
        min: Vec2<f32>,
        max: Vec2<f32>,
        mode: BuilderCutMode,
        #[serde(default)]
        offset: f32,
        #[serde(default)]
        inset: f32,
        #[serde(default)]
        shape: BuilderCutShape,
    },
    Loop {
        points: Vec<Vec2<f32>>,
        mode: BuilderCutMode,
        #[serde(default)]
        offset: f32,
        #[serde(default)]
        inset: f32,
        #[serde(default)]
        shape: BuilderCutShape,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BuilderSurfaceDetail {
    Rect {
        min: Vec2<f32>,
        max: Vec2<f32>,
        #[serde(default)]
        offset: f32,
        #[serde(default)]
        inset: f32,
        #[serde(default)]
        shape: BuilderCutShape,
        material_slot: Option<String>,
        tile_alias: Option<String>,
    },
    Column {
        center: Vec2<f32>,
        height: f32,
        radius: f32,
        #[serde(default)]
        offset: f32,
        #[serde(default)]
        base_height: f32,
        #[serde(default)]
        cap_height: f32,
        #[serde(default)]
        transition_height: f32,
        #[serde(default = "default_column_segments")]
        segments: u16,
        #[serde(default)]
        placement: BuilderDetailPlacement,
        #[serde(default)]
        cut_footprint: bool,
        material_slot: Option<String>,
        rect_material_slot: Option<String>,
        cyl_material_slot: Option<String>,
        tile_alias: Option<String>,
    },
    Masonry {
        min: Vec2<f32>,
        max: Vec2<f32>,
        block: Vec2<f32>,
        #[serde(default)]
        mortar: f32,
        #[serde(default)]
        offset: f32,
        #[serde(default)]
        pattern: BuilderMasonryPattern,
        material_slot: Option<String>,
        tile_alias: Option<String>,
    },
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuilderDetailPlacement {
    #[default]
    Relief,
    Attached,
    Structural,
    Freestanding,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuilderMasonryPattern {
    #[default]
    Grid,
    RunningBond,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuilderPlankDirection {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuilderCutMode {
    Cut,
    Replace,
    CutOverlay,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuilderCutShape {
    #[default]
    Fill,
    Border,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderStaticBillboardBatch {
    pub material_slot: Option<String>,
    #[serde(default)]
    pub instances: Vec<BuilderStaticBillboard>,
    pub facing: BuilderBillboardFacing,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderStaticBillboard {
    pub position: Vec3<f32>,
    pub size: Vec2<f32>,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default = "default_tint")]
    pub tint: [f32; 4],
    #[serde(default)]
    pub variant: u16,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuilderBillboardFacing {
    Camera,
    AxialY,
    FixedCross,
    GroundAligned,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderWarning {
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuilderAttachmentKind {
    Item,
    Material,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub struct BuilderTransform {
    pub translation: Vec3<f32>,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub scale: Vec3<f32>,
}

fn default_output_host_refs() -> u8 {
    1
}

fn default_unit() -> f32 {
    1.0
}

fn default_terrain_size() -> f32 {
    16.0
}

fn default_tint() -> [f32; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

fn default_corner_inset() -> f32 {
    0.10
}

fn default_grid_count() -> u16 {
    1
}

fn default_column_segments() -> u16 {
    12
}

fn parse_placement_ident(name: &str) -> Option<BuilderDetailPlacement> {
    match name {
        "relief" => Some(BuilderDetailPlacement::Relief),
        "attached" | "attach" | "mounted" => Some(BuilderDetailPlacement::Attached),
        "structural" | "embedded" | "inline" => Some(BuilderDetailPlacement::Structural),
        "freestanding" | "free" => Some(BuilderDetailPlacement::Freestanding),
        _ => None,
    }
}

fn eval_placement_expr(
    expr: &BuilderScriptPlacementExpr,
    ident_params: &HashMap<String, String>,
) -> Result<BuilderDetailPlacement, String> {
    match expr {
        BuilderScriptPlacementExpr::Literal(placement) => Ok(*placement),
        BuilderScriptPlacementExpr::Param(name) => {
            let value = ident_params
                .get(name)
                .ok_or_else(|| format!("unknown builder placement parameter '{name}'"))?;
            parse_placement_ident(value)
                .ok_or_else(|| format!("unsupported detail placement parameter '{name} = {value}'"))
        }
    }
}

fn builder_param_rand01(seed: u64, index: u64) -> f32 {
    let mut x = seed
        .wrapping_add(index.wrapping_mul(0x9e3779b97f4a7c15))
        .wrapping_add(0xbf58476d1ce4e5b9);
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^= x >> 31;
    ((x >> 40) as f32) / ((1u64 << 24) as f32)
}

fn plank_detail_rects(
    min: Vec2<f32>,
    max: Vec2<f32>,
    count: usize,
    direction: BuilderPlankDirection,
    jitter: f32,
    alignment_jitter: f32,
    missing_chance: f32,
    seed: u64,
) -> Vec<(Vec2<f32>, Vec2<f32>)> {
    let raw_min = min;
    let raw_max = max;
    let min = Vec2::new(raw_min.x.min(raw_max.x), raw_min.y.min(raw_max.y));
    let max = Vec2::new(raw_min.x.max(raw_max.x), raw_min.y.max(raw_max.y));
    if max.x <= min.x || max.y <= min.y || count == 0 {
        return Vec::new();
    }

    let mut rects = Vec::new();
    let span = max - min;
    match direction {
        BuilderPlankDirection::Horizontal => {
            let lane = span.y / count as f32;
            let gap = (lane * 0.08).min(0.035);
            let end_jitter = span.x * 0.04 * jitter;
            for index in 0..count {
                if missing_chance > 0.0 && builder_param_rand01(seed, index as u64) < missing_chance
                {
                    continue;
                }
                let y_rand = builder_param_rand01(seed ^ 0x517cc1b727220a95, index as u64);
                let h_rand = builder_param_rand01(seed ^ 0x94d049bb133111eb, index as u64);
                let y0 = min.y
                    + index as f32 * lane
                    + gap * 0.5
                    + (y_rand - 0.5) * lane * 0.35 * alignment_jitter;
                let y1 = (y0 + lane * (0.72 + h_rand * 0.18) - gap).min(max.y);
                let x0 = min.x
                    + builder_param_rand01(seed ^ 0xda942042e4dd58b5, index as u64) * end_jitter;
                let x1 = max.x
                    - builder_param_rand01(seed ^ 0xa24baed4963ee407, index as u64) * end_jitter;
                if x1 > x0 + 0.001 && y1 > y0 + 0.001 {
                    rects.push((Vec2::new(x0, y0), Vec2::new(x1, y1)));
                }
            }
        }
        BuilderPlankDirection::Vertical => {
            let lane = span.x / count as f32;
            let gap = (lane * 0.08).min(0.035);
            let end_jitter = span.y * 0.04 * jitter;
            for index in 0..count {
                if missing_chance > 0.0 && builder_param_rand01(seed, index as u64) < missing_chance
                {
                    continue;
                }
                let x_rand = builder_param_rand01(seed ^ 0x517cc1b727220a95, index as u64);
                let w_rand = builder_param_rand01(seed ^ 0x94d049bb133111eb, index as u64);
                let x0 = min.x
                    + index as f32 * lane
                    + gap * 0.5
                    + (x_rand - 0.5) * lane * 0.35 * alignment_jitter;
                let x1 = (x0 + lane * (0.72 + w_rand * 0.18) - gap).min(max.x);
                let y0 = min.y
                    + builder_param_rand01(seed ^ 0xda942042e4dd58b5, index as u64) * end_jitter;
                let y1 = max.y
                    - builder_param_rand01(seed ^ 0xa24baed4963ee407, index as u64) * end_jitter;
                if x1 > x0 + 0.001 && y1 > y0 + 0.001 {
                    rects.push((Vec2::new(x0, y0), Vec2::new(x1, y1)));
                }
            }
        }
    }
    rects
}

fn plank_primitive_boxes(
    size: Vec3<f32>,
    count: usize,
    direction: BuilderPlankDirection,
    jitter: f32,
    alignment_jitter: f32,
    missing_chance: f32,
    seed: u64,
) -> Vec<(Vec3<f32>, Vec3<f32>)> {
    let size = Vec3::new(size.x.max(0.001), size.y.max(0.001), size.z.max(0.001));
    if count == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    match direction {
        BuilderPlankDirection::Horizontal => {
            let lane = size.y / count as f32;
            let gap = (lane * 0.08).min(0.035);
            let end_jitter = size.x * 0.04 * jitter;
            for index in 0..count {
                if missing_chance > 0.0 && builder_param_rand01(seed, index as u64) < missing_chance
                {
                    continue;
                }
                let h_rand = builder_param_rand01(seed ^ 0x94d049bb133111eb, index as u64);
                let plank_h = (lane * (0.72 + h_rand * 0.18) - gap).max(0.002);
                let x_trim_a =
                    builder_param_rand01(seed ^ 0xda942042e4dd58b5, index as u64) * end_jitter;
                let x_trim_b =
                    builder_param_rand01(seed ^ 0xa24baed4963ee407, index as u64) * end_jitter;
                let plank_w = (size.x - x_trim_a - x_trim_b).max(0.002);
                let y_shift = (builder_param_rand01(seed ^ 0x517cc1b727220a95, index as u64)
                    - 0.5)
                    * lane
                    * 0.35
                    * alignment_jitter;
                let local_bottom = Vec3::new(
                    (x_trim_a - x_trim_b) * 0.5,
                    index as f32 * lane + gap * 0.5 + y_shift,
                    0.0,
                );
                out.push((local_bottom, Vec3::new(plank_w, plank_h, size.z)));
            }
        }
        BuilderPlankDirection::Vertical => {
            let lane = size.x / count as f32;
            let gap = (lane * 0.08).min(0.035);
            let end_jitter = size.y * 0.04 * jitter;
            for index in 0..count {
                if missing_chance > 0.0 && builder_param_rand01(seed, index as u64) < missing_chance
                {
                    continue;
                }
                let w_rand = builder_param_rand01(seed ^ 0x94d049bb133111eb, index as u64);
                let plank_w = (lane * (0.72 + w_rand * 0.18) - gap).max(0.002);
                let y_trim_a =
                    builder_param_rand01(seed ^ 0xda942042e4dd58b5, index as u64) * end_jitter;
                let y_trim_b =
                    builder_param_rand01(seed ^ 0xa24baed4963ee407, index as u64) * end_jitter;
                let plank_h = (size.y - y_trim_a - y_trim_b).max(0.002);
                let x_shift = (builder_param_rand01(seed ^ 0x517cc1b727220a95, index as u64)
                    - 0.5)
                    * lane
                    * 0.35
                    * alignment_jitter;
                let local_bottom = Vec3::new(
                    -size.x * 0.5 + index as f32 * lane + lane * 0.5 + x_shift,
                    y_trim_a,
                    0.0,
                );
                out.push((local_bottom, Vec3::new(plank_w, plank_h, size.z)));
            }
        }
    }
    out
}

fn default_true() -> bool {
    true
}

fn default_material_slot_name() -> String {
    "material_top".to_string()
}

fn default_item_slot_name() -> String {
    "item_slot".to_string()
}

impl Default for BuilderTransform {
    fn default() -> Self {
        Self::identity()
    }
}

impl BuilderTransform {
    pub fn identity() -> Self {
        Self {
            translation: Vec3::zero(),
            rotation_x: 0.0,
            rotation_y: 0.0,
            scale: Vec3::one(),
        }
    }

    pub fn compose(&self, local: &Self) -> Self {
        let scaled = Vec3::new(
            local.translation.x * self.scale.x,
            local.translation.y * self.scale.y,
            local.translation.z * self.scale.z,
        );
        let rotated = rotate_y(rotate_x(scaled, self.rotation_x), self.rotation_y);
        Self {
            translation: self.translation + rotated,
            rotation_x: self.rotation_x + local.rotation_x,
            rotation_y: self.rotation_y + local.rotation_y,
            scale: Vec3::new(
                self.scale.x * local.scale.x,
                self.scale.y * local.scale.y,
                self.scale.z * local.scale.z,
            ),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct NodeOutput {
    primitives: Vec<BuilderPrimitive>,
    anchors: Vec<BuilderAnchor>,
    placements: Vec<BuilderPlacement>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BuilderPlacement {
    translation: Vec3<f32>,
    host_position_y_normalized: bool,
    host_scale_y_normalized: bool,
    host_scale_x_normalized: bool,
    host_scale_z_normalized: bool,
}

impl NodeOutput {
    fn append(&mut self, other: Self) {
        for primitive in other.primitives {
            self.merge_primitive(primitive);
        }
        self.anchors.extend(other.anchors);
        self.placements.extend(other.placements);
    }

    fn merge_primitive(&mut self, primitive: BuilderPrimitive) {
        if let Some(existing) = self
            .primitives
            .iter_mut()
            .find(|existing| primitive_same_geometry(existing, &primitive))
        {
            if primitive_has_material_slot(&primitive) {
                *existing = primitive;
            }
            return;
        }
        self.primitives.push(primitive);
    }

    fn host_positioned(&self, translation: Vec3<f32>) -> Self {
        let primitives = self
            .primitives
            .iter()
            .map(|primitive| match primitive {
                BuilderPrimitive::Box {
                    size,
                    transform,
                    material_slot,
                    ..
                } => BuilderPrimitive::Box {
                    size: *size,
                    transform: BuilderTransform {
                        translation: transform.translation + translation,
                        rotation_x: transform.rotation_x,
                        rotation_y: transform.rotation_y,
                        scale: transform.scale,
                    },
                    material_slot: material_slot.clone(),
                    host_position_normalized: true,
                    host_position_y_normalized: false,
                    host_scale_y_normalized: false,
                    host_scale_x_normalized: false,
                    host_scale_z_normalized: false,
                },
                BuilderPrimitive::Cylinder {
                    length,
                    radius,
                    transform,
                    material_slot,
                    ..
                } => BuilderPrimitive::Cylinder {
                    length: *length,
                    radius: *radius,
                    transform: BuilderTransform {
                        translation: transform.translation + translation,
                        rotation_x: transform.rotation_x,
                        rotation_y: transform.rotation_y,
                        scale: transform.scale,
                    },
                    material_slot: material_slot.clone(),
                    host_position_normalized: true,
                    host_position_y_normalized: false,
                    host_scale_y_normalized: false,
                    host_scale_x_normalized: false,
                    host_scale_z_normalized: false,
                },
            })
            .collect();

        let anchors = self
            .anchors
            .iter()
            .map(|anchor| BuilderAnchor {
                name: anchor.name.clone(),
                kind: anchor.kind,
                transform: BuilderTransform {
                    translation: anchor.transform.translation + translation,
                    rotation_x: anchor.transform.rotation_x,
                    rotation_y: anchor.transform.rotation_y,
                    scale: anchor.transform.scale,
                },
                host_position_normalized: true,
                host_position_y_normalized: false,
                surface_extent: anchor.surface_extent,
                surface_extent_normalized: anchor.surface_extent_normalized,
            })
            .collect();

        let placements = self
            .placements
            .iter()
            .map(|placement| BuilderPlacement {
                translation: placement.translation + translation,
                host_position_y_normalized: placement.host_position_y_normalized,
                host_scale_y_normalized: placement.host_scale_y_normalized,
                host_scale_x_normalized: placement.host_scale_x_normalized,
                host_scale_z_normalized: placement.host_scale_z_normalized,
            })
            .collect();

        Self {
            primitives,
            anchors,
            placements,
        }
    }
}

fn primitive_same_geometry(a: &BuilderPrimitive, b: &BuilderPrimitive) -> bool {
    match (a, b) {
        (
            BuilderPrimitive::Box {
                size: size_a,
                transform: transform_a,
                host_position_normalized: host_pos_a,
                host_position_y_normalized: host_pos_y_a,
                host_scale_y_normalized: host_scale_y_a,
                host_scale_x_normalized: host_scale_x_a,
                host_scale_z_normalized: host_scale_z_a,
                ..
            },
            BuilderPrimitive::Box {
                size: size_b,
                transform: transform_b,
                host_position_normalized: host_pos_b,
                host_position_y_normalized: host_pos_y_b,
                host_scale_y_normalized: host_scale_y_b,
                host_scale_x_normalized: host_scale_x_b,
                host_scale_z_normalized: host_scale_z_b,
                ..
            },
        ) => {
            size_a == size_b
                && transform_a.translation == transform_b.translation
                && transform_a.rotation_y == transform_b.rotation_y
                && transform_a.scale == transform_b.scale
                && host_pos_a == host_pos_b
                && host_pos_y_a == host_pos_y_b
                && host_scale_y_a == host_scale_y_b
                && host_scale_x_a == host_scale_x_b
                && host_scale_z_a == host_scale_z_b
        }
        (
            BuilderPrimitive::Cylinder {
                length: length_a,
                radius: radius_a,
                transform: transform_a,
                host_position_normalized: host_pos_a,
                host_position_y_normalized: host_pos_y_a,
                host_scale_y_normalized: host_scale_y_a,
                host_scale_x_normalized: host_scale_x_a,
                host_scale_z_normalized: host_scale_z_a,
                ..
            },
            BuilderPrimitive::Cylinder {
                length: length_b,
                radius: radius_b,
                transform: transform_b,
                host_position_normalized: host_pos_b,
                host_position_y_normalized: host_pos_y_b,
                host_scale_y_normalized: host_scale_y_b,
                host_scale_x_normalized: host_scale_x_b,
                host_scale_z_normalized: host_scale_z_b,
                ..
            },
        ) => {
            length_a == length_b
                && radius_a == radius_b
                && transform_a.translation == transform_b.translation
                && transform_a.rotation_y == transform_b.rotation_y
                && transform_a.scale == transform_b.scale
                && host_pos_a == host_pos_b
                && host_pos_y_a == host_pos_y_b
                && host_scale_y_a == host_scale_y_b
                && host_scale_x_a == host_scale_x_b
                && host_scale_z_a == host_scale_z_b
        }
        _ => false,
    }
}

fn primitive_has_material_slot(primitive: &BuilderPrimitive) -> bool {
    match primitive {
        BuilderPrimitive::Box { material_slot, .. } => material_slot.is_some(),
        BuilderPrimitive::Cylinder { material_slot, .. } => material_slot.is_some(),
    }
}

impl Default for BuilderGraph {
    fn default() -> Self {
        Self::preset_table()
    }
}

impl BuilderGraph {
    pub fn empty_script_named(name: String) -> String {
        format!(
            "name = \"{}\";\nhost = sector;\n\npreview {{\n    width = 1.0;\n    depth = 1.0;\n    height = 1.0;\n}}\n\noutput = [];\n",
            if name.trim().is_empty() {
                "Empty"
            } else {
                &name
            }
        )
    }

    pub fn preset_table_script_named(name: String) -> String {
        let mut script = include_str!("../examples/table.buildergraph").to_string();
        let graph_name = if name.trim().is_empty() {
            "Table".to_string()
        } else {
            name
        };
        if let Some(line_end) = script.find('\n') {
            script.replace_range(0..line_end, &format!("name = \"{graph_name}\";"));
        }
        script
    }

    pub fn preset_wall_torch_script_named(name: String) -> String {
        let mut script = include_str!("../examples/wall_torch.buildergraph").to_string();
        let graph_name = if name.trim().is_empty() {
            "Wall Torch".to_string()
        } else {
            name
        };
        if let Some(line_end) = script.find('\n') {
            script.replace_range(0..line_end, &format!("name = \"{graph_name}\";"));
        }
        script
    }

    pub fn preset_wall_lantern_script_named(name: String) -> String {
        let mut script = include_str!("../examples/wall_lantern.buildergraph").to_string();
        let graph_name = if name.trim().is_empty() {
            "Wall Lantern".to_string()
        } else {
            name
        };
        if let Some(line_end) = script.find('\n') {
            script.replace_range(0..line_end, &format!("name = \"{graph_name}\";"));
        }
        script
    }

    pub fn preset_campfire_script_named(name: String) -> String {
        let mut script = include_str!("../examples/campfire.buildergraph").to_string();
        let graph_name = if name.trim().is_empty() {
            "Campfire".to_string()
        } else {
            name
        };
        if let Some(line_end) = script.find('\n') {
            script.replace_range(0..line_end, &format!("name = \"{graph_name}\";"));
        }
        script
    }

    pub fn preset_surface_masonry_script_named(name: String) -> String {
        let mut script = include_str!("../examples/surface_masonry.buildergraph").to_string();
        let graph_name = if name.trim().is_empty() {
            "Surface Masonry".to_string()
        } else {
            name
        };
        if let Some(line_end) = script.find('\n') {
            script.replace_range(0..line_end, &format!("name = \"{graph_name}\";"));
        }
        script
    }

    pub fn preset_wall_masonry_script_named(name: String) -> String {
        let mut script = include_str!("../examples/wall_masonry.buildergraph").to_string();
        let graph_name = if name.trim().is_empty() {
            "Wall Masonry".to_string()
        } else {
            name
        };
        if let Some(line_end) = script.find('\n') {
            script.replace_range(0..line_end, &format!("name = \"{graph_name}\";"));
        }
        script
    }

    pub fn preset_wall_columns_masonry_script_named(name: String) -> String {
        let mut script = include_str!("../examples/wall_columns_masonry.buildergraph").to_string();
        let graph_name = if name.trim().is_empty() {
            "Wall Columns Masonry".to_string()
        } else {
            name
        };
        if let Some(line_end) = script.find('\n') {
            script.replace_range(0..line_end, &format!("name = \"{graph_name}\";"));
        }
        script
    }

    pub fn empty_named(name: String) -> Self {
        let graph_name = if name.trim().is_empty() {
            "Empty".to_string()
        } else {
            name
        };
        Self {
            id: Uuid::new_v4(),
            name: graph_name,
            connections: Vec::new(),
            nodes: vec![BuilderNode {
                id: 1,
                name: "Output".to_string(),
                kind: BuilderNodeKind::GeometryOutput {
                    target: BuilderOutputTarget::Sector,
                    host_refs: 1,
                },
                pos: Vec2::new(320, 96),
                preview_collapsed: false,
            }],
            selected_node: Some(0),
            scroll_offset: Vec2::zero(),
            preview_host: None,
        }
    }

    pub fn preset_table() -> Self {
        let nodes = vec![
            BuilderNode {
                id: 1,
                name: "Sector Surface".to_string(),
                kind: BuilderNodeKind::SectorSurface,
                pos: Vec2::new(32, 32),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 2,
                name: "Top Offset".to_string(),
                kind: BuilderNodeKind::Offset {
                    translate: Vec3::new(0.0, 0.80, 0.0),
                },
                pos: Vec2::new(256, 32),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 3,
                name: "Tabletop".to_string(),
                kind: BuilderNodeKind::Box {
                    width: 0.0,
                    depth: 0.0,
                    height: 0.05,
                },
                pos: Vec2::new(480, 32),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 4,
                name: "Top Surface".to_string(),
                kind: BuilderNodeKind::ItemSurface {
                    name: "TOP".to_string(),
                },
                pos: Vec2::new(704, -16),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 5,
                name: "Top Material".to_string(),
                kind: BuilderNodeKind::MaterialAnchor {
                    name: "TOP".to_string(),
                },
                pos: Vec2::new(704, 96),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 6,
                name: "Leg".to_string(),
                kind: BuilderNodeKind::Box {
                    width: 0.12,
                    depth: 0.12,
                    height: 0.80,
                },
                pos: Vec2::new(480, 224),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 7,
                name: "Corner Layout".to_string(),
                kind: BuilderNodeKind::CornerLayout {
                    inset_x: 0.10,
                    inset_z: 0.10,
                },
                pos: Vec2::new(256, 224),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 8,
                name: "Leg Material".to_string(),
                kind: BuilderNodeKind::MaterialAnchor {
                    name: "LEGS".to_string(),
                },
                pos: Vec2::new(704, 224),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 9,
                name: "Join Geometry".to_string(),
                kind: BuilderNodeKind::Join,
                pos: Vec2::new(944, 128),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 10,
                name: "Join Attachments".to_string(),
                kind: BuilderNodeKind::Join,
                pos: Vec2::new(944, 16),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 11,
                name: "Assembly".to_string(),
                kind: BuilderNodeKind::Join,
                pos: Vec2::new(1184, 96),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 12,
                name: "Output".to_string(),
                kind: BuilderNodeKind::GeometryOutput {
                    target: BuilderOutputTarget::Sector,
                    host_refs: 1,
                },
                pos: Vec2::new(1408, 96),
                preview_collapsed: false,
            },
        ];

        let connections = vec![
            (1, 0, 2, 0),
            (2, 0, 3, 0),
            (3, 0, 4, 0),
            (3, 0, 5, 0),
            (1, 0, 7, 0),
            (7, 0, 6, 0),
            (6, 0, 8, 0),
            (3, 0, 9, 0),
            (6, 0, 9, 1),
            (3, 0, 10, 0),
            (4, 0, 10, 1),
            (5, 0, 10, 2),
            (8, 0, 10, 3),
            (9, 0, 11, 0),
            (10, 0, 11, 1),
            (11, 0, 12, 0),
        ];

        Self {
            id: Uuid::new_v4(),
            name: default_graph_name(),
            nodes,
            connections,
            selected_node: Some(11),
            scroll_offset: Vec2::zero(),
            preview_host: None,
        }
    }

    pub fn preset_wall_torch() -> Self {
        let nodes = vec![
            BuilderNode {
                id: 1,
                name: "Linedef Surface".to_string(),
                kind: BuilderNodeKind::LinedefSurface,
                pos: Vec2::new(32, 160),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 2,
                name: "Back Plate Offset".to_string(),
                kind: BuilderNodeKind::Offset {
                    translate: Vec3::new(0.0, 0.78, 0.06),
                },
                pos: Vec2::new(256, 32),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 3,
                name: "Back Plate".to_string(),
                kind: BuilderNodeKind::Box {
                    width: 0.28,
                    depth: 0.06,
                    height: 0.22,
                },
                pos: Vec2::new(512, 32),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 4,
                name: "Arm Offset".to_string(),
                kind: BuilderNodeKind::Offset {
                    translate: Vec3::new(0.0, 0.82, 0.18),
                },
                pos: Vec2::new(256, 176),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 5,
                name: "Arm".to_string(),
                kind: BuilderNodeKind::Box {
                    width: 0.10,
                    depth: 0.26,
                    height: 0.04,
                },
                pos: Vec2::new(512, 176),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 6,
                name: "Torch Offset".to_string(),
                kind: BuilderNodeKind::Offset {
                    translate: Vec3::new(0.0, 0.86, 0.28),
                },
                pos: Vec2::new(256, 320),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 7,
                name: "Torch Body".to_string(),
                kind: BuilderNodeKind::Cylinder {
                    length: 0.0,
                    radius: 0.02,
                },
                pos: Vec2::new(512, 320),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 8,
                name: "Flame Offset".to_string(),
                kind: BuilderNodeKind::Offset {
                    translate: Vec3::new(0.0, 0.92, 0.52),
                },
                pos: Vec2::new(256, 464),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 9,
                name: "Flame Volume".to_string(),
                kind: BuilderNodeKind::Box {
                    width: 0.12,
                    depth: 0.12,
                    height: 0.18,
                },
                pos: Vec2::new(512, 464),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 10,
                name: "Base Join".to_string(),
                kind: BuilderNodeKind::Join,
                pos: Vec2::new(768, 144),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 11,
                name: "Base Material".to_string(),
                kind: BuilderNodeKind::MaterialAnchor {
                    name: "BASE".to_string(),
                },
                pos: Vec2::new(1008, 112),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 12,
                name: "Torch Material".to_string(),
                kind: BuilderNodeKind::MaterialAnchor {
                    name: "TORCH".to_string(),
                },
                pos: Vec2::new(768, 320),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 13,
                name: "Flame Material".to_string(),
                kind: BuilderNodeKind::MaterialAnchor {
                    name: "FLAME".to_string(),
                },
                pos: Vec2::new(768, 464),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 14,
                name: "Torch Geometry".to_string(),
                kind: BuilderNodeKind::Join,
                pos: Vec2::new(1248, 224),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 15,
                name: "Torch Materials".to_string(),
                kind: BuilderNodeKind::Join,
                pos: Vec2::new(1248, 416),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 16,
                name: "Torch Assembly".to_string(),
                kind: BuilderNodeKind::Join,
                pos: Vec2::new(1488, 288),
                preview_collapsed: false,
            },
            BuilderNode {
                id: 17,
                name: "Output".to_string(),
                kind: BuilderNodeKind::GeometryOutput {
                    target: BuilderOutputTarget::Linedef,
                    host_refs: 1,
                },
                pos: Vec2::new(1728, 288),
                preview_collapsed: false,
            },
        ];

        let connections = vec![
            (1, 0, 2, 0),
            (2, 0, 3, 0),
            (1, 0, 4, 0),
            (4, 0, 5, 0),
            (1, 0, 6, 0),
            (6, 0, 7, 0),
            (1, 0, 8, 0),
            (8, 0, 9, 0),
            (3, 0, 10, 0),
            (5, 0, 10, 1),
            (10, 0, 11, 0),
            (7, 0, 12, 0),
            (9, 0, 13, 0),
            (3, 0, 14, 0),
            (5, 0, 14, 1),
            (7, 0, 14, 2),
            (9, 0, 14, 3),
            (11, 0, 15, 0),
            (12, 0, 15, 1),
            (13, 0, 15, 2),
            (14, 0, 16, 0),
            (15, 0, 16, 1),
            (16, 0, 17, 0),
        ];

        Self {
            id: Uuid::new_v4(),
            name: "Wall Torch".to_string(),
            nodes,
            connections,
            selected_node: Some(16),
            scroll_offset: Vec2::zero(),
            preview_host: None,
        }
    }

    pub fn from_text(source: &str) -> Result<Self, String> {
        toml::from_str(source)
            .or_else(|_| serde_json::from_str(source))
            .map(|graph: Self| graph.upgrade_legacy_presets())
            .map_err(|err| err.to_string())
    }

    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|err| err.to_string())
    }

    fn upgrade_legacy_presets(self) -> Self {
        if self.name == "Wall Torch" {
            let has_flame = self
                .material_slot_names()
                .iter()
                .any(|name| name == "FLAME");
            let has_torch = self
                .material_slot_names()
                .iter()
                .any(|name| name == "TORCH");
            let output_is_linedef = self.output_spec().target == BuilderOutputTarget::Linedef;
            let mut replace_with_current_preset = false;
            if output_is_linedef && has_flame && !has_torch {
                replace_with_current_preset = true;
            }
            if output_is_linedef && has_flame && has_torch {
                for node in &self.nodes {
                    match &node.kind {
                        BuilderNodeKind::Offset { translate } if node.name == "Flame Offset" => {
                            if translate.y < 0.8 {
                                replace_with_current_preset = true;
                                break;
                            }
                        }
                        BuilderNodeKind::Cylinder { radius, .. } if node.name == "Torch Body" => {
                            if *radius > 0.03 {
                                replace_with_current_preset = true;
                                break;
                            }
                        }
                        BuilderNodeKind::Box { .. } if node.name == "Torch Body" => {
                            replace_with_current_preset = true;
                            break;
                        }
                        _ => {}
                    }
                }
            }
            if replace_with_current_preset {
                let id = self.id;
                let mut upgraded = Self::preset_wall_torch();
                upgraded.id = id;
                return upgraded;
            }
        }
        self
    }

    pub fn evaluate(&self) -> Result<BuilderAssembly, String> {
        let Some(output_node) = self
            .nodes
            .iter()
            .find(|node| matches!(node.kind, BuilderNodeKind::GeometryOutput { .. }))
        else {
            return Err("Builder graph has no GeometryOutput node.".to_string());
        };

        let mut cache: HashMap<u16, NodeOutput> = HashMap::default();
        let output = self.evaluate_node(output_node.id, &mut cache)?;
        Ok(BuilderAssembly {
            primitives: output.primitives,
            anchors: output.anchors,
            cuts: Vec::new(),
            surface_details: Vec::new(),
            static_billboards: Vec::new(),
            warnings: Vec::new(),
        })
    }

    pub fn render_preview(&self, size: u32) -> BuilderPreview {
        let preview_host = self
            .preview_host
            .clone()
            .unwrap_or_else(|| default_preview_host(self.output_spec().target));
        match self.evaluate() {
            Ok(assembly) => {
                render_assembly_preview(size, self.output_spec(), &preview_host, &assembly)
            }
            Err(_) => empty_preview(size.max(32)),
        }
    }

    pub fn output_spec(&self) -> BuilderOutputSpec {
        self.nodes
            .iter()
            .find_map(|node| match node.kind {
                BuilderNodeKind::GeometryOutput { target, host_refs } => Some(BuilderOutputSpec {
                    target,
                    host_refs: sanitize_host_refs(host_refs),
                }),
                _ => None,
            })
            .unwrap_or(BuilderOutputSpec {
                target: BuilderOutputTarget::Sector,
                host_refs: 1,
            })
    }

    pub fn material_slot_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for node in &self.nodes {
            match &node.kind {
                BuilderNodeKind::MaterialAnchor { name }
                | BuilderNodeKind::MaterialSlot { name, .. } => {
                    if !name.trim().is_empty() && !out.iter().any(|existing| existing == name) {
                        out.push(name.clone());
                    }
                }
                _ => {}
            }
        }
        out
    }

    pub fn item_slot_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for node in &self.nodes {
            match &node.kind {
                BuilderNodeKind::ItemAnchor { name }
                | BuilderNodeKind::ItemSurface { name }
                | BuilderNodeKind::ItemSlot { name, .. } => {
                    if !name.trim().is_empty() && !out.iter().any(|existing| existing == name) {
                        out.push(name.clone());
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn evaluate_node(
        &self,
        node_id: u16,
        cache: &mut HashMap<u16, NodeOutput>,
    ) -> Result<NodeOutput, String> {
        if let Some(output) = cache.get(&node_id) {
            return Ok(output.clone());
        }
        let Some(node) = self.nodes.iter().find(|node| node.id == node_id) else {
            return Err(format!("Unknown builder node id {node_id}."));
        };

        let output = match &node.kind {
            BuilderNodeKind::SectorSurface => NodeOutput {
                primitives: Vec::new(),
                anchors: Vec::new(),
                placements: vec![BuilderPlacement {
                    translation: Vec3::zero(),
                    host_position_y_normalized: false,
                    host_scale_y_normalized: false,
                    host_scale_x_normalized: true,
                    host_scale_z_normalized: true,
                }],
            },
            BuilderNodeKind::LinedefSurface => NodeOutput {
                primitives: Vec::new(),
                anchors: Vec::new(),
                placements: vec![BuilderPlacement {
                    translation: Vec3::zero(),
                    host_position_y_normalized: true,
                    host_scale_y_normalized: false,
                    host_scale_x_normalized: true,
                    host_scale_z_normalized: false,
                }],
            },
            BuilderNodeKind::VertexPoint => NodeOutput {
                primitives: Vec::new(),
                anchors: Vec::new(),
                placements: vec![BuilderPlacement {
                    translation: Vec3::zero(),
                    host_position_y_normalized: false,
                    host_scale_y_normalized: false,
                    host_scale_x_normalized: false,
                    host_scale_z_normalized: false,
                }],
            },
            BuilderNodeKind::Offset { translate } => self
                .input_output(node.id, 0, cache)?
                .host_positioned(*translate),
            BuilderNodeKind::CornerLayout { inset_x, inset_z } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                let bases = if input.placements.is_empty() {
                    vec![BuilderPlacement {
                        translation: Vec3::zero(),
                        host_position_y_normalized: false,
                        host_scale_y_normalized: false,
                        host_scale_x_normalized: true,
                        host_scale_z_normalized: true,
                    }]
                } else {
                    input.placements
                };
                let x = 0.5 - inset_x.clamp(0.0, 0.49);
                let z = 0.5 - inset_z.clamp(0.0, 0.49);
                for base in bases {
                    for delta in [
                        Vec3::new(x, 0.0, z),
                        Vec3::new(-x, 0.0, z),
                        Vec3::new(x, 0.0, -z),
                        Vec3::new(-x, 0.0, -z),
                    ] {
                        out.placements.push(BuilderPlacement {
                            translation: base.translation + delta,
                            host_position_y_normalized: base.host_position_y_normalized,
                            host_scale_y_normalized: base.host_scale_y_normalized,
                            host_scale_x_normalized: false,
                            host_scale_z_normalized: false,
                        });
                    }
                }
                out
            }
            BuilderNodeKind::Box {
                width,
                depth,
                height,
            } => NodeOutput {
                primitives: {
                    let placements = self.input_output(node.id, 0, cache)?.placements;
                    if placements.is_empty() {
                        vec![BuilderPrimitive::Box {
                            size: Vec3::new(*width, *height, *depth),
                            transform: BuilderTransform::identity(),
                            material_slot: None,
                            host_position_normalized: false,
                            host_position_y_normalized: false,
                            host_scale_y_normalized: false,
                            host_scale_x_normalized: false,
                            host_scale_z_normalized: false,
                        }]
                    } else {
                        placements
                            .into_iter()
                            .map(|placement| BuilderPrimitive::Box {
                                size: Vec3::new(
                                    if *width <= 0.0 { 1.0 } else { *width },
                                    *height,
                                    if *depth <= 0.0 { 1.0 } else { *depth },
                                ),
                                transform: BuilderTransform {
                                    translation: placement.translation,
                                    rotation_x: 0.0,
                                    rotation_y: 0.0,
                                    scale: Vec3::one(),
                                },
                                material_slot: None,
                                host_position_normalized: true,
                                host_position_y_normalized: placement.host_position_y_normalized,
                                host_scale_y_normalized: placement.host_scale_y_normalized,
                                host_scale_x_normalized: *width <= 0.0
                                    && placement.host_scale_x_normalized,
                                host_scale_z_normalized: *depth <= 0.0
                                    && placement.host_scale_z_normalized,
                            })
                            .collect()
                    }
                },
                anchors: Vec::new(),
                placements: Vec::new(),
            },
            BuilderNodeKind::Cylinder { length, radius } => NodeOutput {
                primitives: {
                    let placements = self.input_output(node.id, 0, cache)?.placements;
                    if placements.is_empty() {
                        vec![BuilderPrimitive::Cylinder {
                            length: if *length <= 0.0 { 1.0 } else { *length },
                            radius: *radius,
                            transform: BuilderTransform::identity(),
                            material_slot: None,
                            host_position_normalized: false,
                            host_position_y_normalized: false,
                            host_scale_y_normalized: false,
                            host_scale_x_normalized: false,
                            host_scale_z_normalized: false,
                        }]
                    } else {
                        placements
                            .into_iter()
                            .map(|placement| BuilderPrimitive::Cylinder {
                                length: if *length <= 0.0 { 1.0 } else { *length },
                                radius: *radius,
                                transform: BuilderTransform {
                                    translation: placement.translation,
                                    rotation_x: 0.0,
                                    rotation_y: 0.0,
                                    scale: Vec3::one(),
                                },
                                material_slot: None,
                                host_position_normalized: true,
                                host_position_y_normalized: placement.host_position_y_normalized,
                                host_scale_y_normalized: *length <= 0.0
                                    && placement.host_position_y_normalized,
                                host_scale_x_normalized: false,
                                host_scale_z_normalized: false,
                            })
                            .collect()
                    }
                },
                anchors: Vec::new(),
                placements: Vec::new(),
            },
            BuilderNodeKind::SectorCorners {
                inset_x,
                inset_z,
                elevation,
            } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                let x = 0.5 - inset_x.clamp(0.0, 0.49);
                let z = 0.5 - inset_z.clamp(0.0, 0.49);
                for translation in [
                    Vec3::new(x, *elevation, z),
                    Vec3::new(-x, *elevation, z),
                    Vec3::new(x, *elevation, -z),
                    Vec3::new(-x, *elevation, -z),
                ] {
                    out.append(input.host_positioned(translation));
                }
                out
            }
            BuilderNodeKind::SectorGrid {
                columns,
                rows,
                inset_x,
                inset_z,
                elevation,
            } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                let cols = (*columns).max(1) as usize;
                let rows = (*rows).max(1) as usize;
                let min_x = -0.5 + inset_x.clamp(0.0, 0.49);
                let max_x = 0.5 - inset_x.clamp(0.0, 0.49);
                let min_z = -0.5 + inset_z.clamp(0.0, 0.49);
                let max_z = 0.5 - inset_z.clamp(0.0, 0.49);

                for row in 0..rows {
                    let tz = if rows == 1 {
                        0.0
                    } else {
                        min_z + (max_z - min_z) * (row as f32 / (rows as f32 - 1.0))
                    };
                    for col in 0..cols {
                        let tx = if cols == 1 {
                            0.0
                        } else {
                            min_x + (max_x - min_x) * (col as f32 / (cols as f32 - 1.0))
                        };
                        out.append(input.host_positioned(Vec3::new(tx, *elevation, tz)));
                    }
                }
                out
            }
            BuilderNodeKind::SectorEdges {
                north,
                south,
                east,
                west,
                inset,
                elevation,
            } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                let edge = 0.5 - inset.clamp(0.0, 0.49);
                let mut add_edge = |enabled: bool, translation: Vec3<f32>| {
                    if enabled {
                        out.append(input.host_positioned(translation));
                    }
                };
                add_edge(*north, Vec3::new(0.0, *elevation, -edge));
                add_edge(*south, Vec3::new(0.0, *elevation, edge));
                add_edge(*east, Vec3::new(edge, *elevation, 0.0));
                add_edge(*west, Vec3::new(-edge, *elevation, 0.0));
                out
            }
            BuilderNodeKind::LinedefRow {
                count,
                inset,
                elevation,
            } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                let count = (*count).max(1) as usize;
                let min_x = -0.5 + inset.clamp(0.0, 0.49);
                let max_x = 0.5 - inset.clamp(0.0, 0.49);
                for i in 0..count {
                    let tx = if count == 1 {
                        0.0
                    } else {
                        min_x + (max_x - min_x) * (i as f32 / (count as f32 - 1.0))
                    };
                    out.append(input.host_positioned(Vec3::new(tx, *elevation, 0.0)));
                }
                out
            }
            BuilderNodeKind::LinedefSpan { inset, elevation } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                let span_scale = (1.0 - inset.clamp(0.0, 0.49) * 2.0).max(0.0);
                for primitive in input.primitives {
                    match primitive {
                        BuilderPrimitive::Box {
                            size,
                            transform,
                            material_slot,
                            ..
                        } => out.primitives.push(BuilderPrimitive::Box {
                            size,
                            transform: BuilderTransform {
                                translation: transform.translation
                                    + Vec3::new(0.0, *elevation, 0.0),
                                rotation_x: transform.rotation_x,
                                rotation_y: transform.rotation_y,
                                scale: Vec3::new(
                                    transform.scale.x * span_scale,
                                    transform.scale.y,
                                    transform.scale.z,
                                ),
                            },
                            material_slot,
                            host_position_normalized: true,
                            host_position_y_normalized: false,
                            host_scale_y_normalized: false,
                            host_scale_x_normalized: true,
                            host_scale_z_normalized: false,
                        }),
                        BuilderPrimitive::Cylinder {
                            length,
                            radius,
                            transform,
                            material_slot,
                            ..
                        } => out.primitives.push(BuilderPrimitive::Cylinder {
                            length,
                            radius,
                            transform: BuilderTransform {
                                translation: transform.translation
                                    + Vec3::new(0.0, *elevation, 0.0),
                                rotation_x: transform.rotation_x,
                                rotation_y: transform.rotation_y,
                                scale: Vec3::new(
                                    transform.scale.x * span_scale,
                                    transform.scale.y,
                                    transform.scale.z,
                                ),
                            },
                            material_slot,
                            host_position_normalized: true,
                            host_position_y_normalized: false,
                            host_scale_y_normalized: false,
                            host_scale_x_normalized: true,
                            host_scale_z_normalized: false,
                        }),
                    }
                }
                for anchor in input.anchors {
                    out.anchors.push(BuilderAnchor {
                        name: anchor.name,
                        kind: anchor.kind,
                        transform: BuilderTransform {
                            translation: anchor.transform.translation
                                + Vec3::new(0.0, *elevation, 0.0),
                            rotation_x: anchor.transform.rotation_x,
                            rotation_y: anchor.transform.rotation_y,
                            scale: anchor.transform.scale,
                        },
                        host_position_normalized: anchor.host_position_normalized,
                        host_position_y_normalized: anchor.host_position_y_normalized,
                        surface_extent: anchor.surface_extent,
                        surface_extent_normalized: anchor.surface_extent_normalized,
                    });
                }
                out
            }
            BuilderNodeKind::ItemAnchor { name } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                for primitive in input.primitives {
                    match primitive {
                        BuilderPrimitive::Box {
                            size,
                            transform,
                            host_position_normalized,
                            host_position_y_normalized,
                            host_scale_y_normalized,
                            ..
                        } => {
                            let h = size.y * transform.scale.y;
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Item,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_x: transform.rotation_x,
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                host_position_y_normalized: host_position_y_normalized
                                    || host_scale_y_normalized,
                                surface_extent: Vec2::zero(),
                                surface_extent_normalized: false,
                            });
                        }
                        BuilderPrimitive::Cylinder {
                            length,
                            radius,
                            transform,
                            host_position_normalized,
                            host_position_y_normalized,
                            host_scale_y_normalized,
                            ..
                        } => {
                            let h = radius * 2.0 * transform.scale.y;
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Item,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_x: transform.rotation_x,
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                host_position_y_normalized: host_position_y_normalized
                                    || host_scale_y_normalized,
                                surface_extent: Vec2::new(length * transform.scale.x, radius * 2.0),
                                surface_extent_normalized: false,
                            });
                        }
                    }
                }
                out
            }
            BuilderNodeKind::ItemSurface { name } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                for primitive in input.primitives {
                    match primitive {
                        BuilderPrimitive::Box {
                            size,
                            transform,
                            host_position_normalized,
                            host_position_y_normalized,
                            host_scale_y_normalized,
                            host_scale_x_normalized,
                            host_scale_z_normalized,
                            ..
                        } => {
                            let h = size.y * transform.scale.y;
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Item,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_x: transform.rotation_x,
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                host_position_y_normalized: host_position_y_normalized
                                    || host_scale_y_normalized,
                                surface_extent: Vec2::new(
                                    size.x * transform.scale.x,
                                    size.z * transform.scale.z,
                                ),
                                surface_extent_normalized: host_scale_x_normalized
                                    || host_scale_z_normalized,
                            });
                        }
                        BuilderPrimitive::Cylinder {
                            length,
                            radius,
                            transform,
                            host_position_normalized,
                            host_position_y_normalized,
                            host_scale_y_normalized,
                            host_scale_x_normalized,
                            ..
                        } => {
                            let h = radius * 2.0 * transform.scale.y;
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Item,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_x: transform.rotation_x,
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                host_position_y_normalized: host_position_y_normalized
                                    || host_scale_y_normalized,
                                surface_extent: Vec2::new(
                                    length * transform.scale.x,
                                    radius * 2.0 * transform.scale.z,
                                ),
                                surface_extent_normalized: host_scale_x_normalized,
                            });
                        }
                    }
                }
                out
            }
            BuilderNodeKind::MaterialAnchor { name } => {
                let input = self.input_output(node.id, 0, cache)?;
                let mut out = NodeOutput::default();
                for primitive in input.primitives {
                    match primitive {
                        BuilderPrimitive::Box {
                            size,
                            transform,
                            host_position_normalized,
                            host_position_y_normalized,
                            host_scale_y_normalized,
                            host_scale_x_normalized,
                            host_scale_z_normalized,
                            ..
                        } => {
                            let h = size.y * transform.scale.y;
                            out.primitives.push(BuilderPrimitive::Box {
                                size,
                                transform,
                                material_slot: Some(name.clone()),
                                host_position_normalized,
                                host_position_y_normalized,
                                host_scale_y_normalized,
                                host_scale_x_normalized,
                                host_scale_z_normalized,
                            });
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Material,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_x: transform.rotation_x,
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                host_position_y_normalized: host_position_y_normalized
                                    || host_scale_y_normalized,
                                surface_extent: Vec2::zero(),
                                surface_extent_normalized: false,
                            });
                        }
                        BuilderPrimitive::Cylinder {
                            length,
                            radius,
                            transform,
                            host_position_normalized,
                            host_position_y_normalized,
                            host_scale_y_normalized,
                            host_scale_x_normalized,
                            host_scale_z_normalized,
                            ..
                        } => {
                            let h = radius * 2.0 * transform.scale.y;
                            out.primitives.push(BuilderPrimitive::Cylinder {
                                length,
                                radius,
                                transform,
                                material_slot: Some(name.clone()),
                                host_position_normalized,
                                host_position_y_normalized,
                                host_scale_y_normalized,
                                host_scale_x_normalized,
                                host_scale_z_normalized,
                            });
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Material,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_x: transform.rotation_x,
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                host_position_y_normalized: host_position_y_normalized
                                    || host_scale_y_normalized,
                                surface_extent: Vec2::zero(),
                                surface_extent_normalized: false,
                            });
                        }
                    }
                }
                out
            }
            BuilderNodeKind::ItemSlot { name, position } => NodeOutput {
                primitives: Vec::new(),
                anchors: vec![BuilderAnchor {
                    name: name.clone(),
                    kind: BuilderAttachmentKind::Item,
                    transform: BuilderTransform {
                        translation: *position,
                        rotation_x: 0.0,
                        rotation_y: 0.0,
                        scale: Vec3::one(),
                    },
                    host_position_normalized: false,
                    host_position_y_normalized: false,
                    surface_extent: Vec2::zero(),
                    surface_extent_normalized: false,
                }],
                placements: Vec::new(),
            },
            BuilderNodeKind::MaterialSlot { name, position } => NodeOutput {
                primitives: Vec::new(),
                anchors: vec![BuilderAnchor {
                    name: name.clone(),
                    kind: BuilderAttachmentKind::Material,
                    transform: BuilderTransform {
                        translation: *position,
                        rotation_x: 0.0,
                        rotation_y: 0.0,
                        scale: Vec3::one(),
                    },
                    host_position_normalized: false,
                    host_position_y_normalized: false,
                    surface_extent: Vec2::zero(),
                    surface_extent_normalized: false,
                }],
                placements: Vec::new(),
            },
            BuilderNodeKind::Join => {
                let mut out = NodeOutput::default();
                for input_index in 0..8 {
                    out.append(self.input_output(node.id, input_index, cache)?);
                }
                out
            }
            BuilderNodeKind::GeometryOutput {
                target: _,
                host_refs,
            } => {
                let mut out = NodeOutput::default();
                let _ = sanitize_host_refs(*host_refs);
                for input_index in 0..8 {
                    if let Some(source_id) = self.input_source(node.id, input_index) {
                        out.append(self.evaluate_node(source_id, cache)?);
                    }
                }
                out
            }
        };

        cache.insert(node.id, output.clone());
        Ok(output)
    }

    fn input_output(
        &self,
        node_id: u16,
        input_index: u8,
        cache: &mut HashMap<u16, NodeOutput>,
    ) -> Result<NodeOutput, String> {
        let Some(source_id) = self.input_source(node_id, input_index) else {
            return Ok(NodeOutput::default());
        };
        self.evaluate_node(source_id, cache)
    }

    fn input_source(&self, node_id: u16, input_index: u8) -> Option<u16> {
        self.connections
            .iter()
            .find(|(_, _, to, to_input)| *to == node_id && *to_input == input_index)
            .map(|(from, _, _, _)| *from)
    }
}

impl BuilderScript {
    pub fn from_text(source: &str) -> Result<Self, String> {
        BuilderScriptParser::new(source)?.parse_script()
    }

    pub fn output_spec(&self) -> BuilderOutputSpec {
        let target = match self.host {
            BuilderScriptHost::Line => BuilderOutputTarget::Linedef,
            BuilderScriptHost::Sector => BuilderOutputTarget::Sector,
            BuilderScriptHost::Vertex => BuilderOutputTarget::VertexPair,
        };
        BuilderOutputSpec {
            target,
            host_refs: 1,
        }
    }

    pub fn evaluate(&self) -> Result<BuilderAssembly, String> {
        let dims = HostPreviewDims::from_preview(self.host, &self.preview_host);
        self.evaluate_with_dims(dims)
    }

    pub fn evaluate_with_host(&self, host: &BuilderHost) -> Result<BuilderAssembly, String> {
        let dims = HostPreviewDims::from_host(host);
        let mut assembly = self.evaluate_with_dims(dims)?;
        if !host_matches_script_host(host, self.host) {
            assembly.warnings.push(BuilderWarning {
                code: "host_mismatch".to_string(),
                message: format!(
                    "script declares host {:?}, evaluated with {} host",
                    self.host,
                    host.kind_name()
                ),
            });
        }
        Ok(assembly)
    }

    fn evaluate_with_dims(&self, dims: HostPreviewDims) -> Result<BuilderAssembly, String> {
        let mut primitives = Vec::new();
        let mut anchors = Vec::new();
        let mut cuts = Vec::new();
        let mut surface_details = Vec::new();
        let mut warnings = Vec::new();
        let mut resolved_parts: HashMap<String, ResolvedPart> = HashMap::default();
        let mut params: HashMap<String, f32> = HashMap::default();
        let mut ident_params: HashMap<String, String> = HashMap::default();
        let output_parts: HashSet<&str> = self.output.iter().map(String::as_str).collect();
        let emit_all_parts = output_parts.is_empty();

        for param in &self.params {
            match &param.value {
                BuilderScriptParamValue::Scalar(expr) => {
                    let value = eval_scalar_expr(expr, self.host, &dims, &params)?;
                    params.insert(param.name.clone(), value);
                }
                BuilderScriptParamValue::Ident(value) => {
                    ident_params.insert(param.name.clone(), value.clone());
                }
            }
        }

        for part in &self.parts {
            let emit_part = emit_all_parts || output_parts.contains(part.name.as_str());
            let parent_transform = part
                .parent
                .as_ref()
                .map(|reference| eval_ref_transform(reference, self.host, &dims, &resolved_parts))
                .transpose()?
                .unwrap_or_else(BuilderTransform::identity);
            let local_rotation_x = part
                .rotate_x
                .as_ref()
                .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                .transpose()?
                .unwrap_or(0.0);
            let attach = if part.parent.is_some() {
                let local_attach =
                    eval_point_expr(&part.attach, self.host, &dims, &resolved_parts, &params)?;
                transform_point(&parent_transform, local_attach)
            } else {
                eval_point_expr(&part.attach, self.host, &dims, &resolved_parts, &params)?
            };
            let rotation_x = parent_transform.rotation_x + local_rotation_x;
            let local_rotation_y = part
                .rotate_y
                .as_ref()
                .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                .transpose()?
                .unwrap_or(0.0);
            let rotation_y = parent_transform.rotation_y + local_rotation_y;
            let primitive = match &part.kind {
                BuilderScriptPartKind::Box { size } => {
                    let sx = eval_scalar_expr(&size[0], self.host, &dims, &params)?;
                    let sy = eval_scalar_expr(&size[1], self.host, &dims, &params)?;
                    let sz = eval_scalar_expr(&size[2], self.host, &dims, &params)?;
                    resolved_parts.insert(
                        part.name.clone(),
                        resolved_box_anchors(attach, Vec3::new(sx, sy, sz), rotation_x, rotation_y),
                    );
                    BuilderPrimitive::Box {
                        size: Vec3::new(sx, sy, sz),
                        transform: BuilderTransform {
                            translation: attach,
                            rotation_x,
                            rotation_y,
                            scale: Vec3::one(),
                        },
                        material_slot: part.material.clone(),
                        host_position_normalized: false,
                        host_position_y_normalized: false,
                        host_scale_y_normalized: false,
                        host_scale_x_normalized: false,
                        host_scale_z_normalized: false,
                    }
                }
                BuilderScriptPartKind::Cylinder { length, radius } => {
                    let sy = eval_scalar_expr(length, self.host, &dims, &params)?;
                    let r = eval_scalar_expr(radius, self.host, &dims, &params)?;
                    resolved_parts.insert(
                        part.name.clone(),
                        resolved_cylinder_anchors(attach, sy, rotation_x, rotation_y),
                    );
                    BuilderPrimitive::Cylinder {
                        length: sy,
                        radius: r,
                        transform: BuilderTransform {
                            translation: attach,
                            rotation_x,
                            rotation_y,
                            scale: Vec3::one(),
                        },
                        material_slot: part.material.clone(),
                        host_position_normalized: false,
                        host_position_y_normalized: false,
                        host_scale_y_normalized: false,
                        host_scale_x_normalized: false,
                        host_scale_z_normalized: false,
                    }
                }
                BuilderScriptPartKind::Planks {
                    size,
                    count,
                    direction,
                    jitter,
                    alignment_jitter,
                    missing_chance,
                    seed,
                } => {
                    let sx = eval_scalar_expr(&size[0], self.host, &dims, &params)?;
                    let sy = eval_scalar_expr(&size[1], self.host, &dims, &params)?;
                    let sz = eval_scalar_expr(&size[2], self.host, &dims, &params)?;
                    let count = eval_scalar_expr(count, self.host, &dims, &params)?
                        .round()
                        .clamp(1.0, 256.0) as usize;
                    let jitter = jitter
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.15)
                        .clamp(0.0, 1.0);
                    let alignment_jitter = alignment_jitter
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(jitter)
                        .clamp(0.0, 1.0);
                    let missing_chance = missing_chance
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0)
                        .clamp(0.0, 1.0);
                    let seed = seed
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0)
                        .round() as u64;
                    resolved_parts.insert(
                        part.name.clone(),
                        resolved_box_anchors(attach, Vec3::new(sx, sy, sz), rotation_x, rotation_y),
                    );
                    let panel_transform = BuilderTransform {
                        translation: attach,
                        rotation_x,
                        rotation_y,
                        scale: Vec3::one(),
                    };
                    let plank_specs = plank_primitive_boxes(
                        Vec3::new(sx, sy, sz),
                        count,
                        *direction,
                        jitter,
                        alignment_jitter,
                        missing_chance,
                        seed,
                    );
                    for (local_bottom, plank_size) in plank_specs {
                        if emit_part {
                            primitives.push(BuilderPrimitive::Box {
                                size: plank_size,
                                transform: BuilderTransform {
                                    translation: transform_point(&panel_transform, local_bottom),
                                    rotation_x,
                                    rotation_y,
                                    scale: Vec3::one(),
                                },
                                material_slot: part.material.clone(),
                                host_position_normalized: false,
                                host_position_y_normalized: false,
                                host_scale_y_normalized: false,
                                host_scale_x_normalized: false,
                                host_scale_z_normalized: false,
                            });
                        }
                    }
                    continue;
                }
            };
            if emit_part {
                primitives.push(primitive);
            }
        }

        for (index, cut) in self.cuts.iter().enumerate() {
            match cut {
                BuilderScriptCut::Rect {
                    min,
                    max,
                    mode,
                    offset,
                    inset,
                    shape,
                } => {
                    let min = Vec2::new(
                        eval_scalar_expr(&min[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&min[1], self.host, &dims, &params)?,
                    );
                    let max = Vec2::new(
                        eval_scalar_expr(&max[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&max[1], self.host, &dims, &params)?,
                    );
                    let offset = offset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let inset = inset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    if max.x <= min.x || max.y <= min.y {
                        warnings.push(BuilderWarning {
                            code: "invalid_cut_rect".to_string(),
                            message: format!(
                                "rect cut {index} has invalid bounds min=({}, {}) max=({}, {})",
                                min.x, min.y, max.x, max.y
                            ),
                        });
                    } else {
                        cuts.push(BuilderCutMask::Rect {
                            min,
                            max,
                            mode: *mode,
                            offset,
                            inset,
                            shape: *shape,
                        });
                    }
                }
            }
        }

        for (index, detail) in self.details.iter().enumerate() {
            match detail {
                BuilderScriptSurfaceDetail::Rect {
                    min,
                    max,
                    offset,
                    inset,
                    shape,
                    material,
                    tile_alias,
                } => {
                    let min = Vec2::new(
                        eval_scalar_expr(&min[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&min[1], self.host, &dims, &params)?,
                    );
                    let max = Vec2::new(
                        eval_scalar_expr(&max[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&max[1], self.host, &dims, &params)?,
                    );
                    let offset = offset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let inset = inset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    if max.x <= min.x || max.y <= min.y {
                        warnings.push(BuilderWarning {
                            code: "invalid_detail_rect".to_string(),
                            message: format!(
                                "rect detail {index} has invalid bounds min=({}, {}) max=({}, {})",
                                min.x, min.y, max.x, max.y
                            ),
                        });
                    } else {
                        surface_details.push(BuilderSurfaceDetail::Rect {
                            min,
                            max,
                            offset,
                            inset,
                            shape: *shape,
                            material_slot: material.clone(),
                            tile_alias: tile_alias.clone(),
                        });
                    }
                }
                BuilderScriptSurfaceDetail::Column {
                    center,
                    height,
                    radius,
                    offset,
                    base_height,
                    cap_height,
                    transition_height,
                    segments,
                    placement,
                    cut_footprint,
                    material,
                    rect_material,
                    cyl_material,
                    tile_alias,
                } => {
                    let center = Vec2::new(
                        eval_scalar_expr(&center[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&center[1], self.host, &dims, &params)?,
                    );
                    let height = eval_scalar_expr(height, self.host, &dims, &params)?;
                    let radius = eval_scalar_expr(radius, self.host, &dims, &params)?;
                    let offset = offset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let base_height = base_height
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let cap_height = cap_height
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let transition_height = transition_height
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let segments = segments
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(default_column_segments() as f32)
                        .round()
                        .clamp(4.0, 32.0) as u16;
                    let placement = eval_placement_expr(placement, &ident_params)?;
                    if height <= 0.0 || radius <= 0.0 {
                        warnings.push(BuilderWarning {
                            code: "invalid_detail_column".to_string(),
                            message: format!(
                                "column detail {index} has invalid height/radius height={} radius={}",
                                height, radius
                            ),
                        });
                    } else {
                        surface_details.push(BuilderSurfaceDetail::Column {
                            center,
                            height,
                            radius,
                            offset,
                            base_height,
                            cap_height,
                            transition_height,
                            segments,
                            placement,
                            cut_footprint: *cut_footprint,
                            material_slot: material.clone(),
                            rect_material_slot: rect_material.clone(),
                            cyl_material_slot: cyl_material.clone(),
                            tile_alias: tile_alias.clone(),
                        });
                    }
                }
                BuilderScriptSurfaceDetail::ColumnSeries {
                    start,
                    end,
                    y,
                    spacing,
                    height,
                    radius,
                    broken_chance,
                    broken_min_height,
                    seed,
                    offset,
                    base_height,
                    cap_height,
                    transition_height,
                    segments,
                    placement,
                    cut_footprint,
                    material,
                    rect_material,
                    cyl_material,
                    tile_alias,
                } => {
                    let start = eval_scalar_expr(start, self.host, &dims, &params)?;
                    let end = eval_scalar_expr(end, self.host, &dims, &params)?;
                    let y = eval_scalar_expr(y, self.host, &dims, &params)?;
                    let spacing = eval_scalar_expr(spacing, self.host, &dims, &params)?.max(0.01);
                    let height = eval_scalar_expr(height, self.host, &dims, &params)?;
                    let radius = eval_scalar_expr(radius, self.host, &dims, &params)?;
                    let broken_chance = broken_chance
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0)
                        .clamp(0.0, 1.0);
                    let broken_min_height = broken_min_height
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.35)
                        .clamp(0.05, 1.0);
                    let seed = seed
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0)
                        .round() as u64;
                    let offset = offset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let base_height = base_height
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let cap_height = cap_height
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let transition_height = transition_height
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0);
                    let segments = segments
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(default_column_segments() as f32)
                        .round()
                        .clamp(4.0, 32.0) as u16;
                    let placement = eval_placement_expr(placement, &ident_params)?;
                    if height <= 0.0 || radius <= 0.0 || (end - start).abs() <= 0.001 {
                        warnings.push(BuilderWarning {
                            code: "invalid_detail_columns".to_string(),
                            message: format!(
                                "column series detail {index} has invalid span/height/radius start={} end={} height={} radius={}",
                                start, end, height, radius
                            ),
                        });
                    } else {
                        let span = end - start;
                        let gaps = ((span.abs() / spacing).round() as usize).max(1);
                        for step in 0..=gaps {
                            let t = step as f32 / gaps as f32;
                            let mut column_height = height;
                            if broken_chance > 0.0
                                && builder_param_rand01(seed, step as u64) < broken_chance
                            {
                                let range = (1.0 - broken_min_height).max(0.0);
                                let factor = broken_min_height
                                    + builder_param_rand01(seed ^ 0x9e3779b97f4a7c15, step as u64)
                                        * range
                                        * 0.65;
                                column_height = height * factor.clamp(0.05, 1.0);
                            }
                            surface_details.push(BuilderSurfaceDetail::Column {
                                center: Vec2::new(start + span * t, y),
                                height: column_height,
                                radius,
                                offset,
                                base_height,
                                cap_height,
                                transition_height,
                                segments,
                                placement,
                                cut_footprint: *cut_footprint,
                                material_slot: material.clone(),
                                rect_material_slot: rect_material.clone(),
                                cyl_material_slot: cyl_material.clone(),
                                tile_alias: tile_alias.clone(),
                            });
                        }
                    }
                }
                BuilderScriptSurfaceDetail::Masonry {
                    min,
                    max,
                    block,
                    mortar,
                    offset,
                    pattern,
                    material,
                    tile_alias,
                } => {
                    let min = Vec2::new(
                        eval_scalar_expr(&min[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&min[1], self.host, &dims, &params)?,
                    );
                    let max = Vec2::new(
                        eval_scalar_expr(&max[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&max[1], self.host, &dims, &params)?,
                    );
                    let block = Vec2::new(
                        eval_scalar_expr(&block[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&block[1], self.host, &dims, &params)?,
                    );
                    let mortar = mortar
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.04);
                    let offset = offset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(-0.035);
                    if max.x <= min.x || max.y <= min.y || block.x <= 0.0 || block.y <= 0.0 {
                        warnings.push(BuilderWarning {
                            code: "invalid_detail_masonry".to_string(),
                            message: format!(
                                "masonry detail {index} has invalid bounds or block size min=({}, {}) max=({}, {}) block=({}, {})",
                                min.x, min.y, max.x, max.y, block.x, block.y
                            ),
                        });
                    } else {
                        surface_details.push(BuilderSurfaceDetail::Masonry {
                            min,
                            max,
                            block,
                            mortar: mortar.max(0.0),
                            offset,
                            pattern: *pattern,
                            material_slot: material.clone(),
                            tile_alias: tile_alias.clone(),
                        });
                    }
                }
                BuilderScriptSurfaceDetail::Planks {
                    min,
                    max,
                    count,
                    direction,
                    jitter,
                    alignment_jitter,
                    missing_chance,
                    seed,
                    offset,
                    material,
                    tile_alias,
                } => {
                    let min = Vec2::new(
                        eval_scalar_expr(&min[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&min[1], self.host, &dims, &params)?,
                    );
                    let max = Vec2::new(
                        eval_scalar_expr(&max[0], self.host, &dims, &params)?,
                        eval_scalar_expr(&max[1], self.host, &dims, &params)?,
                    );
                    let count = eval_scalar_expr(count, self.host, &dims, &params)?
                        .round()
                        .clamp(1.0, 256.0) as usize;
                    let jitter = jitter
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.15)
                        .clamp(0.0, 1.0);
                    let alignment_jitter = alignment_jitter
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(jitter)
                        .clamp(0.0, 1.0);
                    let missing_chance = missing_chance
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0)
                        .clamp(0.0, 1.0);
                    let seed = seed
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(0.0)
                        .round() as u64;
                    let offset = offset
                        .as_ref()
                        .map(|expr| eval_scalar_expr(expr, self.host, &dims, &params))
                        .transpose()?
                        .unwrap_or(-0.035);
                    let local_min = Vec2::new(min.x.min(max.x), min.y.min(max.y));
                    let local_max = Vec2::new(min.x.max(max.x), min.y.max(max.y));
                    for (plank_min, plank_max) in plank_detail_rects(
                        local_min,
                        local_max,
                        count,
                        *direction,
                        jitter,
                        alignment_jitter,
                        missing_chance,
                        seed,
                    ) {
                        surface_details.push(BuilderSurfaceDetail::Rect {
                            min: plank_min,
                            max: plank_max,
                            offset,
                            inset: 0.0,
                            shape: BuilderCutShape::Fill,
                            material_slot: material.clone(),
                            tile_alias: tile_alias.clone(),
                        });
                    }
                }
            }
        }

        for slot in &self.slots {
            let point = eval_ref_point(&slot.source, self.host, &dims, &resolved_parts)?;
            anchors.push(BuilderAnchor {
                name: slot.name.clone(),
                kind: slot.kind,
                transform: BuilderTransform {
                    translation: point,
                    rotation_x: 0.0,
                    rotation_y: 0.0,
                    scale: Vec3::one(),
                },
                host_position_normalized: false,
                host_position_y_normalized: false,
                surface_extent: Vec2::zero(),
                surface_extent_normalized: false,
            });
        }

        Ok(BuilderAssembly {
            primitives,
            anchors,
            cuts,
            surface_details,
            static_billboards: Vec::new(),
            warnings,
        })
    }

    pub fn render_preview(&self, size: u32) -> BuilderPreview {
        match self.evaluate() {
            Ok(assembly) => {
                render_assembly_preview(size, self.output_spec(), &self.preview_host, &assembly)
            }
            Err(_) => empty_preview(size.max(32)),
        }
    }

    pub fn material_slot_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for part in &self.parts {
            if let Some(material) = &part.material
                && !material.trim().is_empty()
                && !out.iter().any(|existing| existing == material)
            {
                out.push(material.clone());
            }
        }
        for detail in &self.details {
            let mut materials = Vec::new();
            match detail {
                BuilderScriptSurfaceDetail::Rect { material, .. }
                | BuilderScriptSurfaceDetail::Masonry { material, .. }
                | BuilderScriptSurfaceDetail::Planks { material, .. } => {
                    materials.push(material);
                }
                BuilderScriptSurfaceDetail::Column {
                    material,
                    rect_material,
                    cyl_material,
                    ..
                }
                | BuilderScriptSurfaceDetail::ColumnSeries {
                    material,
                    rect_material,
                    cyl_material,
                    ..
                } => {
                    materials.push(material);
                    materials.push(rect_material);
                    materials.push(cyl_material);
                }
            }
            for material in materials.into_iter().flatten() {
                if !material.trim().is_empty() && !out.iter().any(|existing| existing == material) {
                    out.push(material.clone());
                }
            }
        }
        out
    }

    pub fn parameter_values(&self) -> Result<Vec<(String, BuilderScriptParameterValue)>, String> {
        let dims = HostPreviewDims::from_preview(self.host, &self.preview_host);
        let mut params: HashMap<String, f32> = HashMap::default();
        let mut out = Vec::new();
        for param in &self.params {
            match &param.value {
                BuilderScriptParamValue::Scalar(expr) => {
                    let value = eval_scalar_expr(expr, self.host, &dims, &params)?;
                    params.insert(param.name.clone(), value);
                    out.push((
                        param.name.clone(),
                        BuilderScriptParameterValue::Number(value),
                    ));
                }
                BuilderScriptParamValue::Ident(value) => {
                    out.push((
                        param.name.clone(),
                        BuilderScriptParameterValue::Ident(value.clone()),
                    ));
                }
            }
        }
        Ok(out)
    }

    pub fn item_slot_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        for slot in &self.slots {
            if slot.kind == BuilderAttachmentKind::Item
                && !slot.name.trim().is_empty()
                && !out.iter().any(|existing| existing == &slot.name)
            {
                out.push(slot.name.clone());
            }
        }
        out
    }
}

impl BuilderDocument {
    pub fn from_text(source: &str) -> Result<Self, String> {
        if let Ok(script) = BuilderScript::from_text(source) {
            return Ok(Self::Script(script));
        }
        Ok(Self::Graph(BuilderGraph::from_text(source)?))
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Script(script) => &script.name,
            Self::Graph(graph) => &graph.name,
        }
    }

    pub fn output_spec(&self) -> BuilderOutputSpec {
        match self {
            Self::Script(script) => script.output_spec(),
            Self::Graph(graph) => graph.output_spec(),
        }
    }

    pub fn material_slot_names(&self) -> Vec<String> {
        match self {
            Self::Script(script) => script.material_slot_names(),
            Self::Graph(graph) => graph.material_slot_names(),
        }
    }

    pub fn item_slot_names(&self) -> Vec<String> {
        match self {
            Self::Script(script) => script.item_slot_names(),
            Self::Graph(graph) => graph.item_slot_names(),
        }
    }

    pub fn parameter_values(&self) -> Result<Vec<(String, BuilderScriptParameterValue)>, String> {
        match self {
            Self::Script(script) => script.parameter_values(),
            Self::Graph(_) => Ok(Vec::new()),
        }
    }

    pub fn preview_host(&self) -> BuilderPreviewHost {
        match self {
            Self::Script(script) => script.preview_host.clone(),
            Self::Graph(graph) => graph
                .preview_host
                .clone()
                .unwrap_or_else(|| default_preview_host(graph.output_spec().target)),
        }
    }

    pub fn evaluate(&self) -> Result<BuilderAssembly, String> {
        match self {
            Self::Script(script) => script.evaluate(),
            Self::Graph(graph) => graph.evaluate(),
        }
    }

    pub fn evaluate_with_host(&self, host: &BuilderHost) -> Result<BuilderAssembly, String> {
        match self {
            Self::Script(script) => script.evaluate_with_host(host),
            Self::Graph(graph) => {
                let mut assembly = graph.evaluate()?;
                assembly.warnings.push(BuilderWarning {
                    code: "graph_host_ignored".to_string(),
                    message: "node graph evaluation currently ignores explicit BuilderHost; script evaluation is host-aware".to_string(),
                });
                Ok(assembly)
            }
        }
    }

    pub fn render_preview(&self, size: u32) -> BuilderPreview {
        match self {
            Self::Script(script) => script.render_preview(size),
            Self::Graph(graph) => graph.render_preview(size),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct HostPreviewDims {
    width: f32,
    depth: f32,
    height: f32,
}

impl HostPreviewDims {
    fn from_preview(host: BuilderScriptHost, preview: &BuilderPreviewHost) -> Self {
        match host {
            BuilderScriptHost::Line => Self {
                width: preview.width.max(0.01),
                depth: preview.depth.max(0.01),
                height: preview.height.max(0.01),
            },
            BuilderScriptHost::Sector => Self {
                width: preview.width.max(0.01),
                depth: preview.depth.max(0.01),
                height: preview.height.max(0.01),
            },
            BuilderScriptHost::Vertex => Self {
                width: preview.width.max(0.01),
                depth: preview.depth.max(0.01),
                height: preview.height.max(0.01),
            },
        }
    }

    fn from_host(host: &BuilderHost) -> Self {
        match host {
            BuilderHost::Object(host) => Self {
                width: host.width.max(0.01),
                depth: host.depth.max(0.01),
                height: host.height.max(0.01),
            },
            BuilderHost::Sector(host) => Self {
                width: host.width.max(0.01),
                depth: host.depth.max(0.01),
                height: host.height.max(0.01),
            },
            BuilderHost::Linedef(host) => Self {
                width: host.length.max(0.01),
                depth: host.width.max(0.01),
                height: host.height.max(0.01),
            },
            BuilderHost::Vertex(host) => Self {
                width: host.width.max(0.01),
                depth: host.depth.max(0.01),
                height: host.height.max(0.01),
            },
            BuilderHost::Surface(host) => Self {
                width: host.width.max(0.01),
                depth: host.thickness.max(0.01),
                height: host.height.max(0.01),
            },
            BuilderHost::Terrain(host) => Self {
                width: host.width.max(0.01),
                depth: host.depth.max(0.01),
                height: host.height.max(0.01),
            },
        }
    }
}

impl BuilderHost {
    pub fn kind_name(&self) -> &'static str {
        match self {
            BuilderHost::Object(_) => "object",
            BuilderHost::Sector(_) => "sector",
            BuilderHost::Linedef(_) => "linedef",
            BuilderHost::Vertex(_) => "vertex",
            BuilderHost::Surface(_) => "surface",
            BuilderHost::Terrain(_) => "terrain",
        }
    }

    pub fn preview_wall(width: f32, height: f32, thickness: f32) -> Self {
        Self::Surface(BuilderSurfaceHost {
            id: Uuid::new_v4(),
            seed: 0,
            width,
            height,
            thickness,
            side: 0,
        })
    }

    pub fn preview_object(width: f32, depth: f32, height: f32) -> Self {
        Self::Object(BuilderObjectHost {
            id: Uuid::new_v4(),
            seed: 0,
            width,
            depth,
            height,
        })
    }

    pub fn preview_floor(width: f32, depth: f32) -> Self {
        Self::Sector(BuilderSectorHost {
            id: Uuid::new_v4(),
            seed: 0,
            width,
            depth,
            height: 1.0,
        })
    }

    pub fn preview_linedef(length: f32, height: f32, width: f32) -> Self {
        Self::Linedef(BuilderLinedefHost {
            id: Uuid::new_v4(),
            seed: 0,
            length,
            height,
            width,
        })
    }

    pub fn preview_vertex(width: f32, depth: f32, height: f32) -> Self {
        Self::Vertex(BuilderVertexHost {
            id: Uuid::new_v4(),
            seed: 0,
            width,
            depth,
            height,
        })
    }

    pub fn preview_terrain(width: f32, depth: f32, seed: u64) -> Self {
        Self::Terrain(BuilderTerrainHost {
            id: Uuid::new_v4(),
            seed,
            width,
            depth,
            height: 0.0,
        })
    }
}

fn host_matches_script_host(host: &BuilderHost, script_host: BuilderScriptHost) -> bool {
    matches!(
        (host, script_host),
        (BuilderHost::Linedef(_), BuilderScriptHost::Line)
            | (BuilderHost::Sector(_), BuilderScriptHost::Sector)
            | (BuilderHost::Surface(_), BuilderScriptHost::Sector)
            | (BuilderHost::Terrain(_), BuilderScriptHost::Sector)
            | (BuilderHost::Vertex(_), BuilderScriptHost::Vertex)
            | (BuilderHost::Object(_), BuilderScriptHost::Sector)
    )
}

#[derive(Clone, Copy, Debug)]
struct ResolvedPart {
    bottom: BuilderTransform,
    center: BuilderTransform,
    top: BuilderTransform,
}

#[derive(Clone, Debug, PartialEq)]
enum BuilderScriptToken {
    Ident(String),
    Number(f32),
    String(String),
    Symbol(char),
}

struct BuilderScriptParser {
    tokens: Vec<BuilderScriptToken>,
    index: usize,
}

impl BuilderScriptParser {
    fn new(source: &str) -> Result<Self, String> {
        Ok(Self {
            tokens: lex_builder_script(source)?,
            index: 0,
        })
    }

    fn parse_script(&mut self) -> Result<BuilderScript, String> {
        let mut name = "Builder Script".to_string();
        let mut host = None;
        let mut preview_host = None;
        let mut params = Vec::new();
        let mut parts = Vec::new();
        let mut cuts = Vec::new();
        let mut details = Vec::new();
        let mut slots = Vec::new();
        let mut output = Vec::new();

        while !self.is_eof() {
            if self.peek_ident("name") {
                self.expect_ident("name")?;
                self.expect_symbol('=')?;
                name = self.expect_string_or_ident()?;
                self.expect_symbol(';')?;
            } else if self.peek_ident("host") {
                self.expect_ident("host")?;
                self.expect_symbol('=')?;
                host = Some(match self.expect_ident_any()?.as_str() {
                    "line" | "linedef" => BuilderScriptHost::Line,
                    "sector" => BuilderScriptHost::Sector,
                    "vertex" | "point" => BuilderScriptHost::Vertex,
                    other => return Err(format!("unsupported host '{other}'")),
                });
                self.expect_symbol(';')?;
            } else if self.peek_ident("param") {
                self.expect_ident("param")?;
                let name = self.expect_ident_any()?;
                self.expect_symbol('=')?;
                let value = if matches!(self.peek(), Some(BuilderScriptToken::Ident(_)))
                    && matches!(
                        self.tokens.get(self.index + 1),
                        Some(BuilderScriptToken::Symbol(';'))
                    ) {
                    BuilderScriptParamValue::Ident(self.expect_ident_any()?)
                } else {
                    BuilderScriptParamValue::Scalar(self.parse_scalar_expr()?)
                };
                self.expect_symbol(';')?;
                params.push(BuilderScriptParam { name, value });
            } else if self.peek_ident("preview") {
                self.expect_ident("preview")?;
                self.expect_symbol('{')?;
                let mut preview = BuilderPreviewHost::default();
                while !self.consume_symbol('}') {
                    let key = self.expect_ident_any()?;
                    self.expect_symbol('=')?;
                    match key.as_str() {
                        "width" => preview.width = self.expect_number()?,
                        "depth" => preview.depth = self.expect_number()?,
                        "height" => preview.height = self.expect_number()?,
                        "surface" | "orientation" => {
                            preview.surface = match self.expect_ident_any()?.as_str() {
                                "floor" | "sector" | "horizontal" => BuilderPreviewSurface::Floor,
                                "wall" | "surface" | "vertical" => BuilderPreviewSurface::Wall,
                                other => {
                                    return Err(format!("unsupported preview surface '{other}'"));
                                }
                            }
                        }
                        other => return Err(format!("unsupported preview field '{other}'")),
                    }
                    self.expect_symbol(';')?;
                }
                preview_host = Some(preview);
            } else if self.peek_ident("let") {
                parts.push(self.parse_part()?);
            } else if self.peek_ident("cut") {
                cuts.push(self.parse_cut()?);
            } else if self.peek_ident("detail") {
                details.push(self.parse_detail()?);
            } else if self.peek_ident("slot") {
                slots.push(self.parse_slot()?);
            } else if self.peek_ident("output") {
                self.expect_ident("output")?;
                self.expect_symbol('=')?;
                self.expect_symbol('[')?;
                while !self.consume_symbol(']') {
                    output.push(self.expect_ident_any()?);
                    let _ = self.consume_symbol(',');
                }
                self.expect_symbol(';')?;
            } else {
                return Err(format!(
                    "unexpected token {:?} in builder script",
                    self.tokens.get(self.index)
                ));
            }
        }

        let host = host.ok_or_else(|| "builder script is missing 'host = ...;'".to_string())?;
        let preview_host = preview_host.unwrap_or_else(|| {
            default_preview_host(match host {
                BuilderScriptHost::Line => BuilderOutputTarget::Linedef,
                BuilderScriptHost::Sector => BuilderOutputTarget::Sector,
                BuilderScriptHost::Vertex => BuilderOutputTarget::VertexPair,
            })
        });

        Ok(BuilderScript {
            name,
            host,
            preview_host,
            params,
            parts,
            cuts,
            details,
            slots,
            output,
        })
    }

    fn parse_part(&mut self) -> Result<BuilderScriptPart, String> {
        self.expect_ident("let")?;
        let name = self.expect_ident_any()?;
        self.expect_symbol('=')?;
        let kind_name = self.expect_ident_any()?;
        self.expect_symbol('{')?;

        let mut attach = None;
        let mut parent = None;
        let mut size = None;
        let mut material = None;
        let mut length = None;
        let mut radius = None;
        let mut axis = None;
        let mut rotate_x = None;
        let mut rotate_y = None;
        let mut count = None;
        let mut direction = BuilderPlankDirection::Horizontal;
        let mut jitter = None;
        let mut alignment_jitter = None;
        let mut missing_chance = None;
        let mut seed = None;

        while !self.consume_symbol('}') {
            let key = self.expect_ident_any()?;
            self.expect_symbol('=')?;
            match key.as_str() {
                "attach" => attach = Some(self.parse_point_expr()?),
                "parent" => parent = Some(self.parse_ref()?),
                "size" => size = Some(self.parse_vec3_expr()?),
                "material" => material = Some(self.expect_ident_any()?),
                "length" => length = Some(self.parse_scalar_expr()?),
                "radius" => radius = Some(self.parse_scalar_expr()?),
                "axis" => axis = Some(self.parse_ref()?),
                "rotate_x" => rotate_x = Some(self.parse_scalar_expr()?),
                "rotate_y" => rotate_y = Some(self.parse_scalar_expr()?),
                "count" => count = Some(self.parse_scalar_expr()?),
                "jitter" | "variation" => jitter = Some(self.parse_scalar_expr()?),
                "alignment_jitter" | "align_jitter" | "alignment" | "unevenness" => {
                    alignment_jitter = Some(self.parse_scalar_expr()?)
                }
                "missing_chance" | "skip_chance" => {
                    missing_chance = Some(self.parse_scalar_expr()?)
                }
                "seed" => seed = Some(self.parse_scalar_expr()?),
                "direction" | "dir" => {
                    direction = match self.expect_ident_any()?.as_str() {
                        "horizontal" | "x" | "along" => BuilderPlankDirection::Horizontal,
                        "vertical" | "y" | "up" => BuilderPlankDirection::Vertical,
                        other => return Err(format!("unsupported plank direction '{other}'")),
                    };
                }
                other => return Err(format!("unsupported part field '{other}'")),
            }
            self.expect_symbol(';')?;
        }
        self.expect_symbol(';')?;

        let attach = attach.ok_or_else(|| format!("part '{name}' is missing attach"))?;
        let kind = match kind_name.as_str() {
            "box" => BuilderScriptPartKind::Box {
                size: size.ok_or_else(|| format!("part '{name}' is missing size"))?,
            },
            "cylinder" => BuilderScriptPartKind::Cylinder {
                length: length.ok_or_else(|| format!("part '{name}' is missing length"))?,
                radius: radius.ok_or_else(|| format!("part '{name}' is missing radius"))?,
            },
            "planks" | "boards" => BuilderScriptPartKind::Planks {
                size: size.ok_or_else(|| format!("part '{name}' is missing size"))?,
                count: count.ok_or_else(|| format!("part '{name}' is missing count"))?,
                direction,
                jitter,
                alignment_jitter,
                missing_chance,
                seed,
            },
            other => return Err(format!("unsupported primitive '{other}'")),
        };

        Ok(BuilderScriptPart {
            name,
            kind,
            attach,
            parent,
            material,
            axis,
            rotate_x,
            rotate_y,
        })
    }

    fn parse_cut(&mut self) -> Result<BuilderScriptCut, String> {
        self.expect_ident("cut")?;
        let kind_name = self.expect_ident_any()?;
        self.expect_symbol('{')?;

        let mut min = None;
        let mut max = None;
        let mut mode = BuilderCutMode::Cut;
        let mut offset = None;
        let mut inset = None;
        let mut shape = BuilderCutShape::Fill;

        while !self.consume_symbol('}') {
            let key = self.expect_ident_any()?;
            self.expect_symbol('=')?;
            match key.as_str() {
                "min" => min = Some(self.parse_vec2_expr()?),
                "max" => max = Some(self.parse_vec2_expr()?),
                "offset" => offset = Some(self.parse_scalar_expr()?),
                "inset" => inset = Some(self.parse_scalar_expr()?),
                "shape" => {
                    shape = match self.expect_ident_any()?.as_str() {
                        "fill" => BuilderCutShape::Fill,
                        "border" => BuilderCutShape::Border,
                        other => return Err(format!("unsupported cut shape '{other}'")),
                    };
                }
                "mode" => {
                    mode = match self.expect_ident_any()?.as_str() {
                        "cut" => BuilderCutMode::Cut,
                        "replace" => BuilderCutMode::Replace,
                        "cut_overlay" | "cutoverlay" => BuilderCutMode::CutOverlay,
                        other => return Err(format!("unsupported cut mode '{other}'")),
                    };
                }
                other => return Err(format!("unsupported cut field '{other}'")),
            }
            self.expect_symbol(';')?;
        }
        self.expect_symbol(';')?;

        match kind_name.as_str() {
            "rect" => Ok(BuilderScriptCut::Rect {
                min: min.ok_or_else(|| "rect cut is missing min".to_string())?,
                max: max.ok_or_else(|| "rect cut is missing max".to_string())?,
                mode,
                offset,
                inset,
                shape,
            }),
            other => Err(format!("unsupported cut primitive '{other}'")),
        }
    }

    fn parse_detail(&mut self) -> Result<BuilderScriptSurfaceDetail, String> {
        self.expect_ident("detail")?;
        let kind_name = self.expect_ident_any()?;
        self.expect_symbol('{')?;

        let mut min = None;
        let mut max = None;
        let mut center = None;
        let mut start = None;
        let mut end = None;
        let mut y = None;
        let mut spacing = None;
        let mut block = None;
        let mut count = None;
        let mut height = None;
        let mut radius = None;
        let mut broken_chance = None;
        let mut missing_chance = None;
        let mut broken_min_height = None;
        let mut jitter = None;
        let mut alignment_jitter = None;
        let mut seed = None;
        let mut offset = None;
        let mut mortar = None;
        let mut inset = None;
        let mut base_height = None;
        let mut cap_height = None;
        let mut transition_height = None;
        let mut segments = None;
        let mut placement = BuilderScriptPlacementExpr::Literal(BuilderDetailPlacement::Relief);
        let mut cut_footprint = false;
        let mut shape = BuilderCutShape::Fill;
        let mut pattern = BuilderMasonryPattern::Grid;
        let mut direction = BuilderPlankDirection::Horizontal;
        let mut material = None;
        let mut rect_material = None;
        let mut cyl_material = None;
        let mut tile_alias = None;

        while !self.consume_symbol('}') {
            let key = self.expect_ident_any()?;
            self.expect_symbol('=')?;
            match key.as_str() {
                "min" => min = Some(self.parse_vec2_expr()?),
                "max" => max = Some(self.parse_vec2_expr()?),
                "center" => center = Some(self.parse_vec2_expr()?),
                "start" => start = Some(self.parse_scalar_expr()?),
                "end" => end = Some(self.parse_scalar_expr()?),
                "y" => y = Some(self.parse_scalar_expr()?),
                "spacing" | "step" | "approx_spacing" => spacing = Some(self.parse_scalar_expr()?),
                "count" => count = Some(self.parse_scalar_expr()?),
                "block" | "block_size" | "stone" | "stone_size" => {
                    block = Some(self.parse_vec2_expr()?)
                }
                "height" => height = Some(self.parse_scalar_expr()?),
                "radius" => radius = Some(self.parse_scalar_expr()?),
                "broken_chance" | "break_chance" => broken_chance = Some(self.parse_scalar_expr()?),
                "missing_chance" | "skip_chance" => {
                    missing_chance = Some(self.parse_scalar_expr()?)
                }
                "jitter" | "variation" => jitter = Some(self.parse_scalar_expr()?),
                "alignment_jitter" | "align_jitter" | "alignment" | "unevenness" => {
                    alignment_jitter = Some(self.parse_scalar_expr()?)
                }
                "broken_min_height" | "break_min_height" => {
                    broken_min_height = Some(self.parse_scalar_expr()?)
                }
                "seed" => seed = Some(self.parse_scalar_expr()?),
                "offset" => offset = Some(self.parse_scalar_expr()?),
                "mortar" | "gap" => mortar = Some(self.parse_scalar_expr()?),
                "inset" => inset = Some(self.parse_scalar_expr()?),
                "base_height" | "base" => base_height = Some(self.parse_scalar_expr()?),
                "cap_height" | "cap" => cap_height = Some(self.parse_scalar_expr()?),
                "transition_height" | "transition" => {
                    transition_height = Some(self.parse_scalar_expr()?)
                }
                "segments" => segments = Some(self.parse_scalar_expr()?),
                "pattern" => {
                    pattern = match self.expect_ident_any()?.as_str() {
                        "grid" | "stacked" => BuilderMasonryPattern::Grid,
                        "running_bond" | "bond" | "brick" => BuilderMasonryPattern::RunningBond,
                        other => return Err(format!("unsupported masonry pattern '{other}'")),
                    };
                }
                "direction" | "dir" => {
                    direction = match self.expect_ident_any()?.as_str() {
                        "horizontal" | "x" | "along" => BuilderPlankDirection::Horizontal,
                        "vertical" | "y" | "up" => BuilderPlankDirection::Vertical,
                        other => return Err(format!("unsupported plank direction '{other}'")),
                    };
                }
                "placement" => {
                    let name = self.expect_ident_any()?;
                    placement = match parse_placement_ident(&name) {
                        Some(placement) => BuilderScriptPlacementExpr::Literal(placement),
                        None => BuilderScriptPlacementExpr::Param(name),
                    };
                }
                "cut_footprint" | "footprint_cut" => {
                    cut_footprint = match self.expect_ident_any()?.as_str() {
                        "true" | "yes" | "on" => true,
                        "false" | "no" | "off" => false,
                        other => return Err(format!("unsupported cut_footprint value '{other}'")),
                    };
                }
                "shape" => {
                    shape = match self.expect_ident_any()?.as_str() {
                        "fill" => BuilderCutShape::Fill,
                        "border" => BuilderCutShape::Border,
                        other => return Err(format!("unsupported detail shape '{other}'")),
                    };
                }
                "material" => material = Some(self.expect_ident_any()?),
                "rect_material" | "block_material" => {
                    rect_material = Some(self.expect_ident_any()?)
                }
                "cyl_material" | "shaft_material" | "round_material" => {
                    cyl_material = Some(self.expect_ident_any()?)
                }
                "tile_alias" | "alias" => tile_alias = Some(self.expect_ident_any()?),
                other => return Err(format!("unsupported detail field '{other}'")),
            }
            self.expect_symbol(';')?;
        }
        self.expect_symbol(';')?;

        match kind_name.as_str() {
            "rect" => Ok(BuilderScriptSurfaceDetail::Rect {
                min: min.ok_or_else(|| "rect detail is missing min".to_string())?,
                max: max.ok_or_else(|| "rect detail is missing max".to_string())?,
                offset,
                inset,
                shape,
                material,
                tile_alias,
            }),
            "masonry" | "blocks" | "brick" | "stonework" => {
                Ok(BuilderScriptSurfaceDetail::Masonry {
                    min: min.ok_or_else(|| "masonry detail is missing min".to_string())?,
                    max: max.ok_or_else(|| "masonry detail is missing max".to_string())?,
                    block: block.ok_or_else(|| "masonry detail is missing block".to_string())?,
                    mortar,
                    offset,
                    pattern,
                    material,
                    tile_alias,
                })
            }
            "planks" | "boards" | "woodwork" | "shingles" => {
                Ok(BuilderScriptSurfaceDetail::Planks {
                    min: min.ok_or_else(|| "planks detail is missing min".to_string())?,
                    max: max.ok_or_else(|| "planks detail is missing max".to_string())?,
                    count: count.ok_or_else(|| "planks detail is missing count".to_string())?,
                    direction,
                    jitter,
                    alignment_jitter,
                    missing_chance,
                    seed,
                    offset,
                    material,
                    tile_alias,
                })
            }
            "column" | "pilaster" => Ok(BuilderScriptSurfaceDetail::Column {
                center: center.ok_or_else(|| "column detail is missing center".to_string())?,
                height: height.ok_or_else(|| "column detail is missing height".to_string())?,
                radius: radius.ok_or_else(|| "column detail is missing radius".to_string())?,
                offset,
                base_height,
                cap_height,
                transition_height,
                segments,
                placement,
                cut_footprint,
                material,
                rect_material,
                cyl_material,
                tile_alias,
            }),
            "columns" | "column_series" | "pilasters" => {
                Ok(BuilderScriptSurfaceDetail::ColumnSeries {
                    start: start
                        .ok_or_else(|| "column series detail is missing start".to_string())?,
                    end: end.ok_or_else(|| "column series detail is missing end".to_string())?,
                    y: y.ok_or_else(|| "column series detail is missing y".to_string())?,
                    spacing: spacing
                        .ok_or_else(|| "column series detail is missing spacing".to_string())?,
                    height: height
                        .ok_or_else(|| "column series detail is missing height".to_string())?,
                    radius: radius
                        .ok_or_else(|| "column series detail is missing radius".to_string())?,
                    broken_chance,
                    broken_min_height,
                    seed,
                    offset,
                    base_height,
                    cap_height,
                    transition_height,
                    segments,
                    placement,
                    cut_footprint,
                    material,
                    rect_material,
                    cyl_material,
                    tile_alias,
                })
            }
            other => Err(format!("unsupported detail primitive '{other}'")),
        }
    }

    fn parse_slot(&mut self) -> Result<BuilderScriptSlot, String> {
        self.expect_ident("slot")?;
        let kind = match self.expect_ident_any()?.as_str() {
            "item" => BuilderAttachmentKind::Item,
            "material" => BuilderAttachmentKind::Material,
            other => return Err(format!("unsupported slot kind '{other}'")),
        };
        let name = self.expect_ident_any()?;
        self.expect_symbol('=')?;
        let source = self.parse_ref()?;
        self.expect_symbol(';')?;
        Ok(BuilderScriptSlot { name, kind, source })
    }

    fn parse_vec3_expr(&mut self) -> Result<[BuilderScriptScalarExpr; 3], String> {
        self.expect_ident("vec3")?;
        self.expect_symbol('(')?;
        let x = self.parse_scalar_expr()?;
        self.expect_symbol(',')?;
        let y = self.parse_scalar_expr()?;
        self.expect_symbol(',')?;
        let z = self.parse_scalar_expr()?;
        self.expect_symbol(')')?;
        Ok([x, y, z])
    }

    fn parse_vec2_expr(&mut self) -> Result<[BuilderScriptScalarExpr; 2], String> {
        self.expect_ident("vec2")?;
        self.expect_symbol('(')?;
        let x = self.parse_scalar_expr()?;
        self.expect_symbol(',')?;
        let y = self.parse_scalar_expr()?;
        self.expect_symbol(')')?;
        Ok([x, y])
    }

    fn parse_point_expr(&mut self) -> Result<BuilderScriptPointExpr, String> {
        let mut terms = Vec::new();
        let mut sign = 1.0f32;
        loop {
            if self.consume_symbol('+') {
                sign = 1.0;
                continue;
            }
            if self.consume_symbol('-') {
                sign = -1.0;
            }
            let term = if self.peek_ident("vec3") {
                BuilderScriptVecExpr::Literal(self.parse_vec3_expr()?)
            } else {
                let reference = self.parse_ref()?;
                if self.consume_symbol('*') {
                    BuilderScriptVecExpr::ScaledRef(reference, self.parse_scalar_factor()?)
                } else {
                    BuilderScriptVecExpr::Ref(reference)
                }
            };
            terms.push((sign, term));
            if !(self.peek_symbol('+') || self.peek_symbol('-')) {
                break;
            }
            sign = 1.0;
        }
        Ok(BuilderScriptPointExpr { terms })
    }

    fn parse_scalar_expr(&mut self) -> Result<BuilderScriptScalarExpr, String> {
        let mut expr = self.parse_scalar_term()?;
        loop {
            if self.consume_symbol('+') {
                let rhs = self.parse_scalar_term()?;
                expr = BuilderScriptScalarExpr::Add(Box::new(expr), Box::new(rhs));
            } else if self.consume_symbol('-') {
                let rhs = self.parse_scalar_term()?;
                expr = BuilderScriptScalarExpr::Sub(Box::new(expr), Box::new(rhs));
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_scalar_term(&mut self) -> Result<BuilderScriptScalarExpr, String> {
        let mut expr = self.parse_scalar_factor()?;
        loop {
            if self.consume_symbol('*') {
                let rhs = self.parse_scalar_factor()?;
                expr = BuilderScriptScalarExpr::Mul(Box::new(expr), Box::new(rhs));
            } else if self.consume_symbol('/') {
                let rhs = self.parse_scalar_factor()?;
                expr = BuilderScriptScalarExpr::Div(Box::new(expr), Box::new(rhs));
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn parse_scalar_factor(&mut self) -> Result<BuilderScriptScalarExpr, String> {
        if self.consume_symbol('-') {
            return Ok(BuilderScriptScalarExpr::Neg(Box::new(
                self.parse_scalar_factor()?,
            )));
        }
        match self.peek() {
            Some(BuilderScriptToken::Number(_)) => {
                Ok(BuilderScriptScalarExpr::Constant(self.expect_number()?))
            }
            Some(BuilderScriptToken::Ident(_)) => {
                Ok(BuilderScriptScalarExpr::Ref(self.parse_ref()?))
            }
            Some(BuilderScriptToken::Symbol('(')) => {
                self.expect_symbol('(')?;
                let expr = self.parse_scalar_expr()?;
                self.expect_symbol(')')?;
                Ok(expr)
            }
            other => Err(format!("expected scalar expression, got {other:?}")),
        }
    }

    fn parse_ref(&mut self) -> Result<BuilderScriptRef, String> {
        let left = self.expect_ident_any()?;
        if !self.consume_symbol('.') {
            return Ok(BuilderScriptRef::Param(left));
        }
        let right = self.expect_ident_any()?;
        if left == "host" {
            Ok(BuilderScriptRef::Host(right))
        } else {
            Ok(BuilderScriptRef::Part(left, right))
        }
    }

    fn peek(&self) -> Option<&BuilderScriptToken> {
        self.tokens.get(self.index)
    }

    fn is_eof(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn peek_symbol(&self, symbol: char) -> bool {
        matches!(self.peek(), Some(BuilderScriptToken::Symbol(current)) if *current == symbol)
    }

    fn consume_symbol(&mut self, symbol: char) -> bool {
        if self.peek_symbol(symbol) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn expect_symbol(&mut self, symbol: char) -> Result<(), String> {
        if self.consume_symbol(symbol) {
            Ok(())
        } else {
            Err(format!("expected symbol '{symbol}'"))
        }
    }

    fn peek_ident(&self, ident: &str) -> bool {
        matches!(self.peek(), Some(BuilderScriptToken::Ident(current)) if current == ident)
    }

    fn expect_ident(&mut self, ident: &str) -> Result<(), String> {
        let current = self.expect_ident_any()?;
        if current == ident {
            Ok(())
        } else {
            Err(format!("expected identifier '{ident}', got '{current}'"))
        }
    }

    fn expect_ident_any(&mut self) -> Result<String, String> {
        match self.tokens.get(self.index).cloned() {
            Some(BuilderScriptToken::Ident(value)) => {
                self.index += 1;
                Ok(value)
            }
            other => Err(format!("expected identifier, got {other:?}")),
        }
    }

    fn expect_string_or_ident(&mut self) -> Result<String, String> {
        match self.tokens.get(self.index).cloned() {
            Some(BuilderScriptToken::String(value)) => {
                self.index += 1;
                Ok(value)
            }
            Some(BuilderScriptToken::Ident(_)) => self.expect_ident_any(),
            other => Err(format!("expected string or identifier, got {other:?}")),
        }
    }

    fn expect_number(&mut self) -> Result<f32, String> {
        match self.tokens.get(self.index).cloned() {
            Some(BuilderScriptToken::Number(value)) => {
                self.index += 1;
                Ok(value)
            }
            other => Err(format!("expected number, got {other:?}")),
        }
    }
}

fn lex_builder_script(source: &str) -> Result<Vec<BuilderScriptToken>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0usize;
    let mut tokens = Vec::new();

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '/' && index + 1 < chars.len() && chars[index + 1] == '/' {
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            tokens.push(BuilderScriptToken::Ident(
                chars[start..index].iter().collect::<String>(),
            ));
            continue;
        }
        if ch.is_ascii_digit()
            || (ch == '.' && index + 1 < chars.len() && chars[index + 1].is_ascii_digit())
        {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_digit()
                    || chars[index] == '.'
                    || chars[index] == 'e'
                    || chars[index] == 'E'
                    || chars[index] == '+'
                    || chars[index] == '-')
            {
                if (chars[index] == '+' || chars[index] == '-')
                    && chars[index - 1] != 'e'
                    && chars[index - 1] != 'E'
                {
                    break;
                }
                index += 1;
            }
            let raw: String = chars[start..index].iter().collect();
            let value = raw
                .parse::<f32>()
                .map_err(|err| format!("invalid number '{raw}': {err}"))?;
            tokens.push(BuilderScriptToken::Number(value));
            continue;
        }
        if ch == '"' {
            index += 1;
            let start = index;
            while index < chars.len() && chars[index] != '"' {
                index += 1;
            }
            if index >= chars.len() {
                return Err("unterminated string literal".to_string());
            }
            tokens.push(BuilderScriptToken::String(
                chars[start..index].iter().collect::<String>(),
            ));
            index += 1;
            continue;
        }
        if "{}[](),;=.+-*/".contains(ch) {
            tokens.push(BuilderScriptToken::Symbol(ch));
            index += 1;
            continue;
        }
        return Err(format!("unexpected character '{ch}' in builder script"));
    }

    Ok(tokens)
}

fn eval_point_expr(
    expr: &BuilderScriptPointExpr,
    host: BuilderScriptHost,
    dims: &HostPreviewDims,
    parts: &HashMap<String, ResolvedPart>,
    params: &HashMap<String, f32>,
) -> Result<Vec3<f32>, String> {
    let mut result = Vec3::zero();
    for (sign, term) in &expr.terms {
        let value = match term {
            BuilderScriptVecExpr::Ref(reference) => eval_ref_point(reference, host, dims, parts)?,
            BuilderScriptVecExpr::ScaledRef(reference, scalar) => {
                eval_ref_vector(reference, host, dims, parts)?
                    * eval_scalar_expr(scalar, host, dims, params)?
            }
            BuilderScriptVecExpr::Literal(values) => Vec3::new(
                eval_scalar_expr(&values[0], host, dims, params)?,
                eval_scalar_expr(&values[1], host, dims, params)?,
                eval_scalar_expr(&values[2], host, dims, params)?,
            ),
        };
        result += value * *sign;
    }
    Ok(result)
}

fn eval_scalar_expr(
    expr: &BuilderScriptScalarExpr,
    host: BuilderScriptHost,
    dims: &HostPreviewDims,
    params: &HashMap<String, f32>,
) -> Result<f32, String> {
    match expr {
        BuilderScriptScalarExpr::Constant(value) => Ok(*value),
        BuilderScriptScalarExpr::Ref(reference) => eval_ref_scalar(reference, host, dims, params),
        BuilderScriptScalarExpr::Add(a, b) => {
            Ok(eval_scalar_expr(a, host, dims, params)? + eval_scalar_expr(b, host, dims, params)?)
        }
        BuilderScriptScalarExpr::Sub(a, b) => {
            Ok(eval_scalar_expr(a, host, dims, params)? - eval_scalar_expr(b, host, dims, params)?)
        }
        BuilderScriptScalarExpr::Mul(a, b) => {
            Ok(eval_scalar_expr(a, host, dims, params)? * eval_scalar_expr(b, host, dims, params)?)
        }
        BuilderScriptScalarExpr::Div(a, b) => {
            Ok(eval_scalar_expr(a, host, dims, params)? / eval_scalar_expr(b, host, dims, params)?)
        }
        BuilderScriptScalarExpr::Neg(inner) => Ok(-eval_scalar_expr(inner, host, dims, params)?),
    }
}

fn eval_ref_scalar(
    reference: &BuilderScriptRef,
    _host: BuilderScriptHost,
    dims: &HostPreviewDims,
    params: &HashMap<String, f32>,
) -> Result<f32, String> {
    match reference {
        BuilderScriptRef::Host(name) => match name.as_str() {
            "length" => Ok(dims.width),
            "width" => Ok(dims.width),
            "depth" => Ok(dims.depth),
            "height" => Ok(dims.height),
            "middle"
            | "top"
            | "bottom"
            | "left"
            | "right"
            | "along"
            | "up"
            | "out"
            | "top_left_corner"
            | "top_right_corner"
            | "bottom_left_corner"
            | "bottom_right_corner" => Err(format!("'host.{name}' is a vector, not a scalar")),
            _ => Err(format!("unsupported host scalar 'host.{name}'")),
        },
        BuilderScriptRef::Param(name) => params
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown builder parameter '{name}'")),
        BuilderScriptRef::Part(part, anchor) => Err(format!(
            "'{part}.{anchor}' cannot be used as a scalar in this builder script"
        )),
    }
}

fn eval_ref_vector(
    reference: &BuilderScriptRef,
    host: BuilderScriptHost,
    dims: &HostPreviewDims,
    parts: &HashMap<String, ResolvedPart>,
) -> Result<Vec3<f32>, String> {
    match reference {
        BuilderScriptRef::Host(name) => match name.as_str() {
            "along" => Ok(Vec3::new(1.0, 0.0, 0.0)),
            "up" => Ok(Vec3::new(0.0, 1.0, 0.0)),
            "out" => Ok(Vec3::new(0.0, 0.0, 1.0)),
            "middle" => Ok(host_anchor(host, dims, "middle")),
            "top" => Ok(host_anchor(host, dims, "top")),
            "bottom" => Ok(host_anchor(host, dims, "bottom")),
            "left" => Ok(host_anchor(host, dims, "left")),
            "right" => Ok(host_anchor(host, dims, "right")),
            "top_left_corner" => Ok(host_anchor(host, dims, "top_left_corner")),
            "top_right_corner" => Ok(host_anchor(host, dims, "top_right_corner")),
            "bottom_left_corner" => Ok(host_anchor(host, dims, "bottom_left_corner")),
            "bottom_right_corner" => Ok(host_anchor(host, dims, "bottom_right_corner")),
            _ => Err(format!("unsupported host vector 'host.{name}'")),
        },
        BuilderScriptRef::Param(name) => Err(format!(
            "builder parameter '{name}' cannot be used as a vector"
        )),
        BuilderScriptRef::Part(part, anchor) => {
            let resolved = parts
                .get(part)
                .ok_or_else(|| format!("unknown part '{part}'"))?;
            match anchor.as_str() {
                "bottom" | "base" => Ok(resolved.bottom.translation),
                "center" | "mid" => Ok(resolved.center.translation),
                "top" | "tip" => Ok(resolved.top.translation),
                _ => Err(format!("unsupported part anchor '{part}.{anchor}'")),
            }
        }
    }
}

fn eval_ref_point(
    reference: &BuilderScriptRef,
    host: BuilderScriptHost,
    dims: &HostPreviewDims,
    parts: &HashMap<String, ResolvedPart>,
) -> Result<Vec3<f32>, String> {
    eval_ref_vector(reference, host, dims, parts)
}

fn eval_ref_transform(
    reference: &BuilderScriptRef,
    host: BuilderScriptHost,
    dims: &HostPreviewDims,
    parts: &HashMap<String, ResolvedPart>,
) -> Result<BuilderTransform, String> {
    match reference {
        BuilderScriptRef::Host(name) => Ok(BuilderTransform {
            translation: host_anchor(host, dims, name),
            rotation_x: 0.0,
            rotation_y: 0.0,
            scale: Vec3::one(),
        }),
        BuilderScriptRef::Param(name) => Err(format!(
            "builder parameter '{name}' cannot be used as a transform"
        )),
        BuilderScriptRef::Part(part, anchor) => {
            let resolved = parts
                .get(part)
                .ok_or_else(|| format!("unknown part '{part}'"))?;
            match anchor.as_str() {
                "bottom" | "base" => Ok(resolved.bottom),
                "center" | "mid" => Ok(resolved.center),
                "top" | "tip" => Ok(resolved.top),
                _ => Err(format!("unsupported part anchor '{part}.{anchor}'")),
            }
        }
    }
}

fn transform_point(transform: &BuilderTransform, local: Vec3<f32>) -> Vec3<f32> {
    transform.translation + rotate_y(rotate_x(local, transform.rotation_x), transform.rotation_y)
}

fn host_anchor(host: BuilderScriptHost, dims: &HostPreviewDims, name: &str) -> Vec3<f32> {
    match host {
        BuilderScriptHost::Line => match name {
            "middle" | "bottom" => Vec3::zero(),
            "top" => Vec3::new(0.0, dims.height, 0.0),
            "left" => Vec3::new(-dims.width * 0.5, 0.0, 0.0),
            "right" => Vec3::new(dims.width * 0.5, 0.0, 0.0),
            "top_left_corner" => Vec3::new(-dims.width * 0.5, dims.height, 0.0),
            "top_right_corner" => Vec3::new(dims.width * 0.5, dims.height, 0.0),
            "bottom_left_corner" => Vec3::new(-dims.width * 0.5, 0.0, 0.0),
            "bottom_right_corner" => Vec3::new(dims.width * 0.5, 0.0, 0.0),
            _ => Vec3::zero(),
        },
        BuilderScriptHost::Sector => match name {
            "middle" | "bottom" => Vec3::zero(),
            "top" => Vec3::new(0.0, 0.0, -dims.depth * 0.5),
            "left" => Vec3::new(-dims.width * 0.5, 0.0, 0.0),
            "right" => Vec3::new(dims.width * 0.5, 0.0, 0.0),
            "top_left_corner" => Vec3::new(-dims.width * 0.5, 0.0, -dims.depth * 0.5),
            "top_right_corner" => Vec3::new(dims.width * 0.5, 0.0, -dims.depth * 0.5),
            "bottom_left_corner" => Vec3::new(-dims.width * 0.5, 0.0, dims.depth * 0.5),
            "bottom_right_corner" => Vec3::new(dims.width * 0.5, 0.0, dims.depth * 0.5),
            _ => Vec3::zero(),
        },
        BuilderScriptHost::Vertex => Vec3::zero(),
    }
}

fn resolved_box_anchors(
    attach: Vec3<f32>,
    size: Vec3<f32>,
    rotation_x: f32,
    rotation_y: f32,
) -> ResolvedPart {
    let bottom = BuilderTransform {
        translation: attach,
        rotation_x,
        rotation_y,
        scale: Vec3::one(),
    };
    let center = BuilderTransform {
        translation: attach
            + rotate_y(
                rotate_x(Vec3::new(0.0, size.y * 0.5, 0.0), rotation_x),
                rotation_y,
            ),
        rotation_x,
        rotation_y,
        scale: Vec3::one(),
    };
    let top = BuilderTransform {
        translation: attach
            + rotate_y(
                rotate_x(Vec3::new(0.0, size.y, 0.0), rotation_x),
                rotation_y,
            ),
        rotation_x,
        rotation_y,
        scale: Vec3::one(),
    };
    ResolvedPart {
        bottom,
        center,
        top,
    }
}

fn resolved_cylinder_anchors(
    attach: Vec3<f32>,
    length: f32,
    rotation_x: f32,
    rotation_y: f32,
) -> ResolvedPart {
    let bottom = BuilderTransform {
        translation: attach,
        rotation_x,
        rotation_y,
        scale: Vec3::one(),
    };
    let center = BuilderTransform {
        translation: attach
            + rotate_y(
                rotate_x(Vec3::new(0.0, length * 0.5, 0.0), rotation_x),
                rotation_y,
            ),
        rotation_x,
        rotation_y,
        scale: Vec3::one(),
    };
    let top = BuilderTransform {
        translation: attach
            + rotate_y(
                rotate_x(Vec3::new(0.0, length, 0.0), rotation_x),
                rotation_y,
            ),
        rotation_x,
        rotation_y,
        scale: Vec3::one(),
    };
    ResolvedPart {
        bottom,
        center,
        top,
    }
}

fn rotate_y(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c - v.z * s, v.y, v.x * s + v.z * c)
}

fn rotate_x(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x, v.y * c - v.z * s, v.y * s + v.z * c)
}

fn sanitize_host_refs(host_refs: u8) -> u8 {
    host_refs.max(1)
}

fn default_preview_host(target: BuilderOutputTarget) -> BuilderPreviewHost {
    match target {
        BuilderOutputTarget::Sector => BuilderPreviewHost {
            width: 1.6,
            depth: 0.9,
            height: 0.8,
            surface: BuilderPreviewSurface::Floor,
        },
        BuilderOutputTarget::VertexPair => BuilderPreviewHost {
            width: 1.0,
            depth: 1.0,
            height: 1.0,
            surface: BuilderPreviewSurface::Floor,
        },
        BuilderOutputTarget::Linedef => BuilderPreviewHost {
            width: 1.0,
            depth: 0.3,
            height: 2.0,
            surface: BuilderPreviewSurface::Wall,
        },
    }
}

fn empty_preview(size: u32) -> BuilderPreview {
    let pixels = vec![0; (size * size * 4) as usize];
    let mut preview = BuilderPreview {
        width: size,
        height: size,
        pixels,
    };
    fill_rect(
        &mut preview,
        0,
        0,
        size as i32,
        size as i32,
        [48, 52, 54, 255],
    );
    draw_checker(&mut preview, 10, [58, 62, 64, 255], [54, 58, 60, 255]);
    preview
}

fn render_assembly_preview(
    size: u32,
    spec: BuilderOutputSpec,
    preview_host: &BuilderPreviewHost,
    assembly: &BuilderAssembly,
) -> BuilderPreview {
    let size = size.max(32);
    let mut preview = empty_preview(size);
    if assembly.primitives.is_empty() {
        return preview;
    }

    let preview_dims = match spec.target {
        BuilderOutputTarget::Sector => Vec3::new(
            preview_host.width.max(0.01),
            preview_host.height.max(0.01),
            preview_host.depth.max(0.01),
        ),
        BuilderOutputTarget::VertexPair => Vec3::new(
            preview_host.width.max(0.01),
            preview_host.height.max(0.01),
            preview_host.depth.max(0.01),
        ),
        BuilderOutputTarget::Linedef => Vec3::new(
            preview_host.width.max(0.01),
            preview_host.height.max(0.01),
            preview_host.depth.max(0.01),
        ),
    };

    #[derive(Clone)]
    struct Face {
        points: [Vec2<f32>; 4],
        depth: f32,
        color: [u8; 4],
        is_top: bool,
    }

    let mut all_points = Vec::new();
    let mut faces = Vec::new();
    let mut shadow_quads = Vec::new();

    for primitive in &assembly.primitives {
        match primitive {
            BuilderPrimitive::Box {
                size,
                transform,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                host_scale_z_normalized,
                ..
            } => {
                let sx = if *host_scale_x_normalized {
                    size.x * transform.scale.x * preview_dims.x
                } else {
                    size.x * transform.scale.x
                };
                let sz = if *host_scale_z_normalized {
                    size.z * transform.scale.z * preview_dims.z
                } else {
                    size.z * transform.scale.z
                };
                let tx = if *host_position_normalized {
                    transform.translation.x * preview_dims.x
                } else {
                    transform.translation.x
                };
                let ty = if *host_position_y_normalized {
                    transform.translation.y * preview_dims.y
                } else {
                    transform.translation.y
                };
                let tz = if *host_position_normalized {
                    transform.translation.z * preview_dims.z
                } else {
                    transform.translation.z
                };
                let sy = if *host_scale_y_normalized {
                    size.y * transform.scale.y * preview_dims.y
                } else {
                    size.y * transform.scale.y
                };

                let hx = sx * 0.5;
                let h = sy;
                let hz = sz * 0.5;
                let local = [
                    Vec3::new(-hx, 0.0, -hz),
                    Vec3::new(hx, 0.0, -hz),
                    Vec3::new(hx, 0.0, hz),
                    Vec3::new(-hx, 0.0, hz),
                    Vec3::new(-hx, h, -hz),
                    Vec3::new(hx, h, -hz),
                    Vec3::new(hx, h, hz),
                    Vec3::new(-hx, h, hz),
                ];
                let world: Vec<Vec3<f32>> = local
                    .iter()
                    .map(|p| rotate_y(*p, transform.rotation_y) + Vec3::new(tx, ty, tz))
                    .collect();
                all_points.extend(world.iter().copied());

                shadow_quads.push([
                    project_u45(Vec3::new(world[0].x, 0.0, world[0].z)),
                    project_u45(Vec3::new(world[1].x, 0.0, world[1].z)),
                    project_u45(Vec3::new(world[2].x, 0.0, world[2].z)),
                    project_u45(Vec3::new(world[3].x, 0.0, world[3].z)),
                ]);

                let candidate_faces = [
                    ([4usize, 5, 6, 7], [165, 136, 104, 255], true),
                    ([0usize, 1, 5, 4], [120, 93, 64, 255], false),
                    ([1usize, 2, 6, 5], [104, 78, 54, 255], false),
                    ([3usize, 2, 6, 7], [132, 102, 72, 255], false),
                    ([0usize, 3, 7, 4], [116, 88, 60, 255], false),
                ];

                for (indices, color, is_top) in candidate_faces {
                    let projected = [
                        project_u45(world[indices[0]]),
                        project_u45(world[indices[1]]),
                        project_u45(world[indices[2]]),
                        project_u45(world[indices[3]]),
                    ];
                    let depth = (world[indices[0]].x
                        + world[indices[0]].z
                        + world[indices[1]].x
                        + world[indices[1]].z
                        + world[indices[2]].x
                        + world[indices[2]].z
                        + world[indices[3]].x
                        + world[indices[3]].z)
                        / 4.0
                        + (world[indices[0]].y
                            + world[indices[1]].y
                            + world[indices[2]].y
                            + world[indices[3]].y)
                            * 0.1;
                    faces.push(Face {
                        points: projected,
                        depth,
                        color,
                        is_top,
                    });
                }
            }
            BuilderPrimitive::Cylinder {
                length,
                radius,
                transform,
                host_position_normalized,
                host_position_y_normalized,
                host_scale_y_normalized,
                host_scale_x_normalized,
                ..
            } => {
                let h = if *host_scale_y_normalized {
                    *length * transform.scale.y * preview_dims.y
                } else {
                    *length * transform.scale.y
                };
                let tx = if *host_position_normalized {
                    transform.translation.x * preview_dims.x
                } else {
                    transform.translation.x
                };
                let ty = if *host_position_y_normalized {
                    transform.translation.y * preview_dims.y
                } else {
                    transform.translation.y
                };
                let tz = if *host_position_normalized {
                    transform.translation.z * preview_dims.z
                } else {
                    transform.translation.z
                };
                let r = if *host_scale_x_normalized {
                    *radius * transform.scale.z * preview_dims.x
                } else {
                    *radius * transform.scale.z
                };

                let local = [
                    Vec3::new(-r, 0.0, -r),
                    Vec3::new(r, 0.0, -r),
                    Vec3::new(r, 0.0, r),
                    Vec3::new(-r, 0.0, r),
                    Vec3::new(-r, h, -r),
                    Vec3::new(r, h, -r),
                    Vec3::new(r, h, r),
                    Vec3::new(-r, h, r),
                ];
                let world: Vec<Vec3<f32>> = local
                    .iter()
                    .map(|p| rotate_y(*p, transform.rotation_y) + Vec3::new(tx, ty, tz))
                    .collect();
                all_points.extend(world.iter().copied());

                shadow_quads.push([
                    project_u45(Vec3::new(world[0].x, 0.0, world[0].z)),
                    project_u45(Vec3::new(world[1].x, 0.0, world[1].z)),
                    project_u45(Vec3::new(world[2].x, 0.0, world[2].z)),
                    project_u45(Vec3::new(world[3].x, 0.0, world[3].z)),
                ]);

                let candidate_faces = [
                    ([4usize, 5, 6, 7], [150, 114, 76, 255], true),
                    ([0usize, 1, 5, 4], [112, 80, 50, 255], false),
                    ([3usize, 2, 6, 7], [124, 92, 60, 255], false),
                ];

                for (indices, color, is_top) in candidate_faces {
                    let projected = [
                        project_u45(world[indices[0]]),
                        project_u45(world[indices[1]]),
                        project_u45(world[indices[2]]),
                        project_u45(world[indices[3]]),
                    ];
                    let depth = (world[indices[0]].x
                        + world[indices[0]].z
                        + world[indices[1]].x
                        + world[indices[1]].z
                        + world[indices[2]].x
                        + world[indices[2]].z
                        + world[indices[3]].x
                        + world[indices[3]].z)
                        / 4.0
                        + (world[indices[0]].y
                            + world[indices[1]].y
                            + world[indices[2]].y
                            + world[indices[3]].y)
                            * 0.1;
                    faces.push(Face {
                        points: projected,
                        depth,
                        color,
                        is_top,
                    });
                }
            }
        }
    }

    if all_points.is_empty() {
        return preview;
    }

    let mut min = project_u45(all_points[0]);
    let mut max = min;
    for p in all_points.into_iter().map(project_u45) {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }

    let extent = Vec2::new((max.x - min.x).max(1e-3), (max.y - min.y).max(1e-3));
    let scale = ((size as f32 - 28.0) / extent.x)
        .min((size as f32 - 28.0) / extent.y)
        .max(1.0);
    let center = Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5);
    let offset = Vec2::new(size as f32 * 0.5, size as f32 * 0.55);

    for quad in &shadow_quads {
        let pts = quad
            .map(|p| (p - center) * scale + offset + Vec2::new(0.0, 6.0))
            .map(|p| Vec2::new(p.x.round() as i32, p.y.round() as i32));
        fill_quad(&mut preview, pts, [24, 22, 20, 88]);
    }

    faces.sort_by(|a, b| {
        a.is_top.cmp(&b.is_top).then_with(|| {
            a.depth
                .partial_cmp(&b.depth)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    for face in &faces {
        let pts = face
            .points
            .map(|p| (p - center) * scale + offset)
            .map(|p| Vec2::new(p.x.round() as i32, p.y.round() as i32));
        fill_quad(&mut preview, pts, face.color);
        draw_poly_outline(&mut preview, &pts, [34, 28, 24, 220]);
    }

    for anchor in &assembly.anchors {
        let p = (project_u45(anchor.transform.translation) - center) * scale + offset;
        let color = match anchor.kind {
            BuilderAttachmentKind::Item => [196, 228, 235, 255],
            BuilderAttachmentKind::Material => [226, 198, 112, 255],
        };
        draw_cross(
            &mut preview,
            p.x.round() as i32,
            p.y.round() as i32,
            2,
            color,
        );
    }

    preview
}

fn project_u45(v: Vec3<f32>) -> Vec2<f32> {
    Vec2::new(v.x - v.y * 0.28, v.z - v.y * 0.84)
}

fn set_pixel(preview: &mut BuilderPreview, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= preview.width as i32 || y >= preview.height as i32 {
        return;
    }
    let idx = ((y as u32 * preview.width + x as u32) * 4) as usize;
    preview.pixels[idx..idx + 4].copy_from_slice(&color);
}

fn fill_rect(preview: &mut BuilderPreview, x: i32, y: i32, w: i32, h: i32, color: [u8; 4]) {
    for yy in y.max(0)..(y + h).min(preview.height as i32) {
        for xx in x.max(0)..(x + w).min(preview.width as i32) {
            set_pixel(preview, xx, yy, color);
        }
    }
}

fn draw_checker(preview: &mut BuilderPreview, cell: i32, a: [u8; 4], b: [u8; 4]) {
    for y in 0..preview.height as i32 {
        for x in 0..preview.width as i32 {
            let color = if ((x / cell) + (y / cell)) % 2 == 0 {
                a
            } else {
                b
            };
            set_pixel(preview, x, y, color);
        }
    }
}

fn fill_triangle(
    preview: &mut BuilderPreview,
    p0: Vec2<i32>,
    p1: Vec2<i32>,
    p2: Vec2<i32>,
    color: [u8; 4],
) {
    let min_x = p0.x.min(p1.x).min(p2.x).max(0);
    let max_x = p0.x.max(p1.x).max(p2.x).min(preview.width as i32 - 1);
    let min_y = p0.y.min(p1.y).min(p2.y).max(0);
    let max_y = p0.y.max(p1.y).max(p2.y).min(preview.height as i32 - 1);

    let edge = |a: Vec2<i32>, b: Vec2<i32>, p: Vec2<i32>| -> i32 {
        (p.x - a.x) * (b.y - a.y) - (p.y - a.y) * (b.x - a.x)
    };

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Vec2::new(x, y);
            let w0 = edge(p1, p2, p);
            let w1 = edge(p2, p0, p);
            let w2 = edge(p0, p1, p);
            if (w0 >= 0 && w1 >= 0 && w2 >= 0) || (w0 <= 0 && w1 <= 0 && w2 <= 0) {
                set_pixel(preview, x, y, color);
            }
        }
    }
}

fn fill_quad(preview: &mut BuilderPreview, pts: [Vec2<i32>; 4], color: [u8; 4]) {
    fill_triangle(preview, pts[0], pts[1], pts[2], color);
    fill_triangle(preview, pts[0], pts[2], pts[3], color);
}

fn draw_line(preview: &mut BuilderPreview, a: Vec2<i32>, b: Vec2<i32>, color: [u8; 4]) {
    let mut x0 = a.x;
    let mut y0 = a.y;
    let x1 = b.x;
    let y1 = b.y;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        set_pixel(preview, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn draw_poly_outline(preview: &mut BuilderPreview, pts: &[Vec2<i32>; 4], color: [u8; 4]) {
    for i in 0..4 {
        draw_line(preview, pts[i], pts[(i + 1) % 4], color);
    }
}

fn draw_cross(preview: &mut BuilderPreview, x: i32, y: i32, r: i32, color: [u8; 4]) {
    for dx in -r..=r {
        set_pixel(preview, x + dx, y, color);
    }
    for dy in -r..=r {
        set_pixel(preview, x, y + dy, color);
    }
}

fn default_graph_name() -> String {
    "Table".to_string()
}

fn default_builder_nodes() -> Vec<BuilderNode> {
    BuilderGraph::preset_table().nodes
}

fn default_translate() -> Vec3<f32> {
    Vec3::zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_host_drives_script_dimensions() {
        let script = BuilderScript::from_text(include_str!("../examples/table.buildergraph"))
            .expect("table script should parse");
        let host = BuilderHost::preview_wall(6.0, 3.0, 0.3);
        let assembly = script
            .evaluate_with_host(&host)
            .expect("surface host should evaluate");

        assert_eq!(assembly.primitives.len(), 5);
        assert!(assembly.warnings.is_empty());

        let BuilderPrimitive::Box { size, .. } = &assembly.primitives[0] else {
            panic!("table top should be a box");
        };
        assert_eq!(*size, Vec3::new(6.0, 0.05, 0.3));
    }

    #[test]
    fn host_and_assembly_json_round_trip() {
        let host = BuilderHost::preview_wall(4.0, 2.5, 0.2);
        let host_json = serde_json::to_string(&host).expect("host should encode");
        let decoded_host: BuilderHost =
            serde_json::from_str(&host_json).expect("host should decode");
        assert_eq!(decoded_host.kind_name(), "surface");

        let assembly = BuilderAssembly {
            warnings: vec![BuilderWarning {
                code: "test".to_string(),
                message: "round trip".to_string(),
            }],
            ..BuilderAssembly::default()
        };
        let assembly_json = serde_json::to_string(&assembly).expect("assembly should encode");
        let decoded_assembly: BuilderAssembly =
            serde_json::from_str(&assembly_json).expect("assembly should decode");
        assert_eq!(decoded_assembly.warnings.len(), 1);
        assert_eq!(decoded_assembly.warnings[0].code, "test");
    }

    #[test]
    fn script_rect_cut_emits_cut_mask() {
        let script = BuilderScript::from_text(
            r#"
name = "Cut Test";
host = sector;

preview {
    width = 4.0;
    depth = 1.0;
    height = 3.0;
}

cut rect {
    min = vec2(-host.width / 2.0, 0.5);
    max = vec2(host.width / 2.0, host.height);
    mode = cut_overlay;
    offset = 0.125;
    inset = 0.25;
    shape = border;
};

output = [];
"#,
        )
        .expect("cut script should parse");

        let assembly = script.evaluate().expect("cut script should evaluate");
        assert_eq!(assembly.cuts.len(), 1);
        assert!(assembly.warnings.is_empty());

        let BuilderCutMask::Rect {
            min,
            max,
            mode,
            offset,
            inset,
            shape,
        } = &assembly.cuts[0]
        else {
            panic!("expected rect cut");
        };
        assert_eq!(*min, Vec2::new(-2.0, 0.5));
        assert_eq!(*max, Vec2::new(2.0, 3.0));
        assert_eq!(*mode, BuilderCutMode::CutOverlay);
        assert_eq!(*offset, 0.125);
        assert_eq!(*inset, 0.25);
        assert_eq!(*shape, BuilderCutShape::Border);
    }

    #[test]
    fn script_rect_detail_emits_surface_detail() {
        let script = BuilderScript::from_text(
            r#"
name = "Detail Test";
host = sector;

detail rect {
    min = vec2(host.width * 0.2, host.depth * 0.2);
    max = vec2(host.width * 0.8, host.depth * 0.8);
    offset = -0.05;
    inset = 0.2;
    shape = border;
    material = TRIM;
    tile_alias = wood;
};

output = [];
"#,
        )
        .expect("detail script should parse");

        let host = BuilderHost::preview_floor(10.0, 5.0);
        let assembly = script
            .evaluate_with_host(&host)
            .expect("detail script should evaluate");

        assert!(assembly.cuts.is_empty());
        assert_eq!(assembly.surface_details.len(), 1);
        assert!(assembly.warnings.is_empty());

        let BuilderSurfaceDetail::Rect {
            min,
            max,
            offset,
            inset,
            shape,
            material_slot,
            tile_alias,
        } = &assembly.surface_details[0]
        else {
            panic!("expected rect detail");
        };

        assert_eq!(*min, Vec2::new(2.0, 1.0));
        assert_eq!(*max, Vec2::new(8.0, 4.0));
        assert_eq!(*offset, -0.05);
        assert_eq!(*inset, 0.2);
        assert_eq!(*shape, BuilderCutShape::Border);
        assert_eq!(material_slot.as_deref(), Some("TRIM"));
        assert_eq!(tile_alias.as_deref(), Some("wood"));
    }

    #[test]
    fn script_column_detail_emits_surface_detail() {
        let script = BuilderScript::from_text(
            r#"
name = "Column Detail Test";
host = sector;

detail column {
    center = vec2(host.width * 0.5, host.depth * 0.1);
    height = host.depth * 0.8;
    radius = 0.125;
    offset = -0.1;
    base = 0.2;
    cap = 0.25;
    transition = 0.08;
    segments = 16;
    material = COLUMN;
    rect_material = BLOCK;
    cyl_material = SHAFT;
    tile_alias = stone;
};

output = [];
"#,
        )
        .expect("column detail script should parse");

        let host = BuilderHost::preview_floor(6.0, 2.5);
        let assembly = script
            .evaluate_with_host(&host)
            .expect("column detail script should evaluate");

        assert_eq!(assembly.surface_details.len(), 1);
        assert!(assembly.warnings.is_empty());

        let BuilderSurfaceDetail::Column {
            center,
            height,
            radius,
            offset,
            base_height,
            cap_height,
            transition_height,
            segments,
            placement,
            cut_footprint,
            material_slot,
            rect_material_slot,
            cyl_material_slot,
            tile_alias,
        } = &assembly.surface_details[0]
        else {
            panic!("expected column detail");
        };

        assert_eq!(*center, Vec2::new(3.0, 0.25));
        assert_eq!(*height, 2.0);
        assert_eq!(*radius, 0.125);
        assert_eq!(*offset, -0.1);
        assert_eq!(*base_height, 0.2);
        assert_eq!(*cap_height, 0.25);
        assert_eq!(*transition_height, 0.08);
        assert_eq!(*segments, 16);
        assert_eq!(*placement, BuilderDetailPlacement::Relief);
        assert!(!*cut_footprint);
        assert_eq!(material_slot.as_deref(), Some("COLUMN"));
        assert_eq!(rect_material_slot.as_deref(), Some("BLOCK"));
        assert_eq!(cyl_material_slot.as_deref(), Some("SHAFT"));
        assert_eq!(tile_alias.as_deref(), Some("stone"));
    }

    #[test]
    fn script_masonry_detail_emits_surface_detail() {
        let script = BuilderScript::from_text(
            r#"
name = "Masonry Detail Test";
host = sector;

detail masonry {
    min = vec2(0.25, 0.5);
    max = vec2(host.width * 0.9375, host.depth * 0.75);
    block = vec2(0.5, 0.25);
    mortar = 0.035;
    offset = -0.04;
    pattern = running_bond;
    material = STONE;
    tile_alias = stone;
};

output = [];
"#,
        )
        .expect("masonry detail script should parse");

        let host = BuilderHost::preview_floor(4.0, 2.0);
        let assembly = script
            .evaluate_with_host(&host)
            .expect("masonry detail script should evaluate");

        assert_eq!(assembly.surface_details.len(), 1);
        assert!(assembly.warnings.is_empty());

        let BuilderSurfaceDetail::Masonry {
            min,
            max,
            block,
            mortar,
            offset,
            pattern,
            material_slot,
            tile_alias,
        } = &assembly.surface_details[0]
        else {
            panic!("expected masonry detail");
        };

        assert_eq!(*min, Vec2::new(0.25, 0.5));
        assert_eq!(*max, Vec2::new(3.75, 1.5));
        assert_eq!(*block, Vec2::new(0.5, 0.25));
        assert_eq!(*mortar, 0.035);
        assert_eq!(*offset, -0.04);
        assert_eq!(*pattern, BuilderMasonryPattern::RunningBond);
        assert_eq!(material_slot.as_deref(), Some("STONE"));
        assert_eq!(tile_alias.as_deref(), Some("stone"));
    }

    #[test]
    fn invalid_rect_cut_reports_warning() {
        let script = BuilderScript::from_text(
            r#"
name = "Bad Cut Test";
host = sector;

cut rect {
    min = vec2(1.0, 1.0);
    max = vec2(0.0, 0.0);
};

output = [];
"#,
        )
        .expect("cut script should parse");

        let assembly = script.evaluate().expect("cut script should evaluate");
        assert!(assembly.cuts.is_empty());
        assert_eq!(assembly.warnings.len(), 1);
        assert_eq!(assembly.warnings[0].code, "invalid_cut_rect");
    }
}
