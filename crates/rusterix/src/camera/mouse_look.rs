use vek::Vec2;

/// Client camera input, independent of movement, targeting and rendering.
#[derive(Default)]
pub struct MouseLook {
    pub active: bool,
    pub pitch: f32,
    pub facing: Vec2<f32>,
    pub sensitivity: f32,
    pub invert_y: bool,
    pub pointer: Option<Vec2<i32>>,
}

impl MouseLook {
    pub fn begin(&mut self, facing: Vec2<f32>, sensitivity: f32, invert_y: bool) {
        self.facing = facing.try_normalized().unwrap_or(Vec2::new(0.0, -1.0));
        self.sensitivity = if sensitivity.is_finite() {
            sensitivity.clamp(0.01, 5.0)
        } else {
            0.2
        };
        self.invert_y = invert_y;
        self.active = true;
    }

    pub fn motion(&mut self, delta: Vec2<f32>) -> Option<Vec2<f32>> {
        if !self.active || !delta.x.is_finite() || !delta.y.is_finite() {
            return None;
        }
        let yaw = (delta.x * self.sensitivity).to_radians();
        let (sin, cos) = yaw.sin_cos();
        self.facing = Vec2::new(
            self.facing.x * cos - self.facing.y * sin,
            self.facing.x * sin + self.facing.y * cos,
        )
        .normalized();
        self.pitch = (self.pitch
            + delta.y * self.sensitivity * if self.invert_y { 1.0 } else { -1.0 })
        .clamp(-85.0, 85.0);
        Some(self.facing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_look_turns_right_clamps_pitch_and_requires_capture() {
        let mut look = MouseLook::default();
        assert_eq!(look.motion(Vec2::one()), None);
        look.begin(Vec2::new(0.0, -1.0), 1.0, false);
        let facing = look.motion(Vec2::new(90.0, -1000.0)).unwrap();
        assert!((facing - Vec2::unit_x()).magnitude() < 1e-5);
        assert_eq!(look.pitch, 85.0);
        look.begin(facing, 1.0, true);
        look.motion(Vec2::new(0.0, -10.0));
        assert_eq!(look.pitch, 75.0);
    }
}
