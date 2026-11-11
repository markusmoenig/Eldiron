use super::*;
use crate::{
    paint::{PaintOp, Rect, Sampling},
    theui::paint::{UiPaintResult, WidgetPainter, bounds},
};
impl TheIconGridView {
    pub(super) fn prepare_commands(&mut self) {
        self.clamp_scroll();
        self.rectangles.clear();
        let metrics = self.metrics();
        let step = metrics.cell_size.saturating_add(self.gap).max(1);
        let first = (self.scroll_offset.saturating_sub(self.padding).max(0) / step) as usize;
        let end = ((self.scroll_offset + self.dim.height - self.padding).max(0) + step - 1) / step;
        for row in first.min(metrics.rows)..(end as usize).min(metrics.rows) {
            for col in 0..metrics.columns {
                let Some(index) = row
                    .checked_mul(metrics.columns)
                    .and_then(|i| i.checked_add(col))
                else {
                    break;
                };
                if index >= self.items.len() {
                    break;
                }
                let x = self.padding + col as i32 * step;
                let y = self.padding + row as i32 * step - self.scroll_offset;
                if y + metrics.cell_size > 0 && y < self.dim.height {
                    self.rectangles.push((
                        index,
                        TheDim::new(x, y, metrics.cell_size, metrics.cell_size),
                    ));
                }
            }
        }
    }
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let r = bounds(&self.dim);
        let metrics = self.metrics();
        p.fill(r, ListLayoutBackground);
        p.list.ops.push(PaintOp::PushClip(Rect::new(
            r.x,
            r.y,
            metrics.viewport_width as f32,
            r.height,
        )));
        for &(index, local) in &self.rectangles {
            let item = &self.items[index];
            let cell = Rect::new(
                r.x + local.x as f32,
                r.y + local.y as f32,
                local.width as f32,
                local.height as f32,
            );
            let selected = self.selected == Some(index);
            let hovered = self.hovered == Some(index);
            let border = if selected {
                DefaultSelection
            } else {
                ListItemIconBorder
            };
            p.list.rect(cell, 4., p.color(border));
            p.list.rect(
                cell.inset(1.),
                3.,
                p.color(if selected {
                    ListItemSelected
                } else if hovered {
                    ListItemHover
                } else {
                    ListItemNormal
                }),
            );
            let label_height = if self.show_labels { 18. } else { 0. };
            if let Some(icon) = &item.icon {
                if icon.is_valid() {
                    let max_size = self
                        .icon_size
                        .min((local.width - self.icon_padding * 2).max(1))
                        .min((local.height - self.icon_padding * 2 - label_height as i32).max(1))
                        .max(1) as f32;
                    let scale = (max_size / icon.dim().width as f32)
                        .min(max_size / icon.dim().height as f32);
                    let width = (icon.dim().width as f32 * scale).round().max(1.);
                    let height = (icon.dim().height as f32 * scale).round().max(1.);
                    let target = Rect::new(
                        cell.x + ((cell.width - width) * 0.5).floor(),
                        cell.y + ((cell.height - label_height - height) * 0.5).floor(),
                        width,
                        height,
                    );
                    p.image(
                        &format!("icon-grid:{}:{index}", self.id.uuid),
                        icon,
                        target,
                        self.image_dirty.contains(&index),
                        Sampling::Linear,
                    )?;
                    self.image_dirty.remove(&index);
                }
            }
            if self.show_labels {
                p.label(
                    Rect::new(
                        cell.x + 3.,
                        cell.y + cell.height - label_height,
                        (cell.width - 6.).max(1.),
                        label_height,
                    ),
                    &item.label,
                    10.,
                    *p.theme.color(ListItemText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
        }
        p.list.ops.push(PaintOp::PopClip);
        if metrics.needs_scrollbar {
            let bar = self.scrollbar_bounds(metrics);
            let thumb = self.scrollbar_thumb(metrics);
            p.scrollbar(
                Rect::new(
                    r.x + bar.x as f32,
                    r.y + bar.y as f32,
                    bar.width as f32,
                    bar.height as f32,
                ),
                Rect::new(
                    r.x + thumb.x as f32,
                    r.y + thumb.y as f32,
                    thumb.width as f32,
                    thumb.height as f32,
                ),
                self.scrollbar_hovered,
                self.scrollbar_drag_anchor.is_some(),
            );
        }
        self.is_dirty = false;
        Ok(())
    }
}
