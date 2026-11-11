use crate::prelude::*;
use crate::thecontext::TheCursorIcon;

pub struct TheCheckButton {
    id: TheId,
    limiter: TheSizeLimiter,
    status: Option<String>,

    state: TheWidgetState,

    dim: TheDim,
    is_dirty: bool,
    cursor_icon: Option<TheCursorIcon>,
    embedded: bool,
}

impl TheWidget for TheCheckButton {
    #[cfg(feature = "text")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::Rect;
        use crate::theui::paint::bounds;
        let r = bounds(&self.dim);
        use crate::paint::{LineCap, LineJoin, PaintOp, Path, PathElement, Stroke};
        use std::sync::Arc;
        let selected = self.state == TheWidgetState::Selected;
        let highlighted =
            !self.embedded && (self.id.equals(&p.ui.hover) || self.id.equals(&p.ui.focus));
        let size = r.width.min(r.height).min(14.).max(0.);
        let b = Rect::new(r.x, r.y + (r.height - size) * 0.5, size, size);
        let role = if selected {
            CheckboxSelected
        } else if highlighted {
            CheckboxHover
        } else {
            CheckboxNormal
        };
        let border = if highlighted || selected {
            SelectedTextEditBorder1
        } else {
            TextEditBorder
        };
        p.button(b, 2.5, role, *p.theme.color(border));
        if selected && size >= 8. {
            let path = Path {
                elements: vec![
                    PathElement::Move([b.x + 3., b.y + 7.]),
                    PathElement::Line([b.x + 5.5, b.y + 9.5]),
                    PathElement::Line([b.x + 11., b.y + 4.]),
                ],
            };
            p.list.ops.push(PaintOp::Stroke {
                path: Arc::new(path),
                stroke: Stroke {
                    width: 1.8,
                    cap: LineCap::Round,
                    join: LineJoin::Round,
                    brush: p.theme_brush(b, CheckboxMark),
                },
            });
        }
        self.is_dirty = false;
        Ok(())
    }
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(16, 18));

        Self {
            id,
            limiter,
            status: None,

            state: TheWidgetState::None,

            dim: TheDim::zero(),
            is_dirty: false,
            cursor_icon: Some(TheCursorIcon::Hand),
            embedded: false,
        }
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        self.cursor_icon
    }

    fn set_cursor_icon(&mut self, icon: Option<TheCursorIcon>) {
        self.cursor_icon = icon;
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
    }

    fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::MouseDown(_coord) => {
                if self.state != TheWidgetState::Selected {
                    self.state = TheWidgetState::Selected;
                } else {
                    self.state = TheWidgetState::None;
                }

                ctx.ui.set_focus(self.id());
                ctx.ui.send_widget_state_changed(self.id(), self.state);
                ctx.ui.send_widget_value_changed(
                    self.id(),
                    TheValue::Bool(self.state == TheWidgetState::Selected),
                );
                self.is_dirty = true;
                redraw = true;
            }
            TheEvent::Hover(_coord) => {
                if self.state != TheWidgetState::Selected && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
            // TheEvent::MouseUp(_coord) => {
            //     self.is_dirty = true;
            //     redraw = true;
            // }
            _ => {}
        }
        redraw
    }

    fn dim(&self) -> &TheDim {
        &self.dim
    }

    fn dim_mut(&mut self) -> &mut TheDim {
        &mut self.dim
    }

    fn set_dim(&mut self, dim: TheDim, _ctx: &mut TheContext) {
        if self.dim != dim {
            self.dim = dim;
            self.is_dirty = true;
        }
    }

    fn limiter(&self) -> &TheSizeLimiter {
        &self.limiter
    }

    fn limiter_mut(&mut self) -> &mut TheSizeLimiter {
        &mut self.limiter
    }

    fn needs_redraw(&mut self) -> bool {
        self.is_dirty
    }

    fn set_needs_redraw(&mut self, redraw: bool) {
        self.is_dirty = redraw;
    }

    fn state(&self) -> TheWidgetState {
        self.state
    }

    fn set_state(&mut self, state: TheWidgetState) {
        self.state = state;
        self.is_dirty = true;
    }

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn value(&self) -> TheValue {
        if self.state == TheWidgetState::Selected {
            TheValue::Bool(true)
        } else {
            TheValue::Bool(false)
        }
    }

    fn set_value(&mut self, value: TheValue) {
        match value {
            TheValue::Bool(b) => {
                if b {
                    self.state = TheWidgetState::Selected;
                } else {
                    self.state = TheWidgetState::None;
                }
                self.is_dirty = true;
            }
            _ => {}
        }
    }

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let highlighted =
            !self.embedded && (self.id().equals(&ctx.ui.hover) || self.id().equals(&ctx.ui.focus));
        let selected = self.state == TheWidgetState::Selected;
        let size = self.dim.width.min(self.dim.height).min(14).max(0);
        let box_rect = ThePixelRect::new(
            self.dim.buffer_x,
            self.dim
                .buffer_y
                .saturating_add(self.dim.height.saturating_sub(size) / 2),
            size,
            size,
        );
        let inner = ThePixelRect::new(
            box_rect.x.saturating_add(1),
            box_rect.y.saturating_add(1),
            box_rect.width.saturating_sub(2),
            box_rect.height.saturating_sub(2),
        );
        let fill_role = if selected {
            CheckboxSelected
        } else if highlighted {
            CheckboxHover
        } else {
            CheckboxNormal
        };
        let fill = style.theme().paint(fill_role, box_rect);
        let mark = style.theme().paint(CheckboxMark, box_rect);
        let border = if highlighted || selected {
            *style.theme().color(SelectedTextEditBorder1)
        } else {
            *style.theme().color(TextEditBorder)
        };
        let width = buffer.dim().width.max(0) as usize;
        let height = buffer.dim().height.max(0) as usize;
        if let Ok(mut surface) = TheSurfaceMut::new(buffer.draw_target(), width, height) {
            surface.set_clip(ThePixelRect::new(
                self.dim.buffer_x,
                self.dim.buffer_y,
                self.dim.width,
                self.dim.height,
            ));
            ctx.painter
                .fill_round_rect(&mut surface, box_rect, 2.5, &ThePaint::solid(border));
            ctx.painter.fill_round_rect(&mut surface, inner, 1.5, &fill);

            if selected && size >= 8 {
                let mut check = ThePath::new();
                check
                    .move_to((box_rect.x as f32 + 3.0, box_rect.y as f32 + 7.0))
                    .line_to((box_rect.x as f32 + 5.5, box_rect.y as f32 + 9.5))
                    .line_to((box_rect.x as f32 + 11.0, box_rect.y as f32 + 4.0));
                ctx.painter.stroke_path(
                    &mut surface,
                    &check,
                    &ThePathStroke::new(1.8, mark)
                        .with_cap(TheLineCap::Round)
                        .with_join(TheLineJoin::Round),
                );
            }
        }

        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
