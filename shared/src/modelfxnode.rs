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
                    coll.set("Depth", TheValue::FloatRange(0.2, 0.0..=1.0));
                }
                Some(Self::WallHorizontal(coll))
            }
            "Wall Vertical" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Position", TheValue::FloatRange(0.5, 0.0..=1.0));
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
            "Material" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Color", TheValue::PaletteIndex(0));
                }
                Some(Self::Material(coll))
            }
            _ => None,
        }
    }

    /// List of input terminals.
    pub fn input_terminals(&self) -> Vec<ModelFXTerminal> {
        match self {
            Self::Material(_) => {
                vec![ModelFXTerminal::new(UV, 2)]
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
                    ModelFXTerminal::new(Face, 6),
                    ModelFXTerminal::new(Face, 7),
                    ModelFXTerminal::new(Face, 4),
                ]
            }
            _ => {
                vec![]
            }
        }
    }

    /// Return the color and terminal index for the given hit position.
    pub fn color_index_for_hit(&self, hit: &Hit) -> (u8, u8) {
        match self {
            Self::WallHorizontal(_) | Self::WallVertical(_) => {
                let nx = hit.normal.x.abs();
                let ny = hit.normal.y.abs();
                let nz = hit.normal.z.abs();

                if nx > ny && nx > nz {
                    // X-face
                    (6, 0)
                } else if ny > nx && ny > nz {
                    // Y-face
                    (7, 1)
                } else {
                    // Z-face
                    (4, 2)
                }
            }
            _ => (0, 0),
        }
    }

    /// Handle the material node for the given terminal.
    pub fn material(&self, _in_terminal: &u8, hit: &mut Hit, palette: &ThePalette) -> Option<u8> {
        match self {
            Self::Material(collection) => {
                if let Some(TheValue::PaletteIndex(index)) = collection.get("Color") {
                    if let Some(color) = &palette.colors[*index as usize] {
                        hit.color = color.to_vec4f();
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Get a reference to the collection.
    pub fn collection(&self) -> &TheCollection {
        match self {
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
            Self::Material(collection) => collection,
        }
    }

    /// Get a mutable reference to the collection.
    pub fn collection_mut(&mut self) -> &mut TheCollection {
        match self {
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
            Self::Material(collection) => collection,
        }
    }

    /// Returns the distance to p.
    pub fn distance(&self, p: Vec3f) -> f32 {
        match self {
            Self::WallHorizontal(collection) => {
                let position = collection.get_f32_default("Position", 0.5);
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
                // let aabb_min = Vec3f::new(0.0, 0.0, min);
                // let aabb_max = Vec3f::new(1.0, 1.0, max);
                sd_box(
                    p - vec3f(0.5, 0.5, (min + max) / 2.0),
                    vec3f(0.5, 0.5, (max - min) / 2.0),
                )
            }
            Self::WallVertical(collection) => {
                let position = collection.get_f32_default("Position", 0.5);
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
                //let aabb_min = Vec3f::new(min, 0.0, 0.0);
                //let aabb_max = Vec3f::new(max, 1.0, 1.0);
                //self.ray_aabb(ray, aabb_min, aabb_max)
                sd_box(
                    p - vec3f((min + max) / 2.0, 0.5, 0.5),
                    vec3f((max - min) / 2.0, 0.5, 0.5),
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
            Self::Material(_) => Material,
        }
    }

    /// Name
    pub fn name(&self) -> String {
        match self {
            Self::WallHorizontal(_) => str!("Wall Horizontal"),
            Self::WallVertical(_) => str!("Wall Vertical"),
            Self::Material(_) => str!("Material"),
        }
    }
}
