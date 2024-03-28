use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
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

    /// Ray hit test for the ModelFX.
    pub fn hit(&self, ray: &Ray) -> Option<Hit> {
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
                let aabb_min = Vec3f::new(0.0, 0.0, min);
                let aabb_max = Vec3f::new(1.0, 1.0, max);
                self.ray_aabb(ray, aabb_min, aabb_max)
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
                let aabb_min = Vec3f::new(min, 0.0, 0.0);
                let aabb_max = Vec3f::new(max, 1.0, 1.0);
                self.ray_aabb(ray, aabb_min, aabb_max)
            }
        }
    }

    /// Ray AABB / Cube hit test.
    pub fn ray_aabb(&self, ray: &Ray, aabb_min: Vec3f, aabb_max: Vec3f) -> Option<Hit> {
        let t0s = (aabb_min - ray.o) * ray.inv_direction;
        let t1s = (aabb_max - ray.o) * ray.inv_direction;

        let mut tmin = f32::NEG_INFINITY;
        let mut tmax = f32::INFINITY;
        let mut normal = Vec3::new(0.0, 0.0, 0.0);

        for i in 0..3 {
            let axis_normal = match i {
                0 => Vec3f::new(1.0, 0.0, 0.0),
                1 => Vec3f::new(0.0, 1.0, 0.0),
                _ => Vec3f::new(0.0, 0.0, 1.0),
            };
            if ray.inv_direction[i] >= 0.0 {
                if t0s[i] > tmin {
                    tmin = t0s[i];
                    normal = axis_normal * -1.0; // Invert the normal if we're hitting the min side
                }
                tmax = tmax.min(t1s[i]);
            } else {
                if t1s[i] > tmin {
                    tmin = t1s[i];
                    normal = axis_normal; // Normal points in the positive axis direction
                }
                tmax = tmax.min(t0s[i]);
            }
        }

        if tmax >= tmin && tmin >= 0.0 {
            // Calculate intersection point
            let hit_point = ray.o + ray.d * tmin;
            let mut face = HitFace::XFace;

            // Determine which face of the box was hit and calculate UV coordinates
            let mut u = 0.0;
            let mut v = 0.0;
            if normal == Vec3::new(1.0, 0.0, 0.0) || normal == Vec3::new(-1.0, 0.0, 0.0) {
                // Hit the X face
                v = 1.0 - (hit_point.y - aabb_min.y) / (aabb_max.y - aabb_min.y);
                u = (hit_point.z - aabb_min.z) / (aabb_max.z - aabb_min.z);
                face = HitFace::XFace;
            } else if normal == Vec3::new(0.0, 1.0, 0.0) || normal == Vec3::new(0.0, -1.0, 0.0) {
                // Hit the Y face
                u = (hit_point.x - aabb_min.x) / (aabb_max.x - aabb_min.x);
                v = (hit_point.z - aabb_min.z) / (aabb_max.z - aabb_min.z);
                face = HitFace::YFace;
            } else if normal == Vec3::new(0.0, 0.0, 1.0) || normal == Vec3::new(0.0, 0.0, -1.0) {
                // Hit the Z face
                u = (hit_point.x - aabb_min.x) / (aabb_max.x - aabb_min.x);
                v = 1.0 - (hit_point.y - aabb_min.y) / (aabb_max.y - aabb_min.y);
                face = HitFace::ZFace;
            }

            Some(Hit {
                distance: tmin,
                hit_point,
                normal,
                uv: vec2f(u, v),
                face,
            })
        } else {
            None
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
