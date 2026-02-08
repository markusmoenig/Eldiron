use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileFXObject {
    /// The nodes which make up the effect.
    pub nodes: Vec<TileFXNode>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    //#[serde(skip)]
    //pub node_previews: Vec<Option<TheRGBABuffer>>,
    pub zoom: f32,
    pub selected_node: Option<usize>,

    pub scroll_offset: Vec2<i32>,
}

impl Default for TileFXObject {
    fn default() -> Self {
        Self::new()
    }
}

impl TileFXObject {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            connections: Vec::new(),

            // node_previews: Vec::new(),
            zoom: 1.0,
            selected_node: None,

            scroll_offset: Vec2::zero(),
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

    /// Get the light node properties (if any).
    pub fn get_light_collection(&self) -> Option<TheCollection> {
        for n in &self.nodes {
            if n.role == TileFXNodeRole::LightEmitter {
                return Some(n.collection());
            }
        }
        None
    }

    /// Computes the 3D region fx.
    pub fn fx(
        &self,
        region: &Region,
        palette: &ThePalette,
        pos: Vec3<f32>,
        color: &mut Vec3<f32>,
        three_d: bool,
        fx_obj_params: &[Vec<f32>],
    ) {
        for (index, node) in self.nodes.iter().enumerate() {
            node.fx(region, palette, pos, color, three_d, &fx_obj_params[index]);
        }
    }

    /// Convert the model to a node canvas.
    pub fn to_canvas(&mut self) -> TheNodeCanvas {
        let mut canvas = TheNodeCanvas {
            node_width: 136,
            selected_node: self.selected_node,
            offset: self.scroll_offset,
            ..Default::default()
        };

        for node in self.nodes.iter() {
            let n = TheNode {
                name: node.name(),
                position: node.position,
                inputs: node.inputs(),
                outputs: node.outputs(),
                preview: node.preview.clone(),
                supports_preview: node.supports_preview,
                preview_is_open: node.preview_is_open,
                can_be_deleted: true,
            };
            canvas.nodes.push(n);
        }
        canvas.connections.clone_from(&self.connections);
        canvas.zoom = self.zoom;
        canvas.selected_node = self.selected_node;

        canvas
    }
}
