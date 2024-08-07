//use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RegionFXNodeRole {
    TiltedIsoCamera,
    TopDownIsoCamera,
    Renderer,
}

use RegionFXNodeRole::*;

use crate::Ray;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionFXNode {
    pub id: Uuid,
    pub role: RegionFXNodeRole,
    pub timeline: TheTimeline,

    pub position: Vec2i,

    pub supports_preview: bool,
    pub preview_is_open: bool,

    pub preview: TheRGBABuffer,
}

impl RegionFXNode {
    pub fn new(role: RegionFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Props"));
        let supports_preview = false;
        let preview_is_open = false;

        match role {
            TiltedIsoCamera => {
                coll.set("Height", TheValue::FloatRange(4.0, 1.0..=10.0));
                coll.set(
                    "Alignment",
                    TheValue::TextList(0, vec![str!("Right"), str!("Left")]),
                );
            }
            TopDownIsoCamera => {
                coll.set("Height", TheValue::FloatRange(4.0, 1.0..=10.0));
                coll.set("X Offset", TheValue::FloatRange(-1.0, -5.0..=5.0));
                coll.set("Z Offset", TheValue::FloatRange(1.0, -5.0..=5.0));
            }
            Renderer => {}
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
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            TiltedIsoCamera => str!("Tilted Iso Camera"),
            TopDownIsoCamera => str!("Top Down Iso Camera"),
            Renderer => str!("Renderer"),
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new(RegionFXNodeRole::TiltedIsoCamera),
            Self::new(RegionFXNodeRole::TopDownIsoCamera),
            Self::new(RegionFXNodeRole::Renderer),
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

        match self.role {
            RegionFXNodeRole::TiltedIsoCamera => {
                params.push(coll.get_f32_default("Height", 0.0));
                params.push(coll.get_i32_default("Alignment", 0) as f32);
            }
            RegionFXNodeRole::TopDownIsoCamera => {
                params.push(coll.get_f32_default("Height", 0.0));
                params.push(coll.get_f32_default("X Offset", -1.0));
                params.push(coll.get_f32_default("Z Offset", 1.0));
            }
            RegionFXNodeRole::Renderer => {} //_ => {}
        }

        params
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            TiltedIsoCamera | TopDownIsoCamera => {
                vec![]
            }
            Renderer => {
                vec![TheNodeTerminal {
                    name: str!("cam"),
                    role: str!("Camera"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            TiltedIsoCamera | TopDownIsoCamera => {
                vec![TheNodeTerminal {
                    name: str!("cam"),
                    role: str!("Camera"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            Renderer => {
                vec![]
            } //_ => vec![],
        }
    }

    /// Create a cameray ray
    pub fn create_ray(
        &self,
        uv: Vec2f,
        position: Vec3f,
        size: Vec2f,
        tiles: Vec2f,
        offset: Vec2f,
        params: &[f32],
    ) -> Ray {
        match self.role {
            TiltedIsoCamera => {
                let height = params[0];
                let alignment = params[1] as i32;

                let mut ro = vec3f(position.x + 0.5, 0.0, position.z + 0.5);
                let rd = ro;
                ro.y = params[0];
                ro.z += 1.0;

                let pixel_size = Vec2f::new(1.0 / size.x, 1.0 / size.y);

                let cam_origin = ro;
                let cam_look_at = rd;

                let half_width = tiles.x;
                let half_height = tiles.y;

                let up_vector = Vec3f::new(0.0, 1.0, 0.0);

                let w = normalize(cam_origin - cam_look_at);
                let u = cross(up_vector, w);
                let v = cross(w, u);

                let horizontal = u * half_width * height;
                let vertical = v * half_height * height;

                let mut out_origin = cam_origin;
                out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
                out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);
                out_origin.y = cam_origin.y;

                Ray::new(
                    out_origin,
                    normalize(vec3f(
                        if alignment == 0 { -0.35 } else { 0.35 },
                        -1.0,
                        -0.35,
                    )),
                )
            }
            TopDownIsoCamera => {
                let height = params[0];

                let mut ro = vec3f(position.x + 0.5, 0.0, position.z + 0.5);
                let rd = ro;

                ro.y = height;
                ro.x += params[1];
                ro.z += params[2];

                let scale_factor = height / 1.5;

                let pixel_size = Vec2f::new(1.0 / size.x, 1.0 / size.y);

                let cam_origin = ro;
                let cam_look_at = rd;

                let half_width = tiles.x;
                let half_height = tiles.y;

                let up_vector = Vec3f::new(0.0, 1.0, 0.0);

                let w = normalize(cam_origin - cam_look_at);
                let u = cross(up_vector, w);
                let v = cross(w, u);

                let horizontal = u * half_width * scale_factor;
                let vertical = v * half_height * scale_factor;

                let mut out_origin = cam_origin;
                out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
                out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);

                Ray::new(out_origin, normalize(-w))
            }
            _ => Ray::new(Vec3f::zero(), Vec3f::zero()),
        }
    }

    /// Creates a new node from a name.
    pub fn new_from_name(name: String) -> Self {
        let nodes = RegionFXNode::nodes();
        for n in nodes {
            if n.name() == name {
                return n;
            }
        }
        RegionFXNode::new(Renderer)
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
}
