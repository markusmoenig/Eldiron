use super::*;
use crate::{
    paint::Rect,
    theui::paint::{WidgetPainter, bounds, place_popup, rgba},
};
#[derive(Default)]
pub(super) struct PalettePopup {
    pub rect: Rect,
    columns: usize,
    count: usize,
}
impl PalettePopup {
    pub fn prepare(ctx: &TheContext, id: &TheId, dim: &TheDim, count: usize) -> (Self, Vec2<i32>) {
        let count = count.max(1);
        let columns = if count >= 144 {
            16
        } else if count >= 64 {
            12
        } else {
            8
        }
        .min(count);
        let rows = count.div_ceil(columns);
        let width = 20 + columns * 18 + (columns - 1) * 2;
        let height = 20 + rows * 18 + (rows - 1) * 2;
        let (rect, offset) = place_popup(ctx, id, dim, [width as f32, height as f32]);
        (
            Self {
                rect,
                columns,
                count,
            },
            offset,
        )
    }
    pub fn paint(&self, p: &mut WidgetPainter<'_>, palette: &ThePalette, selected: i32) {
        let r = self.rect;
        p.fill(r, MenubarPopupBackground);
        p.outline(r, *p.theme.color(MenubarPopupBorder), 1.);
        p.list.ops.push(crate::paint::PaintOp::PushClip(r));
        for i in 0..self.count {
            let rect = Rect::new(
                r.x + 10. + (i % self.columns) as f32 * 20.,
                r.y + 10. + (i / self.columns) as f32 * 20.,
                18.,
                18.,
            );
            if i as i32 == selected {
                p.fill(rect, DefaultSelection);
            }
            let color = palette
                .colors
                .get(i)
                .and_then(|v| v.as_ref())
                .map(|c| c.to_u8_array())
                .unwrap_or(BLACK);
            p.list.rect(rect.inset(2.), 0., rgba(color));
            p.outline(rect.inset(2.), BLACK, 1.);
        }
        p.list.ops.push(crate::paint::PaintOp::PopClip);
    }
}
impl ThePaletteIndexPicker {
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) {
        let r = bounds(&self.dim);
        let interactive = !self.embedded && !self.is_disabled;
        p.dropdown_chrome(
            r,
            TheDropdownChromeState {
                disabled: self.is_disabled,
                pressed: !self.embedded && self.state == TheWidgetState::Clicked,
                hovered: interactive
                    && self.state != TheWidgetState::Clicked
                    && self.id.equals(&p.ui.hover),
                focused: interactive
                    && self.state != TheWidgetState::Clicked
                    && self.id.equals(&p.ui.focus),
            },
        );
        let swatch = Rect::new(r.x + 8., r.y + 4., 16., 12.);
        let color = self
            .palette
            .colors
            .get(self.selected.max(0) as usize)
            .and_then(|v| v.as_ref())
            .map(|c| c.to_u8_array())
            .unwrap_or(BLACK);
        p.list.rect(swatch, 0., rgba(color));
        p.outline(swatch, BLACK, 1.);
        p.label(
            Rect::new(r.x + 30., r.y, (r.width - 46.).max(0.), r.height),
            &self.selected.to_string(),
            12.5,
            *p.theme.color(SectionbarNormalTextColor),
            TheHorizontalAlign::Left,
            TheVerticalAlign::Center,
        );
    }
}
impl ThePaletteIndexRowPicker {
    pub(super) fn paint_commands(&mut self, p: &mut WidgetPainter<'_>) {
        let r = bounds(&self.dim);
        let bg = if self.state == TheWidgetState::Clicked && !self.embedded {
            [76, 76, 76, 255]
        } else if !self.is_disabled && self.id.equals(&p.ui.hover) {
            [86, 86, 86, 255]
        } else {
            [65, 65, 65, 255]
        };
        p.list.rect(r, 0., rgba(bg));
        p.outline(r, [28, 28, 28, 255], 1.);
        let count = self.selected.len().max(1);
        let width = ((r.width - 10. - 4. * count.saturating_sub(1) as f32).max(0.) / count as f32)
            .floor()
            .clamp(18., 30.);
        let height = (r.height - 8.).clamp(10., 14.);
        let y = r.y + (r.height - height) * 0.5;
        for slot in 0..count {
            let x = r.x + 5. + slot as f32 * (width + 4.);
            if x + width > r.x + r.width {
                break;
            }
            let index = self.selected.get(slot).copied().unwrap_or(0).clamp(0, 255);
            let color = self
                .palette
                .colors
                .get(index as usize)
                .and_then(|v| v.as_ref())
                .map(|c| c.to_u8_array())
                .unwrap_or(BLACK);
            let rect = Rect::new(x, y, width, height);
            p.list.rect(rect, 0., rgba(color));
            p.outline(rect, [12, 12, 12, 255], 1.);
            if slot == self.active_slot {
                p.outline(rect.inset(-1.), *p.theme.color(DefaultSelection), 1.);
            }
        }
    }
}
