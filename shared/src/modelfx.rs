use crate::prelude::*;
use indexmap::IndexMap;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFX {
    Cube(TheCollection, ModelFXMetaData),
    WallHorizontal(TheCollection, ModelFXMetaData),
    WallVertical(TheCollection, ModelFXMetaData),
}

impl ModelFX {
    pub fn new_fx(name: &str, collection: Option<TheCollection>) -> ModelFX {
        let mut coll = TheCollection::named(name.into());
        match name {
            "Wall Horizontal" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Depth", TheValue::FloatRange(0.25, 0.0..=1.0));
                }
                let mut meta = ModelFXMetaData::new();
                meta.set_description("Depth", str!("The depth of the wall."));
                ModelFX::WallHorizontal(coll, meta)
            }
            "Wall Vertical" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Depth", TheValue::FloatRange(0.25, 0.0..=1.0));
                }
                let mut meta = ModelFXMetaData::new();
                meta.set_description("Depth", str!("The depth of the wall."));
                ModelFX::WallVertical(coll, meta)
            }
            _ => {
                let meta = ModelFXMetaData::new();
                ModelFX::Cube(coll, meta)
            }
        }
    }

    /// Parse the timeline and extract all models.
    pub fn parse_timeline(time: &TheTime, timeline: &TheTimeline) -> Vec<ModelFX> {
        let mut models = vec![];
        let collections = timeline.adjust_for_time(time);
        for c in collections {
            let fx = ModelFX::new_fx(&c.name, Some(c.clone()));
            models.push(fx);
        }

        models
    }

    /// Ray hit test for the ModelFX array.
    pub fn hit_array(ray: &Ray, array: &Vec<ModelFX>) -> Option<Hit> {
        let mut hit: Option<Hit> = None;
        for fx in array {
            if let Some(h) = fx.hit(ray) {
                if let Some(hit) = &mut hit {
                    if h.distance < hit.distance {
                        *hit = h;
                    }
                } else {
                    hit = Some(h);
                }
            }
        }
        hit
    }

    /// Ray hit test for the ModelFX.
    pub fn hit(&self, ray: &Ray) -> Option<Hit> {
        match self {
            ModelFX::Cube(_, _) => {
                let aabb_min = Vec3f::new(0.0, 0.0, 0.0);
                let aabb_max = Vec3f::new(1.0, 1.0, 1.0);
                self.ray_aabb(ray, aabb_min, aabb_max)
            }
            ModelFX::WallHorizontal(collection, _) => {
                let depth = collection.get_f32_default("Depth", 0.25);
                let aabb_min = Vec3f::new(0.0, 0.0, 0.5 - depth / 2.0);
                let aabb_max = Vec3f::new(1.0, 1.0, 0.5 + depth / 2.0);
                self.ray_aabb(ray, aabb_min, aabb_max)
            }
            ModelFX::WallVertical(collection, _) => {
                let depth = collection.get_f32_default("Depth", 0.25);
                let aabb_min = Vec3f::new(0.5 - depth / 2.0, 0.0, 0.0);
                let aabb_max = Vec3f::new(0.5 + depth / 2.0, 1.0, 1.0);
                self.ray_aabb(ray, aabb_min, aabb_max)
            }
        }
    }

    /// Convert to kind.
    pub fn to_kind(&self) -> String {
        match self {
            ModelFX::Cube(_, _) => str!("Cube"),
            ModelFX::WallHorizontal(_, _) => str!("Wall Horizontal"),
            ModelFX::WallVertical(_, _) => str!("Wall Vertical"),
        }
    }

    /// Reference to the collection.
    pub fn collection(&self) -> Option<&TheCollection> {
        match self {
            ModelFX::Cube(collection, _) => Some(collection),
            ModelFX::WallHorizontal(collection, _) => Some(collection),
            ModelFX::WallVertical(collection, _) => Some(collection),
        }
    }

    /// Convert to cloned collection.
    pub fn collection_cloned(&self) -> TheCollection {
        match self {
            ModelFX::Cube(collection, _) => collection.clone(),
            ModelFX::WallHorizontal(collection, _) => collection.clone(),
            ModelFX::WallVertical(collection, _) => collection.clone(),
        }
    }

    /// Get a reference to the meta data.
    pub fn meta_data(&self) -> Option<&ModelFXMetaData> {
        match self {
            ModelFX::Cube(_, meta) => Some(meta),
            ModelFX::WallHorizontal(_, meta) => Some(meta),
            ModelFX::WallVertical(_, meta) => Some(meta),
        }
    }

    /// Get the description of a key.
    pub fn get_description(&self, name: &str) -> String {
        if let Some(meta) = self.meta_data() {
            if let Some(description) = meta.description.get(name) {
                return description.clone();
            }
        }
        str!("")
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

            // Determine which face of the box was hit and calculate UV coordinates
            let mut u = 0.0;
            let mut v = 0.0;
            if normal == Vec3::new(1.0, 0.0, 0.0) || normal == Vec3::new(-1.0, 0.0, 0.0) {
                // Hit the X face
                v = 1.0 - (hit_point.y - aabb_min.y) / (aabb_max.y - aabb_min.y);
                u = (hit_point.z - aabb_min.z) / (aabb_max.z - aabb_min.z);
            } else if normal == Vec3::new(0.0, 1.0, 0.0) || normal == Vec3::new(0.0, -1.0, 0.0) {
                // Hit the Y face
                u = (hit_point.x - aabb_min.x) / (aabb_max.x - aabb_min.x);
                v = (hit_point.z - aabb_min.z) / (aabb_max.z - aabb_min.z);
            } else if normal == Vec3::new(0.0, 0.0, 1.0) || normal == Vec3::new(0.0, 0.0, -1.0) {
                // Hit the Z face
                u = (hit_point.x - aabb_min.x) / (aabb_max.x - aabb_min.x);
                v = 1.0 - (hit_point.y - aabb_min.y) / (aabb_max.y - aabb_min.y);
            }

            Some(Hit {
                distance: tmin,
                normal,
                uv: vec2f(u, v),
            })
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ModelFXMetaData {
    description: IndexMap<String, String>,
}

impl Default for ModelFXMetaData {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelFXMetaData {
    pub fn new() -> Self {
        Self {
            description: IndexMap::default(),
        }
    }

    pub fn set_description(&mut self, key: &str, description: String) {
        self.description.insert(str!(key), description);
    }
}
