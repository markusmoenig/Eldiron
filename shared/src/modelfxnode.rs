use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
use crate::modelfxterminal::ModelFXTerminalRole::*;
use noiselib::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXNodeRole {
    Geometry,
    Material,
    Noise,
}

use ModelFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXNode {
    // Geometry
    Floor(TheCollection),
    WallHorizontal(TheCollection),
    WallVertical(TheCollection),
    // Material
    Subdivide(TheCollection),
    Bricks(TheCollection),
    Material(TheCollection),
    // Noise
    Noise3D(TheCollection),
}

impl ModelFXNode {
    pub fn new_node(name: &str, collection: Option<TheCollection>) -> Option<Self> {
        let mut coll = TheCollection::named(name.into());
        match name {
            "Floor" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Height", TheValue::FloatRange(0.01, 0.0..=1.0));
                }
                Some(Self::Floor(coll))
            }
            "Wall Horizontal" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Position", TheValue::FloatRange(0.5, 0.0..=1.0));
                    coll.set("Offset", TheValue::FloatRange(0.5, 0.0..=1.0));
                    coll.set("Width", TheValue::FloatRange(1.0, 0.0..=1.0));
                    coll.set("Height", TheValue::FloatRange(1.0, 0.0..=1.0));
                    coll.set("Depth", TheValue::FloatRange(0.2, 0.0..=1.0));
                }
                Some(Self::WallHorizontal(coll))
            }
            "Wall Vertical" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Position", TheValue::FloatRange(0.5, 0.0..=1.0));
                    coll.set("Offset", TheValue::FloatRange(0.5, 0.0..=1.0));
                    coll.set("Width", TheValue::FloatRange(1.0, 0.0..=1.0));
                    coll.set("Height", TheValue::FloatRange(1.0, 0.0..=1.0));
                    coll.set("Depth", TheValue::FloatRange(0.2, 0.0..=1.0));
                    coll.set(
                        "Pattern",
                        TheValue::TextList(
                            0,
                            vec![str!("Tile"), str!("Color"), str!("Brick"), str!("Stripes")],
                        ),
                    );
                }
                Some(Self::WallVertical(coll))
            }
            "Subdivide" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set(
                        "Mode",
                        TheValue::TextList(0, vec![str!("Vertical"), str!("Horizontal")]),
                    );
                    coll.set("Offset", TheValue::FloatRange(0.5, 0.0..=1.0));
                }
                Some(Self::Subdivide(coll))
            }
            "Bricks" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Ratio", TheValue::FloatRange(2.0, 1.0..=10.0));
                    coll.set("Rounding", TheValue::FloatRange(0.0, 0.0..=0.5));
                    coll.set("Bevel", TheValue::FloatRange(0.0, 0.0..=0.5));
                    coll.set("Gap", TheValue::FloatRange(0.1, 0.0..=0.5));
                    coll.set("Cell", TheValue::FloatRange(6.0, 0.0..=15.0));
                    coll.set(
                        "Mode",
                        TheValue::TextList(0, vec![str!("Bricks"), str!("Tiles")]),
                    );
                }
                Some(Self::Bricks(coll))
            }
            "Material" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Color", TheValue::PaletteIndex(0));
                }
                Some(Self::Material(coll))
            }
            "Noise3D" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set(
                        "Type",
                        TheValue::TextList(
                            0,
                            vec![str!("Perlin"), str!("Musgrave"), str!("Simplex")],
                        ),
                    );
                    coll.set("Seed", TheValue::IntRange(0, 0..=100));
                    coll.set("UV Scale", TheValue::FloatRange(1.0, 0.0..=20.0));
                    coll.set("Out Scale", TheValue::FloatRange(1.0, 0.0..=1.0));
                    coll.set("Octaves", TheValue::IntRange(1, 1..=4));
                    coll.set(
                        "Fractal",
                        TheValue::TextList(0, vec![str!("Add"), str!("Add Abs"), str!("Mul")]),
                    );
                }
                Some(Self::Noise3D(coll))
            }
            _ => None,
        }
    }

    /// List of input terminals.
    pub fn input_terminals(&self) -> Vec<ModelFXTerminal> {
        match self {
            Self::Material(_) | Self::Bricks(_) | Self::Subdivide(_) => {
                vec![
                    ModelFXTerminal::new(UV, 4),
                    ModelFXTerminal::new(ModelFXTerminalRole::Noise, 6),
                ]
            }
            Self::WallHorizontal(_) | Self::WallVertical(_) | Self::Floor(_) => {
                vec![ModelFXTerminal::new(ModelFXTerminalRole::Noise, 6)]
            }
            _ => {
                vec![]
            }
        }
    }

    /// List of output terminals.
    pub fn output_terminals(&self) -> Vec<ModelFXTerminal> {
        match self {
            Self::WallHorizontal(_) | Self::WallVertical(_) | Self::Floor(_) => {
                vec![
                    ModelFXTerminal::new(Face, 0),
                    ModelFXTerminal::new(Face, 1),
                    ModelFXTerminal::new(Face, 2),
                ]
            }
            Self::Subdivide(_) => {
                vec![ModelFXTerminal::new(Face, 0), ModelFXTerminal::new(Face, 1)]
            }
            Self::Bricks(_) => {
                vec![ModelFXTerminal::new(Face, 0), ModelFXTerminal::new(Face, 5)]
            }
            Self::Noise3D(_) => {
                vec![ModelFXTerminal::new(ModelFXTerminalRole::Noise, 6)]
            }
            _ => {
                vec![]
            }
        }
    }

    /// Return the color and terminal index for the given hit position.
    pub fn color_index_for_hit(&self, hit: &mut Hit) -> (u8, u8) {
        match self {
            Self::WallHorizontal(_) | Self::WallVertical(_) | Self::Floor(_) => {
                let nx = hit.normal.x.abs();
                let ny = hit.normal.y.abs();
                let nz = hit.normal.z.abs();

                if nx > ny && nx > nz {
                    hit.face = HitFace::XFace;
                    hit.uv = hit.hit_point.zy();
                    (0, 0)
                } else if ny > nx && ny > nz {
                    hit.face = HitFace::YFace;
                    hit.uv = hit.hit_point.xz();
                    (1, 1)
                } else {
                    hit.face = HitFace::ZFace;
                    hit.uv = hit.hit_point.xy();
                    (2, 2)
                }
            }
            Self::Subdivide(coll) => {
                let uv = hit.uv / 3.0;
                subdivide(coll, uv, hit)
            }
            Self::Bricks(coll) => {
                let uv = hit.uv / 3.0;
                bricks(coll, uv, hit)
            }
            _ => (0, 0),
        }
    }

    /// Handle the material node for the given terminal.
    pub fn material(
        &self,
        _in_terminal: &u8,
        hit: &mut Hit,
        palette: &ThePalette,
        noise: f32,
    ) -> Option<u8> {
        match self {
            Self::Material(collection) => {
                if let Some(TheValue::PaletteIndex(index)) = collection.get("Color") {
                    if let Some(color) = &palette.colors[*index as usize] {
                        hit.color.x = color.r + noise;
                        hit.color.y = color.g + noise;
                        hit.color.z = color.b + noise;
                        hit.color.w = 1.0;
                    }
                }
                None
            }
            Self::Subdivide(collection) => {
                let (_, terminal) = subdivide(collection, hit.uv, hit);
                Some(terminal)
            }
            Self::Bricks(collection) => {
                let (_, terminal) = bricks(collection, hit.uv, hit);
                Some(terminal)
            }
            _ => None,
        }
    }

    /// Noise.
    pub fn noise(&self, hit: &Hit) -> f32 {
        match self {
            Self::Noise3D(collection) => {
                let seed = collection.get_i32_default("Seed", 0) as u32;
                let noise_type = collection.get_i32_default("Type", 0);
                let scale = collection.get_f32_default("UV Scale", 1.0);
                let out_scale = collection.get_f32_default("Out Scale", 1.0);
                let octaves = collection.get_i32_default("Octaves", 0);
                let mut value = 0.0;
                let mut rng = UniformRandomGen::new(seed);

                if noise_type == 0 {
                    if octaves == 1 {
                        value = perlin_noise_3d(
                            &mut rng,
                            hit.hit_point.x * scale,
                            hit.hit_point.y * scale,
                            hit.hit_point.z * scale,
                            seed,
                        );
                    } else {
                        let fractal = collection.get_i32_default("Fractal", 0);

                        if fractal == 0 {
                            value = fractal_noise_add_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                perlin_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                seed,
                            );
                        } else if fractal == 1 {
                            value = fractal_noise_add_abs_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                perlin_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                seed,
                            );
                        } else {
                            value = fractal_noise_mul_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                perlin_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                1.5,
                                seed,
                            );
                        }
                    }
                } else if noise_type == 1 {
                    if octaves == 1 {
                        value = musgrave_noise_3d(
                            &mut rng,
                            hit.hit_point.x * scale,
                            hit.hit_point.y * scale,
                            hit.hit_point.z * scale,
                            seed,
                        );
                    } else {
                        let fractal = collection.get_i32_default("Fractal", 0);

                        if fractal == 0 {
                            value = fractal_noise_add_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                musgrave_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                seed,
                            );
                        } else if fractal == 1 {
                            value = fractal_noise_add_abs_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                musgrave_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                seed,
                            );
                        } else {
                            value = fractal_noise_mul_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                musgrave_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                1.5,
                                seed,
                            );
                        }
                    }
                }
                if noise_type == 2 {
                    if octaves == 1 {
                        value = simplex_noise_3d(
                            &mut rng,
                            hit.hit_point.x * scale,
                            hit.hit_point.y * scale,
                            hit.hit_point.z * scale,
                            seed,
                        );
                    } else {
                        let fractal = collection.get_i32_default("Fractal", 0);

                        if fractal == 0 {
                            value = fractal_noise_add_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                simplex_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                seed,
                            );
                        } else if fractal == 1 {
                            value = fractal_noise_add_abs_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                simplex_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                seed,
                            );
                        } else {
                            value = fractal_noise_mul_3d(
                                &mut rng,
                                hit.hit_point.x * scale,
                                hit.hit_point.y * scale,
                                hit.hit_point.z * scale,
                                simplex_noise_3d,
                                octaves,
                                0.5,
                                2.0,
                                1.5,
                                seed,
                            );
                        }
                    }
                }

                value * out_scale
            }
            _ => 0.0,
        }
    }

    /// Get a reference to the collection.
    pub fn collection(&self) -> &TheCollection {
        match self {
            Self::Floor(collection) => collection,
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
            Self::Subdivide(collection) => collection,
            Self::Bricks(collection) => collection,
            Self::Material(collection) => collection,
            Self::Noise3D(collection) => collection,
        }
    }

    /// Get a mutable reference to the collection.
    pub fn collection_mut(&mut self) -> &mut TheCollection {
        match self {
            Self::Floor(collection) => collection,
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
            Self::Subdivide(collection) => collection,
            Self::Bricks(collection) => collection,
            Self::Material(collection) => collection,
            Self::Noise3D(collection) => collection,
        }
    }

    /// Returns the distance to p.
    pub fn distance(&self, p: Vec3f, noise: f32) -> f32 {
        match self {
            Self::Floor(collection) => {
                let height = collection.get_f32_default("Height", 0.01);
                p.y - height - noise
            }
            Self::WallHorizontal(collection) => {
                let position = collection.get_f32_default("Position", 0.5);
                let offset = collection.get_f32_default("Offset", 0.5);
                let width = collection.get_f32_default("Width", 1.0);
                let height = collection.get_f32_default("Height", 1.0);
                let depth = collection.get_f32_default("Depth", 0.2);
                let mut min = position - depth / 2.0;
                let mut max = position + depth / 2.0;
                if min < 0.0 {
                    let adjustment = 0.0 - min;
                    min += adjustment;
                    max += adjustment;
                }
                if max > 1.0 {
                    let adjustment = max - 1.0;
                    min -= adjustment;
                    max -= adjustment;
                }
                let mut min_o = offset - width / 2.0;
                let mut max_o = offset + width / 2.0;
                if min_o < 0.0 {
                    let adjustment = 0.0 - min_o;
                    min_o += adjustment;
                    max_o += adjustment;
                }
                if max_o > 1.0 {
                    let adjustment = max_o - 1.0;
                    min_o -= adjustment;
                    max_o -= adjustment;
                }
                sd_box(
                    p - vec3f((min_o + max_o) / 2.0, height / 2.0, (min + max) / 2.0),
                    vec3f((max_o - min_o) / 2.0, height / 2.0, (max - min) / 2.0),
                ) - noise
            }
            Self::WallVertical(collection) => {
                let position = collection.get_f32_default("Position", 0.5);
                let offset = collection.get_f32_default("Offset", 0.5);
                let width = collection.get_f32_default("Width", 1.0);
                let height = collection.get_f32_default("Height", 1.0);
                let depth = collection.get_f32_default("Depth", 0.2);
                let mut min = position - depth / 2.0;
                let mut max = position + depth / 2.0;
                if min < 0.0 {
                    let adjustment = 0.0 - min;
                    min += adjustment;
                    max += adjustment;
                }
                if max > 1.0 {
                    let adjustment = max - 1.0;
                    min -= adjustment;
                    max -= adjustment;
                }
                let mut min_o = offset - width / 2.0;
                let mut max_o = offset + width / 2.0;
                if min_o < 0.0 {
                    let adjustment = 0.0 - min_o;
                    min_o += adjustment;
                    max_o += adjustment;
                }
                if max_o > 1.0 {
                    let adjustment = max_o - 1.0;
                    min_o -= adjustment;
                    max_o -= adjustment;
                }
                sd_box(
                    p - vec3f((min + max) / 2.0, height / 2.0, (min_o + max_o) / 2.0),
                    vec3f((max - min) / 2.0, height / 2.0, (max_o - min_o) / 2.0),
                ) - noise
            }
            _ => 0.0,
        }
    }

    /// Role
    pub fn role(&self) -> ModelFXNodeRole {
        match self {
            Self::Floor(_) => Geometry,
            Self::WallHorizontal(_) => Geometry,
            Self::WallVertical(_) => Geometry,
            Self::Subdivide(_) => Material,
            Self::Bricks(_) => Material,
            Self::Material(_) => Material,
            Self::Noise3D(_) => ModelFXNodeRole::Noise,
        }
    }

    /// Name
    pub fn name(&self) -> String {
        match self {
            Self::Floor(_) => str!("Floor"),
            Self::WallHorizontal(_) => str!("Wall Horizontal"),
            Self::WallVertical(_) => str!("Wall Vertical"),
            Self::Subdivide(_) => str!("Bricks"),
            Self::Bricks(_) => str!("Bricks"),
            Self::Material(_) => str!("Material"),
            Self::Noise3D(_) => str!("Noise"),
        }
    }
}
