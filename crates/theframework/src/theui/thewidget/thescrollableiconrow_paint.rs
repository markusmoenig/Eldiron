use super::*;
use crate::{
    paint::{PaintOp, Rect, Sampling},
    theui::paint::{UiPaintResult, WidgetPainter, bounds},
};
impl TheScrollableIconRow {
    pub(super) fn prepare_commands(&mut self) {
        self.clamp_scroll();
        self.rectangles.clear();
        let max_height = if self.show_labels { 50 } else { 38 };
        let height = (self.dim.height - 8).clamp(20, max_height);
        let step = (self.tile_width + self.gap).max(1);
        let first = ((self.scroll_offset - 7).max(0) / step) as usize;
        let last = ((self.scroll_offset + self.dim.width - 7).max(0) / step + 1) as usize;
        for index in first..last.min(self.items.len()) {
            let x = 7 + index as i32 * step - self.scroll_offset;
            if x + self.tile_width > 0 && x < self.dim.width {
                self.rectangles
                    .push((index, TheDim::new(x, 4, self.tile_width, height)));
            }
        }
    }
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) -> UiPaintResult {
        let r = bounds(&self.dim);
        p.fill(r, ListLayoutBackground);
        p.list.ops.push(PaintOp::PushClip(r));
        for &(index, local) in &self.rectangles {
            let item = &self.items[index];
            let cell = Rect::new(
                r.x + local.x as f32,
                r.y + local.y as f32,
                local.width as f32,
                local.height as f32,
            );
            let selected = self.selected == index;
            let hovered = self.hovered == Some(index);
            p.fill(
                cell,
                if selected {
                    ListItemSelected
                } else if hovered {
                    ListItemHover
                } else {
                    ListItemNormal
                },
            );
            if let Some(icon) = &item.icon {
                let label_height = if self.show_labels { 13. } else { 0. };
                let padding = self
                    .icon_padding
                    .min(self.tile_width / 2)
                    .min(local.height / 2) as f32;
                let target = Rect::new(
                    cell.x + padding,
                    cell.y + padding,
                    (cell.width - padding * 2.).max(1.),
                    (cell.height - padding * 2. - label_height).max(1.),
                );
                p.image(
                    &format!("icon-row:{}:{index}", self.id.uuid),
                    icon,
                    target,
                    self.image_dirty.contains(&index),
                    Sampling::Linear,
                )?;
                self.image_dirty.remove(&index);
                if self.show_labels {
                    p.label(
                        Rect::new(
                            cell.x + 2.,
                            cell.y + cell.height - label_height,
                            (cell.width - 4.).max(1.),
                            label_height,
                        ),
                        &item.label,
                        9.,
                        *p.theme.color(ListItemText),
                        TheHorizontalAlign::Center,
                        TheVerticalAlign::Center,
                    );
                }
            } else {
                p.label(
                    cell,
                    &item.label,
                    10.,
                    *p.theme.color(ListItemText),
                    TheHorizontalAlign::Center,
                    TheVerticalAlign::Center,
                );
            }
            p.outline(
                cell,
                *p.theme.color(if selected {
                    DefaultSelection
                } else if hovered {
                    ListItemHover
                } else {
                    ListItemIconBorder
                }),
                1.,
            );
        }
        p.list.ops.push(PaintOp::PopClip);
        self.is_dirty = false;
        Ok(())
    }
}
