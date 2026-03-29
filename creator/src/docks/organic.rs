use crate::editor::PALETTE;
use crate::prelude::*;
use organicgraph::{OrganicBrushGraph, OrganicBrushNode, OrganicNodeKind};

const ORGANIC_CANVAS_VIEW: &str = "Organic Brush NodeCanvas";
const ORGANIC_SETTINGS_LAYOUT: &str = "Organic Brush Settings";
const ORGANIC_SURFACE_BUTTON: &str = "Organic Dock Surface Nodes";
const ORGANIC_SHAPE_BUTTON: &str = "Organic Dock Shape Nodes";
const ORGANIC_PLACEMENT_BUTTON: &str = "Organic Dock Placement Nodes";
const ORGANIC_GROWTH_BUTTON: &str = "Organic Dock Growth Nodes";
const ORGANIC_OUTPUT_BUTTON: &str = "Organic Dock Output Nodes";
const ORGANIC_PRESET_BUTTON: &str = "Organic Dock Presets";
const ORGANIC_RESET_BUTTON: &str = "Organic Dock Reset Graph";
const ORGANIC_ACTIVE_GRAPH_PROP: &str = "organic_brush_active_graph";

fn organic_node_name(kind: &OrganicNodeKind) -> &'static str {
    match kind {
        OrganicNodeKind::SurfaceInput => "Surface Input",
        OrganicNodeKind::CircleMask { .. } => "Blob Shape",
        OrganicNodeKind::CanopyShape { .. } => "Canopy Shape",
        OrganicNodeKind::BushShape { .. } => "Bush Shape",
        OrganicNodeKind::LineShape { .. } => "Line Shape",
        OrganicNodeKind::Noise { .. } => "Breakup Noise",
        OrganicNodeKind::Scatter { .. } => "Repeat",
        OrganicNodeKind::HeightProfile { .. } => "Growth Profile",
        OrganicNodeKind::PaletteRange { .. } => "Palette Range",
        OrganicNodeKind::PaletteColors { .. } => "Palette Colors",
        OrganicNodeKind::Material { .. } => "Material",
        OrganicNodeKind::OutputPaint { .. } => "Output Paint",
        OrganicNodeKind::OutputGrowth { .. } => "Output Growth",
        OrganicNodeKind::OutputPath { .. } => "Output Path",
    }
}

fn organic_node_status_text(kind: &OrganicNodeKind) -> &'static str {
    match kind {
        OrganicNodeKind::SurfaceInput => {
            "Provides host context for the brush graph, including surface/terrain placement."
        }
        OrganicNodeKind::CircleMask { .. } => {
            "Defines the base blob or mound shape that gets stamped onto the host."
        }
        OrganicNodeKind::CanopyShape { .. } => {
            "Defines a clustered crown shape for bushes and tree canopies."
        }
        OrganicNodeKind::BushShape { .. } => {
            "Defines a layered shrub volume with taper and breakup."
        }
        OrganicNodeKind::LineShape { .. } => {
            "Defines an elongated directional brush shape for vines, roots, and streaks."
        }
        OrganicNodeKind::Noise { .. } => {
            "Adds breakup and wobble so repeated shapes do not feel too clean or mechanical."
        }
        OrganicNodeKind::Scatter { .. } => {
            "Repeats the base shape into clumps for grass, moss, bubbles, bushes, or rough buildup."
        }
        OrganicNodeKind::HeightProfile { .. } => {
            "Controls how the shape grows away from the host and how quickly that growth tapers."
        }
        OrganicNodeKind::PaletteRange { .. } => {
            "Provides a palette range source for the material branch, with optional variation."
        }
        OrganicNodeKind::PaletteColors { .. } => {
            "Chooses explicit palette colors and how the brush varies between them."
        }
        OrganicNodeKind::Material { .. } => "Chooses which material channel the brush writes into.",
        OrganicNodeKind::OutputPaint { .. } => {
            "Final paint output. Controls stamp scale, deposition strength, and paint resolution."
        }
        OrganicNodeKind::OutputGrowth { .. } => {
            "Final growth output. Controls spawn scale, growth depth, and cluster resolution."
        }
        OrganicNodeKind::OutputPath { .. } => {
            "Future path-growth output for vines, roots, and branch-like strokes."
        }
    }
}

fn organic_node_inputs(kind: &OrganicNodeKind) -> Vec<TheNodeTerminal> {
    let shape = |name: &str| TheNodeTerminal {
        name: name.to_string(),
        category_name: "Shape".to_string(),
    };
    let placement = |name: &str| TheNodeTerminal {
        name: name.to_string(),
        category_name: "Placement".to_string(),
    };
    let growth = |name: &str| TheNodeTerminal {
        name: name.to_string(),
        category_name: "Growth".to_string(),
    };
    match kind {
        OrganicNodeKind::SurfaceInput => Vec::new(),
        OrganicNodeKind::CircleMask { .. } => Vec::new(),
        OrganicNodeKind::CanopyShape { .. } => Vec::new(),
        OrganicNodeKind::BushShape { .. } => Vec::new(),
        OrganicNodeKind::LineShape { .. } => Vec::new(),
        OrganicNodeKind::Noise { .. } => vec![shape("Shape")],
        OrganicNodeKind::Scatter { .. } => vec![shape("Shape"), placement("Placement")],
        OrganicNodeKind::HeightProfile { .. } => vec![growth("Growth")],
        OrganicNodeKind::PaletteRange { .. } => Vec::new(),
        OrganicNodeKind::PaletteColors { .. } => Vec::new(),
        OrganicNodeKind::Material { .. } => Vec::new(),
        OrganicNodeKind::OutputPaint { .. }
        | OrganicNodeKind::OutputGrowth { .. }
        | OrganicNodeKind::OutputPath { .. } => {
            vec![
                shape("Shape"),
                TheNodeTerminal {
                    name: "Palette".to_string(),
                    category_name: "Output".to_string(),
                },
                growth("Growth"),
            ]
        }
    }
}

fn organic_node_outputs(kind: &OrganicNodeKind) -> Vec<TheNodeTerminal> {
    let shape = |name: &str| TheNodeTerminal {
        name: name.to_string(),
        category_name: "Shape".to_string(),
    };
    let growth = |name: &str| TheNodeTerminal {
        name: name.to_string(),
        category_name: "Growth".to_string(),
    };
    match kind {
        OrganicNodeKind::SurfaceInput => Vec::new(),
        OrganicNodeKind::CircleMask { .. } => vec![shape("Shape")],
        OrganicNodeKind::CanopyShape { .. } => vec![shape("Shape")],
        OrganicNodeKind::BushShape { .. } => vec![shape("Shape")],
        OrganicNodeKind::LineShape { .. } => vec![shape("Shape")],
        OrganicNodeKind::Noise { .. } => vec![shape("Shape")],
        OrganicNodeKind::Scatter { .. } => vec![shape("Shape")],
        OrganicNodeKind::HeightProfile { .. } => vec![growth("Growth")],
        OrganicNodeKind::PaletteRange { .. } => Vec::new(),
        OrganicNodeKind::PaletteColors { .. } => vec![TheNodeTerminal {
            name: "Palette".to_string(),
            category_name: "Output".to_string(),
        }],
        OrganicNodeKind::Material { .. } => Vec::new(),
        OrganicNodeKind::OutputPaint { .. }
        | OrganicNodeKind::OutputGrowth { .. }
        | OrganicNodeKind::OutputPath { .. } => Vec::new(),
    }
}

pub struct OrganicDock {
    graph: OrganicBrushGraph,
    active_graph_id: Uuid,
    categories: FxHashMap<String, TheColor>,
}

#[derive(Clone, Copy)]
enum OrganicPresetKind {
    Moss,
    Mud,
    Grass,
    PathVines,
    Bush,
    Tree,
}

impl OrganicDock {
    fn default_categories() -> FxHashMap<String, TheColor> {
        let mut categories = FxHashMap::default();
        categories.insert("Surface".into(), TheColor::from("#d9b65d"));
        categories.insert("Shape".into(), TheColor::from("#4c8bf5"));
        categories.insert("Placement".into(), TheColor::from("#0aa37f"));
        categories.insert("Growth".into(), TheColor::from("#64a6a8"));
        categories.insert("Output".into(), TheColor::from("#d47f4a"));
        categories
    }

    fn load_state_from_map(&mut self, project: &Project, server_ctx: &ServerContext) {
        if let Some(map) = project.get_map(server_ctx) {
            let active_graph_id = match map.properties.get(ORGANIC_ACTIVE_GRAPH_PROP) {
                Some(Value::Id(id)) => Some(*id),
                _ => None,
            };

            if let Some(id) = active_graph_id
                && let Some(graph) = map.organic_brush_graphs.get(&id)
            {
                self.active_graph_id = id;
                self.graph = graph.clone();
                if let Some(preset) = Self::preset_kind_for_graph_name(&self.graph.name) {
                    Self::retarget_preset_palette(&mut self.graph, project, preset);
                }
                return;
            }

            if let Some((id, graph)) = map.organic_brush_graphs.first() {
                self.active_graph_id = *id;
                self.graph = graph.clone();
                if let Some(preset) = Self::preset_kind_for_graph_name(&self.graph.name) {
                    Self::retarget_preset_palette(&mut self.graph, project, preset);
                }
                return;
            }
        }
        self.graph = OrganicBrushGraph::default();
        Self::retarget_preset_palette(&mut self.graph, project, OrganicPresetKind::Moss);
        self.active_graph_id = self.graph.id;
    }

    fn preset_kind_for_graph_name(name: &str) -> Option<OrganicPresetKind> {
        match name {
            "Default Paint Brush" | "Moss Brush" => Some(OrganicPresetKind::Moss),
            "Mud Brush" => Some(OrganicPresetKind::Mud),
            "Grass Brush" => Some(OrganicPresetKind::Grass),
            "Vines Brush" => Some(OrganicPresetKind::PathVines),
            "Bush Brush" => Some(OrganicPresetKind::Bush),
            "Tree Brush" => Some(OrganicPresetKind::Tree),
            _ => None,
        }
    }

    fn preset_palette_mode(kind: OrganicPresetKind) -> i32 {
        match kind {
            OrganicPresetKind::Moss => 0,
            OrganicPresetKind::Mud => 0,
            OrganicPresetKind::Grass => 0,
            OrganicPresetKind::PathVines => 0,
            OrganicPresetKind::Bush => 0,
            OrganicPresetKind::Tree => 0,
        }
    }

    fn preset_palette_targets(kind: OrganicPresetKind) -> [TheColor; 4] {
        match kind {
            OrganicPresetKind::Moss => [
                TheColor::from("#415835"),
                TheColor::from("#567b43"),
                TheColor::from("#7ea25a"),
                TheColor::from("#b2c776"),
            ],
            OrganicPresetKind::Mud => [
                TheColor::from("#4f382a"),
                TheColor::from("#6e4f38"),
                TheColor::from("#8e7050"),
                TheColor::from("#b59a73"),
            ],
            OrganicPresetKind::Grass => [
                TheColor::from("#4f7a39"),
                TheColor::from("#4f7a39"),
                TheColor::from("#7ea653"),
                TheColor::from("#b2c776"),
            ],
            OrganicPresetKind::PathVines => [
                TheColor::from("#415835"),
                TheColor::from("#567b43"),
                TheColor::from("#62724a"),
                TheColor::from("#8c9061"),
            ],
            OrganicPresetKind::Bush => [
                TheColor::from("#3f6a43"),
                TheColor::from("#3f6a43"),
                TheColor::from("#638b55"),
                TheColor::from("#9fb36d"),
            ],
            OrganicPresetKind::Tree => [
                TheColor::from("#3f6a43"),
                TheColor::from("#638b55"),
                TheColor::from("#9fb36d"),
                TheColor::from("#b2c776"),
            ],
        }
    }

    fn retarget_preset_palette(
        graph: &mut OrganicBrushGraph,
        project: &Project,
        preset: OrganicPresetKind,
    ) {
        let target = Self::preset_palette_targets(preset);
        let map_index = |color: TheColor| -> i32 {
            project
                .palette
                .find_closest_color_index(&color)
                .map(|idx| idx as i32)
                .unwrap_or(0)
        };

        for node in &mut graph.nodes {
            if let OrganicNodeKind::PaletteColors {
                color_1,
                color_2,
                color_3,
                color_4,
                mode,
            } = &mut node.kind
            {
                *color_1 = map_index(target[0].clone());
                *color_2 = map_index(target[1].clone());
                *color_3 = map_index(target[2].clone());
                *color_4 = map_index(target[3].clone());
                *mode = Self::preset_palette_mode(preset);
            }
        }
    }

    fn save_state_to_map(&self, project: &mut Project, server_ctx: &ServerContext) {
        if let Some(map) = project.get_map_mut(server_ctx) {
            let mut graph = self.graph.clone();
            graph.id = self.active_graph_id;
            map.properties
                .set(ORGANIC_ACTIVE_GRAPH_PROP, Value::Id(self.active_graph_id));
            map.organic_brush_graphs.insert(self.active_graph_id, graph);
            map.changed += 1;
        }
    }

    fn graph_to_canvas(&self) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 152,
            selected_node: self.graph.selected_node,
            offset: self.graph.scroll_offset,
            connections: self.graph.connections.clone(),
            categories: self.categories.clone(),
            ..Default::default()
        };

        for (index, node) in self.graph.nodes.iter().enumerate() {
            canvas.nodes.push(TheNode {
                name: organic_node_name(&node.kind).to_string(),
                status_text: Some(organic_node_status_text(&node.kind).to_string()),
                position: node.position,
                inputs: organic_node_inputs(&node.kind),
                outputs: organic_node_outputs(&node.kind),
                preview: TheRGBABuffer::default(),
                supports_preview: false,
                preview_is_open: false,
                can_be_deleted: index != 0,
            });
        }

        canvas
    }

    fn sync_canvas(&self, ui: &mut TheUI) {
        ui.set_node_canvas(ORGANIC_CANVAS_VIEW, self.graph_to_canvas());
        ui.set_node_overlay_tiled(ORGANIC_CANVAS_VIEW, false);
    }

    fn sync_preview_overlay(&self, ui: &mut TheUI) {
        ui.set_node_overlay(ORGANIC_CANVAS_VIEW, Some(self.render_preview_overlay()));
    }

    fn render_preview_overlay(&self) -> TheRGBABuffer {
        let preview = self.graph.render_preview(156);
        let mut buffer =
            TheRGBABuffer::new(TheDim::sized(preview.width as i32, preview.height as i32));
        buffer.pixels_mut().copy_from_slice(&preview.pixels);
        buffer
    }

    fn clear_selected_node_ui(&self, ui: &mut TheUI, ctx: &mut TheContext) {
        if let Some(layout) = ui.get_text_layout(ORGANIC_SETTINGS_LAYOUT) {
            layout.clear();
            ctx.ui.relayout = true;
        }
    }

    fn set_selected_node_ui(
        &self,
        _project: &Project,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        _server_ctx: &ServerContext,
    ) {
        let mut nodeui = TheNodeUI::default();

        if let Some(index) = self.graph.selected_node
            && let Some(node) = self.graph.nodes.get(index)
        {
            nodeui.add_item(TheNodeUIItem::OpenTree("node".into()));
            match &node.kind {
                OrganicNodeKind::SurfaceInput => {}
                OrganicNodeKind::CircleMask { radius, softness } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeCircleRadius".into(),
                        "Size".into(),
                        "Size of the base blob shape.".into(),
                        *radius,
                        0.01..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeCircleSoftness".into(),
                        "Edge".into(),
                        "Softness of the blob edge.".into(),
                        *softness,
                        0.0..=1.0,
                        false,
                    ));
                }
                OrganicNodeKind::BushShape {
                    radius,
                    height,
                    layers,
                    taper,
                    breakup,
                    softness,
                } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeBushRadius".into(),
                        "Radius".into(),
                        "Overall shrub width.".into(),
                        *radius,
                        0.05..=2.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeBushHeight".into(),
                        "Height".into(),
                        "Vertical height of the bush mass.".into(),
                        *height,
                        0.1..=3.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "organicNodeBushLayers".into(),
                        "Layers".into(),
                        "Number of stacked canopy layers.".into(),
                        *layers,
                        2..=8,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeBushTaper".into(),
                        "Taper".into(),
                        "How much the upper layers narrow.".into(),
                        *taper,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeBushBreakup".into(),
                        "Breakup".into(),
                        "Irregularity between bush layers.".into(),
                        *breakup,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeBushSoftness".into(),
                        "Edge".into(),
                        "Softness of the shrub edge.".into(),
                        *softness,
                        0.0..=1.0,
                        false,
                    ));
                }
                OrganicNodeKind::CanopyShape {
                    radius,
                    lobes,
                    spread,
                    softness,
                } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeCanopyRadius".into(),
                        "Radius".into(),
                        "Overall crown size.".into(),
                        *radius,
                        0.05..=2.5,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "organicNodeCanopyLobes".into(),
                        "Lobes".into(),
                        "Number of canopy lobes.".into(),
                        *lobes,
                        3..=10,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeCanopySpread".into(),
                        "Spread".into(),
                        "How far canopy lobes spread from the center.".into(),
                        *spread,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeCanopySoftness".into(),
                        "Edge".into(),
                        "Softness of the canopy edge.".into(),
                        *softness,
                        0.0..=1.0,
                        false,
                    ));
                }
                OrganicNodeKind::Noise {
                    scale,
                    strength,
                    seed,
                } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeNoiseScale".into(),
                        "Scale".into(),
                        "Noise scale applied to the brush shape.".into(),
                        *scale,
                        0.01..=2.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeNoiseStrength".into(),
                        "Strength".into(),
                        "Noise strength.".into(),
                        *strength,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "organicNodeNoiseSeed".into(),
                        "Seed".into(),
                        "Noise seed.".into(),
                        *seed,
                        0..=9999,
                        false,
                    ));
                }
                OrganicNodeKind::Scatter { count, jitter } => {
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "organicNodeScatterCount".into(),
                        "Copies".into(),
                        "Number of repeated shape copies.".into(),
                        *count,
                        1..=32,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeScatterJitter".into(),
                        "Spread".into(),
                        "How far repeated copies spread from the center.".into(),
                        *jitter,
                        0.0..=1.0,
                        false,
                    ));
                }
                OrganicNodeKind::HeightProfile { depth, falloff } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeHeightDepth".into(),
                        "Height".into(),
                        "How much the shape grows away from the host.".into(),
                        *depth,
                        0.01..=2.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeHeightFalloff".into(),
                        "Taper".into(),
                        "How quickly the growth profile tapers.".into(),
                        *falloff,
                        0.0..=1.0,
                        false,
                    ));
                }
                OrganicNodeKind::PaletteRange { start, count, mode } => {
                    nodeui.add_item(TheNodeUIItem::PaletteSlider(
                        "organicNodePaletteStart".into(),
                        "Palette".into(),
                        "Base palette index used as the material source for this branch.".into(),
                        (*start).clamp(0, 255),
                        PALETTE.read().unwrap().clone(),
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::IntEditSlider(
                        "organicNodePaletteCount".into(),
                        "Count".into(),
                        "How many palette entries from the base index can be used.".into(),
                        (*count).clamp(1, 16),
                        1..=16,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::Selector(
                        "organicNodePaletteMode".into(),
                        "Mode".into(),
                        "How palette entries are chosen during painting.".into(),
                        vec!["Fixed".into(), "Random Stroke".into(), "Random Blob".into()],
                        (*mode).clamp(0, 2),
                    ));
                }
                OrganicNodeKind::LineShape {
                    length,
                    width,
                    softness,
                } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeLineLength".into(),
                        "Length".into(),
                        "Directional stroke length in local paint space.".into(),
                        *length,
                        0.1..=4.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeLineWidth".into(),
                        "Width".into(),
                        "Directional stroke width.".into(),
                        *width,
                        0.02..=2.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeLineSoftness".into(),
                        "Softness".into(),
                        "Edge softness for the line shape.".into(),
                        *softness,
                        0.0..=1.0,
                        false,
                    ));
                }
                OrganicNodeKind::PaletteColors {
                    color_1,
                    color_2,
                    color_3,
                    color_4,
                    mode,
                } => {
                    let palette = PALETTE.read().unwrap().clone();
                    nodeui.add_item(TheNodeUIItem::PaletteIndexPicker(
                        "organicNodePaletteColor1".into(),
                        "Color 1".into(),
                        "Primary palette color for this brush.".into(),
                        *color_1,
                        palette.clone(),
                    ));
                    nodeui.add_item(TheNodeUIItem::PaletteIndexPicker(
                        "organicNodePaletteColor2".into(),
                        "Color 2".into(),
                        "Secondary palette color for this brush.".into(),
                        *color_2,
                        palette.clone(),
                    ));
                    nodeui.add_item(TheNodeUIItem::PaletteIndexPicker(
                        "organicNodePaletteColor3".into(),
                        "Color 3".into(),
                        "Third palette color for this brush.".into(),
                        *color_3,
                        palette.clone(),
                    ));
                    nodeui.add_item(TheNodeUIItem::PaletteIndexPicker(
                        "organicNodePaletteColor4".into(),
                        "Color 4".into(),
                        "Fourth palette color for this brush.".into(),
                        *color_4,
                        palette,
                    ));
                    nodeui.add_item(TheNodeUIItem::Selector(
                        "organicNodePaletteColorsMode".into(),
                        "Mode".into(),
                        "How the brush varies across the chosen colors.".into(),
                        vec!["Fixed".into(), "Accent".into(), "Per Dab".into()],
                        (*mode).clamp(0, 2),
                    ));
                }
                OrganicNodeKind::Material { channel } => {
                    nodeui.add_item(TheNodeUIItem::Selector(
                        "organicNodeMaterialChannel".into(),
                        "Channel".into(),
                        "Legacy material channel node.".into(),
                        vec![
                            "Foliage".into(),
                            "Soil".into(),
                            "Stone".into(),
                            "Accent".into(),
                        ],
                        (*channel).clamp(0, 3),
                    ));
                }
                OrganicNodeKind::OutputPaint {
                    radius,
                    flow,
                    jitter,
                    depth,
                    cell_size,
                    channel,
                    ..
                }
                | OrganicNodeKind::OutputGrowth {
                    radius,
                    flow,
                    jitter,
                    depth,
                    cell_size,
                    channel,
                    ..
                }
                | OrganicNodeKind::OutputPath {
                    radius,
                    flow,
                    jitter,
                    depth,
                    cell_size,
                    channel,
                    ..
                } => {
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeOutputRadius".into(),
                        "Radius".into(),
                        "Brush radius in local paint space.".into(),
                        *radius,
                        0.05..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeOutputFlow".into(),
                        "Flow".into(),
                        "Per-stroke deposition strength.".into(),
                        *flow,
                        0.05..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeOutputJitter".into(),
                        "Jitter".into(),
                        "Adds slight irregularity to each dab.".into(),
                        *jitter,
                        0.0..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeOutputDepth".into(),
                        "Depth".into(),
                        "How far the painted volume grows away from the host.".into(),
                        *depth,
                        0.01..=4.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::FloatEditSlider(
                        "organicNodeOutputCellSize".into(),
                        "Cell Size".into(),
                        "Resolution of deposited volume columns.".into(),
                        *cell_size,
                        0.05..=1.0,
                        false,
                    ));
                    nodeui.add_item(TheNodeUIItem::Selector(
                        "organicNodeOutputChannel".into(),
                        "Channel".into(),
                        "Target host material channel for this brush output.".into(),
                        vec![
                            "Foliage".into(),
                            "Soil".into(),
                            "Stone".into(),
                            "Accent".into(),
                        ],
                        (*channel).clamp(0, 3),
                    ));
                }
            }
            nodeui.add_item(TheNodeUIItem::CloseTree);
        }

        if let Some(layout) = ui.get_text_layout(ORGANIC_SETTINGS_LAYOUT) {
            layout.clear();
            nodeui.apply_to_text_layout(layout);
            ctx.ui.relayout = true;
        }
    }

    fn add_node(
        &mut self,
        kind: OrganicNodeKind,
        ui: &mut TheUI,
        project: &mut Project,
        server_ctx: &ServerContext,
    ) {
        let pos = Vec2::new(
            self.graph.scroll_offset.x + 220,
            self.graph.scroll_offset.y + 80,
        );
        self.graph.nodes.push(OrganicBrushNode::new(kind, pos));
        self.graph.selected_node = Some(self.graph.nodes.len() - 1);
        self.sync_canvas(ui);
        self.save_state_to_map(project, server_ctx);
    }
}

impl Dock for OrganicDock {
    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            graph: OrganicBrushGraph::default(),
            active_graph_id: Uuid::nil(),
            categories: Self::default_categories(),
        }
    }

    fn setup(&mut self, _ctx: &mut TheContext) -> TheCanvas {
        let mut canvas = TheCanvas::new();

        let mut toolbar_canvas = TheCanvas::default();
        toolbar_canvas.set_widget(TheTraybar::new(TheId::empty()));
        let mut toolbar_hlayout = TheHLayout::new(TheId::empty());
        toolbar_hlayout.set_background_color(None);
        toolbar_hlayout.set_margin(Vec4::new(10, 1, 5, 1));
        toolbar_hlayout.set_padding(3);

        let mut surface_button = TheTraybarButton::new(TheId::named(ORGANIC_SURFACE_BUTTON));
        surface_button.set_text("Surface".to_string());
        surface_button.set_custom_color(Some(TheColor::from_u8_array_3([212, 180, 92])));
        surface_button.set_status_text("Add surface and host-context nodes.");
        surface_button.set_context_menu(Some(TheContextMenu {
            items: vec![TheContextMenuItem::new(
                "Surface Input".into(),
                TheId::named("Organic Add Surface Input"),
            )],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(surface_button));

        let mut shape_button = TheTraybarButton::new(TheId::named(ORGANIC_SHAPE_BUTTON));
        shape_button.set_text("Shape".to_string());
        shape_button.set_custom_color(Some(TheColor::from_u8_array_3([87, 150, 224])));
        shape_button.set_status_text("Add base brush-shape nodes.");
        shape_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Blob Shape".into(), TheId::named("Organic Add Circle")),
                TheContextMenuItem::new("Bush Shape".into(), TheId::named("Organic Add Bush")),
                TheContextMenuItem::new("Canopy Shape".into(), TheId::named("Organic Add Canopy")),
                TheContextMenuItem::new(
                    "Line Shape".into(),
                    TheId::named("Organic Add Line Shape"),
                ),
                TheContextMenuItem::new("Noise".into(), TheId::named("Organic Add Noise")),
                TheContextMenuItem::new(
                    "Growth Profile".into(),
                    TheId::named("Organic Add Height"),
                ),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(shape_button));

        let mut placement_button = TheTraybarButton::new(TheId::named(ORGANIC_PLACEMENT_BUTTON));
        placement_button.set_text("Placement".to_string());
        placement_button.set_custom_color(Some(TheColor::from_u8_array_3([86, 180, 120])));
        placement_button.set_status_text("Add repetition and breakup nodes.");
        placement_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Repeat".into(), TheId::named("Organic Add Scatter")),
                TheContextMenuItem::new("Breakup Noise".into(), TheId::named("Organic Add Noise")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(placement_button));

        let mut growth_button = TheTraybarButton::new(TheId::named(ORGANIC_GROWTH_BUTTON));
        growth_button.set_text("Growth".to_string());
        growth_button.set_custom_color(Some(TheColor::from_u8_array_3([100, 166, 168])));
        growth_button.set_status_text("Add growth and taper nodes.");
        growth_button.set_context_menu(Some(TheContextMenu {
            items: vec![TheContextMenuItem::new(
                "Growth Profile".into(),
                TheId::named("Organic Add Height"),
            )],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(growth_button));

        let mut output_button = TheTraybarButton::new(TheId::named(ORGANIC_OUTPUT_BUTTON));
        output_button.set_text("Output".to_string());
        output_button.set_custom_color(Some(TheColor::from_u8_array_3([214, 134, 96])));
        output_button.set_status_text("Add final output nodes.");
        output_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new(
                    "Palette Colors".into(),
                    TheId::named("Organic Add Palette Colors"),
                ),
                TheContextMenuItem::new(
                    "Paint Output".into(),
                    TheId::named("Organic Add Paint Output"),
                ),
                TheContextMenuItem::new(
                    "Growth Output".into(),
                    TheId::named("Organic Add Growth Output"),
                ),
                TheContextMenuItem::new(
                    "Path Output".into(),
                    TheId::named("Organic Add Path Output"),
                ),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(output_button));

        let mut reset_button = TheTraybarButton::new(TheId::named(ORGANIC_RESET_BUTTON));
        reset_button.set_text("Reset".to_string());
        reset_button.set_status_text("Reset the organic brush graph to the default chain.");
        toolbar_hlayout.add_widget(Box::new(reset_button));

        let mut title = TheText::new(TheId::empty());
        title.set_text("Organic Brush Graph".into());
        toolbar_hlayout.add_widget(Box::new(title));

        let mut preset_button = TheTraybarButton::new(TheId::named(ORGANIC_PRESET_BUTTON));
        preset_button.set_text("Presets".to_string());
        preset_button.set_status_text("Load a usable organic brush preset.");
        preset_button.set_context_menu(Some(TheContextMenu {
            items: vec![
                TheContextMenuItem::new("Paint: Moss".into(), TheId::named("Organic Preset Moss")),
                TheContextMenuItem::new("Paint: Mud".into(), TheId::named("Organic Preset Mud")),
                TheContextMenuItem::new(
                    "Paint: Grass".into(),
                    TheId::named("Organic Preset Grass"),
                ),
                TheContextMenuItem::new(
                    "Path: Vines".into(),
                    TheId::named("Organic Preset Path Vines"),
                ),
                TheContextMenuItem::new("Growth: Bush".into(), TheId::named("Organic Preset Bush")),
                TheContextMenuItem::new("Growth: Tree".into(), TheId::named("Organic Preset Tree")),
            ],
            ..Default::default()
        }));
        toolbar_hlayout.add_widget(Box::new(preset_button));

        toolbar_hlayout.set_reverse_index(Some(2));
        toolbar_canvas.set_layout(toolbar_hlayout);
        canvas.set_top(toolbar_canvas);

        let mut center = TheCanvas::new();

        let mut node_canvas = TheCanvas::new();
        node_canvas.set_widget(TheNodeCanvasView::new(TheId::named(ORGANIC_CANVAS_VIEW)));
        center.set_center(node_canvas);

        let mut settings_canvas = TheCanvas::default();
        let mut text_layout = TheTextLayout::new(TheId::named(ORGANIC_SETTINGS_LAYOUT));
        text_layout.limiter_mut().set_max_width(290);
        text_layout.set_text_margin(20);
        text_layout.set_text_align(TheHorizontalAlign::Right);
        settings_canvas.set_layout(text_layout);
        center.set_right(settings_canvas);

        canvas.set_center(center);

        canvas
    }

    fn activate(
        &mut self,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &Project,
        server_ctx: &mut ServerContext,
    ) {
        self.load_state_from_map(project, server_ctx);
        self.sync_canvas(ui);
        self.sync_preview_overlay(ui);
        self.set_selected_node_ui(project, ui, ctx, server_ctx);
    }

    fn supports_actions(&self) -> bool {
        false
    }

    fn default_state(&self) -> DockDefaultState {
        DockDefaultState::Minimized
    }

    fn maximized_state(&self) -> DockMaximizedState {
        DockMaximizedState::Maximized
    }

    fn handle_event(
        &mut self,
        event: &TheEvent,
        ui: &mut TheUI,
        ctx: &mut TheContext,
        project: &mut Project,
        server_ctx: &mut ServerContext,
    ) -> bool {
        match event {
            TheEvent::ContextMenuSelected(id, item)
                if id.name == ORGANIC_SURFACE_BUTTON
                    || id.name == ORGANIC_SHAPE_BUTTON
                    || id.name == ORGANIC_PLACEMENT_BUTTON
                    || id.name == ORGANIC_GROWTH_BUTTON
                    || id.name == ORGANIC_OUTPUT_BUTTON =>
            {
                let kind = match item.name.as_str() {
                    "Organic Add Surface Input" => OrganicNodeKind::SurfaceInput,
                    "Organic Add Circle" => OrganicNodeKind::CircleMask {
                        radius: 0.55,
                        softness: 0.35,
                    },
                    "Organic Add Bush" => OrganicNodeKind::BushShape {
                        radius: 0.34,
                        height: 1.15,
                        layers: 5,
                        taper: 0.58,
                        breakup: 0.22,
                        softness: 0.72,
                    },
                    "Organic Add Canopy" => OrganicNodeKind::CanopyShape {
                        radius: 0.45,
                        lobes: 6,
                        spread: 0.55,
                        softness: 0.65,
                    },
                    "Organic Add Line Shape" => OrganicNodeKind::LineShape {
                        length: 1.2,
                        width: 0.18,
                        softness: 0.35,
                    },
                    "Organic Add Noise" => OrganicNodeKind::Noise {
                        scale: 0.3,
                        strength: 0.25,
                        seed: 1,
                    },
                    "Organic Add Scatter" => OrganicNodeKind::Scatter {
                        count: 6,
                        jitter: 0.4,
                    },
                    "Organic Add Height" => OrganicNodeKind::HeightProfile {
                        depth: 0.4,
                        falloff: 0.5,
                    },
                    "Organic Add Palette Colors" => OrganicNodeKind::PaletteColors {
                        color_1: 0,
                        color_2: 1,
                        color_3: 2,
                        color_4: 3,
                        mode: 1,
                    },
                    "Organic Add Paint Output" => OrganicNodeKind::OutputPaint {
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
                    "Organic Add Growth Output" => OrganicNodeKind::OutputGrowth {
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
                    "Organic Add Path Output" => OrganicNodeKind::OutputPath {
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
                    _ => return false,
                };
                self.add_node(kind, ui, project, server_ctx);
                self.sync_preview_overlay(ui);
                self.set_selected_node_ui(project, ui, ctx, server_ctx);
                true
            }
            TheEvent::ContextMenuSelected(id, item) if id.name == ORGANIC_PRESET_BUTTON => {
                let preset = match item.name.as_str() {
                    "Organic Preset Moss" => OrganicPresetKind::Moss,
                    "Organic Preset Mud" => OrganicPresetKind::Mud,
                    "Organic Preset Grass" => OrganicPresetKind::Grass,
                    "Organic Preset Path Vines" => OrganicPresetKind::PathVines,
                    "Organic Preset Bush" => OrganicPresetKind::Bush,
                    "Organic Preset Tree" => OrganicPresetKind::Tree,
                    _ => return false,
                };
                self.graph = match preset {
                    OrganicPresetKind::Moss => OrganicBrushGraph::preset_moss(),
                    OrganicPresetKind::Mud => OrganicBrushGraph::preset_mud(),
                    OrganicPresetKind::Grass => OrganicBrushGraph::preset_grass(),
                    OrganicPresetKind::PathVines => OrganicBrushGraph::preset_path_vines(),
                    OrganicPresetKind::Bush => OrganicBrushGraph::preset_bush(),
                    OrganicPresetKind::Tree => OrganicBrushGraph::preset_tree(),
                };
                Self::retarget_preset_palette(&mut self.graph, project, preset);
                self.active_graph_id = self.graph.id;
                self.sync_canvas(ui);
                self.sync_preview_overlay(ui);
                self.save_state_to_map(project, server_ctx);
                self.set_selected_node_ui(project, ui, ctx, server_ctx);
                true
            }
            TheEvent::StateChanged(id, TheWidgetState::Clicked)
                if id.name == ORGANIC_RESET_BUTTON =>
            {
                self.graph = OrganicBrushGraph::default();
                Self::retarget_preset_palette(&mut self.graph, project, OrganicPresetKind::Moss);
                self.sync_canvas(ui);
                self.sync_preview_overlay(ui);
                self.save_state_to_map(project, server_ctx);
                self.set_selected_node_ui(project, ui, ctx, server_ctx);
                true
            }
            TheEvent::NodeSelectedIndexChanged(id, index) if id.name == ORGANIC_CANVAS_VIEW => {
                self.graph.selected_node = *index;
                self.save_state_to_map(project, server_ctx);
                self.set_selected_node_ui(project, ui, ctx, server_ctx);
                true
            }
            TheEvent::NodeDragged(id, index, position) if id.name == ORGANIC_CANVAS_VIEW => {
                if let Some(node) = self.graph.nodes.get_mut(*index) {
                    node.position = *position;
                    self.save_state_to_map(project, server_ctx);
                    return true;
                }
                false
            }
            TheEvent::NodeConnectionAdded(id, connections)
            | TheEvent::NodeConnectionRemoved(id, connections)
                if id.name == ORGANIC_CANVAS_VIEW =>
            {
                self.graph.connections.clone_from(connections);
                self.save_state_to_map(project, server_ctx);
                true
            }
            TheEvent::NodeDeleted(id, deleted_node_index, connections)
                if id.name == ORGANIC_CANVAS_VIEW =>
            {
                if *deleted_node_index < self.graph.nodes.len() {
                    self.graph.nodes.remove(*deleted_node_index);
                    self.graph.connections.clone_from(connections);
                    self.graph.selected_node = None;
                    self.sync_canvas(ui);
                    self.sync_preview_overlay(ui);
                    self.save_state_to_map(project, server_ctx);
                    self.clear_selected_node_ui(ui, ctx);
                    return true;
                }
                false
            }
            TheEvent::NodeViewScrolled(id, offset) if id.name == ORGANIC_CANVAS_VIEW => {
                self.graph.scroll_offset = *offset;
                self.save_state_to_map(project, server_ctx);
                true
            }
            TheEvent::ValueChanged(id, value) => {
                let mut save_graph = false;

                if let Some(index) = self.graph.selected_node
                    && let Some(node) = self.graph.nodes.get_mut(index)
                {
                    match (&mut node.kind, id.name.as_str(), value) {
                        (
                            OrganicNodeKind::CircleMask { radius, .. },
                            "organicNodeCircleRadius",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *radius = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::CircleMask { softness, .. },
                            "organicNodeCircleSoftness",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *softness = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::CanopyShape { radius, .. },
                            "organicNodeCanopyRadius",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *radius = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::CanopyShape { lobes, .. },
                            "organicNodeCanopyLobes",
                            TheValue::IntRange(v, _),
                        ) => {
                            *lobes = (*v).clamp(3, 10);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::CanopyShape { spread, .. },
                            "organicNodeCanopySpread",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *spread = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::CanopyShape { softness, .. },
                            "organicNodeCanopySoftness",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *softness = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::BushShape { radius, .. },
                            "organicNodeBushRadius",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *radius = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::BushShape { height, .. },
                            "organicNodeBushHeight",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *height = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::BushShape { layers, .. },
                            "organicNodeBushLayers",
                            TheValue::IntRange(v, _),
                        ) => {
                            *layers = (*v).clamp(2, 8);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::BushShape { taper, .. },
                            "organicNodeBushTaper",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *taper = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::BushShape { breakup, .. },
                            "organicNodeBushBreakup",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *breakup = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::BushShape { softness, .. },
                            "organicNodeBushSoftness",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *softness = *v;
                            save_graph = true;
                        }
                        (OrganicNodeKind::LineShape { length, .. }, "organicNodeLineLength", v) => {
                            if let Some(new_value) = v.to_f32() {
                                *length = new_value.clamp(0.1, 4.0);
                                save_graph = true;
                            }
                        }
                        (OrganicNodeKind::LineShape { width, .. }, "organicNodeLineWidth", v) => {
                            if let Some(new_value) = v.to_f32() {
                                *width = new_value.clamp(0.02, 2.0);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::LineShape { softness, .. },
                            "organicNodeLineSoftness",
                            v,
                        ) => {
                            if let Some(new_value) = v.to_f32() {
                                *softness = new_value.clamp(0.0, 1.0);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::Noise { scale, .. },
                            "organicNodeNoiseScale",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *scale = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::Noise { strength, .. },
                            "organicNodeNoiseStrength",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *strength = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::Noise { seed, .. },
                            "organicNodeNoiseSeed",
                            TheValue::IntRange(v, _),
                        ) => {
                            *seed = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::Scatter { count, .. },
                            "organicNodeScatterCount",
                            TheValue::IntRange(v, _),
                        ) => {
                            *count = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::Scatter { jitter, .. },
                            "organicNodeScatterJitter",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *jitter = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::HeightProfile { depth, .. },
                            "organicNodeHeightDepth",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *depth = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::HeightProfile { falloff, .. },
                            "organicNodeHeightFalloff",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *falloff = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::PaletteColors { color_1, .. },
                            "organicNodePaletteColor1",
                            v,
                        ) => {
                            if let Some(new_index) = v.to_i32() {
                                *color_1 = new_index.clamp(0, 255);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::PaletteColors { color_2, .. },
                            "organicNodePaletteColor2",
                            v,
                        ) => {
                            if let Some(new_index) = v.to_i32() {
                                *color_2 = new_index.clamp(0, 255);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::PaletteColors { color_3, .. },
                            "organicNodePaletteColor3",
                            v,
                        ) => {
                            if let Some(new_index) = v.to_i32() {
                                *color_3 = new_index.clamp(0, 255);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::PaletteColors { color_4, .. },
                            "organicNodePaletteColor4",
                            v,
                        ) => {
                            if let Some(new_index) = v.to_i32() {
                                *color_4 = new_index.clamp(0, 255);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::PaletteColors { mode, .. },
                            "organicNodePaletteColorsMode",
                            TheValue::Int(v),
                        ) => {
                            *mode = (*v).clamp(0, 2);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { radius, .. }
                            | OrganicNodeKind::OutputGrowth { radius, .. }
                            | OrganicNodeKind::OutputPath { radius, .. },
                            "organicNodeOutputRadius",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *radius = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { flow, .. }
                            | OrganicNodeKind::OutputGrowth { flow, .. }
                            | OrganicNodeKind::OutputPath { flow, .. },
                            "organicNodeOutputFlow",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *flow = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { jitter, .. }
                            | OrganicNodeKind::OutputGrowth { jitter, .. }
                            | OrganicNodeKind::OutputPath { jitter, .. },
                            "organicNodeOutputJitter",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *jitter = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { depth, .. }
                            | OrganicNodeKind::OutputGrowth { depth, .. }
                            | OrganicNodeKind::OutputPath { depth, .. },
                            "organicNodeOutputDepth",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *depth = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { cell_size, .. }
                            | OrganicNodeKind::OutputGrowth { cell_size, .. }
                            | OrganicNodeKind::OutputPath { cell_size, .. },
                            "organicNodeOutputCellSize",
                            TheValue::FloatRange(v, _),
                        ) => {
                            *cell_size = *v;
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { channel, .. }
                            | OrganicNodeKind::OutputGrowth { channel, .. }
                            | OrganicNodeKind::OutputPath { channel, .. },
                            "organicNodeOutputChannel",
                            TheValue::Int(v),
                        ) => {
                            *channel = (*v).clamp(0, 3);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { palette_start, .. }
                            | OrganicNodeKind::OutputGrowth { palette_start, .. }
                            | OrganicNodeKind::OutputPath { palette_start, .. },
                            "organicNodeOutputPaletteStart",
                            v,
                        ) => {
                            if let Some(new_index) = v.to_i32() {
                                *palette_start = new_index.clamp(0, 255);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::OutputPaint { palette_count, .. }
                            | OrganicNodeKind::OutputGrowth { palette_count, .. }
                            | OrganicNodeKind::OutputPath { palette_count, .. },
                            "organicNodeOutputPaletteCount",
                            TheValue::IntRange(v, _),
                        ) => {
                            *palette_count = (*v).clamp(1, 16);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::OutputPaint { palette_mode, .. }
                            | OrganicNodeKind::OutputGrowth { palette_mode, .. }
                            | OrganicNodeKind::OutputPath { palette_mode, .. },
                            "organicNodeOutputPaletteMode",
                            TheValue::Int(v),
                        ) => {
                            *palette_mode = (*v).clamp(0, 2);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::PaletteRange { start, .. },
                            "organicNodePaletteStart",
                            v,
                        ) => {
                            if let Some(new_index) = v.to_i32() {
                                *start = new_index.clamp(0, 255);
                                save_graph = true;
                            }
                        }
                        (
                            OrganicNodeKind::PaletteRange { count, .. },
                            "organicNodePaletteCount",
                            TheValue::IntRange(v, _),
                        ) => {
                            *count = (*v).clamp(1, 16);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::PaletteRange { mode, .. },
                            "organicNodePaletteMode",
                            TheValue::Int(v),
                        ) => {
                            *mode = (*v).clamp(0, 2);
                            save_graph = true;
                        }
                        (
                            OrganicNodeKind::Material { channel, .. },
                            "organicNodeMaterialChannel",
                            TheValue::Int(v),
                        ) => {
                            *channel = (*v).clamp(0, 3);
                            save_graph = true;
                        }
                        _ => {}
                    }
                }
                if save_graph {
                    self.save_state_to_map(project, server_ctx);
                    self.sync_preview_overlay(ui);
                }
                save_graph
            }
            _ => false,
        }
    }
}
