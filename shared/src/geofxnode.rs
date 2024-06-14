use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
//use noiselib::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNodeRole {
    Ground,
    Column,
    LeftWall,
    TopWall,
    RightWall,
    BottomWall,
}

use GeoFXNodeRole::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct GeoFXNode {
    pub id: Uuid,
    pub role: GeoFXNodeRole,
    pub function: String,
    pub timeline: TheTimeline,
}

impl GeoFXNode {
    pub fn new(role: GeoFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Geo"));
        let mut function = str!("Wall");

        match role {
            Ground => {
                coll.set("UV Scale", TheValue::FloatRange(1.0, 0.0..=6.0));
                coll.set("Out Scale", TheValue::FloatRange(1.0, 0.0..=1.0));
                coll.set("Disp Scale", TheValue::FloatRange(0.1, 0.0..=1.0));
                coll.set("Octaves", TheValue::IntRange(5, 0..=5));
                function = str!("Ground");
            }
            Column => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Radius", TheValue::FloatRange(0.4, 0.001..=0.5));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=1.0));
                function = str!("Ground");
            }
            LeftWall => {
                coll.set("Pos X", TheValue::Float(0.1));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Thickness", TheValue::FloatRange(0.2, 0.001..=1.0));
                coll.set("Length", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=1.0));
            }
            TopWall => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.1));
                coll.set("Thickness", TheValue::FloatRange(0.2, 0.001..=1.0));
                coll.set("Length", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=1.0));
            }
            RightWall => {
                coll.set("Pos X", TheValue::Float(0.9));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Thickness", TheValue::FloatRange(0.2, 0.001..=1.0));
                coll.set("Length", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=1.0));
            }
            BottomWall => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.9));
                coll.set("Thickness", TheValue::FloatRange(0.2, 0.001..=1.0));
                coll.set("Length", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=1.0));
            }
        }
        let timeline = TheTimeline::collection(coll);

        Self {
            id: Uuid::new_v4(),
            role,
            function,
            timeline,
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new(GeoFXNodeRole::Ground),
            Self::new(GeoFXNodeRole::Column),
            Self::new(GeoFXNodeRole::LeftWall),
            Self::new(GeoFXNodeRole::TopWall),
            Self::new(GeoFXNodeRole::RightWall),
            Self::new(GeoFXNodeRole::BottomWall),
        ]
    }

    /// The 2D distance from the node to a point.
    pub fn distance(
        &self,
        _time: &TheTime,
        p: Vec2f,
        scale: f32,
        hit: &mut Option<&mut Hit>,
    ) -> f32 {
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            match self.role {
                Ground => {
                    if let Some(hit) = hit {
                        let value = noise2d(&coll, &hit.uv);
                        hit.albedo = vec3f(value, value, value);
                        hit.value = value;
                    }
                    return -0.001;
                }
                Column => {
                    let radius = coll.get_f32_default("Radius", 0.4);
                    return length(p - self.position() * scale) - radius * scale;
                }
                LeftWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.x = pos.x.floor() + thick.fract() / 2.0;

                    let d = abs(p - pos) - vec2f(thick, len);
                    return length(max(d, Vec2f::zero())) + min(max(d.x, d.y), 0.0);
                }
                TopWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.y = pos.y.floor() + thick.fract() / 2.0;

                    let d = abs(p - pos) - vec2f(len, thick);
                    return length(max(d, Vec2f::zero())) + min(max(d.x, d.y), 0.0);
                }
                RightWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.x = pos.x.floor() + 1.0 - thick.fract() / 2.0;

                    let d = abs(p - pos) - vec2f(thick, len);
                    return length(max(d, Vec2f::zero())) + min(max(d.x, d.y), 0.0);
                }
                BottomWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.y = pos.y.floor() + 1.0 - thick.fract() / 2.0;

                    let d = abs(p - pos) - vec2f(len, thick);
                    return length(max(d, Vec2f::zero())) + min(max(d.x, d.y), 0.0);
                }
            }
        }

        f32::INFINITY
    }

    /// The 3D distance from the node to a point.
    pub fn distance_3d(&self, _time: &TheTime, p: Vec3f, hit: &mut Option<&mut Hit>) -> f32 {
        // float opExtrusion( in vec3 p, in sdf2d primitive, in float h )
        // {
        //     float d = primitive(p.xy)
        //     vec2 w = vec2( d, abs(p.z) - h );
        //     return min(max(w.x,w.y),0.0) + length(max(w,0.0));
        // }

        fn op_extrusion_y(p: Vec3f, d: f32, h: f32) -> f32 {
            let w = Vec2f::new(d, abs(p.y) - h);
            min(max(w.x, w.y), 0.0) + length(max(w, Vec2f::zero()))
        }

        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            match self.role {
                Ground => {
                    let uv = p.xz();
                    let value = noise2d(&coll, &uv);
                    if let Some(hit) = hit {
                        hit.albedo = vec3f(value, value, value);
                        hit.value = value;
                    }
                    return p.y - value * 0.05;
                }
                Column => {
                    let radius = coll.get_f32_default("Radius", 0.4);
                    let height = coll.get_f32_default("Height", 1.0);
                    let d = length(vec2f(p.x, p.z) - self.position()) - radius;

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    return op_extrusion_y(p, d, height);
                }
                LeftWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let len = coll.get_f32_default("Length", 1.0) / 2.0 + 0.1;
                    let height = coll.get_f32_default("Height", 1.0);

                    let mut pos = self.position();
                    pos.x = pos.x.floor() + thick.fract() / 2.0;

                    let dd = abs(vec2f(p.x, p.z) - pos) - vec2f(thick, len);
                    let d = length(max(dd, Vec2f::zero())) + min(max(dd.x, dd.y), 0.0);

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    return op_extrusion_y(p, d, height);
                }
                TopWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let len = coll.get_f32_default("Length", 1.0) / 2.0 + 0.1;
                    let height = coll.get_f32_default("Height", 1.0);

                    let mut pos = self.position();
                    pos.y = pos.y.floor() + thick.fract() / 2.0;

                    let dd = abs(vec2f(p.x, p.z) - pos) - vec2f(len, thick);
                    let d = length(max(dd, Vec2f::zero())) + min(max(dd.x, dd.y), 0.0);

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    return op_extrusion_y(p, d, height);
                }
                RightWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let len = coll.get_f32_default("Length", 1.0) / 2.0 + 0.1;
                    let height = coll.get_f32_default("Height", 1.0);

                    let mut pos = self.position();
                    pos.x = pos.x.floor() + 1.0 - thick.fract() / 2.0;

                    let dd = abs(vec2f(p.x, p.z) - pos) - vec2f(thick, len);
                    let d = length(max(dd, Vec2f::zero())) + min(max(dd.x, dd.y), 0.0);

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    return op_extrusion_y(p, d, height);
                }
                BottomWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let len = coll.get_f32_default("Length", 1.0) / 2.0 + 0.1;
                    let height = coll.get_f32_default("Height", 1.0);

                    let mut pos = self.position();
                    pos.y = pos.y.floor() + 1.0 - thick.fract() / 2.0;

                    let dd = abs(vec2f(p.x, p.z) - pos) - vec2f(len, thick);
                    let d = length(max(dd, Vec2f::zero())) + min(max(dd.x, dd.y), 0.0);

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    return op_extrusion_y(p, d, height);
                }
            }
        }

        f32::INFINITY
    }

    pub fn aabb(&self, _time: &TheTime) -> Option<AABB2D> {
        // match self.role {
        //     Disc => {
        //         if let Some(value) =
        //             self.timeline
        //                 .get(str!("Geo"), str!("Radius"), time, TheInterpolation::Linear)
        //         {
        //             if let Some(radius) = value.to_f32() {
        //                 let position = self.position();
        //                 let min = Vec2f::new(position.x - radius, position.y - radius);
        //                 let max = Vec2f::new(position.x + radius, position.y + radius);
        //                 return Some(AABB2D::new(min, max));
        //             }
        //         }
        //     }
        // }

        let pos = self.position();
        Some(AABB2D::new(pos, pos))
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
            Ground => {
                pf.x += 0.5;
                pf.y += 0.5;
            }
            Column => {
                pf.x += 0.5;
                pf.y += 0.5;
            }
            LeftWall => {
                pf.x += 0.1;
                pf.y += 0.5;
            }
            TopWall => {
                pf.x += 0.5;
                pf.y += 0.1;
            }
            RightWall => {
                pf.x += 0.9;
                pf.y += 0.5;
            }
            BottomWall => {
                pf.x += 0.5;
                pf.y += 0.9;
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

        let mut hit = Hit::default();

        for y in 0..height {
            for x in 0..width {
                let p = vec2f(x as f32 / width as f32, y as f32 / height as f32);
                hit.uv = p;
                let d = self.distance(&TheTime::default(), p, 1.0, &mut Some(&mut hit));
                let t = smoothstep(-0.04, 0.0, d);
                if hit.value != 1.0 {
                    buffer.set_pixel(x, y, &TheColor::from_vec3f(hit.albedo).to_u8_array());
                } else {
                    buffer.set_pixel(x, y, &mix_color(&WHITE, &BLACK, t));
                }
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
