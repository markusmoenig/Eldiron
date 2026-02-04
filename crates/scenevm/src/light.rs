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
