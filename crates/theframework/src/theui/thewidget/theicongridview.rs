use crate::prelude::*;

use super::thescrollbarchrome::draw_scrollbar_chrome;

#[derive(Clone, Debug)]
pub struct TheIconGridItem {
    pub label: String,
    pub status: String,
    pub icon: Option<TheRGBABuffer>,
}

impl TheIconGridItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: String::new(),
            icon: None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TheIconGridMetrics {
    columns: usize,
    rows: usize,
    cell_size: i32,
    content_height: i32,
    viewport_width: i32,
    needs_scrollbar: bool,
}

pub struct TheIconGridView {
    id: TheId,
    limiter: TheSizeLimiter,
    dim: TheDim,
    items: Vec<TheIconGridItem>,
    selected: Option<usize>,
    hovered: Option<usize>,
    rectangles: Vec<(usize, TheDim)>,
    preferred_cell_size: i32,
    icon_size: i32,
    icon_padding: i32,
    gap: i32,
    padding: i32,
    show_labels: bool,
    scroll_offset: i32,
    scrollbar_width: i32,
    scrollbar_hovered: bool,
    scrollbar_drag_anchor: Option<(i32, i32)>,
    is_dirty: bool,
}

impl TheIconGridView {
    fn metrics(&self) -> TheIconGridMetrics {
        let calculate = |viewport_width: i32| {
            let available = viewport_width
                .saturating_sub(self.padding.saturating_mul(2))
                .max(24);
            let cell_size = self.preferred_cell_size.min(available).max(24);
            let columns = (available.saturating_add(self.gap) / cell_size.saturating_add(self.gap))
                .max(1) as usize;
            let rows = self.items.len().div_ceil(columns);
            let rows_i32 = i32::try_from(rows).unwrap_or(i32::MAX);
            let gaps_i32 = i32::try_from(rows.saturating_sub(1)).unwrap_or(i32::MAX);
            let content_height = if rows == 0 {
                self.padding * 2
            } else {
                self.padding
                    .saturating_mul(2)
                    .saturating_add(rows_i32.saturating_mul(cell_size))
                    .saturating_add(gaps_i32.saturating_mul(self.gap))
            };
            (columns, rows, cell_size, content_height)
        };

        let full_width = self.dim.width.max(0);
        let (mut columns, mut rows, mut cell_size, mut content_height) = calculate(full_width);
        let needs_scrollbar = content_height > self.dim.height;
        let viewport_width = if needs_scrollbar {
            full_width.saturating_sub(self.scrollbar_width)
        } else {
            full_width
        };
        if needs_scrollbar {
            (columns, rows, cell_size, content_height) = calculate(viewport_width);
        }

        TheIconGridMetrics {
            columns,
            rows,
            cell_size,
            content_height,
            viewport_width,
            needs_scrollbar: content_height > self.dim.height,
        }
    }

    fn max_scroll_for(&self, metrics: TheIconGridMetrics) -> i32 {
        metrics
            .content_height
            .saturating_sub(self.dim.height)
            .max(0)
    }

    fn clamp_scroll(&mut self) {
        let metrics = self.metrics();
        self.scroll_offset = self.scroll_offset.clamp(0, self.max_scroll_for(metrics));
    }

    fn scroll_by(&mut self, delta: i32) -> bool {
        let metrics = self.metrics();
        let old = self.scroll_offset;
        self.scroll_offset = self
            .scroll_offset
            .saturating_add(delta)
            .clamp(0, self.max_scroll_for(metrics));
        if self.scroll_offset != old {
            self.is_dirty = true;
            true
        } else {
            false
        }
    }

    fn scrollbar_bounds(&self, metrics: TheIconGridMetrics) -> TheDim {
        TheDim::new(
            metrics.viewport_width,
            0,
            self.scrollbar_width,
            self.dim.height,
        )
    }

    fn scrollbar_thumb(&self, metrics: TheIconGridMetrics) -> TheDim {
        let bounds = self.scrollbar_bounds(metrics);
        if !metrics.needs_scrollbar || metrics.content_height <= 0 {
            return TheDim::new(bounds.x + 2, 0, (bounds.width - 4).max(1), bounds.height);
        }
        let thumb_height = ((bounds.height as f32 * bounds.height as f32)
            / metrics.content_height as f32)
            .round() as i32;
        let thumb_height = thumb_height.clamp(18.min(bounds.height), bounds.height);
        let travel = (bounds.height - thumb_height).max(0);
        let max_scroll = self.max_scroll_for(metrics);
        let y = if max_scroll > 0 {
            (self.scroll_offset as f32 * travel as f32 / max_scroll as f32).round() as i32
        } else {
            0
        };
        TheDim::new(bounds.x + 2, y, (bounds.width - 4).max(1), thumb_height)
    }

    fn scroll_from_thumb_drag(&mut self, coord_y: i32, anchor_y: i32, start_scroll: i32) {
        let metrics = self.metrics();
        let thumb = self.scrollbar_thumb(metrics);
        let travel = (self.dim.height - thumb.height).max(0);
        let max_scroll = self.max_scroll_for(metrics);
        if travel > 0 && max_scroll > 0 {
            let delta = coord_y.saturating_sub(anchor_y);
            let scroll_delta = (delta as f32 * max_scroll as f32 / travel as f32).round() as i32;
            self.scroll_offset = start_scroll
                .saturating_add(scroll_delta)
                .clamp(0, max_scroll);
            self.is_dirty = true;
        }
    }

    fn ensure_selected_visible(&mut self) {
        let Some(selected) = self.selected else {
            return;
        };
        let metrics = self.metrics();
        if metrics.columns == 0 {
            return;
        }
        let row = selected / metrics.columns;
        let row = i32::try_from(row).unwrap_or(i32::MAX);
        let top = self
            .padding
            .saturating_add(row.saturating_mul(metrics.cell_size.saturating_add(self.gap)));
        let bottom = top.saturating_add(metrics.cell_size);
        if top < self.scroll_offset {
            self.scroll_offset = top;
        } else if bottom > self.scroll_offset.saturating_add(self.dim.height) {
            self.scroll_offset = bottom.saturating_sub(self.dim.height);
        }
        self.clamp_scroll();
    }
}

impl TheWidget for TheIconGridView {
    fn new(id: TheId) -> Self
    where
        Self: Sized,
    {
        let mut limiter = TheSizeLimiter::new();
        limiter.set_max_size(Vec2::new(i32::MAX, i32::MAX));
        Self {
            id,
            limiter,
            dim: TheDim::zero(),
            items: Vec::new(),
            selected: None,
            hovered: None,
            rectangles: Vec::new(),
            preferred_cell_size: 88,
            icon_size: 72,
            icon_padding: 6,
            gap: 8,
            padding: 8,
            show_labels: false,
            scroll_offset: 0,
            scrollbar_width: 12,
            scrollbar_hovered: false,
            scrollbar_drag_anchor: None,
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
                ctx.ui.set_focus(self.id());
                let metrics = self.metrics();
                if metrics.needs_scrollbar && self.scrollbar_bounds(metrics).contains(*coord) {
                    let thumb = self.scrollbar_thumb(metrics);
                    if thumb.contains(*coord) {
                        self.scrollbar_drag_anchor = Some((coord.y, self.scroll_offset));
                    } else if coord.y < thumb.y {
                        self.scroll_by(-self.dim.height);
                    } else {
                        self.scroll_by(self.dim.height);
                    }
                    self.is_dirty = true;
                    return true;
                }

                if let Some((index, _)) = self
                    .rectangles
                    .iter()
                    .find(|(_, rect)| rect.contains(*coord))
                {
                    self.selected = Some(*index);
                    ctx.ui.send(TheEvent::IndexChanged(self.id.clone(), *index));
                    self.is_dirty = true;
                    return true;
                }
                false
            }
            TheEvent::MouseDragged(coord) => {
                if let Some((anchor_y, start_scroll)) = self.scrollbar_drag_anchor {
                    self.scroll_from_thumb_drag(coord.y, anchor_y, start_scroll);
                    return true;
                }
                false
            }
            TheEvent::MouseUp(_) => {
                let was_dragging = self.scrollbar_drag_anchor.take().is_some();
                if was_dragging {
                    self.is_dirty = true;
                }
                was_dragging
            }
            TheEvent::MouseWheel(delta) => {
                let delta = if delta.y != 0 { -delta.y } else { -delta.x };
                self.scroll_by(delta)
            }
            TheEvent::Hover(coord) => {
                if !self.id().equals(&ctx.ui.hover) {
                    ctx.ui.set_hover(self.id());
                    self.is_dirty = true;
                }
                let metrics = self.metrics();
                let scrollbar_hovered =
                    metrics.needs_scrollbar && self.scrollbar_bounds(metrics).contains(*coord);
                let hovered = if scrollbar_hovered {
                    None
                } else {
                    self.rectangles
                        .iter()
                        .find_map(|(index, rect)| rect.contains(*coord).then_some(*index))
                };
                if hovered != self.hovered || scrollbar_hovered != self.scrollbar_hovered {
                    self.hovered = hovered;
                    self.scrollbar_hovered = scrollbar_hovered;
                    let status = hovered
                        .and_then(|index| self.items.get(index))
                        .map(|item| item.status.clone())
                        .unwrap_or_default();
                    ctx.ui
                        .send(TheEvent::SetStatusText(self.id.clone(), status));
                    self.is_dirty = true;
                    return true;
                }
                self.is_dirty
            }
            TheEvent::LostHover(_) => {
                self.hovered = None;
                self.scrollbar_hovered = false;
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
        let metrics = self.metrics();
        let bounds = self.dim.to_buffer_utuple();
        let stride = buffer.stride();
        ctx.draw.rect(
            buffer.pixels_mut(),
            &bounds,
            stride,
            style.theme().color(ListLayoutBackground),
        );

        self.rectangles.clear();
        let step = metrics.cell_size.saturating_add(self.gap).max(1);
        let first_row =
            usize::try_from(self.scroll_offset.saturating_sub(self.padding).max(0) / step)
                .unwrap_or(0)
                .min(metrics.rows);
        let visible_extent = self
            .scroll_offset
            .saturating_add(self.dim.height)
            .saturating_sub(self.padding)
            .max(0);
        let last_row = usize::try_from(visible_extent.saturating_add(step - 1) / step)
            .unwrap_or(usize::MAX)
            .min(metrics.rows);

        for row_index in first_row..last_row {
            for column_index in 0..metrics.columns {
                let Some(index) = row_index
                    .checked_mul(metrics.columns)
                    .and_then(|index| index.checked_add(column_index))
                else {
                    break;
                };
                let Some(item) = self.items.get(index) else {
                    break;
                };
                let column = i32::try_from(column_index).unwrap_or(i32::MAX);
                let row = i32::try_from(row_index).unwrap_or(i32::MAX);
                let x = self.padding.saturating_add(column.saturating_mul(step));
                let y = self
                    .padding
                    .saturating_add(row.saturating_mul(step))
                    .saturating_sub(self.scroll_offset);
                if y.saturating_add(metrics.cell_size) <= 0 || y >= self.dim.height {
                    continue;
                }

                let local_rect = TheDim::new(x, y, metrics.cell_size, metrics.cell_size);
                let mut cell =
                    TheRGBABuffer::new(TheDim::sized(metrics.cell_size, metrics.cell_size));
                let selected = self.selected == Some(index);
                let hovered = self.hovered == Some(index);
                let background = if selected {
                    *style.theme().color(ListItemSelected)
                } else if hovered {
                    *style.theme().color(ListItemHover)
                } else {
                    *style.theme().color(ListItemNormal)
                };
                let border = if selected {
                    *style.theme().color(DefaultSelection)
                } else {
                    *style.theme().color(ListItemIconBorder)
                };
                let cell_bounds = TheDim::sized(metrics.cell_size, metrics.cell_size);
                cell.draw_rounded_rect(
                    &cell_bounds,
                    &background,
                    &(4.0, 4.0, 4.0, 4.0),
                    1.0,
                    &border,
                );

                let label_height = if self.show_labels { 18 } else { 0 };
                if let Some(icon) = item.icon.as_ref()
                    && icon.dim().width > 0
                    && icon.dim().height > 0
                {
                    let available_w = (metrics.cell_size - self.icon_padding * 2).max(1);
                    let available_h =
                        (metrics.cell_size - self.icon_padding * 2 - label_height).max(1);
                    let max_size = self.icon_size.min(available_w).min(available_h).max(1);
                    let scale = (max_size as f32 / icon.dim().width as f32)
                        .min(max_size as f32 / icon.dim().height as f32);
                    let width = (icon.dim().width as f32 * scale).round().max(1.0) as usize;
                    let height = (icon.dim().height as f32 * scale).round().max(1.0) as usize;
                    let target = (
                        ((metrics.cell_size - width as i32) / 2).max(0) as usize,
                        ((metrics.cell_size - label_height - height as i32) / 2).max(0) as usize,
                        width,
                        height,
                    );
                    let cell_stride = cell.stride();
                    ctx.draw.blend_scale_chunk(
                        cell.pixels_mut(),
                        &target,
                        cell_stride,
                        icon.pixels(),
                        &(icon.dim().width as usize, icon.dim().height as usize),
                    );
                }

                if self.show_labels {
                    let label_rect = (
                        3,
                        (metrics.cell_size - label_height) as usize,
                        (metrics.cell_size - 6).max(1) as usize,
                        label_height as usize,
                    );
                    let cell_stride = cell.stride();
                    ctx.draw.text_rect_blend(
                        cell.pixels_mut(),
                        &label_rect,
                        cell_stride,
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

                // A partially visible row must remain clipped to this widget. `copy_into`
                // clips to the destination buffer, which may include neighboring canvases.
                let visible_left = x.max(0);
                let visible_top = y.max(0);
                let visible_right = x
                    .saturating_add(metrics.cell_size)
                    .min(metrics.viewport_width)
                    .min(self.dim.width);
                let visible_bottom = y.saturating_add(metrics.cell_size).min(self.dim.height);
                if visible_right > visible_left && visible_bottom > visible_top {
                    let source = TheDim::new(
                        visible_left - x,
                        visible_top - y,
                        visible_right - visible_left,
                        visible_bottom - visible_top,
                    );
                    if source.x == 0
                        && source.y == 0
                        && source.width == metrics.cell_size
                        && source.height == metrics.cell_size
                    {
                        buffer.copy_into(
                            self.dim.buffer_x + visible_left,
                            self.dim.buffer_y + visible_top,
                            &cell,
                        );
                    } else {
                        let clipped = cell.extract(&source);
                        buffer.copy_into(
                            self.dim.buffer_x + visible_left,
                            self.dim.buffer_y + visible_top,
                            &clipped,
                        );
                    }
                }
                self.rectangles.push((index, local_rect));
            }
        }

        if metrics.needs_scrollbar {
            let scrollbar = self.scrollbar_bounds(metrics);
            let thumb = self.scrollbar_thumb(metrics);
            draw_scrollbar_chrome(
                buffer,
                ThePixelRect::new(
                    self.dim.buffer_x + scrollbar.x,
                    self.dim.buffer_y + scrollbar.y,
                    scrollbar.width,
                    scrollbar.height,
                ),
                ThePixelRect::new(
                    self.dim.buffer_x + thumb.x,
                    self.dim.buffer_y + thumb.y,
                    thumb.width,
                    thumb.height,
                ),
                self.scrollbar_hovered,
                self.scrollbar_drag_anchor.is_some(),
                style,
                ctx,
            );
        }
        self.is_dirty = false;
    }

    fn as_icon_grid_view(&mut self) -> Option<&mut dyn TheIconGridViewTrait> {
        Some(self)
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub trait TheIconGridViewTrait {
    fn set_items(&mut self, items: Vec<TheIconGridItem>);
    fn set_selected(&mut self, selected: Option<usize>);
    fn selected(&self) -> Option<usize>;
    fn set_cell_size(&mut self, size: i32);
    fn set_icon_size(&mut self, size: i32);
    fn set_icon_padding(&mut self, padding: i32);
    fn set_spacing(&mut self, spacing: i32);
    fn set_content_padding(&mut self, padding: i32);
    fn set_show_labels(&mut self, show: bool);
}

impl TheIconGridViewTrait for TheIconGridView {
    fn set_items(&mut self, items: Vec<TheIconGridItem>) {
        self.items = items;
        self.selected = self
            .selected
            .filter(|selected| *selected < self.items.len());
        self.hovered = None;
        self.clamp_scroll();
        self.is_dirty = true;
    }

    fn set_selected(&mut self, selected: Option<usize>) {
        self.selected = selected.filter(|selected| *selected < self.items.len());
        self.ensure_selected_visible();
        self.is_dirty = true;
    }

    fn selected(&self) -> Option<usize> {
        self.selected
    }

    fn set_cell_size(&mut self, size: i32) {
        self.preferred_cell_size = size.clamp(24, 512);
        self.clamp_scroll();
        self.is_dirty = true;
    }

    fn set_icon_size(&mut self, size: i32) {
        self.icon_size = size.clamp(8, 496);
        self.is_dirty = true;
    }

    fn set_icon_padding(&mut self, padding: i32) {
        self.icon_padding = padding.clamp(0, 64);
        self.is_dirty = true;
    }

    fn set_spacing(&mut self, spacing: i32) {
        self.gap = spacing.clamp(0, 64);
        self.clamp_scroll();
        self.is_dirty = true;
    }

    fn set_content_padding(&mut self, padding: i32) {
        self.padding = padding.clamp(0, 64);
        self.clamp_scroll();
        self.is_dirty = true;
    }

    fn set_show_labels(&mut self, show: bool) {
        self.show_labels = show;
        self.is_dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_follow_available_width_and_overflow_enables_scrolling() {
        let mut view = TheIconGridView::new(TheId::named("Grid"));
        view.items = (0..10)
            .map(|index| TheIconGridItem::new(index.to_string()))
            .collect();
        view.dim = TheDim::sized(320, 150);

        let metrics = view.metrics();
        assert_eq!(metrics.columns, 3);
        assert_eq!(metrics.rows, 4);
        assert!(metrics.needs_scrollbar);
        assert!(view.max_scroll_for(metrics) > 0);
    }
}
