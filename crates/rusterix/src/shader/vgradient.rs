use crate::{Pixel, Shader};
use vek::Vec2;

pub struct VGrayGradientShader;

impl Shader for VGrayGradientShader {
    fn new() -> Self {
        VGrayGradientShader
    }

    fn shade_pixel(&self, uv: Vec2<f32>, _screen: Vec2<f32>) -> Pixel {
        let intensity = (uv.y * 128.0).clamp(0.0, 128.0) as u8;
        [intensity, intensity, intensity, 255]
    }
}
