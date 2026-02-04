use crate::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ThePattern {
    Solid(TheColor),
    SolidWithBorder(TheColor, TheColor, f32),
}

use ThePattern::*;

impl ThePattern {
    pub fn get_color(
        &self,
        _p: Vec2<f32>,
        distance: &f32,
        background: &TheColor,
        highlight: Option<&TheColor>,
    ) -> TheColor {
        match self {
            Solid(color) => {
                let c = if let Some(highlight) = highlight {
                    highlight.clone()
                } else {
                    color.clone()
                };
                background.mix(&c, ThePattern::fill_mask(*distance))
            }
            SolidWithBorder(color, border_color, border) => {
                let c = if let Some(highlight) = highlight {
                    highlight.clone()
                } else {
                    color.clone()
                };

                let m = ThePattern::fill_mask(*distance + border / 3.0);
                let b = background.mix(&c, m * c.a);

                let m = ThePattern::border_mask(*distance + border / 3.0, *border);
                b.mix(border_color, m)
            }
        }
    }

    /// Returns the fill mask for the given distance.
    #[inline(always)]
    fn fill_mask(dist: f32) -> f32 {
        (-dist).clamp(0.0, 1.0)
    }

    /// Returns the border mask for a given distance and border width.
    #[inline(always)]
    fn border_mask(distance: f32, width: f32) -> f32 {
        (distance + width).clamp(0.0, 1.0) - distance.clamp(0.0, 1.0)
    }
}
