use crate::thesurface::{ThePixelRect, TheSurfaceMut};
use crate::theui::RGBA;
use zeno::{Cap, Fill, Join, Mask, PathBuilder, Scratch, Stroke};

pub use zeno::{Command as ThePathCommand, PathBuilder as ThePathBuilder};

/// An owned path representation suitable for procedural widget chrome and icons.
pub type ThePath = Vec<ThePathCommand>;

#[derive(Clone, Debug, PartialEq)]
pub enum ThePaint {
    Solid(RGBA),
    LinearGradient {
        start: [f32; 2],
        end: [f32; 2],
        start_color: RGBA,
        end_color: RGBA,
    },
}

impl ThePaint {
    pub const fn solid(color: RGBA) -> Self {
        Self::Solid(color)
    }

    pub const fn linear_gradient(
        start: [f32; 2],
        end: [f32; 2],
        start_color: RGBA,
        end_color: RGBA,
    ) -> Self {
        Self::LinearGradient {
            start,
            end,
            start_color,
            end_color,
        }
    }

    fn sample(&self, x: f32, y: f32) -> RGBA {
        match self {
            Self::Solid(color) => *color,
            Self::LinearGradient {
                start,
                end,
                start_color,
                end_color,
            } => {
                let dx = end[0] - start[0];
                let dy = end[1] - start[1];
                let length_squared = dx * dx + dy * dy;
                let amount = if length_squared <= f32::EPSILON {
                    0.0
                } else {
                    (((x - start[0]) * dx + (y - start[1]) * dy) / length_squared).clamp(0.0, 1.0)
                };
                let mut color = [0; 4];
                for channel in 0..4 {
                    color[channel] = (start_color[channel] as f32
                        + (end_color[channel] as f32 - start_color[channel] as f32) * amount)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                }
                color
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TheLineCap {
    #[default]
    Butt,
    Square,
    Round,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TheLineJoin {
    Bevel,
    #[default]
    Miter,
    Round,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThePathStroke {
    pub width: f32,
    pub cap: TheLineCap,
    pub join: TheLineJoin,
    pub paint: ThePaint,
}

impl ThePathStroke {
    pub fn new(width: f32, paint: ThePaint) -> Self {
        Self {
            width,
            cap: TheLineCap::Butt,
            join: TheLineJoin::Miter,
            paint,
        }
    }

    pub fn with_cap(mut self, cap: TheLineCap) -> Self {
        self.cap = cap;
        self
    }

    pub fn with_join(mut self, join: TheLineJoin) -> Self {
        self.join = join;
        self
    }
}

/// Zeno-backed anti-aliased path rasterizer and RGBA compositor.
///
/// The painter owns and reuses its scratch and mask allocations. Zeno remains an implementation
/// detail so widgets can be migrated without coupling the theme system to a specific rasterizer.
#[derive(Default)]
pub struct ThePainter {
    scratch: Scratch,
    mask: Vec<u8>,
}

impl ThePainter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fill_path(
        &mut self,
        surface: &mut TheSurfaceMut<'_>,
        path: &[ThePathCommand],
        paint: &ThePaint,
    ) {
        self.render_path(surface, path, Fill::NonZero, paint);
    }

    pub fn stroke_path(
        &mut self,
        surface: &mut TheSurfaceMut<'_>,
        path: &[ThePathCommand],
        stroke: &ThePathStroke,
    ) {
        let mut style = Stroke::new(stroke.width.max(0.0));
        style.cap(match stroke.cap {
            TheLineCap::Butt => Cap::Butt,
            TheLineCap::Square => Cap::Square,
            TheLineCap::Round => Cap::Round,
        });
        style.join(match stroke.join {
            TheLineJoin::Bevel => Join::Bevel,
            TheLineJoin::Miter => Join::Miter,
            TheLineJoin::Round => Join::Round,
        });
        self.render_path(surface, path, style, &stroke.paint);
    }

    pub fn fill_round_rect(
        &mut self,
        surface: &mut TheSurfaceMut<'_>,
        rect: ThePixelRect,
        radius: f32,
        paint: &ThePaint,
    ) {
        if rect.is_empty() {
            return;
        }
        let mut path = ThePath::new();
        path.add_round_rect(
            (rect.x as f32, rect.y as f32),
            rect.width as f32,
            rect.height as f32,
            radius,
            radius,
        );
        self.fill_path(surface, &path, paint);
    }

    pub fn fill_rect(
        &mut self,
        surface: &mut TheSurfaceMut<'_>,
        rect: ThePixelRect,
        paint: &ThePaint,
    ) {
        if rect.is_empty() {
            return;
        }
        if let ThePaint::Solid(color) = paint {
            surface.fill_rect(rect, *color);
            return;
        }
        let mut path = ThePath::new();
        path.add_rect(
            (rect.x as f32, rect.y as f32),
            rect.width as f32,
            rect.height as f32,
        );
        self.fill_path(surface, &path, paint);
    }

    pub fn fill_circle(
        &mut self,
        surface: &mut TheSurfaceMut<'_>,
        center: [f32; 2],
        radius: f32,
        paint: &ThePaint,
    ) {
        if radius <= 0.0 {
            return;
        }
        let mut path = ThePath::new();
        path.add_circle(center, radius);
        self.fill_path(surface, &path, paint);
    }

    fn render_path<'a>(
        &mut self,
        surface: &mut TheSurfaceMut<'_>,
        path: &[ThePathCommand],
        style: impl Into<zeno::Style<'a>>,
        paint: &ThePaint,
    ) {
        if path.is_empty() {
            return;
        }

        let Self { scratch, mask } = self;
        let mut rasterizer = Mask::with_scratch(path, scratch);
        rasterizer.style(style);
        rasterizer.inspect(|format, width, height| {
            mask.resize(format.buffer_size(width, height), 0);
            mask.fill(0);
        });
        let placement = rasterizer.render_into(mask, None);

        let mask_width = placement.width as usize;
        for mask_y in 0..placement.height as usize {
            for mask_x in 0..mask_width {
                let coverage = mask[mask_y * mask_width + mask_x];
                if coverage == 0 {
                    continue;
                }
                let x = placement.left + mask_x as i32;
                let y = placement.top + mask_y as i32;
                let color = paint.sample(x as f32 + 0.5, y as f32 + 0.5);
                surface.blend_pixel_coverage(x, y, color, coverage);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zeno_paths_respect_surface_clip_and_guard_bytes() {
        const GUARD: u8 = 0xcd;
        let mut storage = vec![GUARD; 16 * 12 * 4 + 37];
        {
            let mut surface = TheSurfaceMut::new(&mut storage, 16, 12).unwrap();
            let clip = surface.set_clip(ThePixelRect::new(3, 2, 8, 7));
            let mut painter = ThePainter::new();
            painter.fill_round_rect(
                &mut surface,
                ThePixelRect::new(-8, -9, 40, 35),
                7.5,
                &ThePaint::solid([18, 42, 79, 255]),
            );

            for y in 0..12 {
                for x in 0..16 {
                    if !clip.contains(x, y) {
                        assert_eq!(surface.pixel(x, y), Some([GUARD; 4]));
                    }
                }
            }
        }
        assert!(storage[16 * 12 * 4..].iter().all(|byte| *byte == GUARD));
    }

    #[test]
    fn gradient_is_sampled_in_surface_coordinates() {
        let paint = ThePaint::linear_gradient(
            [0.0, 0.0],
            [10.0, 0.0],
            [0, 10, 20, 255],
            [100, 110, 120, 255],
        );
        assert_eq!(paint.sample(0.0, 4.0), [0, 10, 20, 255]);
        assert_eq!(paint.sample(5.0, 4.0), [50, 60, 70, 255]);
        assert_eq!(paint.sample(20.0, 4.0), [100, 110, 120, 255]);
    }

    #[test]
    fn stroked_path_is_antialiased() {
        let mut storage = vec![0; 24 * 24 * 4];
        let mut surface = TheSurfaceMut::new(&mut storage, 24, 24).unwrap();
        let mut path = ThePath::new();
        path.move_to((2.25, 3.75)).line_to((20.5, 17.25));
        ThePainter::new().stroke_path(
            &mut surface,
            &path,
            &ThePathStroke::new(2.25, ThePaint::solid([255, 255, 255, 255]))
                .with_cap(TheLineCap::Round),
        );
        assert!(
            storage
                .chunks_exact(4)
                .any(|pixel| pixel[3] > 0 && pixel[3] < 255)
        );
    }
}
