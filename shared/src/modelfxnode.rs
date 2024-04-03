use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
use crate::modelfxterminal::ModelFXTerminalRole::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXNodeRole {
    Geometry,
    Material,
}

use ModelFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXNode {
    // Geometry
    WallHorizontal(TheCollection),
    WallVertical(TheCollection),
    // Material
    Bricks(TheCollection),
    Material(TheCollection),
}

impl ModelFXNode {
    pub fn new_node(name: &str, collection: Option<TheCollection>) -> Option<Self> {
        let mut coll = TheCollection::named(name.into());
        match name {
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
                    coll.set("Randomness", TheValue::FloatRange(0.0, 0.0..=1.0));
                }
                Some(Self::Material(coll))
            }
            _ => None,
        }
    }

    /// List of input terminals.
    pub fn input_terminals(&self) -> Vec<ModelFXTerminal> {
        match self {
            Self::Material(_) | Self::Bricks(_) => {
                vec![ModelFXTerminal::new(UV, 4)]
            }
            _ => {
                vec![]
            }
        }
    }

    /// List of output terminals.
    pub fn output_terminals(&self) -> Vec<ModelFXTerminal> {
        match self {
            Self::WallHorizontal(_) | Self::WallVertical(_) => {
                vec![
                    ModelFXTerminal::new(Face, 0),
                    ModelFXTerminal::new(Face, 1),
                    ModelFXTerminal::new(Face, 2),
                ]
            }
            Self::Bricks(_) => {
                vec![ModelFXTerminal::new(Face, 0), ModelFXTerminal::new(Face, 5)]
            }
            _ => {
                vec![]
            }
        }
    }

    /// Return the color and terminal index for the given hit position.
    pub fn color_index_for_hit(&self, hit: &mut Hit) -> (u8, u8) {
        match self {
            Self::WallHorizontal(_) | Self::WallVertical(_) => {
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
            Self::Bricks(coll) => {
                let uv = hit.uv / 3.0;
                bricks(coll, uv, hit)
            }
            _ => (0, 0),
        }
    }

    /// Handle the material node for the given terminal.
    pub fn material(&self, _in_terminal: &u8, hit: &mut Hit, palette: &ThePalette) -> Option<u8> {
        match self {
            Self::Material(collection) => {
                if let Some(TheValue::PaletteIndex(index)) = collection.get("Color") {
                    let randomness = collection.get_f32_default("Randomness", 0.0);
                    if let Some(color) = &palette.colors[*index as usize] {
                        if randomness == 0.0 {
                            hit.color = color.to_vec4f();
                        } else {
                            let random_factor = hash21(hit.uv * 1000.0) * randomness;
                            let r = (color.r + random_factor).clamp(0.0, 1.0);
                            let g = (color.g + random_factor).clamp(0.0, 1.0);
                            let b = (color.b + random_factor).clamp(0.0, 1.0);
                            hit.color = vec4f(r, g, b, color.a);
                        }
                    }
                }
                None
            }
            Self::Bricks(collection) => {
                let (_, terminal) = bricks(collection, hit.uv, hit);
                Some(terminal)
            }
            _ => None,
        }
    }

    /// Get a reference to the collection.
    pub fn collection(&self) -> &TheCollection {
        match self {
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
            Self::Bricks(collection) => collection,
            Self::Material(collection) => collection,
        }
    }

    /// Get a mutable reference to the collection.
    pub fn collection_mut(&mut self) -> &mut TheCollection {
        match self {
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
            Self::Bricks(collection) => collection,
            Self::Material(collection) => collection,
        }
    }

    /// Returns the distance to p.
    pub fn distance(&self, p: Vec3f) -> f32 {
        match self {
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
                )
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
                )
            }
            _ => 0.0,
        }
    }

    /// Role
    pub fn role(&self) -> ModelFXNodeRole {
        match self {
            Self::WallHorizontal(_) => Geometry,
            Self::WallVertical(_) => Geometry,
            Self::Bricks(_) => Material,
            Self::Material(_) => Material,
        }
    }

    /// Name
    pub fn name(&self) -> String {
        match self {
            Self::WallHorizontal(_) => str!("Wall Horizontal"),
            Self::WallVertical(_) => str!("Wall Vertical"),
            Self::Bricks(_) => str!("Bricks"),
            Self::Material(_) => str!("Material"),
        }
    }
}
