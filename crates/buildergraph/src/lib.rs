use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

impl Default for BuilderPreviewHost {
    fn default() -> Self {
        Self {
            width: 1.0,
            depth: 1.0,
            height: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderScript {
    pub name: String,
    pub host: BuilderScriptHost,
    pub preview_host: BuilderPreviewHost,
    pub parts: Vec<BuilderScriptPart>,
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
pub struct BuilderScriptPart {
    pub name: String,
    pub kind: BuilderScriptPartKind,
    pub attach: BuilderScriptPointExpr,
    pub parent: Option<BuilderScriptRef>,
    pub material: Option<String>,
    pub axis: Option<BuilderScriptRef>,
    pub rotate_x: Option<BuilderScriptScalarExpr>,
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
    Mul(Box<BuilderScriptScalarExpr>, Box<BuilderScriptScalarExpr>),
    Div(Box<BuilderScriptScalarExpr>, Box<BuilderScriptScalarExpr>),
    Neg(Box<BuilderScriptScalarExpr>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BuilderScriptRef {
    Host(String),
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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuilderAssembly {
    pub primitives: Vec<BuilderPrimitive>,
    pub anchors: Vec<BuilderAnchor>,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct BuilderAnchor {
    pub name: String,
    pub kind: BuilderAttachmentKind,
    pub transform: BuilderTransform,
    pub host_position_normalized: bool,
    pub host_position_y_normalized: bool,
    pub surface_extent: Vec2<f32>,
    pub surface_extent_normalized: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BuilderAttachmentKind {
    Item,
    Material,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BuilderTransform {
    pub translation: Vec3<f32>,
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub scale: Vec3<f32>,
}

fn default_output_host_refs() -> u8 {
    1
}

fn default_corner_inset() -> f32 {
    0.10
}

fn default_grid_count() -> u16 {
    1
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
        let mut primitives = Vec::new();
        let mut anchors = Vec::new();
        let mut resolved_parts: HashMap<String, ResolvedPart> = HashMap::default();

        for part in &self.parts {
            let parent_transform = part
                .parent
                .as_ref()
                .map(|reference| eval_ref_transform(reference, self.host, &dims, &resolved_parts))
                .transpose()?
                .unwrap_or_else(BuilderTransform::identity);
            let local_rotation_x = part
                .rotate_x
                .as_ref()
                .map(|expr| eval_scalar_expr(expr, self.host, &dims))
                .transpose()?
                .unwrap_or(0.0);
            let attach = if part.parent.is_some() {
                let local_attach =
                    eval_point_expr(&part.attach, self.host, &dims, &resolved_parts)?;
                transform_point(&parent_transform, local_attach)
            } else {
                eval_point_expr(&part.attach, self.host, &dims, &resolved_parts)?
            };
            let rotation_x = parent_transform.rotation_x + local_rotation_x;
            let rotation_y = parent_transform.rotation_y;
            let primitive = match &part.kind {
                BuilderScriptPartKind::Box { size } => {
                    let sx = eval_scalar_expr(&size[0], self.host, &dims)?;
                    let sy = eval_scalar_expr(&size[1], self.host, &dims)?;
                    let sz = eval_scalar_expr(&size[2], self.host, &dims)?;
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
                    let sy = eval_scalar_expr(length, self.host, &dims)?;
                    let r = eval_scalar_expr(radius, self.host, &dims)?;
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
            };
            primitives.push(primitive);
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
        out
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
        let mut parts = Vec::new();
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
            } else if self.peek_ident("preview") {
                self.expect_ident("preview")?;
                self.expect_symbol('{')?;
                let mut preview = BuilderPreviewHost::default();
                while !self.consume_symbol('}') {
                    let key = self.expect_ident_any()?;
                    self.expect_symbol('=')?;
                    let value = self.expect_number()?;
                    self.expect_symbol(';')?;
                    match key.as_str() {
                        "width" => preview.width = value,
                        "depth" => preview.depth = value,
                        "height" => preview.height = value,
                        other => return Err(format!("unsupported preview field '{other}'")),
                    }
                }
                preview_host = Some(preview);
            } else if self.peek_ident("let") {
                parts.push(self.parse_part()?);
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
            parts,
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
        })
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
                    BuilderScriptVecExpr::ScaledRef(reference, self.parse_scalar_expr()?)
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
        self.expect_symbol('.')?;
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
) -> Result<Vec3<f32>, String> {
    let mut result = Vec3::zero();
    for (sign, term) in &expr.terms {
        let value = match term {
            BuilderScriptVecExpr::Ref(reference) => eval_ref_point(reference, host, dims, parts)?,
            BuilderScriptVecExpr::ScaledRef(reference, scalar) => {
                eval_ref_vector(reference, host, dims, parts)?
                    * eval_scalar_expr(scalar, host, dims)?
            }
            BuilderScriptVecExpr::Literal(values) => Vec3::new(
                eval_scalar_expr(&values[0], host, dims)?,
                eval_scalar_expr(&values[1], host, dims)?,
                eval_scalar_expr(&values[2], host, dims)?,
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
) -> Result<f32, String> {
    match expr {
        BuilderScriptScalarExpr::Constant(value) => Ok(*value),
        BuilderScriptScalarExpr::Ref(reference) => eval_ref_scalar(reference, host, dims),
        BuilderScriptScalarExpr::Mul(a, b) => {
            Ok(eval_scalar_expr(a, host, dims)? * eval_scalar_expr(b, host, dims)?)
        }
        BuilderScriptScalarExpr::Div(a, b) => {
            Ok(eval_scalar_expr(a, host, dims)? / eval_scalar_expr(b, host, dims)?)
        }
        BuilderScriptScalarExpr::Neg(inner) => Ok(-eval_scalar_expr(inner, host, dims)?),
    }
}

fn eval_ref_scalar(
    reference: &BuilderScriptRef,
    _host: BuilderScriptHost,
    dims: &HostPreviewDims,
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
        },
        BuilderOutputTarget::VertexPair => BuilderPreviewHost {
            width: 1.0,
            depth: 1.0,
            height: 1.0,
        },
        BuilderOutputTarget::Linedef => BuilderPreviewHost {
            width: 1.0,
            depth: 0.3,
            height: 2.0,
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
