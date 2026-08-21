use crate::prelude::*;

pub(crate) struct TheDropdownChromeState {
    pub disabled: bool,
    pub pressed: bool,
    pub hovered: bool,
    pub focused: bool,
}

pub(crate) fn draw_dropdown_chrome(
    buffer: &mut TheRGBABuffer,
    dim: &TheDim,
    state: TheDropdownChromeState,
    style: &mut Box<dyn TheStyle>,
    ctx: &mut TheContext,
) {
    let rect = ThePixelRect::new(dim.buffer_x, dim.buffer_y, dim.width, dim.height);
    let (paint_role, border_role) = if state.disabled {
        (DropdownDisabled, TextEditBorder)
    } else if state.pressed {
        (DropdownPressed, ToolbarButtonClickedBorder)
    } else if state.focused {
        (DropdownFocus, SelectedTextEditBorder1)
    } else if state.hovered {
        (DropdownHover, ToolbarButtonHoverBorder)
    } else {
        (DropdownNormal, TextEditBorder)
    };
    let paint = style.theme().paint(paint_role, rect);
    let marker = style.theme().paint(DropdownMarker, rect);
    let border = if state.disabled {
        *style.theme().color_disabled(border_role)
    } else {
        *style.theme().color(border_role)
    };
    let radius = style.theme().metric(ControlCornerRadius);
    let inner = ThePixelRect::new(
        rect.x.saturating_add(1),
        rect.y.saturating_add(1),
        rect.width.saturating_sub(2),
        rect.height.saturating_sub(2),
    );
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    if let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), width, height) {
        surface.set_clip(rect);
        ctx.painter
            .fill_round_rect(&mut surface, rect, radius, &ThePaint::solid(border));
        ctx.painter
            .fill_round_rect(&mut surface, inner, (radius - 1.0).max(0.0), &paint);

        let marker_x = rect.x.saturating_add(rect.width).saturating_sub(9) as f32;
        let marker_y = rect.y.saturating_add(rect.height / 2) as f32;
        let mut path = ThePath::new();
        path.move_to((marker_x - 3.5, marker_y - 1.5))
            .line_to((marker_x + 3.5, marker_y - 1.5))
            .line_to((marker_x, marker_y + 2.5))
            .close();
        ctx.painter.fill_path(&mut surface, &path, &marker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropdown_chrome_and_marker_clip_to_small_buffers() {
        let mut buffer = TheRGBABuffer::new(TheDim::sized(7, 6));
        let dim = TheDim::rect(-11, -8, 142, 20);
        let mut style: Box<dyn TheStyle> = Box::new(TheClassicStyle::with_theme(Box::new(
            TheBlackBlueTheme::new(),
        )));
        let mut ctx = TheContext::new(7, 6, 1.0);

        draw_dropdown_chrome(
            &mut buffer,
            &dim,
            TheDropdownChromeState {
                disabled: false,
                pressed: false,
                hovered: true,
                focused: false,
            },
            &mut style,
            &mut ctx,
        );

        assert!(buffer.pixels().chunks_exact(4).any(|pixel| pixel[3] != 0));
    }
}
