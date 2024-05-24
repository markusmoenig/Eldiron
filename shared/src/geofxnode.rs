//use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
//use noiselib::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNode {
    Disc(TheCollection),
}

impl GeoFXNode {
    pub fn new_node(name: &str, collection: Option<TheCollection>) -> Option<Self> {
        let mut coll = TheCollection::named(name.into());
        match name {
            "Disc" => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Radius", TheValue::FloatRange(0.01, 0.0..=5.0));
                }
                Some(Self::Disc(coll))
            }
            // Box
            _ => {
                if let Some(collection) = collection {
                    coll = collection;
                } else {
                    coll.set("Size", TheValue::FloatRange(0.01, 0.0..=5.0));
                }
                Some(Self::Disc(coll))
            }
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new_node("Disc", None).unwrap(),
            Self::new_node("Box", None).unwrap(),
        ]
    }

    pub fn distance(&self, p: Vec2f, coll: &TheCollection) -> f32 {
        match self {
            Self::Disc(_) => {
                let radius = coll.get_f32_default("Radius", 0.5);
                length(p) - radius
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
        let mut collection = TheCollection::default();
        collection.set("Radius", TheValue::Float(0.4));

        for y in 0..height {
            for x in 0..width {
                let p = vec2f(
                    x as f32 / width as f32 - 0.5,
                    y as f32 / height as f32 - 0.5,
                );
                let d = self.distance(p, &collection);
                let t = smoothstep(-0.04, 0.0, d);
                buffer.set_pixel(x, y, &mix_color(&WHITE, &BLACK, t));
            }
        }
    }
}
