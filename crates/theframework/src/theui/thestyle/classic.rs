use crate::prelude::*;

pub struct TheClassicStyle {
    dark: Box<dyn TheTheme>,
}

impl TheClassicStyle {
    pub fn with_theme(theme: Box<dyn TheTheme>) -> Self {
        Self { dark: theme }
    }
}

/// Implements TheStyle trait for the default Classic look.
impl TheStyle for TheClassicStyle {
    fn new() -> Self
    where
        Self: Sized,
    {
        let dark = Box::new(TheDarkTheme::new());
        Self { dark }
    }

    fn theme(&mut self) -> &mut Box<dyn TheTheme> {
        &mut self.dark
    }

    fn set_theme(&mut self, theme: Box<dyn TheTheme>) {
        self.dark = theme;
    }

    fn draw_widget_border(
        &mut self,
        buffer: &mut TheRGBABuffer,
        widget: &mut dyn TheWidget,
        shrinker: &mut TheDimShrinker,
        ctx: &mut TheContext,
    ) {
        let stride = buffer.stride();

        let border_color = if widget.id().equals(&ctx.ui.focus) {
            self.theme().color(DefaultSelection)
        } else {
            self.theme().color(DefaultWidgetBorder)
        };

        ctx.draw.rect_outline(
            buffer.pixels_mut(),
            &widget.dim().to_buffer_shrunk_utuple(shrinker),
            stride,
            border_color,
        );

        shrinker.shrink(2);
    }

    fn draw_text_edit_border(
        &mut self,
        buffer: &mut TheRGBABuffer,
        widget: &dyn TheWidget,
        shrinker: &mut TheDimShrinker,
        ctx: &mut TheContext,
        embedded: bool,
        disabled: bool,
    ) {
        let focused = widget.id().equals(&ctx.ui.focus);
        let radius = self.theme().metric(ControlCornerRadius);

        if focused {
            if !embedded {
                let rect = text_input_rect(widget.dim(), shrinker);
                let paint = if disabled {
                    ThePaint::solid(*self.theme().color_disabled(SelectedTextEditBorder1))
                } else {
                    self.theme().paint(Focus, rect)
                };
                paint_round_rect(buffer, rect, radius, &paint, ctx);
            }

            shrinker.shrink(1);

            let rect = text_input_rect(widget.dim(), shrinker);
            let border = if !embedded {
                if disabled {
                    *self.theme().color_disabled(SelectedTextEditBorder2)
                } else {
                    *self.theme().color(SelectedTextEditBorder2)
                }
            } else if disabled {
                *self.theme().color_disabled(TextEditBorder)
            } else {
                *self.theme().color(TextEditBorder)
            };
            paint_round_rect(
                buffer,
                rect,
                (radius - 1.0).max(0.0),
                &ThePaint::solid(border),
                ctx,
            );
            shrinker.shrink(1);
        } else {
            shrinker.shrink(1);
            let rect = text_input_rect(widget.dim(), shrinker);
            let border = if disabled {
                *self.theme().color_disabled(TextEditBorder)
            } else {
                *self.theme().color(TextEditBorder)
            };
            paint_round_rect(
                buffer,
                rect,
                (radius - 1.0).max(0.0),
                &ThePaint::solid(border),
                ctx,
            );
            shrinker.shrink(1);
        }
    }

    fn draw_text_area_border(
        &mut self,
        buffer: &mut TheRGBABuffer,
        widget: &dyn TheWidget,
        shrinker: &mut TheDimShrinker,
        ctx: &mut TheContext,
        embedded: bool,
        disabled: bool,
    ) {
        let focused = widget.id().equals(&ctx.ui.focus);
        let rect = text_input_rect(widget.dim(), shrinker);
        let paint = if focused && !embedded {
            if disabled {
                ThePaint::solid(*self.theme().color_disabled(SelectedTextEditBorder1))
            } else {
                self.theme().paint(Focus, rect)
            }
        } else if disabled {
            ThePaint::solid(*self.theme().color_disabled(TextEditBorder))
        } else {
            ThePaint::solid(*self.theme().color(TextEditBorder))
        };
        paint_round_rect(buffer, rect, 1.0, &paint, ctx);

        if focused {
            if !embedded {
                shrinker.shrink(1);
            } else {
                shrinker.shrink(1);
            }
        } else {
            shrinker.shrink(1);
        }
    }
}

fn text_input_rect(dim: &TheDim, shrinker: &TheDimShrinker) -> ThePixelRect {
    ThePixelRect::new(
        dim.buffer_x.saturating_add(shrinker.left),
        dim.buffer_y.saturating_add(shrinker.top),
        dim.width
            .saturating_sub(shrinker.left)
            .saturating_sub(shrinker.right),
        dim.height
            .saturating_sub(shrinker.top)
            .saturating_sub(shrinker.bottom),
    )
}

fn paint_round_rect(
    buffer: &mut TheRGBABuffer,
    rect: ThePixelRect,
    radius: f32,
    paint: &ThePaint,
    ctx: &mut TheContext,
) {
    let width = buffer.dim().width.max(0) as usize;
    let height = buffer.dim().height.max(0) as usize;
    if let Ok(mut surface) = TheSurfaceMut::new(buffer.pixels_mut(), width, height) {
        surface.set_clip(rect);
        ctx.painter
            .fill_round_rect(&mut surface, rect, radius, paint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_text_input_border_clips_negative_dimensions() {
        let mut buffer = TheRGBABuffer::new(TheDim::sized(7, 6));
        let mut widget = TheTextLineEdit::new(TheId::named("Clipped Input"));
        let mut ctx = TheContext::new(7, 6, 1.0);
        widget.set_dim(TheDim::rect(-9, -7, 22, 18), &mut ctx);
        ctx.ui.set_focus(widget.id());

        let mut style = TheClassicStyle::with_theme(Box::new(TheBlackBlueTheme::new()));
        let mut shrinker = TheDimShrinker::zero();
        style.draw_text_edit_border(&mut buffer, &widget, &mut shrinker, &mut ctx, false, false);

        assert!(
            shrinker
                == TheDimShrinker {
                    left: 2,
                    top: 2,
                    right: 2,
                    bottom: 2,
                }
        );
        assert!(buffer.pixels().chunks_exact(4).any(|pixel| pixel[3] != 0));
    }
}
