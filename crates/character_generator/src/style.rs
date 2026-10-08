use crate::Result;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StylePreset {
    #[default]
    Blocky,
    Stylized,
    Natural,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shading {
    Flat,
    Smooth,
}
/// Caller-owned creation settings, independent of character/gameplay definitions.
/// Serialize this separately as an app/project art profile if persistence is needed.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GenerationOptions {
    pub style: StylePreset,
    pub segments: Option<u16>,
    pub shading: Option<Shading>,
    pub head_scale: f32,
}
impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            style: StylePreset::Blocky,
            segments: None,
            shading: None,
            head_scale: 1.0,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq)]
pub struct ResolvedStyle {
    pub preset: StylePreset,
    pub segments: u16,
    pub shading: Shading,
    pub head_scale: f32,
}
impl GenerationOptions {
    pub fn resolve(&self) -> Result<ResolvedStyle> {
        let preset = self.style;
        let segments = self.segments;
        let shading = self.shading;
        let head_scale = self.head_scale;
        let segments = segments.unwrap_or(match preset {
            StylePreset::Blocky => 4,
            StylePreset::Stylized => 8,
            StylePreset::Natural => 12,
        });
        if !(4..=24).contains(&segments) {
            return Err("style segments must be 4..24".into());
        }
        if preset == StylePreset::Blocky && segments != 4 {
            return Err(
                "blocky style requires four segments; use stylized or natural for rounded geometry"
                    .into(),
            );
        }
        if !(0.75..=1.35).contains(&head_scale) {
            return Err("style head_scale must be 0.75..1.35".into());
        }
        Ok(ResolvedStyle {
            preset,
            segments,
            shading: shading.unwrap_or(if preset == StylePreset::Natural {
                Shading::Smooth
            } else {
                Shading::Flat
            }),
            head_scale,
        })
    }
}
