use crate::prelude::*;

pub(crate) fn draw_button_chrome(
    buffer: &mut TheRGBABuffer,
    bounds: ThePixelRect,
    fill_role: TheThemePaints,
    border: RGBA,
    style: &mut Box<dyn TheStyle>,
    ctx: &mut TheContext,
) {
    let fill = style.theme().paint(fill_role, bounds);
    let radius = style.theme().metric(ControlCornerRadius);
    let border = ThePaint::solid(border);
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    draw_button_chrome_pixels(
        buffer.pixels_mut(),
        width,
        height,
        bounds,
        radius,
        &fill,
        &border,
        &mut ctx.painter,
    );
}

fn draw_button_chrome_pixels(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    bounds: ThePixelRect,
    radius: f32,
    fill: &ThePaint,
    border: &ThePaint,
    painter: &mut ThePainter,
) {
    let inner = ThePixelRect::new(
        bounds.x.saturating_add(1),
        bounds.y.saturating_add(1),
        bounds.width.saturating_sub(2),
        bounds.height.saturating_sub(2),
    );
    let Ok(mut surface) = TheSurfaceMut::new(pixels, width, height) else {
        return;
    };
    surface.set_clip(bounds);
    painter.fill_round_rect(&mut surface, bounds, radius, border);
    painter.fill_round_rect(&mut surface, inner, (radius - 1.0).max(0.0), fill);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_chrome_clips_and_preserves_guard_bytes() {
        const WIDTH: usize = 8;
        const HEIGHT: usize = 7;
        const GUARD: usize = 32;
        const SENTINEL: u8 = 0x9d;

        let body_len = WIDTH * HEIGHT * 4;
        let mut pixels = vec![0; body_len + GUARD];
        pixels[body_len..].fill(SENTINEL);
        draw_button_chrome_pixels(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ThePixelRect::new(-9, -6, 24, 20),
            2.0,
            &ThePaint::solid([42, 48, 57, 255]),
            &ThePaint::solid([88, 96, 108, 255]),
            &mut ThePainter::new(),
        );

        assert!(pixels[..body_len].iter().any(|byte| *byte != 0));
        assert!(pixels[body_len..].iter().all(|byte| *byte == SENTINEL));
    }
}
