use rayon::prelude::*;
use rustc_hash::FxHashSet;
use std::sync::RwLock;
use theframework::prelude::*;

#[inline(always)]
fn hash21(p: Vec2<f32>) -> f32 {
    let mut p3 = Vec3::new(
        (p.x * 0.1031).fract(),
        (p.y * 0.1031).fract(),
        (p.x * 0.1031).fract(),
    );
    let dot = p3.dot(Vec3::new(p3.y + 33.333, p3.z + 33.333, p3.x + 33.333));

    p3.x += dot;
    p3.y += dot;
    p3.z += dot;

    ((p3.x + p3.y) * p3.z).fract()
}

fn rot(a: f32) -> Mat2<f32> {
    Mat2::new(a.cos(), -a.sin(), a.sin(), a.cos())
}

fn box_divide(
    p: Vec2<f32>,
    cell: Vec2<f32>,
    gap: f32,
    rotation: f32,
    rounding: f32,
    iterations: i32,
) -> (f32, f32) {
    fn s_box(p: Vec2<f32>, b: Vec2<f32>, r: f32) -> f32 {
        let d = p.map(|v| v.abs()) - b + Vec2::new(r, r);
        d.x.max(d.y).min(0.0) + (d.map(|v| v.max(0.0))).magnitude() - r
    }

    let mut p = p;

    let mut l = Vec2::new(1.0, 1.0);
    let mut last_l;
    let mut r = hash21(cell);

    for _ in 0..iterations.max(1) {
        r = (l + Vec2::new(r, r)).dot(Vec2::new(123.71, 439.43)).fract() * 0.4 + 0.3;

        last_l = l;
        if l.x > l.y {
            p = Vec2::new(p.y, p.x);
            l = Vec2::new(l.y, l.x);
        }

        if p.x < r {
            l.x /= r;
            p.x /= r;
        } else {
            l.x /= 1.0 - r;
            p.x = (p.x - r) / (1.0 - r);
        }

        if last_l.x > last_l.y {
            p = Vec2::new(p.y, p.x);
            l = Vec2::new(l.y, l.x);
        }
    }
    p -= 0.5;

    let id = hash21(cell + l);
    p = rot((id - 0.5) * rotation) * p;

    let th = l * 0.02 * gap;
    let c = s_box(p, Vec2::new(0.5, 0.5) - th, rounding);

    (c, id)
}

fn default_tile_node_nodes() -> Vec<TileNodeState> {
    vec![TileNodeState {
        kind: TileNodeKind::default_output_root(),
        position: (420, 40),
        preview_open: true,
        bypass: false,
        mute: false,
        solo: false,
    }]
}

fn default_voronoi_warp_amount() -> f32 {
    0.0
}

fn default_voronoi_falloff() -> f32 {
    1.0
}

fn default_layout_warp_amount() -> f32 {
    0.0
}

fn default_layout_falloff() -> f32 {
    1.0
}

fn default_height_shape_rim() -> f32 {
    0.0
}

fn default_brick_staggered() -> bool {
    true
}

fn default_colorize4_color_1() -> u16 {
    0
}

fn default_colorize4_color_2() -> u16 {
    1
}

fn default_colorize4_color_3() -> u16 {
    2
}

fn default_colorize4_color_4() -> u16 {
    3
}

fn default_colorize4_pixel_size() -> u16 {
    1
}

fn default_colorize4_dither() -> bool {
    false
}

fn default_colorize4_auto_range() -> bool {
    true
}

fn default_particle_color_1() -> u16 {
    0
}

fn default_particle_color_2() -> u16 {
    1
}

fn default_particle_color_3() -> u16 {
    2
}

fn default_particle_color_4() -> u16 {
    3
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TileNodeGraphState {
    #[serde(default = "default_tile_node_nodes")]
    pub nodes: Vec<TileNodeState>,
    #[serde(default)]
    pub connections: Vec<(u16, u8, u16, u8)>,
    #[serde(default)]
    pub offset: (i32, i32),
    #[serde(default)]
    pub selected_node: Option<usize>,
    #[serde(default)]
    pub preview_mode: u8,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TileGraphPaletteSource {
    #[default]
    Local,
    Project,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TileNodeGraphExchange {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub graph_name: String,
    #[serde(default)]
    pub palette_source: TileGraphPaletteSource,
    #[serde(default)]
    pub palette_colors: Vec<TheColor>,
    pub output_grid_width: u16,
    pub output_grid_height: u16,
    pub tile_pixel_width: u16,
    pub tile_pixel_height: u16,
    #[serde(default)]
    pub graph_state: TileNodeGraphState,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TileParticleOutput {
    pub rate: f32,
    pub spread: f32,
    pub lifetime_min: f32,
    pub lifetime_max: f32,
    pub radius_min: f32,
    pub radius_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub flame_base: bool,
    pub color_variation: u8,
    pub ramp_colors: [[u8; 4]; 4],
}

#[derive(Clone, Debug)]
pub struct TileLightOutput {
    pub intensity: f32,
    pub range: f32,
    pub flicker: f32,
    pub lift: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TileNodeState {
    pub kind: TileNodeKind,
    pub position: (i32, i32),
    #[serde(default = "default_node_preview_open")]
    pub preview_open: bool,
    #[serde(default)]
    pub bypass: bool,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub solo: bool,
}

impl Default for TileNodeState {
    fn default() -> Self {
        Self {
            kind: TileNodeKind::default_output_root(),
            position: (420, 40),
            preview_open: true,
            bypass: false,
            mute: false,
            solo: false,
        }
    }
}

fn default_node_preview_open() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TileNodeKind {
    OutputRoot {
        #[serde(default = "default_output_roughness")]
        roughness: f32,
        #[serde(default = "default_output_metallic")]
        metallic: f32,
        #[serde(default = "default_output_opacity")]
        opacity: f32,
        #[serde(default = "default_output_emissive")]
        emissive: f32,
        #[serde(default = "default_output_particle_enabled")]
        particle_enabled: bool,
        #[serde(default = "default_output_light_enabled")]
        light_enabled: bool,
    },
    LayerInput {
        name: String,
        value: f32,
    },
    ImportLayer {
        source: String,
    },
    GroupUV,
    Scalar {
        value: f32,
    },
    Colorize4 {
        #[serde(default = "default_colorize4_color_1")]
        color_1: u16,
        #[serde(default = "default_colorize4_color_2")]
        color_2: u16,
        #[serde(default = "default_colorize4_color_3")]
        color_3: u16,
        #[serde(default = "default_colorize4_color_4")]
        color_4: u16,
        #[serde(default = "default_colorize4_pixel_size")]
        pixel_size: u16,
        #[serde(default = "default_colorize4_dither")]
        dither: bool,
        #[serde(default = "default_colorize4_auto_range")]
        auto_range: bool,
    },
    Color {
        color: TheColor,
    },
    PaletteColor {
        index: u16,
    },
    NearestPalette,
    Mix {
        factor: f32,
    },
    Checker {
        scale: u16,
    },
    Gradient {
        mode: u8,
    },
    Curve {
        power: f32,
    },
    Noise {
        scale: f32,
        seed: u32,
        wrap: bool,
    },
    Voronoi {
        scale: f32,
        seed: u32,
        jitter: f32,
        #[serde(default = "default_voronoi_warp_amount")]
        warp_amount: f32,
        #[serde(default = "default_voronoi_falloff")]
        falloff: f32,
    },
    BoxDivide {
        scale: f32,
        gap: f32,
        rotation: f32,
        rounding: f32,
        #[serde(default = "default_layout_warp_amount")]
        warp_amount: f32,
        #[serde(default = "default_layout_falloff")]
        falloff: f32,
    },
    Offset {
        x: f32,
        y: f32,
    },
    Scale {
        x: f32,
        y: f32,
    },
    Repeat {
        repeat_x: f32,
        repeat_y: f32,
    },
    Rotate {
        angle: f32,
    },
    DirectionalWarp {
        amount: f32,
        angle: f32,
    },
    Brick {
        columns: u16,
        rows: u16,
        #[serde(default = "default_brick_staggered")]
        staggered: bool,
        offset: f32,
        #[serde(default = "default_layout_warp_amount")]
        warp_amount: f32,
        #[serde(default = "default_layout_falloff")]
        falloff: f32,
    },
    Disc {
        scale: f32,
        seed: u32,
        jitter: f32,
        radius: f32,
        #[serde(default = "default_layout_warp_amount")]
        warp_amount: f32,
        #[serde(default = "default_layout_falloff")]
        falloff: f32,
    },
    IdRandom,
    Min,
    Max,
    Add,
    Subtract,
    Multiply,
    MakeMaterial,
    Material {
        roughness: f32,
        metallic: f32,
        opacity: f32,
        emissive: f32,
    },
    MaterialMix {
        factor: f32,
    },
    ParticleEmitter {
        rate: f32,
        spread: f32,
        lifetime_min: f32,
        lifetime_max: f32,
        radius_min: f32,
        radius_max: f32,
        speed_min: f32,
        speed_max: f32,
        color_variation: u8,
    },
    ParticleSpawn {
        rate: f32,
        spread: f32,
    },
    ParticleMotion {
        lifetime_min: f32,
        lifetime_max: f32,
        speed_min: f32,
        speed_max: f32,
    },
    ParticleRender {
        radius_min: f32,
        radius_max: f32,
        #[serde(default)]
        flame_base: bool,
        color_variation: u8,
        #[serde(default = "default_particle_color_1")]
        color_1: u16,
        #[serde(default = "default_particle_color_2")]
        color_2: u16,
        #[serde(default = "default_particle_color_3")]
        color_3: u16,
        #[serde(default = "default_particle_color_4")]
        color_4: u16,
    },
    LightEmitter {
        intensity: f32,
        range: f32,
        flicker: f32,
        lift: f32,
    },
    MaskBlend {
        factor: f32,
    },
    Levels {
        level: f32,
        width: f32,
    },
    HeightShape {
        contrast: f32,
        bias: f32,
        plateau: f32,
        #[serde(default = "default_height_shape_rim")]
        rim: f32,
        #[serde(default = "default_layout_warp_amount")]
        warp_amount: f32,
    },
    Threshold {
        cutoff: f32,
    },
    Blur {
        radius: f32,
    },
    SlopeBlur {
        radius: f32,
        amount: f32,
    },
    HeightEdge {
        radius: f32,
    },
    Warp {
        amount: f32,
    },
    Invert,
}

impl Default for TileNodeKind {
    fn default() -> Self {
        Self::default_output_root()
    }
}

fn default_output_roughness() -> f32 {
    0.9
}

fn default_output_metallic() -> f32 {
    0.0
}

fn default_output_opacity() -> f32 {
    1.0
}

fn default_output_emissive() -> f32 {
    0.0
}

fn default_output_particle_enabled() -> bool {
    true
}

fn default_output_light_enabled() -> bool {
    true
}

impl TileNodeKind {
    pub fn default_output_root() -> Self {
        Self::OutputRoot {
            roughness: default_output_roughness(),
            metallic: default_output_metallic(),
            opacity: default_output_opacity(),
            emissive: default_output_emissive(),
            particle_enabled: default_output_particle_enabled(),
            light_enabled: default_output_light_enabled(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TileEvalContext {
    pub cell_x: u16,
    pub cell_y: u16,
    pub group_width: u16,
    pub group_height: u16,
    pub tile_pixel_width: u16,
    pub tile_pixel_height: u16,
    pub u: f32,
    pub v: f32,
}

impl TileEvalContext {
    pub fn group_u(&self) -> f32 {
        ((self.cell_x as f32) + self.u) / (self.group_width.max(1) as f32)
    }

    pub fn group_v(&self) -> f32 {
        ((self.cell_y as f32) + self.v) / (self.group_height.max(1) as f32)
    }

    pub fn with_group_uv(&self, group_u: f32, group_v: f32) -> Self {
        let width = self.group_width.max(1) as f32;
        let height = self.group_height.max(1) as f32;
        let gx = group_u.clamp(0.0, 0.999_999) * width;
        let gy = group_v.clamp(0.0, 0.999_999) * height;
        let cell_x = gx.floor() as u16;
        let cell_y = gy.floor() as u16;
        Self {
            cell_x,
            cell_y,
            group_width: self.group_width,
            group_height: self.group_height,
            tile_pixel_width: self.tile_pixel_width,
            tile_pixel_height: self.tile_pixel_height,
            u: gx.fract(),
            v: gy.fract(),
        }
    }
}

impl TileNodeGraphState {
    pub fn from_graph_data(graph_data: &str) -> Self {
        let mut state = serde_json::from_str::<TileNodeGraphState>(graph_data).unwrap_or_default();
        state.ensure_root();
        state
    }

    pub fn ensure_root(&mut self) {
        if self.nodes.is_empty() {
            self.nodes = default_tile_node_nodes();
        } else if !matches!(
            self.nodes.first().map(|n| &n.kind),
            Some(TileNodeKind::OutputRoot { .. })
        ) {
            self.nodes.insert(
                0,
                TileNodeState {
                    kind: TileNodeKind::default_output_root(),
                    position: (420, 40),
                    preview_open: true,
                    bypass: false,
                    mute: false,
                    solo: false,
                },
            );
        }
    }
}

impl TileNodeGraphExchange {
    pub fn new(
        graph_name: String,
        output_grid_width: u16,
        output_grid_height: u16,
        tile_pixel_width: u16,
        tile_pixel_height: u16,
        graph_state: TileNodeGraphState,
    ) -> Self {
        Self {
            version: 1,
            graph_name,
            palette_source: TileGraphPaletteSource::Local,
            palette_colors: vec![],
            output_grid_width,
            output_grid_height,
            tile_pixel_width,
            tile_pixel_height,
            graph_state,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderedTileGraph {
    pub grid_width: usize,
    pub grid_height: usize,
    pub tile_width: usize,
    pub tile_height: usize,
    pub sheet_color: Vec<u8>,
    pub sheet_material: Vec<u8>,
    pub sheet_height: Vec<u8>,
    pub tiles_color: Vec<Vec<u8>>,
    pub tiles_material: Vec<Vec<u8>>,
    pub tiles_height: Vec<Vec<u8>>,
    pub particle_output: Option<TileParticleOutput>,
    pub light_output: Option<TileLightOutput>,
}

pub trait TileGraphSubgraphResolver {
    fn resolve_subgraph_state(&self, source: &str) -> Option<TileNodeGraphState>;

    fn resolve_subgraph_exchange(&self, source: &str) -> Option<TileNodeGraphExchange> {
        self.resolve_subgraph_state(source)
            .map(|graph_state| TileNodeGraphExchange {
                version: 1,
                graph_name: String::new(),
                palette_source: TileGraphPaletteSource::Local,
                palette_colors: Vec::new(),
                output_grid_width: 1,
                output_grid_height: 1,
                tile_pixel_width: 32,
                tile_pixel_height: 32,
                graph_state,
            })
    }
}

pub struct NoTileGraphSubgraphs;

impl TileGraphSubgraphResolver for NoTileGraphSubgraphs {
    fn resolve_subgraph_state(&self, _source: &str) -> Option<TileNodeGraphState> {
        None
    }
}

#[derive(Clone, Copy, Default)]
struct FlatSubgraphOutputs {
    outputs: [Option<u16>; 8],
}

#[derive(Clone, Default)]
struct FlatSubgraphInputs {
    inputs: Vec<Option<u16>>,
}

pub fn flatten_graph_exchange_with<R: TileGraphSubgraphResolver>(
    graph: &TileNodeGraphExchange,
    resolver: &R,
) -> TileNodeGraphExchange {
    let mut flattened = graph.clone();
    flattened.graph_state = flatten_graph_state_recursive(
        &graph.graph_state,
        resolver,
        &mut FxHashSet::default(),
        graph.palette_source,
        &graph.palette_colors,
    );
    flattened
}

pub fn flatten_graph_state_with<R: TileGraphSubgraphResolver>(
    state: &TileNodeGraphState,
    resolver: &R,
) -> TileNodeGraphState {
    let mut state = state.clone();
    state.ensure_root();
    flatten_graph_state_recursive(
        &state,
        resolver,
        &mut FxHashSet::default(),
        TileGraphPaletteSource::Local,
        &[],
    )
}

fn flatten_graph_state_recursive<R: TileGraphSubgraphResolver>(
    state: &TileNodeGraphState,
    resolver: &R,
    stack: &mut FxHashSet<String>,
    target_palette_source: TileGraphPaletteSource,
    target_palette: &[TheColor],
) -> TileNodeGraphState {
    let mut nodes = Vec::new();
    let mut node_map: Vec<Option<u16>> = vec![None; state.nodes.len()];
    let mut subgraph_outputs: Vec<FlatSubgraphOutputs> =
        vec![FlatSubgraphOutputs::default(); state.nodes.len()];
    let mut subgraph_inputs: Vec<FlatSubgraphInputs> =
        vec![FlatSubgraphInputs::default(); state.nodes.len()];
    let mut connections = Vec::new();

    if let Some(root) = state.nodes.first() {
        nodes.push(root.clone());
        node_map[0] = Some(0);
    }

    for (old_index, node) in state.nodes.iter().enumerate().skip(1) {
        match &node.kind {
            TileNodeKind::ImportLayer { source } => {
                if !stack.insert(source.clone()) {
                    continue;
                }
                let Some(mut sub_exchange) = resolver.resolve_subgraph_exchange(source) else {
                    stack.remove(source);
                    continue;
                };
                sub_exchange.graph_state.ensure_root();
                remap_exchange_palette_for_instancing(
                    &mut sub_exchange,
                    target_palette_source,
                    target_palette,
                );
                let sub_flat = flatten_graph_state_recursive(
                    &sub_exchange.graph_state,
                    resolver,
                    stack,
                    target_palette_source,
                    target_palette,
                );
                stack.remove(source);

                let base = nodes.len() as u16;
                let mut sub_map: Vec<Option<u16>> = vec![None; sub_flat.nodes.len()];
                let mut input_slots = Vec::new();
                for (sub_index, sub_node) in sub_flat.nodes.iter().enumerate().skip(1) {
                    let new_index = nodes.len() as u16;
                    nodes.push(sub_node.clone());
                    sub_map[sub_index] = Some(new_index);
                    if matches!(sub_node.kind, TileNodeKind::LayerInput { .. }) {
                        input_slots.push(Some(new_index));
                    }
                }

                let mut outputs = [None; 8];
                for (terminal, slot) in outputs.iter_mut().enumerate() {
                    *slot = input_connection_source(&sub_flat, 0, terminal as u8)
                        .and_then(|src| remap_sub_index(src, &sub_map, base));
                }
                subgraph_outputs[old_index] = FlatSubgraphOutputs { outputs };
                subgraph_inputs[old_index] = FlatSubgraphInputs {
                    inputs: input_slots,
                };

                for (src_node, src_terminal, dest_node, dest_terminal) in &sub_flat.connections {
                    if *src_node == 0 || *dest_node == 0 {
                        continue;
                    }
                    if let (Some(src), Some(dest)) =
                        (sub_map[*src_node as usize], sub_map[*dest_node as usize])
                    {
                        connections.push((src, *src_terminal, dest, *dest_terminal));
                    }
                }
            }
            _ => {
                let new_index = nodes.len() as u16;
                nodes.push(node.clone());
                node_map[old_index] = Some(new_index);
            }
        }
    }

    for (src_node, src_terminal, dest_node, dest_terminal) in &state.connections {
        let src = if matches!(
            state.nodes.get(*src_node as usize).map(|n| &n.kind),
            Some(TileNodeKind::ImportLayer { .. })
        ) {
            let outputs = subgraph_outputs[*src_node as usize];
            outputs.outputs.get(*src_terminal as usize).and_then(|v| *v)
        } else {
            node_map.get(*src_node as usize).and_then(|v| *v)
        };
        let dest = if matches!(
            state.nodes.get(*dest_node as usize).map(|n| &n.kind),
            Some(TileNodeKind::ImportLayer { .. })
        ) {
            subgraph_inputs
                .get(*dest_node as usize)
                .and_then(|i| i.inputs.get(*dest_terminal as usize))
                .and_then(|v| *v)
        } else {
            node_map.get(*dest_node as usize).and_then(|v| *v)
        };
        if let (Some(src), Some(dest)) = (src, dest) {
            let target_terminal = if matches!(
                nodes.get(dest as usize).map(|n| &n.kind),
                Some(TileNodeKind::LayerInput { .. })
            ) {
                0
            } else {
                *dest_terminal
            };
            connections.push((src, *src_terminal, dest, target_terminal));
        }
    }

    TileNodeGraphState {
        nodes,
        connections,
        offset: state.offset,
        selected_node: state
            .selected_node
            .and_then(|index| node_map.get(index).and_then(|v| *v).map(|v| v as usize)),
        preview_mode: state.preview_mode,
    }
}

fn remap_exchange_palette_for_instancing(
    exchange: &mut TileNodeGraphExchange,
    target_palette_source: TileGraphPaletteSource,
    target_palette: &[TheColor],
) {
    if exchange.palette_source != TileGraphPaletteSource::Local
        || exchange.palette_colors.is_empty()
        || target_palette.is_empty()
    {
        exchange.palette_source = target_palette_source;
        if !target_palette.is_empty() {
            exchange.palette_colors = target_palette.to_vec();
        }
        return;
    }

    for node in &mut exchange.graph_state.nodes {
        match &mut node.kind {
            TileNodeKind::PaletteColor { index } => {
                *index = nearest_palette_index(
                    palette_color_for_index(&exchange.palette_colors, *index),
                    target_palette,
                ) as u16;
            }
            TileNodeKind::Colorize4 {
                color_1,
                color_2,
                color_3,
                color_4,
                ..
            } => {
                *color_1 = nearest_palette_index(
                    palette_color_for_index(&exchange.palette_colors, *color_1),
                    target_palette,
                ) as u16;
                *color_2 = nearest_palette_index(
                    palette_color_for_index(&exchange.palette_colors, *color_2),
                    target_palette,
                ) as u16;
                *color_3 = nearest_palette_index(
                    palette_color_for_index(&exchange.palette_colors, *color_3),
                    target_palette,
                ) as u16;
                *color_4 = nearest_palette_index(
                    palette_color_for_index(&exchange.palette_colors, *color_4),
                    target_palette,
                ) as u16;
            }
            _ => {}
        }
    }

    exchange.palette_source = target_palette_source;
    exchange.palette_colors = target_palette.to_vec();
}

fn palette_color_for_index(palette: &[TheColor], index: u16) -> TheColor {
    palette
        .get(index as usize)
        .cloned()
        .or_else(|| palette.last().cloned())
        .unwrap_or_else(|| TheColor::from_u8_array([0, 0, 0, 255]))
}

fn nearest_palette_index(color: TheColor, palette: &[TheColor]) -> usize {
    if palette.is_empty() {
        return 0;
    }
    let src = color.to_u8_array();
    let mut best = 0usize;
    let mut best_dist = i64::MAX;
    for (i, candidate) in palette.iter().enumerate() {
        let c = candidate.to_u8_array();
        let dr = src[0] as i64 - c[0] as i64;
        let dg = src[1] as i64 - c[1] as i64;
        let db = src[2] as i64 - c[2] as i64;
        let dist = dr * dr + dg * dg + db * db;
        if dist < best_dist {
            best_dist = dist;
            best = i;
        }
    }
    best
}

fn input_connection_source(
    state: &TileNodeGraphState,
    node_index: usize,
    input_terminal: u8,
) -> Option<u16> {
    state
        .connections
        .iter()
        .find(|(_, _, dest_node, dest_terminal)| {
            *dest_node as usize == node_index && *dest_terminal == input_terminal
        })
        .map(|(src_node, _, _, _)| *src_node)
}

fn remap_sub_index(index: u16, sub_map: &[Option<u16>], _base: u16) -> Option<u16> {
    if index == 0 {
        None
    } else {
        sub_map.get(index as usize).and_then(|v| *v)
    }
}

pub struct TileGraphRenderer {
    palette: Vec<TheColor>,
    colorize4_ranges: RwLock<Vec<Option<(f32, f32)>>>,
}

impl TileGraphRenderer {
    pub fn new(palette: Vec<TheColor>) -> Self {
        Self {
            palette,
            colorize4_ranges: RwLock::new(Vec::new()),
        }
    }

    pub fn render_graph(&self, graph: &TileNodeGraphExchange) -> RenderedTileGraph {
        let mut state = graph.graph_state.clone();
        state.ensure_root();

        let tile_width = graph.tile_pixel_width.max(1) as usize;
        let tile_height = graph.tile_pixel_height.max(1) as usize;
        let grid_width = graph.output_grid_width.max(1) as usize;
        let grid_height = graph.output_grid_height.max(1) as usize;

        let sheet_width = tile_width * grid_width;
        let sheet_height = tile_height * grid_height;
        let colorize4_ranges = self.compute_colorize4_ranges(
            &state,
            graph,
            grid_width,
            grid_height,
            tile_width,
            tile_height,
        );
        if let Ok(mut ranges) = self.colorize4_ranges.write() {
            *ranges = colorize4_ranges;
        }
        let mut sheet_color = vec![0_u8; sheet_width * sheet_height * 4];
        let mut sheet_material = vec![0_u8; sheet_width * sheet_height * 4];
        let mut sheet_height_data = vec![0_u8; sheet_width * sheet_height];

        for sy in 0..sheet_height {
            for sx in 0..sheet_width {
                let gx = if sheet_width <= 1 {
                    0.5
                } else {
                    sx as f32 / (sheet_width - 1) as f32
                };
                let gy = if sheet_height <= 1 {
                    0.5
                } else {
                    sy as f32 / (sheet_height - 1) as f32
                };
                let scaled_x = gx * grid_width as f32;
                let scaled_y = gy * grid_height as f32;
                let cell_x = scaled_x.floor().min((grid_width - 1) as f32) as u16;
                let cell_y = scaled_y.floor().min((grid_height - 1) as f32) as u16;
                let local_u = (scaled_x - cell_x as f32).clamp(0.0, 1.0);
                let local_v = (scaled_y - cell_y as f32).clamp(0.0, 1.0);
                let eval = TileEvalContext {
                    cell_x,
                    cell_y,
                    group_width: graph.output_grid_width.max(1),
                    group_height: graph.output_grid_height.max(1),
                    tile_pixel_width: graph.tile_pixel_width.max(1),
                    tile_pixel_height: graph.tile_pixel_height.max(1),
                    u: local_u,
                    v: local_v,
                };
                let color = self
                    .evaluate_node_color(&state, 0, eval, &mut FxHashSet::default())
                    .unwrap_or_else(|| TheColor::from_u8_array_3([96, 96, 96]))
                    .to_u8_array();
                let material = self.evaluate_output_material(&state, eval);
                let height = self.evaluate_output_height(&state, eval);
                let i = (sy * sheet_width + sx) * 4;
                sheet_color[i..i + 4].copy_from_slice(&color);
                sheet_material[i] = unit_to_u8(material.0);
                sheet_material[i + 1] = unit_to_u8(material.1);
                sheet_material[i + 2] = unit_to_u8(material.2);
                sheet_material[i + 3] = unit_to_u8(material.3);
                sheet_height_data[sy * sheet_width + sx] = unit_to_u8(height);
            }
        }

        let tile_count = grid_width * grid_height;
        let rendered_tiles: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)> = (0..tile_count)
            .into_par_iter()
            .map(|tile_index| {
                let cell_x = tile_index % grid_width;
                let cell_y = tile_index / grid_width;
                let mut tile_color = vec![0_u8; tile_width * tile_height * 4];
                let mut tile_material = vec![0_u8; tile_width * tile_height * 4];
                let mut tile_height_data = vec![0_u8; tile_width * tile_height];

                for py in 0..tile_height {
                    for px in 0..tile_width {
                        let u = if tile_width <= 1 {
                            0.5
                        } else {
                            px as f32 / (tile_width - 1) as f32
                        };
                        let v = if tile_height <= 1 {
                            0.5
                        } else {
                            py as f32 / (tile_height - 1) as f32
                        };
                        let eval = TileEvalContext {
                            cell_x: cell_x as u16,
                            cell_y: cell_y as u16,
                            group_width: graph.output_grid_width.max(1),
                            group_height: graph.output_grid_height.max(1),
                            tile_pixel_width: graph.tile_pixel_width.max(1),
                            tile_pixel_height: graph.tile_pixel_height.max(1),
                            // Sample the full 0..1 tile span inclusively so adjacent tiles
                            // duplicate their shared border texels and stitch cleanly when
                            // used as separate atlas tiles on surfaces.
                            u,
                            v,
                        };

                        let color = self
                            .evaluate_node_color(&state, 0, eval, &mut FxHashSet::default())
                            .unwrap_or_else(|| TheColor::from_u8_array_3([96, 96, 96]))
                            .to_u8_array();
                        let material = self.evaluate_output_material(&state, eval);
                        let height = self.evaluate_output_height(&state, eval);

                        let i = (py * tile_width + px) * 4;
                        tile_color[i..i + 4].copy_from_slice(&color);
                        tile_material[i] = unit_to_u8(material.0);
                        tile_material[i + 1] = unit_to_u8(material.1);
                        tile_material[i + 2] = unit_to_u8(material.2);
                        tile_material[i + 3] = unit_to_u8(material.3);
                        tile_height_data[py * tile_width + px] = unit_to_u8(height);
                    }
                }

                (tile_color, tile_material, tile_height_data)
            })
            .collect();

        let mut tiles_color = Vec::with_capacity(tile_count);
        let mut tiles_material = Vec::with_capacity(tile_count);
        let mut tiles_height = Vec::with_capacity(tile_count);

        for (_tile_index, (tile_color, tile_material, tile_height_data)) in
            rendered_tiles.into_iter().enumerate()
        {
            tiles_color.push(tile_color);
            tiles_material.push(tile_material);
            tiles_height.push(tile_height_data);
        }

        RenderedTileGraph {
            grid_width,
            grid_height,
            tile_width,
            tile_height,
            sheet_color,
            sheet_material,
            sheet_height: sheet_height_data,
            tiles_color,
            tiles_material,
            tiles_height,
            particle_output: self.output_particle(&state),
            light_output: self.output_light(&state),
        }
    }

    pub fn output_particle(&self, state: &TileNodeGraphState) -> Option<TileParticleOutput> {
        let Some(root) = state.nodes.first() else {
            return None;
        };
        let particle_enabled = match &root.kind {
            TileNodeKind::OutputRoot {
                particle_enabled, ..
            } => *particle_enabled,
            _ => false,
        };
        if !particle_enabled {
            return None;
        }
        let emitter_index =
            state
                .connections
                .iter()
                .find_map(|(src_node, src_term, dst_node, dst_term)| {
                    if *dst_node == 0 && *dst_term == 6 && *src_term == 0 {
                        Some(*src_node as usize)
                    } else {
                        None
                    }
                })?;
        match state.nodes.get(emitter_index).map(|node| &node.kind) {
            Some(TileNodeKind::ParticleEmitter {
                rate,
                spread,
                lifetime_min,
                lifetime_max,
                radius_min,
                radius_max,
                speed_min,
                speed_max,
                color_variation,
            }) => Some(TileParticleOutput {
                rate: (*rate).max(0.0),
                spread: (*spread).clamp(0.0, std::f32::consts::PI),
                lifetime_min: (*lifetime_min).max(0.01),
                lifetime_max: (*lifetime_max).max(*lifetime_min),
                radius_min: (*radius_min).max(0.001),
                radius_max: (*radius_max).max(*radius_min),
                speed_min: (*speed_min).max(0.0),
                speed_max: (*speed_max).max(*speed_min),
                flame_base: false,
                color_variation: *color_variation,
                ramp_colors: [
                    [255, 240, 200, 255],
                    [255, 176, 72, 255],
                    [224, 84, 24, 255],
                    [40, 36, 36, 255],
                ],
            }),
            Some(TileNodeKind::ParticleRender {
                radius_min,
                radius_max,
                flame_base,
                color_variation,
                color_1,
                color_2,
                color_3,
                color_4,
            }) => {
                let spawn = state.connections.iter().find_map(
                    |(src_node, src_term, dst_node, dst_term)| {
                        if *dst_node as usize == emitter_index && *dst_term == 0 && *src_term == 0 {
                            Some(*src_node as usize)
                        } else {
                            None
                        }
                    },
                );
                let motion = state.connections.iter().find_map(
                    |(src_node, src_term, dst_node, dst_term)| {
                        if *dst_node as usize == emitter_index && *dst_term == 1 && *src_term == 0 {
                            Some(*src_node as usize)
                        } else {
                            None
                        }
                    },
                );
                let (rate, spread) = match spawn
                    .and_then(|index| state.nodes.get(index))
                    .map(|node| &node.kind)
                {
                    Some(TileNodeKind::ParticleSpawn { rate, spread }) => {
                        ((*rate).max(0.0), (*spread).clamp(0.0, std::f32::consts::PI))
                    }
                    _ => (24.0, 0.75),
                };
                let (lifetime_min, lifetime_max, speed_min, speed_max) = match motion
                    .and_then(|index| state.nodes.get(index))
                    .map(|node| &node.kind)
                {
                    Some(TileNodeKind::ParticleMotion {
                        lifetime_min,
                        lifetime_max,
                        speed_min,
                        speed_max,
                    }) => (
                        (*lifetime_min).max(0.01),
                        (*lifetime_max).max(*lifetime_min),
                        (*speed_min).max(0.0),
                        (*speed_max).max(*speed_min),
                    ),
                    _ => (0.35, 0.9, 0.35, 1.1),
                };
                Some(TileParticleOutput {
                    rate,
                    spread,
                    lifetime_min,
                    lifetime_max,
                    radius_min: (*radius_min).max(0.001),
                    radius_max: (*radius_max).max(*radius_min),
                    speed_min,
                    speed_max,
                    flame_base: *flame_base,
                    color_variation: *color_variation,
                    ramp_colors: self.particle_ramp_colors(
                        state,
                        emitter_index,
                        [*color_1, *color_2, *color_3, *color_4],
                    ),
                })
            }
            _ => None,
        }
    }

    pub fn output_light(&self, state: &TileNodeGraphState) -> Option<TileLightOutput> {
        let Some(root) = state.nodes.first() else {
            return None;
        };
        let light_enabled = match &root.kind {
            TileNodeKind::OutputRoot { light_enabled, .. } => *light_enabled,
            _ => false,
        };
        if !light_enabled {
            return None;
        }
        let light_index =
            state
                .connections
                .iter()
                .find_map(|(src_node, src_term, dst_node, dst_term)| {
                    if *dst_node == 0 && *dst_term == 7 && *src_term == 0 {
                        Some(*src_node as usize)
                    } else {
                        None
                    }
                })?;
        match state.nodes.get(light_index).map(|node| &node.kind) {
            Some(TileNodeKind::LightEmitter {
                intensity,
                range,
                flicker,
                lift,
            }) => Some(TileLightOutput {
                intensity: (*intensity).max(0.0),
                range: (*range).max(0.0),
                flicker: (*flicker).clamp(0.0, 1.0),
                lift: (*lift).max(0.0),
            }),
            _ => None,
        }
    }

    fn evaluate_output_height(&self, state: &TileNodeGraphState, eval: TileEvalContext) -> f32 {
        self.evaluate_output_height_internal(
            state,
            eval,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        )
        .unwrap_or(0.5)
        .clamp(0.0, 1.0)
    }

    fn evaluate_output_height_internal(
        &self,
        state: &TileNodeGraphState,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<f32> {
        self.evaluate_connected_scalar(state, 0, 1, eval, visiting, visiting_subgraphs)
            .or_else(|| {
                self.evaluate_connected_color(state, 0, 0, eval, visiting, visiting_subgraphs)
                    .map(Self::color_to_mask)
            })
    }

    fn palette_color(&self, index: u16) -> Option<TheColor> {
        self.palette.get(index as usize).cloned().or_else(|| {
            let v = (index.min(255)) as u8;
            Some(TheColor::from_u8_array([v, v, v, 255]))
        })
    }

    fn nearest_palette_color(&self, color: TheColor) -> TheColor {
        if self.palette.is_empty() {
            return color;
        }
        let rgba = color.to_u8_array();
        let mut best = self.palette[0].clone();
        let mut best_dist = f32::MAX;
        for candidate in &self.palette {
            let c = candidate.to_u8_array();
            let dr = rgba[0] as f32 - c[0] as f32;
            let dg = rgba[1] as f32 - c[1] as f32;
            let db = rgba[2] as f32 - c[2] as f32;
            let dist = dr * dr + dg * dg + db * db;
            if dist < best_dist {
                best_dist = dist;
                best = candidate.clone();
            }
        }
        best
    }

    fn colorize4_palette_color(
        &self,
        slot: usize,
        color_1: u16,
        color_2: u16,
        color_3: u16,
        color_4: u16,
    ) -> TheColor {
        let index = match slot {
            0 => color_1,
            1 => color_2,
            2 => color_3,
            _ => color_4,
        };
        self.palette_color(index)
            .unwrap_or_else(|| TheColor::from_u8_array([255, 255, 255, 255]))
    }

    fn particle_ramp_colors(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        fallback: [u16; 4],
    ) -> [[u8; 4]; 4] {
        let mut colors = fallback.map(|index| {
            self.palette_color(index)
                .unwrap_or_else(|| TheColor::from_u8_array([255, 255, 255, 255]))
                .to_u8_array()
        });

        for terminal in 2..=5u8 {
            let Some(source_index) =
                state
                    .connections
                    .iter()
                    .find_map(|(src_node, src_term, dst_node, dst_term)| {
                        if *dst_node as usize == node_index
                            && *dst_term == terminal
                            && *src_term == 0
                        {
                            Some(*src_node as usize)
                        } else {
                            None
                        }
                    })
            else {
                continue;
            };

            let Some(source_kind) = state.nodes.get(source_index).map(|node| &node.kind) else {
                continue;
            };

            match source_kind {
                TileNodeKind::PaletteColor { index } => {
                    if let Some(color) = self.palette_color(*index) {
                        colors[(terminal - 2) as usize] = color.to_u8_array();
                    }
                }
                TileNodeKind::Color { color } => {
                    colors[(terminal - 2) as usize] = color.to_u8_array();
                }
                TileNodeKind::Colorize4 {
                    color_1,
                    color_2,
                    color_3,
                    color_4,
                    ..
                } if terminal == 2 => {
                    colors = [
                        self.colorize4_palette_color(0, *color_1, *color_2, *color_3, *color_4)
                            .to_u8_array(),
                        self.colorize4_palette_color(1, *color_1, *color_2, *color_3, *color_4)
                            .to_u8_array(),
                        self.colorize4_palette_color(2, *color_1, *color_2, *color_3, *color_4)
                            .to_u8_array(),
                        self.colorize4_palette_color(3, *color_1, *color_2, *color_3, *color_4)
                            .to_u8_array(),
                    ];
                }
                _ => {}
            }
        }

        colors
    }

    fn bayer4(x: usize, y: usize) -> f32 {
        const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
        BAYER[y & 3][x & 3] as f32 / 16.0
    }

    fn colorize4_range(&self, node_index: usize) -> Option<(f32, f32)> {
        self.colorize4_ranges
            .read()
            .ok()
            .and_then(|ranges| ranges.get(node_index).copied().flatten())
    }

    fn compute_colorize4_ranges(
        &self,
        state: &TileNodeGraphState,
        graph: &TileNodeGraphExchange,
        grid_width: usize,
        grid_height: usize,
        tile_width: usize,
        tile_height: usize,
    ) -> Vec<Option<(f32, f32)>> {
        let mut ranges = vec![None; state.nodes.len()];
        for (node_index, node) in state.nodes.iter().enumerate() {
            let TileNodeKind::Colorize4 { auto_range, .. } = &node.kind else {
                continue;
            };
            if !*auto_range || self.input_connection(state, node_index, 0).is_none() {
                continue;
            }

            let mut min_v = f32::INFINITY;
            let mut max_v = f32::NEG_INFINITY;

            for cell_y in 0..grid_height {
                for cell_x in 0..grid_width {
                    for py in 0..tile_height {
                        for px in 0..tile_width {
                            let u = if tile_width <= 1 {
                                0.5
                            } else {
                                px as f32 / (tile_width - 1) as f32
                            };
                            let v = if tile_height <= 1 {
                                0.5
                            } else {
                                py as f32 / (tile_height - 1) as f32
                            };
                            let eval = TileEvalContext {
                                cell_x: cell_x as u16,
                                cell_y: cell_y as u16,
                                group_width: graph.output_grid_width.max(1),
                                group_height: graph.output_grid_height.max(1),
                                tile_pixel_width: graph.tile_pixel_width.max(1),
                                tile_pixel_height: graph.tile_pixel_height.max(1),
                                u,
                                v,
                            };
                            if let Some(color) = self.evaluate_connected_color(
                                state,
                                node_index,
                                0,
                                eval,
                                &mut FxHashSet::default(),
                                &mut FxHashSet::default(),
                            ) {
                                let value = Self::color_to_mask(color).clamp(0.0, 1.0);
                                min_v = min_v.min(value);
                                max_v = max_v.max(value);
                            }
                        }
                    }
                }
            }

            if min_v.is_finite() && max_v.is_finite() {
                if (max_v - min_v).abs() < 1e-5 {
                    let center = min_v.clamp(0.0, 1.0);
                    let lo = (center - 0.5).clamp(0.0, 1.0);
                    let hi = (center + 0.5).clamp(0.0, 1.0);
                    ranges[node_index] = Some((lo, hi.max(lo + 1e-5)));
                } else {
                    ranges[node_index] = Some((min_v, max_v));
                }
            }
        }
        ranges
    }

    fn evaluate_output_material(
        &self,
        state: &TileNodeGraphState,
        eval: TileEvalContext,
    ) -> (f32, f32, f32, f32) {
        self.evaluate_output_material_internal(
            state,
            eval,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        )
        .unwrap_or((0.5, 0.0, 1.0, 0.0))
    }

    fn evaluate_output_material_internal(
        &self,
        state: &TileNodeGraphState,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<(f32, f32, f32, f32)> {
        self.evaluate_connected_material(state, 0, 1, eval, visiting, visiting_subgraphs)
    }

    fn solo_node_index(&self, state: &TileNodeGraphState) -> Option<usize> {
        state.nodes.iter().position(|n| n.solo)
    }

    fn evaluate_node_scalar_internal(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<f32> {
        self.evaluate_node_scalar_output_internal(
            state,
            node_index,
            0,
            eval,
            visiting,
            visiting_subgraphs,
        )
    }

    fn evaluate_node_scalar_output_internal(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        output_terminal: u8,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<f32> {
        state
            .nodes
            .get(node_index)
            .and_then(|node| match &node.kind {
                TileNodeKind::Scalar { value } => Some(*value),
                TileNodeKind::LayerInput { value, .. } => self
                    .evaluate_connected_scalar(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .or(Some(*value)),
                _ => self
                    .evaluate_node_color_output_internal(
                        state,
                        node_index,
                        output_terminal,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(Self::color_to_mask),
            })
    }

    fn evaluate_node_material_internal(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<(f32, f32, f32, f32)> {
        if !visiting.insert(node_index) {
            return None;
        }
        let result = state.nodes.get(node_index).and_then(|node| {
            if node.bypass && !matches!(node.kind, TileNodeKind::OutputRoot { .. }) {
                if let Some(value) = self.evaluate_connected_material(
                    state,
                    node_index,
                    0,
                    eval,
                    visiting,
                    visiting_subgraphs,
                ) {
                    return Some(value);
                }
            }
            match &node.kind {
                TileNodeKind::OutputRoot {
                    roughness,
                    metallic,
                    opacity,
                    emissive,
                    ..
                } => {
                    if let Some(solo) = self.solo_node_index(state)
                        && solo != node_index
                    {
                        self.evaluate_node_material_internal(
                            state,
                            solo,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                    } else {
                        let mut channel = |input_terminal: u8, default: f32| -> f32 {
                            self.evaluate_connected_scalar(
                                state,
                                node_index,
                                input_terminal,
                                eval,
                                visiting,
                                visiting_subgraphs,
                            )
                            .unwrap_or(default)
                            .clamp(0.0, 1.0)
                        };
                        Some((
                            channel(2, *roughness),
                            channel(3, *metallic),
                            channel(4, *opacity),
                            channel(5, *emissive),
                        ))
                    }
                }
                TileNodeKind::ImportLayer { .. } => None,
                TileNodeKind::Material {
                    roughness,
                    metallic,
                    opacity,
                    emissive,
                } => Some((*roughness, *metallic, *opacity, *emissive)),
                TileNodeKind::MakeMaterial => {
                    let mut channel = |input_terminal: u8, default: f32| -> f32 {
                        self.evaluate_connected_scalar(
                            state,
                            node_index,
                            input_terminal,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or(default)
                        .clamp(0.0, 1.0)
                    };
                    Some((
                        channel(0, 0.5),
                        channel(1, 0.0),
                        channel(2, 1.0),
                        channel(3, 0.0),
                    ))
                }
                TileNodeKind::MaterialMix { factor } => {
                    let a = self.evaluate_connected_material(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_material(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let mask = self
                        .evaluate_connected_scalar(
                            state,
                            node_index,
                            2,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or(0.0)
                        .clamp(0.0, 1.0)
                        * factor.clamp(0.0, 1.0);
                    match (a, b) {
                        (Some(a), Some(b)) => Some((
                            a.0 * (1.0 - mask) + b.0 * mask,
                            a.1 * (1.0 - mask) + b.1 * mask,
                            a.2 * (1.0 - mask) + b.2 * mask,
                            a.3 * (1.0 - mask) + b.3 * mask,
                        )),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                _ => {
                    if node.mute {
                        return Some((0.5, 0.0, 0.0, 0.0));
                    }
                    let roughness = self
                        .evaluate_node_scalar_internal(
                            state,
                            node_index,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or(0.5)
                        .clamp(0.0, 1.0);
                    Some((roughness, 0.0, 1.0, 0.0))
                }
            }
        });
        visiting.remove(&node_index);
        result
    }

    fn input_connection(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        input_terminal: u8,
    ) -> Option<(usize, u8)> {
        state
            .connections
            .iter()
            .find(|(_, _, dest_node, dest_terminal)| {
                *dest_node as usize == node_index && *dest_terminal == input_terminal
            })
            .map(|(src_node, src_terminal, _, _)| (*src_node as usize, *src_terminal))
    }

    fn evaluate_connected_color(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        input_terminal: u8,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<TheColor> {
        self.input_connection(state, node_index, input_terminal)
            .and_then(|(src, output_terminal)| {
                self.evaluate_node_color_output_internal(
                    state,
                    src,
                    output_terminal,
                    eval,
                    visiting,
                    visiting_subgraphs,
                )
            })
    }

    fn input_connection_source(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        input_terminal: u8,
    ) -> Option<usize> {
        self.input_connection(state, node_index, input_terminal)
            .map(|(src, _)| src)
    }

    fn evaluate_connected_scalar(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        input_terminal: u8,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<f32> {
        self.input_connection(state, node_index, input_terminal)
            .and_then(|(src, output_terminal)| {
                self.evaluate_node_scalar_output_internal(
                    state,
                    src,
                    output_terminal,
                    eval,
                    visiting,
                    visiting_subgraphs,
                )
            })
    }

    fn connected_warp_vector(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        input_terminal: u8,
        eval: TileEvalContext,
        amount: f32,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Vec2<f32> {
        if amount <= f32::EPSILON
            || self
                .input_connection(state, node_index, input_terminal)
                .is_none()
        {
            return Vec2::new(0.0, 0.0);
        }

        let sx = self
            .evaluate_connected_scalar(
                state,
                node_index,
                input_terminal,
                eval,
                visiting,
                visiting_subgraphs,
            )
            .unwrap_or(0.5);
        let wrapped_u = (eval.group_u() + 0.173).rem_euclid(1.0);
        let wrapped_v = (eval.group_v() + 0.317).rem_euclid(1.0);
        let sy = self
            .evaluate_connected_scalar(
                state,
                node_index,
                input_terminal,
                eval.with_group_uv(wrapped_u, wrapped_v),
                visiting,
                visiting_subgraphs,
            )
            .unwrap_or(sx);

        Vec2::new((sx - 0.5) * 2.0 * amount, (sy - 0.5) * 2.0 * amount)
    }

    fn evaluate_connected_material(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        input_terminal: u8,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<(f32, f32, f32, f32)> {
        self.input_connection(state, node_index, input_terminal)
            .and_then(|(src, _output_terminal)| {
                self.evaluate_node_material_internal(state, src, eval, visiting, visiting_subgraphs)
            })
    }

    fn evaluate_node_color(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
    ) -> Option<TheColor> {
        self.evaluate_node_color_output_internal(
            state,
            node_index,
            0,
            eval,
            visiting,
            &mut FxHashSet::default(),
        )
    }

    fn evaluate_node_color_internal(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<TheColor> {
        self.evaluate_node_color_output_internal(
            state,
            node_index,
            0,
            eval,
            visiting,
            visiting_subgraphs,
        )
    }

    fn evaluate_node_color_output_internal(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        output_terminal: u8,
        eval: TileEvalContext,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Option<TheColor> {
        if !visiting.insert(node_index) {
            return None;
        }
        let result = state.nodes.get(node_index).and_then(|node| {
            if node.mute {
                return Some(TheColor::from_u8_array([0, 0, 0, 0]));
            }
            if node.bypass
                && !matches!(
                    node.kind,
                    TileNodeKind::OutputRoot { .. } | TileNodeKind::GroupUV
                )
            {
                if let Some(color) = self.evaluate_connected_color(
                    state,
                    node_index,
                    0,
                    eval,
                    visiting,
                    visiting_subgraphs,
                ) {
                    return Some(color);
                }
            }
            match &node.kind {
                TileNodeKind::OutputRoot { .. } => {
                    if let Some(solo) = self.solo_node_index(state)
                        && solo != node_index
                    {
                        self.evaluate_node_color_output_internal(
                            state,
                            solo,
                            output_terminal,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                    } else {
                        self.evaluate_connected_color(
                            state,
                            node_index,
                            0,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                    }
                }
                TileNodeKind::LayerInput { value, .. } => self
                    .evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .or_else(|| {
                        let v = unit_to_u8(*value);
                        Some(TheColor::from_u8_array([v, v, v, 255]))
                    }),
                TileNodeKind::ImportLayer { .. } => None,
                TileNodeKind::ParticleEmitter { .. }
                | TileNodeKind::ParticleSpawn { .. }
                | TileNodeKind::ParticleMotion { .. }
                | TileNodeKind::ParticleRender { .. }
                | TileNodeKind::LightEmitter { .. } => None,
                TileNodeKind::GroupUV => Some(TheColor::from_u8_array([
                    unit_to_u8(eval.group_u()),
                    unit_to_u8(eval.group_v()),
                    0,
                    255,
                ])),
                TileNodeKind::Scalar { value } => {
                    let v = unit_to_u8(*value);
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Colorize4 {
                    color_1,
                    color_2,
                    color_3,
                    color_4,
                    pixel_size,
                    dither,
                    auto_range,
                } => {
                    let quantized_eval = if *pixel_size > 1 {
                        let px_size = (*pixel_size).max(1) as f32;
                        let tile_w = eval.tile_pixel_width.max(1) as f32;
                        let tile_h = eval.tile_pixel_height.max(1) as f32;
                        let x = (eval.u * (tile_w - 1.0)).round();
                        let y = (eval.v * (tile_h - 1.0)).round();
                        let qx =
                            ((x / px_size).floor() * px_size).clamp(0.0, (tile_w - 1.0).max(0.0));
                        let qy =
                            ((y / px_size).floor() * px_size).clamp(0.0, (tile_h - 1.0).max(0.0));
                        let u = if tile_w <= 1.0 {
                            0.5
                        } else {
                            qx / (tile_w - 1.0)
                        };
                        let v = if tile_h <= 1.0 {
                            0.5
                        } else {
                            qy / (tile_h - 1.0)
                        };
                        TileEvalContext { u, v, ..eval }
                    } else {
                        eval
                    };
                    self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        quantized_eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(|color| {
                        let mut t = Self::color_to_mask(color).clamp(0.0, 1.0);
                        if *auto_range
                            && let Some((min_v, max_v)) = self.colorize4_range(node_index)
                        {
                            let width = (max_v - min_v).max(1e-5);
                            t = ((t - min_v) / width).clamp(0.0, 1.0);
                        }
                        t = t.clamp(0.0, 0.999_999);
                        if *dither {
                            let x = (quantized_eval.u
                                * (quantized_eval.tile_pixel_width.max(1) - 1) as f32)
                                .round()
                                .max(0.0) as usize;
                            let y = (quantized_eval.v
                                * (quantized_eval.tile_pixel_height.max(1) - 1) as f32)
                                .round()
                                .max(0.0) as usize;
                            let threshold = Self::bayer4(x, y) - 0.5;
                            t = (t + threshold * 0.18).clamp(0.0, 0.999_999);
                        }
                        let slot = (t * 4.0).floor() as usize;
                        self.colorize4_palette_color(slot, *color_1, *color_2, *color_3, *color_4)
                    })
                }
                TileNodeKind::Gradient { mode } => {
                    let gu = eval.group_u().clamp(0.0, 1.0);
                    let gv = eval.group_v().clamp(0.0, 1.0);
                    let value = match mode {
                        0 => gu,
                        1 => gv,
                        _ => {
                            let dx = gu - 0.5;
                            let dy = gv - 0.5;
                            (1.0 - (dx * dx + dy * dy).sqrt() * 2.0).clamp(0.0, 1.0)
                        }
                    };
                    let v = unit_to_u8(value);
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Curve { power } => self
                    .evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(|color| {
                        let value = Self::color_to_mask(color)
                            .clamp(0.0, 1.0)
                            .powf(power.clamp(0.1, 4.0));
                        let v = unit_to_u8(value);
                        TheColor::from_u8_array([v, v, v, 255])
                    }),
                TileNodeKind::Color { color } => Some(color.clone()),
                TileNodeKind::PaletteColor { index } => self.palette_color(*index),
                TileNodeKind::NearestPalette => self
                    .evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(|color| self.nearest_palette_color(color)),
                TileNodeKind::Mix { factor } => {
                    let a = self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_color(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    match (a, b) {
                        (Some(a), Some(b)) => Some(Self::mix_colors(a, b, *factor)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Checker { scale } => {
                    let s = (*scale).max(1) as f32;
                    let cx = (eval.group_u() * s).floor() as i32;
                    let cy = (eval.group_v() * s).floor() as i32;
                    let a = self
                        .evaluate_connected_color(
                            state,
                            node_index,
                            0,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or_else(|| TheColor::from_u8_array_3([255, 255, 255]));
                    let b = self
                        .evaluate_connected_color(
                            state,
                            node_index,
                            1,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or_else(|| TheColor::from_u8_array_3([0, 0, 0]));
                    if (cx + cy) & 1 == 0 { Some(a) } else { Some(b) }
                }
                TileNodeKind::Noise { scale, seed, wrap } => {
                    let s = (*scale).clamp(0.0, 1.0).max(0.0001);
                    let frequency = (s * 64.0).round().max(1.0) as i32;
                    let v = unit_to_u8(Self::value_noise(
                        eval.group_u(),
                        eval.group_v(),
                        frequency,
                        *seed,
                        *wrap,
                    ));
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Voronoi {
                    scale,
                    seed,
                    jitter,
                    warp_amount,
                    falloff,
                } => {
                    let warp = self.connected_warp_vector(
                        state,
                        node_index,
                        0,
                        eval,
                        warp_amount.clamp(0.0, 0.25),
                        visiting,
                        visiting_subgraphs,
                    );
                    let value = match output_terminal {
                        1 => Self::voronoi_height(eval, warp, *scale, *seed, *jitter, *falloff),
                        2 => Self::voronoi_cell_id(eval, warp, *scale, *seed, *jitter),
                        _ => Self::voronoi_center(eval, warp, *scale, *seed, *jitter),
                    };
                    let v = unit_to_u8(value);
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::BoxDivide {
                    scale,
                    gap,
                    rotation,
                    rounding,
                    warp_amount,
                    falloff,
                } => {
                    let warp = self.connected_warp_vector(
                        state,
                        node_index,
                        0,
                        eval,
                        warp_amount.clamp(0.0, 0.25),
                        visiting,
                        visiting_subgraphs,
                    );
                    let value = match output_terminal {
                        1 => Self::box_divide_height(
                            eval, warp, *scale, *gap, *rotation, *rounding, *falloff,
                        ),
                        2 => {
                            Self::box_divide_cell_id(eval, warp, *scale, *gap, *rotation, *rounding)
                        }
                        _ => {
                            Self::box_divide_center(eval, warp, *scale, *gap, *rotation, *rounding)
                        }
                    };
                    let v = unit_to_u8(value);
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Offset { x, y } => self
                    .input_connection(state, node_index, 0)
                    .and_then(|(src, out)| {
                        self.evaluate_node_color_output_internal(
                            state,
                            src,
                            out,
                            eval.with_group_uv(eval.group_u() + *x, eval.group_v() + *y),
                            visiting,
                            visiting_subgraphs,
                        )
                    }),
                TileNodeKind::Scale { x, y } => self
                    .input_connection(state, node_index, 0)
                    .and_then(|(src, out)| {
                        let gu = (eval.group_u() - 0.5) * x.max(0.1) + 0.5;
                        let gv = (eval.group_v() - 0.5) * y.max(0.1) + 0.5;
                        self.evaluate_node_color_output_internal(
                            state,
                            src,
                            out,
                            eval.with_group_uv(gu, gv),
                            visiting,
                            visiting_subgraphs,
                        )
                    }),
                TileNodeKind::Repeat { repeat_x, repeat_y } => self
                    .input_connection(state, node_index, 0)
                    .and_then(|(src, out)| {
                        let wrapped_u = (eval.group_u() * repeat_x.max(0.1)).fract();
                        let wrapped_v = (eval.group_v() * repeat_y.max(0.1)).fract();
                        self.evaluate_node_color_output_internal(
                            state,
                            src,
                            out,
                            eval.with_group_uv(wrapped_u, wrapped_v),
                            visiting,
                            visiting_subgraphs,
                        )
                    }),
                TileNodeKind::Rotate { angle } => self
                    .input_connection(state, node_index, 0)
                    .and_then(|(src, out)| {
                        let radians = angle.to_radians();
                        let s = radians.sin();
                        let c = radians.cos();
                        let dx = eval.group_u() - 0.5;
                        let dy = eval.group_v() - 0.5;
                        let ru = dx * c - dy * s + 0.5;
                        let rv = dx * s + dy * c + 0.5;
                        self.evaluate_node_color_output_internal(
                            state,
                            src,
                            out,
                            eval.with_group_uv(ru, rv),
                            visiting,
                            visiting_subgraphs,
                        )
                    }),
                TileNodeKind::DirectionalWarp { amount, angle } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        let warp = self
                            .input_connection_source(state, node_index, 1)
                            .and_then(|warp_src| {
                                self.evaluate_node_color_internal(
                                    state,
                                    warp_src,
                                    eval,
                                    visiting,
                                    visiting_subgraphs,
                                )
                            })
                            .map(Self::color_to_mask)
                            .unwrap_or(0.5);
                        let radians = angle.to_radians();
                        let delta = (warp - 0.5) * amount.clamp(0.0, 1.0);
                        let du = radians.cos() * delta;
                        let dv = radians.sin() * delta;
                        self.evaluate_node_color_internal(
                            state,
                            src,
                            eval.with_group_uv(eval.group_u() + du, eval.group_v() + dv),
                            visiting,
                            visiting_subgraphs,
                        )
                    }),
                TileNodeKind::Brick {
                    columns,
                    rows,
                    staggered,
                    offset,
                    warp_amount,
                    falloff,
                } => {
                    let warp = self.connected_warp_vector(
                        state,
                        node_index,
                        0,
                        eval,
                        warp_amount.clamp(0.0, 0.25),
                        visiting,
                        visiting_subgraphs,
                    );
                    let value = match output_terminal {
                        1 => Self::brick_height(
                            eval, warp, *columns, *rows, *staggered, *offset, *falloff,
                        ),
                        2 => Self::brick_cell_id(eval, warp, *columns, *rows, *staggered, *offset),
                        _ => Self::brick_center(eval, warp, *columns, *rows, *staggered, *offset),
                    };
                    let v = unit_to_u8(value);
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Disc {
                    scale,
                    seed,
                    jitter,
                    radius,
                    warp_amount,
                    falloff,
                } => {
                    let warp = self.connected_warp_vector(
                        state,
                        node_index,
                        0,
                        eval,
                        warp_amount.clamp(0.0, 0.25),
                        visiting,
                        visiting_subgraphs,
                    );
                    let density = self
                        .evaluate_connected_scalar(
                            state,
                            node_index,
                            1,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or(*scale)
                        .clamp(0.05, 2.0);
                    let jitter = self
                        .evaluate_connected_scalar(
                            state,
                            node_index,
                            2,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or(*jitter)
                        .clamp(0.0, 1.0);
                    let radius = self
                        .evaluate_connected_scalar(
                            state,
                            node_index,
                            3,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or(*radius)
                        .clamp(0.05, 1.0);
                    let falloff = self
                        .evaluate_connected_scalar(
                            state,
                            node_index,
                            4,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .unwrap_or(*falloff)
                        .clamp(0.1, 4.0);
                    let value = match output_terminal {
                        1 => Self::disc_height(eval, warp, density, *seed, jitter, radius, falloff),
                        2 => Self::disc_cell_id(eval, warp, density, *seed, jitter),
                        _ => Self::disc_center(eval, warp, density, *seed, jitter, radius),
                    };
                    let v = unit_to_u8(value);
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::IdRandom => {
                    let id = self
                        .evaluate_connected_color(
                            state,
                            node_index,
                            0,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .map(Self::color_to_mask)
                        .unwrap_or(0.0);
                    let key = (id.clamp(0.0, 1.0) * 65535.0).round() as i32;
                    let v = unit_to_u8(Self::hash2(key, key ^ 0x45d9f3, 0x9e37_79b9));
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Min => {
                    let a = self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_color(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            let av = Self::color_to_mask(a);
                            let bv = Self::color_to_mask(b);
                            let v = unit_to_u8(av.min(bv));
                            Some(TheColor::from_u8_array([v, v, v, 255]))
                        }
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Max => {
                    let a = self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_color(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            let av = Self::color_to_mask(a);
                            let bv = Self::color_to_mask(b);
                            let v = unit_to_u8(av.max(bv));
                            Some(TheColor::from_u8_array([v, v, v, 255]))
                        }
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Add => {
                    let a = self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_color(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            let av = Self::color_to_mask(a);
                            let bv = Self::color_to_mask(b);
                            let v = unit_to_u8((av + bv).clamp(0.0, 1.0));
                            Some(TheColor::from_u8_array([v, v, v, 255]))
                        }
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Subtract => {
                    let a = self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_color(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    match (a, b) {
                        (Some(a), Some(b)) => {
                            let av = Self::color_to_mask(a);
                            let bv = Self::color_to_mask(b);
                            let v = unit_to_u8((av - bv).clamp(0.0, 1.0));
                            Some(TheColor::from_u8_array([v, v, v, 255]))
                        }
                        (Some(a), None) => Some(a),
                        (None, Some(_)) => Some(TheColor::from_u8_array([0, 0, 0, 255])),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Multiply => {
                    let a = self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_color(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    match (a, b) {
                        (Some(a), Some(b)) => Some(Self::multiply_colors(a, b)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::MakeMaterial => self
                    .evaluate_node_material_internal(
                        state,
                        node_index,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(material_to_color),
                TileNodeKind::Material {
                    roughness,
                    metallic,
                    opacity,
                    emissive,
                } => Some(material_to_color((
                    *roughness, *metallic, *opacity, *emissive,
                ))),
                TileNodeKind::MaterialMix { .. } => self
                    .evaluate_node_material_internal(
                        state,
                        node_index,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(material_to_color),
                TileNodeKind::MaskBlend { factor } => {
                    let a = self.evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = self.evaluate_connected_color(
                        state,
                        node_index,
                        1,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    );
                    let mask = self
                        .evaluate_connected_color(
                            state,
                            node_index,
                            2,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .map(Self::color_to_mask)
                        .unwrap_or(0.0);
                    match (a, b) {
                        (Some(a), Some(b)) => Some(Self::mix_colors(a, b, mask * *factor)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    }
                }
                TileNodeKind::Levels { level, width } => self
                    .evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(|color| {
                        let width = width.clamp(0.001, 1.0);
                        let level = level.clamp(0.0, 1.0);
                        let half = width * 0.5;
                        let low = (level - half).clamp(0.0, 1.0);
                        let high = (level + half).clamp(0.0, 1.0);
                        let v = unit_to_u8(Self::remap_unit(Self::color_to_mask(color), low, high));
                        TheColor::from_u8_array([v, v, v, 255])
                    }),
                TileNodeKind::HeightShape {
                    contrast,
                    bias,
                    plateau,
                    rim,
                    warp_amount,
                } => self
                    .evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(|fallback| {
                        let warp = self.connected_warp_vector(
                            state,
                            node_index,
                            1,
                            eval,
                            warp_amount.clamp(0.0, 0.25),
                            visiting,
                            visiting_subgraphs,
                        );
                        let input = self
                            .evaluate_connected_color(
                                state,
                                node_index,
                                0,
                                eval.with_group_uv(
                                    (eval.group_u() + warp.x).rem_euclid(1.0),
                                    (eval.group_v() + warp.y).rem_euclid(1.0),
                                ),
                                visiting,
                                visiting_subgraphs,
                            )
                            .unwrap_or(fallback);
                        let mut v = Self::color_to_mask(input).clamp(0.0, 1.0);
                        let contrast = self
                            .evaluate_connected_scalar(
                                state,
                                node_index,
                                2,
                                eval,
                                visiting,
                                visiting_subgraphs,
                            )
                            .unwrap_or(*contrast)
                            .clamp(0.1, 4.0);
                        v = ((v - 0.5) * contrast + 0.5).clamp(0.0, 1.0);

                        let bias = self
                            .evaluate_connected_scalar(
                                state,
                                node_index,
                                3,
                                eval,
                                visiting,
                                visiting_subgraphs,
                            )
                            .unwrap_or(*bias)
                            .clamp(-1.0, 1.0);
                        if bias < 0.0 {
                            let power = (1.0 + (-bias * 3.0)).clamp(1.0, 4.0);
                            v = v.powf(power);
                        } else if bias > 0.0 {
                            let power = (1.0 + (bias * 3.0)).clamp(1.0, 4.0);
                            v = 1.0 - (1.0 - v).powf(power);
                        }

                        let plateau = self
                            .evaluate_connected_scalar(
                                state,
                                node_index,
                                4,
                                eval,
                                visiting,
                                visiting_subgraphs,
                            )
                            .unwrap_or(*plateau)
                            .clamp(0.0, 3.0);
                        let rim = self
                            .evaluate_connected_scalar(
                                state,
                                node_index,
                                5,
                                eval,
                                visiting,
                                visiting_subgraphs,
                            )
                            .unwrap_or(*rim)
                            .clamp(0.0, 4.0);
                        if rim > 0.0 {
                            let shoulder = (4.0 * v * (1.0 - v)).clamp(0.0, 1.0);
                            v = (v - shoulder * rim * 0.18).clamp(0.0, 1.0);
                        }
                        if plateau > 0.0 {
                            let top = (1.0 - plateau * 0.18).clamp(0.35, 0.999);
                            if v > top {
                                let t = ((v - top) / (1.0 - top).max(0.0001)).clamp(0.0, 1.0);
                                let flatten = (0.2 / (1.0 + plateau * 1.2)).clamp(0.03, 0.2);
                                let curve = t * t * (3.0 - 2.0 * t);
                                v = top + curve * (1.0 - top) * flatten;
                            }
                        }

                        let out = unit_to_u8(v);
                        TheColor::from_u8_array([out, out, out, 255])
                    }),
                TileNodeKind::Threshold { cutoff } => self
                    .evaluate_connected_color(
                        state,
                        node_index,
                        0,
                        eval,
                        visiting,
                        visiting_subgraphs,
                    )
                    .map(|color| {
                        let v = if Self::color_to_mask(color) >= *cutoff {
                            255
                        } else {
                            0
                        };
                        TheColor::from_u8_array([v, v, v, 255])
                    }),
                TileNodeKind::Blur { radius } => {
                    let radius = radius.clamp(0.001, 0.08);
                    let offsets = [
                        (-1.0f32, -1.0f32),
                        (0.0, -1.0),
                        (1.0, -1.0),
                        (-1.0, 0.0),
                        (0.0, 0.0),
                        (1.0, 0.0),
                        (-1.0, 1.0),
                        (0.0, 1.0),
                        (1.0, 1.0),
                    ];
                    let mut sum = 0.0;
                    let mut weight_sum = 0.0;
                    for (ox, oy) in offsets {
                        let weight = if ox == 0.0 && oy == 0.0 {
                            2.0
                        } else if ox == 0.0 || oy == 0.0 {
                            1.0
                        } else {
                            0.75
                        };
                        let value = self
                            .evaluate_connected_color(
                                state,
                                node_index,
                                0,
                                eval.with_group_uv(
                                    eval.group_u() + ox * radius,
                                    eval.group_v() + oy * radius,
                                ),
                                visiting,
                                visiting_subgraphs,
                            )
                            .map(Self::color_to_mask)
                            .unwrap_or(0.0);
                        sum += value * weight;
                        weight_sum += weight;
                    }
                    let v = unit_to_u8((sum / weight_sum.max(0.0001)).clamp(0.0, 1.0));
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::SlopeBlur { radius, amount } => {
                    let radius = radius.clamp(0.001, 0.08);
                    let amount = amount.clamp(0.0, 1.0);
                    let center = self
                        .evaluate_connected_color(
                            state,
                            node_index,
                            0,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                        .map(Self::color_to_mask)
                        .unwrap_or(0.0);
                    let directions = [
                        Vec2::new(1.0f32, 0.0),
                        Vec2::new(0.707, 0.707),
                        Vec2::new(0.0, 1.0),
                        Vec2::new(-0.707, 0.707),
                        Vec2::new(-1.0, 0.0),
                        Vec2::new(-0.707, -0.707),
                        Vec2::new(0.0, -1.0),
                        Vec2::new(0.707, -0.707),
                    ];
                    let mut sum = center * 2.0;
                    let mut weight_sum = 2.0;
                    for dir in directions {
                        let near_u = eval.group_u() + dir.x * radius;
                        let near_v = eval.group_v() + dir.y * radius;
                        let near = self
                            .evaluate_connected_color(
                                state,
                                node_index,
                                0,
                                eval.with_group_uv(near_u, near_v),
                                visiting,
                                visiting_subgraphs,
                            )
                            .map(Self::color_to_mask)
                            .unwrap_or(center);
                        let shifted_u =
                            eval.group_u() + dir.x * radius * (1.0 + near * amount * 2.0);
                        let shifted_v =
                            eval.group_v() + dir.y * radius * (1.0 + near * amount * 2.0);
                        let shifted = self
                            .evaluate_connected_color(
                                state,
                                node_index,
                                0,
                                eval.with_group_uv(shifted_u, shifted_v),
                                visiting,
                                visiting_subgraphs,
                            )
                            .map(Self::color_to_mask)
                            .unwrap_or(near);
                        let weight = 0.75 + near * amount;
                        sum += shifted * weight;
                        weight_sum += weight;
                    }
                    let v = unit_to_u8((sum / weight_sum.max(0.0001)).clamp(0.0, 1.0));
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::HeightEdge { radius } => {
                    let radius = radius.clamp(0.001, 0.08);
                    let sample = |u: f32,
                                  v: f32,
                                  visiting: &mut FxHashSet<usize>,
                                  visiting_subgraphs: &mut FxHashSet<Uuid>|
                     -> f32 {
                        self.evaluate_connected_color(
                            state,
                            node_index,
                            0,
                            eval.with_group_uv(u, v),
                            visiting,
                            visiting_subgraphs,
                        )
                        .map(Self::color_to_mask)
                        .unwrap_or(0.0)
                    };
                    let c = sample(eval.group_u(), eval.group_v(), visiting, visiting_subgraphs);
                    let l = sample(
                        eval.group_u() - radius,
                        eval.group_v(),
                        visiting,
                        visiting_subgraphs,
                    );
                    let r = sample(
                        eval.group_u() + radius,
                        eval.group_v(),
                        visiting,
                        visiting_subgraphs,
                    );
                    let t = sample(
                        eval.group_u(),
                        eval.group_v() - radius,
                        visiting,
                        visiting_subgraphs,
                    );
                    let b = sample(
                        eval.group_u(),
                        eval.group_v() + radius,
                        visiting,
                        visiting_subgraphs,
                    );
                    let edge = (((c - l).abs() + (c - r).abs() + (c - t).abs() + (c - b).abs())
                        * 0.5)
                        .clamp(0.0, 1.0);
                    let v = unit_to_u8(edge);
                    Some(TheColor::from_u8_array([v, v, v, 255]))
                }
                TileNodeKind::Warp { amount } => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        let warp = self
                            .input_connection_source(state, node_index, 1)
                            .and_then(|warp_src| {
                                self.evaluate_node_color_internal(
                                    state,
                                    warp_src,
                                    eval,
                                    visiting,
                                    visiting_subgraphs,
                                )
                            })
                            .map(Self::color_to_mask)
                            .unwrap_or(0.5);
                        let delta = (warp - 0.5) * amount * 0.5;
                        self.evaluate_node_color_internal(
                            state,
                            src,
                            eval.with_group_uv(eval.group_u() + delta, eval.group_v() + delta),
                            visiting,
                            visiting_subgraphs,
                        )
                    }),
                TileNodeKind::Invert => self
                    .input_connection_source(state, node_index, 0)
                    .and_then(|src| {
                        self.evaluate_node_color_internal(
                            state,
                            src,
                            eval,
                            visiting,
                            visiting_subgraphs,
                        )
                    })
                    .map(|color| {
                        let rgba = color.to_u8_array();
                        TheColor::from_u8_array([
                            255 - rgba[0],
                            255 - rgba[1],
                            255 - rgba[2],
                            rgba[3],
                        ])
                    }),
            }
        });
        visiting.remove(&node_index);
        result
    }

    fn mix_colors(a: TheColor, b: TheColor, factor: f32) -> TheColor {
        let t = factor.clamp(0.0, 1.0);
        let aa = a.to_u8_array();
        let bb = b.to_u8_array();
        let lerp = |x: u8, y: u8| -> u8 {
            ((x as f32 * (1.0 - t) + y as f32 * t).round()).clamp(0.0, 255.0) as u8
        };
        TheColor::from_u8_array([
            lerp(aa[0], bb[0]),
            lerp(aa[1], bb[1]),
            lerp(aa[2], bb[2]),
            lerp(aa[3], bb[3]),
        ])
    }

    fn multiply_colors(a: TheColor, b: TheColor) -> TheColor {
        let aa = a.to_u8_array();
        let bb = b.to_u8_array();
        let mul = |x: u8, y: u8| -> u8 { ((x as u16 * y as u16) / 255) as u8 };
        TheColor::from_u8_array([
            mul(aa[0], bb[0]),
            mul(aa[1], bb[1]),
            mul(aa[2], bb[2]),
            mul(aa[3], bb[3]),
        ])
    }

    fn color_to_mask(color: TheColor) -> f32 {
        let rgba = color.to_u8_array();
        (0.2126 * rgba[0] as f32 + 0.7152 * rgba[1] as f32 + 0.0722 * rgba[2] as f32) / 255.0
    }

    fn hash2(x: i32, y: i32, seed: u32) -> f32 {
        let mut n = x as u32;
        n = n
            .wrapping_mul(374761393)
            .wrapping_add((y as u32).wrapping_mul(668265263));
        n ^= seed.wrapping_mul(2246822519);
        n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        ((n ^ (n >> 16)) & 0x00ff_ffff) as f32 / 0x00ff_ffff as f32
    }

    fn smoothstep_unit(t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn value_noise(u: f32, v: f32, frequency: i32, seed: u32, wrap: bool) -> f32 {
        let frequency = frequency.max(1);
        let (u, v) = if wrap {
            (u.rem_euclid(1.0), v.rem_euclid(1.0))
        } else {
            (u, v)
        };
        let x = u * frequency as f32;
        let y = v * frequency as f32;
        let x0 = x.floor() as i32;
        let y0 = y.floor() as i32;
        let x1 = x0 + 1;
        let y1 = y0 + 1;
        let fx = Self::smoothstep_unit(x.fract());
        let fy = Self::smoothstep_unit(y.fract());

        let sample = |ix: i32, iy: i32| -> f32 {
            let (sx, sy) = if wrap {
                (ix.rem_euclid(frequency), iy.rem_euclid(frequency))
            } else {
                (ix, iy)
            };
            Self::hash2(sx, sy, seed)
        };

        let v00 = sample(x0, y0);
        let v10 = sample(x1, y0);
        let v01 = sample(x0, y1);
        let v11 = sample(x1, y1);
        let vx0 = v00 * (1.0 - fx) + v10 * fx;
        let vx1 = v01 * (1.0 - fx) + v11 * fx;
        (vx0 * (1.0 - fy) + vx1 * fy).clamp(0.0, 1.0)
    }

    fn remap_unit(value: f32, low: f32, high: f32) -> f32 {
        let span = (high - low).max(0.000_1);
        ((value - low) / span).clamp(0.0, 1.0)
    }

    fn layout_repeat_from_scale(scale: f32) -> i32 {
        ((scale.clamp(0.05, 2.0) * 6.0).round() as i32).max(1)
    }

    fn box_divide_repeat_from_density(density: f32) -> i32 {
        (1 + (density.clamp(0.0, 0.2) * 20.0).round() as i32).max(1)
    }

    fn box_divide_iterations_from_density(density: f32) -> i32 {
        (1 + (density.clamp(0.0, 0.2) / 0.2 * 5.0).round() as i32).max(1)
    }

    fn voronoi_data(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
        falloff: f32,
    ) -> (f32, f32, f32) {
        let repeat = Self::layout_repeat_from_scale(scale);
        let u = (eval.group_u() + warp.x).rem_euclid(1.0);
        let v = (eval.group_v() + warp.y).rem_euclid(1.0);
        let x = u * repeat as f32;
        let y = v * repeat as f32;
        let cell_x = x.floor() as i32;
        let cell_y = y.floor() as i32;
        let frac_x = x.fract();
        let frac_y = y.fract();
        let jitter = jitter.clamp(0.0, 1.0);
        let mut min_dist = f32::MAX;
        let mut second_dist = f32::MAX;
        let mut nearest = (cell_x, cell_y);
        for oy in -1..=1 {
            for ox in -1..=1 {
                let sx = cell_x + ox;
                let sy = cell_y + oy;
                let wx = sx.rem_euclid(repeat);
                let wy = sy.rem_euclid(repeat);
                let px = 0.5 + (Self::hash2(wx, wy, seed) - 0.5) * jitter;
                let py = 0.5 + (Self::hash2(wx, wy, seed ^ 0x9e37_79b9) - 0.5) * jitter;
                let dx = ox as f32 + px - frac_x;
                let dy = oy as f32 + py - frac_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist {
                    second_dist = min_dist;
                    min_dist = dist;
                    nearest = (wx, wy);
                } else if dist < second_dist {
                    second_dist = dist;
                }
            }
        }
        let center = (1.0 - (min_dist / 1.4142)).clamp(0.0, 1.0);
        let edge_distance = ((second_dist - min_dist) / second_dist.max(0.0001)).clamp(0.0, 1.0);
        let height = edge_distance.powf(falloff.clamp(0.1, 4.0));
        let id = Self::hash2(nearest.0, nearest.1, seed ^ 0x51f1_5e11);
        (center, height, id)
    }

    fn voronoi_center(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
    ) -> f32 {
        Self::voronoi_data(eval, warp, scale, seed, jitter, 1.0).0
    }

    fn voronoi_height(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
        falloff: f32,
    ) -> f32 {
        Self::voronoi_data(eval, warp, scale, seed, jitter, falloff).1
    }

    fn voronoi_cell_id(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
    ) -> f32 {
        Self::voronoi_data(eval, warp, scale, seed, jitter, 1.0).2
    }

    fn brick_data(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        columns: u16,
        rows: u16,
        staggered: bool,
        offset: f32,
        falloff: f32,
    ) -> (f32, f32, f32) {
        let cols = columns.max(1) as i32;
        let rows = rows.max(1) as i32;
        let u = (eval.group_u() + warp.x).rem_euclid(1.0);
        let v = (eval.group_v() + warp.y).rem_euclid(1.0);
        let gv = v * rows as f32;
        let row = gv.floor() as i32;
        let gu = u * cols as f32
            + if staggered && row & 1 == 1 {
                offset
            } else {
                0.0
            };
        let brick_x = gu.rem_euclid(cols as f32);
        let col = brick_x.floor() as i32;
        let local_x = brick_x.fract();
        let local_y = gv.fract();

        let dx = ((local_x - 0.5).abs() * 2.0).clamp(0.0, 1.0);
        let dy = ((local_y - 0.5).abs() * 2.0).clamp(0.0, 1.0);
        let center = (1.0 - ((dx * dx + dy * dy).sqrt() / 1.4142)).clamp(0.0, 1.0);

        let edge =
            (local_x.min(1.0 - local_x).min(local_y.min(1.0 - local_y)) * 2.0).clamp(0.0, 1.0);
        let height = edge.powf(falloff.clamp(0.1, 4.0));

        let id = Self::hash2(col.rem_euclid(cols), row.rem_euclid(rows), 0x61c8_8647);
        (center, height, id)
    }

    fn brick_center(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        columns: u16,
        rows: u16,
        staggered: bool,
        offset: f32,
    ) -> f32 {
        Self::brick_data(eval, warp, columns, rows, staggered, offset, 1.0).0
    }

    fn brick_height(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        columns: u16,
        rows: u16,
        staggered: bool,
        offset: f32,
        falloff: f32,
    ) -> f32 {
        Self::brick_data(eval, warp, columns, rows, staggered, offset, falloff).1
    }

    fn brick_cell_id(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        columns: u16,
        rows: u16,
        staggered: bool,
        offset: f32,
    ) -> f32 {
        Self::brick_data(eval, warp, columns, rows, staggered, offset, 1.0).2
    }

    fn disc_data(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
        radius: f32,
        falloff: f32,
    ) -> (f32, f32, f32) {
        let repeat = Self::layout_repeat_from_scale(scale);
        let u = (eval.group_u() + warp.x).rem_euclid(1.0);
        let v = (eval.group_v() + warp.y).rem_euclid(1.0);
        let x = u * repeat as f32;
        let y = v * repeat as f32;
        let cell_x = x.floor() as i32;
        let cell_y = y.floor() as i32;
        let frac_x = x.fract();
        let frac_y = y.fract();
        let jitter = jitter.clamp(0.0, 1.0);
        let radius = radius.clamp(0.05, 1.0);

        let mut min_dist = f32::MAX;
        let mut nearest = (cell_x, cell_y);
        for oy in -1..=1 {
            for ox in -1..=1 {
                let sx = cell_x + ox;
                let sy = cell_y + oy;
                let wx = sx.rem_euclid(repeat);
                let wy = sy.rem_euclid(repeat);
                let px = 0.5 + (Self::hash2(wx, wy, seed) - 0.5) * jitter;
                let py = 0.5 + (Self::hash2(wx, wy, seed ^ 0x9e37_79b9) - 0.5) * jitter;
                let dx = ox as f32 + px - frac_x;
                let dy = oy as f32 + py - frac_y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_dist {
                    min_dist = dist;
                    nearest = (wx, wy);
                }
            }
        }

        let radius_cells = 0.5 * radius;
        let center = (1.0 - (min_dist / radius_cells.max(0.0001))).clamp(0.0, 1.0);
        let height = center.powf(falloff.clamp(0.1, 4.0));
        let id = Self::hash2(nearest.0, nearest.1, seed ^ 0x2f6e_2b1d);
        (center, height, id)
    }

    fn disc_center(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
        radius: f32,
    ) -> f32 {
        Self::disc_data(eval, warp, scale, seed, jitter, radius, 1.0).0
    }

    fn disc_height(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
        radius: f32,
        falloff: f32,
    ) -> f32 {
        Self::disc_data(eval, warp, scale, seed, jitter, radius, falloff).1
    }

    fn disc_cell_id(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
    ) -> f32 {
        Self::disc_data(eval, warp, scale, seed, jitter, 1.0, 1.0).2
    }

    fn box_divide_data(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        gap: f32,
        rotation: f32,
        rounding: f32,
        falloff: f32,
    ) -> (f32, f32, f32) {
        let repeat = Self::box_divide_repeat_from_density(scale);
        let iterations = Self::box_divide_iterations_from_density(scale);
        let u = (eval.group_u() + warp.x).rem_euclid(1.0);
        let v = (eval.group_v() + warp.y).rem_euclid(1.0);
        let x = u * repeat as f32;
        let y = v * repeat as f32;
        let cell_x = x.floor() as i32;
        let cell_y = y.floor() as i32;
        let local = Vec2::new(x.fract(), y.fract());
        let wrapped_cell = Vec2::new(
            cell_x.rem_euclid(repeat) as f32,
            cell_y.rem_euclid(repeat) as f32,
        );
        let (dist, id) = box_divide(
            local,
            wrapped_cell,
            gap.clamp(0.0, 4.0),
            rotation,
            rounding.clamp(0.0, 0.5),
            iterations,
        );
        let center = (1.0 - (dist.abs() * 6.0)).clamp(0.0, 1.0);
        let height = (1.0 - (dist.max(0.0) * 12.0))
            .clamp(0.0, 1.0)
            .powf(falloff.clamp(0.1, 4.0));
        (center, height, id)
    }

    fn box_divide_center(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        gap: f32,
        rotation: f32,
        rounding: f32,
    ) -> f32 {
        Self::box_divide_data(eval, warp, scale, gap, rotation, rounding, 1.0).0
    }

    fn box_divide_height(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        gap: f32,
        rotation: f32,
        rounding: f32,
        falloff: f32,
    ) -> f32 {
        Self::box_divide_data(eval, warp, scale, gap, rotation, rounding, falloff).1
    }

    fn box_divide_cell_id(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        gap: f32,
        rotation: f32,
        rounding: f32,
    ) -> f32 {
        Self::box_divide_data(eval, warp, scale, gap, rotation, rounding, 1.0).2
    }
}

fn material_to_color(material: (f32, f32, f32, f32)) -> TheColor {
    TheColor::from_u8_array([
        unit_to_u8(material.0),
        unit_to_u8(material.1),
        unit_to_u8(material.2),
        unit_to_u8(material.3),
    ])
}

pub fn unit_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}
