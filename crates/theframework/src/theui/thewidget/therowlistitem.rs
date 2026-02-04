use crate::prelude::*;

pub struct TheRowListItem {
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

    context_menu: Option<TheContextMenu>,

    background: Option<TheColor>,
}

impl TheWidget for TheRowListItem {
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
                self.mouse_down_pos = Vec2::new(coord.x + self.scroll_offset, coord.y);
                ctx.ui.set_focus(self.id());
            }
            TheEvent::MouseDragged(coord) => {
                let coord = Vec2::new(coord.x + self.scroll_offset, coord.y);
                if ctx.ui.drop.is_none()
                    && Vec2::new(self.mouse_down_pos.x as f32, self.mouse_down_pos.y as f32)
                        .distance(Vec2::new(coord.x as f32, coord.y as f32))
                        > 5.0
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
            2,
        );

        shrinker.shrink(1);
        ctx.draw.rect(
            buffer.pixels_mut(),
            &self.dim.to_buffer_shrunk_utuple(&shrinker),
            stride,
            &color,
        );

        let ut = self.dim.to_buffer_shrunk_utuple(&shrinker);

        if let Some(icon) = &self.icon {
            let icon_width = ut.2 - 16;
            let icon_height = icon_width; //ut.3 - 16 - 20;

            let ut = self.dim.to_buffer_shrunk_utuple(&shrinker);
            let off_x = 8;
            let off_y = 8;
            ctx.draw.rect_outline_border(
                buffer.pixels_mut(),
                &(
                    ut.0 + off_x - 1,
                    ut.1 + off_y - 1,
                    icon_width + 2,
                    icon_height + 2,
                ),
                stride,
                style.theme().color(ListItemIconBorder),
                1,
            );
            ctx.draw.scale_chunk(
                buffer.pixels_mut(),
                &(ut.0 + off_x, ut.1 + off_y, icon_width, icon_height),
                stride,
                icon.pixels(),
                &(icon.dim().width as usize, icon.dim().height as usize),
                1.0,
            );
        }

        // if let Some(icon) = &self.icon {
        //     let icon_width = icon.dim().width as usize;
        //     let icon_height = icon.dim().height as usize;

        //     if icon_width < ut.2 && icon_height < ut.3 {
        //         let ut = self.dim.to_buffer_shrunk_utuple(&shrinker);
        //         let off_x = (ut.2 - icon_width) / 2;
        //         let off_y = (ut.3 - icon_height) / 2 - 20;
        //         ctx.draw.rect_outline_border(
        //             buffer.pixels_mut(),
        //             &(
        //                 ut.0 + off_x - 1,
        //                 ut.1 + off_y - 1,
        //                 icon_width + 2,
        //                 icon_height + 2,
        //             ),
        //             stride,
        //             style.theme().color(ListItemIconBorder),
        //             1,
        //         );
        //         ctx.draw.copy_slice(
        //             buffer.pixels_mut(),
        //             icon.pixels(),
        //             &(ut.0 + off_x, ut.1 + off_y, icon_width, icon_height),
        //             stride,
        //         );
        //     }
        // }

        ctx.draw.text_rect_blend(
            buffer.pixels_mut(),
            &(ut.0, ut.1 + self.dim.height as usize - 25, ut.2, 20),
            stride,
            &self.text,
            TheFontSettings {
                size: 12.0,
                ..Default::default()
            },
            style.theme().color(ListItemText),
            TheHorizontalAlign::Center,
            TheVerticalAlign::Center,
        );

        self.is_dirty = false;
    }

    fn as_rowlist_item(&mut self) -> Option<&mut dyn TheRowListItemTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheRowListItemTrait {
    fn set_background_color(&mut self, color: TheColor);
    fn set_text(&mut self, text: String);
    fn set_sub_text(&mut self, sub_text: String);
    fn set_associated_layout(&mut self, id: TheId);
    fn set_size(&mut self, size: i32);
    fn set_icon(&mut self, icon: TheRGBABuffer);
    fn set_scroll_offset(&mut self, offset: i32);
}

impl TheRowListItemTrait for TheRowListItem {
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
}
