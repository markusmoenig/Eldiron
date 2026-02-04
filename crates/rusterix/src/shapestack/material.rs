use std::str::FromStr;
use theframework::prelude::*;
use vek::Vec4;

// Role

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MaterialRole {
    Matte,
    Glossy,
    Metallic,
    Transparent,
    Emissive,
}

use MaterialRole::*;

impl MaterialRole {
    pub fn to_u8(&self) -> u8 {
        match self {
            MaterialRole::Matte => 0,
            MaterialRole::Glossy => 1,
            MaterialRole::Metallic => 2,
            MaterialRole::Transparent => 3,
            MaterialRole::Emissive => 4,
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => MaterialRole::Glossy,
            2 => MaterialRole::Metallic,
            3 => MaterialRole::Transparent,
            4 => MaterialRole::Emissive,
            _ => MaterialRole::Matte,
        }
    }
}

impl FromStr for MaterialRole {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Matte" => Ok(Matte),
            "Glossy" => Ok(Glossy),
            "Metallic" => Ok(Metallic),
            "Transparent" => Ok(Transparent),
            "Emissive" => Ok(Emissive),
            _ => Err(()),
        }
    }
}

// Modifier

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaterialModifier {
    None,
    Luminance,
    Saturation,
    InvLuminance,
    InvSaturation,
}

impl MaterialModifier {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => MaterialModifier::Luminance,
            2 => MaterialModifier::Saturation,
            3 => MaterialModifier::InvLuminance,
            4 => MaterialModifier::InvSaturation,
            _ => MaterialModifier::None,
        }
    }
}

impl MaterialModifier {
    #[inline(always)]
    pub fn modify(&self, color: &Vec4<f32>, strength: &f32) -> f32 {
        match self {
            MaterialModifier::None => *strength,
            MaterialModifier::Luminance => {
                let lum = 0.2126 * color.x + 0.7152 * color.y + 0.0722 * color.z;
                lum * strength
            }
            MaterialModifier::InvLuminance => {
                let lum = 0.2126 * color.x + 0.7152 * color.y + 0.0722 * color.z;
                (1.0 - lum) * strength
            }
            MaterialModifier::Saturation => {
                let r = color.x;
                let g = color.y;
                let b = color.z;
                let max = r.max(g.max(b));
                let min = r.min(g.min(b));
                let sat = if max > 0.0 { (max - min) / max } else { 0.0 };
                sat * strength
            }
            MaterialModifier::InvSaturation => {
                let r = color.x;
                let g = color.y;
                let b = color.z;
                let max = r.max(g.max(b));
                let min = r.min(g.min(b));
                let sat = if max > 0.0 { (max - min) / max } else { 0.0 };
                (1.0 - sat) * strength
            }
        }
    }
}

// Material

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Material {
    pub role: MaterialRole,
    pub modifier: MaterialModifier,
    pub value: f32,
    pub flicker: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self::new(MaterialRole::Matte, MaterialModifier::None, 1.0, 0.0)
    }
}

impl Material {
    pub fn new(role: MaterialRole, modifier: MaterialModifier, value: f32, flicker: f32) -> Self {
        Self {
            role,
            modifier,
            value,
            flicker,
        }
    }
}
