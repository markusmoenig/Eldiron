use crate::{AvatarBuildOutput, AvatarDirection, AvatarMarkerChannel};
use procedural_recipes::{
    MaterialRecipe, RecipeRenderer, RenderError, RenderOptions, RenderSurface, RenderSurfaceFrame,
    RenderSurfaceMapping, SdfRecipe, SdfRenderer,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarAppearanceSpace {
    Avatar,
    #[default]
    Channel,
    Part,
}

impl AvatarAppearanceSpace {
    pub fn from_key(key: &str) -> Self {
        match key.trim().to_ascii_lowercase().as_str() {
            "avatar" => Self::Avatar,
            "part" => Self::Part,
            _ => Self::Channel,
        }
    }
}

pub struct AvatarWearableRecipeAdapter;

impl AvatarWearableRecipeAdapter {
    /// Projects one reusable material through semantic Avatar marker masks.
    pub fn apply(
        renderer: &RecipeRenderer,
        material: &MaterialRecipe,
        output: &mut AvatarBuildOutput,
        channels: &[AvatarMarkerChannel],
        direction: AvatarDirection,
        space: AvatarAppearanceSpace,
        tiling: [f32; 2],
        time: f32,
        seed_offset: u64,
    ) -> Result<usize, RenderError> {
        let size = output.size as usize;
        if size == 0
            || output.rgba.len() != size * size * 4
            || output.marker_channels.len() != size * size
        {
            return Err(RenderError::Dimensions(
                "avatar appearance dimensions do not match".to_string(),
            ));
        }

        let groups = match space {
            AvatarAppearanceSpace::Avatar => vec![channels.to_vec()],
            AvatarAppearanceSpace::Channel | AvatarAppearanceSpace::Part => channels
                .iter()
                .copied()
                .map(|channel| vec![channel])
                .collect(),
        };
        let mut changed = 0;
        for group in groups {
            let mask = output
                .marker_channels
                .iter()
                .map(|channel| channel.is_some_and(|channel| group.contains(&channel)))
                .collect::<Vec<_>>();
            let Some(bounds) = mask_bounds(&mask, size) else {
                continue;
            };
            let mapping = projection_mapping(direction, bounds, size, tiling);
            let surface = RenderSurface {
                width: output.size,
                height: output.size,
                mapping,
                fps: 0.0,
                looping: false,
                frames: vec![RenderSurfaceFrame { time }],
            };
            let rendered = renderer.render_material_on_surface(
                material,
                &surface,
                &RenderOptions { seed_offset },
            )?;
            let frame = &rendered.frames[0];
            let average_luma = mask
                .iter()
                .enumerate()
                .filter(|(_, covered)| **covered)
                .map(|(index, _)| luma(&output.rgba[index * 4..index * 4 + 3]))
                .sum::<f32>()
                / mask.iter().filter(|covered| **covered).count().max(1) as f32;

            for (index, covered) in mask.into_iter().enumerate() {
                if !covered {
                    continue;
                }
                let dst = &mut output.rgba[index * 4..index * 4 + 4];
                let src = &frame.rgba[index * 4..index * 4 + 4];
                let shape_shade = if average_luma > 0.001 {
                    (luma(&dst[..3]) / average_luma).clamp(0.65, 1.2)
                } else {
                    1.0
                };
                let alpha = (src[3] as f32 / 255.0) * frame.material[index][2];
                for channel in 0..3 {
                    let recipe = (src[channel] as f32 * shape_shade).clamp(0.0, 255.0);
                    dst[channel] =
                        (recipe * alpha + dst[channel] as f32 * (1.0 - alpha)).round() as u8;
                }
                changed += 1;
            }
        }
        Ok(changed)
    }
}

/// Places an SDF silhouette in the head marker bounding box and colors it with
/// the same reusable material recipes used by other procedural surfaces.
pub struct AvatarHeadgearRecipeAdapter;

impl AvatarHeadgearRecipeAdapter {
    pub fn apply(
        renderer: &RecipeRenderer,
        sdf: &SdfRecipe,
        material: &MaterialRecipe,
        output: &mut AvatarBuildOutput,
        time: f32,
        seed_offset: u64,
    ) -> Result<usize, RenderError> {
        let size = output.size as usize;
        if size == 0
            || output.rgba.len() != size * size * 4
            || output.marker_channels.len() != size * size
        {
            return Err(RenderError::Dimensions(
                "avatar headgear dimensions do not match".to_string(),
            ));
        }
        let head_mask = output
            .marker_channels
            .iter()
            .map(|channel| {
                matches!(
                    channel,
                    Some(
                        AvatarMarkerChannel::SkinLight
                            | AvatarMarkerChannel::SkinDark
                            | AvatarMarkerChannel::Hair
                            | AvatarMarkerChannel::Eyes
                    )
                )
            })
            .collect::<Vec<_>>();
        let Some(bounds) = mask_bounds(&head_mask, size) else {
            return Ok(0);
        };
        let bounds = expanded_head_bounds(bounds, size);
        let width = (bounds[2] + 1 - bounds[0]) as f32;
        let height = (bounds[3] + 1 - bounds[1]) as f32;
        let surface = RenderSurface {
            width: output.size,
            height: output.size,
            mapping: RenderSurfaceMapping {
                origin: [-(bounds[0] as f32) / width, -(bounds[1] as f32) / height],
                u_axis: [size as f32 / width, 0.0],
                v_axis: [0.0, size as f32 / height],
            },
            fps: 0.0,
            looping: false,
            frames: vec![RenderSurfaceFrame { time }],
        };
        let coverage = SdfRenderer::render(sdf, &surface)?;
        let rendered = renderer.render_material_on_surface(
            material,
            &surface,
            &RenderOptions { seed_offset },
        )?;
        let frame = &rendered.frames[0];
        let mut changed = 0;
        for (index, coverage) in coverage.coverage.into_iter().enumerate() {
            if coverage == 0 {
                continue;
            }
            let src = &frame.rgba[index * 4..index * 4 + 4];
            let dst = &mut output.rgba[index * 4..index * 4 + 4];
            let alpha =
                (coverage as f32 / 255.0) * (src[3] as f32 / 255.0) * frame.material[index][2];
            for channel in 0..3 {
                dst[channel] = (src[channel] as f32 * alpha + dst[channel] as f32 * (1.0 - alpha))
                    .round() as u8;
            }
            dst[3] = dst[3].max((255.0 * alpha).round() as u8);
            changed += 1;
        }
        Ok(changed)
    }
}

fn expanded_head_bounds(bounds: [usize; 4], size: usize) -> [usize; 4] {
    let width = bounds[2] + 1 - bounds[0];
    let height = bounds[3] + 1 - bounds[1];
    [
        bounds[0].saturating_sub((width as f32 * 0.18).ceil() as usize),
        bounds[1].saturating_sub((height as f32 * 0.22).ceil() as usize),
        (bounds[2] + (width as f32 * 0.18).ceil() as usize).min(size - 1),
        (bounds[3] + (height as f32 * 0.06).ceil() as usize).min(size - 1),
    ]
}

fn luma(rgb: &[u8]) -> f32 {
    rgb[0] as f32 * 0.2126 + rgb[1] as f32 * 0.7152 + rgb[2] as f32 * 0.0722
}

fn mask_bounds(mask: &[bool], size: usize) -> Option<[usize; 4]> {
    let mut bounds = [size, size, 0, 0];
    let mut found = false;
    for (index, covered) in mask.iter().copied().enumerate() {
        if !covered {
            continue;
        }
        let x = index % size;
        let y = index / size;
        bounds[0] = bounds[0].min(x);
        bounds[1] = bounds[1].min(y);
        bounds[2] = bounds[2].max(x);
        bounds[3] = bounds[3].max(y);
        found = true;
    }
    found.then_some(bounds)
}

fn direction_turns(direction: AvatarDirection) -> f32 {
    match direction {
        AvatarDirection::Front => 0.0,
        AvatarDirection::FrontRight => 0.125,
        AvatarDirection::Right => 0.25,
        AvatarDirection::BackRight => 0.375,
        AvatarDirection::Back => 0.5,
        AvatarDirection::BackLeft => 0.625,
        AvatarDirection::Left => 0.75,
        AvatarDirection::FrontLeft => 0.875,
    }
}

fn projection_mapping(
    direction: AvatarDirection,
    bounds: [usize; 4],
    size: usize,
    tiling: [f32; 2],
) -> RenderSurfaceMapping {
    let min_x = bounds[0] as f32 / size as f32;
    let min_y = bounds[1] as f32 / size as f32;
    let width = (bounds[2] + 1 - bounds[0]) as f32 / size as f32;
    let height = (bounds[3] + 1 - bounds[1]) as f32 / size as f32;
    let tile_x = tiling[0].max(0.000_1);
    let tile_y = tiling[1].max(0.000_1);
    let visible_turns = 0.5;
    RenderSurfaceMapping {
        origin: [
            tile_x
                * (direction_turns(direction)
                    + visible_turns * 0.5
                    + visible_turns * min_x / width),
            -tile_y * min_y / height,
        ],
        u_axis: [-tile_x * visible_turns / width, 0.0],
        v_axis: [0.0, tile_y / height],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use procedural_recipes::{parse_material_document, parse_sdf_document};

    #[test]
    fn eight_directions_advance_around_one_canonical_surface() {
        let bounds = [0, 0, 7, 7];
        let directions = [
            AvatarDirection::Front,
            AvatarDirection::FrontRight,
            AvatarDirection::Right,
            AvatarDirection::BackRight,
            AvatarDirection::Back,
            AvatarDirection::BackLeft,
            AvatarDirection::Left,
            AvatarDirection::FrontLeft,
        ];
        let centers = directions.map(|direction| {
            projection_mapping(direction, bounds, 8, [1.0, 1.0]).map([0.5, 0.5])[0]
        });
        for index in 1..centers.len() {
            assert!((centers[index] - centers[index - 1] - 0.125).abs() < 0.000_01);
        }
        assert!(projection_mapping(AvatarDirection::Left, bounds, 8, [1.0, 1.0]).u_axis[0] < 0.0);
    }

    #[test]
    fn wearable_material_is_clipped_to_the_requested_marker_mask() {
        let material = parse_material_document(
            r#"
Material cloth
    Surface
        color = #d04020
        opacity = 1.0
"#,
        )
        .unwrap()
        .materials
        .remove(0);
        let renderer = RecipeRenderer::grayscale();
        let mut output = AvatarBuildOutput {
            size: 2,
            rgba: [80, 90, 100, 255].repeat(4),
            marker_channels: vec![
                None,
                Some(AvatarMarkerChannel::Torso),
                Some(AvatarMarkerChannel::Arms),
                None,
            ],
        };
        let unchanged = output.rgba[0..4].to_vec();

        let changed = AvatarWearableRecipeAdapter::apply(
            &renderer,
            &material,
            &mut output,
            &[AvatarMarkerChannel::Torso],
            AvatarDirection::Front,
            AvatarAppearanceSpace::Channel,
            [1.0, 1.0],
            0.0,
            7,
        )
        .unwrap();

        assert_eq!(changed, 1);
        assert_eq!(&output.rgba[0..4], unchanged.as_slice());
        assert_ne!(&output.rgba[4..8], unchanged.as_slice());
        assert_eq!(&output.rgba[8..12], unchanged.as_slice());
        assert_eq!(&output.rgba[12..16], unchanged.as_slice());
    }

    #[test]
    fn headgear_sdf_is_fitted_to_head_markers_and_composited() {
        let sdf = parse_sdf_document(
            r#"
Sdf helm
    Shape Shell
        RoundedRectangle
            position = F2(0.5, 0.5)
            size = F2(0.9, 0.9)
            radius = 0.1
    Output
        coverage = Shell
"#,
        )
        .unwrap()
        .recipes
        .remove(0);
        let material = parse_material_document(
            r#"
Material iron
    Surface
        color = #b0c0d0
        opacity = 1.0
"#,
        )
        .unwrap()
        .materials
        .remove(0);
        let mut output = AvatarBuildOutput {
            size: 16,
            rgba: [20, 30, 40, 255].repeat(16 * 16),
            marker_channels: vec![None; 16 * 16],
        };
        for y in 4..10 {
            for x in 6..10 {
                output.marker_channels[y * 16 + x] = Some(AvatarMarkerChannel::SkinLight);
            }
        }
        let outside_before = output.rgba[0..4].to_vec();
        let center = (6 * 16 + 8) * 4;
        let center_before = output.rgba[center..center + 4].to_vec();

        let changed = AvatarHeadgearRecipeAdapter::apply(
            &RecipeRenderer::grayscale(),
            &sdf,
            &material,
            &mut output,
            0.0,
            1,
        )
        .unwrap();

        assert!(changed > 0);
        assert_eq!(&output.rgba[0..4], outside_before.as_slice());
        assert_ne!(&output.rgba[center..center + 4], center_before.as_slice());
    }
}
