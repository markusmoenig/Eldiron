use rand::Rng;
use theframework::prelude::*;
use vek::Vec3;

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
    pub flame_base: bool,

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
            flame_base: false,

            particles: vec![],
        }
    }

    /// Updates the emitter and its particles over time.
    pub fn update(&mut self, dt: f32) {
        self.time_accum += dt;

        let emit_count = (self.rate * self.time_accum).floor() as usize;
        if emit_count > 0 {
            self.time_accum -= emit_count as f32 / self.rate;
            for _ in 0..emit_count {
                self.emit_particle();
            }
        }

        self.particles.retain_mut(|p| {
            p.lifetime -= dt;
            if p.lifetime > 0.0 {
                p.pos += p.vel * dt;
                let age = (1.0 - p.lifetime / p.initial_lifetime.max(0.001)).clamp(0.0, 1.0);
                p.radius = p.initial_radius * (1.0 - age * 0.45);
                p.color = particle_color_at_age(
                    self.color_ramp
                        .as_ref()
                        .unwrap_or(&[self.color, self.color, self.color, self.color]),
                    age,
                    self.color_variation,
                );
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

        let p = Particle {
            pos: self.origin,
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
    color[3] = 255;

    if color_variation == 0 {
        return color;
    }

    let mut rng = rand::rng();
    for channel in 0..3 {
        let v = rng.random_range(
            (color[channel] as i16 - color_variation as i16).max(0)
                ..=(color[channel] as i16 + color_variation as i16).min(255),
        );
        color[channel] = v as u8;
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
