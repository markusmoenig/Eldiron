use crate::prelude::*;
use rayon::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum MaterialFXNodeRole {
    Geometry,
    MaterialMixer,
    Material,
    Noise2D,
    Noise3D,
    Brick,
}

use MaterialFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct MaterialFXNode {
    pub id: Uuid,
    pub role: MaterialFXNodeRole,
    pub timeline: TheTimeline,

    pub position: Vec2i,

    pub supports_preview: bool,
    pub preview_is_open: bool,

    pub preview: TheRGBABuffer,

    pub resolve_branches: bool,
}

impl MaterialFXNode {
    pub fn new(role: MaterialFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Props"));
        let mut supports_preview = false;
        let mut preview_is_open = false;
        let mut resolve_branches = false;

        match role {
            Geometry => {
                supports_preview = true;
                preview_is_open = true;
            }
            MaterialMixer => {
                resolve_branches = true;
            }
            Material => {
                coll.set("Color", TheValue::PaletteIndex(0));
                coll.set("Roughness", TheValue::FloatRange(0.5, 0.0..=1.0));
                coll.set("Metallic", TheValue::FloatRange(0.0, 0.0..=1.0));
            }
            Noise2D | Noise3D => {
                // coll.set(
                //     "Type",
                //     TheValue::TextList(0, vec![str!("Value"), str!("Musgrave"), str!("Simplex")]),
                // );
                coll.set("UV Scale", TheValue::FloatRange(1.0, 0.0..=6.0));
                coll.set("Out Scale", TheValue::FloatRange(1.0, 0.0..=1.0));
                coll.set("Disp Scale", TheValue::FloatRange(0.1, 0.0..=1.0));
                coll.set("Octaves", TheValue::IntRange(5, 0..=5));
                supports_preview = true;
                preview_is_open = true;
            }
            Brick => {
                coll.set("Ratio", TheValue::FloatRange(2.0, 1.0..=10.0));
                coll.set("Rounding", TheValue::FloatRange(0.0, 0.0..=0.5));
                //coll.set("Bevel", TheValue::FloatRange(0.0, 0.0..=0.5));
                coll.set("Gap", TheValue::FloatRange(0.1, 0.0..=0.5));
                coll.set("Cell", TheValue::FloatRange(6.0, 0.0..=15.0));
                coll.set(
                    "Mode",
                    TheValue::TextList(0, vec![str!("Bricks"), str!("Tiles")]),
                );
                coll.set("Displace", TheValue::FloatRange(0.0, 0.0..=1.0));
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
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            Geometry => str!("Geometry"),
            MaterialMixer => str!("Material Mixer"),
            Material => str!("Material"),
            Noise2D => str!("Noise2D"),
            Noise3D => str!("Noise3D"),
            Brick => str!("Bricks"),
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new(MaterialFXNodeRole::Geometry),
            Self::new(MaterialFXNodeRole::MaterialMixer),
            Self::new(MaterialFXNodeRole::Material),
            Self::new(MaterialFXNodeRole::Noise2D),
            Self::new(MaterialFXNodeRole::Noise3D),
            Self::new(MaterialFXNodeRole::Brick),
        ]
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Brick => {
                vec![
                    TheNodeTerminal {
                        name: str!("in"),
                        role: str!("Input"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("displace"),
                        role: str!("Displace"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            MaterialMixer | Material | Noise3D | Noise2D => {
                vec![TheNodeTerminal {
                    name: str!("in"),
                    role: str!("Input"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            _ => vec![],
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                vec![
                    TheNodeTerminal {
                        name: str!("out"),
                        role: str!("Out"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("displace"),
                        role: str!("Displac"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            MaterialMixer => {
                vec![
                    TheNodeTerminal {
                        name: str!("mat1"),
                        role: str!("Material1"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("mat2"),
                        role: str!("Material2"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            Brick => {
                vec![
                    TheNodeTerminal {
                        name: str!("brick"),
                        role: str!("Brick"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("mortar"),
                        role: str!("Mortar"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            Noise3D | Noise2D => {
                vec![TheNodeTerminal {
                    name: str!("out"),
                    role: str!("Output"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            _ => vec![],
        }
    }

    /// Computes the node.
    pub fn compute(&self, hit: &mut Hit, palette: &ThePalette, resolved: Vec<Hit>) -> Option<u8> {
        match self.role {
            Material => {
                let collection = self.collection();

                if let Some(TheValue::PaletteIndex(index)) = collection.get("Color") {
                    if let Some(color) = &palette.colors[*index as usize] {
                        hit.albedo.x = color.r * hit.value;
                        hit.albedo.y = color.g * hit.value;
                        hit.albedo.z = color.b * hit.value;
                        hit.roughness = collection.get_f32_default("Roughness", 0.5) * hit.value;
                        hit.metallic = collection.get_f32_default("Metallic", 0.0) * hit.value;
                    }
                }

                None
            }
            MaterialMixer => {
                if resolved.len() == 1 {
                    *hit = resolved[0].clone();
                } else if resolved.len() >= 2 {
                    hit.albedo = lerp(resolved[0].albedo, resolved[1].albedo, hit.value);
                    hit.roughness = lerp(resolved[0].roughness, resolved[1].roughness, hit.value);
                    hit.metallic = lerp(resolved[0].metallic, resolved[1].metallic, hit.value);
                }
                None
            }
            Noise2D => {
                let collection = self.collection();
                hit.value = noise2d(&collection, &hit.uv);
                hit.albedo = vec3f(hit.value, hit.value, hit.value);
                Some(0)
            }
            Noise3D => {
                let collection = self.collection();
                hit.value = noise3d(&collection, &hit.hit_point);
                hit.albedo = vec3f(hit.value, hit.value, hit.value);
                Some(0)
            }
            Brick => {
                let collection = self.collection();
                let (_, terminal) = bricks(&collection, hit.uv, hit);
                Some(terminal)
            }
            _ => None,
        }
    }

    /// Computes the displacement of the node.
    pub fn displacement(&self, hit: &mut Hit) {
        match self.role {
            Brick => {
                let collection = self.collection();
                let (_, terminal) = bricks(&collection, hit.uv, hit);
                if terminal == 1 {
                    hit.displacement = collection.get_f32_default("Displace", 0.0);
                } else {
                    hit.displacement = 0.0;
                }
            }
            Noise2D => {
                let collection = self.collection();
                let value = noise2d(&collection, &hit.uv);
                let disp_scale = collection.get_f32_default("Disp Scale", 0.1);
                hit.displacement = value * disp_scale;
            }
            Noise3D => {
                let collection = self.collection();
                let value = noise3d(&collection, &hit.hit_point);
                let disp_scale = collection.get_f32_default("Disp Scale", 0.1);
                hit.displacement = value * disp_scale;
            }
            _ => {}
        }
    }

    /// Creates a new node from a name.
    pub fn new_from_name(name: String) -> Self {
        let nodes = MaterialFXNode::nodes();
        for n in nodes {
            if n.name() == name {
                return n;
            }
        }
        MaterialFXNode::new(Geometry)
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
                        Noise2D => {
                            let uv = Vec2f::new(xx / width as f32, yy / height as f32);

                            let value = noise2d(&collection, &uv);
                            color = Vec4f::new(value, value, value, 1.0);
                        }
                        Noise3D => {
                            let hit_point = Vec3f::new(xx / width as f32, 0.0, yy / height as f32);

                            let value = noise3d(&collection, &hit_point);
                            color = Vec4f::new(value, value, value, 1.0);
                        }
                        _ => {}
                    }

                    pixel.copy_from_slice(&TheColor::from_vec4f(color).to_u8_array());
                }
            });

        self.preview = buffer;
    }
}

/*#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNode {
    Disc(Uuid, TheTimeline),
}

impl GeoFXNode {
    pub fn new_disc() -> Self {
        let mut coll = TheCollection::named(str!("Geo"));
        coll.set("Radius", TheValue::FloatRange(0.4, 0.001..=5.0));
        Self::Disc(Uuid::new_v4(), TheTimeline::collection(coll))
    }

    pub fn nodes() -> Vec<Self> {
        vec![Self::new_disc()]
    }

    pub fn distance(&self, time: &TheTime, p: Vec2f, scale: f32) -> f32 {
        match self {
            Self::Disc(_, timeline) => {
                if let Some(value) =
                    timeline.get(str!("Geo"), str!("Radius"), time, TheInterpolation::Linear)
                {
                    if let Some(radius) = value.to_f32() {
                        return length(p) - radius * scale;
                    }
                }
            }
        }

        f32::INFINITY
    }

    pub fn collection(&self) -> TheCollection {
        match self {
            Self::Disc(_, timeline) => {
                if let Some(coll) = timeline.get_collection_at(&TheTime::default(), str!("Geo")) {
                    return coll.clone();
                }
            }
        }

        TheCollection::default()
    }

    pub fn set_id(&mut self, id: Uuid) {
        match self {
            Self::Disc(ref mut node_id, _) => {
                *node_id = id;
            }
        }
    }

    pub fn set(&mut self, key: &str, value: TheValue) {
        match self {
            Self::Disc(_, timeline) => {
                timeline.set(&TheTime::default(), key, "Geo", value);
            }
        }
    }

    pub fn preview(&self, buffer: &mut TheRGBABuffer) {
        fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
            [
                (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
            ]
        }

        let width = buffer.dim().width;
        let height = buffer.dim().height;

        for y in 0..height {
            for x in 0..width {
                let p = vec2f(
                    x as f32 / width as f32 - 0.5,
                    y as f32 / height as f32 - 0.5,
                );
                let d = self.distance(&TheTime::default(), p, 1.0);
                let t = smoothstep(-0.04, 0.0, d);
                buffer.set_pixel(x, y, &mix_color(&WHITE, &BLACK, t));
            }
        }
    }
} */
