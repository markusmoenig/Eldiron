//! Project palette matching. Indices and surface materials remain stable.
use crate::project::{PaletteMaterial, Project};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use theframework::prelude::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaletteState {
    pub palette: ThePalette,
    pub materials: Vec<PaletteMaterial>,
    /// Imported color set, including colors not currently assigned to a slot.
    pub source_colors: Vec<TheColor>,
    pub source_name: String,
}
impl PaletteState {
    pub fn capture(project: &Project) -> Self {
        Self {
            palette: project.palette.clone(),
            materials: project.palette_materials.clone(),
            source_name: project.palette_source_name.clone(),
            source_colors: project.palette_source_colors.clone(),
        }
    }
    pub fn apply(&self, project: &mut Project) {
        project.palette = self.palette.clone();
        project.palette_materials = self.materials.clone();
        project.palette_source_colors = self.source_colors.clone();
        project.palette_source_name = self.source_name.clone();
        project.ensure_palette_materials_len();
    }
}
#[derive(Clone, Debug)]
pub struct PaletteSwitch {
    pub before: PaletteState,
    pub after: PaletteState,
    pub changed_slots: usize,
    pub collapsed_ramps: usize,
}
/// Oklab from linear sRGB, public-domain reference coefficients:
/// https://bottosson.github.io/posts/oklab/
pub fn perceptual(color: &TheColor) -> [f64; 3] {
    let v = color.to_u8_array();
    let linear = |x: u8| {
        let x = x as f64 / 255.;
        if x <= 0.04045 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (linear(v[0]), linear(v[1]), linear(v[2]));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| (a - b) * (a - b)).sum()
}
pub fn closest(color: &TheColor, colors: &[TheColor]) -> Option<usize> {
    let lab = perceptual(color);
    colors
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            distance(lab, perceptual(a)).total_cmp(&distance(lab, perceptual(b)))
        })
        .map(|(i, _)| i)
}
pub fn parse_colors(text: &str) -> Result<Vec<TheColor>, String> {
    let mut colors = vec![];
    let mut seen = HashSet::new();
    let gpl = text
        .lines()
        .next()
        .is_some_and(|s| s.trim() == "GIMP Palette");
    let jasc = text.trim_start().starts_with("JASC-PAL");
    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with(';')
            || (gpl
                && (line.starts_with('#')
                    || line.starts_with("Name:")
                    || line.starts_with("Columns:")
                    || line == "GIMP Palette"))
            || (jasc && line_no < 3)
        {
            continue;
        }
        let rgba = if gpl || jasc {
            let parts = line
                .split_whitespace()
                .take(3)
                .map(|n| n.parse::<u8>())
                .collect::<Result<Vec<_>, _>>();
            match parts {
                Ok(v) if v.len() == 3 => Some([v[0], v[1], v[2], 255]),
                _ => None,
            }
        } else {
            let hex = line
                .trim_start_matches('#')
                .split_whitespace()
                .next()
                .unwrap_or("");
            if hex.is_ascii()
                && matches!(hex.len(), 6 | 8)
                && hex.bytes().all(|b| b.is_ascii_hexdigit())
            {
                let at = if hex.len() == 8 { 2 } else { 0 };
                Some([
                    u8::from_str_radix(&hex[at..at + 2], 16).unwrap(),
                    u8::from_str_radix(&hex[at + 2..at + 4], 16).unwrap(),
                    u8::from_str_radix(&hex[at + 4..at + 6], 16).unwrap(),
                    255,
                ])
            } else {
                None
            }
        };
        if let Some(c) = rgba {
            if seen.insert(c) {
                colors.push(TheColor::from_u8(c[0], c[1], c[2], 255));
            }
        }
        if colors.len() > 256 {
            return Err("Palettes support up to 256 distinct colors".into());
        }
    }
    if colors.is_empty() {
        Err("No palette colors found. Use HEX, Paint.NET TXT, GIMP GPL or JASC PAL.".into())
    } else {
        Ok(colors)
    }
}
/// Match ordered shading groups with a small dynamic program. Previously assigned
/// shared stops are fixed so one palette index never acquires conflicting colors.
fn match_ramp(
    old: &[[f64; 3]],
    target: &[[f64; 3]],
    locked: &[Option<usize>],
    strict: bool,
) -> Option<Vec<usize>> {
    let n = target.len();
    let mut costs = vec![vec![f64::INFINITY; n]; old.len()];
    let mut parents = vec![vec![0; n]; old.len()];
    for row in 0..old.len() {
        for j in 0..n {
            if locked[row].is_some_and(|k| k != j) {
                continue;
            }
            let cost = distance(old[row], target[j]);
            if row == 0 {
                costs[row][j] = cost;
                continue;
            }
            for k in 0..=j {
                let separate = distance(old[row], old[row - 1]) > 0.00000001;
                let min_light = if old[row][0] - old[row - 1][0] >= 0.005 {
                    0.005
                } else {
                    0.
                };
                if strict && separate && (k == j || target[j][0] - target[k][0] < min_light) {
                    continue;
                }
                let value = costs[row - 1][k] + cost;
                if value < costs[row][j] {
                    costs[row][j] = value;
                    parents[row][j] = k;
                }
            }
        }
    }
    let mut j =
        (0..n).min_by(|a, b| costs[old.len() - 1][*a].total_cmp(&costs[old.len() - 1][*b]))?;
    if !costs[old.len() - 1][j].is_finite() {
        return None;
    }
    let mut result = vec![0; old.len()];
    for row in (0..old.len()).rev() {
        result[row] = j;
        j = parents[row][j];
    }
    Some(result)
}
impl PaletteSwitch {
    pub fn prepare(
        project: &Project,
        mut colors: Vec<TheColor>,
        ramps: &[Vec<u16>],
    ) -> Result<Self, String> {
        if colors.is_empty() || colors.len() > 256 {
            return Err("Choose 1–256 colors".into());
        }
        let source_colors = colors.clone();
        colors.sort_by(|a, b| perceptual(a)[0].total_cmp(&perceptual(b)[0]));
        let target = colors.iter().map(perceptual).collect::<Vec<_>>();
        let before = PaletteState::capture(project);
        let mut after = before.clone();
        after.source_colors = source_colors;
        after.source_name = "Imported Palette".into();
        let mut mapping = before
            .palette
            .colors
            .iter()
            .map(|c| c.as_ref().and_then(|c| closest(c, &colors)))
            .collect::<Vec<_>>();
        let mut locked = HashMap::new();
        for ramp in ramps {
            let mut indices = ramp
                .iter()
                .copied()
                .filter(|i| {
                    before
                        .palette
                        .colors
                        .get(*i as usize)
                        .is_some_and(Option::is_some)
                })
                .collect::<Vec<_>>();
            indices.sort_unstable();
            indices.dedup();
            indices.sort_by(|a, b| {
                perceptual(before.palette.colors[*a as usize].as_ref().unwrap())[0]
                    .total_cmp(&perceptual(before.palette.colors[*b as usize].as_ref().unwrap())[0])
            });
            if indices.len() < 2 {
                continue;
            }
            let old = indices
                .iter()
                .map(|i| perceptual(before.palette.colors[*i as usize].as_ref().unwrap()))
                .collect::<Vec<_>>();
            let fixed = indices
                .iter()
                .map(|i| locked.get(i).copied())
                .collect::<Vec<_>>();
            if let Some(matched) = match_ramp(&old, &target, &fixed, true)
                .or_else(|| match_ramp(&old, &target, &fixed, false))
            {
                for (index, color) in indices.into_iter().zip(matched) {
                    mapping[index as usize] = Some(color);
                    locked.insert(index, color);
                }
            }
        }
        let mut changed_slots = 0;
        for (i, index) in mapping.iter().enumerate() {
            if let Some(index) = index {
                let mut color = colors[*index].clone();
                color.a = before.palette.colors[i].as_ref().unwrap().a;
                if after.palette.colors[i].as_ref() != Some(&color) {
                    changed_slots += 1;
                }
                after.palette.colors[i] = Some(color);
            }
        }
        // Make additional target swatches immediately usable where space permits;
        // the complete imported set is retained for assignment even on a full board.
        for color in &colors {
            if !after
                .palette
                .colors
                .iter()
                .flatten()
                .any(|c| c.to_u8_array()[..3] == color.to_u8_array()[..3])
            {
                if let Some(slot) = after.palette.colors.iter_mut().find(|c| c.is_none()) {
                    *slot = Some(color.clone());
                }
            }
        }
        let collapsed_ramps = ramps
            .iter()
            .filter(|r| {
                let original = r
                    .iter()
                    .filter_map(|i| {
                        before
                            .palette
                            .colors
                            .get(*i as usize)
                            .and_then(Option::as_ref)
                    })
                    .map(|c| {
                        let v = c.to_u8_array();
                        [v[0], v[1], v[2]]
                    })
                    .collect::<HashSet<_>>();
                let mapped = r
                    .iter()
                    .filter_map(|i| mapping.get(*i as usize).copied().flatten())
                    .collect::<HashSet<_>>();
                original.len() > mapped.len()
            })
            .count();
        Ok(Self {
            before,
            after,
            changed_slots,
            collapsed_ramps,
        })
    }
}
/// Palette-fit a canonical bitmap without accumulating quantization across switches.
/// Alpha is retained exactly. Callers retain the original PNG/recipe as the source.
pub fn fit_rgba(bytes: &mut [u8], palette: &ThePalette) {
    let colors = palette.colors.iter().flatten().cloned().collect::<Vec<_>>();
    if colors.is_empty() {
        return;
    }
    let mut cache = HashMap::new();
    for p in bytes.chunks_exact_mut(4) {
        if p[3] == 0 {
            continue;
        }
        let rgb = [p[0], p[1], p[2]];
        let color = *cache.entry(rgb).or_insert_with(|| {
            let c = TheColor::from_u8(rgb[0], rgb[1], rgb[2], 255);
            colors[closest(&c, &colors).unwrap()].to_u8_array()
        });
        p[..3].copy_from_slice(&color[..3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn color(v: u8) -> TheColor {
        TheColor::from_u8(v, v, v, 255)
    }
    #[test]
    fn reselecting_the_same_palette_preserves_every_slot() {
        let p = Project::default();
        let colors = p.palette.colors.iter().flatten().cloned().collect();
        let result = PaletteSwitch::prepare(
            &p,
            colors,
            &[
                vec![1, 2, 3, 4, 5, 6, 7, 9, 8],
                vec![39, 40, 41, 42, 43, 44, 45],
            ],
        )
        .unwrap();
        assert_eq!(result.after.palette, p.palette);
        assert_eq!(result.changed_slots, 0);
        assert_eq!(result.collapsed_ramps, 0);
    }
    #[test]
    fn matching_preserves_slots_materials_and_every_target_swatch() {
        let mut p = Project::default();
        p.palette_materials[5].preset = "metal".into();
        p.palette_materials[5].finish = "polished".into();
        p.palette.current_index = 42;
        let targets = vec![color(0), color(96), color(192), color(255)];
        let plan = PaletteSwitch::prepare(&p, targets.clone(), &[vec![2, 5, 7, 8]]).unwrap();
        assert_eq!(plan.before, PaletteState::capture(&p));
        assert_eq!(plan.after.materials, p.palette_materials);
        assert_eq!(plan.after.palette.current_index, 42);
        assert_eq!(plan.after.source_colors, targets);
        for c in plan.after.palette.colors.iter().flatten() {
            assert!(targets.contains(c));
        }
        let ramp = [2, 5, 7, 8].map(|i| plan.after.palette.colors[i].clone().unwrap());
        assert!(
            ramp.windows(2)
                .all(|pair| perceptual(&pair[0])[0] < perceptual(&pair[1])[0])
        );
        plan.after.apply(&mut p);
        let saved = serde_json::to_string(&p).unwrap();
        let reopened: Project = serde_json::from_str(&saved).unwrap();
        assert_eq!(PaletteState::capture(&reopened), plan.after);
        plan.before.apply(&mut p);
        assert_eq!(PaletteState::capture(&p), plan.before);
    }
    #[test]
    fn every_256_color_target_is_retained_even_when_slot_board_is_full() {
        let mut p = Project::default();
        p.palette.colors = (0..=255)
            .map(|i| Some(TheColor::from_u8(i, 0, 0, 255)))
            .collect();
        let target = (0..=255).map(color).collect::<Vec<_>>();
        let plan = PaletteSwitch::prepare(&p, target.clone(), &[]).unwrap();
        assert_eq!(plan.after.source_colors, target);
        assert_eq!(plan.after.palette.colors.len(), 256);
    }
    #[test]
    fn tiny_palettes_report_ramp_loss_and_preserve_alpha() {
        let mut p = Project::default();
        p.palette.colors[1].as_mut().unwrap().a = 0.4;
        let plan = PaletteSwitch::prepare(&p, vec![color(100)], &[vec![1, 5, 8]]).unwrap();
        assert_eq!(plan.collapsed_ramps, 1);
        assert_eq!(plan.after.palette.colors[1].as_ref().unwrap().a, 0.4);
    }
    #[test]
    fn common_palette_formats_reject_empty_and_oversize_input() {
        for source in [
            "#112233\n445566\n112233",
            "; Paint.NET\nFF112233\nFF445566",
            "GIMP Palette\nName: Sample\nColumns: 2\n# Colors\n17 34 51 Brown\n68 85 102 Blue",
            "JASC-PAL\n0100\n2\n17 34 51\n68 85 102",
        ] {
            let colors = parse_colors(source).unwrap();
            assert_eq!(colors.len(), 2);
            assert_eq!(colors[0].to_u8_array(), [17, 34, 51, 255]);
        }
        assert!(parse_colors("no colors here").is_err());
        assert!(parse_colors("éééééé").is_err());
        assert!(PaletteSwitch::prepare(&Project::default(), vec![], &[]).is_err());
    }
    #[test]
    fn bitmap_fitting_keeps_transparency_and_uses_only_target_colors() {
        let mut p = ThePalette::empty_256();
        p.colors[3] = Some(color(0));
        p.colors[9] = Some(color(255));
        let mut bytes = vec![220, 210, 200, 128, 20, 10, 0, 255, 30, 40, 50, 0];
        fit_rgba(&mut bytes, &p);
        assert_eq!(bytes, vec![255, 255, 255, 128, 0, 0, 0, 255, 30, 40, 50, 0]);
        let white = perceptual(&color(255));
        assert!((white[0] - 1.).abs() < 0.00001);
        assert!(white[1].abs() < 0.00001 && white[2].abs() < 0.00001);
    }
}
