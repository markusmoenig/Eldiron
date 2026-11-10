//! Editable native paint generators embedded in portable MZ Image Source recipes.
//! Pixels are a replaceable cache; the source reference holds the generator settings.
use denrim_materials::{image_source::ImageSource, integration::EditableMaterialAsset};
use theframework::prelude::*;

pub const PREFIX: &str = "eldiron:paint:";
pub const BRUSHES: [&str; 17] = [
    "material",
    "brick",
    "moss",
    "grass",
    "dirt",
    "crack",
    "puddle",
    "mud",
    "rubble",
    "leaves",
    "flowers",
    "vines",
    "roots",
    "bushes",
    "tree",
    "candles",
    "footprints",
];
pub const STAMPS: [&str; 11] = [
    "grass",
    "mud",
    "rubble",
    "leaves",
    "flowers",
    "vines",
    "roots",
    "bushes",
    "tree",
    "candles",
    "footprints",
];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub brush: String,
    pub stamp: bool,
    pub shape: String,
    pub seed: u32,
    pub variant: String,
    pub size: f32,
    pub opacity: f32,
    pub material_id: u8,
    pub colors: Vec<[u8; 4]>,
    pub palette: Vec<Option<u16>>,
    pub scale: f32,
    pub detail: f32,
    pub variation: f32,
}
impl Spec {
    pub fn new(brush: &str, stamp: bool) -> Self {
        let preset = crate::iso_paint_brush::preset_for_key(brush);
        // These organic generators have no continuous-brush preset. Preserve their
        // foliage/wood/wax palette roles instead of the generic beige fill fallback.
        let colors = match brush {
            "vines" => vec![
                [48, 73, 43, 255],
                [93, 130, 69, 255],
                [30, 46, 34, 255],
                [74, 49, 28, 255],
            ],
            "roots" => vec![[92, 58, 36, 255], [44, 31, 23, 255], [134, 94, 55, 255]],
            "bushes" => vec![
                [30, 49, 34, 255],
                [82, 124, 58, 255],
                [111, 151, 74, 255],
                [74, 49, 28, 255],
            ],
            "tree" => vec![
                [30, 49, 34, 255],
                [82, 124, 58, 255],
                [92, 58, 36, 255],
                [111, 151, 74, 255],
            ],
            "candles" => vec![
                [233, 213, 166, 255],
                [154, 131, 92, 255],
                [255, 151, 45, 255],
                [255, 239, 142, 255],
            ],
            _ => crate::iso_paint_brush::preset_layer_infos(brush)
                .into_iter()
                .find(|l| !l.colors.is_empty())
                .map_or_else(|| vec![preset.default_color], |l| l.colors.to_vec()),
        };
        Self {
            brush: brush.into(),
            stamp,
            shape: preset.default_shape.into(),
            seed: 19,
            variant: "wildflowers".into(),
            size: match brush {
                "tree" => 2.35,
                "bushes" => 1.85,
                "material" | "brick" => 8.,
                "moss" => 1.6,
                "grass" if !stamp => 1.2,
                "dirt" => 1.4,
                "crack" => 0.6,
                "puddle" => 1.8,
                "mud" => 1.1,
                "vines" | "roots" => 1.15,
                _ => 1.,
            },
            opacity: match brush {
                "moss" => 0.75,
                "dirt" | "mud" => 0.82,
                "puddle" => 0.68,
                "footprints" => 0.9,
                _ => 1.,
            },
            material_id: 0,
            colors,
            palette: vec![],
            scale: 1.,
            detail: 0.65,
            variation: 0.6,
        }
    }
    pub fn from_image(image: &ImageSource) -> Option<Self> {
        image
            .reference
            .as_deref()?
            .strip_prefix(PREFIX)
            .and_then(|s| serde_json::from_str(s).ok())
    }
    pub fn from_asset(asset: &EditableMaterialAsset) -> Option<Self> {
        asset
            .root
            .project
            .materials
            .iter()
            .flat_map(|m| &m.rows)
            .flat_map(|r| &r.tiles)
            .filter_map(|t| t.image.as_ref())
            .find_map(Self::from_image)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !BRUSHES.contains(&self.brush.as_str())
            || self.stamp && !STAMPS.contains(&self.brush.as_str())
            || !self.size.is_finite()
            || !(0.1..=8.).contains(&self.size)
            || !self.scale.is_finite()
            || !(0.25..=4.).contains(&self.scale)
            || !self.detail.is_finite()
            || !(0. ..=1.).contains(&self.detail)
            || !self.variation.is_finite()
            || !(0. ..=1.).contains(&self.variation)
            || !self.opacity.is_finite()
            || !(0. ..=1.).contains(&self.opacity)
            || self.colors.is_empty()
            || self.colors.len() > 8
            || self.material_id > 59
        {
            return Err("Invalid paint source settings".into());
        }
        Ok(())
    }
    pub fn resolve_palette(&mut self, palette: &ThePalette) {
        for (i, index) in self.palette.iter().enumerate() {
            if let Some(color) = index
                .and_then(|i| palette.colors.get(i as usize))
                .and_then(Option::as_ref)
                && let Some(slot) = self.colors.get_mut(i)
            {
                *slot = color.to_u8_array();
            }
        }
    }
    pub fn layout(&self) -> rusterix::map::tile::PaintAssetLayout {
        let (native, anchor) = if self.stamp {
            let side = (self.size * 24.).ceil().max(16.) as u32 + 4;
            let above = (self.size * 34.).ceil().max(24.) as u32 + 4;
            let below = (self.size * 14.).ceil().max(12.) as u32 + 4;
            (
                [side * 2 + 1, above + below + 1],
                [side as f32, above as f32],
            )
        } else {
            ([64; 2], [32.; 2])
        };
        rusterix::map::tile::PaintAssetLayout {
            brush: self.brush.clone(),
            stamp: self.stamp,
            shape: self.shape.clone(),
            material_id: self.material_id,
            size: self.size,
            opacity: self.opacity,
            native_size: native,
            anchor,
        }
    }
    #[cfg(feature = "graphics")]
    pub fn render(
        &self,
    ) -> Result<
        (
            ImageSource,
            rusterix::map::tile::PaintAssetLayout,
            Vec<rusterix::Texture>,
        ),
        String,
    > {
        self.validate()?;
        let mut textures = Vec::new();
        let mut native = [64, 64];
        let mut anchor = [32., 32.];
        let mut pattern = crate::iso_paint::IsoPaintLayer::default();
        pattern.active_color = self.colors[0];
        pattern.active_palette_colors = self.colors.clone();
        pattern.active_pattern_scale = self.scale;
        pattern.active_pattern_detail = self.detail;
        pattern.active_pattern_variation = self.variation;
        for frame in 0..4 {
            let seed = self.seed.wrapping_add(frame * 7919);
            let texture = if self.stamp {
                let mut stamp = crate::iso_paint::IsoPaintStamp::new(
                    self.brush.clone(),
                    crate::iso_paint::IsoPaintPoint::new([0, 0], None, None),
                    self.colors[0],
                    vec![],
                    self.colors.clone(),
                    self.material_id,
                    self.size,
                    1.,
                    self.variant.clone(),
                    0.,
                    0.,
                );
                stamp.variation = seed;
                let (image, at) =
                    crate::iso_paint_render::IsoPaintRenderer::iso_paint_native_stamp_raster(
                        &stamp,
                    );
                native = [image.dim().width as u32, image.dim().height as u32];
                anchor = at.map(|v| v as f32);
                rusterix::Texture::new(
                    image.pixels().to_vec(),
                    native[0] as usize,
                    native[1] as usize,
                )
            } else {
                let mut texture = rusterix::Texture::alloc(64, 64);
                let sample = crate::iso_paint_brush::IsoPaintBrushSample {
                    brush: &self.brush,
                    shape: &self.shape,
                    color: self.colors[0],
                    palette: &self.colors,
                    opacity: 1.,
                    radius: 8,
                    seed,
                };
                for y in 0..64 {
                    for x in 0..64 {
                        let rgba = if self.brush == "brick" {
                            pattern.paint_baked_pattern_color(x as i32, y as i32)
                        } else {
                            crate::iso_paint_brush::sample_tiled_pixel(
                                &sample,
                                (x as f32 / self.scale) as i32,
                                (y as f32 / self.scale) as i32,
                            )
                            .unwrap_or([0; 4])
                        };
                        let i = (y * 64 + x) * 4;
                        texture.data[i..i + 4].copy_from_slice(&rgba);
                    }
                }
                texture
            };
            textures.push(texture);
        }
        let frames = textures.iter().map(|t| t.data.clone()).collect::<Vec<_>>();
        let mut image = ImageSource::rgba8(native[0] as usize, native[1] as usize, &frames)
            .map_err(|e| e.to_string())?;
        image.reference = Some(format!(
            "{PREFIX}{}",
            serde_json::to_string(self).map_err(|e| e.to_string())?
        ));
        let layout = rusterix::map::tile::PaintAssetLayout {
            brush: self.brush.clone(),
            stamp: self.stamp,
            shape: self.shape.clone(),
            material_id: self.material_id,
            size: self.size,
            opacity: self.opacity,
            native_size: native,
            anchor,
        };
        Ok((image, layout, textures))
    }
}
