use crate::{
    Assets, BBox, BLACK, CompiledLight, LightType, Linedef, Map, Material, MaterialModifier,
    MaterialRole, Pixel, Rasterizer, Ray, Sector, ShapeContext, ShapeFXGraph, Terrain,
    TerrainChunk, Texture, ValueContainer, pixel_to_vec4, vec4_to_pixel,
};
use noiselib::prelude::*;
use std::str::FromStr;
use theframework::prelude::*;
use uuid::Uuid;
use vek::Vec4;

#[inline(always)]
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

const BAYER_4X4: [[f32; 4]; 4] = [
    [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
    [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
    [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
    [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeFXModifierPass {
    Height,
    Colorize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShapeFXParam {
    /// Id, Name, Status, Value, Range
    Float(String, String, String, f32, std::ops::RangeInclusive<f32>),
    /// Id, Name, Status, Value, Range
    Int(String, String, String, i32, std::ops::RangeInclusive<i32>),
    /// Id, Name, Status, Value
    PaletteIndex(String, String, String, i32),
    /// Id, Name, Status, Options, Value
    Selector(String, String, String, Vec<String>, i32),
    /// Id, Name, Status, Value
    Color(String, String, String, TheColor),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShapeFXRole {
    // Material Group
    // These nodes get attached to geometry and produce pixel output
    MaterialGeometry,
    Gradient,
    Color,
    Outline,
    NoiseOverlay,
    Glow,
    Wood,
    Stone,
    // Sector and Linedef Group
    // These nodes get attached to geometry and control mesh creation
    // or produce rendering fx like lights, particles etc.
    LinedefGeometry,
    SectorGeometry,
    Flatten,
    Colorize,
    // Render Group
    Render, // Main Render Node
    Fog,
    Sky,
    // FX Group
    Material,
    PointLight,
    // Shape Group
    Shape,
    Circle,
    Line,
    Box,
    // UI Group
    Widget,
}

use ShapeFXRole::*;

impl FromStr for ShapeFXRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Material Geometry" => Ok(ShapeFXRole::MaterialGeometry),
            "Gradient" => Ok(ShapeFXRole::Gradient),
            "Color" => Ok(ShapeFXRole::Color),
            "Outline" => Ok(ShapeFXRole::Outline),
            "Noise Overlay" => Ok(ShapeFXRole::NoiseOverlay),
            "Glow" => Ok(ShapeFXRole::Glow),
            "Wood" => Ok(ShapeFXRole::Wood),
            "Stone" => Ok(ShapeFXRole::Stone),
            "Sector Geometry" => Ok(ShapeFXRole::SectorGeometry),
            "Flatten" => Ok(ShapeFXRole::Flatten),
            "Colorize" => Ok(ShapeFXRole::Colorize),
            "Render" => Ok(ShapeFXRole::Render),
            "Fog" => Ok(ShapeFXRole::Fog),
            "Sky" => Ok(ShapeFXRole::Sky),
            "Material" => Ok(ShapeFXRole::Material),
            "Point Light" => Ok(ShapeFXRole::PointLight),
            "Shape" => Ok(ShapeFXRole::Shape),
            "Circle" => Ok(ShapeFXRole::Circle),
            "Line" => Ok(ShapeFXRole::Line),
            "Box" => Ok(ShapeFXRole::Box),
            "Widget" => Ok(ShapeFXRole::Widget),
            _ => Err(()),
        }
    }
}

impl ShapeFXRole {
    pub fn iterator() -> impl Iterator<Item = ShapeFXRole> {
        [ShapeFXRole::MaterialGeometry, ShapeFXRole::Gradient]
            .iter()
            .copied()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeFX {
    pub id: Uuid,
    pub role: ShapeFXRole,
    pub values: ValueContainer,

    pub position: Vec2<i32>,

    // Used for precomputing values from the Container
    #[serde(skip)]
    precomputed: Vec<Vec4<f32>>,
}

impl ShapeFX {
    pub fn new(role: ShapeFXRole) -> Self {
        let values = ValueContainer::default();

        Self {
            id: Uuid::new_v4(),
            role,
            values,
            position: Vec2::new(20, 20),
            precomputed: vec![],
        }
    }

    pub fn supports_modifier_pass(&self, pass: ShapeFXModifierPass) -> bool {
        match self.role {
            Flatten => true,
            Colorize => pass == ShapeFXModifierPass::Colorize,
            _ => false,
        }
    }

    pub fn name(&self) -> String {
        match self.role {
            MaterialGeometry => "Geometry".into(),
            Gradient => "Gradient".into(),
            Color => "Color".into(),
            Outline => "Outline".into(),
            NoiseOverlay => "Noise Overlay".into(),
            Glow => "Glow".into(),
            Wood => "Wood".into(),
            Stone => "Stone".into(),
            LinedefGeometry => "Linedef Geometry".into(),
            SectorGeometry => "Sector Geometry".into(),
            Flatten => "Terrain: Flatten".into(),
            Colorize => "Terrain: Colorize".into(),
            Render => "Render".into(),
            Fog => "Fog".into(),
            Sky => "Sky".into(),
            Material => "Material".into(),
            PointLight => "Point Light".into(),
            Shape => "Shape".into(),
            Circle => "Circle".into(),
            Line => "Line".into(),
            Box => "Box".into(),
            Widget => "Widget".into(),
        }
    }

    pub fn inputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            MaterialGeometry | SectorGeometry | LinedefGeometry | Shape | Widget => {
                vec![]
            }
            Render => {
                vec![
                    TheNodeTerminal {
                        name: "camera".into(),
                        category_name: "Render".into(),
                    },
                    TheNodeTerminal {
                        name: "fx".into(),
                        category_name: "Render".into(),
                    },
                ]
            }
            Fog | Sky => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "Render".into(),
                }]
            }
            Flatten | Colorize => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "Modifier".into(),
                }]
            }
            Material | PointLight => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "FX".into(),
                }]
            }
            Circle | Line | Box => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "Shape".into(),
                }]
            }
            _ => {
                vec![TheNodeTerminal {
                    name: "in".into(),
                    category_name: "ShapeFX".into(),
                }]
            }
        }
    }

    pub fn outputs(&self) -> Vec<TheNodeTerminal> {
        match self.role {
            MaterialGeometry => {
                vec![
                    TheNodeTerminal {
                        name: "inside".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "outside".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            LinedefGeometry => {
                vec![
                    TheNodeTerminal {
                        name: "modifier".into(),
                        category_name: "Modifier".into(),
                    },
                    TheNodeTerminal {
                        name: "row1".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "row2".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "row3".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "row4".into(),
                        category_name: "FX".into(),
                    },
                ]
            }
            SectorGeometry => {
                vec![
                    TheNodeTerminal {
                        name: "ground".into(),
                        category_name: "Modifier".into(),
                    },
                    TheNodeTerminal {
                        name: "ceiling".into(),
                        category_name: "Modifier".into(),
                    },
                    TheNodeTerminal {
                        name: "ground".into(),
                        category_name: "FX".into(),
                    },
                    TheNodeTerminal {
                        name: "ceiling".into(),
                        category_name: "FX".into(),
                    },
                ]
            }
            Render => {
                vec![
                    TheNodeTerminal {
                        name: "hit".into(),
                        category_name: "Render".into(),
                    },
                    TheNodeTerminal {
                        name: "miss".into(),
                        category_name: "Render".into(),
                    },
                ]
            }
            Fog | Sky => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "Render".into(),
                }]
            }
            Flatten | Colorize => {
                vec![
                    TheNodeTerminal {
                        name: "out".into(),
                        category_name: "Modifier".into(),
                    },
                    TheNodeTerminal {
                        name: "color".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            Material | PointLight => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "FX".into(),
                }]
            }
            NoiseOverlay => {
                vec![
                    TheNodeTerminal {
                        name: "out".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "color".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            Stone => {
                vec![
                    TheNodeTerminal {
                        name: "out".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "stone".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "mortar".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            Wood => {
                vec![
                    TheNodeTerminal {
                        name: "out".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "light".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "dark".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            Shape => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "Shape".into(),
                }]
            }
            Circle | Line | Box => {
                vec![
                    TheNodeTerminal {
                        name: "out".into(),
                        category_name: "Shape".into(),
                    },
                    TheNodeTerminal {
                        name: "color".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            Widget => {
                vec![
                    TheNodeTerminal {
                        name: "normal".into(),
                        category_name: "ShapeFX".into(),
                    },
                    TheNodeTerminal {
                        name: "selected".into(),
                        category_name: "ShapeFX".into(),
                    },
                ]
            }
            _ => {
                vec![TheNodeTerminal {
                    name: "out".into(),
                    category_name: "ShapeFX".into(),
                }]
            }
        }
    }

    /// Modify the given heightmap with the region nodes of the given sector
    #[allow(clippy::too_many_arguments)]
    pub fn sector_modify_heightmap(
        &self,
        sector: &Sector,
        map: &Map,
        terrain: &Terrain,
        bbox: &BBox,
        chunk: &TerrainChunk,
        heights: &mut FxHashMap<(i32, i32), f32>,
        graph_node: (&ShapeFXGraph, usize),
        assets: &Assets,
        texture: &mut Texture,
        pass: ShapeFXModifierPass,
    ) {
        #[allow(clippy::single_match)]
        match self.role {
            Flatten | Colorize => {
                let is_colorize = matches!(self.role, Colorize);
                let shapefx_nodes = graph_node.0.collect_nodes_from(graph_node.1, 1);

                let bevel = self.values.get_float_default("bevel", 0.5);
                let fade_distance = self.values.get_float_default("fade_distance", 0.5);
                let floor_height = sector.properties.get_float_default("floor_height", 0.0);
                let noise_strength = self.values.get_float_default("fade_noise", 0.0);
                let uv_scale = self.values.get_float_default("uv_scale", 1.0);
                let mut expanded_bbox = *bbox;
                expanded_bbox.expand(Vec2::broadcast(bevel));

                let chunk_bbox = chunk.bounds();

                let mut bounds = sector.bounding_box(map);
                bounds.expand(Vec2::broadcast(bevel));

                let min_x = bounds.min.x.floor() as i32;
                let max_x = bounds.max.x.ceil() as i32;
                let min_y = bounds.min.y.floor() as i32;
                let max_y = bounds.max.y.ceil() as i32;

                for y in min_y..=max_y {
                    for x in min_x..=max_x {
                        let p = Vec2::new(x as f32, y as f32);

                        if !expanded_bbox.contains(p) {
                            continue;
                        }

                        let Some(sd) = sector.signed_distance(map, p) else {
                            continue;
                        };

                        if sd < bevel * 4.0 {
                            let world = Vec2::new(x, y);
                            let local = chunk.world_to_local(world);

                            if !is_colorize && pass == ShapeFXModifierPass::Height {
                                // We modify heights only in Flatten mode
                                let s = Self::smoothstep(0.0, bevel, bevel - sd);
                                let original =
                                    terrain.get_height_unprocessed(x, y).unwrap_or(floor_height);
                                let new_height = original * (1.0 - s) + floor_height * s;
                                if chunk_bbox.contains(Vec2::new(world.x as f32, world.y as f32)) {
                                    heights.insert((local.x, local.y), new_height);
                                } else {
                                    continue;
                                }
                            }

                            if pass == ShapeFXModifierPass::Height {
                                continue;
                            }

                            let pixels_per_tile = texture.width as i32 / terrain.chunk_size;

                            for dy in 0..pixels_per_tile {
                                if local.x < 0 || local.y < 0 {
                                    continue;
                                }
                                let max_x = texture.width as i32;
                                let max_y = texture.height as i32;
                                let pixel_base_x = local.x * pixels_per_tile;
                                let pixel_base_y = local.y * pixels_per_tile;

                                if pixel_base_x < 0
                                    || pixel_base_y < 0
                                    || pixel_base_x + pixels_per_tile > max_x
                                    || pixel_base_y + pixels_per_tile > max_y
                                {
                                    continue;
                                }
                                for dx in 0..pixels_per_tile {
                                    let pixel_local_x = local.x * pixels_per_tile + dx;
                                    let pixel_local_y = local.y * pixels_per_tile + dy;

                                    let uv_in_tile = Vec2::new(
                                        (dx as f32 + 0.5) / pixels_per_tile as f32,
                                        (dy as f32 + 0.5) / pixels_per_tile as f32,
                                    );

                                    let world_pos = Vec2::new(
                                        (x as f32 + uv_in_tile.x) * terrain.scale.x,
                                        (y as f32 + uv_in_tile.y) * terrain.scale.y,
                                    );

                                    // let world_pos = Vec2::new(
                                    //     (x as f32 + dx as f32 / pixels_per_tile as f32)
                                    //         * terrain.scale.x,
                                    //     (y as f32 + dy as f32 / pixels_per_tile as f32)
                                    //         * terrain.scale.y,
                                    // );

                                    let Some(mut sd_pixel) = sector.signed_distance(map, world_pos)
                                    else {
                                        continue;
                                    };

                                    if noise_strength > 0.0 {
                                        let noise =
                                            self.noise2d(&world_pos, Vec2::broadcast(1.0), 2);
                                        sd_pixel += noise * noise_strength;
                                    }

                                    let uv = world_pos / uv_scale;
                                    //let uv = pixel_pos / (texture.width as f32); // Or based on world space
                                    let px = terrain.scale.x.max(terrain.scale.y);

                                    let ctx = ShapeContext {
                                        point_world: world_pos,
                                        point: Vec2::new(
                                            pixel_local_x as f32,
                                            pixel_local_y as f32,
                                        ),
                                        uv,
                                        distance_world: sd_pixel,
                                        distance: sd_pixel / px,
                                        shape_id: sector.id,
                                        px,
                                        anti_aliasing: 1.0,
                                        t: None,
                                        line_dir: None,
                                        override_color: None,
                                    };

                                    let mut pixel_color: Option<Vec4<f32>> = None;
                                    for node in &shapefx_nodes {
                                        pixel_color = graph_node.0.nodes[*node as usize]
                                            .evaluate_pixel(
                                                &ctx,
                                                pixel_color,
                                                assets,
                                                (graph_node.0, *node as usize),
                                            )
                                            .or(pixel_color);
                                    }

                                    // Combines Colorize-based fades with the boundary fade
                                    let mut total_fade = 1.0;

                                    if is_colorize {
                                        let min_h =
                                            self.values.get_float_default("min_height", 0.0);
                                        let max_h =
                                            self.values.get_float_default("max_height", 10.0);
                                        let min_s =
                                            self.values.get_float_default("min_steepness", 0.0);
                                        let max_s =
                                            self.values.get_float_default("max_steepness", 1.0);

                                        fn fade_outside_range(
                                            value: f32,
                                            min: f32,
                                            max: f32,
                                            fade: f32,
                                        ) -> f32 {
                                            if value < min {
                                                let t = ((min - value) / fade).clamp(0.0, 1.0);
                                                1.0 - t * t * (3.0 - 2.0 * t) // smoothstep
                                            } else if value > max {
                                                let t = ((value - max) / fade).clamp(0.0, 1.0);
                                                1.0 - t * t * (3.0 - 2.0 * t)
                                            } else {
                                                1.0
                                            }
                                        }

                                        // Steepness
                                        if min_s > 0.0 || max_s < 1.0 {
                                            let steepness = terrain.compute_steepness(world_pos);
                                            total_fade *= fade_outside_range(
                                                steepness,
                                                min_s,
                                                max_s,
                                                fade_distance,
                                            );
                                        }

                                        // Height
                                        if min_h != 0.0 || max_h != 10.0 {
                                            let height = terrain
                                                .sample_height_bilinear(world_pos.x, world_pos.y);
                                            total_fade *= fade_outside_range(
                                                height,
                                                min_h,
                                                max_h,
                                                fade_distance,
                                            );
                                        }
                                    }

                                    if let Some(mut color) = pixel_color {
                                        let mut fade = if fade_distance == 0.0 {
                                            if sd_pixel <= 0.0 { 1.0 } else { 0.0 }
                                        } else {
                                            let fd = fade_distance.abs();
                                            if fade_distance > 0.0 {
                                                // ----- fade OUTSIDE the border -----
                                                if sd_pixel <= 0.0 {
                                                    1.0
                                                } else if sd_pixel <= fd {
                                                    let t = sd_pixel / fd;
                                                    1.0 - t * t * (3.0 - 2.0 * t) // smoothstep
                                                } else {
                                                    0.0
                                                }
                                            } else {
                                                // ----- fade INSIDE the border -----
                                                if sd_pixel <= -fd {
                                                    1.0
                                                } else if sd_pixel <= 0.0 {
                                                    let t = (sd_pixel + fd) / fd;
                                                    1.0 - t * t * (3.0 - 2.0 * t)
                                                } else {
                                                    0.0
                                                }
                                            }
                                        };

                                        fade *= total_fade;

                                        if color.w < 0.99 {
                                            color *= fade;
                                        } else {
                                            color *= fade;
                                            color.w = fade;
                                        }
                                        let existing = texture
                                            .get_pixel(pixel_local_x as u32, pixel_local_y as u32);
                                        let existing_color = pixel_to_vec4(&existing);
                                        let blended = existing_color * (1.0 - fade) + color;

                                        let mut pixel = vec4_to_pixel(&blended);
                                        pixel[3] = 255;
                                        texture.set_pixel(
                                            pixel_local_x as u32,
                                            pixel_local_y as u32,
                                            pixel,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Modify the given heightmap with the region nodes of the given sector
    #[allow(clippy::too_many_arguments)]
    pub fn linedef_modify_heightmap(
        &self,
        linedefs: &Vec<Linedef>,
        map: &Map,
        terrain: &Terrain,
        bbox: &BBox,
        chunk: &TerrainChunk,
        heights: &mut FxHashMap<(i32, i32), f32>,
        graph_node: (&ShapeFXGraph, usize),
        assets: &Assets,
        texture: &mut Texture,
        pass: ShapeFXModifierPass,
    ) {
        let is_flatten = matches!(self.role, ShapeFXRole::Flatten);
        let is_colorize = matches!(self.role, ShapeFXRole::Colorize);
        if !is_flatten && !is_colorize {
            return;
        }

        #[derive(Debug, Clone)]
        struct SegmentInfo {
            start: Vec2<f32>,
            end: Vec2<f32>,
            dir: Vec2<f32>,
            height_start: f32,
            height_end: f32,
            shape_id: u32,
        }

        fn point_to_line_segment(p: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> (f32, f32) {
            let ab = b - a;
            let ap = p - a;
            let t = ap.dot(ab) / ab.dot(ab);
            let t_clamped = t.clamp(0.0, 1.0);
            let closest = a + ab * t_clamped;
            let distance = (p - closest).magnitude();
            (distance, t_clamped)
        }

        fn closest_segment(
            p: Vec2<f32>,
            segments: &[SegmentInfo],
        ) -> Option<(f32, f32, &SegmentInfo)> {
            let mut best: Option<(f32, f32, &SegmentInfo)> = None;

            for seg in segments {
                let (dist, t) = point_to_line_segment(p, seg.start, seg.end);
                if best
                    .map(|(best_dist, _, _)| dist < best_dist)
                    .unwrap_or(true)
                {
                    best = Some((dist, t, seg));
                }
            }

            best
        }

        let shapefx_nodes = graph_node.0.collect_nodes_from(graph_node.1, 1);
        let bevel = self.values.get_float_default("bevel", 0.5);
        let path_width = self.values.get_float_default("path_width", 2.0);
        let fade_distance = self.values.get_float_default("fade_distance", 0.5);
        let noise_strength = self.values.get_float_default("fade_noise", 0.0);
        let uv_scale = self.values.get_float_default("uv_scale", 1.0);

        let half_path = path_width * 0.5;
        let fade_start = half_path;
        let fade_end = half_path + fade_distance;

        let chunk_bbox = chunk.bounds();

        let mut written_blends = FxHashMap::<(i32, i32), f32>::default();

        // 1. Collect all segments
        let mut segments = Vec::new();
        for linedef in linedefs {
            let Some(start) = map.vertices.iter().find(|v| v.id == linedef.start_vertex) else {
                continue;
            };
            let Some(end) = map.vertices.iter().find(|v| v.id == linedef.end_vertex) else {
                continue;
            };

            let start_pos = start.as_vec2();
            let end_pos = end.as_vec2();
            let height_start = start.properties.get_float_default("height", 0.0);
            let height_end = end.properties.get_float_default("height", 0.0);
            let dir = (end_pos - start_pos).normalized();

            segments.push(SegmentInfo {
                start: start_pos,
                end: end_pos,
                dir,
                height_start,
                height_end,
                shape_id: linedef.id,
            });
        }

        // 2. Iterate all tiles within expanded bbox
        let min = bbox.min - bevel * terrain.scale;
        let max = bbox.max + bevel * terrain.scale;
        let tile_min = (min / terrain.scale).floor().as_::<i32>();
        let tile_max = (max / terrain.scale).ceil().as_::<i32>();

        let pixels_per_tile = texture.width as i32 / terrain.chunk_size;

        for ty in tile_min.y..tile_max.y {
            for tx in tile_min.x..tile_max.x {
                let tile_center = Vec2::new(
                    (tx as f32 + 0.5) * terrain.scale.x,
                    (ty as f32 + 0.5) * terrain.scale.y,
                );

                let Some((dist, t, seg)) = closest_segment(tile_center, &segments) else {
                    continue;
                };

                if dist > bevel {
                    continue;
                }

                let height = seg.height_start * (1.0 - t) + seg.height_end * t;
                let blend = ShapeFX::smoothstep(0.0, bevel, bevel - dist);

                let world_tile = Vec2::new(tx, ty);
                let local = chunk.world_to_local(world_tile);
                let key = (local.x, local.y);

                if is_flatten
                    && pass == ShapeFXModifierPass::Height
                    && chunk_bbox.contains(Vec2::new(world_tile.x as f32, world_tile.y as f32))
                {
                    let original = terrain
                        .get_height_unprocessed(world_tile.x, world_tile.y)
                        .unwrap_or(height);
                    let new_height = original * (1.0 - blend) + height * blend;
                    heights.insert(key, new_height);
                }

                if pass == ShapeFXModifierPass::Height {
                    continue;
                }

                for dy in 0..pixels_per_tile {
                    if local.x < 0 || local.y < 0 {
                        continue;
                    }
                    let max_x = texture.width as i32;
                    let max_y = texture.height as i32;
                    let pixel_base_x = local.x * pixels_per_tile;
                    let pixel_base_y = local.y * pixels_per_tile;

                    if pixel_base_x < 0
                        || pixel_base_y < 0
                        || pixel_base_x + pixels_per_tile > max_x
                        || pixel_base_y + pixels_per_tile > max_y
                    {
                        continue;
                    }
                    for dx in 0..pixels_per_tile {
                        let pixel_local_x = local.x * pixels_per_tile + dx;
                        let pixel_local_y = local.y * pixels_per_tile + dy;

                        let uv_in_tile = Vec2::new(
                            (dx as f32 + 0.5) / pixels_per_tile as f32,
                            (dy as f32 + 0.5) / pixels_per_tile as f32,
                        );

                        let world_pos = Vec2::new(
                            (tx as f32 + uv_in_tile.x) * terrain.scale.x,
                            (ty as f32 + uv_in_tile.y) * terrain.scale.y,
                        );

                        let Some((perpendicular, t_px, seg)) =
                            closest_segment(world_pos, &segments)
                        else {
                            continue;
                        };

                        let mut perturbed = perpendicular;
                        if noise_strength > 0.0 {
                            let noise = self.noise2d(&world_pos, Vec2::broadcast(10.0), 2);
                            perturbed += noise * noise_strength;
                        }

                        let fade = if perturbed <= fade_start {
                            1.0
                        } else if perturbed <= fade_end {
                            let t = (perturbed - fade_start) / fade_distance; // 0‥1
                            1.0 - t * t * (3.0 - 2.0 * t) // smoothstep
                        } else {
                            0.0
                        };

                        if fade <= 0.01 {
                            continue;
                        }

                        let pixel_key = (pixel_local_x, pixel_local_y);
                        if let Some(&existing_fade) = written_blends.get(&pixel_key) {
                            if fade <= existing_fade {
                                continue;
                            }
                        }
                        written_blends.insert(pixel_key, fade);

                        let uv = world_pos / uv_scale;
                        let px = terrain.scale.x.max(terrain.scale.y);

                        let ctx = ShapeContext {
                            point_world: world_pos,
                            point: Vec2::new(pixel_local_x as f32, pixel_local_y as f32),
                            uv,
                            distance_world: perpendicular,
                            distance: perpendicular / px,
                            shape_id: seg.shape_id,
                            px,
                            anti_aliasing: 1.0,
                            t: Some(t_px),
                            line_dir: Some(seg.dir),
                            override_color: None,
                        };

                        let mut pixel_color: Option<Vec4<f32>> = None;
                        for node in &shapefx_nodes {
                            pixel_color = graph_node.0.nodes[*node as usize]
                                .evaluate_pixel(
                                    &ctx,
                                    pixel_color,
                                    assets,
                                    (graph_node.0, *node as usize),
                                )
                                .or(pixel_color);
                        }

                        if let Some(color) = pixel_color {
                            let existing = pixel_to_vec4(
                                &texture.get_pixel(pixel_local_x as u32, pixel_local_y as u32),
                            );
                            let fade = fade * color.w;
                            let blended = existing * (1.0 - fade) + color * fade;

                            let mut pixel = vec4_to_pixel(&blended);
                            pixel[3] = 255;
                            texture.set_pixel(pixel_local_x as u32, pixel_local_y as u32, pixel);
                        }
                    }
                }
            }
        }
    }

    pub fn render_setup(&mut self, hour: f32) -> Option<(Vec3<f32>, f32)> {
        self.precomputed.clear();
        match &self.role {
            Gradient => {
                let steps = self.values.get_int_default("steps", 4).max(1);
                let blend_mode = self.values.get_int_default("blend_mode", 0);

                let from_index = self.values.get_int_default("edge", 0);
                let to_index = self.values.get_int_default("interior", 1);

                self.precomputed.push(Vec4::new(
                    steps as f32,
                    blend_mode as f32,
                    from_index as f32,
                    to_index as f32,
                ));

                let thickness = self.values.get_float_default("thickness", 1.0);
                let offset = self.values.get_float_default("distance_offset", 0.0);
                let line_mode = self.values.get_int_default("line_mode", 0);

                self.precomputed
                    .push(Vec4::new(thickness, offset, line_mode as f32, 0.0));
            }
            Fog => {
                let fog_color = self
                    .values
                    .get_color_default("fog_color", TheColor::black())
                    .to_vec4();

                let end = self.values.get_float_default("fog_end_distance", 30.0);
                let fade = self.values.get_float_default("fog_fade_out", 20.0).max(1.0);

                self.precomputed.push(fog_color);
                self.precomputed.push(Vec4::new(end, fade, 0.0, 0.0));
            }
            Sky => {
                fn smoothstep_transition(hour: f32) -> f32 {
                    let dawn = ((hour - 6.0).clamp(0.0, 2.0) / 2.0).powi(2)
                        * (3.0 - 2.0 * (hour - 6.0).clamp(0.0, 2.0) / 2.0);
                    let dusk = ((20.0 - hour).clamp(0.0, 2.0) / 2.0).powi(2)
                        * (3.0 - 2.0 * (20.0 - hour).clamp(0.0, 2.0) / 2.0);

                    match hour {
                        h if h < 6.0 => 0.0,
                        h if h < 8.0 => dawn,
                        h if h < 18.0 => 1.0,
                        h if h < 20.0 => dusk,
                        _ => 0.0,
                    }
                }

                // Precompute sun position and atmospheric values
                // daylight window
                let sunrise = 6.0;
                let sunset = 20.0;

                let t_day = ((hour - sunrise) / (sunset - sunrise)).clamp(0.0, 1.0);

                let theta = t_day * std::f32::consts::PI;

                let sun_dir = Vec3::new(
                    theta.cos(), // +1 at sunrise, −1 at sunset
                    theta.sin(), //  0 at horizon, +1 overhead
                    0.0,
                );

                // Keep existing day factor calculation
                let day_factor = smoothstep_transition(hour);

                // Store in precomputed[0] as before
                self.precomputed
                    .push(Vec4::new(sun_dir.x, sun_dir.y, sun_dir.z, day_factor));

                // Precompute haze color (rgba)
                let haze_color = Vec4::lerp(
                    Vec4::new(0.1, 0.1, 0.15, 0.0), // Night haze
                    Vec4::new(0.3, 0.3, 0.35, 0.0), // Day haze
                    day_factor,
                );
                self.precomputed.push(haze_color);

                let day_horizon = self
                    .values
                    .get_color_default(
                        "day_horizon",
                        TheColor::from(Vec4::new(0.87, 0.80, 0.70, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(day_horizon);

                let day_zenith = self
                    .values
                    .get_color_default(
                        "day_zenith",
                        TheColor::from(Vec4::new(0.36, 0.62, 0.98, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(day_zenith);

                let night_horizon = self
                    .values
                    .get_color_default(
                        "night_horizon",
                        TheColor::from(Vec4::new(0.03, 0.04, 0.08, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(night_horizon);

                let night_zenith = self
                    .values
                    .get_color_default(
                        "night_zenith",
                        TheColor::from(Vec4::new(0.00, 0.01, 0.05, 1.0)),
                    )
                    .to_vec4();
                self.precomputed.push(night_zenith);

                return Some((sun_dir, day_factor));
            }
            _ => {}
        }

        None
    }

    pub fn render_hit_d3(
        &self,
        color: &mut Vec4<f32>,
        camera_pos: &Vec3<f32>,
        world_hit: &Vec3<f32>,
        _normal: &Vec3<f32>,
        _rasterizer: &Rasterizer,
        _time: f32,
    ) {
        #[allow(clippy::single_match)]
        match &self.role {
            Fog => {
                let distance = (world_hit - camera_pos).magnitude();
                let end = self.precomputed[1].x;
                let fade = self.precomputed[1].y;

                if distance > end {
                    let t = ((distance - end) / fade).clamp(0.0, 1.0);
                    *color = *color * (1.0 - t) + self.precomputed[0] * t;
                }
            }
            _ => {}
        }
    }

    pub fn render_ambient_color(&self, _hour: f32) -> Option<Vec4<f32>> {
        #[allow(clippy::single_match)]
        match &self.role {
            Sky => {
                // 0 : sun_dir.xyz  day_factor.w
                // 2 : day_horizon
                // 3 : day_zenith
                // 4 : night_horizon
                // 5 : night_zenith
                let day_factor = self.precomputed[0].w;

                let day_h = self.precomputed[2];
                let day_z = self.precomputed[3];
                let night_h = self.precomputed[4];
                let night_z = self.precomputed[5];

                // quick cosine-weighted average for each half-sphere
                let day_avg = (day_h * 0.5) + (day_z * 0.5);
                let night_avg = (night_h * 0.5) + (night_z * 0.5);

                // Blend between day and night tones by the pre-computed factor
                let c = Vec4::lerp(night_avg, day_avg, day_factor);

                let min_lim = 0.2;

                Some(Vec4::new(
                    linear_to_srgb(c.x.max(min_lim)),
                    linear_to_srgb(c.y.max(min_lim)),
                    linear_to_srgb(c.z.max(min_lim)),
                    1.0,
                ))
            }
            _ => None,
        }
    }

    pub fn render_miss_d3(
        &self,
        color: &mut Vec4<f32>,
        _camera_pos: &Vec3<f32>,
        ray: &Ray,
        _uv: &Vec2<f32>,
        _hour: f32,
    ) {
        #[allow(clippy::single_match)]
        match &self.role {
            Sky => {
                let sun_data = self.precomputed[0];
                let haze_color = self.precomputed[1];

                let sun_dir = Vec3::new(sun_data.x, sun_data.y, sun_data.z);
                let day_factor = sun_data.w;

                let up = ray.dir.y.clamp(-1.0, 1.0);
                let t = (up + 1.0) * 0.5;

                let day_zenith = self.precomputed[3];
                let day_horizon = self.precomputed[2];
                let night_zenith = self.precomputed[5];
                let night_horizon = self.precomputed[4];

                *color = Vec4::lerp(
                    Vec4::lerp(night_horizon, night_zenith, t),
                    Vec4::lerp(day_horizon, day_zenith, t),
                    day_factor,
                );

                // Atmospheric effects
                let haze = (1.0 - up).powi(3);
                let fog = haze_color * haze * 0.3;
                *color = *color * (1.0 - haze * 0.2) + fog;

                // Sun rendering
                if day_factor > 0.0 {
                    const SUN_RADIUS: f32 = 0.04;
                    let dot = ray.dir.dot(sun_dir).clamp(-1.0, 1.0);
                    let dist = (1.0 - dot).max(0.0);

                    if dist < SUN_RADIUS {
                        let k = 1.0 - dist / SUN_RADIUS;
                        let glare = k * k * (3.0 - 2.0 * k); // Smoothstep falloff
                        *color += Vec4::new(1.0, 0.85, 0.6, 0.0) * glare * day_factor;
                    }
                }

                if ray.dir.y > 0.0 {
                    const CLOUD_HEIGHT: f32 = 1500.0;
                    let t_hit = (CLOUD_HEIGHT - _camera_pos.y) / ray.dir.y;

                    if t_hit.is_finite() && t_hit > 0.0 {
                        let hit = *_camera_pos + ray.dir * t_hit;
                        let uv = Vec2::new(hit.x, hit.z) * 0.0005;

                        // let octaves = 1;
                        // let freq_falloff = 0.5;
                        // let lacunarity = 2.0;

                        let mut rng = UniformRandomGen::new(1);
                        let n = perlin_noise_2d(&mut rng, uv.x, uv.y, 5);

                        // let n = fractal_noise_add_2d(
                        //     &mut rng,
                        //     uv.x,
                        //     uv.y,
                        //     perlin_noise_2d,
                        //     octaves,
                        //     freq_falloff,
                        //     lacunarity,
                        //     1,
                        // );

                        let alpha_raw = (n + 1.0) * 0.5;
                        let alpha = alpha_raw * (ray.dir.y * 6.0).clamp(0.0, 1.0);

                        if alpha > 0.0 {
                            // ── Base brightness: never drop below 15 % grey
                            let min_whiteness = 0.15;
                            let whiteness = min_whiteness + (0.6 - min_whiteness) * day_factor; // 0.15 at night → 0.6 day
                            let base_colour = Vec4::lerp(*color, Vec4::one(), whiteness);

                            let sun_lit = (ray.dir.dot(sun_dir)).max(0.0).powf(3.0);
                            let rim = if day_factor > 0.0 {
                                // day: warm rim light
                                Vec4::new(1.0, 0.9, 0.8, 1.0) * sun_lit * 0.4 * day_factor
                            } else {
                                // night: cool moonlight at 20 % strength
                                Vec4::new(0.6, 0.7, 1.0, 1.0) * sun_lit * 0.08
                            };

                            let cloud_colour = base_colour + rim;
                            *color = Vec4::lerp(*color, cloud_colour, alpha);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn compile_material(&self) -> Option<Material> {
        if self.role == ShapeFXRole::Material {
            let role = self.values.get_int_default("role", 0);
            let modifier = self.values.get_int_default("modifier", 0);
            let value = self.values.get_float_default("value", 1.0);
            let flicker = self.values.get_float_default("flicker", 0.0);

            Some(Material {
                role: MaterialRole::from_u8(role as u8),
                modifier: MaterialModifier::from_u8(modifier as u8),
                value,
                flicker,
            })
        } else {
            None
        }
    }

    pub fn compile_light(&self, position: Vec3<f32>) -> Option<CompiledLight> {
        match self.role {
            PointLight => {
                let color = self
                    .values
                    .get_color_default("color", TheColor::white())
                    .to_vec3();
                let strength = self.values.get_float_default("strength", 5.0);
                let range = self.values.get_float_default("range", 10.0);
                let flick = self.values.get_float_default("flicker", 0.0);

                Some(CompiledLight {
                    light_type: LightType::Point,
                    position,
                    color: color.into_array(),
                    intensity: strength,
                    emitting: true,
                    start_distance: 0.0,
                    end_distance: range,
                    flicker: flick,
                    // unused fields:
                    direction: Vec3::unit_y(),
                    cone_angle: 0.0,
                    normal: Vec3::unit_y(),
                    width: 0.0,
                    height: 0.0,
                    from_linedef: false,
                })
            }
            _ => None,
        }
    }

    /// Evaluate the distance to the given shape.
    pub fn evaluate_distance(&self, pos: Vec2<f32>, vertices: &[Vec2<f32>]) -> Option<f32> {
        if vertices.is_empty() {
            return None;
        }
        match self.role {
            Circle => {
                let radius = self.values.get_float_default("radius", 0.5);
                Some((pos - vertices[0]).magnitude() - radius)
            }
            Line => {
                #[inline(always)]
                fn sd_segment(p: Vec2<f32>, a: Vec2<f32>, b: Vec2<f32>) -> f32 {
                    let pa = p - a;
                    let ba = b - a;
                    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
                    (pa - ba * h).magnitude()
                }

                #[inline(always)]
                fn sd_segment_asymmetric(
                    p: Vec2<f32>,
                    a: Vec2<f32>,
                    b: Vec2<f32>,
                    r0: f32,
                    r1: f32,
                ) -> f32 {
                    let pa = p - a;
                    let ba = b - a;
                    let ba_dot = ba.dot(ba);
                    if ba_dot == 0.0 {
                        return (p - a).magnitude() - r0.max(r1);
                    }

                    let h = (pa.dot(ba) / ba_dot).clamp(0.0, 1.0);
                    let interp_radius = r0 * (1.0 - h) + r1 * h;
                    (pa - ba * h).magnitude() - interp_radius
                }

                if vertices.len() >= 2 {
                    let radius = self.values.get_float_default("radius", 0.5);
                    let radius2 = self.values.get_float_default("radius2", 0.0);
                    if radius2 == 0.0 {
                        Some(sd_segment(pos, vertices[0], vertices[1]) - radius)
                    } else {
                        Some(sd_segment_asymmetric(
                            pos,
                            vertices[0],
                            vertices[1],
                            radius,
                            radius2,
                        ))
                    }
                } else {
                    None
                }
            }
            Box => {
                /// Signed distance to an oriented box defined by line segment `a` to `b` and thickness `th`.
                pub fn sd_oriented_box(
                    p: Vec2<f32>,
                    a: Vec2<f32>,
                    b: Vec2<f32>,
                    th: f32,
                    rounding: f32,
                ) -> f32 {
                    let ba = b - a;
                    let l = ba.magnitude();
                    if l == 0.0 {
                        return f32::MAX;
                    }

                    let d = ba / l;
                    let center = (a + b) * 0.5;
                    let mut q = p - center;

                    // Rotate into box frame
                    let rotated_x = d.x * q.x + d.y * q.y;
                    let rotated_y = -d.y * q.x + d.x * q.y;
                    q = Vec2::new(rotated_x.abs(), rotated_y.abs());

                    let half_size = Vec2::new(l * 0.5, th * 0.5);
                    let q_minus = q - half_size + Vec2::broadcast(rounding);

                    let max_q = Vec2::new(q_minus.x.max(0.0), q_minus.y.max(0.0));
                    let outside_dist = max_q.magnitude();
                    let inside_dist = q_minus.x.max(q_minus.y).min(0.0);

                    outside_dist + inside_dist - rounding
                }

                if vertices.len() >= 2 {
                    let thickness = self.values.get_float_default("thickness", 0.5);
                    let rounding = self.values.get_float_default("rounding", 0.0);
                    Some(sd_oriented_box(
                        pos,
                        vertices[0],
                        vertices[1],
                        thickness,
                        rounding,
                    ))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn evaluate_pixel(
        &self,
        ctx: &ShapeContext,
        color: Option<Vec4<f32>>,
        assets: &Assets,
        graph_node: (&ShapeFXGraph, usize),
    ) -> Option<Vec4<f32>> {
        match self.role {
            MaterialGeometry => None,
            /*
            Gradient => {
                let alpha = 1.0 - ShapeFX::smoothstep(-ctx.anti_aliasing, 0.0, ctx.distance);
                if alpha > 0.0 {
                    let mut from = Vec4::zero();
                    let top_index = self.values.get_int_default("from", 0);
                    if let Some(Some(top_color)) = palette.colors.get(top_index as usize) {
                        from = top_color.to_vec4();
                    }
                    let mut to = Vec4::zero();
                    let bottom_index = self.values.get_int_default("to", 1);
                    if let Some(Some(bottom_color)) = palette.colors.get(bottom_index as usize) {
                        to = bottom_color.to_vec4();
                    }

                    let angle_rad =
                        (90.0 - self.values.get_float_default("direction", 0.0)).to_radians();
                    let dir = Vec2::new(angle_rad.cos(), angle_rad.sin());

                    let pixel_size = self.values.get_float_default("pixelsize", 0.05);
                    let snapped_uv = Vec2::new(
                        (ctx.uv.x / pixel_size).floor() * pixel_size,
                        (ctx.uv.y / pixel_size).floor() * pixel_size,
                    );

                    let centered_uv = snapped_uv - Vec2::new(0.5, 0.5);
                    let projection = centered_uv.dot(dir);
                    let mut t =
                        (projection / std::f32::consts::FRAC_1_SQRT_2 * 0.5 + 0.5).clamp(0.0, 1.0);
                    if let Some(line_t) = ctx.t {
                        t = line_t.fract();
                    }

                    let dithering = self.values.get_int_default("dithering", 1);
                    if dithering == 1 {
                        let px = (ctx.uv.x / pixel_size).floor() as i32;
                        let py = (ctx.uv.y / pixel_size).floor() as i32;
                        let checker = ((px + py) % 2) as f32 * 0.03; // small tweak value
                        t = (t + checker).clamp(0.0, 1.0);
                    }

                    let mut c = from * (1.0 - t) + to * t;
                    /*
                    c.w = 1.0;
                    if let Some(index) = palette.find_closest_color_index(&TheColor::from(c)) {
                        if let Some(Some(col)) = palette.colors.get(index) {
                            c = col.to_vec4();
                        }
                    }*/
                    c.w = alpha;
                    Some(c)
                } else {
                    None
                }
            }*/
            Gradient => {
                let pixel_size = 0.05;
                // let steps = self.values.get_int_default("steps", 4).max(1);
                // let blend_mode = self.values.get_int_default("blend_mode", 0);

                // let from_index = self.values.get_int_default("edge", 0);
                // let to_index = self.values.get_int_default("interior", 1);

                let steps = self.precomputed[0].x as i32;
                let blend_mode = self.precomputed[0].y as i32;
                let from_index = self.precomputed[0].z as i32;
                let to_index = self.precomputed[0].w as i32;

                let mut from = assets
                    .palette
                    .colors
                    .get(from_index as usize)
                    .and_then(|c| c.clone())
                    .unwrap_or(TheColor::black())
                    .to_vec4();
                if blend_mode == 1 && color.is_some() {
                    from = color.unwrap();
                }

                let to = if let Some(color) = ctx.override_color {
                    color
                } else {
                    assets
                        .palette
                        .colors
                        .get(to_index as usize)
                        .and_then(|c| c.clone())
                        .unwrap_or(TheColor::white())
                        .to_vec4()
                };

                // let thickness = self.values.get_float_default("thickness", 40.0);
                // let offset = self.values.get_float_default("distance_offset", 0.0);

                let thickness = self.precomputed[1].x / ctx.px;
                let offset = self.precomputed[1].y / ctx.px;
                let depth = (-(ctx.distance + offset)).clamp(0.0, thickness);

                let snapped_depth = (depth / pixel_size).floor() * pixel_size;
                let mut t = (snapped_depth / thickness).clamp(0.0, 1.0);

                if let Some(line_t) = ctx.t {
                    // let line_mode = self.values.get_int_default("line_mode", 0);
                    let line_mode = self.precomputed[1].z as i32;
                    if line_mode == 1 {
                        let line_factor = line_t.clamp(0.0, 1.0);
                        let radial_factor = (depth / thickness).clamp(0.0, 1.0);
                        t = radial_factor * (1.0 - line_factor);
                    }
                }

                let px = (ctx.uv.x / pixel_size).floor() as i32;
                let py = (ctx.uv.y / pixel_size).floor() as i32;

                let bx = (px & 3) as usize;
                let by = (py & 3) as usize;
                let threshold = BAYER_4X4[by][bx];

                let ft = t * steps as f32;
                let base_step = ft.floor();
                let step_frac = ft - base_step;

                let dithered_step = if step_frac > threshold {
                    base_step + 1.0
                } else {
                    base_step
                }
                .min((steps - 1) as f32);

                let quantized_t = dithered_step / (steps - 1).max(1) as f32;

                let color = from * (1.0 - quantized_t) + to * quantized_t;
                Some(Vec4::new(color.x, color.y, color.z, 1.0))
            }
            Color => {
                let alpha = if ctx.distance > 0.0 {
                    1.0
                } else {
                    1.0 - ShapeFX::smoothstep(-ctx.anti_aliasing, 0.0, ctx.distance)
                };
                if alpha > 0.0 {
                    let mut color = Vec4::zero();
                    let index = self.values.get_int_default("color", 0);
                    if let Some(Some(col)) = assets.palette.colors.get(index as usize) {
                        color = col.to_vec4();
                    }
                    color.w = alpha;
                    Some(color)
                } else {
                    None
                }
            }
            Outline => {
                let mut color = Vec4::zero();
                let index = self.values.get_int_default("color", 0);
                if let Some(Some(col)) = assets.palette.colors.get(index as usize) {
                    color = col.to_vec4();
                }
                let thickness = self.values.get_float_default("thickness", 1.5);
                if ctx.distance < 0.0 && ctx.distance >= -thickness {
                    Some(color)
                } else {
                    None
                }
            }
            NoiseOverlay => {
                let pixel_size = self.values.get_float_default("pixel_size", 0.05);
                let randomness = self.values.get_float_default("randomness", 0.2);
                let octaves = self.values.get_int_default("octaves", 3);

                if let Some(mut color) = color {
                    let mut other_color: Option<Vec4<f32>> = None;
                    let shapefx_nodes = graph_node.0.collect_nodes_from(graph_node.1, 1);
                    for node in &shapefx_nodes {
                        other_color = graph_node.0.nodes[*node as usize]
                            .evaluate_pixel(
                                ctx,
                                other_color,
                                assets,
                                (graph_node.0, *node as usize),
                            )
                            .or(other_color);
                    }

                    let uv = ctx.uv;
                    let scale = Vec2::broadcast(1.0 / pixel_size);
                    let noise_value =
                        self.noise2d_tileable(&uv, scale, octaves, Vec2::new(-5.0, 5.0));

                    let n = (noise_value * 2.0 - 1.0) * randomness;

                    if let Some(other) = other_color {
                        let blend_factor = (noise_value * randomness).clamp(0.0, 1.0);
                        color = Vec4::lerp(color, other, blend_factor);
                    } else {
                        color.x = (color.x + n).clamp(0.0, 1.0);
                        color.y = (color.y + n).clamp(0.0, 1.0);
                        color.z = (color.z + n).clamp(0.0, 1.0);
                    }
                    Some(color)
                } else {
                    None
                }
            }
            Glow => {
                let thickness = self.values.get_float_default("radius", 10.0);
                if ctx.distance > 0.0 && ctx.distance <= thickness {
                    let index = self.values.get_int_default("color", 0);
                    let mut color = assets
                        .palette
                        .colors
                        .get(index as usize)
                        .and_then(|c| c.clone())
                        .unwrap_or(TheColor::white())
                        .to_vec4();

                    let t = (ctx.distance / thickness).clamp(0.0, 1.0);
                    let alpha = 1.0 - ShapeFX::smoothstep(0.0, 1.0, t);
                    color.w = alpha;

                    Some(color)
                } else {
                    None
                }
            }
            ShapeFXRole::Wood => {
                let alpha = if ctx.distance >= 0.0 {
                    1.0 - (ctx.distance / ctx.px).clamp(0.0, 1.0)
                } else {
                    1.0
                };

                if alpha <= 0.0 {
                    return None;
                }

                let mut light = Vec4::one();
                let mut dark = Vec4::zero();

                let light_nodes = graph_node.0.collect_nodes_from(graph_node.1, 1);
                for node in &light_nodes {
                    light = graph_node.0.nodes[*node as usize]
                        .evaluate_pixel(ctx, Some(light), assets, (graph_node.0, *node as usize))
                        .unwrap_or(light);
                }

                let dark_nodes = graph_node.0.collect_nodes_from(graph_node.1, 2);
                for node in &dark_nodes {
                    dark = graph_node.0.nodes[*node as usize]
                        .evaluate_pixel(ctx, Some(dark), assets, (graph_node.0, *node as usize))
                        .unwrap_or(dark);
                }

                let direction_deg = self.values.get_float_default("direction", 0.0);
                let scale = self.values.get_float_default("grain_scale", 4.0); // px between streaks
                let streak_noise = self.values.get_float_default("streak_noise", 1.5); // jaggedness
                let fine_noise = self.values.get_float_default("fine_noise", 0.10); // subtle speckle
                let octaves = self.values.get_int_default("octaves", 3);

                let dir_rad = direction_deg.to_radians();
                let axis = Vec2::new(dir_rad.cos(), dir_rad.sin()); // along plank
                let perpendicular = Vec2::new(-axis.y, axis.x); // across plank

                // Distance “across” the plank controls the stripe colour.
                let across = ctx.uv.dot(perpendicular) * scale; // repeat every 'scale' px
                // Low-freq noise makes the stripes wavy
                let wobble = self.noise2d(&ctx.uv, Vec2::broadcast(0.5), octaves) * streak_noise;
                // Sharpen to make pronounced early/late wood bands
                // let stripe = (across + wobble).fract(); // 0..1 saw wave
                // let stripe_mask = (stripe.min(1.0 - stripe)).powf(0.4);
                // -- 4. main streaks -------------------------------------------------
                let raw = across + wobble;
                let mut s = raw.fract(); // [-1,1)        <-- may be negative
                if s < 0.0 {
                    s += 1.0;
                } // wrap into 0‥1

                // triangle wave: 0 at edges, 1 in the middle
                let stripe_mask = 1.0 - (2.0 * s - 1.0).abs(); // 0‥1
                let stripe_mask = stripe_mask.powf(0.4); // sharpen

                // === 5. fine noise overlay =====================================
                let grain = self.noise2d(&(ctx.uv * 120.0), Vec2::one(), 1) * fine_noise;

                // === 6. final blend ============================================
                let t = (stripe_mask + grain).clamp(0.0, 1.0);
                let mut c = light * (1.0 - t) + dark * t;
                c.w = alpha;
                c = c.map(|v| v.clamp(0.0, 1.0));
                Some(c)
            }
            ShapeFXRole::Stone => {
                let gap = self.values.get_float_default("gap", 0.2); // mortar width
                let rounding = self.values.get_float_default("rounding", 0.2); // edge rounding
                let rotation = self.values.get_float_default("rotation", 3.0); // random twist strength

                let mut stone = Vec4::one();
                let mut mortar = Vec4::zero();

                let stone_nodes = graph_node.0.collect_nodes_from(graph_node.1, 1);
                for node in &stone_nodes {
                    stone = graph_node.0.nodes[*node as usize]
                        .evaluate_pixel(ctx, Some(stone), assets, (graph_node.0, *node as usize))
                        .unwrap_or(stone);
                }

                let mortar_nodes = graph_node.0.collect_nodes_from(graph_node.1, 2);
                for node in &mortar_nodes {
                    mortar = graph_node.0.nodes[*node as usize]
                        .evaluate_pixel(ctx, Some(mortar), assets, (graph_node.0, *node as usize))
                        .unwrap_or(mortar);
                }

                // UVs – rotate with linedef if present
                let uv = if let Some(dir) = ctx.line_dir {
                    let along = dir.normalized();
                    let across = Vec2::new(-along.y, along.x);
                    Vec2::new(ctx.uv.dot(along), ctx.uv.dot(across))
                } else {
                    ctx.uv
                };

                // uv += Vec2::new(
                //     self.noise2d(&(uv / 8.5), Vec2::one(), 0),
                //     self.noise2d(&(uv / 8.5 + 7.3), Vec2::one(), 0),
                // ) * 0.008;

                // get SDF + cell id
                let (d, id) = self.box_divide(uv / 10.0, gap, rotation / 10.0, rounding);

                let edge = ShapeFX::smoothstep(-0.08, 0.0, d);

                // Subtle per-tile tint to break monotony
                let shade = 0.85 + 0.3 * (id * 2.0 - 1.0); // 0.55‥1.15

                // Blend stone ↔ mortar, then apply tint to the stone only
                let shaded_stone = stone * shade;
                let mut c = mortar * edge + shaded_stone * (1.0 - edge);
                c.w = 1.0;

                Some(c)
            }
            _ => None,
        }
    }

    /// The parameters for the shapefx
    pub fn params(&self) -> Vec<ShapeFXParam> {
        let mut params = vec![];
        match self.role {
            /*
            Gradient => {
                params.push(ShapeFXParam::Float(
                    "direction".into(),
                    "Direction".into(),
                    "The direction of the gradient.".into(),
                    self.values.get_float_default("direction", 0.0),
                    0.0..=360.0,
                ));
                params.push(ShapeFXParam::Float(
                    "pixelsize".into(),
                    "Pixel Size".into(),
                    "The direction of the gradient.".into(),
                    self.values.get_float_default("pixelsize", 0.05),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Selector(
                    "dithering".into(),
                    "Dithering".into(),
                    "Dithering options for the gradient.".into(),
                    vec!["None".into(), "Checker".into()],
                    self.values.get_int_default("dithering", 1),
                ));
                params.push(ShapeFXParam::PaletteIndex(
                    "from".into(),
                    "From".into(),
                    "The start color of the gradient.".into(),
                    self.values.get_int_default("from", 0),
                ));
                params.push(ShapeFXParam::PaletteIndex(
                    "to".into(),
                    "To".into(),
                    "The end color of the gradient.".into(),
                    self.values.get_int_default("to", 1),
                ))
            }*/
            ShapeFXRole::MaterialGeometry => {
                params.push(ShapeFXParam::Float(
                    "rounding".into(),
                    "Rounding".into(),
                    "Optional rounding of the sector shape.".into(),
                    self.values.get_float_default("rounding", 0.0),
                    0.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "line_width".into(),
                    "Line Width".into(),
                    "Width of linedefs.".into(),
                    self.values.get_float_default("line_width", 1.0),
                    1.0..=10.0,
                ));
            }
            Gradient => {
                params.push(ShapeFXParam::PaletteIndex(
                    "edge".into(),
                    "Edge Color".into(),
                    "The color at the shape's edge.".into(),
                    self.values.get_int_default("edge", 0),
                ));

                params.push(ShapeFXParam::PaletteIndex(
                    "interior".into(),
                    "Interior Color".into(),
                    "The color towards the shape center.".into(),
                    self.values.get_int_default("interior", 1),
                ));

                params.push(ShapeFXParam::Float(
                    "thickness".into(),
                    "Thickness".into(),
                    "How far the gradient extends inward.".into(),
                    self.values.get_float_default("thickness", 1.0),
                    0.0..=10.0,
                ));
                params.push(ShapeFXParam::Int(
                    "steps".into(),
                    "Steps".into(),
                    "Number of shading bands.".into(),
                    self.values.get_int_default("steps", 4),
                    1..=8,
                ));
                params.push(ShapeFXParam::Selector(
                    "blend_mode".into(),
                    "Blend Mode".into(),
                    "If enabled, uses the incoming color from the previous node as the edge color instead of the palette."
                        .into(),
                    vec!["Off".into(), "Use Incoming Color".into()],
                    self.values.get_int_default("blend_mode", 0),
                ));
                params.push(ShapeFXParam::Selector(
                    "line_mode".into(),
                    "Line Mode".into(),
                    "If the geometry is a line, choose how the gradient is applied: either fading in from the edge (Outside In), or along the line's direction (Line Direction)."
                        .into(),
                    vec!["Outside In".into(), "Line Direction".into()],
                    self.values.get_int_default("line_mode", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "distance_offset".into(),
                    "Distance Offset".into(),
                    "Shift the start of the gradient inward or outward from the shape edge.".into(),
                    self.values.get_float_default("distance_offset", 0.0),
                    -10.0..=10.0,
                ));
            }
            Color => {
                params.push(ShapeFXParam::PaletteIndex(
                    "color".into(),
                    "Color".into(),
                    "The fill color.".into(),
                    self.values.get_int_default("color", 0),
                ));
            }
            Outline => {
                params.push(ShapeFXParam::PaletteIndex(
                    "color".into(),
                    "Color".into(),
                    "The fill color.".into(),
                    self.values.get_int_default("color", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "thickness".into(),
                    "Thickness.".into(),
                    "The thickness of the outlint.".into(),
                    self.values.get_float_default("pixelsize", 1.5),
                    0.0..=10.0,
                ));
            }
            NoiseOverlay => {
                params.push(ShapeFXParam::Float(
                    "pixel_size".into(),
                    "Pixel Size".into(),
                    "Size of the noise pixel grid.".into(),
                    self.values.get_float_default("pixel_size", 0.05),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Float(
                    "randomness".into(),
                    "Randomness".into(),
                    "Randomness factor applied to each pixel.".into(),
                    self.values.get_float_default("randomness", 0.2),
                    0.0..=2.0,
                ));
                params.push(ShapeFXParam::Int(
                    "octaves".into(),
                    "Octaves".into(),
                    "Number of noise layers.".into(),
                    self.values.get_int_default("octaves", 3),
                    0..=6,
                ));
            }
            Glow => {
                params.push(ShapeFXParam::PaletteIndex(
                    "color".into(),
                    "Glow Color".into(),
                    "Color of the glow.".into(),
                    self.values.get_int_default("color", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "radius".into(),
                    "Glow Radius".into(),
                    "How far the glow extends outside the shape.".into(),
                    self.values.get_float_default("radius", 10.0),
                    0.0..=100.0,
                ));
            }
            ShapeFXRole::Wood => {
                params.push(ShapeFXParam::Float(
                    "grain_scale".into(),
                    "Streak Spacing".into(),
                    "Average pixel distance between streaks.".into(),
                    self.values.get_float_default("grain_scale", 4.0),
                    0.5..=50.0,
                ));
                params.push(ShapeFXParam::Float(
                    "streak_noise".into(),
                    "Streak Noise".into(),
                    "Side-to-side waviness of the streaks.".into(),
                    self.values.get_float_default("streak_noise", 1.5),
                    0.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "fine_noise".into(),
                    "Fine Grain".into(),
                    "Subtle high-frequency speckles.".into(),
                    self.values.get_float_default("fine_noise", 0.10),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Int(
                    "octaves".into(),
                    "Noise Octaves".into(),
                    "Detail levels for streak wobble.".into(),
                    self.values.get_int_default("octaves", 3),
                    0..=6,
                ));
                params.push(ShapeFXParam::Float(
                    "direction".into(),
                    "Direction".into(),
                    "Plank direction (°).".into(),
                    self.values.get_float_default("direction", 0.0),
                    0.0..=360.0,
                ));
            }
            ShapeFXRole::Stone => {
                params.push(ShapeFXParam::Float(
                    "gap".into(),
                    "Gap Width".into(),
                    "Thickness of the mortar.".into(),
                    self.values.get_float_default("gap", 0.2),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Float(
                    "rounding".into(),
                    "Corner Rounding".into(),
                    "Smooth the brick corners.".into(),
                    self.values.get_float_default("rounding", 0.2),
                    0.0..=0.5,
                ));
                params.push(ShapeFXParam::Float(
                    "rotation".into(),
                    "Random Twist".into(),
                    "Random rotation amount per tile.".into(),
                    self.values.get_float_default("rotation", 3.0),
                    0.0..=10.0,
                ));
            }
            Flatten => {
                params.push(ShapeFXParam::Float(
                    "bevel".into(),
                    "Bevel".into(),
                    "Smoothly blends the shape's height into the surrounding terrain over this distance.".into(),
                    self.values.get_float_default("bevel", 0.5),
                    0.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "fade_distance".into(),
                    "Pixel Fade".into(),
                    "Fades outward from the sector / linedef. For sectors negative fades start inward."
                        .into(),
                    self.values.get_float_default("fade_distance", 0.5),
                    -10.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "fade_noise".into(),
                    "Fade Noise".into(),
                    "Adds noise-based distortion to the fade shape.".into(),
                    self.values.get_float_default("fade_noise", 0.0),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Float(
                    "path_width".into(),
                    "Path Width".into(),
                    "Total width of the colorized path for linedefs. Ignored by sectors".into(),
                    self.values.get_float_default("path_width", 2.0),
                    0.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "uv_scale".into(),
                    "UV Scale".into(),
                    "Tiling scale for procedural textures.".into(),
                    self.values.get_float_default("uv_scale", 1.0),
                    0.01..=20.0,
                ));
            }
            ShapeFXRole::Colorize => {
                params.push(ShapeFXParam::Float(
                    "fade_distance".into(),
                    "Fade Distance".into(),
                    "Fades the color outward from the shape boundary.".into(),
                    self.values.get_float_default("fade_distance", 0.5),
                    -10.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "fade_noise".into(),
                    "Fade Noise".into(),
                    "Adds noise distortion to the fade boundary.".into(),
                    self.values.get_float_default("fade_noise", 0.0),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Float(
                    "path_width".into(),
                    "Path Width".into(),
                    "Width of the effect for linedefs.".into(),
                    self.values.get_float_default("path_width", 2.0),
                    0.0..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "uv_scale".into(),
                    "UV Scale".into(),
                    "Tiling scale for effects that use UVs.".into(),
                    self.values.get_float_default("uv_scale", 1.0),
                    0.01..=20.0,
                ));

                // Height-based filter
                params.push(ShapeFXParam::Float(
                    "min_height".into(),
                    "Min Height".into(),
                    "Only apply color if world height is above this value.".into(),
                    self.values.get_float_default("min_height", 0.0),
                    -100.0..=100.0,
                ));
                params.push(ShapeFXParam::Float(
                    "max_height".into(),
                    "Max Height".into(),
                    "Only apply color if world height is below this value.".into(),
                    self.values.get_float_default("max_height", 10.0),
                    -100.0..=100.0,
                ));

                // Steepness-based filter
                params.push(ShapeFXParam::Float(
                    "min_steepness".into(),
                    "Min Steepness".into(),
                    "Only apply color if slope steepness is above this value (0 = flat).".into(),
                    self.values.get_float_default("min_steepness", 0.0),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Float(
                    "max_steepness".into(),
                    "Max Steepness".into(),
                    "Only apply color if slope steepness is below this value (1 = vertical)."
                        .into(),
                    self.values.get_float_default("max_steepness", 1.0),
                    0.0..=1.0,
                ));
            }
            ShapeFXRole::Fog => {
                params.push(ShapeFXParam::Color(
                    "fog_color".into(),
                    "Fog Color".into(),
                    "Colour applied to distant fragments.".into(),
                    self.values
                        .get_color_default("fog_color", TheColor::black()),
                ));
                params.push(ShapeFXParam::Float(
                    "fog_end_distance".into(),
                    "End Distance".into(),
                    "World-space distance where fog is 100 % opaque.".into(),
                    self.values.get_float_default("fog_end_distance", 30.0),
                    0.0..=2_000.0,
                ));
                params.push(ShapeFXParam::Float(
                    "fog_fade_out".into(),
                    "Fade-out Length".into(),
                    "How far the fog takes to fade back to clear after the end distance.".into(),
                    self.values.get_float_default("fog_fade_out", 20.0),
                    0.0..=2_000.0,
                ));
            }
            ShapeFXRole::Sky => {
                params.push(ShapeFXParam::Color(
                    "day_horizon".into(),
                    "Day Horizon".into(),
                    "Colour blended along the horizon during daylight.".into(),
                    self.values
                        .get_color_default("day_horizon", TheColor::new(0.87, 0.80, 0.70, 1.0)),
                ));
                params.push(ShapeFXParam::Color(
                    "day_zenith".into(),
                    "Day Zenith".into(),
                    "Colour blended straight overhead during daylight.".into(),
                    self.values
                        .get_color_default("day_zenith", TheColor::new(0.36, 0.62, 0.98, 1.0)),
                ));
                params.push(ShapeFXParam::Color(
                    "night_horizon".into(),
                    "Night Horizon".into(),
                    "Colour along the horizon after sunset / before sunrise.".into(),
                    self.values
                        .get_color_default("night_horizon", TheColor::new(0.03, 0.04, 0.08, 1.0)),
                ));
                params.push(ShapeFXParam::Color(
                    "night_zenith".into(),
                    "Night Zenith".into(),
                    "Colour straight overhead during the night.".into(),
                    self.values
                        .get_color_default("night_zenith", TheColor::new(0.00, 0.01, 0.05, 1.0)),
                ));
            }
            Material => {
                params.push(ShapeFXParam::Selector(
                    "role".into(),
                    "Type".into(),
                    "The material type.".into(),
                    vec![
                        "Matte".into(),
                        "Glossy".into(),
                        "Metallic".into(),
                        "Transparent".into(),
                        "Emissive".into(),
                    ],
                    self.values.get_int_default("role", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "value".into(),
                    "Value".into(),
                    "The material value.".into(),
                    self.values.get_float_default("value", 1.0),
                    0.0..=1.0,
                ));
                params.push(ShapeFXParam::Selector(
                    "modifier".into(),
                    "Modifier".into(),
                    "The material modifier (applies value based on color).".into(),
                    vec![
                        "None".into(),
                        "Luminance".into(),
                        "Saturation".into(),
                        "Inverse Luminance".into(),
                        "Inverse Saturation".into(),
                    ],
                    self.values.get_int_default("modifier", 0),
                ));
                params.push(ShapeFXParam::Float(
                    "flicker".into(),
                    "Flicker".into(),
                    "Flicker for emissive materials.".into(),
                    self.values.get_float_default("flicker", 0.0),
                    0.0..=1.0,
                ));
            }
            ShapeFXRole::PointLight => {
                params.push(ShapeFXParam::Color(
                    "color".into(),
                    "Colour".into(),
                    "Light colour".into(),
                    self.values.get_color_default("color", TheColor::white()),
                ));
                params.push(ShapeFXParam::Float(
                    "strength".into(),
                    "Strength".into(),
                    "How bright the light is.".into(),
                    self.values.get_float_default("strength", 5.0),
                    0.0..=100.0,
                ));
                params.push(ShapeFXParam::Float(
                    "range".into(),
                    "Range".into(),
                    "Rough maximum reach.".into(),
                    self.values.get_float_default("range", 10.0),
                    0.5..=100.0,
                ));
                params.push(ShapeFXParam::Float(
                    "flicker".into(),
                    "Flicker".into(),
                    "0 = steady, 1 = candles.".into(),
                    self.values.get_float_default("flicker", 0.0),
                    0.0..=1.0,
                ));
            }
            ShapeFXRole::Circle | ShapeFXRole::Line => {
                params.push(ShapeFXParam::Float(
                    "radius".into(),
                    "Radius".into(),
                    "The radius of the shape.".into(),
                    self.values.get_float_default("radius", 0.5),
                    0.001..=3.0,
                ));

                if let ShapeFXRole::Line = self.role {
                    params.push(ShapeFXParam::Float(
                        "radius2".into(),
                        "End Radius".into(),
                        "The radius at the end of the line segment. If set to 0, the same radius is used at both ends.".into(),
                        self.values.get_float_default("radius2", 0.0),
                        0.001..=3.0,
                    ));
                }

                params.push(ShapeFXParam::Float(
                    "blend_k".into(),
                    "Smooth Blend".into(),
                    "Blending smoothness for combining this shape with others. 0 = hard edge, higher = smoother union."
                        .into(),
                    self.values.get_float_default("blend_k", 0.0),
                    0.0..=1.0,
                ));
            }
            ShapeFXRole::Box => {
                params.push(ShapeFXParam::Float(
                    "thickness".into(),
                    "Thickness".into(),
                    "The thickness of the box.".into(),
                    self.values.get_float_default("thickness", 0.5),
                    0.001..=10.0,
                ));
                params.push(ShapeFXParam::Float(
                    "rounding".into(),
                    "Rounding".into(),
                    "The rounding of the box.".into(),
                    self.values.get_float_default("rounding", 0.0),
                    0.001..=3.0,
                ));
                params.push(ShapeFXParam::Float(
                    "blend_k".into(),
                    "Smooth Blend".into(),
                    "Blending smoothness for combining this shape with others. 0 = hard edge, higher = smoother union."
                        .into(),
                    self.values.get_float_default("blend_k", 0.0),
                    0.0..=1.0,
                ));
            }
            _ => {}
        }
        params
    }

    #[inline]
    fn _lerp(a: f32, b: f32, t: f32) -> f32 {
        a * (1.0 - t) + b * t
    }

    #[inline]
    pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn noise2d(&self, p: &Vec2<f32>, scale: Vec2<f32>, octaves: i32) -> f32 {
        fn hash(p: Vec2<f32>) -> f32 {
            let mut p3 = Vec3::new(p.x, p.y, p.x).map(|v| (v * 0.13).fract());
            p3 += p3.dot(Vec3::new(p3.y, p3.z, p3.x) + 3.333);
            ((p3.x + p3.y) * p3.z).fract()
        }

        fn noise(x: Vec2<f32>) -> f32 {
            let i = x.map(|v| v.floor());
            let f = x.map(|v| v.fract());

            let a = hash(i);
            let b = hash(i + Vec2::new(1.0, 0.0));
            let c = hash(i + Vec2::new(0.0, 1.0));
            let d = hash(i + Vec2::new(1.0, 1.0));

            let u = f * f * f.map(|v| 3.0 - 2.0 * v);
            f32::lerp(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y
        }

        let mut x = *p * 8.0 * scale;

        if octaves == 0 {
            return noise(x);
        }

        let mut v = 0.0;
        let mut a = 0.5;
        let shift = Vec2::new(100.0, 100.0);
        let rot = Mat2::new(0.5f32.cos(), 0.5f32.sin(), -0.5f32.sin(), 0.5f32.cos());
        for _ in 0..octaves {
            v += a * noise(x);
            x = rot * x * 2.0 + shift;
            a *= 0.5;
        }
        v
    }

    fn noise2d_tileable(
        &self,
        p: &Vec2<f32>,
        scale: Vec2<f32>,
        octaves: i32,
        tile_size: Vec2<f32>,
    ) -> f32 {
        fn hash(p: Vec2<f32>, tile_size: Vec2<f32>) -> f32 {
            let p = (p % tile_size + tile_size) % tile_size; // wrap into positive range
            let mut p3 = Vec3::new(p.x, p.y, p.x).map(|v| (v * 0.13).fract());
            p3 += p3.dot(Vec3::new(p3.y, p3.z, p3.x) + 3.333);
            ((p3.x + p3.y) * p3.z).fract()
        }

        fn noise(x: Vec2<f32>, tile_size: Vec2<f32>) -> f32 {
            let i = x.map(|v| v.floor());
            let f = x.map(|v| v.fract());

            let a = hash(i, tile_size);
            let b = hash(i + Vec2::new(1.0, 0.0), tile_size);
            let c = hash(i + Vec2::new(0.0, 1.0), tile_size);
            let d = hash(i + Vec2::new(1.0, 1.0), tile_size);

            let u = f * f * f.map(|v| 3.0 - 2.0 * v);
            f32::lerp(a, b, u.x) + (c - a) * u.y * (1.0 - u.x) + (d - b) * u.x * u.y
        }

        let mut x = p * 8.0 * scale;
        let mut current_tile_size = tile_size * 8.0 * scale;

        if octaves == 0 {
            return noise(x, current_tile_size);
        }

        let mut v = 0.0;
        let mut a = 0.5;

        for _ in 0..octaves {
            v += a * noise(x, current_tile_size);
            x *= 2.0;
            current_tile_size *= 2.0; // maintain tiling across octaves
            a *= 0.5;
        }

        v
    }

    #[inline(always)]
    fn rot(&self, a: f32) -> Mat2<f32> {
        let (s, c) = a.sin_cos();
        Mat2::new(c, s, -s, c)
    }

    #[inline(always)]
    fn hash21(&self, p: Vec2<f32>) -> f32 {
        let mut p3 = Vec3::new(
            (p.x * 0.1031).fract(),
            (p.y * 0.1031).fract(),
            (p.x * 0.1031).fract(),
        );
        let dot = p3.dot(Vec3::new(p3.y + 33.333, p3.z + 33.333, p3.x + 33.333));
        p3 += Vec3::broadcast(dot);
        ((p3.x + p3.y) * p3.z).fract()
    }

    // Shane — box-divide SDF
    fn box_divide(&self, p: Vec2<f32>, gap: f32, rotation: f32, rounding: f32) -> (f32, f32) {
        #[inline(always)]
        fn s_box(p: Vec2<f32>, b: Vec2<f32>, r: f32) -> f32 {
            let d = p.map(|v| v.abs()) - b + Vec2::broadcast(r);
            d.x.max(d.y).min(0.0) + (d.map(|v| v.max(0.0))).magnitude() - r
        }

        let mut p = p;
        let ip = p.map(|v| v.floor());
        p -= ip;

        let mut l = Vec2::broadcast(1.0);
        let mut r = self.hash21(ip);
        for _ in 0..6 {
            r = (l + Vec2::new(r, r)).dot(Vec2::new(123.71, 439.43)).fract() * 0.4 + 0.3;

            if l.x > l.y {
                p = Vec2::new(p.y, p.x);
                l = Vec2::new(l.y, l.x);
            }

            if p.x < r {
                l.x /= r;
                p.x /= r;
            } else {
                l.x /= 1.0 - r;
                p.x = (p.x - r) / (1.0 - r);
            }

            if l.x > l.y {
                p = Vec2::new(p.y, p.x);
                l = Vec2::new(l.y, l.x);
            }
        }

        p -= 0.5;
        let id = self.hash21(ip + l);
        p = self.rot((id - 0.5) * rotation) * p;

        let th = l * 0.02 * gap;
        let c = s_box(p, Vec2::broadcast(0.5) - th, rounding);
        (c, id)
    }

    /// Get the dominant node color for sector previews
    pub fn get_dominant_color(&self, palette: &ThePalette) -> Pixel {
        match self.role {
            Gradient => self.get_palette_color("interior", palette),
            _ => self.get_palette_color("color", palette),
        }
    }

    /// Get the color of a given name from the values.
    pub fn get_palette_color(&self, named: &str, palette: &ThePalette) -> Pixel {
        let mut color = BLACK;
        let index = self.values.get_int_default(named, 0);
        if let Some(Some(col)) = palette.colors.get(index as usize) {
            color = col.to_u8_array();
        }
        color
    }
}
