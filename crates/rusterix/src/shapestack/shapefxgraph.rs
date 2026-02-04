use crate::{
    Assets, BBox, Linedef, Map, Pixel, Sector, ShapeContext, ShapeFX, ShapeFXModifierPass,
    ShapeFXRole, Terrain, TerrainChunk, Texture,
};
use rayon::prelude::*;
use theframework::prelude::*;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeFXGraph {
    pub id: Uuid,
    pub nodes: Vec<ShapeFX>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    pub selected_node: Option<usize>,

    pub scroll_offset: Vec2<i32>,
    pub zoom: f32,
}

impl Default for ShapeFXGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ShapeFXGraph {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            nodes: vec![],
            connections: vec![],
            selected_node: None,
            scroll_offset: Vec2::zero(),
            zoom: 1.0,
        }
    }

    /// Modifies the terrain with the given geometry nodes for the given sector
    #[allow(clippy::too_many_arguments)]
    pub fn sector_modify_heightmap(
        &self,
        sector: &Sector,
        map: &Map,
        terrain: &Terrain,
        bounds: &BBox,
        chunk: &TerrainChunk,
        heights: &mut FxHashMap<(i32, i32), f32>,
        assets: &Assets,
        texture: &mut Texture,
        pass: ShapeFXModifierPass,
    ) {
        if self.nodes.is_empty() {
            return;
        }
        if self.nodes[0].role != ShapeFXRole::SectorGeometry {
            return;
        }
        let mut curr_index = 0_usize;
        let mut curr_terminal = 0_usize;

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                if self.nodes[next_node as usize].supports_modifier_pass(pass) {
                    self.nodes[next_node as usize].sector_modify_heightmap(
                        sector,
                        map,
                        terrain,
                        bounds,
                        chunk,
                        heights,
                        (self, next_node as usize),
                        assets,
                        texture,
                        pass,
                    );
                }
                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }
    }

    /// Modifies the terrain with the given geometry nodes for the given sector
    #[allow(clippy::too_many_arguments)]
    pub fn linedef_modify_heightmap(
        &self,
        linedef: &Vec<Linedef>,
        map: &Map,
        terrain: &Terrain,
        bounds: &BBox,
        chunk: &TerrainChunk,
        heights: &mut FxHashMap<(i32, i32), f32>,
        assets: &Assets,
        texture: &mut Texture,
        pass: ShapeFXModifierPass,
    ) {
        if self.nodes.is_empty() {
            return;
        }
        if self.nodes[0].role != ShapeFXRole::LinedefGeometry {
            return;
        }
        let mut curr_index = 0_usize;
        let mut curr_terminal = 0_usize;

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                if self.nodes[next_node as usize].supports_modifier_pass(pass) {
                    self.nodes[next_node as usize].linedef_modify_heightmap(
                        linedef,
                        map,
                        terrain,
                        bounds,
                        chunk,
                        heights,
                        (self, next_node as usize),
                        assets,
                        texture,
                        pass,
                    );
                }
                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }
    }

    /// Evaluate a shape graph for its distance
    pub fn evaluate_shape_distance(
        &self,
        world_pos: Vec2<f32>,
        vertices: &[Vec2<f32>],
    ) -> (f32, usize) {
        let mut d = (f32::MAX, 0);

        if self.nodes.is_empty() {
            return d;
        }
        if self.nodes[0].role != ShapeFXRole::Shape {
            return d;
        }

        let mut curr_index = 0_usize;
        let mut curr_terminal = 0;

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                if let Some(distance) =
                    self.nodes[next_node as usize].evaluate_distance(world_pos, vertices)
                {
                    if distance < d.0 {
                        d = (distance, next_node as usize);
                    }
                }
                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }
        d
    }

    /// Evaluate a shape node for its color.
    pub fn evaluate_shape_color(
        &self,
        ctx: &ShapeContext,
        node_index: usize,
        assets: &Assets,
    ) -> Option<Vec4<f32>> {
        let mut curr_index = node_index;
        let mut curr_terminal = 1;

        let mut color = None;

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                if let Some(col) = self.nodes[next_node as usize].evaluate_pixel(
                    ctx,
                    color,
                    assets,
                    (self, next_node as usize),
                ) {
                    color = Some(col);
                }

                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }

        color
    }

    /// Evaluate the material graph
    pub fn evaluate_material(
        &self,
        ctx: &ShapeContext,
        mut incoming: Vec4<f32>,
        assets: &Assets,
    ) -> Option<Vec4<f32>> {
        if self.nodes.is_empty() {
            return None;
        }
        if self.nodes[0].role != ShapeFXRole::MaterialGeometry {
            return None;
        }

        let mut curr_index = 0_usize;
        let mut curr_terminal = if ctx.distance > 0.0 { 1_usize } else { 0_usize };

        let mut color = None;

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                if let Some(col) = self.nodes[next_node as usize].evaluate_pixel(
                    ctx,
                    Some(incoming),
                    assets,
                    (self, next_node as usize),
                ) {
                    color = Some(col);
                    incoming = col;
                }
                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }
        color
    }

    /// Returns the connected input node and terminal for the given output node and terminal.
    pub fn find_connected_input_node(
        &self,
        node: usize,
        terminal_index: usize,
    ) -> Option<(u16, u8)> {
        for (o, ot, i, it) in &self.connections {
            if *o == node as u16 && *ot == terminal_index as u8 {
                return Some((*i, *it));
            }
        }
        None
    }

    /// Returns the connected output node for the given input node and terminal.
    pub fn find_connected_output_node(&self, node: usize, terminal_index: usize) -> Option<usize> {
        for (o, _, i, it) in &self.connections {
            if *i == node as u16 && *it == terminal_index as u8 {
                return Some(*o as usize);
            }
        }
        None
    }

    /// Collects all connected nodes from the given start node and terminal.
    pub fn collect_nodes_from(&self, start_node_index: usize, start_terminal: usize) -> Vec<u16> {
        if self.nodes.is_empty() {
            return vec![];
        }

        let mut curr_index = start_node_index;
        let mut curr_terminal = start_terminal;

        let mut connected_nodes = vec![];

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                connected_nodes.push(next_node);
                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }

        connected_nodes
    }

    /// Create a preview of the graph
    pub fn material_preview(&self, buffer: &mut Texture, assets: &Assets) {
        let width = buffer.width;
        let height = buffer.height;

        let px = 1.0;

        buffer
            .data
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let x = i as f32;
                    let y = j as f32;

                    // Normalized UVs
                    let uv = Vec2::new(x / width as f32, 1.0 - y / height as f32);

                    // Centered pixel coordinate in "world space"
                    let world = uv * Vec2::new(width as f32, height as f32);

                    // Simulated distance to nearest edge of the preview "shape"
                    let dist_left = uv.x;
                    let dist_right = 1.0 - uv.x;
                    let dist_top = 1.0 - uv.y;
                    let dist_bottom = uv.y;
                    let edge_distance = dist_left.min(dist_right).min(dist_top).min(dist_bottom);

                    // Optional: scale to world/pixel units if needed
                    let distance = -edge_distance * width.min(height) as f32;

                    // Build ShapeContext with no sector
                    let ctx = ShapeContext {
                        point_world: world,
                        point: world / px,
                        uv,
                        distance_world: distance,
                        distance,
                        shape_id: 0,
                        px,
                        anti_aliasing: 1.0,
                        t: None,
                        line_dir: None,
                        override_color: None,
                    };

                    let color = if let Some(col) =
                        self.evaluate_material(&ctx, Vec4::new(0.0, 0.0, 0.0, 1.0), assets)
                    {
                        col
                    } else {
                        Vec4::new(0.0, 0.0, 0.0, 1.0)
                    };

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });
    }

    /// Get the dominant color of the graph for sector previews
    pub fn get_dominant_color(&self, palette: &ThePalette) -> Pixel {
        let mut pixel = [128, 128, 128, 255];
        if self.nodes.len() > 1 {
            pixel = self.nodes[1].get_dominant_color(palette)
        }
        pixel
    }

    /// Evaluate as a screen widget
    pub fn evaluate_screen_widget(
        &self,
        ctx: &ShapeContext,
        mut incoming: Vec4<f32>,
        terminal: usize,
        assets: &Assets,
    ) -> Option<Vec4<f32>> {
        if self.nodes.is_empty() {
            return None;
        }
        if self.nodes[0].role != ShapeFXRole::Widget {
            return None;
        }

        let mut curr_index = 0_usize;
        let mut curr_terminal = terminal;

        let mut color = None;

        let mut steps = 0;
        while steps < 16 {
            if let Some((next_node, next_terminal)) =
                self.find_connected_input_node(curr_index, curr_terminal)
            {
                if let Some(col) = self.nodes[next_node as usize].evaluate_pixel(
                    ctx,
                    Some(incoming),
                    assets,
                    (self, next_node as usize),
                ) {
                    color = Some(col);
                    incoming = col;
                }
                curr_index = next_node as usize;
                curr_terminal = next_terminal as usize;
                steps += 1;
            } else {
                break;
            }
        }
        color
    }

    /// Create the screen widgets
    pub fn create_screen_widgets(
        &self,
        width: usize,
        height: usize,
        assets: &Assets,
    ) -> Vec<Texture> {
        let px = 1.0;

        let mut textures = vec![Texture::alloc(width, height), Texture::alloc(width, height)];

        for (index, texture) in textures.iter_mut().enumerate() {
            texture
                .data
                .par_rchunks_exact_mut(width * 4)
                .enumerate()
                .for_each(|(j, line)| {
                    for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                        let x = i as f32;
                        let y = j as f32;

                        // Normalized UVs
                        let uv = Vec2::new(x / width as f32, 1.0 - y / height as f32);

                        // Centered pixel coordinate in "world space"
                        let world = uv * Vec2::new(width as f32, height as f32);

                        // Simulated distance to nearest edge of the preview "shape"
                        let dist_left = uv.x;
                        let dist_right = 1.0 - uv.x;
                        let dist_top = 1.0 - uv.y;
                        let dist_bottom = uv.y;
                        let edge_distance =
                            dist_left.min(dist_right).min(dist_top).min(dist_bottom);

                        // Optional: scale to world/pixel units if needed
                        let distance = -edge_distance * width.min(height) as f32;

                        // Build ShapeContext with no sector
                        let ctx = ShapeContext {
                            point_world: world,
                            point: world / px,
                            uv,
                            distance_world: distance,
                            distance,
                            shape_id: 0,
                            px,
                            anti_aliasing: 1.0,
                            t: None,
                            line_dir: None,
                            override_color: None,
                        };

                        let color = if let Some(col) = self.evaluate_screen_widget(
                            &ctx,
                            Vec4::new(0.0, 0.0, 0.0, 1.0),
                            index,
                            assets,
                        ) {
                            col
                        } else {
                            Vec4::new(0.0, 0.0, 0.0, 1.0)
                        };

                        pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                    }
                });
        }

        textures
    }
}
