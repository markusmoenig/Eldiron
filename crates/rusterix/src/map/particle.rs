use rand::Rng;
use theframework::prelude::*;
use vek::{Mat3, Vec3};

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Particle {
    pub pos: Vec3<f32>,
    pub vel: Vec3<f32>,
    pub lifetime: f32,
    pub radius: f32,
    pub color: [u8; 4],
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ParticleEmitter {
    pub origin: Vec3<f32>,
    pub direction: Vec3<f32>, // Preferred direction (normalized)
    pub spread: f32,          // Angle in radians (0 = tight beam, PI = full sphere)
    pub rate: f32,            // Particles per second
    pub time_accum: f32,

    pub color: [u8; 4],      // Base color
    pub color_variation: u8, // +/- variation for flicker

    pub lifetime_range: (f32, f32), // Seconds
    pub radius_range: (f32, f32),   // Radius size range
    pub speed_range: (f32, f32),    // Velocity magnitude range

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
            color_variation: 30,

            lifetime_range: (0.5, 1.5),
            radius_range: (0.05, 0.15),
            speed_range: (0.5, 1.5),

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
                p.radius *= 0.98;
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

        let mut color = self.color;
        for i in 0..3 {
            let v = rng.random_range(
                (color[i] as i16 - self.color_variation as i16).max(0)
                    ..=(color[i] as i16 + self.color_variation as i16).min(255),
            );
            color[i] = v as u8;
        }

        let p = Particle {
            pos: self.origin,
            vel: velocity,
            lifetime,
            radius,
            color,
        };

        self.particles.push(p);
    }
}

/// Generates a random unit vector within a cone defined by direction and spread.
fn random_unit_vector_in_cone(dir: Vec3<f32>, spread: f32) -> Vec3<f32> {
    let mut rng = rand::rng();

    // Generate a random direction in spherical coordinates
    let theta = rng.random_range(0.0..std::f32::consts::TAU);
    let phi = rng.random_range(0.0..spread);

    // Local vector
    let x = phi.sin() * theta.cos();
    let y = phi.sin() * theta.sin();
    let z = phi.cos();
    let local = Vec3::new(x, y, z);

    // Rotate local cone direction to align with `dir`
    align_vector(local, dir)
}

/// Rotates vector `v` to align with the target direction.
fn align_vector(v: Vec3<f32>, target: Vec3<f32>) -> Vec3<f32> {
    let from = Vec3::unit_z(); // Local cone direction
    let to = target.normalized();

    let cos_theta = from.dot(to);
    if cos_theta > 0.9999 {
        return v; // Already aligned
    } else if cos_theta < -0.9999 {
        // 180 degree flip — any perpendicular axis works
        let up = Vec3::unit_y();
        let axis = from.cross(up).normalized();
        let rot = rotation_matrix(axis, std::f32::consts::PI);
        return rot * v;
    }

    let axis = from.cross(to).normalized();
    let angle = cos_theta.acos();
    let rot = rotation_matrix(axis, angle);
    rot * v
}

/// Constructs a rotation matrix from an axis and angle (Rodrigues' formula).
fn rotation_matrix(axis: Vec3<f32>, angle: f32) -> Mat3<f32> {
    let (sin, cos) = angle.sin_cos();
    let one_minus_cos = 1.0 - cos;

    let x = axis.x;
    let y = axis.y;
    let z = axis.z;

    Mat3::new(
        cos + x * x * one_minus_cos,
        y * x * one_minus_cos + z * sin,
        z * x * one_minus_cos - y * sin,
        x * y * one_minus_cos - z * sin,
        cos + y * y * one_minus_cos,
        z * y * one_minus_cos + x * sin,
        x * z * one_minus_cos + y * sin,
        y * z * one_minus_cos - x * sin,
        cos + z * z * one_minus_cos,
    )
}
