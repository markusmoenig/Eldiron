#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;

use web_time::Instant;

use crate::{prelude::*, theui::thewidget::thetextedit::TheCursor};

use super::thetextedit::{TheTextEditState, TheTextRenderer};

pub struct TheTextView {
    // Widget Basic
    id: TheId,
    limiter: TheSizeLimiter,
    status: Option<String>,

    // Dimension
    dim: TheDim,

    // Text state
    state: TheTextEditState,

    // Text render
    renderer: TheTextRenderer,
    scrollbar_size: usize,
    draw_background: bool,
    draw_border: bool,
    word_wrap: bool,

    // Interaction
    drag_start_index: usize,
    hover_coord: Vec2<i32>,
    is_clicking_on_selection: bool,
    last_mouse_down_coord: Vec2<i32>,
    last_mouse_down_time: Instant,
    selectable: bool,

    // Modifiers
    modifier_ctrl: bool,

    // Scrollbar
    hscrollbar: Box<dyn TheWidget>,
    vscrollbar: Box<dyn TheWidget>,
    is_hscrollbar_clicked: bool,
    is_hscrollbar_hovered: bool,
    is_vscrollbar_clicked: bool,
    is_vscrollbar_hovered: bool,

    is_dirty: bool,
    embedded: bool,
}

impl TheWidget for TheTextView {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut state = TheTextEditState::default();
        state.allow_select_blank = false;

        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_width(200);
        limiter.set_max_height(300);

        let hscrollbar = Box::new(TheHorizontalScrollbar::new(TheId::named(
            (id.name.clone() + " Horizontal Scrollbar").as_str(),
        )));
        let vscrollbar = Box::new(TheVerticalScrollbar::new(TheId::named(
            (id.name.clone() + " Vertical Scrollbar").as_str(),
        )));

        Self {
            id,
            limiter,
            status: None,

            dim: TheDim::zero(),

            state,

            renderer: TheTextRenderer::default(),
            scrollbar_size: 13,
            draw_border: false,
            draw_background: false,
            word_wrap: true,

            drag_start_index: 0,
            hover_coord: Vec2::zero(),
            is_clicking_on_selection: false,
            last_mouse_down_coord: Vec2::zero(),
            last_mouse_down_time: Instant::now(),
            selectable: true,

            modifier_ctrl: false,

            hscrollbar,
            vscrollbar,
            is_hscrollbar_clicked: false,
            is_hscrollbar_hovered: false,
            is_vscrollbar_clicked: false,
            is_vscrollbar_hovered: false,

            is_dirty: false,
            embedded: false,
        }
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

    fn disabled(&self) -> bool {
        true
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

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn supports_text_input(&self) -> bool {
        true
    }

    fn supports_clipboard(&mut self) -> bool {
        true
    }

    fn supports_undo_redo(&mut self) -> bool {
        true
    }

    #[allow(clippy::single_match)]
    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        let mut redraw = false;
        match event {
            TheEvent::Copy => {
                let text = self.state.copy_text();
                if !text.is_empty() {
                    redraw = true;

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let mut clipboard = Clipboard::new().unwrap();
                        clipboard.set_text(text.clone()).unwrap();
                    }

                    ctx.ui
                        .send(TheEvent::SetClipboard(TheValue::Text(text), None));
                }
            }
            TheEvent::ModifierChanged(_shift, ctrl, _alt, _logo) => {
                self.modifier_ctrl = *ctrl;
            }
            TheEvent::MouseDown(coord) => {
                if !self.state.is_empty() {
                    let global_coord = coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                    if self.renderer.is_horizontal_overflow()
                        && self.hscrollbar.dim().contains(global_coord)
                    {
                        self.is_hscrollbar_clicked = true;
                        self.hscrollbar.on_event(
                            &TheEvent::MouseDown(self.hscrollbar.dim().to_local(global_coord)),
                            ctx,
                        );
                    } else if self.renderer.is_vertical_overflow()
                        && self.vscrollbar.dim().contains(global_coord)
                    {
                        self.is_vscrollbar_clicked = true;
                        self.vscrollbar.on_event(
                            &TheEvent::MouseDown(self.vscrollbar.dim().to_local(global_coord)),
                            ctx,
                        );
                    } else if self.renderer.dim().contains(global_coord) {
                        self.drag_start_index = self.renderer.find_cursor_index(&coord);
                        let (cursor_row, cursor_column) =
                            self.state.find_row_col_of_index(self.drag_start_index);
                        self.state
                            .set_cursor(TheCursor::new(cursor_row, cursor_column));

                        let is_double_click = self.last_mouse_down_time.elapsed().as_millis() < 500
                            && self.last_mouse_down_coord == *coord;
                        if is_double_click {
                            if self.state.selection.is_none() {
                                // Select a word, a whole row or a spacing etc.
                                self.state.quick_select();
                            } else if self.state.is_row_all_selected(self.state.cursor.row) {
                                self.state.reset_selection();
                            } else {
                                self.state.select_row();
                            }
                        } else if self.drag_start_index >= self.state.selection.start
                            && self.drag_start_index < self.state.selection.end
                        {
                            self.is_clicking_on_selection = true;
                        } else {
                            self.state.reset_selection();
                        }
                    }
                }

                ctx.ui.set_focus(self.id());
                self.is_dirty = true;
                redraw = true;

                self.last_mouse_down_coord = *coord;
                self.last_mouse_down_time = Instant::now();
            }
            TheEvent::MouseDragged(coord) => {
                self.is_dirty = true;

                if !self.state.is_empty() {
                    if self.is_hscrollbar_clicked {
                        redraw =
                            self.hscrollbar.on_event(
                                &TheEvent::MouseDragged(self.hscrollbar.dim().to_local(
                                    coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y),
                                )),
                                ctx,
                            );
                        if let Some(scrollbar) = self.hscrollbar.as_horizontal_scrollbar() {
                            redraw = self.renderer.scroll(
                                &Vec2::new(
                                    scrollbar.scroll_offset()
                                        - self.renderer.scroll_offset.x as i32,
                                    0,
                                ),
                                false,
                            ) || redraw;
                        }
                    } else if self.is_vscrollbar_clicked {
                        redraw =
                            self.vscrollbar.on_event(
                                &TheEvent::MouseDragged(self.vscrollbar.dim().to_local(
                                    coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y),
                                )),
                                ctx,
                            );
                        if let Some(scrollbar) = self.vscrollbar.as_vertical_scrollbar() {
                            redraw = self.renderer.scroll(
                                &Vec2::new(
                                    0,
                                    scrollbar.scroll_offset()
                                        - self.renderer.scroll_offset.y as i32,
                                ),
                                false,
                            ) || redraw;
                        }
                    } else {
                        let coord = *coord;

                        let delta_x = if self.word_wrap {
                            0
                        } else {
                            if coord.x < 0 {
                                coord.x
                            } else if coord.x > self.dim.width {
                                coord.x - self.dim.width
                            } else {
                                0
                            }
                        };
                        let delta_y = if coord.y < 0 {
                            coord.y
                        } else if coord.y > self.dim.height {
                            coord.y - self.dim.height
                        } else {
                            0
                        };

                        if delta_x != 0 || delta_y != 0 {
                            let ratio = if self.last_mouse_down_time.elapsed().as_millis() > 500 {
                                8
                            } else {
                                4
                            };
                            self.renderer
                                .scroll(&Vec2::new(delta_x / ratio, delta_y / ratio), true);
                        }

                        let cursor_index = self.renderer.find_cursor_index(&coord);
                        let (cursor_row, cursor_column) =
                            self.state.find_row_col_of_index(cursor_index);
                        self.state
                            .set_cursor(TheCursor::new(cursor_row, cursor_column));

                        if self.selectable && !self.is_clicking_on_selection {
                            if self.drag_start_index != cursor_index {
                                let start = self.drag_start_index.min(cursor_index);
                                let end = self.drag_start_index.max(cursor_index);
                                self.state.select(start, end);
                            } else {
                                self.state.reset_selection();
                            }
                        }

                        redraw = true;
                    }
                }
            }
            TheEvent::MouseUp(coord) => {
                let global_coord = coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                if self.is_hscrollbar_clicked {
                    self.hscrollbar.on_event(
                        &TheEvent::MouseUp(self.hscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );
                } else if self.is_vscrollbar_clicked {
                    self.vscrollbar.on_event(
                        &TheEvent::MouseUp(self.vscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );
                } else if self.renderer.dim().contains(global_coord)
                    && self.is_clicking_on_selection
                {
                    self.state.reset_selection();
                }

                self.is_dirty = true;
                redraw = true;

                self.is_clicking_on_selection = false;
                self.is_hscrollbar_clicked = false;
                self.is_vscrollbar_clicked = false;
                self.drag_start_index = 0;
            }
            TheEvent::MouseWheel(delta) => {
                let global_coord =
                    self.hover_coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                let scrolled = if self.hscrollbar.dim().contains(global_coord) {
                    let delta = if delta.x.abs() > delta.y.abs() {
                        delta.x
                    } else {
                        delta.y
                    };
                    self.renderer.scroll(&Vec2::new(delta, 0), false)
                } else if self.vscrollbar.dim().contains(global_coord) {
                    let delta = if delta.x.abs() > delta.y.abs() {
                        delta.x
                    } else {
                        delta.y
                    };
                    self.renderer.scroll(&Vec2::new(0, -delta), false)
                } else {
                    self.renderer.scroll(&Vec2::new(delta.x, -delta.y), false)
                };
                if scrolled {
                    self.is_dirty = true;
                    redraw = true;
                }
            }
            TheEvent::Hover(coord) => {
                // The hovered widget is always current widget not scrollbars
                // We should manually draw hovered style to scrollbar hovered
                let global_coord = coord + Vec2::new(self.dim.buffer_x, self.dim.buffer_y);
                if self.renderer.is_horizontal_overflow() {
                    self.hscrollbar.on_event(
                        &TheEvent::Hover(self.hscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );

                    self.is_hscrollbar_hovered = self.hscrollbar.id().equals(&ctx.ui.hover);
                    redraw = redraw || self.hscrollbar.needs_redraw();
                }
                if self.renderer.is_vertical_overflow() {
                    self.vscrollbar.on_event(
                        &TheEvent::Hover(self.vscrollbar.dim().to_local(global_coord)),
                        ctx,
                    );

                    self.is_vscrollbar_hovered = self.vscrollbar.id().equals(&ctx.ui.hover);
                    redraw = redraw || self.vscrollbar.needs_redraw();
                }

                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    redraw = true;
                }

                self.hover_coord = *coord;
            }
            _ => {}
        }

        redraw
    }

    fn value(&self) -> TheValue {
        TheValue::Text(self.state.to_text())
    }

    fn set_value(&mut self, value: TheValue) {
        match value {
            TheValue::Empty => {
                self.state.reset();
                self.is_dirty = true;
            }
            TheValue::Text(text) => {
                self.state.reset();
                self.state.set_text(text);
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
        if !self.dim.is_valid() {
            return;
        }

        let mut shrinker = TheDimShrinker::zero();
        self.renderer.render_widget(
            &mut shrinker,
            true,
            self.embedded,
            self.draw_background,
            self.draw_border,
            self,
            buffer,
            style,
            ctx,
            true,
        );

        if self.is_dirty || self.renderer.row_count() == 0 {
            if self.word_wrap {
                self.renderer.max_width = Some(
                    self.dim.to_buffer_shrunk_utuple(&shrinker).2 as f32
                        - self.scrollbar_size as f32,
                );
            }

            self.renderer
                .prepare(&self.state.to_text(), TheFontPreference::Code, &ctx.draw);

            shrinker.shrink_by(
                -(self.renderer.padding.0 as i32),
                -(self.renderer.padding.1 as i32),
                -(self.renderer.padding.2 as i32),
                -(self.renderer.padding.3 as i32),
            );
            let outer_area = self.dim.to_buffer_shrunk_utuple(&shrinker);

            shrinker.shrink_by(
                self.renderer.padding.0 as i32,
                self.renderer.padding.1 as i32,
                self.renderer.padding.2 as i32,
                self.renderer.padding.3 as i32,
            );
            let mut visible_area = self.dim.to_buffer_shrunk_utuple(&shrinker);

            let content_w = self.renderer.actual_size.x;
            let content_h = self.renderer.actual_size.y;
            let outer_w = visible_area.2;
            let outer_h = visible_area.3;
            let inner_w = outer_w.saturating_sub(self.scrollbar_size);
            let inner_h = outer_h.saturating_sub(self.scrollbar_size);
            let (is_hoverflow, is_voverflow) = if content_w <= outer_w && content_h <= outer_h {
                (false, false)
            } else if content_w > outer_w && content_h > outer_h {
                (true, true)
            } else {
                (content_w > inner_w, content_h > inner_h)
            };
            let is_hoverflow = !self.word_wrap && is_hoverflow;
            if is_hoverflow {
                visible_area.3 = inner_h;
            }
            if is_voverflow {
                visible_area.2 = inner_w;
            }
            self.renderer.set_dim(
                visible_area.0,
                visible_area.1,
                visible_area.2,
                visible_area.3,
            );

            if is_hoverflow {
                let mut dim = TheDim::new(
                    outer_area.0 as i32,
                    (outer_area.1 + outer_area.3).saturating_sub(self.scrollbar_size) as i32,
                    outer_area
                        .2
                        .saturating_sub(if is_voverflow { self.scrollbar_size } else { 0 })
                        as i32,
                    self.scrollbar_size as i32,
                );
                dim.set_buffer_offset(dim.x, dim.y);
                self.hscrollbar.set_dim(dim, ctx);
            }
            if let Some(scrollbar) = self.hscrollbar.as_horizontal_scrollbar() {
                scrollbar.set_total_width(
                    self.renderer.actual_size.x as i32
                        + self.renderer.padding.0
                        + self.renderer.padding.2,
                );
            }

            if is_voverflow {
                let mut dim = TheDim::new(
                    (outer_area.0 + outer_area.2).saturating_sub(self.scrollbar_size) as i32,
                    outer_area.1 as i32,
                    self.scrollbar_size as i32,
                    outer_area
                        .3
                        .saturating_sub(if is_hoverflow { self.scrollbar_size } else { 0 })
                        as i32,
                );
                dim.set_buffer_offset(dim.x, dim.y);
                self.vscrollbar.set_dim(dim, ctx);
            }
            if let Some(scrollbar) = self.vscrollbar.as_vertical_scrollbar() {
                scrollbar.set_total_height(
                    self.renderer.actual_size.y as i32
                        + self.renderer.padding.1
                        + self.renderer.padding.3,
                );
            }
        }

        self.renderer.render_text(
            &self.state,
            ctx.ui.has_focus(self.id()),
            true,
            buffer,
            style,
            TheFontPreference::Code,
            &ctx.draw,
        );

        if self.renderer.is_horizontal_overflow() {
            if let Some(scrollbar) = self.hscrollbar.as_horizontal_scrollbar() {
                scrollbar.set_scroll_offset(self.renderer.scroll_offset.x as i32);

                if self.is_hscrollbar_hovered {
                    ctx.ui.set_hover(self.hscrollbar.id());
                }
                self.hscrollbar.draw(buffer, style, ctx);
                if self.is_hscrollbar_hovered {
                    ctx.ui.set_hover(self.id());
                }
            }
        }
        if self.renderer.is_vertical_overflow() {
            if let Some(scrollbar) = self.vscrollbar.as_vertical_scrollbar() {
                scrollbar.set_scroll_offset(self.renderer.scroll_offset.y as i32);

                if self.is_vscrollbar_hovered {
                    ctx.ui.set_hover(self.vscrollbar.id());
                }
                self.vscrollbar.draw(buffer, style, ctx);
                if self.is_vscrollbar_hovered {
                    ctx.ui.set_hover(self.id());
                }
            }
        }

        self.is_dirty = false;
    }

    fn as_text_view(&mut self) -> Option<&mut dyn TheTextViewTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheTextViewTrait: TheWidget {
    fn text(&self) -> String;
    fn set_text(&mut self, text: String);
    fn set_font_size(&mut self, font_size: f32);
    fn set_embedded(&mut self, embedded: bool);
    fn set_selectable(&mut self, selectable: bool);
    fn set_word_wrap(&mut self, word_wrap: bool);
    fn set_padding(&mut self, padding: (usize, usize, usize, usize));
    fn draw_background(&mut self, draw_background: bool);
    fn draw_border(&mut self, draw_border: bool);
}

impl TheTextViewTrait for TheTextView {
    fn text(&self) -> String {
        self.state.to_text()
    }
    fn set_text(&mut self, text: String) {
        self.state.set_text(text);
        self.is_dirty = true;
    }
    fn set_font_size(&mut self, font_size: f32) {
        self.renderer.set_font_size(font_size);
        self.is_dirty = true;
    }
    fn set_embedded(&mut self, embedded: bool) {
        self.embedded = embedded;
    }
    fn set_selectable(&mut self, selectable: bool) {
        self.selectable = selectable;
        if !self.selectable {
            self.state.reset_selection();
            self.is_dirty = true;
        }
    }
    fn set_word_wrap(&mut self, word_wrap: bool) {
        self.word_wrap = word_wrap;
        self.is_dirty = true;
    }
    fn set_padding(&mut self, padding: (usize, usize, usize, usize)) {
        self.renderer.padding = (
            padding.0 as i32,
            padding.1 as i32,
            padding.2 as i32,
            padding.3 as i32,
        );
        self.is_dirty = true;
    }
    fn draw_background(&mut self, draw_background: bool) {
        self.draw_background = draw_background;
        self.is_dirty = true;
    }
    fn draw_border(&mut self, draw_border: bool) {
        self.draw_border = draw_border;
        self.is_dirty = true;
    }
}
