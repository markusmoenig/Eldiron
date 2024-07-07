use crate::prelude::*;
use rayon::prelude::*;
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
    Gate,
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

        match role {
            Ground => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("UV Scale X", TheValue::FloatRange(1.0, 0.0..=10.0));
                coll.set("UV Scale Y", TheValue::FloatRange(1.0, 0.0..=10.0));
                coll.set("Octaves", TheValue::IntRange(5, 0..=5));
            }
            Floor => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Height", TheValue::FloatRange(0.01, 0.001..=1.0));
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
            Column => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set("Radius", TheValue::FloatRange(0.4, 0.001..=2.0));
                coll.set("Height", TheValue::FloatRange(1.0, 0.001..=1.0));
                coll.set("Hole", TheValue::FloatRange(0.0, 0.0..=1.0));
            }
            Gate => {
                coll.set("Pos X", TheValue::Float(0.5));
                coll.set("Pos Y", TheValue::Float(0.5));
                coll.set(
                    "Align",
                    TheValue::TextList(0, vec![str!("North/South"), str!("West/East")]),
                );
                coll.set("Height", TheValue::FloatRange(0.8, 0.001..=1.0));
            }
        }
        let timeline = TheTimeline::collection(coll);

        Self {
            id: Uuid::new_v4(),
            role,
            timeline,
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new(GeoFXNodeRole::Ground),
            Self::new(GeoFXNodeRole::Floor),
            Self::new(GeoFXNodeRole::LeftWall),
            Self::new(GeoFXNodeRole::TopWall),
            Self::new(GeoFXNodeRole::RightWall),
            Self::new(GeoFXNodeRole::BottomWall),
            Self::new(GeoFXNodeRole::BendWallNW),
            Self::new(GeoFXNodeRole::BendWallNE),
            Self::new(GeoFXNodeRole::BendWallSW),
            Self::new(GeoFXNodeRole::BendWallSE),
            Self::new(GeoFXNodeRole::Column),
            Self::new(GeoFXNodeRole::Gate),
        ]
    }

    /// Returns the layer role (Ground, Wall etc) for this node.
    pub fn get_layer_role(&self) -> Layer2DRole {
        match self.role {
            GeoFXNodeRole::Ground | GeoFXNodeRole::Floor => Layer2DRole::Ground,
            _ => Layer2DRole::Wall,
        }
    }

    /// Gives the node a chance to update its parameters in case things changed.
    pub fn update_parameters(&mut self) {
        // match self.role {
        //     Floor => {
        //         if let Some(coll) = self
        //             .timeline
        //             .get_collection_at(&TheTime::default(), str!("Geo"))
        //         {
        //             if coll.get_f32_default("Height", 0.01) == 0.1 {
        //                 self.set("Height", TheValue::FloatRange(0.01, 0.001..=1.0));
        //             }
        //         }
        //     }
        //     _ => {}
        // }
    }

    /// Loads the parameters of the nodes into memory for faster access.
    pub fn load_parameters(&self, time: &TheTime) -> Vec<f32> {
        let mut params = vec![];

        if let Some(coll) = self.timeline.get_collection_at(time, str!("Geo")) {
            params.push(coll.get_f32_default("Pos X", 0.0));
            params.push(coll.get_f32_default("Pos Y", 0.0));
            match self.role {
                Ground => {
                    params.push(coll.get_f32_default("UV Scale X", 1.0));
                    params.push(coll.get_f32_default("UV Scale Y", 1.0));
                    params.push(coll.get_i32_default("Octaves", 5) as f32);
                }
                Floor => {
                    params.push(coll.get_f32_default("Height", 0.01));
                }
                Column => {
                    params.push(coll.get_f32_default("Radius", 0.4));
                    params.push(coll.get_f32_default("Height", 1.0));
                    params.push(coll.get_f32_default("Hole", 0.0));
                }
                LeftWall | TopWall | RightWall | BottomWall => {
                    params.push(coll.get_f32_default("Thickness", 0.2));
                    params.push(coll.get_f32_default("Length", 1.0) / 2.0 + 0.1);
                    params.push(coll.get_f32_default("Height", 1.0));
                }
                BendWallNW | BendWallNE | BendWallSW | BendWallSE => {
                    params.push(coll.get_f32_default("Thickness", 0.2));
                    params.push(coll.get_f32_default("Rounding", 0.3));
                    params.push(coll.get_f32_default("Height", 1.0));
                }
                Gate => {
                    params.push(coll.get_i32_default("Align", 0) as f32);
                    params.push(coll.get_f32_default("Height", 0.8));
                }
            }
        }

        params
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
                        let scale = vec2f(
                            coll.get_f32_default("UV Scale X", 1.0),
                            coll.get_f32_default("UV Scale Y", 1.0),
                        );
                        let octaves = coll.get_i32_default("Octaves", 5);
                        let value = if hit.two_d {
                            noise2d(&hit.global_uv, scale, octaves)
                        } else {
                            noise2d(&hit.uv, scale, octaves)
                        };
                        hit.mat.base_color = vec3f(value, value, value);
                        hit.value = value;
                    }
                    return -0.001;
                }
                Floor => {
                    let pos = self.position(&coll) * scale;
                    let d = sdf_box2d(p, pos, 0.6 * scale, 0.6 * scale);

                    return d;
                }
                LeftWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position(&coll) * scale;
                    pos.x = pos.x.floor() + thick.fract() / 2.0;

                    return sdf_box2d(p, pos, thick, len);
                }
                TopWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position(&coll) * scale;
                    pos.y = pos.y.floor() + thick.fract() / 2.0;

                    return sdf_box2d(p, pos, len, thick);
                }
                RightWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position(&coll) * scale;
                    pos.x = pos.x.floor() + 1.0 - thick.fract() / 2.0;

                    return sdf_box2d(p, pos, thick, len);
                }
                BottomWall => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let len = coll.get_f32_default("Length", 1.0) * scale / 2.0 + 0.1;

                    let mut pos = self.position(&coll) * scale;
                    pos.y = pos.y.floor() + 1.0 - thick.fract() / 2.0;

                    return sdf_box2d(p, pos, len, thick);
                }
                BendWallNW => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let pos = self.position(&coll) * scale + 1.0 * scale;
                    let rounding = (round, round, round, round);

                    let p = p - pos;

                    let size = if scale != 1.0 {
                        1.0 * scale
                    } else {
                        1.5 * scale
                    };

                    let d = sdf_rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
                }
                BendWallNE => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let mut pos = self.position(&coll) * scale;
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

                    let d = sdf_rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
                }
                BendWallSW => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let mut pos = self.position(&coll) * scale;
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

                    let d = sdf_rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
                }
                BendWallSE => {
                    let thick = coll.get_f32_default("Thickness", 0.2) * scale;
                    let round = coll.get_f32_default("Rounding", 0.3) * scale;

                    let mut pos = self.position(&coll) * scale;
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

                    let d = sdf_rounded_box2d(p, size, thick, rounding);

                    return d.abs() - thick;
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

                    let mut d = length(p - self.position(&coll) * scale) - radius * scale + hole;
                    if hole > 0.0 {
                        d = d.abs() - hole;
                    }

                    return d;
                }
                Gate => {
                    let mut pos = self.position(&coll) * scale;
                    let height = coll.get_f32_default("Height", 0.8) * scale;
                    pos.y -= (height - 1.0 * scale) / 2.0;

                    let r = op_rep_lim(p - pos, 0.32 * scale, vec2f(-1., 0.), vec2f(1., 0.));

                    let d = sdf_box2d(r, Vec2f::zero(), 0.06 * scale, height / 2.0);

                    return d;
                }
            }
        }

        f32::INFINITY
    }

    /// The 3D distance from the node to a point.
    pub fn distance_3d(
        &self,
        _time: &TheTime,
        p: Vec3f,
        hit: &mut Option<&mut Hit>,
        params: &[f32],
    ) -> f32 {
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

        match self.role {
            Ground => {
                let uv = p.xz();
                let value = noise2d(&uv, vec2f(params[2], params[3]), params[4] as i32);
                if let Some(hit) = hit {
                    hit.mat.base_color = vec3f(value, value, value);
                    hit.value = value;
                    hit.eps = 0.15;
                }
                p.y - value * 0.05
            }
            Floor => {
                let height = params[2];

                let pos = vec2f(params[0], params[1]);
                let d = sdf_box2d(vec2f(p.x, p.z), pos, 0.5, 0.5);

                if let Some(hit) = hit {
                    hit.pattern_pos = vec2f(p.x, p.z);
                    hit.extrusion = GeoFXNodeExtrusion::Y;
                    hit.extrusion_length = height;
                    hit.interior_distance = d;
                    hit.hit_point = p - vec3f(pos.x, 0.0, pos.y);
                }

                d
            }
            LeftWall => {
                let thick = params[2];
                let len = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]);
                let d = sdf_box2d(
                    vec2f(p.z, p.y),
                    vec2f(pos.y, height / 2.0),
                    len,
                    height / 2.0,
                );

                if let Some(hit) = hit {
                    hit.pattern_pos = vec2f(p.z, p.y);
                    hit.extrusion = GeoFXNodeExtrusion::X;
                    hit.extrusion_length = thick;
                    hit.interior_distance = d;
                    hit.hit_point = p - vec3f(pos.x.floor() + thick.fract() / 2.0, 0.0, 0.0);
                }

                d
            }
            TopWall => {
                let thick = params[2];
                let len = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]);
                let d = sdf_box2d(
                    vec2f(p.x, p.y),
                    vec2f(pos.x, height / 2.0),
                    len,
                    height / 2.0,
                );

                if let Some(hit) = hit {
                    hit.pattern_pos = vec2f(p.x, p.y);
                    hit.extrusion = GeoFXNodeExtrusion::Z;
                    hit.extrusion_length = thick;
                    hit.interior_distance = d;
                    hit.hit_point =
                        p - vec3f(0.0, 0.0, pos.y.floor() + hit.extrusion_length.fract() / 2.0);
                }

                d
            }
            RightWall => {
                let thick = params[2];
                let len = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]);
                let d = sdf_box2d(
                    vec2f(p.z, p.y),
                    vec2f(pos.y, height / 2.0),
                    len,
                    height / 2.0,
                );

                if let Some(hit) = hit {
                    hit.pattern_pos = vec2f(p.z, p.y);
                    hit.extrusion = GeoFXNodeExtrusion::X;
                    hit.extrusion_length = thick;
                    hit.interior_distance = d;
                    hit.hit_point = p - vec3f(pos.x.floor() + 1.0 - thick.fract() / 2.0, 0.0, 0.0);
                }

                d
            }
            BottomWall => {
                let thick = params[2];
                let len = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]);
                let d = sdf_box2d(
                    vec2f(p.x, p.y),
                    vec2f(pos.x, height / 2.0),
                    len,
                    height / 2.0,
                );

                if let Some(hit) = hit {
                    hit.pattern_pos = vec2f(p.x, p.y);
                    hit.extrusion = GeoFXNodeExtrusion::Z;
                    hit.extrusion_length = thick;
                    hit.interior_distance = d;
                    hit.hit_point = p - vec3f(
                        0.0,
                        0.0,
                        pos.y.floor() + 1.0 - hit.extrusion_length.fract() / 2.0,
                    );
                }

                d
            }
            BendWallNW => {
                let thick = params[2];
                let round = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]) + 1.0;
                let rounding = (round, round, round, round);

                let pp = vec2f(p.x, p.z) - pos;

                let size = 1.0;
                let mut d = sdf_rounded_box2d(pp, size, thick, rounding);

                d = d.abs() - thick;

                if let Some(hit) = hit {
                    hit.interior_distance = d;
                }

                let d = op_extrusion_y(p, d, height);
                let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                max(-plane, d)
            }
            BendWallNE => {
                let thick = params[2];
                let round = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]) + vec2f(0.0, 1.0);
                let rounding = (round, round, round, round);

                let pp = vec2f(p.x, p.z) - pos;

                let size = 1.0;
                let mut d = sdf_rounded_box2d(pp, size, thick, rounding);

                d = d.abs() - thick;

                if let Some(hit) = hit {
                    hit.interior_distance = d;
                }

                let d = op_extrusion_y(p, d, height);
                let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                max(-plane, d)
            }
            BendWallSW => {
                let thick = params[2];
                let round = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]) + vec2f(1.0, 0.0);
                let rounding = (round, round, round, round);

                let pp = vec2f(p.x, p.z) - pos;

                let size = 1.0;
                let mut d = sdf_rounded_box2d(pp, size, thick, rounding);

                d = d.abs() - thick;

                if let Some(hit) = hit {
                    hit.interior_distance = d;
                }

                let d = op_extrusion_y(p, d, height);
                let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                max(-plane, d)
            }
            BendWallSE => {
                let thick = params[2];
                let round = params[3];
                let height = params[4];

                let pos = vec2f(params[0], params[1]);
                let rounding = (round, round, round, round);

                let pp = vec2f(p.x, p.z) - pos;

                let size = 1.0;
                let mut d = sdf_rounded_box2d(pp, size, thick, rounding);

                d = d.abs() - thick;

                if let Some(hit) = hit {
                    hit.interior_distance = d;
                }

                let d = op_extrusion_y(p, d, height);
                let plane = dot(p, vec3f(0.0, 1.0, 0.0));
                max(-plane, d)
            }
            Column => {
                let radius = params[2];
                let height = params[3];
                let hole = params[4];

                let pos = vec2f(params[0], params[1]);
                let mut d = length(vec2f(p.x, p.z) - pos) - radius + hole;
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

                d
            }
            Gate => {
                let pos = vec2f(params[0], params[1]);
                let align = params[2] as i32;
                let height = params[3];

                let r = if align == 0 {
                    op_rep_lim(vec2f(p.x, p.z) - pos, 0.32, vec2f(0., -1.), vec2f(0., 1.))
                } else {
                    op_rep_lim(vec2f(p.x, p.z) - pos, 0.32, vec2f(-1., 0.), vec2f(1., 0.))
                };
                let d = sdf_box2d(r, Vec2f::zero(), 0.06, 0.06);

                if let Some(hit) = hit {
                    hit.pattern_pos = vec2f(p.x, p.z);
                    hit.extrusion = GeoFXNodeExtrusion::Y;
                    hit.extrusion_length = height;
                    hit.interior_distance = d;
                    hit.hit_point = p - vec3f(pos.x.floor(), 0.0, pos.y.floor());
                }

                d
            }
        }
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

                    let center = self.position(&coll);
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
                    area.push(Vec2i::from(self.position(&coll)));
                }
            }
        }
        area
    }

    #[inline(always)]
    pub fn position(&self, coll: &TheCollection) -> Vec2f {
        let x = coll.get_f32_default("Pos X", 0.0);
        let y = coll.get_f32_default("Pos Y", 0.0);
        vec2f(x, y)
    }

    pub fn set_default_position(&mut self, p: Vec2i) {
        let mut pf = vec2f(p.x as f32, p.y as f32);
        match self.role {
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
            _ => {
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

    pub fn is_blocking(&self) -> bool {
        match self.role {
            Ground => false,
            Floor => {
                if let Some(coll) = self
                    .timeline
                    .get_collection_at(&TheTime::default(), str!("Geo"))
                {
                    let height = coll.get_f32_default("Height", 0.01);
                    height > 0.3
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    pub fn preview(
        &self,
        buffer: &mut TheRGBABuffer,
        material: Option<&MaterialFXObject>,
        palette: &ThePalette,
        tiles: &FxHashMap<Uuid, TheRGBATile>,
        coord: Vec2f,
    ) {
        fn mix_color(a: &[u8; 4], b: &[u8; 4], v: f32) -> [u8; 4] {
            [
                (((1.0 - v) * (a[0] as f32 / 255.0) + b[0] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[1] as f32 / 255.0) + b[1] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[2] as f32 / 255.0) + b[2] as f32 / 255.0 * v) * 255.0) as u8,
                (((1.0 - v) * (a[3] as f32 / 255.0) + b[3] as f32 / 255.0 * v) * 255.0) as u8,
            ]
        }

        let width = buffer.dim().width as usize;
        let height = buffer.dim().height;

        let time = TheTime::default();

        let mut mat_obj_params: Vec<Vec<f32>> = vec![];

        if let Some(material) = material {
            mat_obj_params = material.load_parameters(&time);
        }

        buffer
            .pixels_mut()
            .par_rchunks_exact_mut(width * 4)
            .enumerate()
            .for_each(|(j, line)| {
                for (i, pixel) in line.chunks_exact_mut(4).enumerate() {
                    let i = j * width + i;

                    let x = (i % width) as f32;
                    let y = (i / width) as f32;

                    let mut hit = Hit {
                        two_d: true,
                        ..Default::default()
                    };

                    let p = vec2f(x / width as f32, 1.0 - y / height as f32);
                    let p_coord = p + coord;
                    hit.uv = p;
                    hit.global_uv = p_coord;
                    hit.pattern_pos = p_coord;
                    hit.hit_point = vec3f(p.x + coord.x, 0.0, p.y + coord.y);
                    hit.normal = vec3f(0.0, 1.0, 0.0);
                    let d = self.distance(&time, p_coord, 1.0, &mut Some(&mut hit));
                    hit.distance = d;

                    if let Some(material) = material {
                        material.follow_geo_trail(&TheTime::default(), &mut hit, &mat_obj_params);
                        if self.role != GeoFXNodeRole::Ground {
                            if hit.interior_distance <= 0.01 {
                                hit.value = 0.0;
                            } else {
                                hit.value = 1.0;
                            }
                        }
                        material.compute(&mut hit, palette, tiles, &mat_obj_params);
                    };

                    let t = smoothstep(-0.04, 0.0, d);

                    if self.role == GeoFXNodeRole::Ground {
                        pixel.copy_from_slice(
                            &TheColor::from_vec3f(hit.mat.base_color).to_u8_array(),
                        );
                    } else {
                        let color = if material.is_some() {
                            TheColor::from_vec3f(hit.mat.base_color).to_u8_array()
                        } else {
                            [209, 209, 209, 255]
                        };
                        pixel.copy_from_slice(&mix_color(&color, &[81, 81, 81, 255], t));
                    }
                }
            });
    }
}
