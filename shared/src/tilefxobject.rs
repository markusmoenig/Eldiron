use crate::prelude::*;
use theframework::prelude::*;

/// A character instance.
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileFXObject {
    /// The nodes which make up the effect.
    pub nodes: Vec<MaterialFXNode>,

    /// The node connections: Source node index, source terminal, dest node index, dest terminal
    pub connections: Vec<(u16, u8, u16, u8)>,

    //#[serde(skip)]
    //pub node_previews: Vec<Option<TheRGBABuffer>>,
    pub zoom: f32,
    pub selected_node: Option<usize>,

    #[serde(default = "Vec2i::zero")]
    pub scroll_offset: Vec2i,
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
}
