use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
use crate::modelfxterminal::ModelFXTerminalRole::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXNode {
    WallHorizontal(TheCollection),
    WallVertical(TheCollection),
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
            _ => None,
        }
    }

    /// Get a reference to the collection.
    pub fn output_terminals(&self) -> Vec<ModelFXTerminal> {
        match self {
            Self::WallHorizontal(_) | Self::WallVertical(_) => {
                vec![
                    ModelFXTerminal::new(Face, 6),
                    ModelFXTerminal::new(Face, 7),
                    ModelFXTerminal::new(Face, 4),
                ]
            }
        }
    }

    pub fn color_for_normal(&self, normal: Vec3f) -> ModelFXColor {
        match self {
            Self::WallHorizontal(_) | Self::WallVertical(_) => {
                let nx = normal.x.abs();
                let ny = normal.y.abs();
                let nz = normal.z.abs();

                if nx > ny && nx > nz {
                    // X-face
                    ModelFXColor::create(6)
                } else if ny > nx && ny > nz {
                    // Y-face
                    ModelFXColor::create(7)
                } else {
                    // Z-face
                    ModelFXColor::create(4)
                }
            }
        }
    }

    /// Get a reference to the collection.
    pub fn collection(&self) -> &TheCollection {
        match self {
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
        }
    }

    /// Get a mutable reference to the collection.
    pub fn collection_mut(&mut self) -> &mut TheCollection {
        match self {
            Self::WallHorizontal(collection) => collection,
            Self::WallVertical(collection) => collection,
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
        }
    }

    /// Convert to kind.
    pub fn to_kind(&self) -> String {
        match self {
            Self::WallHorizontal(_) => str!("Wall Horizontal"),
            Self::WallVertical(_) => str!("Wall Vertical"),
        }
    }
}
