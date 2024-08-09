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

    /// Computes the material
    pub fn compute(
        &self,
        _hit: &mut Hit,
        _palette: &ThePalette,
        _textures: &FxHashMap<Uuid, TheRGBATile>,
        _fx_obj_params: &[Vec<f32>],
    ) {
        //self.follow_trail(0, 0, hit, palette, textures, mat_obj_params);
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
