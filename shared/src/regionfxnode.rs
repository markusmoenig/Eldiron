use crate::prelude::*;
use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub enum RegionFXNodeRole {
    TiltedIsoCamera,
    TopDownIsoCamera,
    Renderer,
    Saturation,
}

use RegionFXNodeRole::*;

use crate::Ray;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct RegionFXNode {
    pub id: Uuid,
    pub role: RegionFXNodeRole,
    pub timeline: TheTimeline,

    pub position: Vec2<i32>,

    pub supports_preview: bool,
    pub preview_is_open: bool,

    pub preview: TheRGBABuffer,
}

impl RegionFXNode {
    pub fn new(role: RegionFXNodeRole) -> Self {
        let mut coll = TheCollection::named(str!("Props"));
        let supports_preview = false;
        let preview_is_open = false;

        match role {
            TiltedIsoCamera => {
                coll.set("Height", TheValue::FloatRange(4.0, 1.0..=7.0));
                coll.set("Tilt Angle", TheValue::FloatRange(0.35, 0.0..=1.0));
                coll.set(
                    "Alignment",
                    TheValue::TextList(0, vec![str!("Right"), str!("Left")]),
                );
            }
            TopDownIsoCamera => {
                coll.set("Height", TheValue::FloatRange(4.0, 1.0..=7.0));
                coll.set(
                    "Alignment",
                    TheValue::TextList(0, vec![str!("Right"), str!("Left")]),
                );
            }
            Renderer => {
                coll.set(
                    "2D Renderer",
                    TheValue::TextList(0, vec![str!("Normal"), str!("Lambertian")]),
                );
            }
            Saturation => {
                coll.set("Saturation", TheValue::FloatRange(1.0, 0.0..=2.0));
            }
        }

        let timeline = TheTimeline::collection(coll);

        Self {
            id: Uuid::new_v4(),
            role,
            timeline,
            position: Vec2::new(10, 5),
            supports_preview,
            preview_is_open,
            preview: TheRGBABuffer::empty(),
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            TiltedIsoCamera => str!("Tilted Iso Camera"),
            TopDownIsoCamera => str!("Top Down Iso Camera"),
            Renderer => str!("Renderer"),
            Saturation => str!("Saturation"),
        }
    }

    pub fn nodes() -> Vec<Self> {
        vec![
            Self::new(RegionFXNodeRole::TiltedIsoCamera),
            Self::new(RegionFXNodeRole::TopDownIsoCamera),
            Self::new(RegionFXNodeRole::Renderer),
            Self::new(RegionFXNodeRole::Saturation),
        ]
    }

    /// Gives the node a chance to update its parameters in case things changed.
    pub fn update_parameters(&mut self) {
        // match self.role {
        //     Renderer => {
        //         self.set(
        //             "2D Renderer",
        //             TheValue::TextList(1, vec![str!("Plain"), str!("Lambertian")]),
        //         );
        //     }
        //     _ => {}
        // }
    }

    /// Loads the parameters of the nodes into memory for faster access.
    pub fn load_parameters(&self, _time: &TheTime) -> Vec<f32> {
        let mut params = vec![];

        let coll = self.collection();

        match self.role {
            RegionFXNodeRole::TiltedIsoCamera => {
                params.push(coll.get_f32_default("Height", 4.0));
                params.push(coll.get_f32_default("Tilt Angle", 0.35));
                params.push(coll.get_i32_default("Alignment", 0) as f32);
            }
            RegionFXNodeRole::TopDownIsoCamera => {
                params.push(coll.get_f32_default("Height", 4.0));
                params.push(coll.get_i32_default("Alignment", 0) as f32);
            }
            RegionFXNodeRole::Saturation => {
                params.push(coll.get_f32_default("Saturation", 1.0));
            }
            _ => {}
        }

        params
    }

    pub fn is_camera(&self) -> bool {
        matches!(self.role, TiltedIsoCamera | TopDownIsoCamera)
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            TiltedIsoCamera | TopDownIsoCamera => {
                vec![]
            }
            Renderer => {
                vec![TheNodeTerminal {
                    name: str!("cam"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            Saturation => {
                vec![TheNodeTerminal {
                    name: str!("in"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            TiltedIsoCamera | TopDownIsoCamera => {
                vec![TheNodeTerminal {
                    name: str!("cam"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            }
            Renderer => {
                vec![
                    TheNodeTerminal {
                        name: str!("2D FX"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                    TheNodeTerminal {
                        name: str!("3D FX"),
                        color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                    },
                ]
            }
            Saturation => {
                vec![TheNodeTerminal {
                    name: str!("out"),
                    color: TheColor::new(0.5, 0.5, 0.5, 1.0),
                }]
            } //_ => vec![],
        }
    }

    /// Convert a world position into a pixel offset in the canvas.
    pub fn cam_world_to_canvas(&self, region: &Region, world_pos: Vec3<f32>) -> Vec2<i32> {
        match self.role {
            TopDownIsoCamera => {
                let tile_size = region.tile_size;
                let tile_size_half = tile_size as f32;

                let sx = tile_size * region.width;

                let x = sx + ((world_pos.x - world_pos.z) * tile_size_half) as i32;
                let y = ((world_pos.x + world_pos.z) * (tile_size_half / 2.0)) as i32;
                Vec2::new(x, y)
            }
            _ => Vec2::new(
                (world_pos.x * region.tile_size as f32) as i32,
                (world_pos.z * region.tile_size as f32) as i32,
            ),
        }
    }

    /// Convert a canvas pixel position into a world position.
    pub fn cam_canvas_to_world(&self, region: &Region, mut canvas_pos: Vec2<i32>) -> Vec3<f32> {
        match self.role {
            TopDownIsoCamera => {
                canvas_pos.x -= region.width * region.tile_size;

                let x = canvas_pos.x as f32;
                let y = canvas_pos.y as f32;

                let tile_width = region.tile_size as f32;
                let tile_height_half = region.tile_size as f32 / 2.0;

                let map_x = (x / tile_width + y / tile_height_half) / 2.0;
                let map_y = (y / tile_height_half - (x / tile_width)) / 2.0;

                Vec3::new(map_x, 0.0, map_y)
            }
            _ => Vec3::new(
                canvas_pos.x as f32 / region.tile_size as f32,
                0.0,
                canvas_pos.y as f32 / region.tile_size as f32,
            ),
        }
    }

    /*
    /// Render the prerendered tiles into the game canvas.
    pub fn cam_render_canvas(&self, region: &Region, canvas: &mut GameCanvas) {
        match self.role {
            TiltedIsoCamera => {
                let tile_size = region.tile_size;
                let width = tile_size * region.width;
                let height = tile_size * region.height;

                canvas.resize(width, height);

                // for (key, tile) in &region.prerendered.tiles {
                //     let x = key.x * region.tile_size;
                //     let y = key.y * region.tile_size;
                //     canvas.copy_into(x, y, tile);
                // }
            }
            TopDownIsoCamera => {
                let tile_size = region.tile_size;
                let tile_size_half = tile_size;

                let width = tile_size * region.width * 2;
                let height = tile_size * region.height;

                canvas.resize(width, height);

                let sx = tile_size * region.width;

                let mut keys: Vec<Vec2i> = region.prerendered.tiles.keys().cloned().collect();
                keys.sort_by(|a, b| {
                    let sum_a = a.x + a.y;
                    let sum_b = b.x + b.y;
                    if sum_a == sum_b {
                        a.x.cmp(&b.x)
                    } else {
                        sum_a.cmp(&sum_b)
                    }
                });

                for key in keys {
                    // if let Some(tile) = &region.prerendered.tiles.get(&key) {
                    //     let x = sx + (key.x - key.y) * tile_size_half;
                    //     let y = (key.x + key.y) * (tile_size_half / 2);

                    //     canvas.copy_into(x, y, tile);
                    // }
                }
            }
            _ => {}
        }
    }*/

    /// Returns the size of the region world in pixels
    pub fn cam_region_size(&self, region: &Region) -> Vec2<i32> {
        let mut size = Vec2::zero();
        match self.role {
            TiltedIsoCamera => {
                let tile_size = region.tile_size;
                size.x = tile_size * region.width;
                size.y = tile_size * region.height;
            }
            TopDownIsoCamera => {
                let tile_size = region.tile_size;

                size.x = tile_size * region.width * 2;
                size.y = tile_size * region.height;
            }
            _ => {}
        }
        size
    }

    /// Create a cameray ray
    pub fn cam_create_ray(
        &self,
        uv: Vec2<f32>,
        position: Vec3<f32>,
        size: Vec2<f32>,
        offset: Vec2<f32>,
        params: &[f32],
    ) -> Ray {
        match self.role {
            TiltedIsoCamera => {
                let height = params[0];
                let angle = params[1];
                let alignment = params[2] as i32;

                let mut ro = Vec3::new(position.x, height, position.z + 1.0);
                let mut rd = Vec3::new(position.x, 0.0, position.z);

                if alignment == 0 {
                    ro.x += height / 2.0;
                    rd.x += height / 2.0;
                } else {
                    ro.x -= height / 4.0;
                    rd.x -= height / 4.0;
                }

                let ratio = size.x / size.y;
                let pixel_size = Vec2::new(1.0 / size.x, 1.0 / size.y);

                let cam_origin = ro;
                let cam_look_at = rd;

                let fov: f32 = 124.0;
                let half_width = (fov.to_radians() * 0.5).tan();
                let half_height = half_width / ratio;

                let up_vector = Vec3::new(0.0, 1.0, 0.0);

                let w = (cam_origin - cam_look_at).normalized();
                let u = up_vector.cross(w);
                let v = w.cross(u);

                let horizontal = u * half_width * 2.0;
                let vertical = v * half_height * 2.0;

                let mut out_origin = cam_origin;
                out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
                out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);
                out_origin.y = cam_origin.y;

                Ray::new(
                    out_origin,
                    Vec3::new(if alignment == 0 { -angle } else { angle }, -1.0, -angle)
                        .normalized(),
                )
            }
            TopDownIsoCamera => {
                let height = params[0];

                let ro = Vec3::new(position.x + height, height - 0.5, position.z + height);
                let rd = Vec3::new(position.x, 0.0, position.z);

                let ratio = size.x / size.y;
                let pixel_size = Vec2::new(1.0 / size.x, 1.0 / size.y);

                let cam_origin = ro;
                let cam_look_at = rd;

                let fov: f32 = 47.0;
                let half_width = (fov.to_radians() * 0.5).tan();
                let half_height = half_width / ratio;

                let up_vector = Vec3::new(0.0, 1.0, 0.0);

                let w = (cam_origin - cam_look_at).normalized();
                let u = up_vector.cross(w);
                let v = w.cross(u);

                let horizontal = u * half_width * 2.0;
                let vertical = v * half_height * 2.0;

                let mut out_origin = cam_origin;
                out_origin += horizontal * (pixel_size.x * offset.x + uv.x - 0.5);
                out_origin += vertical * (pixel_size.y * offset.y + uv.y - 0.5);

                Ray::new(out_origin, (-w).normalized())
            }
            _ => Ray::new(Vec3::zero(), Vec3::zero()),
        }
    }

    /// Apply a region effect.
    pub fn fx(
        &self,
        _region: &Region,
        _palette: &ThePalette,
        _canvas_pos: Vec2<i32>,
        color: &mut Vec3<f32>,
        params: &[f32],
    ) -> Option<u8> {
        match self.role {
            RegionFXNodeRole::Saturation => {
                let mut hsl = TheColor::from_vec3(*color).as_hsl();
                hsl.y *= params[0];
                *color = TheColor::from_hsl(hsl.x * 360.0, hsl.y.clamp(0.0, 1.0), hsl.z).to_vec3();
                Some(0)
            }
            _ => None,
        }
    }

    /// Creates a new node from a name.
    pub fn new_from_name(name: String) -> Self {
        let nodes = RegionFXNode::nodes();
        for n in nodes {
            if n.name() == name {
                return n;
            }
        }
        RegionFXNode::new(Renderer)
    }

    pub fn collection(&self) -> TheCollection {
        if let Some(coll) = self
            .timeline
            .get_collection_at(&TheTime::default(), str!("Props"))
        {
            return coll;
        }

        TheCollection::default()
    }

    pub fn get(&self, key: &str) -> Option<TheValue> {
        self.timeline.get(
            "Props".to_string(),
            key.to_string(),
            &TheTime::default(),
            TheInterpolation::Linear,
        )
    }

    /// Clears the collection.
    pub fn clear(&mut self) {
        self.timeline.clear_collection(&TheTime::default(), "Props");
    }

    /// Sets a value in the collection.
    pub fn set(&mut self, key: &str, value: TheValue) {
        self.timeline.set(&TheTime::default(), key, "Props", value);
    }
}
