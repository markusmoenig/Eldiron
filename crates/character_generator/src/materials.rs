use crate::*;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct MaterialTexture {
    pub size: u32,
    pub rgba: Vec<u8>,
    pub orm: Vec<u8>,
}
pub fn linear_byte(value: u8) -> f32 {
    let s = value as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
pub fn srgb_byte(value: f32) -> u8 {
    let x = value.clamp(0.0, 1.0);
    let s = if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0).round() as u8
}
impl Material {
    pub fn sample_color(&self, uv: [f32; 2]) -> [f32; 4] {
        let mut result = self.color;
        if let Some(texture) = &self.texture {
            let x = (uv[0].clamp(0.0, 1.0) * (texture.size - 1) as f32).round() as u32;
            let y = (uv[1].clamp(0.0, 1.0) * (texture.size - 1) as f32).round() as u32;
            let index = ((y * texture.size + x) * 4) as usize;
            for (i, channel) in result.iter_mut().enumerate().take(3) {
                *channel *= linear_byte(texture.rgba[index + i]);
            }
        }
        result
    }
}
