use crate::prelude::*;

#[derive(Clone, Copy)]
pub(crate) enum TheSliderChromeState {
    Normal,
    Hovered,
    Pressed,
}

pub(crate) fn draw_slider_chrome(
    buffer: &mut TheRGBABuffer,
    bounds: ThePixelRect,
    track_width: i32,
    thumb_offset: i32,
    state: TheSliderChromeState,
    style: &mut Box<dyn TheStyle>,
    ctx: &mut TheContext,
) {
    let track = style.theme().paint(SliderTrackChrome, bounds);
    let track_accent = style.theme().paint(SliderTrackAccent, bounds);
    let (thumb_role, border_role) = match state {
        TheSliderChromeState::Normal => (SliderThumbNormal, SliderSmallColor1),
        TheSliderChromeState::Hovered => (SliderThumbHover, SelectedTextEditBorder1),
        TheSliderChromeState::Pressed => (SliderThumbPressed, ToolbarButtonClickedBorder),
    };
    let thumb = style.theme().paint(thumb_role, bounds);
    let thumb_border = ThePaint::solid(*style.theme().color(border_role));
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    draw_slider_chrome_pixels(
        buffer.pixels_mut(),
        width,
        height,
        bounds,
        track_width,
        thumb_offset,
        &track,
        &track_accent,
        &thumb,
        &thumb_border,
        &mut ctx.painter,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_slider_chrome_pixels(
    pixels: &mut [u8],
    width: usize,
    height: usize,
    bounds: ThePixelRect,
    track_width: i32,
    thumb_offset: i32,
    track: &ThePaint,
    track_accent: &ThePaint,
    thumb: &ThePaint,
    thumb_border: &ThePaint,
    painter: &mut ThePainter,
) {
    let Ok(mut surface) = TheSurfaceMut::new(pixels, width, height) else {
        return;
    };
    surface.set_clip(bounds);

    let track_bounds = ThePixelRect::new(
        bounds.x.saturating_add(1),
        bounds.y.saturating_add(bounds.height / 2).saturating_sub(1),
        track_width.saturating_sub(2),
        3,
    );
    let track_inner = ThePixelRect::new(
        track_bounds.x.saturating_add(1),
        track_bounds.y.saturating_add(1),
        track_bounds.width.saturating_sub(2),
        1,
    );
    painter.fill_round_rect(&mut surface, track_bounds, 1.5, track);
    painter.fill_round_rect(&mut surface, track_inner, 0.5, track_accent);

    let thumb_bounds = ThePixelRect::new(
        bounds.x.saturating_add(thumb_offset),
        bounds.y,
        5,
        bounds.height.min(13),
    );
    let thumb_inner = ThePixelRect::new(
        thumb_bounds.x.saturating_add(1),
        thumb_bounds.y.saturating_add(1),
        thumb_bounds.width.saturating_sub(2),
        thumb_bounds.height.saturating_sub(2),
    );
    painter.fill_round_rect(&mut surface, thumb_bounds, 2.0, thumb_border);
    painter.fill_round_rect(&mut surface, thumb_inner, 1.0, thumb);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_chrome_clips_and_preserves_guard_bytes() {
        const WIDTH: usize = 8;
        const HEIGHT: usize = 7;
        const GUARD: usize = 32;
        const SENTINEL: u8 = 0xb3;

        let body_len = WIDTH * HEIGHT * 4;
        let mut pixels = vec![0; body_len + GUARD];
        pixels[body_len..].fill(SENTINEL);
        draw_slider_chrome_pixels(
            &mut pixels,
            WIDTH,
            HEIGHT,
            ThePixelRect::new(-7, -4, 30, 13),
            22,
            10,
            &ThePaint::solid([25, 27, 30, 255]),
            &ThePaint::solid([83, 88, 94, 255]),
            &ThePaint::solid([91, 120, 163, 255]),
            &ThePaint::solid([83, 151, 207, 255]),
            &mut ThePainter::new(),
        );

        assert!(pixels[..body_len].iter().any(|byte| *byte != 0));
        assert!(pixels[body_len..].iter().all(|byte| *byte == SENTINEL));
    }
}
