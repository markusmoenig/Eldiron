use crate::ast::ColorFamily;
use std::{collections::BTreeSet, error::Error, fmt};
use theframework::prelude::{TheColor, ThePalette};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaletteError {
    Empty,
}

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "the supplied palette contains no colors"),
        }
    }
}

impl Error for PaletteError {}

#[derive(Clone, Debug)]
struct PaletteEntry {
    source_index: usize,
    color: TheColor,
    hue: f32,
    saturation: f32,
    lightness: f32,
    luma: f32,
}

#[derive(Clone, Debug)]
pub struct PaletteModel {
    entries: Vec<PaletteEntry>,
}

impl PaletteModel {
    pub fn new(palette: &ThePalette) -> Result<Self, PaletteError> {
        let entries = palette
            .colors
            .iter()
            .enumerate()
            .filter_map(|(source_index, color)| {
                color.clone().map(|color| {
                    let hsl = color.as_hsl();
                    let luma = color.r * 0.2126 + color.g * 0.7152 + color.b * 0.0722;
                    PaletteEntry {
                        source_index,
                        color,
                        hue: hsl.x,
                        saturation: hsl.y,
                        lightness: hsl.z,
                        luma,
                    }
                })
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return Err(PaletteError::Empty);
        }
        Ok(Self { entries })
    }

    pub fn color_count(&self) -> usize {
        self.entries.len()
    }

    pub fn resolve(&self, family: ColorFamily, position: f32) -> usize {
        self.resolve_with_saturation(family, position, [0.0, 1.0])
    }

    pub fn resolve_with_saturation(
        &self,
        family: ColorFamily,
        position: f32,
        saturation_range: [f32; 2],
    ) -> usize {
        let target_luma = position.clamp(0.0, 1.0);
        let saturation_min = saturation_range[0].min(saturation_range[1]).clamp(0.0, 1.0);
        let saturation_max = saturation_range[0].max(saturation_range[1]).clamp(0.0, 1.0);
        self.entries
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let score_a = self.family_score(a, family)
                    + (a.luma - target_luma).abs() * 3.2
                    + (a.lightness - target_luma).abs() * 0.6
                    + saturation_range_penalty(a.saturation, saturation_min, saturation_max);
                let score_b = self.family_score(b, family)
                    + (b.luma - target_luma).abs() * 3.2
                    + (b.lightness - target_luma).abs() * 0.6
                    + saturation_range_penalty(b.saturation, saturation_min, saturation_max);
                score_a.total_cmp(&score_b)
            })
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    pub fn resolve_ramp(
        &self,
        family: ColorFamily,
        positions: &[f32],
        saturation_range: [f32; 2],
    ) -> Vec<usize> {
        if positions.is_empty() {
            return Vec::new();
        }
        let anchor_position =
            positions.iter().copied().sum::<f32>() / positions.len().max(1) as f32;
        let anchor_index = self.resolve_with_saturation(family, anchor_position, saturation_range);
        let anchor = self.entries.get(anchor_index).unwrap_or(&self.entries[0]);
        let saturation_min = saturation_range[0].min(saturation_range[1]).clamp(0.0, 1.0);
        let saturation_max = saturation_range[0].max(saturation_range[1]).clamp(0.0, 1.0);

        positions
            .iter()
            .map(|position| {
                let target_luma = position.clamp(0.0, 1.0);
                self.entries
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        coherent_ramp_score(anchor, a, target_luma, saturation_min, saturation_max)
                            .total_cmp(&coherent_ramp_score(
                                anchor,
                                b,
                                target_luma,
                                saturation_min,
                                saturation_max,
                            ))
                    })
                    .map(|(index, _)| index)
                    .unwrap_or(anchor_index)
            })
            .collect()
    }

    pub fn resolve_anchored_ramp(
        &self,
        base: [u8; 4],
        steps: usize,
        brightness: [f32; 2],
        saturation: [f32; 2],
        hue: [f32; 2],
    ) -> Vec<usize> {
        let base = TheColor::from_u8_array(base);
        let base_hsl = base.as_hsl();
        let steps = steps.max(2);
        let mut ramp = (0..steps)
            .map(|index| {
                let factor = index as f32 / (steps - 1) as f32;
                let target_hue = (base_hsl.x + lerp(hue[0], hue[1], factor)).rem_euclid(1.0);
                let target_saturation =
                    (base_hsl.y + lerp(saturation[0], saturation[1], factor)).clamp(0.0, 1.0);
                let target_lightness =
                    (base_hsl.z + lerp(brightness[0], brightness[1], factor)).clamp(0.0, 1.0);
                self.entries
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        anchored_ramp_score(a, target_hue, target_saturation, target_lightness)
                            .total_cmp(&anchored_ramp_score(
                                b,
                                target_hue,
                                target_saturation,
                                target_lightness,
                            ))
                    })
                    .map(|(index, _)| index)
                    .unwrap_or(0)
            })
            .collect::<Vec<_>>();

        let distinct_count = ramp.iter().copied().collect::<BTreeSet<_>>().len();
        if self.entries.len() > 1
            && brightness[0] != brightness[1]
            && distinct_count < self.entries.len().min(3)
        {
            let anchor = ramp[steps / 2];
            let fallback = (0..steps)
                .map(|index| {
                    let factor = index as f32 / (steps - 1) as f32;
                    let offset = lerp(brightness[0], brightness[1], factor);
                    let shade_steps = if offset.abs() < 0.025 {
                        0
                    } else {
                        (offset / 0.10).round() as i32
                    };
                    self.shade(anchor, shade_steps)
                })
                .collect::<Vec<_>>();
            if fallback.iter().copied().collect::<BTreeSet<_>>().len() > distinct_count {
                ramp = fallback;
            }
        }

        ramp
    }

    /// Maps the authored base to the palette once, then derives unrestricted colors around
    /// that mapped anchor. This preserves project color identity without quantizing every step.
    pub fn resolve_base_only_ramp(
        &self,
        base: [u8; 4],
        steps: usize,
        brightness: [f32; 2],
        saturation: [f32; 2],
        hue: [f32; 2],
    ) -> Vec<[u8; 4]> {
        let authored = TheColor::from_u8_array(base);
        let anchor_index = self.closest([authored.r, authored.g, authored.b, authored.a]);
        let anchor = self.entries.get(anchor_index).unwrap_or(&self.entries[0]);
        let anchor_hsl = anchor.color.as_hsl();
        let alpha = anchor.color.to_u8_array()[3];
        let steps = steps.max(2);

        (0..steps)
            .map(|index| {
                let factor = index as f32 / (steps - 1) as f32;
                let target_hue = (anchor_hsl.x + lerp(hue[0], hue[1], factor)).rem_euclid(1.0);
                let target_saturation =
                    (anchor_hsl.y + lerp(saturation[0], saturation[1], factor)).clamp(0.0, 1.0);
                let target_lightness =
                    (anchor_hsl.z + lerp(brightness[0], brightness[1], factor)).clamp(0.0, 1.0);
                let mut rgba = TheColor::from_hsl(target_hue, target_saturation, target_lightness)
                    .to_u8_array();
                rgba[3] = alpha;
                rgba
            })
            .collect()
    }

    pub fn shade(&self, current: usize, steps: i32) -> usize {
        let Some(base) = self.entries.get(current) else {
            return self.resolve(ColorFamily::Any, 0.5);
        };
        if steps == 0 {
            return current;
        }
        let target_luma = (base.luma + steps as f32 * 0.12).clamp(0.0, 1.0);
        let direction = steps.signum();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                if direction < 0 {
                    candidate.luma < base.luma - 0.005
                } else {
                    candidate.luma > base.luma + 0.005
                }
            })
            .min_by(|(_, a), (_, b)| {
                let score_a = shade_score(base, a, target_luma);
                let score_b = shade_score(base, b, target_luma);
                score_a.total_cmp(&score_b)
            })
            .map(|(index, _)| index)
            .unwrap_or(current)
    }

    pub fn rgba(&self, index: usize) -> [u8; 4] {
        self.entries
            .get(index)
            .unwrap_or(&self.entries[0])
            .color
            .to_u8_array()
    }

    pub fn source_index(&self, index: usize) -> usize {
        self.entries
            .get(index)
            .unwrap_or(&self.entries[0])
            .source_index
    }

    pub fn closest(&self, rgba: [f32; 4]) -> usize {
        self.entries
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| color_distance(a, rgba).total_cmp(&color_distance(b, rgba)))
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn family_score(&self, color: &PaletteEntry, family: ColorFamily) -> f32 {
        let chroma = color.saturation;
        match family {
            ColorFamily::Any => 0.0,
            ColorFamily::Neutral => chroma * 2.5,
            ColorFamily::Warm => hue_distance(color.hue, 0.08) * 1.3 + (chroma - 0.55).abs() * 0.12,
            ColorFamily::Cool => hue_distance(color.hue, 0.58) * 1.3 + (chroma - 0.5).abs() * 0.12,
            ColorFamily::Earth => {
                hue_distance(color.hue, 0.075) * 1.6 + (chroma - 0.38).abs() * 0.35
            }
            ColorFamily::Red => hue_distance(color.hue, 0.0) * 2.0,
            ColorFamily::Orange => hue_distance(color.hue, 0.083) * 2.0,
            ColorFamily::Yellow => hue_distance(color.hue, 0.16) * 2.0,
            ColorFamily::Green => hue_distance(color.hue, 0.33) * 2.0,
            ColorFamily::Cyan => hue_distance(color.hue, 0.5) * 2.0,
            ColorFamily::Blue => hue_distance(color.hue, 0.63) * 2.0,
            ColorFamily::Purple => hue_distance(color.hue, 0.76) * 2.0,
            ColorFamily::Magenta => hue_distance(color.hue, 0.9) * 2.0,
        }
    }
}

fn saturation_range_penalty(saturation: f32, min: f32, max: f32) -> f32 {
    if saturation < min {
        (min - saturation) * 8.0
    } else if saturation > max {
        (saturation - max) * 8.0
    } else {
        0.0
    }
}

fn coherent_ramp_score(
    anchor: &PaletteEntry,
    candidate: &PaletteEntry,
    target_luma: f32,
    saturation_min: f32,
    saturation_max: f32,
) -> f32 {
    let hue_penalty = if anchor.saturation < 0.12 {
        candidate.saturation * 2.5
    } else if candidate.saturation < 0.12 {
        0.8 + anchor.saturation
    } else {
        hue_distance(anchor.hue, candidate.hue) * 3.0
    };
    (candidate.luma - target_luma).abs() * 4.0
        + (candidate.lightness - target_luma).abs() * 0.4
        + hue_penalty
        + (candidate.saturation - anchor.saturation).abs() * 0.45
        + saturation_range_penalty(candidate.saturation, saturation_min, saturation_max)
}

fn anchored_ramp_score(
    candidate: &PaletteEntry,
    target_hue: f32,
    target_saturation: f32,
    target_lightness: f32,
) -> f32 {
    let target_luma = TheColor::from_hsl(target_hue, target_saturation, target_lightness);
    let target_luma = target_luma.r * 0.2126 + target_luma.g * 0.7152 + target_luma.b * 0.0722;
    let hue_weight = target_saturation.max(candidate.saturation) * 3.2;
    hue_distance(candidate.hue, target_hue) * hue_weight
        + (candidate.saturation - target_saturation).abs() * 0.9
        + (candidate.lightness - target_lightness).abs() * 2.2
        + (candidate.luma - target_luma).abs() * 2.8
}

fn hue_distance(a: f32, b: f32) -> f32 {
    let distance = (a - b).abs();
    distance.min(1.0 - distance)
}

fn shade_score(base: &PaletteEntry, candidate: &PaletteEntry, target_luma: f32) -> f32 {
    let hue_penalty = if base.saturation < 0.12 || candidate.saturation < 0.12 {
        0.08
    } else {
        hue_distance(base.hue, candidate.hue)
    };
    (candidate.luma - target_luma).abs() * 3.0
        + hue_penalty * 1.2
        + (candidate.saturation - base.saturation).abs() * 0.25
}

fn color_distance(entry: &PaletteEntry, rgba: [f32; 4]) -> f32 {
    let dr = entry.color.r - rgba[0];
    let dg = entry.color.g - rgba[1];
    let db = entry.color.b - rgba[2];
    let da = entry.color.a - rgba[3];
    dr * dr * 0.30 + dg * dg * 0.59 + db * db * 0.11 + da * da * 0.05
}

fn lerp(a: f32, b: f32, factor: f32) -> f32 {
    a + (b - a) * factor
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> ThePalette {
        ThePalette::new(vec![
            Some(TheColor::from_u8(10, 10, 12, 255)),
            Some(TheColor::from_u8(80, 52, 35, 255)),
            Some(TheColor::from_u8(160, 110, 70, 255)),
            Some(TheColor::from_u8(230, 220, 190, 255)),
            Some(TheColor::from_u8(40, 90, 180, 255)),
        ])
    }

    #[test]
    fn resolves_and_shades_palette_entries() {
        let model = PaletteModel::new(&palette()).unwrap();
        let earth = model.resolve(ColorFamily::Earth, 0.55);
        let darker = model.shade(earth, -1);
        let lighter = model.shade(earth, 1);
        let earth_rgba = model.rgba(earth);
        assert_ne!(model.rgba(darker), earth_rgba);
        assert_ne!(model.rgba(lighter), earth_rgba);
    }

    #[test]
    fn rejects_empty_palettes() {
        assert_eq!(
            PaletteModel::new(&ThePalette::empty_256()).unwrap_err(),
            PaletteError::Empty
        );
    }

    #[test]
    fn saturation_range_avoids_vivid_palette_entries() {
        let palette = ThePalette::new(vec![
            Some(TheColor::from_u8(130, 75, 48, 255)),
            Some(TheColor::from_u8(255, 80, 0, 255)),
        ]);
        let model = PaletteModel::new(&palette).unwrap();

        let constrained = model.resolve_with_saturation(ColorFamily::Earth, 0.45, [0.0, 0.55]);

        assert_eq!(model.source_index(constrained), 0);
    }

    #[test]
    fn ramp_stays_in_one_color_family() {
        let palette = ThePalette::new(vec![
            Some(TheColor::from_u8(55, 31, 28, 255)),
            Some(TheColor::from_u8(105, 62, 44, 255)),
            Some(TheColor::from_u8(128, 128, 128, 255)),
            Some(TheColor::from_u8(175, 112, 70, 255)),
        ]);
        let model = PaletteModel::new(&palette).unwrap();

        let ramp = model.resolve_ramp(ColorFamily::Earth, &[0.18, 0.32, 0.48], [0.05, 0.65]);

        assert!(!ramp.contains(&2));
    }

    #[test]
    fn anchored_ramp_tracks_the_requested_base_color() {
        let palette = ThePalette::new(vec![
            Some(TheColor::from_u8(28, 19, 15, 255)),
            Some(TheColor::from_u8(58, 36, 24, 255)),
            Some(TheColor::from_u8(105, 67, 40, 255)),
            Some(TheColor::from_u8(50, 65, 110, 255)),
            Some(TheColor::from_u8(150, 150, 150, 255)),
        ]);
        let model = PaletteModel::new(&palette).unwrap();

        let ramp = model.resolve_anchored_ramp(
            [58, 36, 24, 255],
            3,
            [-0.12, 0.18],
            [-0.04, 0.04],
            [0.0, 0.0],
        );

        assert_eq!(
            ramp.iter()
                .map(|index| model.source_index(*index))
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn anchored_ramp_does_not_collapse_on_a_sparse_palette() {
        let palette = ThePalette::new(vec![
            Some(TheColor::from_u8(12, 12, 16, 255)),
            Some(TheColor::from_u8(61, 52, 52, 255)),
            Some(TheColor::from_u8(24, 74, 175, 255)),
            Some(TheColor::from_u8(232, 232, 226, 255)),
        ]);
        let model = PaletteModel::new(&palette).unwrap();

        let ramp = model.resolve_anchored_ramp(
            [58, 36, 24, 255],
            6,
            [-0.16, 0.18],
            [-0.05, 0.04],
            [0.0, 0.0],
        );

        assert!(ramp.windows(2).any(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn anchored_ramp_does_not_collapse_on_nice31() {
        let colors = [
            "636663", "87857c", "bcad9f", "f2b888", "eb9661", "b55945", "734c44", "3d3333",
            "593e47", "7a5859", "a57855", "de9f47", "fdd179", "fee1b8", "d4c692", "a6b04f",
            "819447", "44702d", "2f4d2f", "546756", "89a477", "a4c5af", "cae6d9", "f1f6f0",
            "d5d6db", "bbc3d0", "96a9c1", "6c81a1", "405273", "303843", "14233a",
        ];
        let palette = ThePalette::new(
            colors
                .iter()
                .map(|hex| {
                    Some(TheColor::from_u8(
                        u8::from_str_radix(&hex[0..2], 16).unwrap(),
                        u8::from_str_radix(&hex[2..4], 16).unwrap(),
                        u8::from_str_radix(&hex[4..6], 16).unwrap(),
                        255,
                    ))
                })
                .collect(),
        );
        let model = PaletteModel::new(&palette).unwrap();
        let ramp = model.resolve_anchored_ramp(
            [58, 36, 24, 255],
            6,
            [-0.16, 0.18],
            [-0.05, 0.04],
            [0.0, 0.0],
        );

        assert!(ramp.iter().copied().collect::<BTreeSet<_>>().len() >= 3);
    }

    #[test]
    fn base_only_ramp_maps_the_anchor_but_not_every_gradient_step() {
        let model = PaletteModel::new(&palette()).unwrap();
        let ramp = model.resolve_base_only_ramp(
            [58, 36, 24, 255],
            32,
            [-0.2, 0.2],
            [-0.05, 0.05],
            [0.0, 0.0],
        );
        let palette_colors = (0..model.color_count())
            .map(|index| model.rgba(index))
            .collect::<Vec<_>>();

        assert_eq!(ramp.len(), 32);
        assert!(
            ramp.iter()
                .filter(|color| palette_colors.contains(color))
                .count()
                < ramp.len()
        );
    }
}
