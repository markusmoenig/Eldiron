use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub struct Daylight {
    pub sunrise: i32,              // Sunrise time in minutes
    pub sunset: i32,               // Sunset time in minutes
    pub transition_duration: i32,  // Duration of the sunrise/sunset transition in minutes
    pub daylight_color: Vec3<f32>, // Color during the day
    pub sunrise_color: Vec3<f32>,  // Color at sunrise
    pub sunset_color: Vec3<f32>,   // Color at sunset
    pub night_color: Vec3<f32>,    // Color at night
}

impl Default for Daylight {
    fn default() -> Self {
        Self {
            sunrise: 300,                             // 5:00 AM
            sunset: 1200,                             // 8:00 PM
            transition_duration: 60,                  // 1 hour transition
            daylight_color: Vec3::new(0.9, 0.9, 1.0), // Slightly yellowish white
            sunrise_color: Vec3::new(1.0, 0.8, 0.8),  // Soft red
            sunset_color: Vec3::new(1.0, 0.8, 0.8),   // Soft red
            night_color: Vec3::new(0.3, 0.3, 0.3),    // Dark blue
        }
    }
}

impl Daylight {
    /// Computes the current sky color based on time, using XZ plane lighting.
    pub fn daylight(&self, time: i32, min_bright: f32, max_bright: f32) -> Vec3<f32> {
        let minutes = time;
        let transition_duration = self.transition_duration;
        let daylight_start = self.sunrise + transition_duration;
        let sunset_transition_start = self.sunset;
        let sunset_transition_end = self.sunset + transition_duration;

        let color = if minutes < self.sunrise || minutes > sunset_transition_end {
            self.night_color
        } else if minutes < daylight_start {
            Vec3::lerp(
                self.night_color,
                self.sunrise_color,
                (minutes - self.sunrise) as f32 / transition_duration as f32,
            )
        } else if minutes < self.sunset {
            self.daylight_color
        } else {
            Vec3::lerp(
                self.sunset_color,
                self.night_color,
                (minutes - sunset_transition_start) as f32 / transition_duration as f32,
            )
        };

        // Apply brightness clamping
        color.clamped(
            Vec3::new(min_bright, min_bright, min_bright),
            Vec3::new(max_bright, max_bright, max_bright),
        )
    }

    /// Returns the intensity of daylight at a given time (0.0 = night, 1.0 = full daylight).
    pub fn daylight_intensity(&self, time: i32) -> f32 {
        let minutes = time;
        let transition_duration = self.transition_duration;
        let daylight_start = self.sunrise + transition_duration;
        let sunset_transition_start = self.sunset;
        let sunset_transition_end = self.sunset + transition_duration;

        match minutes {
            _ if minutes < self.sunrise || minutes > sunset_transition_end => 0.0,
            _ if minutes < daylight_start => {
                (minutes - self.sunrise) as f32 / transition_duration as f32
            }
            _ if minutes < self.sunset => 1.0,
            _ => 1.0 - (minutes - sunset_transition_start) as f32 / transition_duration as f32,
        }
    }

    /// Computes the direction of the sun in the XZ plane based on the time of day.
    pub fn calculate_light_direction(&self, time: i32) -> Vec3<f32> {
        let minutes = time;
        let total_daylight_duration = self.sunset - self.sunrise;

        let daylight_time = if minutes < self.sunrise {
            0.0
        } else if minutes > self.sunset {
            total_daylight_duration as f32
        } else {
            (minutes - self.sunrise) as f32
        };

        let normalized_time = daylight_time / total_daylight_duration as f32;

        // Sun moves east (-1) to west (1) across X axis
        let sun_x = (normalized_time * std::f32::consts::PI * 2.0).sin();
        let sun_y = (normalized_time * std::f32::consts::PI).sin(); // Sun height arc
        let sun_z = 0.0; // Keep it in the XZ plane

        Vec3::new(sun_x, sun_y, sun_z).normalized()
    }
}
