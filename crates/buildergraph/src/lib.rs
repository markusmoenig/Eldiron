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
        let rotated = rotate_y(scaled, self.rotation_y);
        Self {
            translation: self.translation + rotated,
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
                        rotation_y: transform.rotation_y,
                        scale: transform.scale,
                    },
                    material_slot: material_slot.clone(),
                    host_position_normalized: true,
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
                    rotation_y: anchor.transform.rotation_y,
                    scale: anchor.transform.scale,
                },
                host_position_normalized: true,
                surface_extent: anchor.surface_extent,
                surface_extent_normalized: anchor.surface_extent_normalized,
            })
            .collect();

        let placements = self
            .placements
            .iter()
            .map(|placement| BuilderPlacement {
                translation: placement.translation + translation,
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
                host_scale_x_normalized: host_scale_x_a,
                host_scale_z_normalized: host_scale_z_a,
                ..
            },
            BuilderPrimitive::Box {
                size: size_b,
                transform: transform_b,
                host_position_normalized: host_pos_b,
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
                && host_scale_x_a == host_scale_x_b
                && host_scale_z_a == host_scale_z_b
        }
    }
}

fn primitive_has_material_slot(primitive: &BuilderPrimitive) -> bool {
    match primitive {
        BuilderPrimitive::Box { material_slot, .. } => material_slot.is_some(),
    }
}

impl Default for BuilderGraph {
    fn default() -> Self {
        Self::preset_table()
    }
}

impl BuilderGraph {
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
        }
    }

    pub fn from_text(source: &str) -> Result<Self, String> {
        toml::from_str(source)
            .or_else(|_| serde_json::from_str(source))
            .map_err(|err| err.to_string())
    }

    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|err| err.to_string())
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
        let size = size.max(32);
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

        let Ok(assembly) = self.evaluate() else {
            return preview;
        };
        if assembly.primitives.is_empty() {
            return preview;
        }
        let preview_host = preview_host_dimensions(self.output_spec().target);

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
                    host_scale_x_normalized,
                    host_scale_z_normalized,
                    ..
                } => {
                    let sx = if *host_scale_x_normalized {
                        size.x * transform.scale.x * preview_host.x
                    } else {
                        size.x * transform.scale.x
                    };
                    let sz = if *host_scale_z_normalized {
                        size.z * transform.scale.z * preview_host.y
                    } else {
                        size.z * transform.scale.z
                    };
                    let tx = if *host_position_normalized {
                        transform.translation.x * preview_host.x
                    } else {
                        transform.translation.x
                    };
                    let tz = if *host_position_normalized {
                        transform.translation.z * preview_host.y
                    } else {
                        transform.translation.z
                    };

                    let hx = sx * 0.5;
                    let h = size.y * transform.scale.y;
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
                        .map(|p| {
                            rotate_y(*p, transform.rotation_y)
                                + Vec3::new(tx, transform.translation.y, tz)
                        })
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
            let anchor_world = Vec3::new(
                anchor.transform.translation.x * preview_host.x,
                anchor.transform.translation.y,
                anchor.transform.translation.z * preview_host.y,
            );
            let p = (project_u45(anchor_world) - center) * scale + offset;
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
                    host_scale_x_normalized: true,
                    host_scale_z_normalized: true,
                }],
            },
            BuilderNodeKind::LinedefSurface => NodeOutput {
                primitives: Vec::new(),
                anchors: Vec::new(),
                placements: vec![BuilderPlacement {
                    translation: Vec3::zero(),
                    host_scale_x_normalized: true,
                    host_scale_z_normalized: false,
                }],
            },
            BuilderNodeKind::VertexPoint => NodeOutput {
                primitives: Vec::new(),
                anchors: Vec::new(),
                placements: vec![BuilderPlacement {
                    translation: Vec3::zero(),
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
                                    rotation_y: 0.0,
                                    scale: Vec3::one(),
                                },
                                material_slot: None,
                                host_position_normalized: true,
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
                                rotation_y: transform.rotation_y,
                                scale: Vec3::new(
                                    transform.scale.x * span_scale,
                                    transform.scale.y,
                                    transform.scale.z,
                                ),
                            },
                            material_slot,
                            host_position_normalized: true,
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
                            rotation_y: anchor.transform.rotation_y,
                            scale: anchor.transform.scale,
                        },
                        host_position_normalized: anchor.host_position_normalized,
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
                            ..
                        } => {
                            let h = size.y * transform.scale.y;
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Item,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                surface_extent: Vec2::zero(),
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
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
                                surface_extent: Vec2::new(
                                    size.x * transform.scale.x,
                                    size.z * transform.scale.z,
                                ),
                                surface_extent_normalized: host_scale_x_normalized
                                    || host_scale_z_normalized,
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
                                host_scale_x_normalized,
                                host_scale_z_normalized,
                            });
                            out.anchors.push(BuilderAnchor {
                                name: name.clone(),
                                kind: BuilderAttachmentKind::Material,
                                transform: BuilderTransform {
                                    translation: transform.translation + Vec3::new(0.0, h, 0.0),
                                    rotation_y: transform.rotation_y,
                                    scale: Vec3::one(),
                                },
                                host_position_normalized,
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
                        rotation_y: 0.0,
                        scale: Vec3::one(),
                    },
                    host_position_normalized: false,
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
                        rotation_y: 0.0,
                        scale: Vec3::one(),
                    },
                    host_position_normalized: false,
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

fn rotate_y(v: Vec3<f32>, angle: f32) -> Vec3<f32> {
    let (s, c) = angle.sin_cos();
    Vec3::new(v.x * c - v.z * s, v.y, v.x * s + v.z * c)
}

fn sanitize_host_refs(host_refs: u8) -> u8 {
    host_refs.max(1)
}

fn preview_host_dimensions(target: BuilderOutputTarget) -> Vec2<f32> {
    match target {
        BuilderOutputTarget::Sector => Vec2::new(1.6, 0.9),
        BuilderOutputTarget::VertexPair => Vec2::new(1.4, 0.7),
        BuilderOutputTarget::Linedef => Vec2::new(2.0, 0.6),
    }
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
