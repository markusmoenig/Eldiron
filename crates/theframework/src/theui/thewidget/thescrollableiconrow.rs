use crate::prelude::*;

#[derive(Clone, Debug)]
pub struct TheScrollableIconRowItem {
    pub label: String,
    pub status: String,
    pub icon: Option<TheRGBABuffer>,
}

impl TheScrollableIconRowItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: String::new(),
            icon: None,
        }
    }
}

pub struct TheScrollableIconRow {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    selected: usize,
    hovered: Option<usize>,
    items: Vec<TheScrollableIconRowItem>,
    rectangles: Vec<(usize, TheDim)>,
    scroll_offset: i32,
    drag_anchor: Option<Vec2<i32>>,
    drag_start_scroll: i32,
    is_dragging: bool,
    tile_width: i32,
    icon_padding: i32,
    gap: i32,
    is_dirty: bool,
}

impl TheScrollableIconRow {
    fn content_width(&self) -> i32 {
        if self.items.is_empty() {
            0
        } else {
            self.items.len() as i32 * (self.tile_width + self.gap) - self.gap + 14
        }
    }

    fn max_scroll(&self) -> i32 {
        (self.content_width() - self.dim.width).max(0)
    }

    fn clamp_scroll(&mut self) {
        self.scroll_offset = self.scroll_offset.clamp(0, self.max_scroll());
    }

    fn scroll_by(&mut self, delta: i32) -> bool {
        let old = self.scroll_offset;
        self.scroll_offset = (self.scroll_offset + delta).clamp(0, self.max_scroll());
        if self.scroll_offset != old {
            self.is_dirty = true;
            true
        } else {
            false
        }
    }
}

impl TheWidget for TheScrollableIconRow {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_min_height(46);
        limiter.set_max_height(46);
        limiter.set_max_size(Vec2::new(i32::MAX, 46));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            selected: 0,
            hovered: None,
            items: Vec::new(),
            rectangles: Vec::new(),
            scroll_offset: 0,
            drag_anchor: None,
            drag_start_scroll: 0,
            is_dragging: false,
            tile_width: 64,
            icon_padding: 3,
            gap: 5,
            is_dirty: true,
        }
    }

    fn id(&self) -> &TheId {
        &self.id
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
            self.clamp_scroll();
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

    fn supports_hover(&mut self) -> bool {
        true
    }

    fn on_event(&mut self, event: &TheEvent, ctx: &mut TheContext) -> bool {
        match event {
            TheEvent::MouseDown(coord) => {
                self.drag_anchor = Some(*coord);
                self.drag_start_scroll = self.scroll_offset;
                self.is_dragging = false;
                ctx.ui.set_hover(self.id());
                ctx.ui.set_focus(self.id());
                true
            }
            TheEvent::MouseDragged(coord) => {
                if let Some(anchor) = self.drag_anchor {
                    let dx = coord.x - anchor.x;
                    if dx.abs() > 2 {
                        self.is_dragging = true;
                    }
                    let old = self.scroll_offset;
                    self.scroll_offset = (self.drag_start_scroll - dx).clamp(0, self.max_scroll());
                    if self.scroll_offset != old {
                        self.is_dirty = true;
                        return true;
                    }
                }
                false
            }
            TheEvent::MouseUp(coord) => {
                let clicked = !self.is_dragging;
                self.drag_anchor = None;
                self.is_dragging = false;
                if clicked {
                    for (index, rect) in &self.rectangles {
                        if rect.contains(*coord) {
                            self.selected = *index;
                            ctx.ui.send(TheEvent::IndexChanged(self.id.clone(), *index));
                            self.is_dirty = true;
                            return true;
                        }
                    }
                }
                self.is_dirty
            }
            TheEvent::MouseWheel(delta) => self.scroll_by(-delta.x - delta.y),
            TheEvent::Hover(coord) => {
                let mut redraw = false;
                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    self.is_dirty = true;
                    redraw = true;
                }
                let hovered = self
                    .rectangles
                    .iter()
                    .find_map(|(index, rect)| rect.contains(*coord).then_some(*index));
                if hovered != self.hovered {
                    self.hovered = hovered;
                    let text = hovered
                        .and_then(|index| self.items.get(index))
                        .map(|item| item.status.clone())
                        .unwrap_or_default();
                    ctx.ui.send(TheEvent::SetStatusText(self.id.clone(), text));
                    self.is_dirty = true;
                    return true;
                }
                redraw
            }
            TheEvent::LostHover(_) => {
                self.hovered = None;
                ctx.ui
                    .send(TheEvent::SetStatusText(self.id.clone(), String::new()));
                self.is_dirty = true;
                true
            }
            _ => false,
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
        self.clamp_scroll();
        let utuple = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &utuple,
            stride,
            style.theme().color(ListLayoutBackground),
        );
        self.rectangles.clear();

        let tile_h = (self.dim.height - 8).clamp(20, 38);
        for (index, item) in self.items.iter().enumerate() {
            let x = 7 + index as i32 * (self.tile_width + self.gap) - self.scroll_offset;
            let y = 4;
            if x + self.tile_width <= 0 || x >= self.dim.width {
                continue;
            }
            let rect = TheDim::new(x, y, self.tile_width, tile_h);
            let mut tile_buffer = TheRGBABuffer::new(TheDim::sized(self.tile_width, tile_h));
            let tile_stride = tile_buffer.stride();
            let outer = (0, 0, self.tile_width as usize, tile_h as usize);
            let bg = if self.selected == index {
                style.theme().color(ListItemSelected)
            } else if self.hovered == Some(index) {
                style.theme().color(ListItemHover)
            } else {
                style.theme().color(ListItemNormal)
            };
            ctx.draw
                .rect(tile_buffer.pixels_mut(), &outer, tile_stride, bg);
            if let Some(icon) = item.icon.as_ref() {
                let padding = self.icon_padding.min(self.tile_width / 2).min(tile_h / 2);
                let preview = (
                    padding as usize,
                    padding as usize,
                    (self.tile_width - padding * 2).max(1) as usize,
                    (tile_h - padding * 2).max(1) as usize,
                );
                ctx.draw.blend_scale_chunk(
                    tile_buffer.pixels_mut(),
                    &preview,
                    tile_stride,
                    icon.pixels(),
                    &(icon.dim().width as usize, icon.dim().height as usize),
                );
            } else {
                ctx.draw.text_rect_blend(
                    tile_buffer.pixels_mut(),
                    &outer,
                    tile_stride,
                    &item.label,
                    TheFontSettings {
                        size: 10.0,
                        ..Default::default()
                    },
                    style.theme().color(ListItemText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
            ctx.draw.rect_outline_border(
                tile_buffer.pixels_mut(),
                &outer,
                tile_stride,
                if self.selected == index {
                    style.theme().color(DefaultSelection)
                } else if self.hovered == Some(index) {
                    style.theme().color(ListItemHover)
                } else {
                    style.theme().color(ListItemIconBorder)
                },
                1,
            );
            buffer.copy_into(utuple.0 as i32 + x, utuple.1 as i32 + y, &tile_buffer);
            self.rectangles.push((index, rect));
        }
        self.is_dirty = false;
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheScrollableIconRowTrait {
    fn set_items(&mut self, items: Vec<TheScrollableIconRowItem>);
    fn set_selected(&mut self, selected: usize);
    fn selected(&self) -> usize;
    fn set_tile_width(&mut self, width: i32);
    fn set_icon_padding(&mut self, padding: i32);
}

impl TheScrollableIconRowTrait for TheScrollableIconRow {
    fn set_items(&mut self, items: Vec<TheScrollableIconRowItem>) {
        self.items = items;
        self.selected = self.selected.min(self.items.len().saturating_sub(1));
        self.clamp_scroll();
        self.is_dirty = true;
    }

    fn set_selected(&mut self, selected: usize) {
        self.selected = selected.min(self.items.len().saturating_sub(1));
        self.is_dirty = true;
    }

    fn selected(&self) -> usize {
        self.selected
    }

    fn set_tile_width(&mut self, width: i32) {
        self.tile_width = width.clamp(24, 160);
        self.clamp_scroll();
        self.is_dirty = true;
    }

    fn set_icon_padding(&mut self, padding: i32) {
        self.icon_padding = padding.clamp(0, 24);
        self.is_dirty = true;
    }
}
