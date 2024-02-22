use theframework::prelude::*;

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub struct Daylight {
    pub sunrise: i32,             // Sunrise time in minutes
    pub sunset: i32,              // Sunset time in minutes
    pub transition_duration: i32, // Duration of the sunrise/sunset transition in minutes
    pub daylight_color: Vec3f,    // Color during the day
    pub sunrise_color: Vec3f,     // Color at sunrise
    pub sunset_color: Vec3f,      // Color at sunset
    pub night_color: Vec3f,       // Color at night
}

impl Default for Daylight {
    fn default() -> Self {
        Self {
            sunrise: 300,                               // 5:00 am
            sunset: 1200,                               // 8:00 pm
            transition_duration: 60,                    // 1 hour
            daylight_color: Vec3f::new(1.0, 0.95, 0.9), // Slightly yellowish white
            sunrise_color: Vec3f::new(1.0, 0.8, 0.8),   // Soft red
            sunset_color: Vec3f::new(1.0, 0.8, 0.8),    // Soft red
            night_color: Vec3f::new(0.0, 0.0, 0.4),     // Dark blue
        }
    }
}

impl Daylight {
    pub fn daylight(&self, time: i32, min_bright: f32, max_bright: f32) -> Vec3f {
        let minutes = time;

        let sunrise = self.sunrise;
        let sunset = self.sunset;
        let transition_duration = self.transition_duration;

        let daylight_start = sunrise + transition_duration;
        //let daylight_end = sunset - transition_duration;

        let mut daylight = if minutes < sunrise || minutes > sunset + transition_duration {
            self.night_color
        } else if minutes >= sunrise && minutes < daylight_start {
            let v = (minutes - sunrise) as f32 / transition_duration as f32;
            Vec3f::lerp(self.night_color, self.sunrise_color, v)
        } else if minutes >= daylight_start && minutes < sunset {
            self.daylight_color
        } else if minutes >= sunset && minutes <= sunset + transition_duration {
            let v = (minutes - sunset) as f32 / transition_duration as f32;
            Vec3f::lerp(self.sunset_color, self.night_color, v)
        } else {
            self.night_color
        };

        let mi = Vec3f::new(min_bright, min_bright, min_bright);
        let ma = Vec3f::new(max_bright, max_bright, max_bright);

        daylight.x = daylight.x.clamp(mi.x.min(ma.x), mi.x.max(ma.x));
        daylight.y = daylight.y.clamp(mi.y.min(ma.y), mi.y.max(ma.y));
        daylight.z = daylight.z.clamp(mi.z.min(ma.z), mi.z.max(ma.z));

        daylight
    }
}
