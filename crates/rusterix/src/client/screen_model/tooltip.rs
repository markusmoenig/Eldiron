//! Shared tooltip placement. Text measurement and scheduling belong to hosts.
use crate::Rect;
use vek::Vec2;
pub fn tooltip_bounds(
    anchor: Rect,
    width: f32,
    height: f32,
    viewport: Vec2<i32>,
    below: bool,
) -> Rect {
    let width = width.max(0.).min((viewport.x - 4).max(0) as f32);
    let height = height.max(0.).min((viewport.y - 4).max(0) as f32);
    let (mut x, mut y) = if below {
        (
            (anchor.x + (anchor.width - width) * 0.5).round(),
            (anchor.y + anchor.height + 8.).round(),
        )
    } else {
        let right = (anchor.x + anchor.width + 8.).round();
        (
            if right + width <= viewport.x as f32 {
                right
            } else {
                (anchor.x.round() - width - 8.).max(2.)
            },
            (anchor.y + 2.).round(),
        )
    };
    if x + width > viewport.x as f32 {
        x = (viewport.x as f32 - width - 2.).max(2.);
    }
    x = x.max(2.);
    if y + height > viewport.y as f32 {
        y = if below {
            (anchor.y.round() - height - 8.).max(2.)
        } else {
            (viewport.y as f32 - height - 2.).max(2.)
        };
    }
    x = x.clamp(2., (viewport.x as f32 - width - 2.).max(2.));
    y = y.clamp(2., (viewport.y as f32 - height - 2.).max(2.));
    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tooltips_flip_around_edges_and_clip_to_tiny_viewports_without_font_or_pixel_storage() {
        let anchor = Rect::new(360., 220., 32., 20.);
        let below = tooltip_bounds(anchor, 200., 120., Vec2::new(400, 240), true);
        assert!(below.y + below.height < anchor.y && below.x + below.width <= 400.);
        let side = tooltip_bounds(anchor, 200., 120., Vec2::new(400, 240), false);
        assert!(side.x + side.width < anchor.x && side.y + side.height <= 240.);
        let tiny = tooltip_bounds(anchor, 500., 500., Vec2::new(16, 12), true);
        assert!(
            tiny.x >= 0.
                && tiny.y >= 0.
                && tiny.x + tiny.width <= 16.
                && tiny.y + tiny.height <= 12.
        );
    }
}
