use crate::prelude::*;

pub struct TheListItem {
    id: TheId,
    limiter: TheSizeLimiter,

    state: TheWidgetState,

    text: String,
    sub_text: String,

    dim: TheDim,
    is_dirty: bool,

    mouse_down_pos: Vec2<i32>,

    icon: Option<TheRGBABuffer>,
    status: Option<String>,

    layout_id: TheId,
    scroll_offset: i32,

    values: Vec<(i32, TheValue)>,

    context_menu: Option<TheContextMenu>,

    background: Option<TheColor>,
}

impl TheWidget for TheListItem {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_height(17);

        Self {
            id,
            limiter,

            state: TheWidgetState::None,

            text: "".to_string(),
            sub_text: "".to_string(),

            dim: TheDim::zero(),
            is_dirty: true,

            mouse_down_pos: Vec2::zero(),

            icon: None,
            status: None,

            layout_id: TheId::empty(),
            scroll_offset: 0,

            values: Vec::new(),

            context_menu: None,

            background: None,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
    }

    fn set_context_menu(&mut self, menu: Option<TheContextMenu>) {
        self.context_menu = menu;
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        // println!("event ({}): {:?}", self.widget_id.name, event);
        match event {
            TheEvent::Context(coord) => {
                if let Some(context_menu) = &self.context_menu {
                    ctx.ui.send(TheEvent::ShowContextMenu(
                        self.id().clone(),
                        *coord,
                        context_menu.clone(),
                    ));
                    ctx.ui.set_focus(self.id());
                    redraw = true;
                    self.is_dirty = true;
                }
            }
            TheEvent::MouseDown(coord) => {
                if self.state != TheWidgetState::Selected || !self.id().equals(&ctx.ui.focus) {
                    self.is_dirty = true;
                    self.state = TheWidgetState::Selected;
                    ctx.ui.send_widget_state_changed(self.id(), self.state);
                    ctx.ui.send(TheEvent::NewListItemSelected(
                        self.id().clone(),
                        self.layout_id.clone(),
                    ));
                    redraw = true;
                }
                self.mouse_down_pos = Vec2::new(coord.x, coord.y + self.scroll_offset);
                ctx.ui.set_focus(self.id());
            }
            TheEvent::MouseDragged(coord) => {
                let coord = Vec2::new(coord.x, coord.y + self.scroll_offset);
                if ctx.ui.drop.is_none()
                    && Vec2::new(self.mouse_down_pos.x as f32, self.mouse_down_pos.y as f32)
                        .distance(Vec2::new(coord.x as f32, coord.y as f32))
                        >= 5.0
                {
                    ctx.ui.send(TheEvent::DragStarted(
                        self.id().clone(),
                        self.text.clone(),
                        coord,
                    ));
                }
            }
            TheEvent::Hover(_coord) => {
                if self.state != TheWidgetState::Selected && !self.id().equals(&ctx.ui.hover) {
                    self.is_dirty = true;
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }
            }
            TheEvent::MouseWheel(delta) => {
                ctx.ui
                    .send(TheEvent::ScrollLayout(self.layout_id.clone(), *delta));
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

    fn value(&self) -> TheValue {
        TheValue::Text(self.text.clone())
    }

    fn set_value(&mut self, value: TheValue) {
        match value {
            TheValue::Empty => {
                self.text = "".to_string();
                self.is_dirty = true;
            }
            TheValue::Text(text) => {
                self.text.clone_from(&text);
                self.is_dirty = true;
            }
            TheValue::Image(image) => {
                self.icon = Some(image);
                self.is_dirty = true;
            }
            _ => {}
        }
    }

    fn status_text(&self) -> Option<String> {
        self.status.clone()
    }

    fn set_status_text(&mut self, text: &str) {
        self.status = Some(text.to_string());
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

        let mut color = if self.state == TheWidgetState::Selected {
            if !self.id().equals(&ctx.ui.focus) {
                *style.theme().color(ListItemSelectedNoFocus)
            } else {
                *style.theme().color(ListItemSelected)
            }
        } else if let Some(background) = &self.background {
            background.to_u8_array()
        } else {
            *style.theme().color(ListItemNormal)
        };

        if self.state != TheWidgetState::Selected && self.id().equals(&ctx.ui.hover) {
            color = *style.theme().color(ListItemHover)
        }

        let stride = buffer.stride();
        let mut shrinker = TheDimShrinker::zero();

        ctx.draw.rect_outline_border(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &color,
            1,
        );

        shrinker.shrink(1);
        ctx.draw.rect(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &color,
        );

        if let Some(icon) = &self.icon {
            let ut = self.dim.to_buffer_shrunk_utuple(&shrinker);
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &(ut.0 + 1, ut.1 + 1, 38, 38),
                stride,
                style.theme().color(ListItemIconBorder),
                1,
            );
            ctx.draw.copy_slice(
                buffer.pixels_mut(),
                icon.pixels(),
                &(ut.0 + 2, ut.1 + 2, 36, 36),
                stride,
            );

            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(
                    ut.0 + 38 + 7 + 5,
                    ut.1 + 5,
                    (self.dim.width - 38 - 7 - 10) as usize,
                    13,
                ),
                stride,
                &self.text,
                TheFontSettings {
                    size: 12.0,
                    ..Default::default()
                },
                style.theme().color(ListItemText),
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );

            if !self.sub_text.is_empty() {
                ctx.draw.text_rect_blend(
                    buffer.pixels_mut(),
                    &(
                        ut.0 + 38 + 7 + 5,
                        ut.1 + 22,
                        (self.dim.width - 38 - 7 - 10) as usize,
                        13,
                    ),
                    stride,
                    &self.sub_text,
                    TheFontSettings {
                        size: 12.0,
                        ..Default::default()
                    },
                    style.theme().color(ListItemText),
                    TheHorizontalAlign::Left,
                    TheVerticalAlign::Center,
                );
            }
        } else {
            let mut right_width = 5;
            for v in self.values.iter() {
                right_width += v.0;
            }

            shrinker.shrink_by(9, 0, 0, 0);
            let mut rect: (usize, usize, usize, usize) =
                self.dim.to_buffer_shrunk_utuple(&shrinker);

            ctx.draw.text_rect_blend(
                buffer.pixels_mut(),
                &(rect.0, rect.1, rect.2 - right_width as usize, rect.3),
                stride,
                &self.text,
                TheFontSettings {
                    size: 13.0,
                    ..Default::default()
                },
                style.theme().color(ListItemText),
                TheHorizontalAlign::Left,
                TheVerticalAlign::Center,
            );

            rect.0 += rect.2 - right_width as usize;

            for (width, value) in self.values.iter() {
                ctx.draw.rect(
                    buffer.pixels_mut(),
                    &(rect.0, rect.1 - 1, 1, rect.3 + 2),
                    stride,
                    style.theme().color(ListLayoutBackground),
                );

                #[allow(clippy::single_match)]
                match value {
                    TheValue::Text(text) => {
                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            &(rect.0 + 9, rect.1, *width as usize - 10, rect.3),
                            stride,
                            text,
                            TheFontSettings {
                                size: 13.0,
                                ..Default::default()
                            },
                            style.theme().color(ListItemText),
                            TheHorizontalAlign::Left,
                            TheVerticalAlign::Center,
                        );
                    }
                    _ => {
                        ctx.draw.text_rect_blend(
                            buffer.pixels_mut(),
                            &(rect.0 + 9, rect.1, *width as usize - 10, rect.3),
                            stride,
                            &value.describe(),
                            TheFontSettings {
                                size: 13.0,
                                ..Default::default()
                            },
                            style.theme().color(ListItemText),
                            TheHorizontalAlign::Left,
                            TheVerticalAlign::Center,
                        );
                    }
                }
            }
        }

        self.is_dirty = false;
    }

    fn as_list_item(&mut self) -> Option<&mut dyn TheListItemTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheListItemTrait {
    fn set_background_color(&mut self, color: TheColor);
    fn set_text(&mut self, text: String);
    fn set_sub_text(&mut self, sub_text: String);
    fn set_associated_layout(&mut self, id: TheId);
    fn set_size(&mut self, size: i32);
    fn set_icon(&mut self, icon: TheRGBABuffer);
    fn set_scroll_offset(&mut self, offset: i32);
    fn add_value_column(&mut self, width: i32, value: TheValue);
}

impl TheListItemTrait for TheListItem {
    fn set_background_color(&mut self, color: TheColor) {
        self.background = Some(color);
        self.is_dirty = true;
    }
    fn set_text(&mut self, text: String) {
        self.text = text;
        self.is_dirty = true;
    }
    fn set_sub_text(&mut self, sub_text: String) {
        self.sub_text = sub_text;
        self.is_dirty = true;
    }
    fn set_associated_layout(&mut self, layout_id: TheId) {
        self.layout_id = layout_id;
    }
    fn set_size(&mut self, size: i32) {
        self.limiter_mut().set_max_height(size);
        self.is_dirty = true;
    }
    fn set_icon(&mut self, icon: TheRGBABuffer) {
        self.icon = Some(icon);
    }
    fn set_scroll_offset(&mut self, offset: i32) {
        self.scroll_offset = offset;
    }
    fn add_value_column(&mut self, width: i32, value: TheValue) {
        self.values.push((width, value));
    }
}
