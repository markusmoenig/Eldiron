use crate::prelude::*;
use crate::thecontext::TheCursorIcon;
use crate::theui::thewidget::thedropdownchrome::{TheDropdownChromeState, draw_dropdown_chrome};

pub struct TheDropdownMenu {
    id: TheId,
    limiter: TheSizeLimiter,
    status: Option<String>,

    is_disabled: bool,

    state: TheWidgetState,

    options: Vec<String>,
    selected: i32,
    original: i32,

    dim: TheDim,
    is_dirty: bool,

    safety_offset: Vec2<i32>,
    embedded: bool,

    parent_id: Option<TheId>,
    overlay_offset: Vec2<i32>,
    #[cfg(feature = "render-wgpu")]
    gpu_overlay: crate::paint::Rect,
}

impl TheWidget for TheDropdownMenu {
    #[cfg(feature = "render-wgpu")]
    fn prepare_paint(&mut self, ctx: &mut TheContext) {
        if self.state != TheWidgetState::Clicked {
            return;
        }
        let transform = ctx
            .ui
            .input_transforms
            .get(&self.id.uuid)
            .copied()
            .unwrap_or_default();
        let anchor = transform.point([self.dim.x as f32, self.dim.y as f32]);
        let width = (self.dim.width.max(142) as f32).min(ctx.width as f32);
        let height =
            2. + self.options.len() as f32 * 20. + self.options.len().saturating_sub(1) as f32;
        let x = anchor[0].clamp(0., (ctx.width as f32 - width - 5.).max(0.));
        let y = (anchor[1] + 20.).clamp(0., (ctx.height as f32 - height - 5.).max(0.));
        self.safety_offset = Vec2::new((anchor[0] - x) as i32, (anchor[1] + 20. - y) as i32);
        self.overlay_offset = Vec2::new(x as i32, y as i32);
        self.gpu_overlay = crate::paint::Rect::new(x, y, width, height);
    }
    #[cfg(feature = "render-wgpu")]
    fn paint(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::{FillRule, PaintOp, Path, PathElement, Rect};
        use crate::theui::paint::bounds;
        use std::sync::Arc;
        let r = bounds(&self.dim);
        let interactive = !self.is_disabled && !self.embedded;
        let (role, border) = if self.is_disabled {
            (DropdownDisabled, TextEditBorder)
        } else if !self.embedded && self.state == TheWidgetState::Clicked {
            (DropdownPressed, ToolbarButtonClickedBorder)
        } else if interactive && self.id.equals(&p.ui.focus) {
            (DropdownFocus, SelectedTextEditBorder1)
        } else if interactive && self.id.equals(&p.ui.hover) {
            (DropdownHover, ToolbarButtonHoverBorder)
        } else {
            (DropdownNormal, TextEditBorder)
        };
        let border = if self.is_disabled {
            *p.theme.color_disabled(border)
        } else {
            *p.theme.color(border)
        };
        p.button(r, p.theme.metric(ControlCornerRadius), role, border);
        let x = r.x + r.width - 9.;
        let y = r.y + (r.height * 0.5).floor();
        p.list.ops.push(PaintOp::Fill {
            path: Arc::new(Path {
                elements: vec![
                    PathElement::Move([x - 3.5, y - 1.5]),
                    PathElement::Line([x + 3.5, y - 1.5]),
                    PathElement::Line([x, y + 2.5]),
                    PathElement::Close,
                ],
            }),
            rule: FillRule::NonZero,
            brush: p.theme_brush(r, DropdownMarker),
        });
        if let Some(text) = self.options.get(self.selected.max(0) as usize) {
            p.label(
                Rect::new(r.x + 8., r.y, (r.width - 20.).max(0.), r.height),
                text,
                12.5,
                *p.theme.color(if self.state == TheWidgetState::Selected {
                    SectionbarSelectedTextColor
                } else {
                    SectionbarNormalTextColor
                }),
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
        }
        self.is_dirty = false;
        Ok(())
    }
    #[cfg(feature = "render-wgpu")]
    fn paint_overlay(
        &mut self,
        p: &mut crate::theui::paint::WidgetPainter<'_>,
    ) -> crate::theui::paint::UiPaintResult {
        use crate::paint::Rect;
        let r = self.gpu_overlay;
        p.fill(r, MenubarPopupBackground);
        p.outline(r, *p.theme.color(MenubarPopupBorder), 1.);
        for (i, text) in self.options.iter().enumerate() {
            let row = Rect::new(r.x, r.y + i as f32 * 21., r.width, 21.);
            if i == self.selected as usize {
                p.fill(row, DefaultSelection);
            }
            p.outline(row, *p.theme.color(MenubarPopupBorder), 1.);
            p.label(
                Rect::new(row.x + 8., row.y, (row.width - 8.).max(0.), row.height),
                text,
                12.5,
                *p.theme.color(SectionbarNormalTextColor),
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
        }
        Ok(())
    }

    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_width(142);
        limiter.set_max_height(20);

        Self {
            id,
            limiter,
            status: None,

            is_disabled: false,

            state: TheWidgetState::None,

            options: vec![],
            selected: 0,
            original: 0,

            dim: TheDim::zero(),
            is_dirty: false,

            safety_offset: Vec2::new(0, 0),
            embedded: false,

            parent_id: None,
            overlay_offset: Vec2::zero(),
            #[cfg(feature = "render-wgpu")]
            gpu_overlay: crate::paint::Rect::default(),
        }
    }

    fn cursor_icon(&self) -> Option<TheCursorIcon> {
        Some(TheCursorIcon::Hand)
    }

    fn set_cursor_icon(&mut self, _icon: Option<TheCursorIcon>) {
        // Store the cursor icon if needed for dynamic changes
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn value(&self) -> TheValue {
        TheValue::Int(self.selected)
    }

    fn set_value(&mut self, value: TheValue) {
        if let Some(index) = value.to_i32() {
            if index >= 0 && index < self.options.len() as i32 {
                self.selected = index;
                self.is_dirty = true;
            }
        }
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
    }

    fn disabled(&self) -> bool {
        self.is_disabled
    }

    fn set_disabled(&mut self, disabled: bool) {
        if disabled != self.is_disabled {
            self.is_disabled = disabled;
            self.is_dirty = true;
        }
    }

    fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
    }

    fn set_parent_id(&mut self, parent_id: TheId) {
        self.parent_id = Some(parent_id);
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        if self.is_disabled {
            return false;
        }
        match event {
            TheEvent::MouseDown(_coord) => {
                self.is_dirty = true;
                if self.state != TheWidgetState::Clicked {
                    self.state = TheWidgetState::Clicked;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.set_focus(self.id());

                    // When embedded, set overlay to parent so parent's draw_overlay is called
                    if let Some(parent_id) = &self.parent_id {
                        ctx.ui.set_overlay(parent_id);
                    } else {
                        ctx.ui.set_overlay(self.id());
                    }

                    self.original = self.selected;
                }
                redraw = true;
            }
            TheEvent::MouseDragged(coord) => {
                if !self.options.is_empty() {
                    let y: i32 = coord.y - 20 + self.safety_offset.y;
                    if y >= 0 {
                        let index = y / 21;
                        if index < self.options.len() as i32 && index != self.selected {
                            self.selected = index;
                        }
                    }
                    self.is_dirty = true;
                    redraw = true;
                }
            }
            TheEvent::MouseUp(_coord) => {
                self.is_dirty = true;
                if self.state == TheWidgetState::Clicked {
                    self.state = TheWidgetState::None;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.clear_overlay();

                    if self.selected != self.original {
                        ctx.ui
                            .send_widget_value_changed(self.id(), TheValue::Int(self.selected));
                        ctx.ui.send(TheEvent::IndexChanged(
                            self.id().clone(),
                            self.selected as usize,
                        ));
                    }
                }
                redraw = true;
            }
            TheEvent::Hover(_coord) => {
                if self.state != TheWidgetState::Clicked && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
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

    fn draw(
        &mut self,
        buffer: &mut TheRGBABuffer,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) {
        if !self.dim().is_valid() {
            return;
        }

        let interactive = !self.is_disabled && !self.embedded;
        draw_dropdown_chrome(
            buffer,
            &self.dim,
            TheDropdownChromeState {
                disabled: self.is_disabled,
                pressed: !self.embedded && self.state == TheWidgetState::Clicked,
                hovered: interactive
                    && self.state != TheWidgetState::Clicked
                    && self.id().equals(&ctx.ui.hover),
                focused: interactive
                    && self.state != TheWidgetState::Clicked
                    && self.id().equals(&ctx.ui.focus),
            },
            style,
            ctx,
        );
        let text_color = if self.state == TheWidgetState::Selected {
            *style.theme().color(SectionbarSelectedTextColor)
        } else {
            *style.theme().color(SectionbarNormalTextColor)
        };
        let stride = buffer.stride();
        let mut shrinker = TheDimShrinker::zero();
        shrinker.shrink_by(8, 0, 12, 0);

        if !self.options.is_empty() {
            ctx.draw.text_rect_blend(
                buffer.draw_target(),
                &self.dim.to_buffer_shrunk_utuple(&shrinker),
                stride,
                self.options[self.selected as usize].as_str(),
                TheFontSettings {
                    size: 12.5,
                    ..Default::default()
                },
                &text_color,
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );
        }

        self.is_dirty = false;
    }

    fn draw_overlay(
        &mut self,
        style: &mut Box<dyn TheStyle>,
        ctx: &mut TheContext,
    ) -> TheRGBABuffer {
        let len = self.options.len();
        let width = self.dim.width.max(142) as usize;

        #[allow(clippy::implicit_saturating_sub)]
        let height = 2 + len * 20 + (if len > 1 { len - 1 } else { 0 });

        let mut dim = TheDim::new(self.dim.x, self.dim.y + 20, width as i32, height as i32);
        self.safety_offset = Vec2::zero();

        // Safety check for width being larger than the window width
        if dim.x + width as i32 > ctx.width as i32 {
            self.safety_offset.x = dim.x + width as i32 - ctx.width as i32 + 5;
            dim.x -= self.safety_offset.x;
        }

        // Safety check for height being larger than the window height
        if dim.y + height as i32 > ctx.height as i32 {
            self.safety_offset.y = dim.y + height as i32 - ctx.height as i32 + 5;
            dim.y -= self.safety_offset.y;
        }

        // The overlay buffer must follow the safety-adjusted screen position. Keeping the
        // original buffer offset draws the popup in one place while hit testing uses another.
        dim.buffer_x = dim.x;
        dim.buffer_y = dim.y;

        // Store the overlay offset (will be adjusted by parent if embedded).
        self.overlay_offset = Vec2::new(dim.x, dim.y);

        let mut buffer = TheRGBABuffer::new(dim);
        buffer.set_render_scale(ctx.ui_render_scale);
        ctx.draw.rect(
            buffer.draw_target(),
            &(0, 0, width, height),
            width,
            style.theme().color(MenubarPopupBackground),
        );

        ctx.draw.rect_outline(
            buffer.draw_target(),
            &(0, 0, width, height),
            width,
            style.theme().color(MenubarPopupBorder),
        );

        let x = 0;
        let mut y = 0;

        for i in 0..len {
            if i == self.selected as usize {
                ctx.draw.rect(
                    buffer.draw_target(),
                    &(x, y, width, 21),
                    width,
                    style.theme().color(DefaultSelection),
                );
            }

            ctx.draw.rect_outline(
                buffer.draw_target(),
                &(x, y, width, 21),
                width,
                style.theme().color(MenubarPopupBorder),
            );

            if !self.options.is_empty() {
                ctx.draw.text_rect_blend(
                    buffer.draw_target(),
                    &(x + 8, y, width - 8, 21),
                    width,
                    self.options[i].as_str(),
                    TheFontSettings {
                        size: 12.5,
                        ..Default::default()
                    },
                    style.theme().color(SectionbarNormalTextColor),
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }

            y += 21;
        }

        buffer
    }

    fn as_drop_down_menu(&mut self) -> Option<&mut dyn TheDropdownMenuTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheDropdownMenuTrait: TheWidget {
    fn add_option(&mut self, option: String);
    fn clear_options(&mut self);
    fn selected_text(&self) -> String;
    fn set_selected_index(&mut self, index: i32);
    fn selected_index(&self) -> usize;
}

impl TheDropdownMenuTrait for TheDropdownMenu {
    fn add_option(&mut self, option: String) {
        self.options.push(option);
    }
    fn clear_options(&mut self) {
        self.options.clear();
        self.selected = 0;
        self.original = 0;
        self.is_dirty = true;
    }
    fn selected_index(&self) -> usize {
        self.selected as usize
    }
    fn selected_text(&self) -> String {
        self.options[self.selected as usize].clone()
    }
    fn set_selected_index(&mut self, index: i32) {
        self.selected = index;
        self.is_dirty = true;
    }
}
