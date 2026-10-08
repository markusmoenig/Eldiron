use crate::*;
use procedural_recipes::ColorSource;
use procedural_recipes::{
    RecipeRenderer, RenderOptions, RenderSurface, RenderSurfaceFrame, RenderSurfaceMapping,
    parse_material_document,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use theframework::prelude::{TheColor, ThePalette};
fn default_size() -> u32 {
    64
}
fn default_tiling() -> [f32; 2] {
    [1.0, 1.0]
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDefinition {
    pub source: String,
    #[serde(default)]
    pub palette: Vec<String>,
    #[serde(default = "default_size")]
    pub size: u32,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_tiling")]
    pub tiling: [f32; 2],
}
impl MaterialDefinition {
    pub fn validate(&self) -> Result<()> {
        if !(4..=256).contains(&self.size) {
            return Err("size must be 4..256".into());
        }
        if self
            .tiling
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.0 || *x > 64.0)
        {
            return Err("tiling must be finite and in (0,64]".into());
        }
        for value in &self.palette {
            color(value)?;
        }
        if parse_material_document(&self.source)?.materials.len() != 1 {
            return Err("each material alias must contain exactly one Material definition".into());
        }
        Ok(())
    }
    pub fn render(&self) -> Result<MaterialTexture> {
        self.validate()?;
        let document = parse_material_document(&self.source)?;
        let surface = RenderSurface {
            width: self.size,
            height: self.size,
            mapping: RenderSurfaceMapping {
                u_axis: [self.tiling[0], 0.0],
                v_axis: [0.0, self.tiling[1]],
                ..Default::default()
            },
            fps: 1.0,
            looping: false,
            frames: vec![RenderSurfaceFrame { time: 0.0 }],
        };
        let colors = if self.palette.is_empty() {
            document.materials[0]
                .colors
                .iter()
                .filter_map(|c| match c.source {
                    ColorSource::Exact(rgba) | ColorSource::Nearest(rgba) => {
                        Some(Some(TheColor::from_u8(rgba[0], rgba[1], rgba[2], rgba[3])))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        } else {
            self.palette
                .iter()
                .map(|c| Some(TheColor::from_hex(c)))
                .collect()
        };
        let palette = ThePalette::new(if colors.is_empty() {
            vec![Some(TheColor::from_u8(255, 255, 255, 255))]
        } else {
            colors
        });
        let rendered = RecipeRenderer::new(&palette)?.render_material_on_surface(
            &document.materials[0],
            &surface,
            &RenderOptions {
                seed_offset: self.seed,
            },
        )?;
        let frame = rendered
            .frames
            .into_iter()
            .next()
            .ok_or("material produced no frame")?;
        // glTF ORM: occlusion in R, roughness in G, metallic in B.
        let orm = frame
            .material
            .iter()
            .flat_map(|m| {
                [
                    255,
                    (m[0].clamp(0.0, 1.0) * 255.0).round() as u8,
                    (m[1].clamp(0.0, 1.0) * 255.0).round() as u8,
                    255,
                ]
            })
            .collect();
        // Opaque fitted garments only; no silent loss of translucent cloth semantics.
        if frame.rgba.as_chunks::<4>().0.iter().any(|p| p[3] != 255)
            || frame.material.iter().any(|m| m[2] < 1.0 || m[3] > 0.0)
        {
            return Err(
                "character materials must be opaque and non-emissive in this prototype".into(),
            );
        }
        Ok(MaterialTexture {
            size: self.size,
            rgba: frame.rgba,
            orm,
        })
    }
}
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
pub(crate) fn apply_materials(
    asset: &mut CharacterAsset,
    items: &BTreeMap<String, ItemDefinition>,
    definitions: &BTreeMap<String, MaterialDefinition>,
) -> Result<()> {
    let mut textures = BTreeMap::new();
    for material in &mut asset.materials {
        if let Some(alias) = items.get(&material.name).and_then(|i| i.material.as_ref()) {
            if !textures.contains_key(alias) {
                textures.insert(
                    alias.clone(),
                    definitions[alias]
                        .render()
                        .map_err(|e| format!("material {alias}: {e}"))?,
                );
            }
            material.texture = Some(textures[alias].clone());
        }
    }
    Ok(())
}
