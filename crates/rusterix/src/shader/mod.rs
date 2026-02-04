pub mod grid;
pub mod vgradient;

use crate::{BLACK, Pixel};
use vek::{Vec2, Vec3, Vec4};

/// The shader trait.
#[allow(unused)]
pub trait Shader: Send + Sync {
    fn new() -> Self
    where
        Self: Sized;

    /// Shade a pixel (for 2D Shaders).
    fn shade_pixel(&self, uv: Vec2<f32>, screen: Vec2<f32>) -> Pixel {
        BLACK
    }

    /// Set an f32 parameter.
    fn set_parameter_f32(&mut self, key: &str, value: f32) {}

    /// Set a Vec2 parameter.
    fn set_parameter_vec2(&mut self, key: &str, value: Vec2<f32>) {}

    /// Set a Vec3 parameter.
    fn set_parameter_vec3(&mut self, key: &str, value: Vec3<f32>) {}

    /// Set a Vec4 parameter.
    fn set_parameter_vec4(&mut self, key: &str, value: Vec4<f32>) {}

    /// Set a Pixel parameter.
    fn set_parameter_pixel(&mut self, key: &str, value: Pixel) {}
}
