use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum MaterialFamily {
    Default = 0,
    Stone = 1,
    Dirt = 2,
    Wood = 3,
    Metal = 4,
    Glass = 5,
    Water = 6,
    Mirror = 7,
    Emissive = 8,
    Fabric = 9,
    Plastic = 10,
    Foliage = 11,
    Skin = 12,
    Bone = 13,
    Wax = 14,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(u8)]
pub enum MaterialFinish {
    Natural = 0,
    Matte = 1,
    Polished = 2,
    Wet = 3,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct MaterialDefinition {
    pub family: MaterialFamily,
    pub finish: MaterialFinish,
    pub roughness: f32,
    pub metallic: f32,
    pub opacity: f32,
    pub emissive: f32,
    pub subsurface: f32,
    pub transmission: f32,
    pub fuzz: f32,
    pub porosity: f32,
    pub sheen: f32,
}

pub const SEMANTIC_MATERIAL_MARKER: u8 = 0xFE;
pub const MATERIAL_TABLE_ID_COUNT: usize = 256;
pub const MATERIAL_TABLE_ROWS_PER_ID: usize = 3;
pub const MATERIAL_PRESET_NAMES: [&str; 15] = [
    "default", "stone", "dirt", "wood", "metal", "glass", "water", "mirror", "emissive", "fabric",
    "plastic", "foliage", "skin", "bone", "wax",
];
pub const MATERIAL_FINISH_NAMES: [&str; 4] = ["natural", "matte", "polished", "wet"];

impl MaterialFamily {
    pub fn from_preset(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "stone" => Self::Stone,
            "dirt" => Self::Dirt,
            "wood" => Self::Wood,
            "metal" => Self::Metal,
            "glass" => Self::Glass,
            "water" => Self::Water,
            "mirror" => Self::Mirror,
            "emissive" => Self::Emissive,
            "fabric" => Self::Fabric,
            "plastic" => Self::Plastic,
            "foliage" | "moss" | "grass" | "leaf" | "leaves" => Self::Foliage,
            "skin" => Self::Skin,
            "bone" => Self::Bone,
            "wax" => Self::Wax,
            _ => Self::Default,
        }
    }
}

impl MaterialFinish {
    pub fn from_name(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "matte" => Self::Matte,
            "polished" => Self::Polished,
            "wet" => Self::Wet,
            _ => Self::Natural,
        }
    }
}

impl MaterialDefinition {
    pub fn id_for(family: MaterialFamily, finish: MaterialFinish) -> u8 {
        (family as u8)
            .saturating_mul(4)
            .saturating_add(finish as u8)
    }

    pub fn from_id(id: u8) -> Self {
        let family = match id / 4 {
            1 => MaterialFamily::Stone,
            2 => MaterialFamily::Dirt,
            3 => MaterialFamily::Wood,
            4 => MaterialFamily::Metal,
            5 => MaterialFamily::Glass,
            6 => MaterialFamily::Water,
            7 => MaterialFamily::Mirror,
            8 => MaterialFamily::Emissive,
            9 => MaterialFamily::Fabric,
            10 => MaterialFamily::Plastic,
            11 => MaterialFamily::Foliage,
            12 => MaterialFamily::Skin,
            13 => MaterialFamily::Bone,
            14 => MaterialFamily::Wax,
            _ => MaterialFamily::Default,
        };
        let finish = match id & 3 {
            1 => MaterialFinish::Matte,
            2 => MaterialFinish::Polished,
            3 => MaterialFinish::Wet,
            _ => MaterialFinish::Natural,
        };
        Self::new(family, finish)
    }

    pub fn from_preset_finish(preset: &str, finish: &str) -> Self {
        Self::new(
            MaterialFamily::from_preset(preset),
            MaterialFinish::from_name(finish),
        )
    }

    pub fn id(self) -> u8 {
        Self::id_for(self.family, self.finish)
    }

    pub fn rmoe_values(self) -> [f32; 4] {
        [self.roughness, self.metallic, self.opacity, self.emissive]
    }

    pub fn gpu_rows(self) -> [[f32; 4]; MATERIAL_TABLE_ROWS_PER_ID] {
        [
            [self.roughness, self.metallic, self.opacity, self.emissive],
            [self.subsurface, self.transmission, self.fuzz, self.porosity],
            [
                self.sheen,
                self.family as u8 as f32,
                self.finish as u8 as f32,
                0.0,
            ],
        ]
    }

    pub fn new(family: MaterialFamily, finish: MaterialFinish) -> Self {
        let (
            mut roughness,
            metallic,
            opacity,
            emissive,
            subsurface,
            transmission,
            fuzz,
            porosity,
            mut sheen,
        ): (f32, f32, f32, f32, f32, f32, f32, f32, f32) = match family {
            MaterialFamily::Stone => (0.78, 0.0, 1.0, 0.0, 0.00, 0.00, 0.00, 0.48, 0.08),
            MaterialFamily::Dirt => (0.92, 0.0, 1.0, 0.0, 0.00, 0.00, 0.00, 0.82, 0.02),
            MaterialFamily::Wood => (0.64, 0.0, 1.0, 0.0, 0.02, 0.00, 0.00, 0.36, 0.08),
            MaterialFamily::Metal => (0.34, 0.9, 1.0, 0.0, 0.00, 0.00, 0.00, 0.00, 0.58),
            MaterialFamily::Glass => (0.06, 0.0, 0.35, 0.0, 0.00, 0.72, 0.00, 0.00, 0.80),
            MaterialFamily::Water => (0.03, 0.0, 0.55, 0.0, 0.00, 0.86, 0.00, 0.00, 0.76),
            MaterialFamily::Mirror => (0.02, 1.0, 1.0, 0.0, 0.00, 0.00, 0.00, 0.00, 0.95),
            MaterialFamily::Emissive => (0.45, 0.0, 1.0, 1.0, 0.00, 0.00, 0.00, 0.00, 0.10),
            MaterialFamily::Fabric => (0.86, 0.0, 1.0, 0.0, 0.03, 0.00, 0.76, 0.48, 0.03),
            MaterialFamily::Plastic => (0.45, 0.0, 1.0, 0.0, 0.00, 0.00, 0.00, 0.02, 0.28),
            MaterialFamily::Foliage => (0.78, 0.0, 1.0, 0.0, 0.18, 0.35, 0.12, 0.62, 0.04),
            MaterialFamily::Skin => (0.56, 0.0, 1.0, 0.0, 0.62, 0.12, 0.16, 0.18, 0.18),
            MaterialFamily::Bone => (0.66, 0.0, 1.0, 0.0, 0.16, 0.03, 0.00, 0.24, 0.10),
            MaterialFamily::Wax => (0.42, 0.0, 1.0, 0.0, 0.78, 0.22, 0.00, 0.08, 0.26),
            MaterialFamily::Default => (0.50, 0.0, 1.0, 0.0, 0.00, 0.00, 0.00, 0.12, 0.08),
        };

        match finish {
            MaterialFinish::Matte => {
                roughness += 0.15;
                sheen *= 0.55;
            }
            MaterialFinish::Polished => {
                roughness -= 0.25;
                sheen += 0.18;
            }
            MaterialFinish::Wet => {
                roughness -= 0.35;
                sheen += 0.32;
            }
            MaterialFinish::Natural => {}
        }

        Self {
            family,
            finish,
            roughness: roughness.clamp(0.02, 1.0),
            metallic,
            opacity,
            emissive,
            subsurface,
            transmission,
            fuzz,
            porosity,
            sheen: sheen.clamp(0.0, 1.0),
        }
    }
}

pub fn material_table_rows() -> Vec<[f32; 4]> {
    let mut rows = Vec::with_capacity(MATERIAL_TABLE_ID_COUNT * MATERIAL_TABLE_ROWS_PER_ID);
    for id in 0..MATERIAL_TABLE_ID_COUNT {
        rows.extend(MaterialDefinition::from_id(id as u8).gpu_rows());
    }
    rows
}
