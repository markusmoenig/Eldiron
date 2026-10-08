//! Deterministic appearance generation, independent of Eldiron gameplay and rendering.
mod bake;
#[cfg(feature = "eldiron")]
pub mod eldiron;
mod export;
mod viewer;
pub use viewer::export_viewer;
mod materials;
mod math;
mod model;
mod style;
pub use style::{GenerationOptions, ResolvedStyle, Shading, StylePreset};
pub mod ruleset;
pub use bake::{AtlasMetadata, BakeOptions, bake_atlas};
pub use export::export_glb;
pub use materials::{MaterialDefinition, MaterialTexture};
pub use model::*;

use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub version: u32,
    pub characters: BTreeMap<String, CharacterDefinition>,
    #[serde(default)]
    pub items: BTreeMap<String, ItemDefinition>,
    #[serde(default)]
    pub materials: BTreeMap<String, MaterialDefinition>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterDefinition {
    pub body: String,
    pub height: f32,
    pub build: f32,
    pub skin: String,
    pub hair: String,
    #[serde(default)]
    pub head: HeadDefinition,
    #[serde(default)]
    pub outfit: Vec<String>,
    #[serde(default)]
    pub equipment: Vec<String>,
}
/// Physical appearance traits; art style remains a caller-owned GenerationOptions setting.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HeadDefinition {
    pub jaw: f32,
    pub cheekbones: f32,
    pub nose_length: f32,
    pub nose_width: f32,
    pub eye_spacing: f32,
    pub ear_size: f32,
    pub eye_color: String,
}
impl Default for HeadDefinition {
    fn default() -> Self {
        Self {
            jaw: 1.0,
            cheekbones: 1.0,
            nose_length: 1.0,
            nose_width: 1.0,
            eye_spacing: 1.0,
            ear_size: 1.0,
            eye_color: "#222631".into(),
        }
    }
}
impl HeadDefinition {
    fn validate(&self) -> Result<()> {
        for (name, value, range) in [
            ("jaw", self.jaw, 0.7..=1.3),
            ("cheekbones", self.cheekbones, 0.8..=1.2),
            ("nose_length", self.nose_length, 0.6..=1.4),
            ("nose_width", self.nose_width, 0.6..=1.4),
            ("eye_spacing", self.eye_spacing, 0.75..=1.25),
            ("ear_size", self.ear_size, 0.6..=1.4),
        ] {
            if !range.contains(&value) {
                return Err(format!("invalid head {name}").into());
            }
        }
        color(&self.eye_color)?;
        Ok(())
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemDefinition {
    pub generator: ItemKind,
    pub color: String,
    pub material: Option<String>,
    pub attachment: Option<Attachment>,
    pub blade_length: Option<f32>,
    pub blade_width: Option<f32>,
    /// Shield width/height/depth or spike width/depth/length, in meters.
    pub size: Option<[f32; 3]>,
}
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Shirt,
    Trousers,
    Boots,
    Sword,
    Shield,
    Spike,
    Helmet,
}
impl ItemKind {
    pub fn is_held(self) -> bool {
        matches!(self, Self::Sword | Self::Shield | Self::Spike)
    }
}
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Attachment {
    MainHand,
    OffHand,
}

pub fn color(value: &str) -> Result<[f32; 4]> {
    let hex = value.strip_prefix('#').ok_or("color must start with #")?;
    if hex.len() != 6 || !hex.is_ascii() {
        return Err("color must be #RRGGBB".into());
    }
    // glTF base colors are linear; the baker converts back to sRGB.
    let mut c = [1.0; 4];
    for i in 0..3 {
        let s = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)? as f32 / 255.0;
        c[i] = if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        };
    }
    Ok(c)
}
impl Catalog {
    pub fn parse(source: &str) -> Result<Self> {
        let catalog: Self = toml::from_str(source)?;
        catalog.validate()?;
        Ok(catalog)
    }
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err("unsupported catalog version (expected 1)".into());
        }
        if self.characters.is_empty() {
            return Err("catalog needs at least one character".into());
        }
        for (id, material) in &self.materials {
            material
                .validate()
                .map_err(|e| format!("material {id}: {e}"))?;
        }
        for (id, item) in &self.items {
            if let Some(alias) = &item.material
                && !self.materials.contains_key(alias)
            {
                return Err(format!("item {id}: unknown material {alias}").into());
            }
            color(&item.color).map_err(|e| format!("item {id}: {e}"))?;
            if item.generator.is_held() && item.attachment.is_none() {
                return Err(format!("item {id}: held item needs attachment").into());
            }
            if item.generator == ItemKind::Sword {
                for (name, value, range) in [
                    ("blade_length", item.blade_length, 0.1..=1.5),
                    ("blade_width", item.blade_width, 0.01..=0.2),
                ] {
                    if let Some(v) = value
                        && !range.contains(&v)
                    {
                        return Err(format!("item {id}: invalid {name}").into());
                    }
                }
            } else if item.blade_length.is_some() || item.blade_width.is_some() {
                return Err(format!("item {id}: blade properties require a sword").into());
            }
            if !item.generator.is_held() && item.attachment.is_some() {
                return Err(format!("item {id}: attachment requires a held item").into());
            }
            if let Some(size) = item.size
                && (!matches!(item.generator, ItemKind::Shield | ItemKind::Spike)
                    || size.iter().any(|v| !(0.01..=1.5).contains(v)))
            {
                return Err(format!(
                    "item {id}: size requires shield/spike dimensions in 0.01..1.5 meters"
                )
                .into());
            }
        }
        for (id, c) in &self.characters {
            if c.body != "humanoid" {
                return Err(format!("character {id}: unsupported body {}", c.body).into());
            }
            if !(1.0..=2.5).contains(&c.height) || !(0.0..=1.0).contains(&c.build) {
                return Err(format!("character {id}: height must be 1..2.5 and build 0..1").into());
            }
            color(&c.skin)?;
            color(&c.hair)?;
            c.head
                .validate()
                .map_err(|e| format!("character {id}: {e}"))?;
            let mut slots = BTreeSet::new();
            for (ids, weapon) in [(&c.outfit, false), (&c.equipment, true)] {
                for item_id in ids {
                    let item = self
                        .items
                        .get(item_id)
                        .ok_or_else(|| format!("character {id}: unknown item {item_id}"))?;
                    if item.generator.is_held() != weapon {
                        return Err(
                            format!("character {id}: item {item_id} is in the wrong list").into(),
                        );
                    }
                    let slot = match item.generator {
                        ItemKind::Sword | ItemKind::Shield | ItemKind::Spike => {
                            match item.attachment.unwrap() {
                                Attachment::MainHand => "main_hand",
                                Attachment::OffHand => "off_hand",
                            }
                        }
                        ItemKind::Shirt => "shirt",
                        ItemKind::Trousers => "trousers",
                        ItemKind::Boots => "boots",
                        ItemKind::Helmet => "helmet",
                    };
                    if !slots.insert(slot) {
                        return Err(format!("character {id}: duplicate {slot}").into());
                    }
                }
            }
        }
        Ok(())
    }
    pub fn generate(&self, id: &str) -> Result<CharacterAsset> {
        self.generate_with_options(id, &GenerationOptions::default())
    }
    pub fn generate_with_options(
        &self,
        id: &str,
        options: &GenerationOptions,
    ) -> Result<CharacterAsset> {
        self.validate()?;
        let style = options.resolve()?;
        let definition = self
            .characters
            .get(id)
            .ok_or_else(|| format!("unknown character {id}"))?;
        let mut asset = model::generate(id, definition, &self.items, style)?;
        materials::apply_materials(&mut asset, &self.items, &self.materials)?;
        Ok(asset)
    }
}
