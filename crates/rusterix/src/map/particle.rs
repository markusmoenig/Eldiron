use rand::Rng;
use theframework::prelude::*;
use vek::Vec3;

const MAX_ACTIVE_PARTICLES_PER_EMITTER: usize = 2048;

/// Geometry used to distribute newly born particles around an attachment.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Copy, Default)]
pub enum ParticleEmissionShape {
    /// Emit every particle at the attachment origin.
    Point,
    /// Emit throughout the configured axis-aligned volume.
    #[default]
    Box,
    /// Emit across the configured horizontal rectangle (useful for crate tops,
    /// braziers, floor cracks and similar sources).
    Surface,
}

/// Serializable, author-facing particle settings. Runtime simulation state is
/// deliberately kept in [`ParticleEmitter`] so Prefabs never persist active
/// particles or a partially accumulated emission interval.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ParticleEmitterDef {
    pub direction: Vec3<f32>,
    pub spread: f32,
    pub rate: f32,
    pub color: [u8; 4],
    #[serde(default)]
    pub color_ramp: Option<[[u8; 4]; 4]>,
    pub color_variation: u8,
    pub lifetime_range: (f32, f32),
    pub radius_range: (f32, f32),
    pub speed_range: (f32, f32),
    #[serde(default)]
    pub spawn_area: [f32; 3],
    #[serde(default)]
    pub emission_shape: ParticleEmissionShape,
    #[serde(default)]
    pub flame_base: bool,
    /// Four normalized size multipliers sampled over particle lifetime.
    #[serde(default = "default_particle_size_curve")]
    pub size_curve: [f32; 4],
    /// Four normalized opacity multipliers sampled over particle lifetime.
    #[serde(default = "default_particle_opacity_curve")]
    pub opacity_curve: [f32; 4],
    #[serde(default)]
    pub gravity: [f32; 3],
    #[serde(default)]
    pub turbulence: f32,
}

impl ParticleEmitterDef {
    pub fn instantiate(&self, origin: Vec3<f32>, direction: Vec3<f32>) -> ParticleEmitter {
        let mut emitter = ParticleEmitter::new(origin, direction);
        emitter.spread = self.spread;
        emitter.rate = self.rate;
        emitter.color = self.color;
        emitter.color_ramp = self.color_ramp;
        emitter.color_variation = self.color_variation;
        emitter.lifetime_range = self.lifetime_range;
        emitter.radius_range = self.radius_range;
        emitter.speed_range = self.speed_range;
        emitter.spawn_area = self.spawn_area;
        emitter.emission_shape = self.emission_shape;
        emitter.flame_base = self.flame_base;
        emitter.size_curve = self.size_curve;
        emitter.opacity_curve = self.opacity_curve;
        emitter.gravity = Vec3::new(self.gravity[0], self.gravity[1], self.gravity[2]);
        emitter.turbulence = self.turbulence;
        emitter
    }
}

impl Default for ParticleEmitterDef {
    fn default() -> Self {
        Self::from(&ParticleEmitter::new(Vec3::zero(), Vec3::unit_y()))
    }
}

impl From<&ParticleEmitter> for ParticleEmitterDef {
    fn from(emitter: &ParticleEmitter) -> Self {
        Self {
            direction: emitter.direction,
            spread: emitter.spread,
            rate: emitter.rate,
            color: emitter.color,
            color_ramp: emitter.color_ramp,
            color_variation: emitter.color_variation,
            lifetime_range: emitter.lifetime_range,
            radius_range: emitter.radius_range,
            speed_range: emitter.speed_range,
            spawn_area: emitter.spawn_area,
            emission_shape: emitter.emission_shape,
            flame_base: emitter.flame_base,
            size_curve: emitter.size_curve,
            opacity_curve: emitter.opacity_curve,
            gravity: [emitter.gravity.x, emitter.gravity.y, emitter.gravity.z],
            turbulence: emitter.turbulence,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Particle {
    pub pos: Vec3<f32>,
    pub vel: Vec3<f32>,
    pub lifetime: f32,
    pub initial_lifetime: f32,
    pub radius: f32,
    pub initial_radius: f32,
    pub color: [u8; 4],
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ParticleEmitter {
    pub origin: Vec3<f32>,
    pub direction: Vec3<f32>, // Preferred direction (normalized)
    pub spread: f32,          // Angle in radians (0 = tight beam, PI = full sphere)
    pub rate: f32,            // Particles per second
    pub time_accum: f32,

    pub color: [u8; 4], // Base color
    #[serde(default)]
    pub color_ramp: Option<[[u8; 4]; 4]>,
    pub color_variation: u8, // +/- variation for flicker

    pub lifetime_range: (f32, f32), // Seconds
    pub radius_range: (f32, f32),   // Radius size range
    pub speed_range: (f32, f32),    // Velocity magnitude range
    #[serde(default)]
    pub spawn_area: [f32; 3], // Random +/- spawn offset per axis before velocity is applied.
    #[serde(default)]
    pub emission_shape: ParticleEmissionShape,
    #[serde(default)]
    pub flame_base: bool,
    #[serde(default = "default_particle_size_curve")]
    pub size_curve: [f32; 4],
    #[serde(default = "default_particle_opacity_curve")]
    pub opacity_curve: [f32; 4],
    #[serde(default)]
    pub gravity: Vec3<f32>,
    #[serde(default)]
    pub turbulence: f32,

    /// Optional world-space triangles used by face-authored emitters. This is runtime-only: the
    /// persistent face effect stores authoring settings and rebuilds these triangles from current
    /// geometry, so procedural regeneration and object transforms remain authoritative.
    #[serde(skip)]
    pub spawn_surface_triangles: Vec<[Vec3<f32>; 3]>,

    pub particles: Vec<Particle>, // Active particles
}

impl ParticleEmitter {
    /// Creates a new ParticleEmitter with default parameters.
    pub fn new(origin: Vec3<f32>, direction: Vec3<f32>) -> Self {
        Self {
            origin,
            direction: direction.normalized(),
            spread: std::f32::consts::FRAC_PI_4, // 45° cone by default
            rate: 30.0,
            time_accum: 0.0,

            color: [255, 160, 0, 255],
            color_ramp: None,
            color_variation: 30,

            lifetime_range: (0.5, 1.5),
            radius_range: (0.05, 0.15),
            speed_range: (0.5, 1.5),
            spawn_area: [0.0, 0.0, 0.0],
            emission_shape: ParticleEmissionShape::default(),
            flame_base: false,
            size_curve: default_particle_size_curve(),
            opacity_curve: default_particle_opacity_curve(),
            gravity: Vec3::zero(),
            turbulence: 0.0,
            spawn_surface_triangles: vec![],

            particles: vec![],
        }
    }

    /// Refresh authorable settings while preserving active particles and the
    /// fractional emission clock. Editor previews use this when a Prefab
    /// effect is adjusted live.
    pub fn sync_settings_from(&mut self, source: &ParticleEmitter) {
        self.spread = source.spread;
        self.rate = source.rate;
        self.color = source.color;
        self.color_ramp = source.color_ramp;
        self.color_variation = source.color_variation;
        self.lifetime_range = source.lifetime_range;
        self.radius_range = source.radius_range;
        self.speed_range = source.speed_range;
        self.spawn_area = source.spawn_area;
        self.emission_shape = source.emission_shape;
        self.flame_base = source.flame_base;
        self.size_curve = source.size_curve;
        self.opacity_curve = source.opacity_curve;
        self.gravity = source.gravity;
        self.turbulence = source.turbulence;
        self.spawn_surface_triangles = source.spawn_surface_triangles.clone();
    }

    /// Updates the emitter and its particles over time.
    pub fn update(&mut self, dt: f32) {
        self.time_accum += dt;

        let emit_count = (self.rate * self.time_accum).floor() as usize;
        if emit_count > 0 {
            self.time_accum -= emit_count as f32 / self.rate;
            let available = MAX_ACTIVE_PARTICLES_PER_EMITTER.saturating_sub(self.particles.len());
            for _ in 0..emit_count.min(available) {
                self.emit_particle();
            }
        }

        self.particles.retain_mut(|p| {
            p.lifetime -= dt;
            if p.lifetime > 0.0 {
                p.vel += self.gravity * dt;
                if self.turbulence > 0.0 {
                    let mut rng = rand::rng();
                    p.vel += Vec3::new(
                        rng.random_range(-1.0..=1.0),
                        rng.random_range(-1.0..=1.0),
                        rng.random_range(-1.0..=1.0),
                    ) * self.turbulence
                        * dt;
                }
                p.pos += p.vel * dt;
                let age = (1.0 - p.lifetime / p.initial_lifetime.max(0.001)).clamp(0.0, 1.0);
                p.radius = p.initial_radius * sample_particle_curve(self.size_curve, age).max(0.0);
                p.color = particle_color_at_age(
                    self.color_ramp
                        .as_ref()
                        .unwrap_or(&[self.color, self.color, self.color, self.color]),
                    age,
                    self.color_variation,
                );
                p.color[3] = (p.color[3] as f32
                    * sample_particle_curve(self.opacity_curve, age).clamp(0.0, 1.0))
                .round()
                .clamp(0.0, 255.0) as u8;
                true
            } else {
                false
            }
        });
    }

    /// Emits a single new particle with randomized properties.
    fn emit_particle(&mut self) {
        let mut rng = rand::rng();

        let angle_offset = random_unit_vector_in_cone(self.direction, self.spread);
        let speed = rng.random_range(self.speed_range.0..=self.speed_range.1);
        let velocity = angle_offset * speed;

        let lifetime = rng.random_range(self.lifetime_range.0..=self.lifetime_range.1);
        let radius = rng.random_range(self.radius_range.0..=self.radius_range.1);

        let color = particle_color_at_age(
            self.color_ramp
                .as_ref()
                .unwrap_or(&[self.color, self.color, self.color, self.color]),
            0.0,
            self.color_variation,
        );

        let surface_position = if self.emission_shape == ParticleEmissionShape::Surface
            && !self.spawn_surface_triangles.is_empty()
        {
            let areas = self
                .spawn_surface_triangles
                .iter()
                .map(|triangle| {
                    (triangle[1] - triangle[0])
                        .cross(triangle[2] - triangle[0])
                        .magnitude()
                        * 0.5
                })
                .collect::<Vec<_>>();
            let total_area = areas.iter().sum::<f32>();
            if total_area > 1e-8 {
                let mut selected = rng.random_range(0.0..total_area);
                let mut triangle_index = areas.len() - 1;
                for (index, area) in areas.iter().copied().enumerate() {
                    if selected <= area {
                        triangle_index = index;
                        break;
                    }
                    selected -= area;
                }
                let triangle = self.spawn_surface_triangles[triangle_index];
                let u = rng.random::<f32>().sqrt();
                let v = rng.random::<f32>();
                Some(
                    triangle[0] * (1.0 - u) + triangle[1] * (u * (1.0 - v)) + triangle[2] * (u * v),
                )
            } else {
                None
            }
        } else {
            None
        };
        let spawn_offset = match self.emission_shape {
            ParticleEmissionShape::Point => Vec3::zero(),
            ParticleEmissionShape::Box => Vec3::new(
                rng.random_range(-self.spawn_area[0]..=self.spawn_area[0]),
                rng.random_range(-self.spawn_area[1]..=self.spawn_area[1]),
                rng.random_range(-self.spawn_area[2]..=self.spawn_area[2]),
            ),
            ParticleEmissionShape::Surface => Vec3::new(
                rng.random_range(-self.spawn_area[0]..=self.spawn_area[0]),
                0.0,
                rng.random_range(-self.spawn_area[2]..=self.spawn_area[2]),
            ),
        };
        let p = Particle {
            pos: surface_position.unwrap_or(self.origin + spawn_offset),
            vel: velocity,
            lifetime,
            initial_lifetime: lifetime,
            radius,
            initial_radius: radius,
            color,
        };

        self.particles.push(p);
    }
}

fn default_particle_size_curve() -> [f32; 4] {
    [1.0, 0.92, 0.68, 0.28]
}

fn default_particle_opacity_curve() -> [f32; 4] {
    [1.0, 0.9, 0.55, 0.0]
}

fn sample_particle_curve(curve: [f32; 4], age: f32) -> f32 {
    let scaled = age.clamp(0.0, 0.999_999) * 3.0;
    let index = scaled.floor() as usize;
    let fraction = scaled.fract();
    curve[index.min(3)] * (1.0 - fraction) + curve[(index + 1).min(3)] * fraction
}

fn particle_color_at_age(ramp: &[[u8; 4]; 4], age: f32, color_variation: u8) -> [u8; 4] {
    let age = age.clamp(0.0, 0.999);
    let scaled = age * 3.0;
    let idx = scaled.floor() as usize;
    let frac = scaled.fract();
    let c0 = ramp[idx.min(3)];
    let c1 = ramp[(idx + 1).min(3)];
    let mut color = [0u8; 4];
    for channel in 0..3 {
        color[channel] =
            (c0[channel] as f32 * (1.0 - frac) + c1[channel] as f32 * frac).clamp(0.0, 255.0) as u8;
    }
    color[3] = (c0[3] as f32 * (1.0 - frac) + c1[3] as f32 * frac).clamp(0.0, 255.0) as u8;

    if color_variation == 0 {
        return color;
    }

    let mut rng = rand::rng();
    // Variation is brightness flicker, not independent RGB noise. Varying
    // channels separately made neutral smoke and fog produce green, magenta,
    // and blue particles even when every authored ramp stop was grayscale.
    let variation = color_variation as i16;
    let offset = rng.random_range(-variation..=variation);
    for channel in 0..3 {
        color[channel] = (color[channel] as i16 + offset).clamp(0, 255) as u8;
    }
    color
}

/// Generates a random unit vector within a cone defined by direction and spread.
fn random_unit_vector_in_cone(dir: Vec3<f32>, spread: f32) -> Vec3<f32> {
    let mut rng = rand::rng();
    let forward = dir.try_normalized().unwrap_or(Vec3::unit_y());
    let helper = if forward.y.abs() < 0.999 {
        Vec3::unit_y()
    } else {
        Vec3::unit_x()
    };
    let tangent = forward
        .cross(helper)
        .try_normalized()
        .unwrap_or(Vec3::unit_x());
    let bitangent = tangent
        .cross(forward)
        .try_normalized()
        .unwrap_or(Vec3::unit_z());

    let theta = rng.random_range(0.0..std::f32::consts::TAU);
    let phi = rng.random_range(0.0..spread.max(0.0));
    let radial = phi.sin();
    let axial = phi.cos();

    (forward * axial + tangent * (radial * theta.cos()) + bitangent * (radial * theta.sin()))
        .try_normalized()
        .unwrap_or(forward)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifetime_curves_gravity_and_alpha_affect_particles() {
        let mut emitter = ParticleEmitter::new(Vec3::zero(), Vec3::unit_y());
        emitter.rate = 0.0;
        emitter.gravity = Vec3::new(0.0, -2.0, 0.0);
        emitter.size_curve = [1.0, 1.0, 0.5, 0.0];
        emitter.opacity_curve = [1.0, 0.8, 0.4, 0.0];
        emitter.color_ramp = Some([
            [255, 255, 255, 255],
            [255, 255, 255, 220],
            [255, 255, 255, 120],
            [255, 255, 255, 0],
        ]);
        emitter.color_variation = 0;
        emitter.particles.push(Particle {
            pos: Vec3::zero(),
            vel: Vec3::zero(),
            lifetime: 1.0,
            initial_lifetime: 1.0,
            radius: 2.0,
            initial_radius: 2.0,
            color: [255; 4],
        });

        emitter.update(0.5);

        let particle = &emitter.particles[0];
        assert!(particle.vel.y < 0.0);
        assert!(particle.pos.y < 0.0);
        assert!(particle.radius < 2.0);
        assert!(particle.color[3] < 120);
    }
}
