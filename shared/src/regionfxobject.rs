use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionFXObject {
    /// The nodes which make up the effect.
    pub nodes: Vec<RegionFXNode>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    pub zoom: f32,
    pub selected_node: Option<usize>,

    #[serde(default = "Vec2i::zero")]
    pub scroll_offset: Vec2i,
}

impl Default for RegionFXObject {
    fn default() -> Self {
        Self::new()
    }
}

impl RegionFXObject {
    pub fn new() -> Self {
        let mut nodes = vec![];

        let mut node = RegionFXNode::new_from_name(str!("Renderer"));
        node.position = vec2i(220, 50);
        nodes.push(node);

        let mut node = RegionFXNode::new_from_name(str!("Tilted Iso Camera"));
        node.position = vec2i(20, 20);
        nodes.push(node);

        let connections = vec![(1, 0, 0, 0)];

        Self {
            nodes,
            connections,

            zoom: 1.0,
            selected_node: None,

            scroll_offset: Vec2i::zero(),
        }
    }

    /// Gives a chance to each node to update its parameters in case things changed.
    pub fn update_parameters(&mut self) {
        for n in &mut self.nodes {
            n.update_parameters();
        }
    }

    /// Loads the parameters of the nodes into memory for faster access.
    pub fn load_parameters(&self, time: &TheTime) -> Vec<Vec<f32>> {
        let mut data = vec![];

        for n in &self.nodes {
            data.push(n.load_parameters(time));
        }
        data
    }

    /// Get render settings
    pub fn get_render_settings(&self) -> TheCollection {
        if let Some(renderer) = self.nodes.first() {
            renderer.collection()
        } else {
            TheCollection::default()
        }
    }

    /// Convert a world position into a pixel offset in the canvas.
    pub fn cam_world_to_canvas(&self, region: &Region, world_pos: Vec3f) -> Vec2i {
        if let Some(node_index) = self.find_connected_output_node(0, 0) {
            self.nodes[node_index].cam_world_to_canvas(region, world_pos)
        } else {
            Vec2i::zero()
        }
    }

    /// Convert a canvas pixel position into a world position.
    pub fn cam_canvas_to_world(&self, region: &Region, canvas_pos: Vec2i) -> Vec3f {
        if let Some(node_index) = self.find_connected_output_node(0, 0) {
            self.nodes[node_index].cam_canvas_to_world(region, canvas_pos)
        } else {
            Vec3f::zero()
        }
    }

    /// Render the prerendered tiles into the game canvas.
    pub fn cam_render_canvas(&self, region: &Region, canvas: &mut GameCanvas) {
        if let Some(node_index) = self.find_connected_output_node(0, 0) {
            self.nodes[node_index].cam_render_canvas(region, canvas)
        }
    }

    /// Render the prerendered tiles into the game canvas.
    pub fn cam_region_size(&self, region: &Region) -> Vec2i {
        if let Some(node_index) = self.find_connected_output_node(0, 0) {
            self.nodes[node_index].cam_region_size(region)
        } else {
            Vec2i::zero()
        }
    }

    /// Get the camera role.
    pub fn get_camera_node(&self) -> Option<&RegionFXNode> {
        self.find_connected_output_node(0, 0)
            .map(|node_index| &self.nodes[node_index])
    }

    /// Create a camera ray
    pub fn cam_create_ray(
        &self,
        uv: Vec2f,
        position: Vec3f,
        size: Vec2f,
        offset: Vec2f,
        params: &[Vec<f32>],
    ) -> Ray {
        if let Some(node_index) = self.find_connected_output_node(0, 0) {
            self.nodes[node_index].cam_create_ray(uv, position, size, offset, &params[node_index])
        } else {
            Ray::new(Vec3f::zero(), Vec3f::zero())
        }
    }

    /// Computes the 2D region fx.
    pub fn fx_2d(
        &self,
        region: &Region,
        palette: &ThePalette,
        canvas_pos: Vec2i,
        color: &mut Vec3f,
        fx_obj_params: &[Vec<f32>],
    ) {
        self.follow_trail(0, 0, region, palette, canvas_pos, color, fx_obj_params)
    }

    /// Computes the 3D region fx.
    pub fn fx_3d(
        &self,
        region: &Region,
        palette: &ThePalette,
        canvas_pos: Vec2i,
        color: &mut Vec3f,
        fx_obj_params: &[Vec<f32>],
    ) {
        self.follow_trail(0, 1, region, palette, canvas_pos, color, fx_obj_params)
    }

    /// After exiting a geometry node follow the trail of material nodes to calculate the final material.
    #[allow(clippy::too_many_arguments)]
    pub fn follow_trail(
        &self,
        node: usize,
        terminal_index: usize,
        region: &Region,
        palette: &ThePalette,
        canvas_pos: Vec2i,
        color: &mut Vec3f,
        fx_obj_params: &[Vec<f32>],
    ) {
        let mut connections = vec![];
        for (o, ot, i, it) in &self.connections {
            if *o == node as u16 && *ot == terminal_index as u8 {
                connections.push((*i, *it));
            }
        }

        for (o, _) in connections {
            if let Some(ot) = self.nodes[o as usize].fx(
                region,
                palette,
                canvas_pos,
                color,
                &fx_obj_params[o as usize],
            ) {
                self.follow_trail(
                    o as usize,
                    ot as usize,
                    region,
                    palette,
                    canvas_pos,
                    color,
                    fx_obj_params,
                );
            }
        }
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

    /// Convert the model to a node canvas.
    pub fn to_canvas(&mut self) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 136,
            selected_node: self.selected_node,
            offset: self.scroll_offset,
            ..Default::default()
        };

        for (index, node) in self.nodes.iter().enumerate() {
            let n = TheNode {
                name: node.name(),
                position: node.position,
                inputs: node.inputs(),
                outputs: node.outputs(),
                preview: node.preview.clone(),
                supports_preview: node.supports_preview,
                preview_is_open: node.preview_is_open,
                can_be_deleted: index != 0,
            };
            canvas.nodes.push(n);
        }
        canvas.connections.clone_from(&self.connections);
        canvas.zoom = self.zoom;
        canvas.selected_node = self.selected_node;

        canvas
    }
}
