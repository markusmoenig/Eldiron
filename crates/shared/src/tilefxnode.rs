use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum TileFXNodeRole {
    LightEmitter,
    Saturation,
}

use TileFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct TileFXNode {
    pub id: Uuid,
    pub role: TileFXNodeRole,
    pub timeline: TheTimeline,

    pub position: Vec2<i32>,

    pub supports_preview: bool,
    pub preview_is_open: bool,

    pub preview: TheRGBABuffer,

    pub resolve_branches: bool,

    pub texture_id: Option<Uuid>,
}

impl TileFXNode {
    pub fn new(role: TileFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Props"));
        let supports_preview = false;
        let preview_is_open = false;
        let resolve_branches = false;

        match role {
            LightEmitter => {
                coll.set("Strength", TheValue::FloatRange(1.0, 0.1..=3.0));
                coll.set("Max. Distance", TheValue::IntRange(10, 1..=20));
                coll.set("Samples #", TheValue::IntRange(5, 1..=7));
                coll.set("Sample Offset", TheValue::FloatRange(0.5, 0.01..=0.5));
                coll.set(
                    "Limit Direction",
                    TheValue::TextList(
                        0,
                        vec![
                            str!("No"),
                            str!("Only North"),
                            str!("Only East"),
                            str!("Only South"),
                            str!("Only West"),
                        ],
                    ),
                );
                coll.set(
                    "Light Color",
                    TheValue::TextList(0, vec![str!("Color"), str!("Daylight")]),
                );
                coll.set("Color", TheValue::ColorObject(TheColor::white()));
                coll.set("Mask", TheValue::TileMask(TheTileMask::default()));
            }
            Saturation => {
                coll.set("Saturation", TheValue::FloatRange(1.0, 0.0..=2.0));
            }
        }

        let timeline = TheTimeline::collection(coll);

        Self {
            id: Uuid::new_v4(),
            role,
            timeline,
            position: Vec2::new(10, 5),
            supports_preview,
            preview_is_open,
            preview: TheRGBABuffer::empty(),
            resolve_branches,
            texture_id: None,
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            LightEmitter => str!("Light Emitter"),
            Saturation => str!("Saturation"),
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new(TileFXNodeRole::LightEmitter),
            Self::new(TileFXNodeRole::Saturation),
        ]
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

        #[allow(clippy::single_match)]
        match self.role {
            TileFXNodeRole::LightEmitter => {}
            TileFXNodeRole::Saturation => {
                params.push(coll.get_f32_default("Saturation", 1.0));
            } //_ => {}
        }

        params
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Saturation | LightEmitter => {
                vec![]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Saturation | LightEmitter => {
                // vec![TheNodeTerminal {
                //     name: str!("out"),
                //     role: str!("Out"),
                //     color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                // }]
                vec![]
            } //_ => vec![],
        }
    }

    /// Computes the FX.
    pub fn fx(
        &self,
        _region: &Region,
        _palette: &ThePalette,
        _pos: Vec3<f32>,
        color: &mut Vec3<f32>,
        _three_d: bool,
        params: &[f32],
    ) {
        #[allow(clippy::single_match)]
        match self.role {
            TileFXNodeRole::Saturation => {
                let mut hsl = TheColor::from_vec3(*color).as_hsl();
                hsl.y *= params[0];
                *color = TheColor::from_hsl(hsl.x * 360.0, hsl.y.clamp(0.0, 1.0), hsl.z).to_vec3();
            }
            _ => {}
        }
    }

    /// Creates a new node from a name.
    pub fn new_from_name(name: String) -> Self {
        let nodes = TileFXNode::nodes();
        for n in nodes {
            if n.name() == name {
                return n;
            }
        }
        TileFXNode::new(Saturation)
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
        //let collection = self.collection();

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let _xx = (i % width) as f32;
                    let _yy = (i / width) as f32;

                    let color = Vec4::zero();

                    // match &self.role {
                    //     Brightness => {}
                    //     _ => {}
                    // }

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });

        self.preview = buffer;
    }
}
