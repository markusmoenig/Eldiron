use crate::PixelSource;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vek::Vec2;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OrganicBrushGraph {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "default_graph_name")]
    pub name: String,
    #[serde(default = "default_organic_nodes")]
    pub nodes: Vec<OrganicBrushNode>,
    #[serde(default)]
    pub connections: Vec<(u16, u8, u16, u8)>,
    #[serde(default)]
    pub selected_node: Option<usize>,
    #[serde(default)]
    pub scroll_offset: Vec2<i32>,
}

impl Default for OrganicBrushGraph {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: default_graph_name(),
            nodes: default_organic_nodes(),
            connections: default_organic_connections(),
            selected_node: Some(5),
            scroll_offset: Vec2::zero(),
        }
    }
}

impl OrganicBrushGraph {
    pub fn preset_moss() -> Self {
        Self::from_recipe(
            "Moss",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Blob {
                    radius: 0.24,
                    softness: 0.92,
                },
                noise: Some((0.38, 0.16, 19)),
                scatter_count: 18,
                scatter_jitter: 0.28,
                height_depth: 0.07,
                height_falloff: 0.96,
                material_channel: 0,
                output_radius: 0.68,
                output_flow: 0.96,
                output_jitter: 0.08,
                output_depth: 0.08,
                output_cell_size: 0.10,
            },
        )
    }

    pub fn preset_mud() -> Self {
        Self::from_recipe(
            "Mud",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Blob {
                    radius: 0.72,
                    softness: 0.78,
                },
                noise: Some((0.22, 0.18, 7)),
                scatter_count: 4,
                scatter_jitter: 0.22,
                height_depth: 0.1,
                height_falloff: 0.92,
                material_channel: 1,
                output_radius: 0.95,
                output_flow: 0.95,
                output_jitter: 0.1,
                output_depth: 0.11,
                output_cell_size: 0.18,
            },
        )
    }

    pub fn preset_grass() -> Self {
        Self::from_recipe(
            "Grass",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Blob {
                    radius: 0.2,
                    softness: 0.62,
                },
                noise: Some((0.45, 0.22, 13)),
                scatter_count: 18,
                scatter_jitter: 0.78,
                height_depth: 0.42,
                height_falloff: 0.18,
                material_channel: 0,
                output_radius: 0.58,
                output_flow: 0.82,
                output_jitter: 0.34,
                output_depth: 0.46,
                output_cell_size: 0.12,
            },
        )
    }

    pub fn preset_vines() -> Self {
        Self::from_recipe(
            "Vines",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Line {
                    length: 2.8,
                    width: 0.10,
                    softness: 0.72,
                },
                noise: Some((0.6, 0.3, 23)),
                scatter_count: 1,
                scatter_jitter: 0.0,
                height_depth: 0.18,
                height_falloff: 0.58,
                material_channel: 0,
                output_radius: 0.38,
                output_flow: 0.88,
                output_jitter: 0.03,
                output_depth: 0.18,
                output_cell_size: 0.08,
            },
        )
    }

    pub fn preset_bush() -> Self {
        let mut graph = Self::from_recipe(
            "Bush",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Bush {
                    radius: 0.34,
                    height: 1.15,
                    layers: 5,
                    taper: 0.58,
                    breakup: 0.22,
                    softness: 0.72,
                },
                noise: Some((0.32, 0.18, 31)),
                scatter_count: 1,
                scatter_jitter: 0.0,
                height_depth: 1.0,
                height_falloff: 0.82,
                material_channel: 0,
                output_radius: 0.88,
                output_flow: 1.0,
                output_jitter: 0.06,
                output_depth: 0.92,
                output_cell_size: 0.14,
            },
        );
        for node in &mut graph.nodes {
            if let OrganicNodeKind::PaletteRange { count, mode, .. } = &mut node.kind {
                *count = 3;
                *mode = 2;
            }
        }
        graph
    }

    fn from_recipe(name: &str, recipe: OrganicBrushRecipe) -> Self {
        let mut nodes = vec![OrganicBrushNode::new(
            match recipe.shape {
                OrganicBrushShape::Blob { radius, softness } => {
                    OrganicNodeKind::CircleMask { radius, softness }
                }
                OrganicBrushShape::Bush {
                    radius,
                    height,
                    layers,
                    taper,
                    breakup,
                    softness,
                } => OrganicNodeKind::BushShape {
                    radius,
                    height,
                    layers,
                    taper,
                    breakup,
                    softness,
                },
                OrganicBrushShape::Line {
                    length,
                    width,
                    softness,
                } => OrganicNodeKind::LineShape {
                    length,
                    width,
                    softness,
                },
            },
            Vec2::new(200, 36),
        )];
        let mut connections = Vec::new();

        let mut shape_source_index = 0u16;
        if let Some((scale, strength, seed)) = recipe.noise {
            nodes.push(OrganicBrushNode::new(
                OrganicNodeKind::Noise {
                    scale,
                    strength,
                    seed,
                },
                Vec2::new(410, 28),
            ));
            connections.push((0, 0, 1, 0));
            shape_source_index = 1;
        }

        let scatter_index = nodes.len() as u16;
        nodes.push(OrganicBrushNode::new(
            OrganicNodeKind::Scatter {
                count: recipe.scatter_count,
                jitter: recipe.scatter_jitter,
            },
            Vec2::new(620, 36),
        ));
        connections.push((shape_source_index, 0, scatter_index, 0));

        let palette_index = nodes.len() as u16;
        nodes.push(OrganicBrushNode::new(
            OrganicNodeKind::PaletteRange {
                start: 0,
                count: 1,
                mode: 0,
            },
            Vec2::new(420, 330),
        ));
        let material_index = nodes.len() as u16;
        nodes.push(OrganicBrushNode::new(
            OrganicNodeKind::Material {
                channel: recipe.material_channel,
            },
            Vec2::new(640, 330),
        ));
        connections.push((palette_index, 0, material_index, 0));

        let growth_index = nodes.len() as u16;
        nodes.push(OrganicBrushNode::new(
            OrganicNodeKind::HeightProfile {
                depth: recipe.height_depth,
                falloff: recipe.height_falloff,
            },
            Vec2::new(620, 185),
        ));

        let output_index = nodes.len() as u16;
        nodes.push(OrganicBrushNode::new(
            OrganicNodeKind::OutputVolume {
                radius: recipe.output_radius,
                flow: recipe.output_flow,
                jitter: recipe.output_jitter,
                depth: recipe.output_depth,
                cell_size: recipe.output_cell_size,
            },
            Vec2::new(900, 108),
        ));

        connections.push((scatter_index, 0, output_index, 0));
        connections.push((material_index, 0, output_index, 1));
        connections.push((growth_index, 0, output_index, 2));

        Self {
            id: Uuid::new_v4(),
            name: format!("{} Brush", name),
            nodes,
            connections,
            selected_node: Some(output_index as usize),
            scroll_offset: Vec2::zero(),
        }
    }
}

struct OrganicBrushRecipe {
    shape: OrganicBrushShape,
    noise: Option<(f32, f32, i32)>,
    scatter_count: i32,
    scatter_jitter: f32,
    height_depth: f32,
    height_falloff: f32,
    material_channel: i32,
    output_radius: f32,
    output_flow: f32,
    output_jitter: f32,
    output_depth: f32,
    output_cell_size: f32,
}

enum OrganicBrushShape {
    Blob {
        radius: f32,
        softness: f32,
    },
    Bush {
        radius: f32,
        height: f32,
        layers: i32,
        taper: f32,
        breakup: f32,
        softness: f32,
    },
    Line {
        length: f32,
        width: f32,
        softness: f32,
    },
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OrganicBrushNode {
    pub kind: OrganicNodeKind,
    pub position: Vec2<i32>,
}

impl OrganicBrushNode {
    pub fn new(kind: OrganicNodeKind, position: Vec2<i32>) -> Self {
        Self { kind, position }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum OrganicNodeKind {
    SurfaceInput,
    CircleMask {
        radius: f32,
        softness: f32,
    },
    CanopyShape {
        radius: f32,
        lobes: i32,
        spread: f32,
        softness: f32,
    },
    BushShape {
        radius: f32,
        height: f32,
        layers: i32,
        taper: f32,
        breakup: f32,
        softness: f32,
    },
    LineShape {
        length: f32,
        width: f32,
        softness: f32,
    },
    Noise {
        scale: f32,
        strength: f32,
        seed: i32,
    },
    Scatter {
        count: i32,
        jitter: f32,
    },
    HeightProfile {
        depth: f32,
        falloff: f32,
    },
    PaletteRange {
        start: i32,
        count: i32,
        mode: i32,
    },
    Material {
        channel: i32,
    },
    OutputVolume {
        radius: f32,
        flow: f32,
        jitter: f32,
        depth: f32,
        cell_size: f32,
    },
}

fn default_graph_name() -> String {
    "Default Organic Brush".to_string()
}

fn default_organic_nodes() -> Vec<OrganicBrushNode> {
    vec![
        OrganicBrushNode::new(
            OrganicNodeKind::CircleMask {
                radius: 0.55,
                softness: 0.35,
            },
            Vec2::new(200, 36),
        ),
        OrganicBrushNode::new(
            OrganicNodeKind::Scatter {
                count: 5,
                jitter: 0.35,
            },
            Vec2::new(420, 28),
        ),
        OrganicBrushNode::new(
            OrganicNodeKind::HeightProfile {
                depth: 0.4,
                falloff: 0.5,
            },
            Vec2::new(420, 188),
        ),
        OrganicBrushNode::new(
            OrganicNodeKind::PaletteRange {
                start: 0,
                count: 1,
                mode: 0,
            },
            Vec2::new(260, 340),
        ),
        OrganicBrushNode::new(
            OrganicNodeKind::Material { channel: 0 },
            Vec2::new(500, 340),
        ),
        OrganicBrushNode::new(
            OrganicNodeKind::OutputVolume {
                radius: 0.6,
                flow: 1.0,
                jitter: 0.15,
                depth: 0.45,
                cell_size: 0.25,
            },
            Vec2::new(780, 108),
        ),
    ]
}

fn default_organic_connections() -> Vec<(u16, u8, u16, u8)> {
    vec![
        (0, 0, 1, 0),
        (3, 0, 4, 0),
        (1, 0, 5, 0),
        (4, 0, 5, 1),
        (2, 0, 5, 2),
    ]
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicChannelBinding {
    pub channel: i32,
    pub name: String,
    pub source: Option<PixelSource>,
    pub roughness: f32,
    pub metallic: f32,
    pub opacity: f32,
    pub emissive: f32,
}

impl OrganicChannelBinding {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                channel: 0,
                name: "Foliage".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 1,
                name: "Soil".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 2,
                name: "Stone".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
            Self {
                channel: 3,
                name: "Accent".to_string(),
                source: None,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            },
        ]
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicSpan {
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub offset: f32,
    pub height: f32,
    pub density: f32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicColumn {
    pub x: i32,
    pub y: i32,
    pub spans: Vec<OrganicSpan>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicVineStroke {
    pub stroke_id: i32,
    pub seq: i32,
    pub start: Vec2<f32>,
    pub end: Vec2<f32>,
    pub anchor_offset: f32,
    pub width: f32,
    pub depth: f32,
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub grow_positive: bool,
    pub cap_start: bool,
    pub cap_end: bool,
}

pub fn default_organic_vine_strokes() -> Vec<OrganicVineStroke> {
    Vec::new()
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicBushCluster {
    pub center: Vec2<f32>,
    pub anchor_offset: f32,
    pub radius: f32,
    pub height: f32,
    pub layers: i32,
    pub taper: f32,
    pub breakup: f32,
    pub channel: i32,
    pub source: Option<PixelSource>,
    pub grow_positive: bool,
}

pub fn default_organic_bush_clusters() -> Vec<OrganicBushCluster> {
    Vec::new()
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct OrganicVolumeLayer {
    pub id: Uuid,
    pub name: String,
    pub cell_size: f32,
    pub columns: Vec<OrganicColumn>,
    pub channel_bindings: Vec<OrganicChannelBinding>,
}

impl Default for OrganicVolumeLayer {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "Main Organic Layer".to_string(),
            cell_size: 0.25,
            columns: Vec::new(),
            channel_bindings: OrganicChannelBinding::defaults(),
        }
    }
}

impl OrganicVolumeLayer {
    pub fn set_channel_source(&mut self, channel: i32, source: Option<PixelSource>) {
        if let Some(binding) = self
            .channel_bindings
            .iter_mut()
            .find(|binding| binding.channel == channel)
        {
            binding.source = source;
        } else {
            self.channel_bindings.push(OrganicChannelBinding {
                channel,
                name: format!("Channel {}", channel),
                source,
                roughness: 0.5,
                metallic: 0.0,
                opacity: 1.0,
                emissive: 0.0,
            });
        }
    }

    pub fn set_channel_material(
        &mut self,
        channel: i32,
        roughness: f32,
        metallic: f32,
        opacity: f32,
        emissive: f32,
    ) {
        if let Some(binding) = self
            .channel_bindings
            .iter_mut()
            .find(|binding| binding.channel == channel)
        {
            binding.roughness = roughness.clamp(0.0, 1.0);
            binding.metallic = metallic.clamp(0.0, 1.0);
            binding.opacity = opacity.clamp(0.0, 1.0);
            binding.emissive = emissive.clamp(0.0, 1.0);
        } else {
            self.channel_bindings.push(OrganicChannelBinding {
                channel,
                name: format!("Channel {}", channel),
                source: None,
                roughness: roughness.clamp(0.0, 1.0),
                metallic: metallic.clamp(0.0, 1.0),
                opacity: opacity.clamp(0.0, 1.0),
                emissive: emissive.clamp(0.0, 1.0),
            });
        }
    }

    pub fn source_for_channel(&self, channel: i32) -> Option<&PixelSource> {
        self.channel_bindings
            .iter()
            .find(|binding| binding.channel == channel)
            .and_then(|binding| binding.source.as_ref())
    }

    pub fn binding_for_channel(&self, channel: i32) -> Option<&OrganicChannelBinding> {
        self.channel_bindings
            .iter()
            .find(|binding| binding.channel == channel)
    }

    pub fn paint_sphere(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((center.x - radius) / cell_size).floor() as i32;
        let max_x = ((center.x + radius) / cell_size).ceil() as i32;
        let min_y = ((center.y - radius) / cell_size).floor() as i32;
        let max_y = ((center.y + radius) / cell_size).ceil() as i32;

        let mut changed = false;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let delta = cell_center - center;
                let dist = delta.magnitude();
                if dist > radius {
                    continue;
                }

                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                if height <= 0.0 {
                    continue;
                }

                let offset = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };

                if self.paint_column_span(
                    x,
                    y,
                    channel,
                    source.clone(),
                    offset,
                    height,
                    density * edge,
                ) {
                    changed = true;
                }
            }
        }

        changed
    }

    pub fn paint_capsule(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((start.x.min(end.x) - radius) / cell_size).floor() as i32;
        let max_x = ((start.x.max(end.x) + radius) / cell_size).ceil() as i32;
        let min_y = ((start.y.min(end.y) - radius) / cell_size).floor() as i32;
        let max_y = ((start.y.max(end.y) + radius) / cell_size).ceil() as i32;
        let segment = end - start;
        let segment_len_sq = segment.magnitude_squared();

        let mut changed = false;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let t = if segment_len_sq <= f32::EPSILON {
                    0.0
                } else {
                    ((cell_center - start).dot(segment) / segment_len_sq).clamp(0.0, 1.0)
                };
                let closest = start + segment * t;
                let dist = (cell_center - closest).magnitude();
                if dist > radius {
                    continue;
                }

                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                if height <= 0.0 {
                    continue;
                }

                let offset = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };

                if self.paint_column_span(
                    x,
                    y,
                    channel,
                    source.clone(),
                    offset,
                    height,
                    density * edge,
                ) {
                    changed = true;
                }
            }
        }

        changed
    }

    pub fn erase_sphere(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((center.x - radius) / cell_size).floor() as i32;
        let max_x = ((center.x + radius) / cell_size).ceil() as i32;
        let min_y = ((center.y - radius) / cell_size).floor() as i32;
        let max_y = ((center.y + radius) / cell_size).ceil() as i32;

        let mut changed = false;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let delta = cell_center - center;
                let dist = delta.magnitude();
                if dist > radius {
                    continue;
                }
                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                if edge <= 0.0 {
                    continue;
                }
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                let start = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };
                let end = start + height;
                if self.erase_column_range(x, y, start, end) {
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn erase_capsule(
        &mut self,
        start: Vec2<f32>,
        end: Vec2<f32>,
        radius: f32,
        anchor_offset: f32,
        max_height: f32,
        edge_softness: f32,
        height_falloff: f32,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 0.5);
        let min_x = ((start.x.min(end.x) - radius) / cell_size).floor() as i32;
        let max_x = ((start.x.max(end.x) + radius) / cell_size).ceil() as i32;
        let min_y = ((start.y.min(end.y) - radius) / cell_size).floor() as i32;
        let max_y = ((start.y.max(end.y) + radius) / cell_size).ceil() as i32;
        let segment = end - start;
        let segment_len_sq = segment.magnitude_squared();

        let mut changed = false;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let t = if segment_len_sq <= f32::EPSILON {
                    0.0
                } else {
                    ((cell_center - start).dot(segment) / segment_len_sq).clamp(0.0, 1.0)
                };
                let closest = start + segment * t;
                let dist = (cell_center - closest).magnitude();
                if dist > radius {
                    continue;
                }
                let radial = 1.0 - (dist / radius).clamp(0.0, 1.0);
                let softness = edge_softness.clamp(0.0, 0.999);
                let edge = if softness <= 0.001 {
                    if radial > 0.0 { 1.0 } else { 0.0 }
                } else {
                    let start = 1.0 - softness;
                    if radial >= start {
                        1.0
                    } else {
                        (radial / start.max(0.001)).clamp(0.0, 1.0)
                    }
                };
                if edge <= 0.0 {
                    continue;
                }
                let falloff = radial.powf((1.0 - height_falloff.clamp(0.0, 1.0)) * 2.0 + 0.5);
                let height = (max_height * falloff).max(cell_size * 0.20);
                let start = if grow_positive {
                    anchor_offset
                } else {
                    anchor_offset - height
                };
                let end = start + height;
                if self.erase_column_range(x, y, start, end) {
                    changed = true;
                }
            }
        }
        changed
    }

    pub fn paint_bush_cluster(
        &mut self,
        center: Vec2<f32>,
        radius: f32,
        total_height: f32,
        anchor_offset: f32,
        layers: i32,
        taper: f32,
        breakup: f32,
        edge_softness: f32,
        density: f32,
        channel: i32,
        source: Option<PixelSource>,
        grow_positive: bool,
    ) -> bool {
        let cell_size = self.cell_size.max(0.01);
        let radius = radius.max(cell_size * 1.5);
        let total_height = total_height.max(cell_size * 1.5);
        let layer_count = layers.max(2) as usize;
        let min_x = ((center.x - radius) / cell_size).floor() as i32;
        let max_x = ((center.x + radius) / cell_size).ceil() as i32;
        let min_y = ((center.y - radius) / cell_size).floor() as i32;
        let max_y = ((center.y + radius) / cell_size).ceil() as i32;
        let slice_height = (total_height / layer_count as f32).max(cell_size * 0.6);

        let mut changed = false;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let cell_center =
                    Vec2::new((x as f32 + 0.5) * cell_size, (y as f32 + 0.5) * cell_size);
                let to_cell = cell_center - center;
                let dist = to_cell.magnitude();
                if dist > radius {
                    continue;
                }

                // Stable per-cell breakup so the bush silhouette is chunky instead of conical.
                let hash = (((x * 73_856_093) ^ (y * 19_349_663)) & 1023) as f32 / 1023.0;
                let noise = (hash - 0.5) * 2.0;

                for layer_index in 0..layer_count {
                    let t = if layer_count <= 1 {
                        0.0
                    } else {
                        layer_index as f32 / (layer_count - 1) as f32
                    };
                    let layer_radius =
                        radius * (1.0 - t * taper.clamp(0.0, 1.0) * 0.72 + noise * breakup * 0.12);
                    if dist > layer_radius.max(cell_size * 0.6) {
                        continue;
                    }
                    let radial = 1.0 - (dist / layer_radius.max(cell_size * 0.6)).clamp(0.0, 1.0);
                    let softness = edge_softness.clamp(0.0, 0.999);
                    let edge = if softness <= 0.001 {
                        if radial > 0.0 { 1.0 } else { 0.0 }
                    } else {
                        let start = 1.0 - softness;
                        if radial >= start {
                            1.0
                        } else {
                            (radial / start.max(0.001)).clamp(0.0, 1.0)
                        }
                    };
                    if edge <= 0.0 {
                        continue;
                    }

                    let vertical = total_height * t * 0.72;
                    let span_height = (slice_height * (1.05 - t * 0.18 + noise * breakup * 0.08))
                        .max(cell_size * 0.45);
                    let offset = if grow_positive {
                        anchor_offset + vertical
                    } else {
                        anchor_offset - vertical - span_height
                    };
                    if self.paint_column_span(
                        x,
                        y,
                        channel,
                        source.clone(),
                        offset,
                        span_height,
                        density * edge,
                    ) {
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    fn paint_column_span(
        &mut self,
        x: i32,
        y: i32,
        channel: i32,
        source: Option<PixelSource>,
        offset: f32,
        height: f32,
        density: f32,
    ) -> bool {
        let Some(column_index) = self
            .columns
            .iter()
            .position(|column| column.x == x && column.y == y)
        else {
            self.columns.push(OrganicColumn {
                x,
                y,
                spans: vec![OrganicSpan {
                    channel,
                    source,
                    offset,
                    height,
                    density,
                }],
            });
            return true;
        };

        let start = offset;
        let end = offset + height;
        let column = &mut self.columns[column_index];

        for span in &mut column.spans {
            if span.channel != channel || span.source != source {
                continue;
            }
            let span_start = span.offset;
            let span_end = span.offset + span.height;
            if end < span_start || start > span_end {
                continue;
            }
            let merged_start = span_start.min(start);
            let merged_end = span_end.max(end);
            let merged_density = span.density.max(density);
            if (span.offset - merged_start).abs() > f32::EPSILON
                || (span.height - (merged_end - merged_start)).abs() > f32::EPSILON
                || (span.density - merged_density).abs() > f32::EPSILON
            {
                span.offset = merged_start;
                span.height = merged_end - merged_start;
                span.density = merged_density;
                return true;
            }
            return false;
        }

        column.spans.push(OrganicSpan {
            channel,
            source,
            offset,
            height,
            density,
        });
        true
    }

    fn erase_column_range(&mut self, x: i32, y: i32, start: f32, end: f32) -> bool {
        let Some(column_index) = self
            .columns
            .iter()
            .position(|column| column.x == x && column.y == y)
        else {
            return false;
        };

        let mut changed = false;
        let column = &mut self.columns[column_index];
        let mut new_spans = Vec::with_capacity(column.spans.len());

        for span in &column.spans {
            let span_start = span.offset;
            let span_end = span.offset + span.height;
            if end <= span_start || start >= span_end {
                new_spans.push(span.clone());
                continue;
            }
            changed = true;
            if start > span_start {
                new_spans.push(OrganicSpan {
                    channel: span.channel,
                    source: span.source.clone(),
                    offset: span_start,
                    height: start - span_start,
                    density: span.density,
                });
            }
            if end < span_end {
                new_spans.push(OrganicSpan {
                    channel: span.channel,
                    source: span.source.clone(),
                    offset: end,
                    height: span_end - end,
                    density: span.density,
                });
            }
        }

        column.spans = new_spans;
        if column.spans.is_empty() {
            self.columns.remove(column_index);
        }
        changed
    }
}

pub fn default_organic_layers() -> IndexMap<Uuid, OrganicVolumeLayer> {
    IndexMap::default()
}
