use crate::prelude::*;
use indexmap::IndexMap;
use rayon::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum ModelFXWall {
    Empty(TheCollection, ModelFXMetaData),
    WallHorizontal(TheCollection, ModelFXMetaData),
    WallVertical(TheCollection, ModelFXMetaData),
}

impl ModelFXWall {
    pub fn new_fx(name: &str, collection: Option<TheCollection>) -> Self {
        let mut coll = TheCollection::named(name.into());
        match name {
            "Wall Horizontal" => {
                let mut meta = ModelFXMetaData::new();
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
                    ModelFXWall::add_pattern(&mut coll, &mut meta);
                }
                meta.set_description("Position", str!("The position of the wall."));
                meta.set_description("Depth", str!("The depth of the wall."));
                Self::WallHorizontal(coll, meta)
            }
            "Wall Vertical" => {
                let mut meta = ModelFXMetaData::new();
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
                    ModelFXWall::add_pattern(&mut coll, &mut meta);
                }
                meta.set_description("Depth", str!("The depth of the wall."));
                Self::WallVertical(coll, meta)
            }
            _ => Self::Empty(TheCollection::new(), ModelFXMetaData::new()),
        }
    }

    /// Add the pattern properties to the collection.
    pub fn add_pattern(coll: &mut TheCollection, _meta: &mut ModelFXMetaData) {
        coll.set("Pattern Size", TheValue::FloatRange(0.5, 0.0..=1.0));

        coll.set("Color #1", TheValue::PaletteIndex(0));
        coll.set("Color #2", TheValue::PaletteIndex(1));
    }

    /// Returns the unsupported pattern properties for the collection.
    pub fn unsupported(coll: &TheCollection) -> Vec<String> {
        let pattern = coll.get_i32_default("Pattern", 0);

        if pattern == 1 {
            // Color
            vec![str!("Pattern Size"), str!("Color #2")]
        } else if pattern == 2 {
            // Brick
            vec![]
        } else {
            // Tile
            vec![str!("Pattern Size"), str!("Color #1"), str!("Color #2")]
        }
    }

    /// Create an array of all models.
    pub fn fx_array() -> Vec<Self> {
        vec![
            Self::new_fx("Wall Horizontal", None),
            Self::new_fx("Wall Vertical", None),
        ]
    }

    /// Parse the timeline and extract all models.
    pub fn parse_timeline(time: &TheTime, timeline: &TheTimeline) -> Vec<Self> {
        let mut models = vec![];
        let array = Self::fx_array();
        let collections = timeline.adjust_for_time(time);
        for c in collections {
            for a in array.iter() {
                if a.to_kind() == c.name {
                    let fx = Self::new_fx(&c.name, Some(c.clone()));
                    models.push(fx);
                }
            }
        }

        models
    }

    /// Ray hit test for the ModelFX array.
    pub fn hit_array(ray: &Ray, array: &Vec<Self>) -> Option<Hit> {
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
            Self::WallHorizontal(collection, _) => {
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
            Self::WallVertical(collection, _) => {
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
            _ => None,
        }
    }

    /// Convert to kind.
    pub fn to_kind(&self) -> String {
        match self {
            Self::WallHorizontal(_, _) => str!("Wall Horizontal"),
            Self::WallVertical(_, _) => str!("Wall Vertical"),
            _ => str!("Empty"),
        }
    }

    /// Reference to the collection.
    pub fn collection(&self) -> &TheCollection {
        match self {
            Self::Empty(collection, _) => collection,
            Self::WallHorizontal(collection, _) => collection,
            Self::WallVertical(collection, _) => collection,
        }
    }

    /// Convert to cloned collection.
    pub fn collection_cloned(&self) -> TheCollection {
        match self {
            Self::Empty(collection, _) => collection.clone(),
            Self::WallHorizontal(collection, _) => collection.clone(),
            Self::WallVertical(collection, _) => collection.clone(),
        }
    }

    /// Get a reference to the meta data.
    pub fn meta_data(&self) -> Option<&ModelFXMetaData> {
        match self {
            Self::Empty(_, meta) => Some(meta),
            Self::WallHorizontal(_, meta) => Some(meta),
            Self::WallVertical(_, meta) => Some(meta),
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

    pub fn render_preview(buffer: &mut TheRGBABuffer, fx: Vec<ModelFXWall>) {
        let width = buffer.dim().width as usize;
        let height = buffer.dim().height as usize;

        let ro = vec3f(2.0, 2.0, 2.0);
        let rd = vec3f(0.0, 0.0, 0.0);

        let aa = 2;
        let aa_f = aa as f32;

        let camera = Camera::new(ro, rd, 160.0);

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let xx = (i % width) as f32;
                    let yy = (i / width) as f32;

                    let mut total = Vec4f::zero();

                    for m in 0..aa {
                        for n in 0..aa {
                            let camera_offset =
                                vec2f(m as f32 / aa_f, n as f32 / aa_f) - vec2f(0.5, 0.5);

                            let mut color = vec4f(0.01, 0.01, 0.01, 1.0);

                            let ray = camera.create_ortho_ray(
                                vec2f(xx / width as f32, 1.0 - yy / height as f32),
                                vec2f(width as f32, height as f32),
                                camera_offset,
                            );

                            let mut hit: Option<Hit> = None;

                            for fx in fx.iter() {
                                if let Some(h) = fx.hit(&ray) {
                                    if let Some(chit) = hit.clone() {
                                        if h.distance < chit.distance {
                                            hit = Some(h);
                                        }
                                    } else {
                                        hit = Some(h);
                                    }
                                }
                            }

                            if let Some(hit) = hit {
                                let c =
                                    dot(hit.normal, normalize(vec3f(1.0, 2.0, 3.0))) * 0.5 + 0.5;
                                color.x = c;
                                color.y = c;
                                color.z = c;
                            }

                            total += color;
                        }
                    }

                    let aa_aa = aa_f * aa_f;
                    total[0] /= aa_aa;
                    total[1] /= aa_aa;
                    total[2] /= aa_aa;
                    total[3] /= aa_aa;

                    pixel.copy_from_slice(&TheColor::from_vec4f(total).to_u8_array());
                }
            });

        /*
        for y in 0..height {
            for x in 0..width {
                let uv = vec2f(x as f32 / width as f32, y as f32 / height as f32);
                let mut color = vec4f(0.01, 0.01, 0.01, 1.0);

                let ray =
                    camera.create_ortho_ray(uv, vec2f(width as f32, height as f32), Vec2f::one());

                if let Some(hit) = fx.hit(&ray) {
                    //color = vec4f(1.0, 0.0, 0.0, 1.0);
                    //float dif = dot(n, normalize(vec3(1,2,3)))*.5+.5;
                    let c = dot(hit.normal, normalize(vec3f(1.0, 2.0, 3.0))) * 0.5 + 0.5;
                    color.x = c;
                    color.y = c;
                    color.z = c;
                }

                buffer.set_pixel(x, y, TheColor::from_vec4f(color).to_u8_array());
            }
            }*/
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct ModelFXMetaData {
    pub description: IndexMap<String, String>,
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

    /// Get the description of a key.
    pub fn get_description(&self, name: &str) -> String {
        if let Some(description) = self.description.get(name) {
            return description.clone();
        }
        str!("")
    }
}
