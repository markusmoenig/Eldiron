use rayon::prelude::*;
use rustc_hash::FxHashSet;
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

fn box_divide(p: Vec2<f32>, gap: f32, rotation: f32, rounding: f32) -> (f32, f32) {
    fn s_box(p: Vec2<f32>, b: Vec2<f32>, r: f32) -> f32 {
        let d = p.map(|v| v.abs()) - b + Vec2::new(r, r);
        d.x.max(d.y).min(0.0) + (d.map(|v| v.max(0.0))).magnitude() - r
    }

    let mut p = p;
    let ip = p.map(|v| v.floor());
    p -= ip;

    let mut l = Vec2::new(1.0, 1.0);
    let mut last_l;
    let mut r = hash21(ip);

    for _ in 0..6 {
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

    let id = hash21(ip + l);
    p = rot((id - 0.5) * rotation) * p;

    let th = l * 0.02 * gap;
    let c = s_box(p, Vec2::new(0.5, 0.5) - th, rounding);

    (c, id)
}

fn default_tile_node_nodes() -> Vec<TileNodeState> {
    vec![TileNodeState {
        kind: TileNodeKind::default_output_root(),
        position: (420, 40),
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TileNodeGraphExchange {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub graph_name: String,
    #[serde(default)]
    pub palette_colors: Vec<TheColor>,
    pub output_grid_width: u16,
    pub output_grid_height: u16,
    pub tile_pixel_width: u16,
    pub tile_pixel_height: u16,
    #[serde(default)]
    pub graph_state: TileNodeGraphState,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TileNodeState {
    pub kind: TileNodeKind,
    pub position: (i32, i32),
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
            bypass: false,
            mute: false,
            solo: false,
        }
    }
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
    },
    Subgraph {
        subgraph_id: Uuid,
    },
    GroupUV,
    Scalar {
        value: f32,
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
    MaskBlend {
        factor: f32,
    },
    Levels {
        level: f32,
        width: f32,
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

impl TileNodeKind {
    pub fn default_output_root() -> Self {
        Self::OutputRoot {
            roughness: default_output_roughness(),
            metallic: default_output_metallic(),
            opacity: default_output_opacity(),
            emissive: default_output_emissive(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct TileEvalContext {
    pub cell_x: u16,
    pub cell_y: u16,
    pub group_width: u16,
    pub group_height: u16,
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
}

pub trait TileGraphSubgraphResolver {
    fn resolve_subgraph_state(&self, subgraph_id: Uuid) -> Option<TileNodeGraphState>;
}

pub struct NoTileGraphSubgraphs;

impl TileGraphSubgraphResolver for NoTileGraphSubgraphs {
    fn resolve_subgraph_state(&self, _subgraph_id: Uuid) -> Option<TileNodeGraphState> {
        None
    }
}

#[derive(Clone, Copy, Default)]
struct FlatSubgraphOutputs {
    color: Option<u16>,
    material: Option<u16>,
}

pub fn flatten_graph_exchange_with<R: TileGraphSubgraphResolver>(
    graph: &TileNodeGraphExchange,
    resolver: &R,
) -> TileNodeGraphExchange {
    let mut flattened = graph.clone();
    flattened.graph_state = flatten_graph_state_with(&graph.graph_state, resolver);
    flattened
}

pub fn flatten_graph_state_with<R: TileGraphSubgraphResolver>(
    state: &TileNodeGraphState,
    resolver: &R,
) -> TileNodeGraphState {
    let mut state = state.clone();
    state.ensure_root();
    flatten_graph_state_recursive(&state, resolver, &mut FxHashSet::default())
}

fn flatten_graph_state_recursive<R: TileGraphSubgraphResolver>(
    state: &TileNodeGraphState,
    resolver: &R,
    stack: &mut FxHashSet<Uuid>,
) -> TileNodeGraphState {
    let mut nodes = Vec::new();
    let mut node_map: Vec<Option<u16>> = vec![None; state.nodes.len()];
    let mut subgraph_outputs: Vec<FlatSubgraphOutputs> =
        vec![FlatSubgraphOutputs::default(); state.nodes.len()];
    let mut connections = Vec::new();

    if let Some(root) = state.nodes.first() {
        nodes.push(root.clone());
        node_map[0] = Some(0);
    }

    for (old_index, node) in state.nodes.iter().enumerate().skip(1) {
        match &node.kind {
            TileNodeKind::Subgraph { subgraph_id } => {
                if !stack.insert(*subgraph_id) {
                    continue;
                }
                let Some(sub_state) = resolver.resolve_subgraph_state(*subgraph_id) else {
                    stack.remove(subgraph_id);
                    continue;
                };
                let sub_flat = flatten_graph_state_recursive(&sub_state, resolver, stack);
                stack.remove(subgraph_id);

                let base = nodes.len() as u16;
                let mut sub_map: Vec<Option<u16>> = vec![None; sub_flat.nodes.len()];
                for (sub_index, sub_node) in sub_flat.nodes.iter().enumerate().skip(1) {
                    let new_index = nodes.len() as u16;
                    nodes.push(sub_node.clone());
                    sub_map[sub_index] = Some(new_index);
                }

                let color_src = input_connection_source(&sub_flat, 0, 0)
                    .and_then(|src| remap_sub_index(src, &sub_map, base));
                let material_src = input_connection_source(&sub_flat, 0, 1)
                    .and_then(|src| remap_sub_index(src, &sub_map, base));
                subgraph_outputs[old_index] = FlatSubgraphOutputs {
                    color: color_src,
                    material: material_src,
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
            Some(TileNodeKind::Subgraph { .. })
        ) {
            let outputs = subgraph_outputs[*src_node as usize];
            if *src_terminal == 1 {
                outputs.material
            } else {
                outputs.color
            }
        } else {
            node_map.get(*src_node as usize).and_then(|v| *v)
        };
        let dest = node_map.get(*dest_node as usize).and_then(|v| *v);
        if let (Some(src), Some(dest)) = (src, dest) {
            connections.push((src, *src_terminal, dest, *dest_terminal));
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
}

impl TileGraphRenderer {
    pub fn new(palette: Vec<TheColor>) -> Self {
        Self { palette }
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
        let mut sheet_color = vec![0_u8; sheet_width * sheet_height * 4];
        let mut sheet_material = vec![0_u8; sheet_width * sheet_height * 4];
        let mut sheet_height_data = vec![0_u8; sheet_width * sheet_height];
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

        for (tile_index, (tile_color, tile_material, tile_height_data)) in
            rendered_tiles.into_iter().enumerate()
        {
            let cell_x = tile_index % grid_width;
            let cell_y = tile_index / grid_width;

            for py in 0..tile_height {
                let sx = cell_x * tile_width;
                let sy = cell_y * tile_height + py;
                let src_row_start = py * tile_width * 4;
                let src_row_end = src_row_start + tile_width * 4;
                let dst_row_start = (sy * sheet_width + sx) * 4;
                let dst_row_end = dst_row_start + tile_width * 4;
                sheet_color[dst_row_start..dst_row_end]
                    .copy_from_slice(&tile_color[src_row_start..src_row_end]);
                sheet_material[dst_row_start..dst_row_end]
                    .copy_from_slice(&tile_material[src_row_start..src_row_end]);
                let src_h_row_start = py * tile_width;
                let src_h_row_end = src_h_row_start + tile_width;
                let dst_h_row_start = sy * sheet_width + sx;
                let dst_h_row_end = dst_h_row_start + tile_width;
                sheet_height_data[dst_h_row_start..dst_h_row_end]
                    .copy_from_slice(&tile_height_data[src_h_row_start..src_h_row_end]);
            }

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
                TileNodeKind::Subgraph { subgraph_id } => {
                    let _ = visiting_subgraphs.insert(*subgraph_id);
                    None
                }
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

    fn voronoi_warp_vector(
        &self,
        state: &TileNodeGraphState,
        node_index: usize,
        eval: TileEvalContext,
        amount: f32,
        visiting: &mut FxHashSet<usize>,
        visiting_subgraphs: &mut FxHashSet<Uuid>,
    ) -> Vec2<f32> {
        if amount <= f32::EPSILON || self.input_connection(state, node_index, 0).is_none() {
            return Vec2::new(0.0, 0.0);
        }

        let sx = self
            .evaluate_connected_scalar(state, node_index, 0, eval, visiting, visiting_subgraphs)
            .unwrap_or(0.5);
        let wrapped_u = (eval.group_u() + 0.173).rem_euclid(1.0);
        let wrapped_v = (eval.group_v() + 0.317).rem_euclid(1.0);
        let sy = self
            .evaluate_connected_scalar(
                state,
                node_index,
                0,
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
                TileNodeKind::Subgraph { subgraph_id } => {
                    let _ = visiting_subgraphs.insert(*subgraph_id);
                    None
                }
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
                    let warp = self.voronoi_warp_vector(
                        state,
                        node_index,
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
                    let warp = self.voronoi_warp_vector(
                        state,
                        node_index,
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
                    offset,
                    warp_amount,
                    falloff,
                } => {
                    let warp = self.voronoi_warp_vector(
                        state,
                        node_index,
                        eval,
                        warp_amount.clamp(0.0, 0.25),
                        visiting,
                        visiting_subgraphs,
                    );
                    let value = match output_terminal {
                        1 => Self::brick_height(eval, warp, *columns, *rows, *offset, *falloff),
                        2 => Self::brick_cell_id(eval, warp, *columns, *rows, *offset),
                        _ => Self::brick_center(eval, warp, *columns, *rows, *offset),
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
                    let warp = self.voronoi_warp_vector(
                        state,
                        node_index,
                        eval,
                        warp_amount.clamp(0.0, 0.25),
                        visiting,
                        visiting_subgraphs,
                    );
                    let value = match output_terminal {
                        1 => {
                            Self::disc_height(eval, warp, *scale, *seed, *jitter, *radius, *falloff)
                        }
                        2 => Self::disc_cell_id(eval, warp, *scale, *seed, *jitter),
                        _ => Self::disc_center(eval, warp, *scale, *seed, *jitter, *radius),
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

    fn voronoi_data(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        scale: f32,
        seed: u32,
        jitter: f32,
        falloff: f32,
    ) -> (f32, f32, f32) {
        let repeat = ((scale.clamp(0.01, 1.0) * 16.0).round() as i32).max(1);
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
        offset: f32,
        falloff: f32,
    ) -> (f32, f32, f32) {
        let cols = columns.max(1) as i32;
        let rows = rows.max(1) as i32;
        let u = (eval.group_u() + warp.x).rem_euclid(1.0);
        let v = (eval.group_v() + warp.y).rem_euclid(1.0);
        let gv = v * rows as f32;
        let row = gv.floor() as i32;
        let gu = u * cols as f32 + if row & 1 == 1 { offset } else { 0.0 };
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
        offset: f32,
    ) -> f32 {
        Self::brick_data(eval, warp, columns, rows, offset, 1.0).0
    }

    fn brick_height(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        columns: u16,
        rows: u16,
        offset: f32,
        falloff: f32,
    ) -> f32 {
        Self::brick_data(eval, warp, columns, rows, offset, falloff).1
    }

    fn brick_cell_id(
        eval: TileEvalContext,
        warp: Vec2<f32>,
        columns: u16,
        rows: u16,
        offset: f32,
    ) -> f32 {
        Self::brick_data(eval, warp, columns, rows, offset, 1.0).2
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
        let repeat = ((scale.clamp(0.01, 1.0) * 16.0).round() as i32).max(1);
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
        let u = (eval.group_u() + warp.x).rem_euclid(1.0);
        let v = (eval.group_v() + warp.y).rem_euclid(1.0);
        let uv = Vec2::new(u, v) * scale.max(0.1);
        let (dist, id) = box_divide(uv, gap.clamp(0.0, 4.0), rotation, rounding.clamp(0.0, 0.5));
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
