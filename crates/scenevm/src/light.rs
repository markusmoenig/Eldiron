use vek::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightType {
    Point,
}

#[derive(Debug, Clone)]
pub struct Light {
    pub light_type: LightType,
    pub position: Vec3<f32>,
    pub color: Vec3<f32>,
    pub intensity: f32,
    pub radius: f32,
    pub emitting: bool,
    pub start_distance: f32,
    pub end_distance: f32,
    pub flicker: f32,
}

impl Light {
    pub fn new_pointlight(position: Vec3<f32>) -> Self {
        Light {
            light_type: LightType::Point,
            position,
            color: Vec3::new(1.0, 1.0, 1.0),
            intensity: 100.0,
            radius: 10.0,
            emitting: true,
            start_distance: 0.0,
            end_distance: 10.0,
            flicker: 0.0,
        }
    }

    pub fn with_position(mut self, position: Vec3<f32>) -> Self {
        self.position = position;
        self
    }
    pub fn with_color(mut self, color: Vec3<f32>) -> Self {
        self.color = color;
        self
    }
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
    pub fn with_emitting(mut self, emitting: bool) -> Self {
        self.emitting = emitting;
        self
    }
    pub fn with_start_distance(mut self, start_distance: f32) -> Self {
        self.start_distance = start_distance;
        self
    }
    pub fn with_end_distance(mut self, end_distance: f32) -> Self {
        self.end_distance = end_distance;
        self
    }
    pub fn with_flicker(mut self, flicker: f32) -> Self {
        self.flicker = flicker;
        self
    }
}

/// Smooth torch flicker shared by CPU lighting and GPU scene preparation.
/// Both intensity and range vary around their authored values.
pub fn light_flicker_multipliers(light: &Light, animation_counter: usize) -> (f32, f32) {
    let amount = light.flicker.clamp(0.0, 1.0);
    if amount <= 0.0 {
        return (1.0, 1.0);
    }

    let position_hash = flicker_hash(
        light.position.x.to_bits()
            ^ light.position.y.to_bits().rotate_left(11)
            ^ light.position.z.to_bits().rotate_left(22),
    );
    let phase = position_hash as f32 / u32::MAX as f32 * std::f32::consts::TAU;
    let time = animation_counter as f32 * 0.12;
    let wave = (time + phase).sin() * 0.55
        + (time * 1.83 + phase * 1.37).sin() * 0.28
        + (time * 0.41 + phase * 2.11).sin() * 0.17;

    let intensity = (1.0 + wave * amount * 0.70).max(0.05);
    let range = (1.0 + wave * amount * 0.22).max(0.25);
    (intensity, range)
}

fn flicker_hash(mut state: u32) -> u32 {
    state = (state ^ 61) ^ (state >> 16);
    state = state.wrapping_add(state << 3);
    state ^= state >> 4;
    state = state.wrapping_mul(0x27d4eb2d);
    state ^= state >> 15;
    state
}

#[cfg(test)]
mod cpu_tests {
    use super::*;
    use vek::Vec3;
    #[test]
    fn light_flicker_changes_intensity_and_range_around_authored_values() {
        let light = Light::new_pointlight(Vec3::new(2.0, 1.0, -3.0)).with_flicker(0.4);
        let samples = (0..256)
            .map(|frame| light_flicker_multipliers(&light, frame))
            .collect::<Vec<_>>();
        let min_intensity = samples
            .iter()
            .map(|sample| sample.0)
            .fold(f32::INFINITY, f32::min);
        let max_intensity = samples
            .iter()
            .map(|sample| sample.0)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_range = samples
            .iter()
            .map(|sample| sample.1)
            .fold(f32::INFINITY, f32::min);
        let max_range = samples
            .iter()
            .map(|sample| sample.1)
            .fold(f32::NEG_INFINITY, f32::max);

        assert!(min_intensity < 0.9 && max_intensity > 1.1);
        assert!(min_range < 0.98 && max_range > 1.02);
    }
}
