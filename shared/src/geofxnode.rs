use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
//use noiselib::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNodeRole {
    Disc,
}

use GeoFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXNode {
    pub id: Uuid,
    pub role: GeoFXNodeRole,
    pub timeline: TheTimeline,
}

impl GeoFXNode {
    pub fn new(role: GeoFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Geo"));
        coll.set("Pos X", TheValue::Float(0.5));
        coll.set("Pos Y", TheValue::Float(0.5));
        coll.set("Radius", TheValue::FloatRange(0.4, 0.001..=5.0));
        let timeline = TheTimeline::collection(coll);

        Self {
            id: Uuid::new_v4(),
            role,
            timeline,
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![Self::new(GeoFXNodeRole::Disc)]
    }

    pub fn distance(&self, time: &TheTime, p: Vec2f, scale: f32) -> f32 {
        match self.role {
            Disc => {
                if let Some(value) =
                    self.timeline
                        .get(str!("Geo"), str!("Radius"), time, TheInterpolation::Linear)
                {
                    if let Some(radius) = value.to_f32() {
                        return length(p - self.position() * scale) - radius * scale;
                    }
                }
            }
        }

        f32::INFINITY
    }

    pub fn aabb(&self, time: &TheTime) -> Option<AABB2D> {
        match self.role {
            Disc => {
                if let Some(value) =
                    self.timeline
                        .get(str!("Geo"), str!("Radius"), time, TheInterpolation::Linear)
                {
                    if let Some(radius) = value.to_f32() {
                        let position = self.position();
                        let min = Vec2f::new(position.x - radius, position.y - radius);
                        let max = Vec2f::new(position.x + radius, position.y + radius);
                        return Some(AABB2D::new(min, max));
                    }
                }
            }
        }

        None
    }

    pub fn position(&self) -> Vec2f {
        let mut x = 0.0;
        let mut y = 0.0;
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            x = coll.get("Pos X").unwrap().to_f32().unwrap();
            y = coll.get("Pos Y").unwrap().to_f32().unwrap();
        }

        vec2f(x, y)
    }

    pub fn set_default_position(&mut self, p: Vec2i) {
        let mut pf = vec2f(p.x as f32, p.y as f32);
        match self.role {
            Disc => {
                pf.x += 0.5;
                pf.y += 0.5;
            }
        }
        self.set("Pos X", TheValue::Float(pf.x));
        self.set("Pos Y", TheValue::Float(pf.y));
    }

    pub fn collection(&self) -> TheCollection {
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            return coll;
        }

        TheCollection::default()
    }

    pub fn set(&mut self, key: &str, value: TheValue) {
        self.timeline.set(&TheTime::default(), key, "Geo", value);
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
                let p = vec2f(x as f32 / width as f32, y as f32 / height as f32);
                let d = self.distance(&TheTime::default(), p, 1.0);
                let t = smoothstep(-0.04, 0.0, d);
                buffer.set_pixel(x, y, &mix_color(&WHITE, &BLACK, t));
            }
        }
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