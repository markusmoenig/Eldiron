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
    UVSplitter,
    Subdivide,
    BoxSubdivision,
    Tiles,
    Distance,
    Bump,
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
    pub texture_id: Option<Uuid>,
}

impl MaterialFXNode {
    pub fn new(role: MaterialFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Props"));
        let mut supports_preview = false;
        let mut preview_is_open = false;

        match role {
            Geometry => {
                coll.set("Name", TheValue::Text(str!("")));
                coll.set("Tags", TheValue::Text(str!("")));
                supports_preview = true;
                preview_is_open = true;
            }
            MaterialMixer => {}
            Material => {
                coll.set("Color", TheValue::PaletteIndex(0));
                coll.set("Roughness", TheValue::FloatRange(0.5, 0.0..=1.0));
                coll.set("Metallic", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Anisotropic", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Subsurface", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Specular Tint", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Sheen", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Sheen Tint", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Clearcoat", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Clearcoat Gloss", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("Transmission", TheValue::FloatRange(0.0, 0.0..=1.0));
                //coll.set("Emission", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("IOR", TheValue::FloatRange(1.5, 0.0..=2.0));
                coll.set("Texture", TheValue::Text(str!("")));
            }
            UVSplitter => {
                coll.set("Map", TheValue::TextList(0, vec![str!("Cylinder")]));
            }
            Noise2D => {
                coll.set("UV Scale X", TheValue::FloatRange(1.0, 0.0..=10.0));
                coll.set("UV Scale Y", TheValue::FloatRange(1.0, 0.0..=10.0));
                coll.set("Out Scale", TheValue::FloatRange(1.0, 0.0..=1.0));
                coll.set("Octaves", TheValue::IntRange(5, 0..=5));
                supports_preview = true;
                preview_is_open = true;
            }
            Noise3D => {
                coll.set("UV Scale X", TheValue::FloatRange(1.0, 0.0..=10.0));
                coll.set("UV Scale Y", TheValue::FloatRange(1.0, 0.0..=10.0));
                coll.set("UV Scale Z", TheValue::FloatRange(1.0, 0.0..=10.0));
                coll.set("Out Scale", TheValue::FloatRange(1.0, 0.0..=1.0));
                coll.set("Octaves", TheValue::IntRange(5, 0..=5));
                supports_preview = true;
                preview_is_open = true;
            }
            Brick => {
                coll.set("Ratio", TheValue::FloatRange(2.0, 1.0..=10.0));
                coll.set("Rounding", TheValue::FloatRange(0.0, 0.0..=0.5));
                coll.set("Rotation", TheValue::FloatRange(0.15, 0.0..=2.0));
                coll.set("Gap", TheValue::FloatRange(0.1, 0.0..=0.5));
                coll.set("Cell", TheValue::FloatRange(6.0, 0.0..=15.0));
                coll.set(
                    "Mode",
                    TheValue::TextList(0, vec![str!("Bricks"), str!("Tiles")]),
                );
            }
            Subdivide => {
                coll.set(
                    "Mode",
                    TheValue::TextList(0, vec![str!("Horizontal"), str!("Vertical")]),
                );
                coll.set("Offset", TheValue::FloatRange(0.5, 0.0..=1.0));
            }
            Distance => {
                coll.set("From", TheValue::FloatRange(0.0, 0.0..=1.0));
                coll.set("To", TheValue::FloatRange(0.2, 0.0..=1.0));
            }
            BoxSubdivision => {
                coll.set("Scale", TheValue::FloatRange(1.0, 0.0..=2.0));
                coll.set("Gap", TheValue::FloatRange(0.8, 0.0..=2.0));
                coll.set("Rotation", TheValue::FloatRange(0.15, 0.0..=2.0));
                coll.set("Rounding", TheValue::FloatRange(0.15, 0.0..=1.0));
            }
            Tiles => {
                coll.set("Subdivisions", TheValue::IntRange(2, 1..=8));
                coll.set("Size", TheValue::FloatRange(0.8, 0.0..=1.0));
                coll.set("Rotation", TheValue::FloatRange(0.15, 0.0..=2.0));
                coll.set("Rounding", TheValue::FloatRange(0.15, 0.0..=1.0));
            }
            Bump => {}
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
            texture_id: None,
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            Geometry => str!("Geometry"),
            MaterialMixer => str!("Material Mixer"),
            Material => str!("Material"),
            Noise2D => str!("Noise2D"),
            Noise3D => str!("Noise3D"),
            Brick => str!("Bricks & Tiles"),
            UVSplitter => str!("UV Splitter"),
            Subdivide => str!("Subdivide"),
            Distance => str!("Distance"),
            BoxSubdivision => str!("Box Subdivision"),
            Tiles => str!("Tiles"),
            Bump => str!("Bump"),
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
            Self::new(MaterialFXNodeRole::UVSplitter),
            Self::new(MaterialFXNodeRole::Subdivide),
            Self::new(MaterialFXNodeRole::Distance),
            Self::new(MaterialFXNodeRole::BoxSubdivision),
            Self::new(MaterialFXNodeRole::Tiles),
            Self::new(MaterialFXNodeRole::Bump),
        ]
    }

    /// Gives the node a chance to update its parameters in case things changed.
    pub fn update_parameters(&mut self) {
        // match self.role {
        //     Geometry => {}
        //     _ => {}
        // }
    }

    /// Loads the parameters of the nodes into memory for faster access.
    pub fn load_parameters(&self, _time: &TheTime) -> Vec<f32> {
        let mut params = vec![];

        let coll = self.collection();

        match self.role {
            MaterialFXNodeRole::Geometry => {}
            MaterialFXNodeRole::Noise2D => {
                params.push(coll.get_f32_default("UV Scale X", 1.0));
                params.push(coll.get_f32_default("UV Scale Y", 1.0));
                params.push(coll.get_f32_default("Out Scale", 1.0));
                params.push(coll.get_i32_default("Octaves", 5) as f32);
            }
            MaterialFXNodeRole::Material => {
                params.push(coll.get_i32_default("Color", 0) as f32);
                params.push(coll.get_f32_default("Roughness", 0.5));
                params.push(coll.get_f32_default("Metallic", 0.0));
                params.push(coll.get_f32_default("Anisotropic", 0.0));
                params.push(coll.get_f32_default("Subsurface", 0.0));
                params.push(coll.get_f32_default("Specular Tint", 0.0));
                params.push(coll.get_f32_default("Sheen", 0.0));
                params.push(coll.get_f32_default("Sheen Tint", 0.0));
                params.push(coll.get_f32_default("Clearcoat", 0.0));
                params.push(coll.get_f32_default("Clearcoat Gloss", 0.0));
                params.push(coll.get_f32_default("Transmission", 0.0));
                params.push(coll.get_f32_default("IOR", 1.5));
            }
            MaterialFXNodeRole::BoxSubdivision => {
                params.push(coll.get_f32_default("Scale", 1.0));
                params.push(coll.get_f32_default("Gap", 0.8));
                params.push(coll.get_f32_default("Rotation", 0.15));
                params.push(coll.get_f32_default("Rounding", 0.15));
            }
            MaterialFXNodeRole::Brick => {
                params.push(coll.get_f32_default("Ratio", 2.0));
                params.push(coll.get_f32_default("Rounding", 0.0));
                params.push(coll.get_f32_default("Rotation", 0.15));
                params.push(coll.get_f32_default("Gap", 0.1));
                params.push(coll.get_f32_default("Cell", 6.0));
                params.push(coll.get_i32_default("Mode", 0) as f32);
            }
            MaterialFXNodeRole::Tiles => {
                params.push(coll.get_f32_default("Size", 0.8));
                params.push(coll.get_i32_default("Subdivisions", 2) as f32);
                params.push(coll.get_f32_default("Rotation", 0.15));
                params.push(coll.get_f32_default("Rounding", 0.15));
            }
            _ => {}
        }

        params
    }

    /// Returns the outgoing trails which this node needs to have resolved before calling compute.
    /// Mostly used for mixing materials.
    pub fn trails_to_resolve(&self) -> Vec<u8> {
        match self.role {
            BoxSubdivision | Brick => {
                vec![1, 2]
            }
            _ => {
                vec![]
            }
        }
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                // vec![TheNodeTerminal {
                //     name: str!("noise"),
                //     role: str!("Noise"),
                //     color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                // }]
                vec![]
            }
            Noise3D | Noise2D | UVSplitter | Subdivide | Distance | Brick | BoxSubdivision
            | Bump => {
                vec![TheNodeTerminal {
                    name: str!("in"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            Material | MaterialMixer => {
                vec![
                    TheNodeTerminal {
                        name: str!("in"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("noise"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            Tiles => {
                vec![
                    TheNodeTerminal {
                        name: str!("geo"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("noise"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            Geometry => {
                vec![TheNodeTerminal {
                    name: str!("mat"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            MaterialMixer => {
                vec![
                    TheNodeTerminal {
                        name: str!("mat1"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("mat2"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            Brick | BoxSubdivision => {
                vec![
                    TheNodeTerminal {
                        name: str!("out"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("mat1"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("mat2"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("bump"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            Material | Noise3D | Noise2D | Distance => {
                vec![TheNodeTerminal {
                    name: str!("out"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            UVSplitter => {
                vec![
                    TheNodeTerminal {
                        name: str!("top"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("side"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("front"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("mapped"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            _ => vec![],
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
    ) -> Option<u8> {
        match self.role {
            Material => {
                let mut used_texture = false;

                if let Some(texture_id) = &self.texture_id {
                    if let Some(texture) = textures.get(texture_id) {
                        if let Some(color) = texture.buffer[0].at_f_vec4f(hit.uv) {
                            hit.mat.base_color.x = color.x;
                            hit.mat.base_color.y = color.y;
                            hit.mat.base_color.z = color.z;
                            used_texture = true;
                        }
                    }
                }

                if !used_texture {
                    let index = params[0] as usize;
                    if let Some(color) = &palette.colors[index] {
                        hit.mat.base_color.x = color.r;
                        hit.mat.base_color.y = color.g;
                        hit.mat.base_color.z = color.b;
                        if let Some(noise) = hit.noise {
                            let hash = if hit.hash != 0.0 {
                                hit.hash * 2.0 - 1.0
                            } else {
                                0.0
                            };
                            let noise = (noise * 2.0 - 1.0 + hash) * hit.noise_scale;
                            hit.mat.base_color.x += noise;
                            hit.mat.base_color.y += noise;
                            hit.mat.base_color.z += noise;
                        }
                    }
                }

                hit.mat.roughness = params[1];
                hit.mat.metallic = params[2];
                hit.mat.anisotropic = params[3];
                hit.mat.subsurface = params[4];
                hit.mat.specular_tint = params[5];
                hit.mat.sheen = params[6];
                hit.mat.sheen_tint = params[7];
                hit.mat.clearcoat = params[8];
                hit.mat.clearcoat_roughness = params[9];
                hit.mat.spec_trans = params[10];
                hit.mat.ior = params[11];

                Some(0)
            }
            MaterialMixer => {
                if resolved.len() == 1 {
                    *hit = resolved[0].clone();
                } else if resolved.len() >= 2 {
                    if let Some(noise) = hit.noise {
                        let noise = noise * hit.noise_scale;

                        hit.mat.base_color = lerp(
                            resolved[0].mat.base_color,
                            resolved[1].mat.base_color,
                            noise,
                        );
                        hit.mat.roughness =
                            lerp(resolved[0].mat.roughness, resolved[1].mat.roughness, noise);
                        hit.mat.metallic =
                            lerp(resolved[0].mat.metallic, resolved[1].mat.metallic, noise);
                    } else {
                        hit.mat.base_color = lerp(
                            resolved[0].mat.base_color,
                            resolved[1].mat.base_color,
                            hit.value,
                        );
                        hit.mat.roughness = lerp(
                            resolved[0].mat.roughness,
                            resolved[1].mat.roughness,
                            hit.value,
                        );
                        hit.mat.metallic = lerp(
                            resolved[0].mat.metallic,
                            resolved[1].mat.metallic,
                            hit.value,
                        );
                    }
                }
                None
            }
            Noise2D => {
                hit.noise_scale = params[2];
                let scale = vec2f(params[0], params[1]);
                let octaves = params[3] as i32;
                // let value = if hit.two_d {
                //     noise2d(&hit.global_uv, scale, octaves)
                // } else {
                //     noise2d(&hit.global_uv, scale, octaves)
                // };
                let value = noise2d(&hit.global_uv, scale, octaves);
                hit.noise = Some(value);
                hit.mat.base_color = vec3f(hit.value, hit.value, hit.value);
                Some(0)
            }
            Noise3D => {
                let collection = self.collection();
                hit.noise_scale = collection.get_f32_default("Out Scale", 1.0);
                hit.noise = Some(noise3d(&collection, &hit.hit_point));
                hit.mat.base_color = vec3f(hit.value, hit.value, hit.value);
                Some(0)
            }
            Brick => {
                let dist = bricks(hit.global_uv, hit, params);
                let value = 1.0 - smoothstep(-0.08, 0.0, dist);
                hit.value = value;

                if hit.mode == HitMode::Albedo {
                    if resolved.len() == 1 {
                        hit.mat.clone_from(&resolved[0].mat);
                    } else if resolved.len() == 2 {
                        hit.mat.mix(&resolved[1].mat, &resolved[0].mat, hit.value);
                    }
                    Some(0)
                } else {
                    Some(3)
                }
            }
            BoxSubdivision => {
                let scale = params[0];
                let gap = params[1];
                let rotation = params[2];
                let rounding = params[3];

                let p = hit.pattern_pos / (5.0 * scale);
                let rc = box_divide(p, gap, rotation, rounding);
                hit.hash = rc.1;

                let value = 1.0 - smoothstep(-0.08, 0.0, rc.0);
                hit.value = value;

                if hit.mode == HitMode::Albedo {
                    if resolved.len() == 1 {
                        hit.mat.clone_from(&resolved[0].mat);
                    } else if resolved.len() == 2 {
                        hit.mat.mix(&resolved[1].mat, &resolved[0].mat, value);
                    }
                    Some(0)
                } else {
                    Some(3)
                }
            }
            UVSplitter => {
                if hit.two_d {
                    // In 2D mode, we akways return the top face, UV is already set
                    return Some(2);
                }
                let normal = hit.normal;
                let hp = hit.hit_point;
                // if abs(normal.y) > abs(normal.x) && abs(normal.y) > abs(normal.z) {
                if abs(normal.y) > 0.9 && abs(normal.x) < 0.1 && abs(normal.z) < 0.1 {
                    // Top (or bottom) face
                    hit.uv = Vec2f::new(frac(hp.x), frac(hp.z));
                    Some(0)
                // } else if abs(normal.x) > abs(normal.y) && abs(normal.x) > abs(normal.z) {
                } else if abs(normal.x) > 0.9 && abs(normal.y) < 0.1 && abs(normal.z) < 0.1 {
                    // Side face (left or right)
                    hit.uv = Vec2f::new(frac(hp.z), 1.0 - frac(hp.y));
                    Some(1)
                // } else if abs(normal.z) > abs(normal.y) && abs(normal.z) > abs(normal.x) {
                } else if abs(normal.z) > 0.9 && abs(normal.y) < 0.1 && abs(normal.x) < 0.1 {
                    // Front (or back) face
                    hit.uv = Vec2f::new(frac(hp.x), 1.0 - frac(hp.y));
                    Some(2)
                } else {
                    let collection = self.collection();
                    let map = collection.get_i32_default("Map", 0);

                    if map == 0 {
                        // Cylindrical mapping

                        let u = atan2(hp.z, hp.x) / (2.0 * f32::pi()) + 0.5; // Map the angle to [0, 1]
                        let v = hp.y;

                        hit.uv = Vec2f::new(u, v);
                    }

                    Some(3)
                }
            }
            Subdivide => {
                let collection = self.collection();
                Some(subdivide(&collection, hit.uv, hit))
            }
            Distance => {
                let collection = self.collection();
                let from = collection.get_f32_default("From", 0.0);
                let to = collection.get_f32_default("To", 0.2);

                if hit.interior_distance > PATTERN2D_DISTANCE_BORDER {
                    return None;
                }

                let value = smoothstep(from, to, -hit.interior_distance);

                if resolved.len() == 1 {
                    hit.mat.base_color =
                        lerp(resolved[0].mat.base_color, hit.mat.base_color, value);
                    hit.mat.roughness = lerp(resolved[0].mat.roughness, hit.mat.roughness, value);
                    hit.mat.metallic = lerp(resolved[0].mat.metallic, hit.mat.metallic, value);
                }

                Some(0)
            }
            Bump => {
                hit.bump = hit.value / 50.0;
                None
            }
            _ => None,
        }
    }

    pub fn geometry(&self, hit: &mut Hit, params: &[f32]) -> Option<u8> {
        #[allow(clippy::single_match)]
        match &self.role {
            Brick => {
                if hit.interior_distance < 0.0 || hit.two_d {
                    let p = hit.pattern_pos;
                    let d = bricks(p, hit, params);
                    let bump = 1.0 - smoothstep(-0.08, 0.0, d);

                    hit.value = bump;
                }
            }
            BoxSubdivision => {
                if hit.interior_distance < 0.0 || hit.two_d {
                    let scale = params[0];
                    let gap = params[1];
                    let rotation = params[2];
                    let rounding = params[3];

                    let p = hit.pattern_pos / (5.0 * scale);
                    let rc = box_divide(p, gap, rotation, rounding);
                    hit.hash = rc.1;

                    let bump = 1.0 - smoothstep(-0.08, 0.0, rc.0);
                    hit.value = bump;
                }
            }
            Tiles => {
                let size = params[0];
                let subdivisions = params[1] as i32;
                let _rotation = params[2];
                let rounding = params[3];

                let p = hit.pattern_pos; // / (5.0);

                let x = p.x.floor();
                let y = p.y.floor();

                let mut d = f32::INFINITY;

                let grid_size = subdivisions;
                let box_size = 1.0 / grid_size as f32;
                let half_box_size = box_size * 0.5;

                let rounding = rounding * half_box_size;

                // Check distance to each box in the grid
                for by in 0..grid_size {
                    for bx in 0..grid_size {
                        let center = Vec2f::new(
                            x + (bx as f32 + 0.5) * box_size,
                            y + (by as f32 + 0.5) * box_size,
                        );
                        let dist = sdf_box2d(
                            p,
                            center,
                            half_box_size * size - rounding / 1.0,
                            half_box_size * size - rounding / 1.0,
                        ) - rounding;
                        d = d.min(dist);
                    }
                }

                let bump = 1.0 - smoothstep(-0.08, 0.0, d);
                hit.value = bump;
                //hit.hash = rc.1;
            }
            _ => {}
        }
        None
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
                        Noise2D => {
                            let uv = Vec2f::new(xx / width as f32, yy / height as f32);

                            let scale = vec2f(
                                collection.get_f32_default("UV Scale X", 1.0),
                                collection.get_f32_default("UV Scale Y", 1.0),
                            );
                            let octaves = collection.get_i32_default("Octaves", 5);

                            let value = noise2d(&uv, scale, octaves);
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
