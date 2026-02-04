use crate::{
    Assets, BLACK, D2PreviewBuilder, Map, PixelSource, Rasterizer, ShapeFXParam, Value,
    ValueContainer,
};
// use rand::rngs::StdRng;
// use rand::{Rng, SeedableRng};
use theframework::prelude::*;
use vek::Vec2;

#[derive(Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeType {
    #[default]
    Circle,
    Star,
    Bricks,
}

impl From<i32> for ShapeType {
    fn from(value: i32) -> Self {
        match value {
            1 => ShapeType::Star,
            2 => ShapeType::Bricks,
            _ => ShapeType::Circle,
        }
    }
}

impl std::fmt::Display for ShapeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ShapeType::Circle => "Circle",
            ShapeType::Star => "Star",
            ShapeType::Bricks => "Bricks",
        };
        write!(f, "{}", name)
    }
}

#[derive(Clone)]
pub struct Shape {
    pub shape_type: ShapeType,
    pub values: ValueContainer,
}

impl Default for Shape {
    fn default() -> Self {
        Self::new(ShapeType::Circle)
    }
}

impl Shape {
    pub fn new(shape_type: ShapeType) -> Self {
        Self {
            shape_type,
            values: ValueContainer::default(),
        }
    }

    /*
    pub fn new_with_type(shape: ShapeType) -> Self {
        match shape {
            ShapeType::Circle => Self {
                shape,
                subdivisions: 10,
                size: Vec2::new(0.5, 0.5),
                rotation: 0.0,
                randomness: 0.0,
                seed: None,
                center: Vec2::zero(),
                content_size: Vec2::zero(),
                spacing: Vec2::zero(),
            },
            ShapeType::Star => Self {
                shape,
                subdivisions: 4,
                size: Vec2::new(0.5, 0.5),
                rotation: 0.0,
                randomness: 0.0,
                seed: None,
                center: Vec2::zero(),
                content_size: Vec2::zero(),
                spacing: Vec2::zero(),
            },
            ShapeType::Bricks => Self {
                shape,
                subdivisions: 4,
                size: Vec2::new(5.0, 5.0),
                rotation: 0.0,
                randomness: 0.0,
                seed: None,
                center: Vec2::zero(),
                content_size: Vec2::new(4.0, 2.0),
                spacing: Vec2::new(0.3, 0.3),
            },
        }
    }*/

    /// Create the shape and returns the id of the new sector.
    pub fn create(
        &self,
        map: &mut Map,
        pixel_center: Option<Vec2<f32>>,
        pixel_size: Option<Vec2<f32>>,
    ) -> Vec<u32> {
        let mut vertex_ids = vec![];
        let mut linedef_ids = vec![];
        let mut sector_ids = vec![];

        let center = if let Some(pixel_center) = pixel_center {
            pixel_center
        } else {
            Vec2::zero()
        };

        let size = if let Some(pixel_size) = pixel_size {
            pixel_size
        } else {
            Vec2::broadcast(self.values.get_float_default("size", 4.0))
        };

        /*
        let mut rng: StdRng = match self.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::seed_from_u64(rand::rng().random()),
        };*/

        match self.shape_type {
            ShapeType::Circle => {
                let count = self.values.get_int_default("subdivisions", 10);
                let angle_step = std::f32::consts::TAU / count as f32;

                for i in 0..count {
                    let angle = i as f32 * angle_step; // + self.rotation;
                    let x = center.x + size.x * angle.cos();
                    let y = center.y + size.y * angle.sin();

                    // if self.randomness > 0.0 {
                    //     x += rng.random_range(-self.randomness..=self.randomness);
                    //     y += rng.random_range(-self.randomness..=self.randomness);
                    // }

                    let vid = map.add_vertex_at(x, y);
                    vertex_ids.push(vid);
                }

                for i in 0..count {
                    let start = vertex_ids[i as usize];
                    let end = vertex_ids[((i + 1) % count) as usize];
                    let (linedef_id, sector_id) = map.create_linedef(start, end);
                    if let Some(sector_id) = sector_id {
                        sector_ids.push(sector_id);
                    }
                    linedef_ids.push(linedef_id);
                }
            }

            ShapeType::Star => {
                let points = self.values.get_int_default("subdivisions", 4);
                let total = points * 2; // outer + inner alternating
                let angle_step = std::f32::consts::TAU / total as f32;
                let inner_radius = size * 0.5;

                for i in 0..total {
                    let is_outer = i % 2 == 0;
                    let radius = if is_outer { size } else { inner_radius };
                    let angle = i as f32 * angle_step; // + self.rotation;

                    let x = center.x + radius.x * angle.cos();
                    let y = center.y + radius.y * angle.sin();

                    // if self.randomness > 0.0 {
                    //     x += rng.random_range(-self.randomness..=self.randomness);
                    //     y += rng.random_range(-self.randomness..=self.randomness);
                    // }

                    let vid = map.add_vertex_at(x, y);
                    vertex_ids.push(vid);
                }

                for i in 0..total {
                    let start = vertex_ids[i as usize];
                    let end = vertex_ids[((i + 1) % total) as usize];
                    let (linedef_id, sector_id) = map.create_linedef(start, end);
                    if let Some(sector_id) = sector_id {
                        sector_ids.push(sector_id);
                    }
                    linedef_ids.push(linedef_id);
                }
            }
            _ => {} /*
                    ShapeType::Bricks => {
                        let mut y = self.center.y - self.size.y / 2.0;

                        let mut counter = 0;
                        while y < self.center.y + self.size.y / 2.0 {
                            let mut x = self.center.x - self.size.x / 2.0;
                            let offset = if counter % 2 == 1 {
                                //((y / (self.content_size.y + self.spacing.y)).floor() as i32) % 2 == 1 {
                                -self.content_size.x / 2.0
                            } else {
                                0.0
                            };

                            x += offset;

                            while x < self.center.x + self.size.x / 2.0 {
                                // Calculate corners of the brick
                                let tl = Vec2::new(x, y);
                                let tr = Vec2::new(x + self.content_size.x, y);
                                let br = Vec2::new(x + self.content_size.x, y + self.content_size.y);
                                let bl = Vec2::new(x, y + self.content_size.y);

                                // Optionally add randomness here per brick corner

                                let v0 = map.add_vertex_at(tl.x, tl.y);
                                let v1 = map.add_vertex_at(tr.x, tr.y);
                                let v2 = map.add_vertex_at(br.x, br.y);
                                let v3 = map.add_vertex_at(bl.x, bl.y);

                                let (l0, _) = map.create_linedef(v0, v1);
                                let (l1, _) = map.create_linedef(v1, v2);
                                let (l2, _) = map.create_linedef(v2, v3);
                                let (l3, sector_id) = map.create_linedef(v3, v0);
                                if let Some(sector_id) = sector_id {
                                    sector_ids.push(sector_id);
                                }
                                vertex_ids.extend([v0, v1, v2, v3]);
                                linedef_ids.extend([l0, l1, l2, l3]);

                                x += self.content_size.x + self.spacing.x;
                            }

                            y += self.content_size.y + self.spacing.y;
                            counter += 1;
                        }
                    }*/
        }

        sector_ids
    }

    pub fn params(&self) -> Vec<ShapeFXParam> {
        let mut params = vec![
            // ShapeFXParam::Selector(
            //     "shape".into(),
            //     "Shape".into(),
            //     "The shape type to generate.".into(),
            //     vec!["Circle".into(), "Star".into(), "Bricks".into()],
            //     self.shape_type as i32,
            // ),
            // ShapeFXParam::Float(
            //     "randomness".into(),
            //     "Randomness".into(),
            //     "Amount of position jitter.".into(),
            //     self.randomness,
            // ),
            // ShapeFXParam::Int(
            //     "seed".into(),
            //     "Seed".into(),
            //     "Random seed (0 for random).".into(),
            //     self.seed.unwrap_or(0) as i32,
            // ),
        ];

        match self.shape_type {
            ShapeType::Circle | ShapeType::Star => {
                params.push(ShapeFXParam::Int(
                    "subdivisions".into(),
                    "Subdivisions".into(),
                    "Number of points/segments.".into(),
                    self.values.get_int_default("subdivisions", 10),
                    3..=100,
                ));
                params.push(ShapeFXParam::Float(
                    "size".into(),
                    "Size".into(),
                    "Size of the shape.".into(),
                    self.values.get_float_default("size", 3.0),
                    0.5..=50.0,
                ));
            }
            _ => {}
        }

        params
    }

    pub fn preview(&mut self, buffer: &mut TheRGBABuffer, assets: &Assets) {
        buffer.fill(BLACK);
        let width = buffer.dim().width as f32;
        let height = buffer.dim().height as f32;

        let mut map = Map::default();
        let center = Vec2::new(width / 2.0, height / 2.0);
        let size = Vec2::new(width / 2.2, height / 2.2);

        let ids = self.create(&mut map, Some(center), Some(size));
        for sector_id in &ids {
            if let Some(sector) = map.find_sector_mut(*sector_id) {
                sector.properties.set(
                    "source",
                    Value::Source(PixelSource::Color(TheColor::white())),
                );
            }

            let builder = D2PreviewBuilder::default();
            // let mut scene = builder.build(
            //     &map,
            //     assets,
            //     Vec2::new(width, height),
            //     &ValueContainer::default(),
            // );
            let mut scene = crate::Scene::default();
            builder.build_linedefs_cpu(&map, &mut scene, Vec2::new(width, height));
            scene.background = None;

            let mut rast = Rasterizer::setup(None, Mat4::identity(), Mat4::identity());
            rast.rasterize(
                &mut scene,
                buffer.pixels_mut(),
                width as usize,
                height as usize,
                40,
                assets,
            );
        }
    }
}
