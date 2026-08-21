use crate::prelude::*;

pub(crate) fn draw_scrollbar_chrome(
    buffer: &mut TheRGBABuffer,
    bounds: ThePixelRect,
    thumb: ThePixelRect,
    hovered: bool,
    pressed: bool,
    style: &mut Box<dyn TheStyle>,
    ctx: &mut TheContext,
) {
    let thumb_role = if pressed {
        ScrollbarThumbPressed
    } else if hovered {
        ScrollbarThumbHover
    } else {
        ScrollbarThumbNormal
    };
    let border_role = if pressed {
        ToolbarButtonClickedBorder
    } else if hovered {
        ToolbarButtonHoverBorder
    } else {
        DefaultWidgetBorder
    };
    let track = style.theme().paint(ScrollbarTrack, bounds);
    let thumb_paint = style.theme().paint(thumb_role, thumb);
    let border = ThePaint::solid(*style.theme().color(border_role));
    let radius = thumb.width.min(thumb.height).max(0) as f32 * 0.5;
    let inner = ThePixelRect::new(
        thumb.x.saturating_add(1),
        thumb.y.saturating_add(1),
        thumb.width.saturating_sub(2),
        thumb.height.saturating_sub(2),
    );

    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    if let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), width, height) {
        surface.set_clip(bounds);
        ctx.painter.fill_rect(&mut surface, bounds, &track);
        ctx.painter
            .fill_round_rect(&mut surface, thumb, radius, &border);
        ctx.painter
            .fill_round_rect(&mut surface, inner, (radius - 1.0).max(0.0), &thumb_paint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollbar_chrome_clips_to_its_surface() {
        let mut buffer = TheRGBABuffer::new(TheDim::sized(8, 7));
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
            TheBlackBlueTheme::new(),
        )));
        let mut ctx = TheContext::new(8, 7, 1.0);

        draw_scrollbar_chrome(
            &mut buffer,
            ThePixelRect::new(-5, -4, 20, 18),
            ThePixelRect::new(-2, -1, 13, 6),
            true,
            false,
            &mut style,
            &mut ctx,
        );

        assert!(buffer.pixels().chunks_exact(4).any(|pixel| pixel[3] != 0));
    }
}
