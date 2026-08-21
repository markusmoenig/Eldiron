use crate::prelude::*;

pub(crate) fn draw_time_slider_chrome(
    buffer: &mut TheRGBABuffer,
    bounds: ThePixelRect,
    track_width: i32,
    position_offset: Option<i32>,
    markers: &[(i32, bool)],
    style: &mut Box<dyn TheStyle>,
    ctx: &mut TheContext,
) {
    let background = style.theme().paint(TimeSliderBackgroundChrome, bounds);
    let marker = style.theme().paint(TimeSliderMarkerChrome, bounds);
    let position = style.theme().paint(TimeSliderPositionChrome, bounds);
    let border = *style.theme().color(TimeSliderBorder);
    let tick = *style.theme().color(TimeSliderLine);
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;

    draw_time_slider_chrome_pixels(
        buffer.pixels_mut(),
        width,
        height,
        bounds,
        track_width,
        position_offset,
        markers,
        &background,
        &marker,
        &position,
        border,
        tick,
        &mut ctx.painter,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_time_slider_chrome_pixels(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    bounds: ThePixelRect,
    track_width: i32,
    position_offset: Option<i32>,
    markers: &[(i32, bool)],
    background: &ThePaint,
    marker: &ThePaint,
    position: &ThePaint,
    border: RGBA,
    tick: RGBA,
    painter: &mut ThePainter,
) {
    let Ok(mut surface) = TheSurfaceMut::new(pixels, width, height) else {
        return;
    };
    surface.set_clip(bounds);

    surface.fill_rect(bounds, border);
    let inner = ThePixelRect::new(
        bounds.x.saturating_add(1),
        bounds.y.saturating_add(1),
        bounds.width.saturating_sub(2),
        bounds.height.saturating_sub(2),
    );
    painter.fill_rect(&mut surface, inner, background);

    let track_width = track_width.max(0).min(bounds.width.max(0));
    if track_width == 0 {
        return;
    }

    for hour in 1..=24 {
        let offset = track_width.saturating_mul(hour) / 24;
        surface.fill_rect(
            ThePixelRect::new(
                bounds.x.saturating_add(offset),
                bounds.y.saturating_add(bounds.height).saturating_sub(4),
                2,
                2,
            ),
            tick,
        );
    }

    if let Some(offset) = position_offset {
        painter.fill_rect(
            &mut surface,
            ThePixelRect::new(
                bounds.x.saturating_add(offset.clamp(0, track_width)),
                bounds.y.saturating_add(1),
                2,
                bounds.height.saturating_sub(2),
            ),
            position,
        );
    }

    for &(offset, selected) in markers {
        let offset = offset.clamp(0, track_width);
        let paint = if selected { position } else { marker };
        painter.fill_rect(
            &mut surface,
            ThePixelRect::new(
                bounds.x.saturating_add(offset),
                bounds.y.saturating_add(1),
                2,
                bounds.height.saturating_sub(2),
            ),
            paint,
        );
        painter.fill_rect(
            &mut surface,
            ThePixelRect::new(
                bounds.x.saturating_add(offset),
                bounds.y.saturating_add(1),
                10,
                bounds.height.saturating_sub(2).min(12),
            ),
            paint,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_slider_chrome_clips_and_preserves_guard_bytes() {
        const WIDTH: usize = 11;
        const HEIGHT: usize = 8;
        const GUARD: usize = 37;
        const SENTINEL: u8 = 0xd7;

        let body_len = WIDTH * HEIGHT * 4;
        let mut pixels = vec![0; body_len + GUARD];
        pixels[body_len..].fill(SENTINEL);
        draw_time_slider_chrome_pixels(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ThePixelRect::new(-9, -5, 36, 27),
            29,
            Some(26),
            &[(1, false), (27, true)],
            &ThePaint::linear_gradient(
                [0.0, 0.0],
                [0.0, 27.0],
                [43, 46, 50, 255],
                [22, 24, 27, 255],
            ),
            &ThePaint::solid([57, 75, 105, 255]),
            &ThePaint::solid([126, 181, 225, 255]),
            [82, 86, 91, 255],
            [68, 71, 75, 255],
            &mut ThePainter::new(),
        );

        assert!(pixels[..body_len].iter().any(|byte| *byte != 0));
        assert!(pixels[body_len..].iter().all(|byte| *byte == SENTINEL));
    }
}
