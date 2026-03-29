use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use vek::Vec2;

#[derive(Clone, Debug)]
pub struct OrganicPreview {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OrganicBrushOutputKind {
    Paint,
    Growth,
    Path,
}

#[derive(Clone, Copy, Debug)]
pub struct OrganicOutputParams {
    pub radius: f32,
    pub flow: f32,
    pub jitter: f32,
    pub depth: f32,
    pub cell_size: f32,
    pub channel: i32,
    #[allow(dead_code)]
    pub palette_start: i32,
    #[allow(dead_code)]
    pub palette_count: i32,
    #[allow(dead_code)]
    pub palette_mode: i32,
}

fn default_output_channel() -> i32 {
    0
}

fn default_output_palette_start() -> i32 {
    0
}

fn default_output_palette_count() -> i32 {
    1
}

fn default_output_palette_mode() -> i32 {
    0
}

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
        Self::default_paint()
    }
}

impl OrganicBrushGraph {
    pub fn default_paint() -> Self {
        let mut graph = Self::preset_moss();
        graph.name = default_graph_name();
        graph
    }

    pub fn preset_moss() -> Self {
        Self::from_recipe(
            "Moss",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Blob {
                    radius: 0.46,
                    softness: 0.78,
                },
                noise: Some((0.42, 0.30, 19)),
                scatter_count: 14,
                scatter_jitter: 0.28,
                height_depth: 0.36,
                height_falloff: 0.78,
                material_channel: 0,
                palette_colors: [23, 22, 21, 20],
                palette_mode: 0,
                output_radius: 0.82,
                output_flow: 1.0,
                output_jitter: 0.12,
                output_depth: 0.22,
                output_cell_size: 0.10,
                output_kind: OrganicBrushOutputKind::Paint,
            },
        )
    }

    pub fn preset_mud() -> Self {
        Self::from_recipe(
            "Mud",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Blob {
                    radius: 0.78,
                    softness: 0.68,
                },
                noise: Some((0.22, 0.26, 7)),
                scatter_count: 5,
                scatter_jitter: 0.12,
                height_depth: 0.26,
                height_falloff: 0.82,
                material_channel: 1,
                palette_colors: [16, 17, 18, 19],
                palette_mode: 0,
                output_radius: 1.05,
                output_flow: 0.95,
                output_jitter: 0.08,
                output_depth: 0.24,
                output_cell_size: 0.18,
                output_kind: OrganicBrushOutputKind::Paint,
            },
        )
    }

    pub fn preset_grass() -> Self {
        Self::from_recipe(
            "Grass",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Blob {
                    radius: 0.28,
                    softness: 0.42,
                },
                noise: Some((0.58, 0.34, 13)),
                scatter_count: 22,
                scatter_jitter: 0.78,
                height_depth: 0.54,
                height_falloff: 0.18,
                material_channel: 0,
                palette_colors: [23, 22, 21, 20],
                palette_mode: 0,
                output_radius: 0.72,
                output_flow: 0.90,
                output_jitter: 0.28,
                output_depth: 0.58,
                output_cell_size: 0.11,
                output_kind: OrganicBrushOutputKind::Paint,
            },
        )
    }

    pub fn preset_bush() -> Self {
        Self::from_recipe(
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
                palette_colors: [23, 22, 21, 20],
                palette_mode: 0,
                output_radius: 0.88,
                output_flow: 1.0,
                output_jitter: 0.06,
                output_depth: 0.92,
                output_cell_size: 0.14,
                output_kind: OrganicBrushOutputKind::Growth,
            },
        )
    }

    pub fn preset_tree() -> Self {
        Self::from_recipe(
            "Tree",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Canopy {
                    radius: 0.54,
                    lobes: 5,
                    spread: 0.48,
                    softness: 0.70,
                },
                noise: Some((0.28, 0.14, 37)),
                scatter_count: 1,
                scatter_jitter: 0.0,
                height_depth: 1.25,
                height_falloff: 0.76,
                material_channel: 0,
                palette_colors: [23, 21, 20, 25],
                palette_mode: 0,
                output_radius: 0.74,
                output_flow: 1.0,
                output_jitter: 0.04,
                output_depth: 1.30,
                output_cell_size: 0.14,
                output_kind: OrganicBrushOutputKind::Growth,
            },
        )
    }

    pub fn preset_path_vines() -> Self {
        Self::from_recipe(
            "Vines",
            OrganicBrushRecipe {
                shape: OrganicBrushShape::Line {
                    length: 2.8,
                    width: 0.18,
                    softness: 0.34,
                },
                noise: Some((0.48, 0.18, 23)),
                scatter_count: 1,
                scatter_jitter: 0.0,
                height_depth: 0.36,
                height_falloff: 0.42,
                material_channel: 0,
                palette_colors: [23, 22, 21, 24],
                palette_mode: 0,
                output_radius: 0.58,
                output_flow: 0.90,
                output_jitter: 0.08,
                output_depth: 0.18,
                output_cell_size: 0.10,
                output_kind: OrganicBrushOutputKind::Path,
            },
        )
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
                OrganicBrushShape::Canopy {
                    radius,
                    lobes,
                    spread,
                    softness,
                } => OrganicNodeKind::CanopyShape {
                    radius,
                    lobes,
                    spread,
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

        let growth_index = nodes.len() as u16;
        nodes.push(OrganicBrushNode::new(
            OrganicNodeKind::HeightProfile {
                depth: recipe.height_depth,
                falloff: recipe.height_falloff,
            },
            Vec2::new(620, 300),
        ));

        let palette_index = nodes.len() as u16;
        nodes.push(OrganicBrushNode::new(
            OrganicNodeKind::PaletteColors {
                color_1: recipe.palette_colors[0],
                color_2: recipe.palette_colors[1],
                color_3: recipe.palette_colors[2],
                color_4: recipe.palette_colors[3],
                mode: recipe.palette_mode,
            },
            Vec2::new(620, 185),
        ));

        let output_index = nodes.len() as u16;
        let output_kind = match recipe.output_kind {
            OrganicBrushOutputKind::Paint => OrganicNodeKind::OutputPaint {
                radius: recipe.output_radius,
                flow: recipe.output_flow,
                jitter: recipe.output_jitter,
                depth: recipe.output_depth,
                cell_size: recipe.output_cell_size,
                channel: recipe.material_channel,
                palette_start: 0,
                palette_count: 1,
                palette_mode: 0,
            },
            OrganicBrushOutputKind::Growth => OrganicNodeKind::OutputGrowth {
                radius: recipe.output_radius,
                flow: recipe.output_flow,
                jitter: recipe.output_jitter,
                depth: recipe.output_depth,
                cell_size: recipe.output_cell_size,
                channel: recipe.material_channel,
                palette_start: 0,
                palette_count: 1,
                palette_mode: 0,
            },
            OrganicBrushOutputKind::Path => OrganicNodeKind::OutputPath {
                radius: recipe.output_radius,
                flow: recipe.output_flow,
                jitter: recipe.output_jitter,
                depth: recipe.output_depth,
                cell_size: recipe.output_cell_size,
                channel: recipe.material_channel,
                palette_start: 0,
                palette_count: 1,
                palette_mode: 0,
            },
        };
        nodes.push(OrganicBrushNode::new(output_kind, Vec2::new(900, 108)));

        connections.push((scatter_index, 0, output_index, 0));
        connections.push((palette_index, 0, output_index, 1));
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

    pub fn from_text(text: &str) -> Result<Self, String> {
        let trimmed = text.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            serde_json::from_str(text).map_err(|e| e.to_string())
        } else {
            toml::from_str(text)
                .or_else(|_| serde_json::from_str(text))
                .map_err(|e| e.to_string())
        }
    }

    pub fn to_json_string(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn render_preview(&self, size: u32) -> OrganicPreview {
        let size = size.max(32) as usize;
        let mut pixels = vec![0u8; size * size * 4];
        paint_preview_background(&mut pixels, size);

        let state = self.preview_state();
        let radius = state.radius;
        let flow = state.flow;
        let jitter = state.jitter;
        let output_kind = state.output_kind;
        let circle_radius = state.circle_radius;
        let circle_softness = state.circle_softness;
        let line_length = state.line_length;
        let line_width = state.line_width;
        let line_mode = state.line_mode;
        let canopy_lobes = state.canopy_lobes;
        let canopy_spread = state.canopy_spread;
        let bush_layers = state.bush_layers;
        let bush_taper = state.bush_taper;
        let scatter_count = state.scatter_count;
        let scatter_jitter = state.scatter_jitter;
        let height_depth = state.height_depth;
        let height_falloff = state.height_falloff;
        let noise_strength = state.noise_strength;
        let noise_seed = state.noise_seed;
        let palette_mode = state.palette_mode;
        let palette_indices = state.palette_indices.clone();

        let center = Vec2::new(size as f32 * 0.52, size as f32 * 0.54);
        let base_radius = (radius * circle_radius).clamp(0.05, 4.0) * 28.0;
        let scatter_count = scatter_count.max(1) as usize;
        let dab_radius = if scatter_count > 1 {
            base_radius * 0.84
        } else {
            base_radius
        };

        if matches!(output_kind, OrganicBrushOutputKind::Growth) && bush_layers > 0 {
            let layer_count = bush_layers as usize;
            paint_ground_shadow(&mut pixels, size, center, base_radius * 1.15);
            for i in 0..layer_count {
                let t = if layer_count <= 1 {
                    0.0
                } else {
                    i as f32 / (layer_count - 1) as f32
                };
                let radius_scale = 1.0 - t * bush_taper * 0.55;
                let y_bias = (0.5 - t) * base_radius * 0.28;
                paint_preview_dab(
                    &mut pixels,
                    size,
                    center + Vec2::new(0.0, y_bias),
                    dab_radius * radius_scale,
                    preview_palette_color_variant(&palette_indices, palette_mode, i as i32),
                    flow,
                    circle_softness,
                    height_falloff,
                    height_depth,
                );
            }
        } else if matches!(output_kind, OrganicBrushOutputKind::Growth) && canopy_lobes > 0 {
            paint_ground_shadow(&mut pixels, size, center, base_radius * 1.25);
            let trunk_width = (base_radius * 0.18).max(4.0);
            let trunk_height = (base_radius * 0.95).max(22.0);
            let trunk_top = center.y + base_radius * 0.28;
            let trunk_bottom = trunk_top + trunk_height;
            for y in trunk_top as i32..=trunk_bottom as i32 {
                let t = ((y as f32 - trunk_top) / trunk_height).clamp(0.0, 1.0);
                let half_w = trunk_width * (1.0 - t * 0.28);
                for x in (center.x - half_w).floor() as i32..=(center.x + half_w).ceil() as i32 {
                    let bark = preview_palette_color_variant(&[17, 18, 24], 2, y + x);
                    let tone = (0.52 + (1.0 - t) * 0.26).clamp(0.0, 1.0);
                    set_pixel(
                        &mut pixels,
                        size,
                        x,
                        y,
                        [
                            (bark[0] as f32 * tone) as u8,
                            (bark[1] as f32 * tone) as u8,
                            (bark[2] as f32 * tone) as u8,
                            255,
                        ],
                    );
                }
            }
            let canopy_center = center - Vec2::new(0.0, base_radius * 0.24);
            paint_preview_dab(
                &mut pixels,
                size,
                canopy_center,
                dab_radius * 0.95,
                preview_palette_color_variant(&palette_indices, palette_mode, -1),
                flow,
                circle_softness,
                height_falloff,
                height_depth,
            );
            let lobe_radius = dab_radius * (0.78 - canopy_spread * 0.18);
            let lobe_ring = base_radius * (0.30 + canopy_spread * 0.35);
            for i in 0..canopy_lobes as usize {
                let angle = i as f32 / canopy_lobes as f32 * std::f32::consts::TAU;
                let offset = Vec2::new(angle.cos(), angle.sin()) * lobe_ring;
                paint_preview_dab(
                    &mut pixels,
                    size,
                    canopy_center + offset,
                    lobe_radius,
                    preview_palette_color_variant(&palette_indices, palette_mode, i as i32),
                    flow,
                    circle_softness,
                    height_falloff,
                    height_depth,
                );
            }
        } else if line_mode && matches!(output_kind, OrganicBrushOutputKind::Path) {
            let stroke_length = (radius * line_length).clamp(0.2, 4.0) * 72.0;
            let stroke_width = (radius * line_width).clamp(0.03, 1.2) * 24.0;
            paint_ground_shadow(
                &mut pixels,
                size,
                Vec2::new(size as f32 * 0.50, size as f32 * 0.54),
                stroke_length * 0.20,
            );
            paint_preview_stroke(
                &mut pixels,
                size,
                Vec2::new(size as f32 * 0.44, size as f32 * 0.42),
                stroke_length,
                stroke_width,
                preview_palette_color_variant(&palette_indices, palette_mode, 0),
                flow,
                circle_softness * 0.8,
                height_falloff,
                height_depth * 1.15,
                jitter,
                scatter_jitter,
            );
            paint_preview_stroke(
                &mut pixels,
                size,
                Vec2::new(size as f32 * 0.68, size as f32 * 0.66),
                stroke_length * 0.62,
                stroke_width * 0.78,
                preview_palette_color_variant(&palette_indices, palette_mode, 1),
                flow * 0.88,
                circle_softness * 0.75,
                height_falloff,
                height_depth,
                jitter + 0.27 + noise_strength * 0.18 + noise_seed as f32 * 0.0001,
                scatter_jitter,
            );
        } else if line_mode {
            let stroke_length = (radius * line_length).clamp(0.2, 4.0) * 64.0;
            let stroke_width = (radius * line_width).clamp(0.03, 1.2) * 30.0;
            paint_preview_stroke(
                &mut pixels,
                size,
                Vec2::new(size as f32 * 0.33, size as f32 * 0.38),
                stroke_length,
                stroke_width,
                preview_palette_color_variant(&palette_indices, palette_mode, 0),
                flow,
                circle_softness,
                height_falloff,
                height_depth,
                jitter,
                scatter_jitter,
            );
            paint_preview_stroke(
                &mut pixels,
                size,
                Vec2::new(size as f32 * 0.60, size as f32 * 0.66),
                stroke_length * 0.84,
                stroke_width * 0.9,
                preview_palette_color_variant(&palette_indices, palette_mode, 1),
                flow * 0.92,
                circle_softness,
                height_falloff,
                height_depth,
                jitter + 0.19 + noise_strength * 0.2 + noise_seed as f32 * 0.0001,
                scatter_jitter,
            );
        } else {
            let anchors = [
                Vec2::new(size as f32 * 0.34, size as f32 * 0.36),
                Vec2::new(size as f32 * 0.55, size as f32 * 0.48),
                Vec2::new(size as f32 * 0.71, size as f32 * 0.66),
            ];
            for (cluster_index, anchor) in anchors.into_iter().enumerate() {
                let cluster_scale = 0.85 + cluster_index as f32 * 0.09;
                for i in 0..scatter_count {
                    let offset = preview_scatter_offset(
                        i,
                        scatter_count,
                        jitter
                            + cluster_index as f32 * 0.13
                            + noise_strength * 0.3
                            + noise_seed as f32 * 0.0001,
                        scatter_jitter,
                        base_radius * cluster_scale,
                    );
                    paint_preview_dab(
                        &mut pixels,
                        size,
                        anchor + offset,
                        dab_radius * cluster_scale,
                        preview_palette_color_variant(
                            &palette_indices,
                            palette_mode,
                            (cluster_index * scatter_count + i) as i32,
                        ),
                        flow,
                        circle_softness,
                        height_falloff,
                        height_depth,
                    );
                }
            }
        }

        OrganicPreview {
            width: size as u32,
            height: size as u32,
            pixels,
        }
    }

    fn preview_state(&self) -> OrganicPreviewState {
        let mut out = OrganicPreviewState::default();
        if let Some(output_index) = self.active_output_node_index() {
            out.output_kind = if self.nodes[output_index].kind.is_growth_output() {
                OrganicBrushOutputKind::Growth
            } else if self.nodes[output_index].kind.is_path_output() {
                OrganicBrushOutputKind::Path
            } else {
                OrganicBrushOutputKind::Paint
            };
            if let Some(params) = self.nodes[output_index].kind.output_params() {
                out.radius = params.radius.max(0.05);
                out.flow = params.flow.clamp(0.05, 1.0);
                out.jitter = params.jitter.clamp(0.0, 1.0);
            }

            let shape_nodes = self.collect_branch_nodes(output_index, 0, |kind| {
                matches!(
                    kind,
                    OrganicNodeKind::CircleMask { .. }
                        | OrganicNodeKind::BushShape { .. }
                        | OrganicNodeKind::CanopyShape { .. }
                        | OrganicNodeKind::LineShape { .. }
                        | OrganicNodeKind::Noise { .. }
                        | OrganicNodeKind::Scatter { .. }
                )
            });
            let growth_nodes = self.collect_branch_nodes(output_index, 2, |kind| {
                matches!(kind, OrganicNodeKind::HeightProfile { .. })
            });
            let material_nodes = self.collect_branch_nodes(output_index, 1, |kind| {
                matches!(
                    kind,
                    OrganicNodeKind::PaletteColors { .. } | OrganicNodeKind::PaletteRange { .. }
                )
            });

            for node_index in shape_nodes {
                match self.nodes[node_index].kind {
                    OrganicNodeKind::CircleMask { radius, softness } => {
                        out.circle_radius = radius.max(0.05);
                        out.circle_softness = softness.clamp(0.0, 1.0);
                    }
                    OrganicNodeKind::CanopyShape {
                        radius,
                        lobes,
                        spread,
                        softness,
                    } => {
                        out.circle_radius = radius.max(0.05);
                        out.circle_softness = softness.clamp(0.0, 1.0);
                        out.canopy_lobes = lobes.max(3);
                        out.canopy_spread = spread.clamp(0.0, 1.0);
                    }
                    OrganicNodeKind::BushShape {
                        radius,
                        layers,
                        taper,
                        softness,
                        ..
                    } => {
                        out.circle_radius = radius.max(0.05);
                        out.circle_softness = softness.clamp(0.0, 1.0);
                        out.bush_layers = layers.max(2);
                        out.bush_taper = taper.clamp(0.0, 1.0);
                    }
                    OrganicNodeKind::LineShape {
                        length,
                        width,
                        softness,
                    } => {
                        out.line_mode = true;
                        out.line_length = length.max(0.1);
                        out.line_width = width.max(0.02);
                        out.circle_radius = (length.max(width) * 0.5).max(0.05);
                        out.circle_softness = softness.clamp(0.0, 1.0);
                    }
                    OrganicNodeKind::Scatter { count, jitter } => {
                        out.scatter_count = count.max(1);
                        out.scatter_jitter = jitter.clamp(0.0, 1.0);
                    }
                    OrganicNodeKind::Noise {
                        scale: _,
                        strength,
                        seed,
                    } => {
                        out.noise_strength = strength.clamp(0.0, 1.0);
                        out.noise_seed = seed;
                    }
                    _ => {}
                }
            }
            for node_index in growth_nodes {
                if let OrganicNodeKind::HeightProfile { depth, falloff } =
                    self.nodes[node_index].kind
                {
                    out.height_depth = depth.max(0.05);
                    out.height_falloff = falloff.clamp(0.0, 1.0);
                }
            }
            for node_index in material_nodes {
                match self.nodes[node_index].kind {
                    OrganicNodeKind::PaletteColors {
                        color_1,
                        color_2,
                        color_3,
                        color_4,
                        mode,
                    } => {
                        out.palette_indices = vec![
                            color_1.clamp(0, 255) as usize,
                            color_2.clamp(0, 255) as usize,
                            color_3.clamp(0, 255) as usize,
                            color_4.clamp(0, 255) as usize,
                        ];
                        out.palette_mode = mode.clamp(0, 2);
                    }
                    OrganicNodeKind::PaletteRange { start, count, mode } => {
                        out.palette_indices = (0..count.clamp(1, 16))
                            .map(|i| (start + i).clamp(0, 255) as usize)
                            .collect();
                        out.palette_mode = mode.clamp(0, 2);
                    }
                    _ => {}
                }
            }
        }
        out
    }

    fn active_output_node_index(&self) -> Option<usize> {
        if let Some(index) = self.selected_node
            && matches!(self.nodes.get(index).map(|node| &node.kind), Some(kind) if kind.is_output())
        {
            return Some(index);
        }
        self.nodes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, node)| node.kind.is_output().then_some(index))
    }

    fn collect_branch_nodes<F>(&self, dst_node: usize, dst_terminal: u8, allow: F) -> HashSet<usize>
    where
        F: Fn(&OrganicNodeKind) -> bool + Copy,
    {
        let mut visited = HashSet::default();
        self.collect_upstream_nodes_filtered(dst_node, dst_terminal, allow, &mut visited);
        visited
    }

    fn collect_upstream_nodes_filtered<F>(
        &self,
        dst_node: usize,
        dst_terminal: u8,
        allow: F,
        visited: &mut HashSet<usize>,
    ) where
        F: Fn(&OrganicNodeKind) -> bool + Copy,
    {
        for (from_node, _, to_node, to_terminal) in &self.connections {
            if *to_node as usize != dst_node || *to_terminal != dst_terminal {
                continue;
            }
            let from_index = *from_node as usize;
            if visited.contains(&from_index) {
                continue;
            }
            let Some(node) = self.nodes.get(from_index) else {
                continue;
            };
            if !allow(&node.kind) {
                continue;
            }
            visited.insert(from_index);
            for upstream_terminal in 0..3u8 {
                self.collect_upstream_nodes_filtered(from_index, upstream_terminal, allow, visited);
            }
        }
    }
}

struct OrganicPreviewState {
    radius: f32,
    flow: f32,
    jitter: f32,
    output_kind: OrganicBrushOutputKind,
    circle_radius: f32,
    circle_softness: f32,
    line_length: f32,
    line_width: f32,
    line_mode: bool,
    canopy_lobes: i32,
    canopy_spread: f32,
    bush_layers: i32,
    bush_taper: f32,
    scatter_count: i32,
    scatter_jitter: f32,
    height_depth: f32,
    height_falloff: f32,
    noise_strength: f32,
    noise_seed: i32,
    palette_indices: Vec<usize>,
    palette_mode: i32,
}

impl Default for OrganicPreviewState {
    fn default() -> Self {
        Self {
            radius: 0.6,
            flow: 1.0,
            jitter: 0.15,
            output_kind: OrganicBrushOutputKind::Paint,
            circle_radius: 1.0,
            circle_softness: 0.35,
            line_length: 0.0,
            line_width: 0.0,
            line_mode: false,
            canopy_lobes: 0,
            canopy_spread: 0.0,
            bush_layers: 0,
            bush_taper: 0.0,
            scatter_count: 1,
            scatter_jitter: 0.0,
            height_depth: 1.0,
            height_falloff: 0.5,
            noise_strength: 0.0,
            noise_seed: 1,
            palette_indices: vec![4, 8, 10, 12],
            palette_mode: 1,
        }
    }
}

fn set_pixel(pixels: &mut [u8], size: usize, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
        return;
    }
    let i = ((y as usize) * size + x as usize) * 4;
    pixels[i..i + 4].copy_from_slice(&color);
}

fn paint_preview_background(pixels: &mut [u8], size: usize) {
    for y in 0..size as i32 {
        for x in 0..size as i32 {
            let fx = x as f32 / size as f32;
            let fy = y as f32 / size as f32;
            let vignette = ((fx - 0.5).powi(2) + (fy - 0.5).powi(2))
                .sqrt()
                .clamp(0.0, 0.75);
            let grain = preview_noise(x, y);
            let bands = preview_noise(x / 8, y / 8) * 0.6 + preview_noise(x / 19, y / 19) * 0.4;
            let base = 66.0 + bands * 26.0 - vignette * 34.0 + grain * 8.0;
            let green = base + 9.0;
            let blue = base - 2.0;
            set_pixel(
                pixels,
                size,
                x,
                y,
                [
                    base.clamp(0.0, 255.0) as u8,
                    green.clamp(0.0, 255.0) as u8,
                    blue.clamp(0.0, 255.0) as u8,
                    255,
                ],
            );
        }
    }
}

fn get_pixel(pixels: &[u8], size: usize, x: i32, y: i32) -> [u8; 4] {
    if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
        return [0, 0, 0, 0];
    }
    let i = ((y as usize) * size + x as usize) * 4;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

fn preview_noise(x: i32, y: i32) -> f32 {
    let mut n = x.wrapping_mul(374_761_393) ^ y.wrapping_mul(668_265_263) ^ 0x27d4_eb2d_u32 as i32;
    n = (n ^ (n >> 13)).wrapping_mul(1_274_126_177);
    ((n ^ (n >> 16)) as u32 & 1023) as f32 / 1023.0
}

fn preview_scatter_offset(
    index: usize,
    count: usize,
    jitter: f32,
    scatter_jitter: f32,
    base_radius: f32,
) -> Vec2<f32> {
    if count <= 1 {
        return Vec2::zero();
    }
    let angle = (index as f32 * 2.3999632) + jitter * std::f32::consts::PI;
    let ring = ((index + 1) as f32 / count as f32).sqrt();
    let amount = base_radius * scatter_jitter * (0.15 + jitter * 0.35);
    Vec2::new(angle.cos(), angle.sin()) * (ring * amount)
}

fn paint_preview_dab(
    pixels: &mut [u8],
    size: usize,
    center: Vec2<f32>,
    radius: f32,
    tint: [u8; 3],
    flow: f32,
    edge_softness: f32,
    height_falloff: f32,
    height_depth: f32,
) {
    let min_x = (center.x - radius).floor().max(0.0) as i32;
    let max_x = (center.x + radius).ceil().min((size - 1) as f32) as i32;
    let min_y = (center.y - radius).floor().max(0.0) as i32;
    let max_y = (center.y + radius).ceil().min((size - 1) as f32) as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let dist = (p - center).magnitude();
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
            let alpha = (edge * flow * 220.0).clamp(0.0, 255.0) as u8;
            let shade =
                (52.0 + falloff * height_depth.clamp(0.2, 2.5) * 165.0).clamp(0.0, 255.0) / 255.0;
            let existing = get_pixel(pixels, size, x, y);
            let edge_dark = (1.0 - radial).powf(1.8) * 0.32;
            let highlight = radial.powf(0.7) * 0.18;
            let tone = (0.26 + shade * 0.70 + highlight - edge_dark).clamp(0.0, 1.0);
            let tinted = [
                (tint[0] as f32 * tone).clamp(0.0, 255.0) as u8,
                (tint[1] as f32 * tone).clamp(0.0, 255.0) as u8,
                (tint[2] as f32 * tone).clamp(0.0, 255.0) as u8,
            ];
            let out = [
                ((existing[0] as f32 * 0.78) + tinted[0] as f32 * 0.92).clamp(0.0, 255.0) as u8,
                ((existing[1] as f32 * 0.78) + tinted[1] as f32 * 0.92).clamp(0.0, 255.0) as u8,
                ((existing[2] as f32 * 0.78) + tinted[2] as f32 * 0.92).clamp(0.0, 255.0) as u8,
                existing[3].max(alpha),
            ];
            set_pixel(pixels, size, x, y, out);
        }
    }
}

fn paint_ground_shadow(pixels: &mut [u8], size: usize, center: Vec2<f32>, radius: f32) {
    let min_x = (center.x - radius).floor().max(0.0) as i32;
    let max_x = (center.x + radius).ceil().min((size - 1) as f32) as i32;
    let min_y = (center.y - radius * 0.5).floor().max(0.0) as i32;
    let max_y = (center.y + radius * 0.5).ceil().min((size - 1) as f32) as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
            let local = p - center;
            let e = ((local.x / radius).powi(2) + (local.y / (radius * 0.42)).powi(2)).sqrt();
            if e > 1.0 {
                continue;
            }
            let shade = (1.0 - e).powf(1.6) * 52.0;
            let existing = get_pixel(pixels, size, x, y);
            set_pixel(
                pixels,
                size,
                x,
                y,
                [
                    existing[0].saturating_sub(shade as u8),
                    existing[1].saturating_sub((shade * 0.8) as u8),
                    existing[2].saturating_sub((shade * 0.5) as u8),
                    255,
                ],
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_preview_stroke(
    pixels: &mut [u8],
    size: usize,
    center: Vec2<f32>,
    length: f32,
    width: f32,
    tint: [u8; 3],
    flow: f32,
    edge_softness: f32,
    height_falloff: f32,
    height_depth: f32,
    jitter: f32,
    scatter_jitter: f32,
) {
    let steps = ((length / (width.max(2.0) * 0.55)).ceil() as usize).max(6);
    let half = (steps.saturating_sub(1)) as f32 * 0.5;
    let angle = -0.7 + jitter * 1.4;
    let tangent = Vec2::new(angle.cos(), angle.sin());
    let normal = Vec2::new(-tangent.y, tangent.x);
    let wobble = width * (0.18 + scatter_jitter * 0.35);

    for i in 0..steps {
        let t = if steps <= 1 {
            0.0
        } else {
            i as f32 / (steps - 1) as f32
        };
        let along = (i as f32 - half) * (length / steps as f32);
        let snake = ((t * std::f32::consts::TAU * 1.6) + jitter * 5.0).sin() * wobble;
        let taper = 0.75 + (1.0 - (t - 0.5).abs() * 2.0).clamp(0.0, 1.0) * 0.35;
        let pos = center + tangent * along + normal * snake;
        paint_preview_dab(
            pixels,
            size,
            pos,
            width * taper,
            tint,
            flow,
            edge_softness,
            height_falloff,
            height_depth,
        );
    }
}

fn preview_palette_color_variant(indices: &[usize], mode: i32, variant: i32) -> [u8; 3] {
    let idx = match indices {
        [] => 0,
        [only] => *only,
        many => {
            let pick = match mode {
                2 => variant.rem_euclid(many.len() as i32) as usize,
                1 => {
                    let seed = variant.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                    if many.len() > 1 && seed.rem_euclid(5) == 0 {
                        1 + seed.rem_euclid((many.len() as i32 - 1).max(1)) as usize
                    } else {
                        0
                    }
                }
                _ => 0,
            };
            many[pick]
        }
    };
    preview_palette_color(idx)
}

fn preview_palette_color(index: usize) -> [u8; 3] {
    const PALETTE: [[u8; 3]; 29] = [
        [0xf2, 0xf0, 0xe5],
        [0xb8, 0xb5, 0xb9],
        [0x86, 0x81, 0x88],
        [0x64, 0x63, 0x65],
        [0x45, 0x44, 0x4f],
        [0x3a, 0x38, 0x58],
        [0x21, 0x21, 0x23],
        [0x35, 0x2b, 0x42],
        [0x43, 0x43, 0x6a],
        [0x4b, 0x80, 0xca],
        [0x68, 0xc2, 0xd3],
        [0xa2, 0xdc, 0xc7],
        [0xed, 0xe1, 0x9e],
        [0xd3, 0xa0, 0x68],
        [0xb4, 0x52, 0x52],
        [0x6a, 0x53, 0x6e],
        [0x4b, 0x41, 0x58],
        [0x80, 0x49, 0x3a],
        [0xa7, 0x7b, 0x5b],
        [0xe5, 0xce, 0xb4],
        [0xc2, 0xd3, 0x68],
        [0x8a, 0xb0, 0x60],
        [0x56, 0x7b, 0x79],
        [0x4e, 0x58, 0x4a],
        [0x7b, 0x72, 0x43],
        [0xb2, 0xb4, 0x7e],
        [0xed, 0xc8, 0xc4],
        [0xcf, 0x8a, 0xcb],
        [0x5f, 0x55, 0x6a],
    ];
    PALETTE[index % PALETTE.len()]
}

struct OrganicBrushRecipe {
    shape: OrganicBrushShape,
    noise: Option<(f32, f32, i32)>,
    scatter_count: i32,
    scatter_jitter: f32,
    height_depth: f32,
    height_falloff: f32,
    material_channel: i32,
    palette_colors: [i32; 4],
    palette_mode: i32,
    output_radius: f32,
    output_flow: f32,
    output_jitter: f32,
    output_depth: f32,
    output_cell_size: f32,
    output_kind: OrganicBrushOutputKind,
}

enum OrganicBrushShape {
    Blob {
        radius: f32,
        softness: f32,
    },
    Canopy {
        radius: f32,
        lobes: i32,
        spread: f32,
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
    PaletteColors {
        color_1: i32,
        color_2: i32,
        color_3: i32,
        color_4: i32,
        mode: i32,
    },
    Material {
        channel: i32,
    },
    #[serde(alias = "OutputVolume")]
    OutputPaint {
        radius: f32,
        flow: f32,
        jitter: f32,
        depth: f32,
        cell_size: f32,
        #[serde(default = "default_output_channel")]
        channel: i32,
        #[serde(default = "default_output_palette_start")]
        palette_start: i32,
        #[serde(default = "default_output_palette_count")]
        palette_count: i32,
        #[serde(default = "default_output_palette_mode")]
        palette_mode: i32,
    },
    OutputGrowth {
        radius: f32,
        flow: f32,
        jitter: f32,
        depth: f32,
        cell_size: f32,
        #[serde(default = "default_output_channel")]
        channel: i32,
        #[serde(default = "default_output_palette_start")]
        palette_start: i32,
        #[serde(default = "default_output_palette_count")]
        palette_count: i32,
        #[serde(default = "default_output_palette_mode")]
        palette_mode: i32,
    },
    OutputPath {
        radius: f32,
        flow: f32,
        jitter: f32,
        depth: f32,
        cell_size: f32,
        #[serde(default = "default_output_channel")]
        channel: i32,
        #[serde(default = "default_output_palette_start")]
        palette_start: i32,
        #[serde(default = "default_output_palette_count")]
        palette_count: i32,
        #[serde(default = "default_output_palette_mode")]
        palette_mode: i32,
    },
}

impl OrganicNodeKind {
    pub fn output_params(&self) -> Option<OrganicOutputParams> {
        match self {
            Self::OutputPaint {
                radius,
                flow,
                jitter,
                depth,
                cell_size,
                channel,
                palette_start,
                palette_count,
                palette_mode,
            }
            | Self::OutputGrowth {
                radius,
                flow,
                jitter,
                depth,
                cell_size,
                channel,
                palette_start,
                palette_count,
                palette_mode,
            }
            | Self::OutputPath {
                radius,
                flow,
                jitter,
                depth,
                cell_size,
                channel,
                palette_start,
                palette_count,
                palette_mode,
            } => Some(OrganicOutputParams {
                radius: *radius,
                flow: *flow,
                jitter: *jitter,
                depth: *depth,
                cell_size: *cell_size,
                channel: *channel,
                palette_start: *palette_start,
                palette_count: *palette_count,
                palette_mode: *palette_mode,
            }),
            _ => None,
        }
    }

    pub fn is_output(&self) -> bool {
        matches!(
            self,
            Self::OutputPaint { .. } | Self::OutputGrowth { .. } | Self::OutputPath { .. }
        )
    }

    pub fn is_growth_output(&self) -> bool {
        matches!(self, Self::OutputGrowth { .. })
    }

    pub fn is_path_output(&self) -> bool {
        matches!(self, Self::OutputPath { .. })
    }
}

fn default_graph_name() -> String {
    "Default Paint Brush".to_string()
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
                count: 6,
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
            OrganicNodeKind::OutputPaint {
                radius: 0.6,
                flow: 1.0,
                jitter: 0.15,
                depth: 0.45,
                cell_size: 0.25,
                channel: 0,
                palette_start: 0,
                palette_count: 1,
                palette_mode: 0,
            },
            Vec2::new(700, 108),
        ),
    ]
}
