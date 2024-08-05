use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TileFXNodeRole {
    Brightness,
}

use TileFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileFXNode {
    pub id: Uuid,
    pub role: TileFXNodeRole,
    pub timeline: TheTimeline,

    pub position: Vec2i,

    pub supports_preview: bool,
    pub preview_is_open: bool,

    pub preview: TheRGBABuffer,

    pub resolve_branches: bool,

    pub texture_id: Option<Uuid>,
}

impl TileFXNode {
    pub fn new(role: TileFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Props"));
        let mut supports_preview = false;
        let mut preview_is_open = false;
        let mut resolve_branches = false;

        match role {
            Brightness => {
                coll.set("Add", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Rounding", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set(
                    "Profile",
                    TheValue::TextList(0, vec![str!("None"), str!("Rounded")]),
                );
                coll.set("Steps", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set(
                    "Mortar",
                    TheValue::TextList(0, vec![str!("No"), str!("Yes")]),
                );
                coll.set("Mortar Sub", TheValue::FloatRange(0.05, 0.0..=1.0));
                coll.set("Hash Weight", TheValue::FloatRange(0.0, 0.0..=1.0));
                supports_preview = true;
                preview_is_open = true;
            }
        }

        let timeline = TheTimeline::collection(coll);

        Self {
            id: Uuid::new_v4(),
            role,
            timeline,
            position: Vec2i::new(10, 5),
            supports_preview,
            preview_is_open,
            preview: TheRGBABuffer::empty(),
            resolve_branches,
            texture_id: None,
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            Brightness => str!("Brightness"),
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![Self::new(TileFXNodeRole::Brightness)]
    }

    /// Gives the node a chance to update its parameters in case things changed.
    pub fn update_parameters(&mut self) {
        // match self.role {
        //     Geometry => {
        //         self.set("Hash Weight", TheValue::FloatRange(0.0, 0.0..=1.0));
        //     }
        //     _ => {}
        // }
    }

    /// Loads the parameters of the nodes into memory for faster access.
    pub fn load_parameters(&self, _time: &TheTime) -> Vec<f32> {
        let mut params = vec![];

        let coll = self.collection();

        match self.role {
            TileFXNodeRole::Brightness => {
                params.push(coll.get_f32_default("Add", 0.0));
                params.push(coll.get_f32_default("Rounding", 0.0));
                params.push(coll.get_i32_default("Profile", 0) as f32);
                params.push(coll.get_f32_default("Steps", 0.0));
                params.push(coll.get_i32_default("Mortar", 0) as f32);
                params.push(coll.get_f32_default("Mortar Sub", 0.05));
                params.push(coll.get_f32_default("Hash Weight", 0.0));
            }
            _ => {}
        }

        params
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Brightness => {
                vec![]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Brightness => {
                vec![]
            } //_ => vec![],
        }
    }

    /// Computes the node.
    pub fn compute(
        &self,
        hit: &mut Hit,
        palette: &ThePalette,
        textures: &FxHashMap<Uuid, TheRGBATile>,
        resolved: Vec<Hit>,
        params: &[f32],
    ) -> Vec4f {
        Vec4f::zero()
    }

    /// Creates a new node from a name.
    pub fn new_from_name(name: String) -> Self {
        let nodes = TileFXNode::nodes();
        for n in nodes {
            if n.name() == name {
                return n;
            }
        }
        TileFXNode::new(Brightness)
    }

    pub fn collection(&self) -> TheCollection {
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Props"))
        {
            return coll;
        }

        TheCollection::default()
    }

    pub fn get(&self, key: &str) -> Option<TheValue> {
        self.timeline.get(
            "Props".to_string(),
            key.to_string(),
            &TheTime::default(),
            TheInterpolation::Linear,
        )
    }

    /// Clears the collection.
    pub fn clear(&mut self) {
        self.timeline.clear_collection(&TheTime::default(), "Props");
    }

    /// Sets a value in the collection.
    pub fn set(&mut self, key: &str, value: TheValue) {
        self.timeline.set(&TheTime::default(), key, "Props", value);
    }

    pub fn render_preview(&mut self, _palette: &ThePalette) {
        let width = 111;
        let height = 104;

        let mut buffer = TheRGBABuffer::new(TheDim::sized(width as i32, height));
        let collection = self.collection();

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut color = Vec4f::zero();

                    match &self.role {
                        Brightness => {}
                        _ => {}
                    }

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });

        self.preview = buffer;
    }
}
