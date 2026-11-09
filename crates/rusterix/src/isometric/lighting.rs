//! Live lighting for the native iso canvas. Cached sprites contain reflectance,
//! normals and geometry, never a sun direction or a light's contribution.
use super::{
    Projection,
    surface::{Sample, ShadowGrid},
};
use crate::RenderSettings;
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::sync::OnceLock;
use vek::{Clamp, Vec2, Vec3};

pub struct Lighting {
    sun: Vec3<f32>,
    sun_color: Vec3<f32>,
    ambient: Vec3<f32>,
    points: Vec<scenevm::Light>,
    shadow_distance: f32,
    shadow_strength: f32,
    point_shadow_softness: f32,
}
impl Lighting {
    /// SceneHandler's 2D lights use display-space colors, as do RenderSettings.
    pub fn new<'a>(
        settings: &RenderSettings,
        lights: impl Iterator<Item = &'a scenevm::Light>,
        frame: usize,
    ) -> Self {
        let direction = -Vec3::from(settings.sun_direction);
        Self {
            sun: if direction.magnitude_squared() > 1e-8 {
                direction.normalized()
            } else {
                Vec3::unit_y()
            },
            sun_color: Vec3::<f32>::from(settings.sun_color).map(|c| c.max(0.).powf(2.2))
                * if settings.sun_enabled {
                    settings.sun_intensity.max(0.)
                } else {
                    0.
                },
            ambient: Vec3::<f32>::from(settings.ambient_color).map(|c| c.max(0.).powf(2.2))
                * settings.ambient_strength.max(0.),
            points: lights
                .filter(|l| l.emitting && l.intensity > 0. && l.end_distance > 0.)
                .map(|l| {
                    let mut l = l.clone();
                    let (intensity, range) = scenevm::vm::light_flicker_multipliers(&l, frame);
                    l.intensity *= intensity;
                    l.end_distance *= range;
                    l.color = l.color.map(|c| c.max(0.).powf(2.2));
                    l
                })
                .collect(),
            shadow_distance: if settings.raster_shadow_enabled {
                settings.max_shadow_distance.max(0.)
            } else {
                0.
            },
            shadow_strength: settings.raster_shadow_strength.clamp(0., 1.),
            point_shadow_softness: settings.point_shadow_softness.max(0.),
        }
    }
}
#[derive(Clone, Copy)]
pub(super) struct Caster<'a> {
    pub grid: &'a ShadowGrid,
    pub exact: bool,
    pub position: Vec3<f32>,
    pub scale: f32,
}
pub(super) struct Frame<'a> {
    pub lighting: &'a Lighting,
    pub casters: Vec<Caster<'a>>,
    pub canvas: Option<std::sync::Mutex<Vec<[u32; 2]>>>,
    pub geometry: Option<std::sync::Arc<scenevm::isometric::ShadowGeometry>>,
    sun_cells: FxHashMap<(i32, i32), Vec<usize>>,
    sun_global: Vec<usize>,
    point_casters: Vec<Vec<usize>>,
}
impl<'a> Frame<'a> {
    pub fn gpu(lighting: &'a Lighting, casters: Vec<Caster<'a>>, pixels: usize) -> Self {
        Self {
            lighting,
            casters,
            canvas: Some(std::sync::Mutex::new(vec![[0, f32::INFINITY.to_bits()]; pixels])),
            geometry: None,
            sun_cells: Default::default(),
            sun_global: Vec::new(),
            point_casters: Vec::new(),
        }
    }
    pub fn new(lighting: &'a Lighting, casters: Vec<Caster<'a>>) -> Self {
        let mut frame = Self {
            lighting,
            casters,
            canvas: None,
            geometry: None,
            sun_cells: Default::default(),
            sun_global: Vec::new(),
            point_casters: Vec::new(),
        };
        for (i, caster) in frame.casters.iter().enumerate() {
            if lighting.sun.y.abs() < 0.05 {
                frame.sun_global.push(i);
                continue;
            }
            let mut min = Vec2::broadcast(f32::INFINITY);
            let mut max = Vec2::broadcast(f32::NEG_INFINITY);
            for x in [caster.grid.min[0], caster.grid.max[0]] {
                for y in [caster.grid.min[1], caster.grid.max[1]] {
                    for z in [caster.grid.min[2], caster.grid.max[2]] {
                        let at = Vec3::new(x, y, z) * caster.scale + caster.position;
                        let q = Vec2::new(
                            at.x - at.y * lighting.sun.x / lighting.sun.y,
                            at.z - at.y * lighting.sun.z / lighting.sun.y,
                        );
                        min = min.map2(q, f32::min);
                        max = max.map2(q, f32::max);
                    }
                }
            }
            let lo = min.map(|v| v.floor() as i32);
            let hi = max.map(|v| v.floor() as i32);
            if !min.x.is_finite()
                || !min.y.is_finite()
                || !max.x.is_finite()
                || !max.y.is_finite()
                || (i64::from(hi.x) - i64::from(lo.x) + 1) * (i64::from(hi.y) - i64::from(lo.y) + 1)
                    > 4096
            {
                frame.sun_global.push(i);
                continue;
            }
            for z in lo.y..=hi.y {
                for x in lo.x..=hi.x {
                    frame.sun_cells.entry((x, z)).or_default().push(i);
                }
            }
        }
        frame.point_casters = lighting
            .points
            .iter()
            .map(|light| {
                frame
                    .casters
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| {
                        let min = Vec3::from(c.grid.min) * c.scale + c.position;
                        let max = Vec3::from(c.grid.max) * c.scale + c.position;
                        let nearest = light.position.clamped(min, max);
                        ((light.position - nearest).magnitude_squared()
                            <= light.end_distance * light.end_distance)
                            .then_some(i)
                    })
                    .collect()
            })
            .collect();
        frame
    }
    fn blocked(
        &self,
        origin: Vec3<f32>,
        direction: Vec3<f32>,
        distance: f32,
        light: usize,
    ) -> bool {
        let blocks = |c: &Caster<'_>| {
            if light > 0 && c.grid.emissive && c.contains(self.lighting.points[light - 1].position)
            {
                return false;
            }
            c.blocks(origin, direction, distance)
        };
        if light == 0 && self.lighting.sun.y.abs() >= 0.05 {
            let q = Vec2::new(
                origin.x - origin.y * direction.x / direction.y,
                origin.z - origin.y * direction.z / direction.y,
            );
            self.sun_cells
                .get(&(q.x.floor() as i32, q.y.floor() as i32))
                .is_some_and(|indices| indices.iter().any(|i| blocks(&self.casters[*i])))
                || self.sun_global.iter().any(|i| blocks(&self.casters[*i]))
        } else if light > 0 {
            self.point_casters[light - 1]
                .iter()
                .any(|i| blocks(&self.casters[*i]))
        } else {
            self.casters.iter().any(blocks)
        }
    }
}
pub(super) struct Shading<'a, 'b> {
    frame: &'a Frame<'b>,
    visibility: FxHashMap<([i32; 3], usize), bool>,
}
impl<'a, 'b> Shading<'a, 'b> {
    pub fn new(frame: &'a Frame<'b>) -> Self {
        Self {
            frame,
            visibility: Default::default(),
        }
    }
    fn visible(
        &mut self,
        point: Vec3<f32>,
        normal: Vec3<f32>,
        direction: Vec3<f32>,
        distance: f32,
        light: usize,
    ) -> f32 {
        let settings = self.frame.lighting;
        if settings.shadow_distance <= 0. {
            return 1.;
        }
        let origin = point + normal * 0.045 + direction * 0.025;
        let key = (
            [
                (origin.x * 16.).floor() as i32,
                (origin.y * 16.).floor() as i32,
                (origin.z * 16.).floor() as i32,
            ],
            light,
        );
        let blocked = *self.visibility.entry(key).or_insert_with(|| {
            self.frame.blocked(
                origin,
                direction,
                distance.min(settings.shadow_distance),
                light,
            )
        });
        if blocked {
            1. - settings.shadow_strength
        } else {
            1.
        }
    }
    pub fn color(&mut self, color: [u8; 4], point: Vec3<f32>, sample: Sample) -> [u8; 4] {
        let (illumination, specular) = self.irradiance(point, sample);
        response(color, illumination, specular, sample.emission)
    }
    fn irradiance(&mut self, point: Vec3<f32>, sample: Sample) -> (Vec3<f32>, Vec3<f32>) {
        let l = self.frame.lighting;
        let n = sample.normal;
        let mut illumination = l.ambient;
        let sun = n.dot(l.sun).max(0.);
        if sun > 0. && l.sun_color.magnitude_squared() > 1e-8 {
            illumination += l.sun_color * sun * self.visible(point, n, l.sun, l.shadow_distance, 0);
        }
        // A small material-dependent highlight preserves metallic/roughness controls.
        let view = Vec3::new(-0.61237246, 0.5, 0.61237246);
        let mut specular = Vec3::zero();
        for (i, light) in l.points.iter().enumerate() {
            let delta = light.position - point;
            let d2 = delta.magnitude_squared().max(0.01);
            let distance = d2.sqrt();
            if distance >= light.end_distance {
                continue;
            }
            let direction = delta / distance;
            let diffuse = n.dot(direction).max(0.);
            if diffuse <= 0. {
                continue;
            }
            let fall = ((light.end_distance - distance)
                / (light.end_distance - light.start_distance).max(0.001))
            .clamp(0., 1.);
            let radiance = light.color
                * (light.intensity * fall / d2)
                * self.visible(point, n, direction, distance, i + 1);
            illumination += radiance * diffuse;
            let h = direction + view;
            if h.magnitude_squared() > 1e-6 {
                let power = 2. + (1. - sample.roughness) * 62.;
                specular += radiance
                    * n.dot(h.normalized()).max(0.).powf(power)
                    * (0.04 + sample.metallic * 0.6)
                    * (1. - sample.roughness * 0.75);
            }
        }
        (illumination, specular)
    }
}
fn response(
    color: [u8; 4],
    illumination: Vec3<f32>,
    specular: Vec3<f32>,
    emission: f32,
) -> [u8; 4] {
    let base = Vec3::new(linear(color[0]), linear(color[1]), linear(color[2]));
    let result = base * (illumination + emission) + specular;
    [
        encode(result.x),
        encode(result.y),
        encode(result.z),
        color[3],
    ]
}
fn linear(v: u8) -> f32 {
    static TABLE: OnceLock<[f32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| std::array::from_fn(|i| (i as f32 / 255.).powf(2.2)))[v as usize]
}
fn encode(v: f32) -> u8 {
    static TABLE: OnceLock<Vec<u8>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        (0..8193)
            .map(|i| ((i as f32 / 8192.).powf(1. / 2.2) * 255.).round() as u8)
            .collect()
    });
    table[(v.clamp(0., 1.) * 8192.).round() as usize]
}
pub(super) fn ground(
    frame: &Frame<'_>,
    buffer: &mut theframework::prelude::TheRGBABuffer,
    p: Projection,
    geometry: &[[f32; 5]],
    source_width: usize,
    offset: Vec2<usize>,
) {
    let width = buffer.dim().width.max(1) as usize;
    if let Some(canvas) = &frame.canvas {
        let mut canvas = canvas.lock().unwrap();
        canvas
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, pixel) in row.iter_mut().enumerate() {
                    if let Some(hit) = geometry
                        .get((y + offset.y) * source_width + x + offset.x)
                        .filter(|p| p[0].is_finite())
                    {
                        *pixel = super::surface::packed(
                            Vec3::new(hit[2], hit[3], hit[4]),
                            hit[1],
                            0.95,
                            0.,
                            0.,
                        );
                    }
                }
            });
        return;
    }

    buffer
        .pixels_mut()
        .par_chunks_mut(width * 4 * 16)
        .enumerate()
        .for_each(|(band, pixels)| {
            let mut shading = Shading::new(frame);
            let height = pixels.len() / (width * 4);
            let sample = |hit: &[f32; 5]| Sample {
                normal: Vec3::new(hit[2], hit[3], hit[4]),
                height: hit[1],
                roughness: 0.95,
                metallic: 0.,
                emission: 0.,
            };
            let position = |x: usize, y: usize, hit: &[f32; 5]| {
                let at = p.ground(Vec2::new(
                    x as f32 + 0.5,
                    y as f32 + 0.5 + hit[1] * p.unit * 1.2247449,
                ));
                Vec3::new(at.x, hit[1], at.y)
            };
            // Irradiance varies much more slowly than albedo. Evaluate it on 2x2
            // screen blocks, retaining every material pixel. Height/normal edges
            // fall back to their own samples rather than leaking across a cliff.
            for y in (0..height).step_by(2) {
                for x in (0..width).step_by(2) {
                    let world_y = band * 16 + y;
                    let Some(anchor) =
                        geometry.get((world_y + offset.y) * source_width + x + offset.x)
                    else {
                        continue;
                    };
                    let shared = anchor[0]
                        .is_finite()
                        .then(|| shading.irradiance(position(x, world_y, anchor), sample(anchor)));
                    for dy in 0..2 {
                        for dx in 0..2 {
                            if x + dx >= width || y + dy >= height {
                                continue;
                            }
                            let Some(hit) = geometry
                                .get((world_y + dy + offset.y) * source_width + x + dx + offset.x)
                            else {
                                continue;
                            };
                            if !hit[0].is_finite() {
                                continue;
                            }
                            let (illumination, specular) = if let Some(radiance) =
                                shared.filter(|_| {
                                    (hit[1] - anchor[1]).abs() < 0.1
                                        && (Vec3::new(hit[2], hit[3], hit[4])
                                            - Vec3::new(anchor[2], anchor[3], anchor[4]))
                                        .magnitude_squared()
                                            < 0.001
                                }) {
                                radiance
                            } else {
                                shading.irradiance(position(x + dx, world_y + dy, hit), sample(hit))
                            };
                            let at = ((y + dy) * width + x + dx) * 4;
                            let color = response(
                                pixels[at..at + 4].try_into().unwrap(),
                                illumination,
                                specular,
                                0.,
                            );
                            pixels[at..at + 4].copy_from_slice(&color);
                        }
                    }
                }
            }
        });
}

impl Caster<'_> {
    fn contains(&self, point: Vec3<f32>) -> bool {
        let point = (point - self.position) / self.scale.max(0.001);
        (0..3).all(|i| point[i] >= self.grid.min[i] - 0.01 && point[i] <= self.grid.max[i] + 0.01)
    }

    // Traverse the alpha-tested mesh's X/Z columns, clipping the ray's height
    // against each occupied interval. Unlike a painted oval this moves with lights.
    fn blocks(&self, origin: Vec3<f32>, direction: Vec3<f32>, distance: f32) -> bool {
        let g = self.grid;
        if g.width == 0 || g.height == 0 || g.width > 1024 || g.height > 1024 || self.scale <= 0. {
            return false;
        }
        let o = (origin - self.position) / self.scale;
        let d = direction / self.scale;
        let min = Vec3::from(g.min);
        let max = Vec3::from(g.max);
        let mut begin = 0.001f32;
        let mut end = distance;
        for axis in 0..3 {
            if d[axis].abs() < 1e-8 {
                if o[axis] < min[axis] || o[axis] > max[axis] {
                    return false;
                }
            } else {
                let a = (min[axis] - o[axis]) / d[axis];
                let b = (max[axis] - o[axis]) / d[axis];
                begin = begin.max(a.min(b));
                end = end.min(a.max(b));
            }
        }
        if begin >= end {
            return false;
        }
        let size = Vec2::new(
            (max.x - min.x) / g.width as f32,
            (max.z - min.z) / g.height as f32,
        );
        if size.x <= 0. || size.y <= 0. {
            return false;
        }
        let entry = o + d * (begin + 1e-5);
        let mut cell = [
            ((entry.x - min.x) / size.x).floor() as i32,
            ((entry.z - min.z) / size.y).floor() as i32,
        ];
        let axes = [0usize, 2];
        let mut next = [f32::INFINITY; 2];
        let mut delta = [f32::INFINITY; 2];
        let mut step = [0i32; 2];
        for i in 0..2 {
            let axis = axes[i];
            if d[axis].abs() > 1e-8 {
                step[i] = if d[axis] > 0. { 1 } else { -1 };
                let edge = cell[i] + if step[i] > 0 { 1 } else { 0 };
                next[i] = (min[axis] + edge as f32 * size[i] - o[axis]) / d[axis];
                delta[i] = size[i] / d[axis].abs();
            }
        }
        for _ in 0..g.width + g.height + 2 {
            if cell[0] < 0
                || cell[1] < 0
                || cell[0] >= g.width as i32
                || cell[1] >= g.height as i32
                || begin > end
            {
                break;
            }
            let stop = next[0].min(next[1]).min(end);
            for layer in 0..g.layers.min(8) {
                if let Some([lo, hi]) = g.interval_layer(cell[0] as usize, cell[1] as usize, layer)
                {
                    let a = o.y + d.y * begin;
                    let b = o.y + d.y * stop;
                    if a.min(b) <= hi + 0.008 && a.max(b) >= lo - 0.008 {
                        return true;
                    }
                }
            }
            if stop >= end {
                break;
            }
            let axis = if next[0] < next[1] { 0 } else { 1 };
            begin = next[axis];
            next[axis] += delta[axis];
            cell[axis] += step[axis];
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(normal: Vec3<f32>) -> Sample {
        Sample {
            normal,
            height: 0.,
            roughness: 1.,
            metallic: 0.,
            emission: 0.,
        }
    }
    fn shade(lighting: &Lighting, point: Vec3<f32>, normal: Vec3<f32>) -> [u8; 4] {
        let frame = Frame::new(lighting, vec![]);
        Shading::new(&frame).color([160, 140, 100, 255], point, sample(normal))
    }
    #[test]
    fn sun_direction_enable_and_simulation_update_existing_surfaces() {
        let mut settings = RenderSettings::default();
        settings.ambient_strength = 0.;
        settings.sun_color = [1.; 3];
        settings.sun_direction = [-1., 0., 0.];
        let light = Lighting::new(&settings, std::iter::empty(), 0);
        assert!(shade(&light, Vec3::zero(), Vec3::unit_x())[0] > 150);
        assert_eq!(shade(&light, Vec3::zero(), -Vec3::unit_x())[0], 0);
        settings.sun_enabled = false;
        let disabled = Lighting::new(&settings, std::iter::empty(), 0);
        assert_eq!(shade(&disabled, Vec3::zero(), Vec3::unit_x())[0], 0);
        settings.sun_enabled = true;
        settings.simulation.enabled = true;
        settings.apply_hour(12.);
        let noon = shade(
            &Lighting::new(&settings, std::iter::empty(), 0),
            Vec3::zero(),
            Vec3::unit_y(),
        );
        settings.apply_hour(23.);
        let night = shade(
            &Lighting::new(&settings, std::iter::empty(), 0),
            Vec3::zero(),
            Vec3::unit_y(),
        );
        assert!(noon[0] > night[0] + 100);
    }
    #[test]
    fn point_lights_move_change_color_switch_off_and_respect_surface_normals() {
        let mut settings = RenderSettings::default();
        settings.sun_enabled = false;
        settings.ambient_strength = 0.;
        let mut point = scenevm::Light::new_pointlight(Vec3::new(0., 1., 0.))
            .with_color(Vec3::new(1., 0., 0.))
            .with_intensity(2.)
            .with_end_distance(3.);
        let lit = shade(
            &Lighting::new(&settings, std::iter::once(&point), 0),
            Vec3::zero(),
            Vec3::unit_y(),
        );
        assert!(lit[0] > 100 && lit[1] == 0);
        assert_eq!(
            shade(
                &Lighting::new(&settings, std::iter::once(&point), 0),
                Vec3::zero(),
                -Vec3::unit_y()
            )[0],
            0
        );
        point.position.x = 5.;
        assert_eq!(
            shade(
                &Lighting::new(&settings, std::iter::once(&point), 0),
                Vec3::zero(),
                Vec3::unit_y()
            )[0],
            0
        );
        point.position.x = 0.;
        point.emitting = false;
        assert_eq!(
            shade(
                &Lighting::new(&settings, std::iter::once(&point), 0),
                Vec3::zero(),
                Vec3::unit_y()
            )[0],
            0
        );
    }
    #[test]
    fn sun_spatial_index_matches_rays_at_negative_coordinates_and_raised_surfaces() {
        let grid = ShadowGrid {
            layers: 1,
            emissive: false,
            min: [-0.5, 1., -0.5],
            max: [0.5, 2., 0.5],
            width: 1,
            height: 1,
            data: [1f32, 2f32]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
        };
        for direction in [[-0.4, -0.8, 0.3], [0.8, -0.2, -0.3], [0.3, -0.01, 0.2]] {
            let mut settings = RenderSettings::default();
            settings.sun_direction = direction;
            let lighting = Lighting::new(&settings, std::iter::empty(), 0);
            let frame = Frame::new(
                &lighting,
                vec![
                    Caster {
                        exact: false,
                        grid: &grid,
                        position: Vec3::new(-1., 0., -2.),
                        scale: 1.,
                    },
                    Caster {
                        exact: false,
                        grid: &grid,
                        position: Vec3::new(2., 1., 1.),
                        scale: 1.5,
                    },
                ],
            );
            for x in -16..16 {
                for z in -16..16 {
                    for y in [0., 0.5, 2.5] {
                        let origin = Vec3::new(x as f32 * 0.25, y, z as f32 * 0.25);
                        let brute = frame
                            .casters
                            .iter()
                            .any(|c| c.blocks(origin, lighting.sun, 10.));
                        assert_eq!(
                            frame.blocked(origin, lighting.sun, 10., 0),
                            brute,
                            "{origin:?} {direction:?}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn emissive_lantern_does_not_trap_its_own_point_light() {
        let mut settings = RenderSettings::default();
        settings.sun_enabled = false;
        settings.ambient_strength = 0.;
        settings.raster_shadow_strength = 1.;
        let lamp = scenevm::Light::new_pointlight(Vec3::new(0., 1.1, 0.)).with_intensity(2.);
        let lighting = Lighting::new(&settings, std::iter::once(&lamp), 0);
        let mut grid = ShadowGrid {
            layers: 1,
            emissive: false,
            min: [-0.5, 1., -0.5],
            max: [0.5, 1.2, 0.5],
            width: 1,
            height: 1,
            data: [1f32, 1.2f32]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
        };
        let render = |grid: &ShadowGrid| {
            let frame = Frame::new(
                &lighting,
                vec![Caster {
                        exact: false,
                    grid,
                    position: Vec3::zero(),
                    scale: 1.,
                }],
            );
            Shading::new(&frame).color([160; 4], Vec3::zero(), sample(Vec3::unit_y()))
        };
        assert_eq!(render(&grid)[0], 0);
        grid.emissive = true;
        assert!(render(&grid)[0] > 100);
    }
    #[test]
    fn geometric_shadows_follow_light_direction_and_leave_space_below_roofs() {
        let grid = ShadowGrid {
            layers: 1,
            emissive: false,
            min: [-0.5, 1., -0.5],
            max: [0.5, 1.2, 0.5],
            width: 1,
            height: 1,
            data: [1f32, 1.2f32]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
        };
        let caster = Caster {
                        exact: false,
            grid: &grid,
            position: Vec3::zero(),
            scale: 1.,
        };
        assert!(caster.blocks(Vec3::zero(), Vec3::unit_y(), 4.));
        assert!(!caster.blocks(Vec3::new(0., 0.5, 0.), Vec3::unit_x(), 4.));
        assert!(!caster.blocks(Vec3::new(1., 0., 0.), Vec3::unit_y(), 4.));
        assert!(caster.blocks(Vec3::new(1., 0., 0.), Vec3::new(-0.6, 0.8, 0.), 4.));
        let hole = ShadowGrid {
            data: [f32::INFINITY, f32::NEG_INFINITY]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect(),
            ..grid.clone()
        };
        assert!(
            !Caster {
                        exact: false,
                grid: &hole,
                ..caster
            }
            .blocks(Vec3::zero(), Vec3::unit_y(), 4.)
        );
    }
}

#[cfg(feature = "graphics")]
pub(super) fn gpu_draw(
    scene: &mut scenevm::SceneVM,
    frame: &Frame<'_>,
    buffer: &mut theframework::prelude::TheRGBABuffer,
    p: Projection,
    particles: &[scenevm::isometric::Particle],
) -> Result<(), String> {
    use scenevm::isometric::{Caster, PointLight, Uniforms};
    let l = frame.lighting;
    let mut columns = Vec::new();
    let mut casters = Vec::new();
    for c in &frame.casters {
        if frame.geometry.is_some() && c.exact { continue; }
        let g = c.grid;
        if g.width == 0
            || g.height == 0
            || g.width > 128
            || g.height > 128
            || g.layers == 0
            || g.layers > 8
            || g.data.len() != g.width * g.height * g.layers * 8
        {
            continue;
        }
        let offset = columns.len();
        columns.extend(g.data.chunks_exact(8).map(|d| {
            [
                f32::from_le_bytes(d[..4].try_into().unwrap()),
                f32::from_le_bytes(d[4..].try_into().unwrap()),
            ]
        }));
        casters.push(Caster {
            min: [g.min[0], g.min[1], g.min[2], 0.],
            max: [g.max[0], g.max[1], g.max[2], 0.],
            position_scale: [c.position.x, c.position.y, c.position.z, c.scale],
            grid: [
                g.width as u32,
                g.height as u32,
                offset as u32,
                u32::from(g.emissive) | (g.layers as u32) << 8,
            ],
        });
    }
    let lights = l
        .points
        .iter()
        .map(|v| PointLight {
            position: [v.position.x, v.position.y, v.position.z, 0.],
            color: [v.color.x, v.color.y, v.color.z, 0.],
            parameters: [v.intensity, v.end_distance, v.start_distance, l.point_shadow_softness],
        })
        .collect::<Vec<_>>();
    let u = Uniforms {
        projection: [p.origin.x, p.origin.y, p.unit, buffer.dim().width as f32],
        sun: [l.sun.x, l.sun.y, l.sun.z, 0.],
        sun_color: [l.sun_color.x, l.sun_color.y, l.sun_color.z, 0.],
        ambient: [l.ambient.x, l.ambient.y, l.ambient.z, 0.],
        counts: [
            buffer.dim().width as u32,
            buffer.dim().height as u32,
            lights.len() as u32,
            casters.len() as u32,
        ],
        shadow: [
            l.shadow_distance,
            l.shadow_strength,
            particles.len() as f32,
            if frame.geometry.is_some() { 1. } else { 0. },
        ],
    };
    scene
        .render_isometric_geometry(
            &u,
            buffer.pixels_mut(),
            &frame.canvas.as_ref().unwrap().lock().unwrap(),
            &lights,
            &casters,
            &columns,
            particles,
            frame.geometry.as_deref(),
        )
        .map_err(|e| format!("Iso GPU render: {e:?}"))?;
    Ok(())
}
