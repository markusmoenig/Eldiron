use crate::prelude::*;
//use indexmap::IndexMap;
//use rayon::prelude::*;
//use noiselib::prelude::*;
use theframework::prelude::*;

// https://www.shadertoy.com/view/3syGzz
// vec2 opRepLim( in vec2 p, in float s, in vec2 lima, in vec2 limb )
// {
//     p.x += s*.5* floor(mod(p.y/s+.5,2.));
//     return p-s*clamp(round(p/s),lima,limb);
// }
//
// vec2 opRep( in vec2 p, in float s )
// {
//     p.x += s*.5* floor(mod(p.y/s+.5,2.));
//     return mod(p+s*.5,s)-s*0.5;
// }

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNodeExtrusion {
    None,
    X,
    Y,
    Z,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNodeFacing {
    NorthSouth,
    WestEast,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum GeoFXNodeRole {
    Ground,
    Floor,
    Column,
    LeftWall,
    TopWall,
    RightWall,
    BottomWall,
    BendWallNW,
    BendWallNE,
    BendWallSW,
    BendWallSE,
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
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("UV Scale", TheValue::FloatRange(1.0, 0.0..=6.0));
                coll.set("Out Scale", TheValue::FloatRange(1.0, 0.0..=1.0));
                coll.set("Disp Scale", TheValue::FloatRange(0.1, 0.0..=1.0));
                coll.set("Octaves", TheValue::IntRange(5, 0..=5));
                function = str!("Ground");
            }
            Floor => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Height", TheValue::FloatRange(0.1, 0.001..=1.0));
                coll.set("Hole", TheValue::FloatRange(0.0, 0.0..=1.0));
                function = str!("Ground");
            }
            Column => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Radius", TheValue::FloatRange(0.4, 0.001..=2.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Hole", TheValue::FloatRange(0.0, 0.0..=1.0));
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
            BendWallNW | BendWallNE | BendWallSW | BendWallSE => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Thickness", TheValue::FloatRange(0.2, 0.001..=1.0));
                coll.set("Rounding", TheValue::FloatRange(0.3, 0.0..=1.0));
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
            Self::new(GeoFXNodeRole::Floor),
            Self::new(GeoFXNodeRole::Column),
            Self::new(GeoFXNodeRole::LeftWall),
            Self::new(GeoFXNodeRole::TopWall),
            Self::new(GeoFXNodeRole::RightWall),
            Self::new(GeoFXNodeRole::BottomWall),
            Self::new(GeoFXNodeRole::BendWallNW),
            Self::new(GeoFXNodeRole::BendWallNE),
            Self::new(GeoFXNodeRole::BendWallSW),
            Self::new(GeoFXNodeRole::BendWallSE),
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
                Floor => {
                    let mut pos = self.position() * scale;
                    pos.x = pos.x.floor();

                    let hole = coll.get_f32_default("Hole", 0.0) * scale;

                    let mut d = self.box2d(p, pos, 1.0 * scale, 1.0 * scale);

                    if hole > 0.0 {
                        d = d.abs() - hole;
                    }

                    return d;
                }
                Column => {
                    let radius = coll.get_f32_default("Radius", 0.4);

                    // let waveAmplitude = 0.05;
                    // let waveFrequency = 12.0 * 4.0; // Higher frequency for more fluting patterns

                    // let angle = atan2(p.y + 0.5, p.x + 0.5);

                    // // Modulate the radius with a sine wave to create fluting
                    // let wave = waveAmplitude * sin(waveFrequency * angle);

                    // // Calculate the modified radius
                    // let modifiedRadius = radius + wave; // * 0.05;

                    let hole = coll.get_f32_default("Hole", 0.0) * scale;

                    let mut d = length(p - self.position() * scale) - radius * scale + hole;
                    if hole > 0.0 {
                        d = d.abs() - hole;
                    }

                    return d;
                }
                LeftWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.x = pos.x.floor() + thick.fract() / 2.0;

                    return self.box2d(p, pos, thick, len);
                }
                TopWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.y = pos.y.floor() + thick.fract() / 2.0;

                    return self.box2d(p, pos, len, thick);
                }
                RightWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.x = pos.x.floor() + 1.0 - thick.fract() / 2.0;

                    return self.box2d(p, pos, thick, len);
                }
                BottomWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position() * scale;
                    pos.y = pos.y.floor() + 1.0 - thick.fract() / 2.0;

                    return self.box2d(p, pos, len, thick);
                }
                BendWallNW => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let pos = self.position() * scale + 1.0 * scale;
                    let rounding = (round, round, round, round);

                    let p = p - pos;

                    let size = if scale != 1.0 {
                        1.0 * scale
                    } else {
                        1.5 * scale
                    };

                    let d = self.rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
                }
                BendWallNE => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let mut pos = self.position() * scale;
                    pos += if scale != 1.0 {
                        vec2f(0.0, 1.0) * scale
                    } else {
                        vec2f(-1.0, 1.0) * scale
                    };

                    let rounding = (round, round, round, round);

                    let p = p - pos;

                    let size = if scale != 1.0 {
                        1.0 * scale
                    } else {
                        1.5 * scale
                    };

                    let d = self.rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
                }
                BendWallSW => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let mut pos = self.position() * scale;
                    pos += if scale != 1.0 {
                        vec2f(1.0, 0.0) * scale
                    } else {
                        vec2f(1.0, -1.0) * scale
                    };
                    let rounding = (round, round, round, round);

                    let p = p - pos;

                    let size = if scale != 1.0 {
                        1.0 * scale
                    } else {
                        1.5 * scale
                    };

                    let d = self.rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
                }
                BendWallSE => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let mut pos = self.position() * scale;
                    pos += if scale != 1.0 {
                        vec2f(0.0, 0.0) * scale
                    } else {
                        vec2f(-1.0, -1.0) * scale
                    };

                    let rounding = (round, round, round, round);

                    let p = p - pos;

                    let size = if scale != 1.0 {
                        1.0 * scale
                    } else {
                        1.5 * scale
                    };

                    let d = self.rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
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
                Floor => {
                    let height = coll.get_f32_default("Height", 0.01);
                    let hole = coll.get_f32_default("Hole", 0.0);

                    let pos = self.position();
                    let mut d = self.box2d(vec2f(p.x, p.z), pos, 1.0, 1.0);

                    if hole > 0.0 {
                        d = d.abs() - hole;
                    }

                    if let Some(hit) = hit {
                        hit.pattern_pos = vec2f(p.x, p.z);
                        hit.extrusion = GeoFXNodeExtrusion::Y;
                        hit.extrusion_length = height;
                        hit.interior_distance = d;
                        hit.hit_point = p - vec3f(pos.x.floor() + 0.5, 0.0, 0.0);
                    }

                    return d;
                }
                Column => {
                    let radius = coll.get_f32_default("Radius", 0.4);
                    let height = coll.get_f32_default("Height", 1.0);
                    let hole = coll.get_f32_default("Hole", 0.0);

                    let pos = self.position();
                    let mut d = length(vec2f(p.x, p.z) - self.position()) - radius + hole;
                    if hole > 0.0 {
                        d = d.abs() - hole;
                    }

                    if let Some(hit) = hit {
                        hit.pattern_pos = vec2f(p.x, p.z);
                        hit.extrusion = GeoFXNodeExtrusion::Y;
                        hit.extrusion_length = height;
                        hit.interior_distance = d;
                        hit.hit_point = p - vec3f(pos.x.floor(), 0.0, pos.y.floor());
                    }

                    return d;
                }
                LeftWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let len = coll.get_f32_default("Length", 1.0) / 2.0;
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position();
                    let d = self.box2d(vec2f(p.z, p.y), vec2f(pos.y, height / 2.0), len, height);

                    if let Some(hit) = hit {
                        hit.pattern_pos = vec2f(p.z, p.y);
                        hit.extrusion = GeoFXNodeExtrusion::X;
                        hit.extrusion_length = coll.get_f32_default("Thickness", 0.2);
                        hit.interior_distance = d;
                        hit.hit_point = p - vec3f(pos.x.floor() + thick.fract() / 2.0, 0.0, 0.0);
                    }

                    return d;
                }
                TopWall => {
                    let len = coll.get_f32_default("Length", 1.0) / 2.0;
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position();
                    let d = self.box2d(vec2f(p.x, p.y), vec2f(pos.x, height / 2.0), len, height);

                    if let Some(hit) = hit {
                        hit.pattern_pos = vec2f(p.x, p.y);
                        hit.extrusion = GeoFXNodeExtrusion::Z;
                        hit.extrusion_length = coll.get_f32_default("Thickness", 0.2);
                        hit.interior_distance = d;
                        hit.hit_point =
                            p - vec3f(0.0, 0.0, pos.y.floor() + hit.extrusion_length.fract() / 2.0);
                    }

                    return d;
                }
                RightWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let len = coll.get_f32_default("Length", 1.0) / 2.0;
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position();
                    let d = self.box2d(vec2f(p.z, p.y), vec2f(pos.y, height / 2.0), len, height);

                    if let Some(hit) = hit {
                        hit.pattern_pos = vec2f(p.z, p.y);
                        hit.extrusion = GeoFXNodeExtrusion::X;
                        hit.extrusion_length = coll.get_f32_default("Thickness", 0.2);
                        hit.interior_distance = d;
                        hit.hit_point =
                            p - vec3f(pos.x.floor() + 1.0 - thick.fract() / 2.0, 0.0, 0.0);
                    }

                    return d;
                }
                BottomWall => {
                    let len = coll.get_f32_default("Length", 1.0) / 2.0;
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position();
                    let d = self.box2d(vec2f(p.x, p.y), vec2f(pos.x, height / 2.0), len, height);

                    if let Some(hit) = hit {
                        hit.pattern_pos = vec2f(p.x, p.y);
                        hit.extrusion = GeoFXNodeExtrusion::Z;
                        hit.extrusion_length = coll.get_f32_default("Thickness", 0.2);
                        hit.interior_distance = d;
                        hit.hit_point = p - vec3f(
                            0.0,
                            0.0,
                            pos.y.floor() + 1.0 - hit.extrusion_length.fract() / 2.0,
                        );
                    }

                    return d;
                }
                BendWallNW => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let round = coll.get_f32_default("Rounding", 0.3);
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position() + 1.0;
                    let rounding = (round, round, round, round);

                    let pp = vec2f(p.x, p.z) - pos;

                    let size = 1.0;
                    let mut d = self.rounded_box2d(pp, size, thick, rounding);

                    d = d.abs() - thick;

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    let d = op_extrusion_y(p, d, height);
                    let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                    return max(-plane, d);
                }
                BendWallNE => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let round = coll.get_f32_default("Rounding", 0.3);
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position() + vec2f(0.0, 1.0);
                    let rounding = (round, round, round, round);

                    let pp = vec2f(p.x, p.z) - pos;

                    let size = 1.0;
                    let mut d = self.rounded_box2d(pp, size, thick, rounding);

                    d = d.abs() - thick;

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    let d = op_extrusion_y(p, d, height);
                    let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                    return max(-plane, d);
                }
                BendWallSW => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let round = coll.get_f32_default("Rounding", 0.3);
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position() + vec2f(1.0, 0.0);
                    let rounding = (round, round, round, round);

                    let pp = vec2f(p.x, p.z) - pos;

                    let size = 1.0;
                    let mut d = self.rounded_box2d(pp, size, thick, rounding);

                    d = d.abs() - thick;

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    let d = op_extrusion_y(p, d, height);
                    let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                    return max(-plane, d);
                }
                BendWallSE => {
                    let thick = coll.get_f32_default("Thickness", 0.2);
                    let round = coll.get_f32_default("Rounding", 0.3);
                    let height = coll.get_f32_default("Height", 1.0);

                    let pos = self.position();
                    let rounding = (round, round, round, round);

                    let pp = vec2f(p.x, p.z) - pos;

                    let size = 1.0;
                    let mut d = self.rounded_box2d(pp, size, thick, rounding);

                    d = d.abs() - thick;

                    if let Some(hit) = hit {
                        hit.interior_distance = d;
                    }

                    let d = op_extrusion_y(p, d, height);
                    let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                    return max(-plane, d);
                }
            }
        }

        f32::INFINITY
    }

    /// Returns all tiles which are touched by this geometry.
    pub fn area(&self) -> Vec<Vec2i> {
        let mut area = Vec::new();
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Geo"))
        {
            match self.role {
                Column => {
                    let radius = coll.get_f32_default("Radius", 0.4);

                    let center = self.position();
                    let min_x = (center.x - radius).floor() as i32;
                    let max_x = (center.x + radius).ceil() as i32;
                    let min_y = (center.y - radius).floor() as i32;
                    let max_y = (center.y + radius).ceil() as i32;

                    fn tile_intersects_disc(center: Vec2f, radius: f32, x: i32, y: i32) -> bool {
                        let closest_x = if center.x < x as f32 {
                            x as f32
                        } else if center.x > (x + 1) as f32 {
                            (x + 1) as f32
                        } else {
                            center.x
                        };
                        let closest_y = if center.y < y as f32 {
                            y as f32
                        } else if center.y > (y + 1) as f32 {
                            (y + 1) as f32
                        } else {
                            center.y
                        };

                        let dist_x = center.x - closest_x;
                        let dist_y = center.y - closest_y;

                        dist_x * dist_x + dist_y * dist_y <= radius * radius
                    }

                    for x in min_x..=max_x {
                        for y in min_y..=max_y {
                            if tile_intersects_disc(center, radius, x, y) {
                                area.push(Vec2i::new(x, y));
                            }
                        }
                    }
                }
                _ => {
                    area.push(Vec2i::from(self.position()));
                }
            }
        }
        area
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
            _ => {}
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

    #[inline(always)]
    fn box2d(&self, p: Vec2f, pos: Vec2f, dim1: f32, dim2: f32) -> f32 {
        let d = abs(p - pos) - vec2f(dim1, dim2);
        length(max(d, Vec2f::zero())) + min(max(d.x, d.y), 0.0)
    }

    #[inline(always)]
    fn rounded_box2d(
        &self,
        p: Vec2f,
        size: f32,
        thick: f32,
        rounding: (f32, f32, f32, f32),
    ) -> f32 {
        let mut r: (f32, f32);

        if p.x > 0.0 {
            r = (rounding.0, rounding.1);
        } else {
            r = (rounding.2, rounding.3);
        }

        if p.y <= 0.0 {
            r.0 = r.1;
        }
        let hb = thick / 2.0;
        let q: (f32, f32) = (
            p.x.abs() - size + hb + rounding.0,
            p.y.abs() - size + hb + rounding.0,
        );
        f32::min(f32::max(q.0, q.1), 0.0) + length(vec2f(f32::max(q.0, 0.0), f32::max(q.1, 0.0)))
            - rounding.0
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
                    let uv_scaled = p * 10.0;
                    let square_pos = Vec2i::from(floor(uv_scaled));

                    let color = if (square_pos.x + square_pos.y) % 2 == 0 {
                        [81, 81, 81, 255]
                    } else {
                        [209, 209, 209, 255]
                    };

                    buffer.set_pixel(x, y, &mix_color(&color, &BLACK, t));
                }
            }
        }
    }
}
